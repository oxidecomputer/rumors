//! Reusable two-proxy session harness for transport-adversity properties.

use std::{
    convert::Infallible,
    io,
    ops::RangeInclusive,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use futures::join;
use tokio::io::{AsyncRead, AsyncWrite};

use tokio::io::ReadBuf;

use crate::link::{Acceptor, Connector, Done, Link, MemoryLink, memory_with_capacity};
use crate::testing::{IoPlan, IoReportHandle, IoSide, wrap_link};
use crate::tree::mirror::framing::{GREETING_WORD_LEN, LENGTH_HEADER_LEN};
use crate::tree::mirror::streaming::window::WindowConfig;
use crate::tree::{
    Root as TreeRoot,
    mirror::{
        Error as MirrorError,
        streaming::{
            Local, Root,
            materialized::{Error as MaterializedError, Handshaking},
            mirror,
            remote::{Error as RemoteError, Handshaking as RemoteHandshaking},
        },
    },
};

/// Bytes buffered by each per-stream pipe before backpressure applies.
const TRANSPORT_CAPACITY: usize = 37;

/// Dense states occupied by the two nonempty-query flow variants.
const QUERY_STATES: RangeInclusive<u8> = 4..=5;

/// Dense states below this boundary carry reactions rather than bare ends.
const REACTION_STATE_COUNT: u8 = 8;

// The label's width is defined canonically beside the sender that writes
// it; the harness scripts frames with the same constant.
use super::super::super::streams::LABEL_LEN;

/// Failure returned by the materialized-left/proxy-right driver.
pub type LeftError = MirrorError<MaterializedError<Infallible>, RemoteError<Infallible>>;

/// Failure returned by the proxy-left/materialized-right driver.
pub type RightError = MirrorError<RemoteError<Infallible>, MaterializedError<Infallible>>;

/// Both endpoint results and their physical-I/O observations.
pub struct Outcome {
    /// The first materialized tree, or its session failure.
    pub left: Result<TreeRoot, LeftError>,
    /// The second materialized tree, or its session failure.
    pub right: Result<TreeRoot, RightError>,
    /// I/O performed by the first proxy endpoint.
    pub left_io: IoReportHandle,
    /// I/O performed by the second proxy endpoint.
    pub right_io: IoReportHandle,
}

/// A complete frame selected by its dense signal state.
#[derive(Clone, Copy)]
pub enum FrameSelector {
    /// The first frame regardless of its signal.
    First,
    /// The first frame carrying this semantic signal state.
    State(u8),
    /// The first nonempty query.
    Query,
    /// The first reaction which ends its reply.
    EndingReaction,
}

/// One mutation applied to the selected complete frame.
#[derive(Clone, Copy)]
pub enum FrameMutation {
    /// Replace only the signal byte, retaining the original body.
    Signal(u8),
    /// Emit the complete frame twice before its flush completes.
    Duplicate,
    /// Make the second query radix duplicate the first.
    UnorderQuery,
}

struct ScriptState {
    selector: FrameSelector,
    mutation: FrameMutation,
    fired: bool,
}

/// Observation handle proving that a configured mutation was reached.
///
/// Shared across every data stream the scripted side opens: the mutation
/// fires once, on the first frame — in deterministic poll order across
/// streams — that matches the selector.
#[derive(Clone)]
pub struct Script(Arc<Mutex<ScriptState>>);

impl Script {
    /// Select and configure one complete-frame mutation.
    pub fn new(selector: FrameSelector, mutation: FrameMutation) -> Self {
        Self(Arc::new(Mutex::new(ScriptState {
            selector,
            mutation,
            fired: false,
        })))
    }

    /// Return whether the selected frame was mutated.
    pub fn fired(&self) -> bool {
        self.0.lock().expect("frame script lock").fired
    }
}

/// A data-stream writer which edits one complete frame at its flush boundary.
///
/// Every flush below the [`StreamSender`] carries exactly one frame, so the
/// flush boundary is the frame boundary. The stream's first flush carries
/// the two-byte label ahead of its frame; mutations offset past it and leave
/// it intact.
///
/// [`StreamSender`]: crate::tree::mirror::streaming::remote::streams::StreamSender
pub struct ScriptedWrite<W> {
    inner: W,
    script: Option<Script>,
    /// Bytes of label still ahead of the frame in the next flush.
    label: usize,
    frame: Vec<u8>,
    output: Vec<u8>,
    sent: usize,
}

impl<W> ScriptedWrite<W> {
    /// Wrap one stream's `inner`, applying `script` once if it reaches its
    /// selector.
    fn new(inner: W, script: Option<Script>) -> Self {
        Self {
            inner,
            script,
            label: LABEL_LEN,
            frame: Vec::new(),
            output: Vec::new(),
            sent: 0,
        }
    }

    /// Materialize the selected mutation before bytes reach the transport.
    fn prepare(&mut self) {
        if !self.output.is_empty() || self.sent > 0 {
            return;
        }
        self.output.clone_from(&self.frame);
        let at = self.label;
        let Some(script) = &self.script else {
            return;
        };
        let mut script = script.0.lock().expect("frame script lock");
        if script.fired || self.frame.len() <= at {
            return;
        }
        let state = self.frame[at] / crate::tree::mirror::streaming::remote::codec::Stream::COUNT;
        let selected = match script.selector {
            FrameSelector::First => true,
            FrameSelector::State(expected) => state == expected,
            FrameSelector::Query => QUERY_STATES.contains(&state),
            FrameSelector::EndingReaction => state < REACTION_STATE_COUNT && state % 2 == 1,
        };
        if !selected {
            return;
        }
        match script.mutation {
            FrameMutation::Signal(signal) => self.output[at] = signal,
            FrameMutation::Duplicate => self.output.extend_from_slice(&self.frame[at..]),
            FrameMutation::UnorderQuery => {
                let query_header = at + 2;
                const QUERY_CHILD_LEN: usize = 1 + crate::tree::typed::hash::MERKLE_HASH_LEN;
                let first = self.output[query_header];
                if self.output[at + 1] == 0 {
                    self.output[at + 1] = 1;
                    self.output
                        .extend_from_within(query_header..query_header + QUERY_CHILD_LEN);
                } else {
                    self.output[query_header + QUERY_CHILD_LEN] = first;
                }
            }
        }
        script.fired = true;
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for ScriptedWrite<W> {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.get_mut().frame.extend_from_slice(bytes);
        Poll::Ready(Ok(bytes.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        this.prepare();
        while this.sent < this.output.len() {
            match Pin::new(&mut this.inner).poll_write(cx, &this.output[this.sent..]) {
                Poll::Ready(Ok(0)) => return Poll::Ready(Err(io::ErrorKind::WriteZero.into())),
                Poll::Ready(Ok(written)) => this.sent += written,
                Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                Poll::Pending => return Poll::Pending,
            }
        }
        match Pin::new(&mut this.inner).poll_flush(cx) {
            Poll::Ready(Ok(())) => {
                this.label = 0;
                this.frame.clear();
                this.output.clear();
                this.sent = 0;
                Poll::Ready(Ok(()))
            }
            other => other,
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.as_mut().poll_flush(cx) {
            Poll::Ready(Ok(())) => Pin::new(&mut self.get_mut().inner).poll_shutdown(cx),
            other => other,
        }
    }
}

/// A connector wrapping every opened data stream in a [`ScriptedWrite`]
/// sharing one [`Script`].
#[derive(Clone)]
pub struct ScriptedConnector<C> {
    inner: C,
    script: Option<Script>,
}

impl<C: Connector> Connector for ScriptedConnector<C> {
    type Tx = ScriptedWrite<C::Tx>;

    async fn connect(&self) -> io::Result<(Self::Tx, Done<Self::Tx>)> {
        let (tx, _) = self.inner.connect().await?;
        Ok((ScriptedWrite::new(tx, self.script.clone()), Done::discard()))
    }
}

/// One 8-byte little-endian greeting size word replaced in the traffic a
/// side receives.
///
/// Rewriting the *received* greeting simulates a buggy counterparty whose
/// declaration disagrees with the traffic it then sends: the receiving side
/// negotiates against the rewritten word while the sender behaves per its
/// honest tree. The offsets are absolute control-stream positions — the
/// greeting's version frame is the first control traffic at this layer, so
/// its size words sit at fixed offsets behind the frame's length header.
#[derive(Clone, Copy)]
pub struct GreetingRewrite {
    /// Absolute control-stream offset of the rewritten word.
    offset: usize,
    /// The declaration the receiving side decodes instead of the honest one.
    value: u64,
}

impl GreetingRewrite {
    /// Rewrite the received greeting's `set_len` word.
    pub fn set_len(value: u64) -> Self {
        Self {
            offset: LENGTH_HEADER_LEN,
            value,
        }
    }

    /// Rewrite the received greeting's `max_version_bytes` word.
    pub fn max_version_bytes(value: u64) -> Self {
        Self {
            offset: LENGTH_HEADER_LEN + GREETING_WORD_LEN,
            value,
        }
    }

    /// Rewrite the received greeting's `target_message_size` word.
    pub fn target_message_size(value: u64) -> Self {
        Self {
            offset: LENGTH_HEADER_LEN + 2 * GREETING_WORD_LEN,
            value,
        }
    }
}

/// A control-stream reader replacing one absolute byte range with a pinned
/// little-endian word, robust to arbitrary read chunking.
pub struct RewriteRead<R> {
    inner: R,
    rewrite: Option<GreetingRewrite>,
    /// Bytes already delivered to the reader, locating the next chunk's
    /// absolute stream offsets.
    consumed: usize,
}

impl<R> RewriteRead<R> {
    fn new(inner: R, rewrite: Option<GreetingRewrite>) -> Self {
        Self {
            inner,
            rewrite,
            consumed: 0,
        }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for RewriteRead<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let before = buf.filled().len();
        match Pin::new(&mut this.inner).poll_read(cx, buf) {
            Poll::Ready(Ok(())) => {
                let chunk = &mut buf.filled_mut()[before..];
                if let Some(rewrite) = this.rewrite {
                    let word = rewrite.value.to_le_bytes();
                    for (index, byte) in chunk.iter_mut().enumerate() {
                        if let Some(within) = (this.consumed + index).checked_sub(rewrite.offset)
                            && within < word.len()
                        {
                            *byte = word[within];
                        }
                    }
                }
                this.consumed += chunk.len();
                Poll::Ready(Ok(()))
            }
            other => other,
        }
    }
}

/// Reconcile one pair through two proxies over independently wrapped links.
pub async fn reconcile(
    left: TreeRoot,
    right: TreeRoot,
    capacity: usize,
    left_plan: IoPlan,
    right_plan: IoPlan,
) -> Outcome {
    let (left_link, right_link) = memory_with_capacity(capacity.max(1));
    let (left_link, left_io) = wrap_link(IoSide::Left, left_plan, left_link);
    let (right_link, right_io) = wrap_link(IoSide::Right, right_plan, right_link);

    let (left, right) = drive(left, right, left_link, right_link, WindowConfig::FLOOR).await;

    Outcome {
        left,
        right,
        left_io,
        right_io,
    }
}

/// Reconcile with each side's *received* greeting optionally rewritten.
///
/// Runs under the default budget-derived window so the rewritten
/// declarations flow into the live window solve, not a fixed test floor.
pub async fn reconcile_rewritten_greetings(
    left: TreeRoot,
    right: TreeRoot,
    left_hears: Option<GreetingRewrite>,
    right_hears: Option<GreetingRewrite>,
) -> (Result<TreeRoot, LeftError>, Result<TreeRoot, RightError>) {
    let (left_link, right_link) = memory_with_capacity(TRANSPORT_CAPACITY);
    drive(
        left,
        right,
        rewritten(left_link, left_hears),
        rewritten(right_link, right_hears),
        WindowConfig::default(),
    )
    .await
}

/// Wrap one link's control-read half in a greeting-word rewriter.
fn rewritten(
    link: MemoryLink,
    rewrite: Option<GreetingRewrite>,
) -> Link<
    RewriteRead<tokio::io::DuplexStream>,
    tokio::io::DuplexStream,
    crate::link::MemoryConnector,
    crate::link::MemoryAcceptor,
> {
    let parts = link.into_parts();
    crate::link::LinkParts {
        control_read: RewriteRead::new(parts.control_read, rewrite),
        control_write: parts.control_write,
        connector: parts.connector,
        acceptor: parts.acceptor,
        session: parts.session,
    }
    .into_link()
}

/// Whether the left tree wins the initiator election against the right,
/// mirroring the session's role election exactly (the smaller exchanged
/// set initiates, canonical version bytes break ties).
///
/// Role-sensitive tests arrange their corrupt or faulted side through
/// this predicate rather than through any byte-order proxy: which side
/// initiates is a function of live counts and canonical version bytes,
/// both of which move whenever the wire coding or a fixture's content
/// addresses do.
pub fn left_initiates(left: &TreeRoot, right: &TreeRoot) -> bool {
    let len = |root: &TreeRoot| {
        root.root
            .as_ref()
            .map(|node| node.len() as u64)
            .unwrap_or_default()
    };
    crate::tree::mirror::streaming::message::initiates(
        len(left),
        &left.ceiling,
        len(right),
        &right.ceiling,
    )
}

/// Reconcile while mutating at most one data-stream frame on each side.
pub async fn reconcile_scripted(
    left: TreeRoot,
    right: TreeRoot,
    left_script: Option<Script>,
    right_script: Option<Script>,
) -> (Result<TreeRoot, LeftError>, Result<TreeRoot, RightError>) {
    let (left_link, right_link) = memory_with_capacity(TRANSPORT_CAPACITY);
    drive(
        left,
        right,
        scripted(left_link, left_script),
        scripted(right_link, right_script),
        WindowConfig::FLOOR,
    )
    .await
}

/// Wrap one link's outgoing data streams with a frame script.
fn scripted(
    link: MemoryLink,
    script: Option<Script>,
) -> Link<
    tokio::io::DuplexStream,
    tokio::io::DuplexStream,
    ScriptedConnector<crate::link::MemoryConnector>,
    crate::link::MemoryAcceptor,
> {
    let parts = link.into_parts();
    crate::link::LinkParts {
        control_read: parts.control_read,
        control_write: parts.control_write,
        connector: ScriptedConnector {
            inner: parts.connector,
            script,
        },
        acceptor: parts.acceptor,
        session: parts.session,
    }
    .into_link()
}

/// Drive the shared two-mirror topology over already-wrapped links.
async fn drive<LR, LW, LC, LA, RR, RW, RC, RA>(
    left: TreeRoot,
    right: TreeRoot,
    left_link: Link<LR, LW, LC, LA>,
    right_link: Link<RR, RW, RC, RA>,
    window: WindowConfig,
) -> (Result<TreeRoot, LeftError>, Result<TreeRoot, RightError>)
where
    LR: AsyncRead + Unpin + Send,
    LW: AsyncWrite + Unpin + Send,
    LC: Connector,
    LA: Acceptor,
    RR: AsyncRead + Unpin + Send,
    RW: AsyncWrite + Unpin + Send,
    RC: Connector,
    RA: Acceptor,
{
    let left = Handshaking::start(Local, Root::<Local, ()>::from(left)).window(window);
    let right = Handshaking::start(Local, Root::<Local, ()>::from(right)).window(window);
    let remote_right = RemoteHandshaking::start(Local, left_link).window(window);
    let remote_left = RemoteHandshaking::start(Local, right_link).window(window);
    let (left, right) = join!(
        Box::pin(mirror(left, remote_right)),
        Box::pin(mirror(remote_left, right)),
    );
    (
        left.map(|(root, _control)| root.into()),
        right.map(|(_control, root)| root.into()),
    )
}
