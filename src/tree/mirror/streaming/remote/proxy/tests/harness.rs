//! Shared two-proxy test support: session drivers, link decorators, greeting
//! rewrites, frame scripts, fixtures, and role-sensitive result inspection.

use std::{
    convert::Infallible,
    fmt, io,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
};

use futures::join;
use tokio::io::{AsyncRead, AsyncWrite, DuplexStream, ReadBuf};

use serde::{Serialize, de::DeserializeOwned};

use crate::link::{
    Acceptor, Connector, Done, Link, MemoryAcceptor, MemoryConnector, MemoryLink,
    memory_with_capacity,
};
use crate::testing::{IoFault, IoPlan, IoReportHandle, IoSide, wrap_link};
use crate::tree::mirror::cbor;
use crate::tree::mirror::streaming::window::WindowConfig;
use crate::tree::typed::height::Z;
use crate::tree::{
    Action, Root as TreeRoot, Tree,
    arb::nth_party,
    mirror::{
        Error as MirrorError,
        streaming::{
            Backend, Failing, FailingNode, Leaf, Local, Root,
            materialized::{Error as MaterializedError, Handshaking},
            message::RoleKey,
            mirror,
            remote::{
                Error as RemoteError, Handshaking as RemoteHandshaking,
                codec::{Flow, Signal},
            },
        },
    },
};
use crate::{
    DEFAULT_TARGET_MESSAGE_SIZE,
    message::{Message, PayloadCodec, PayloadDepthLimit},
};

/// Bytes buffered by each per-stream pipe before backpressure applies.
pub const TRANSPORT_CAPACITY: usize = 37;

/// One endpoint's session failure, named by the participant that raised
/// it.
///
/// `MirrorError` names protocol positions; which position a participant
/// holds is the [`Topology`]'s choice, so the harness reports by
/// participant and the topology alone says who was client and who was
/// server.
#[derive(Debug)]
pub enum EndpointError<E> {
    /// The endpoint's materialized participant failed.
    Local(MaterializedError<E>),
    /// The endpoint's proxy failed.
    Proxy(RemoteError<E>),
}

/// An endpoint failure over the infallible `Local` backend.
pub type EndpointFailure = EndpointError<Infallible>;

/// How each endpoint pairs its materialized participant with its proxy.
#[derive(Clone, Copy)]
pub enum Topology {
    /// What `Peer` runs: each materialized participant is the client of
    /// its own proxy, so both proxies accept and exchange greetings
    /// concurrently.
    Production,
    /// The right endpoint's proxy connects and its materialized
    /// participant accepts: the arrangement that pins the materialized
    /// participant's behavior in the server position.
    RightProxyConnects,
}

/// A backend whose roots convert to and from `tree::Root`: `Local`, and
/// the failing wrapper over it.
pub trait TreeBackend: Backend<Node<Z>: Leaf> + Clone + Send + Sync + 'static {
    /// Wrap a tree root in this backend's node type.
    fn lift(root: TreeRoot) -> Root<Self>;
    /// Unwrap a reconciled root back to the tree's.
    fn lower(root: Root<Self>) -> TreeRoot;
}

/// Adapt ordinary local nodes to the proxy test harness.
impl TreeBackend for Local {
    /// Wrap an ordinary root without changing it.
    fn lift(root: TreeRoot) -> Root<Self> {
        root.into()
    }

    /// Unwrap an ordinary root without changing it.
    fn lower(root: Root<Self>) -> TreeRoot {
        root.into()
    }
}

/// Adapt operation-failing local nodes to the proxy test harness.
impl TreeBackend for Failing<Local> {
    /// Wrap every present root node with the failing backend.
    fn lift(root: TreeRoot) -> Root<Self> {
        Root {
            ceiling: root.ceiling,
            root: root.node.map(FailingNode::new),
        }
    }

    /// Remove the failing wrapper from every present root node.
    fn lower(root: Root<Self>) -> TreeRoot {
        TreeRoot {
            ceiling: root.ceiling,
            node: root.root.map(FailingNode::into_inner),
        }
    }
}

/// The four participants' backends: each endpoint's materialized
/// participant and the proxy beside it.
pub struct Backends<B> {
    /// The left materialized participant.
    pub left: B,
    /// The proxy paired with the left participant.
    pub left_proxy: B,
    /// The right materialized participant.
    pub right: B,
    /// The proxy paired with the right participant.
    pub right_proxy: B,
}

/// Construct the ordinary in-memory backend arrangement.
impl Backends<Local> {
    /// Every participant on the infallible in-memory backend.
    pub fn local() -> Self {
        Self {
            left: Local,
            left_proxy: Local,
            right: Local,
            right_proxy: Local,
        }
    }
}

/// The payload codec for `T` at the default depth limit: the one every
/// proxy in the partition decodes through.
pub fn codec<T>() -> PayloadCodec
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    PayloadCodec::new::<T>(PayloadDepthLimit::default())
}

/// Return the complete root produced by the in-memory join oracle.
pub fn join_oracle(left: &TreeRoot, right: &TreeRoot) -> TreeRoot {
    let mut joined = Tree::<()>::from_root(left.clone());
    joined.join(Tree::from_root(right.clone()));
    joined.root
}

/// Build two trees from disjoint parties, each containing the requested
/// number of unit messages.
pub fn disjoint_pair(left_messages: usize, right_messages: usize) -> (TreeRoot, TreeRoot) {
    let build = |party, messages| {
        let mut tree = Tree::<()>::new();
        tree.act(
            &nth_party(party),
            (0..messages).map(|_| Action::Insert(Message::new(()))),
        );
        tree.root
    };
    (build(0, left_messages), build(1, right_messages))
}

/// Build an I/O plan shared by successful and fault-injection properties.
pub fn io_plan(
    read_chunk: usize,
    write_chunk: usize,
    delays: Vec<u8>,
    hold_until_flush: bool,
    fault: Option<IoFault>,
) -> IoPlan {
    IoPlan {
        read_chunk,
        write_chunk,
        read_delays: delays.clone(),
        write_delays: delays.clone(),
        flush_delays: delays,
        hold_until_flush,
        fault,
    }
}

/// Order a pair so the elected initiator comes first.
pub fn order_by_election(left: TreeRoot, right: TreeRoot) -> (TreeRoot, TreeRoot) {
    if left_initiates(&left, &right) {
        (left, right)
    } else {
        (right, left)
    }
}

/// Return the proxy error reported by one physical endpoint.
pub fn proxy_error<'a, E: fmt::Debug>(
    side: IoSide,
    left: &'a Result<TreeRoot, EndpointError<E>>,
    right: &'a Result<TreeRoot, EndpointError<E>>,
) -> Result<&'a RemoteError<E>, String> {
    let result = match side {
        IoSide::Left => left,
        IoSide::Right => right,
    };
    match result {
        Err(EndpointError::Proxy(error)) => Ok(error),
        other => Err(format!("{side:?} proxy did not report an error: {other:?}")),
    }
}

/// Both endpoint results and their physical-I/O observations.
pub struct Outcome {
    /// The first materialized tree, or its session failure.
    pub left: Result<TreeRoot, EndpointFailure>,
    /// The second materialized tree, or its session failure.
    pub right: Result<TreeRoot, EndpointFailure>,
    /// I/O performed by the first proxy endpoint.
    pub left_io: IoReportHandle,
    /// I/O performed by the second proxy endpoint.
    pub right_io: IoReportHandle,
}

/// A complete frame selected by its signal state code.
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
    /// Replace only the state item, retaining the stream item and body.
    State(u8),
    /// Emit the complete frame twice before its flush completes.
    Duplicate,
    /// Make the second query radix duplicate the first.
    UnorderQuery,
}

/// Mutable state shared by the streams carrying one frame script.
struct ScriptState {
    /// Which frame should be changed.
    selector: FrameSelector,
    /// How the selected frame should be changed.
    mutation: FrameMutation,
    /// Whether a matching frame has already been changed.
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
/// the label items ahead of its frame; mutations parse past them and leave
/// them intact.
///
/// [`StreamSender`]: crate::tree::mirror::streaming::remote::streams::StreamSender
pub struct ScriptedWrite<W> {
    /// The stream that receives the resulting bytes.
    inner: W,
    /// The shared mutation, while it remains unfired.
    script: Option<Script>,
    /// Whether the next flush still carries the label items ahead of its
    /// frame.
    label: bool,
    /// Bytes buffered for the current frame.
    frame: Vec<u8>,
    /// Bytes to write after applying the script.
    output: Vec<u8>,
    /// Bytes already written from `output`.
    sent: usize,
}

/// Construct and prepare a scripted data-stream writer.
impl<W> ScriptedWrite<W> {
    /// Wrap one stream's `inner`, applying `script` once if it reaches its
    /// selector.
    fn new(inner: W, script: Option<Script>) -> Self {
        Self {
            inner,
            script,
            label: true,
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
        let Some(script) = &self.script else {
            return;
        };
        let mut script = script.0.lock().expect("frame script lock");
        if script.fired {
            return;
        }
        // Locate the frame behind any leading label items, then parse its
        // array head, stream item, and state item through the wire's own
        // head grammar.
        let mut rest = self.frame.as_slice();
        if self.label {
            for _ in 0..2 {
                if cbor::read_head(&mut rest).is_err() {
                    return;
                }
            }
        }
        let frame_at = self.frame.len() - rest.len();
        if cbor::read_head(&mut rest).is_err() || cbor::read_head(&mut rest).is_err() {
            return;
        }
        let state_at = self.frame.len() - rest.len();
        let Ok(state) = cbor::read_head(&mut rest) else {
            return;
        };
        let body_at = self.frame.len() - rest.len();
        let Ok(state) = u8::try_from(state.value) else {
            return;
        };
        let selected = match script.selector {
            FrameSelector::First => true,
            FrameSelector::State(expected) => state == expected,
            FrameSelector::Query => matches!(Signal::from_state(state), Ok(Signal::Query(_))),
            FrameSelector::EndingReaction => matches!(
                Signal::from_state(state),
                Ok(Signal::Match(Flow::End)
                    | Signal::QueryEmpty(Flow::End)
                    | Signal::Query(Flow::End)
                    | Signal::Supply(Flow::End))
            ),
        };
        if !selected {
            return;
        }
        match script.mutation {
            FrameMutation::State(state) => {
                let mut head = Vec::new();
                cbor::write_head(&mut head, cbor::MAJOR_UINT, u64::from(state));
                self.output.splice(state_at..body_at, head);
            }
            FrameMutation::Duplicate => self.output.extend_from_slice(&self.frame[frame_at..]),
            FrameMutation::UnorderQuery => {
                use crate::tree::mirror::streaming::remote::codec::{
                    parse_listing_map, write_listing,
                };
                let mut listing_input = &self.frame[body_at..];
                let Ok(mut children) = parse_listing_map(&mut listing_input) else {
                    return;
                };
                if children.len() == 1 {
                    // A duplicated single child is the equal-pair
                    // violation, exactly like a descent.
                    children.push(children[0]);
                } else {
                    children[1].0 = children[0].0;
                }
                let mut listing = Vec::new();
                write_listing(&mut listing, &children);
                let listing_end = self.frame.len() - listing_input.len();
                self.output.splice(body_at..listing_end, listing);
            }
        }
        script.fired = true;
    }
}

/// Buffer writes by frame while preserving ordinary async-write behavior.
impl<W: AsyncWrite + Unpin> AsyncWrite for ScriptedWrite<W> {
    /// Buffer bytes until the frame's flush boundary reveals its full shape.
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.get_mut().frame.extend_from_slice(bytes);
        Poll::Ready(Ok(bytes.len()))
    }

    /// Apply the script, then flush the resulting complete frame.
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
                this.label = false;
                this.frame.clear();
                this.output.clear();
                this.sent = 0;
                Poll::Ready(Ok(()))
            }
            other => other,
        }
    }

    /// Flush the pending frame before closing the wrapped stream.
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
    /// The connector that opens the underlying stream.
    inner: C,
    /// The mutation shared by every opened stream.
    script: Option<Script>,
}

/// Preserve the underlying connector's completion callback through the wrapper.
impl<C: Connector> Connector for ScriptedConnector<C> {
    type Tx = ScriptedWrite<C::Tx>;

    /// Wrap an outgoing stream and return its underlying half on completion.
    async fn connect(&self) -> io::Result<(Self::Tx, Done<Self::Tx>)> {
        let (tx, done) = self.inner.connect().await?;
        Ok((
            ScriptedWrite::new(tx, self.script.clone()),
            Done::new(move |tx: Self::Tx| done.complete(tx.inner)),
        ))
    }
}

/// One greeting size declaration replaced in the traffic a side receives.
///
/// Rewriting the *received* greeting simulates a non-conforming peer whose
/// declaration disagrees with the traffic it then sends: the receiving side
/// negotiates against the rewritten value while the sender behaves per its
/// unchanged tree. The greeting is the first control traffic at this layer,
/// so the rewriter buffers the one item, encodes it again with the field
/// replaced, and passes everything after it through untouched.
#[derive(Clone)]
pub struct GreetingRewrite {
    /// Whether this rewrite reached a complete greeting.
    fired: Arc<AtomicBool>,
    /// Which declaration to replace.
    field: GreetingField,
    /// The declaration the receiving side decodes instead of the encoded one.
    value: u64,
}

#[derive(Clone, Copy)]
/// The size declaration replaced in a received greeting.
enum GreetingField {
    /// The peer's message count.
    SetLen,
    /// The largest encoded version the peer may send.
    MaxVersionBytes,
    /// The peer's target encoded message size.
    TargetMessageSize,
}

impl GreetingRewrite {
    /// Configure one declaration rewrite with a shared observation flag.
    fn new(field: GreetingField, value: u64) -> Self {
        Self {
            fired: Arc::new(AtomicBool::new(false)),
            field,
            value,
        }
    }

    /// Rewrite the received greeting's `set_len` entry.
    pub fn set_len(value: u64) -> Self {
        Self::new(GreetingField::SetLen, value)
    }

    /// Rewrite the received greeting's `max_version_bytes` entry.
    pub fn max_version_bytes(value: u64) -> Self {
        Self::new(GreetingField::MaxVersionBytes, value)
    }

    /// Rewrite the received greeting's `target_message_size` entry.
    pub fn target_message_size(value: u64) -> Self {
        Self::new(GreetingField::TargetMessageSize, value)
    }

    /// Return whether a complete greeting was rewritten.
    pub fn fired(&self) -> bool {
        self.fired.load(Ordering::Relaxed)
    }

    /// Encode one buffered greeting item again with this rewrite applied.
    fn apply(&self, item: &[u8]) -> Vec<u8> {
        use crate::tree::mirror::streaming::remote::codec::greeting::{
            encode_greeting, parse_greeting,
        };
        let mut input = item;
        cbor::read_head(&mut input).expect("a complete greeting item has its tag head");
        cbor::read_head(&mut input).expect("a complete greeting item has its string head");
        let mut greeting = parse_greeting(input).expect("the harness rewrites a valid greeting");
        match self.field {
            GreetingField::SetLen => greeting.set_len = self.value,
            GreetingField::MaxVersionBytes => greeting.max_version_bytes = self.value,
            GreetingField::TargetMessageSize => greeting.target_message_size = self.value,
        }
        self.fired.store(true, Ordering::Relaxed);
        encode_greeting(&greeting)
    }
}

/// A control-stream reader replacing one greeting declaration, robust to
/// arbitrary read chunking: it buffers the greeting item, serves the
/// rewritten bytes, and passes the rest of the stream through.
pub struct RewriteRead<R> {
    /// The control stream being read.
    inner: R,
    /// The current rewrite phase.
    state: RewriteState,
}

/// Progress through buffering and replacing the greeting at stream start.
enum RewriteState {
    /// Accumulating the greeting item's bytes.
    Buffering {
        /// The replacement to apply once the greeting is complete.
        rewrite: GreetingRewrite,
        /// Greeting bytes received so far.
        pending: Vec<u8>,
    },
    /// Serving the re-spelled bytes ahead of the untouched stream.
    Serving {
        /// Rewritten and trailing bytes awaiting delivery.
        bytes: Vec<u8>,
        /// The next byte to deliver.
        at: usize,
    },
    /// Everything further passes through.
    PassThrough,
}

impl<R> RewriteRead<R> {
    /// Wrap a control reader with an optional greeting rewrite.
    fn new(inner: R, rewrite: Option<GreetingRewrite>) -> Self {
        Self {
            inner,
            state: match rewrite {
                Some(rewrite) => RewriteState::Buffering {
                    rewrite,
                    pending: Vec::new(),
                },
                None => RewriteState::PassThrough,
            },
        }
    }
}

/// Bytes of a complete greeting item at the front of `pending`, when its
/// heads have arrived whole.
fn greeting_item_len(pending: &[u8]) -> Option<usize> {
    let mut input = pending;
    cbor::read_head(&mut input).ok()?;
    let body = cbor::read_head(&mut input).ok()?;
    let heads = pending.len() - input.len();
    Some(heads + usize::try_from(body.value).ok()?)
}

/// Replace the greeting before forwarding subsequent control bytes.
impl<R: AsyncRead + Unpin> AsyncRead for RewriteRead<R> {
    /// Buffer through the greeting, then serve the rewrite and remaining bytes.
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        loop {
            match &mut this.state {
                RewriteState::PassThrough => {
                    return Pin::new(&mut this.inner).poll_read(cx, buf);
                }
                RewriteState::Serving { bytes, at } => {
                    let take = (bytes.len() - *at).min(buf.remaining());
                    buf.put_slice(&bytes[*at..*at + take]);
                    *at += take;
                    if *at == bytes.len() {
                        this.state = RewriteState::PassThrough;
                    }
                    return Poll::Ready(Ok(()));
                }
                RewriteState::Buffering { rewrite, pending } => {
                    let mut chunk = [0u8; 4096];
                    let mut chunk = ReadBuf::new(&mut chunk);
                    match Pin::new(&mut this.inner).poll_read(cx, &mut chunk) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                        Poll::Ready(Ok(())) => {
                            let filled = chunk.filled();
                            if filled.is_empty() {
                                // Closed before a whole greeting: hand the
                                // raw bytes through so truncation surfaces
                                // exactly as sent.
                                let bytes = std::mem::take(pending);
                                this.state = RewriteState::Serving { bytes, at: 0 };
                                continue;
                            }
                            pending.extend_from_slice(filled);
                            if let Some(len) = greeting_item_len(pending)
                                && pending.len() >= len
                            {
                                let mut bytes = rewrite.apply(&pending[..len]);
                                bytes.extend_from_slice(&pending[len..]);
                                this.state = RewriteState::Serving { bytes, at: 0 };
                            }
                            continue;
                        }
                    }
                }
            }
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
    let (left_link, right_link) = memory_with_capacity(capacity);
    let (left_link, left_io) = wrap_link(IoSide::Left, left_plan, left_link);
    let (right_link, right_io) = wrap_link(IoSide::Right, right_plan, right_link);

    let (left, right) = drive(
        Topology::Production,
        Backends::local(),
        left,
        right,
        left_link,
        right_link,
        codec::<()>(),
        WindowConfig::FLOOR,
    )
    .await;

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
) -> (
    Result<TreeRoot, EndpointFailure>,
    Result<TreeRoot, EndpointFailure>,
) {
    let (left_link, right_link) = memory_with_capacity(TRANSPORT_CAPACITY);
    drive(
        Topology::Production,
        Backends::local(),
        left,
        right,
        rewritten(left_link, left_hears),
        rewritten(right_link, right_hears),
        codec::<()>(),
        WindowConfig::default(),
    )
    .await
}

/// Reconcile after changing the greeting one selected endpoint receives.
pub async fn reconcile_rewritten_for(
    receiver: IoSide,
    receiver_root: TreeRoot,
    sender_root: TreeRoot,
    rewrite: GreetingRewrite,
) -> (
    Result<TreeRoot, EndpointFailure>,
    Result<TreeRoot, EndpointFailure>,
) {
    match receiver {
        IoSide::Left => {
            reconcile_rewritten_greetings(receiver_root, sender_root, Some(rewrite), None).await
        }
        IoSide::Right => {
            reconcile_rewritten_greetings(sender_root, receiver_root, None, Some(rewrite)).await
        }
    }
}

/// Wrap one link's control-read half in a greeting-word rewriter.
pub fn rewritten(
    link: MemoryLink,
    rewrite: Option<GreetingRewrite>,
) -> Link<RewriteRead<DuplexStream>, DuplexStream, MemoryConnector, MemoryAcceptor> {
    link.map_transport(|control_read, control_write, connector, acceptor| {
        (
            RewriteRead::new(control_read, rewrite),
            control_write,
            connector,
            acceptor,
        )
    })
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
    let key = |root: &TreeRoot| RoleKey::new(root.len() as u64, root.ceiling.clone());
    key(left).initiates(&key(right))
}

/// Reconcile while mutating at most one data-stream frame on each side.
pub async fn reconcile_scripted(
    left: TreeRoot,
    right: TreeRoot,
    left_script: Option<Script>,
    right_script: Option<Script>,
) -> (
    Result<TreeRoot, EndpointFailure>,
    Result<TreeRoot, EndpointFailure>,
) {
    let (left_link, right_link) = memory_with_capacity(TRANSPORT_CAPACITY);
    drive(
        Topology::Production,
        Backends::local(),
        left,
        right,
        scripted(left_link, left_script),
        scripted(right_link, right_script),
        codec::<()>(),
        WindowConfig::FLOOR,
    )
    .await
}

/// Wrap one link's outgoing data streams with a frame script.
pub fn scripted<R, W, C, A>(
    link: Link<R, W, C, A>,
    script: Option<Script>,
) -> Link<R, W, ScriptedConnector<C>, A>
where
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
    C: Connector,
    A: Acceptor,
{
    link.map_transport(|control_read, control_write, connector, acceptor| {
        (
            control_read,
            control_write,
            ScriptedConnector {
                inner: connector,
                script,
            },
            acceptor,
        )
    })
}

/// Drive two proxy endpoints over already-wrapped links.
///
/// `left_link` carries the left endpoint, whose proxy represents the right
/// peer; `right_link` the reverse. Every result is the endpoint's
/// materialized tree or the failure of whichever of its two participants
/// raised one.
// One premise per argument: the topology, the four backends, the two
// roots, the two links, the codec, and the window.
#[allow(clippy::too_many_arguments)]
pub async fn drive<B, LR, LW, LC, LA, RR, RW, RC, RA>(
    topology: Topology,
    backends: Backends<B>,
    left: TreeRoot,
    right: TreeRoot,
    left_link: Link<LR, LW, LC, LA>,
    right_link: Link<RR, RW, RC, RA>,
    codec: PayloadCodec,
    window: WindowConfig,
) -> (
    Result<TreeRoot, EndpointError<B::Error>>,
    Result<TreeRoot, EndpointError<B::Error>>,
)
where
    B: TreeBackend,
    LR: AsyncRead + Unpin + Send,
    LW: AsyncWrite + Unpin + Send,
    LC: Connector,
    LA: Acceptor,
    RR: AsyncRead + Unpin + Send,
    RW: AsyncWrite + Unpin + Send,
    RC: Connector,
    RA: Acceptor,
{
    let Backends {
        left: left_backend,
        left_proxy,
        right: right_backend,
        right_proxy,
    } = backends;
    let left = Handshaking::start(
        left_backend,
        B::lift(left),
        DEFAULT_TARGET_MESSAGE_SIZE as u64,
    )
    .window(window);
    let right = Handshaking::start(
        right_backend,
        B::lift(right),
        DEFAULT_TARGET_MESSAGE_SIZE as u64,
    )
    .window(window);
    let remote_right = RemoteHandshaking::start(left_proxy, left_link, codec).window(window);
    let remote_left = RemoteHandshaking::start(right_proxy, right_link, codec).window(window);

    // The left endpoint's materialized participant is the client of its
    // proxy in both arrangements; the topology decides the right one.
    let left = mirror(left, remote_right);
    match topology {
        Topology::Production => {
            let (left, right) = join!(Box::pin(left), Box::pin(mirror(right, remote_left)));
            (local_client(left), local_client(right))
        }
        Topology::RightProxyConnects => {
            let (left, right) = join!(Box::pin(left), Box::pin(mirror(remote_left, right)));
            (local_client(left), local_server(right))
        }
    }
}

/// Name an endpoint result whose materialized participant was the client.
fn local_client<B: TreeBackend, W>(
    result: Result<(Root<B>, W), MirrorError<MaterializedError<B::Error>, RemoteError<B::Error>>>,
) -> Result<TreeRoot, EndpointError<B::Error>> {
    match result {
        Ok((root, _control)) => Ok(B::lower(root)),
        Err(MirrorError::Client(error)) => Err(EndpointError::Local(error)),
        Err(MirrorError::Server(error)) => Err(EndpointError::Proxy(error)),
    }
}

/// Name an endpoint result whose materialized participant was the server.
fn local_server<B: TreeBackend, W>(
    result: Result<(W, Root<B>), MirrorError<RemoteError<B::Error>, MaterializedError<B::Error>>>,
) -> Result<TreeRoot, EndpointError<B::Error>> {
    match result {
        Ok((_control, root)) => Ok(B::lower(root)),
        Err(MirrorError::Client(error)) => Err(EndpointError::Proxy(error)),
        Err(MirrorError::Server(error)) => Err(EndpointError::Local(error)),
    }
}
