//! Reusable two-proxy session harness for transport-adversity properties.

use std::{
    convert::Infallible,
    io,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use futures::join;
use tokio::io::{AsyncRead, AsyncWrite, duplex, split};

use crate::testing::{IoPlan, IoReportHandle, IoSide, wrap_io};
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

/// Dense states occupied by the two nonempty-query flow variants.
const QUERY_STATES: std::ops::RangeInclusive<u8> = 4..=5;

/// Dense states below this boundary carry reactions rather than bare ends.
const REACTION_STATE_COUNT: u8 = 8;

/// Failure returned by the materialized-left/proxy-right driver.
pub type LeftError = MirrorError<MaterializedError<Infallible>, RemoteError<Infallible>>;

/// Failure returned by the proxy-left/materialized-right driver.
pub type RightError = MirrorError<RemoteError<Infallible>, MaterializedError<Infallible>>;

/// Both endpoint results and their physical-I/O observations.
pub struct Outcome {
    /// The first materialized tree, or its session failure.
    pub left: Result<TreeRoot<()>, LeftError>,
    /// The second materialized tree, or its session failure.
    pub right: Result<TreeRoot<()>, RightError>,
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

/// A writer which edits one complete frame at its flush boundary.
pub struct ScriptedWrite<W> {
    inner: W,
    script: Option<Script>,
    handshake: bool,
    frame: Vec<u8>,
    output: Vec<u8>,
    sent: usize,
}

impl<W> ScriptedWrite<W> {
    /// Wrap `inner`, applying `script` once if it reaches its selector.
    fn new(inner: W, script: Option<Script>) -> Self {
        Self {
            inner,
            script,
            handshake: true,
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
        // The first flush is the framed causal Version. Mutations target the
        // multiplexed codec which begins only after that protocol handshake.
        if self.handshake {
            return;
        }
        let Some(script) = &self.script else {
            return;
        };
        let mut script = script.0.lock().expect("frame script lock");
        if script.fired || self.frame.is_empty() {
            return;
        }
        let state = self.frame[0] / crate::tree::mirror::streaming::remote::codec::Stream::COUNT;
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
            FrameMutation::Signal(signal) => self.output[0] = signal,
            FrameMutation::Duplicate => self.output.extend_from_slice(&self.frame),
            FrameMutation::UnorderQuery => {
                const QUERY_HEADER: usize = 2;
                const QUERY_CHILD_LEN: usize = 1 + crate::tree::typed::hash::MERKLE_HASH_LEN;
                let first = self.output[QUERY_HEADER];
                if self.output[1] == 0 {
                    self.output[1] = 1;
                    self.output
                        .extend_from_within(QUERY_HEADER..QUERY_HEADER + QUERY_CHILD_LEN);
                } else {
                    self.output[QUERY_HEADER + QUERY_CHILD_LEN] = first;
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
                this.handshake = false;
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

/// Reconcile one pair through two proxies over independently wrapped endpoints.
pub async fn reconcile(
    left: TreeRoot<()>,
    right: TreeRoot<()>,
    capacity: usize,
    left_plan: IoPlan,
    right_plan: IoPlan,
) -> Outcome {
    let (left_transport, right_transport) = duplex(capacity.max(1));
    let (left_read, left_write) = split(left_transport);
    let (right_read, right_write) = split(right_transport);
    let (left_read, left_write, left_io) = wrap_io(IoSide::Left, left_plan, left_read, left_write);
    let (right_read, right_write, right_io) =
        wrap_io(IoSide::Right, right_plan, right_read, right_write);

    let (left, right) = drive(left, right, left_read, left_write, right_read, right_write).await;

    Outcome {
        left,
        right,
        left_io,
        right_io,
    }
}

/// Reconcile while mutating at most one flushed frame in each direction.
pub async fn reconcile_scripted(
    left: TreeRoot<()>,
    right: TreeRoot<()>,
    left_script: Option<Script>,
    right_script: Option<Script>,
) -> (
    Result<TreeRoot<()>, LeftError>,
    Result<TreeRoot<()>, RightError>,
) {
    let (left_transport, right_transport) = duplex(37);
    let (left_read, left_write) = split(left_transport);
    let (right_read, right_write) = split(right_transport);
    drive(
        left,
        right,
        left_read,
        ScriptedWrite::new(left_write, left_script),
        right_read,
        ScriptedWrite::new(right_write, right_script),
    )
    .await
}

/// Drive the shared two-mirror topology over already-wrapped transport halves.
async fn drive<LR, LW, RR, RW>(
    left: TreeRoot<()>,
    right: TreeRoot<()>,
    left_read: LR,
    left_write: LW,
    right_read: RR,
    right_write: RW,
) -> (
    Result<TreeRoot<()>, LeftError>,
    Result<TreeRoot<()>, RightError>,
)
where
    LR: AsyncRead + Unpin + Send,
    LW: AsyncWrite + Unpin + Send,
    RR: AsyncRead + Unpin + Send,
    RW: AsyncWrite + Unpin + Send,
{
    let left = Handshaking::start(Local, Root::from(left));
    let right = Handshaking::start(Local, Root::from(right));
    let remote_right = RemoteHandshaking::start(Local, left_read, left_write);
    let remote_left = RemoteHandshaking::start(Local, right_read, right_write);
    let (left, right) = join!(
        Box::pin(mirror(left, remote_right)),
        Box::pin(mirror(remote_left, right)),
    );
    (
        left.map(|(root, _transport)| root.into()),
        right.map(|(_transport, root)| root.into()),
    )
}
