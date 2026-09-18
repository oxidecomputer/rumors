//! Channel constructors for the materialized walk.
//!
//! Each function names one edge in the protocol dataflow. Keeping capacity
//! choices here makes them reviewable alongside the exact item type and keeps
//! queue arithmetic out of the walk itself. Every edge carries the erased
//! payload vocabulary — one channel-machinery instantiation per backend —
//! and keeps its height as the runtime [`QueueRole`] label the walk
//! threads through (the instrumented diagnostics and capacity tests key
//! on it). The one typed exception is [`leaf_requests`], whose item is
//! already the single-height [`Prefix<Z>`].
//!
//! Recursive query and resolution queues rely on two halves of the walk's
//! progress invariant: publish a scope's resolution before sending the work
//! that fulfills its `Pending` slots, then launch all such work before
//! publishing the enclosing parent resolution. That ordering makes one slot
//! *sufficient* for those queues — the liveness floor — but a one-slot edge
//! serializes the descent into a round trip per disputed scope, so the
//! recursive edges take their capacity from the session's
//! [`Window`](crate::tree::mirror::streaming::window::Window) instead. The
//! constructors below document the separate cardinality or flow argument
//! for every remaining one-slot edge; only the inter-level return boundary
//! needs a fan.

use futures::{StreamExt as _, stream};

use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        channel::{QueueKind, QueueRole, Receiver, ReceiverStream, Sender, channel, into_stream},
        erased::{self, Reply, ReplyResultStream},
        materialized::{Error, Query, Resolution},
        stats::Recorder,
        window::FAN,
    },
    typed::{
        Prefix,
        height::{Height, Root, S, UnderRoot, UnderUnderRoot, Z},
    },
};

/// A channel receiver stream whose items are wrapped in `Ok`.
pub(super) type OkReceiverStream<T, E> = stream::Map<ReceiverStream<T>, fn(T) -> Result<T, E>>;

/// Create a channel whose receiver presents every item as `Ok`.
fn ok_channel<T, E>(role: QueueRole, capacity: usize) -> (Sender<T>, OkReceiverStream<T, E>) {
    let (sender, receiver) = channel(role, capacity);
    (sender, into_stream(receiver).map(Ok))
}

/// A window-controlled sender that records capacity pressure.
pub(super) struct WindowSender<T> {
    /// The bounded protocol edge.
    inner: Sender<T>,
    /// The session receiving the pressure signal.
    stats: Recorder,
}

impl<T> Clone for WindowSender<T> {
    /// Share the channel and its session recorder.
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            stats: self.stats.clone(),
        }
    }
}

impl<T> WindowSender<T> {
    /// Wrap a sender whose capacity was derived from the session window.
    fn new(inner: Sender<T>, stats: &Recorder) -> Self {
        Self {
            inner,
            stats: stats.clone(),
        }
    }

    /// Send one item and record whether the window was full on arrival.
    pub(super) async fn send(&self, item: T) -> Result<(), tokio::sync::mpsc::error::SendError<T>> {
        if self.inner.capacity() == 0 {
            self.stats.window_stall();
        }
        self.inner.send(item).await
    }
}

/// Buffer outgoing protocol replies one at a time.
///
/// A blocked producer has already made one reply available to the counterparty,
/// and consuming that reply is sufficient to release the producer. More slots
/// retain whole messages without breaking another dependency.
pub(super) fn outgoing_responses<B, H>() -> (
    Sender<Result<Reply<B::Erased>, Error<B::Error>>>,
    ReplyResultStream<B, H, Error<B::Error>>,
)
where
    B: Backend<Node<Z>: Leaf>,
    H: Height,
{
    erased::reply_channel::<B, H, Error<B::Error>>(
        QueueRole::new(QueueKind::OutgoingResponses, H::HEIGHT),
        1,
    )
}

/// Buffer lower-level completions until their enclosing resolution arrives.
///
/// `height` is the completions' own height, the level boundary's label.
///
/// Processing one incoming reply can launch a full fan of disputed child
/// scopes. Their lower assemblers may finish immediately and send completed
/// nodes here, but this queue's consumer first waits for the enclosing parent
/// resolution: only its ordered `Pending` slots tell the assembler to drain
/// those nodes. The walk cannot construct and publish that resolution until it
/// has processed every reaction in the reply.
///
/// Capacity `FAN` therefore lets every child completion enqueue while the walk
/// finishes the reaction loop. A smaller queue can sometimes progress because
/// blocked sends live in independently driven work futures, but correctness
/// would then depend on that incidental scheduling slack. Once the parent
/// resolution arrives, assembly drains the completions in order, so the bound
/// does not multiply with tree width or depth.
///
/// **One full fan is this edge's hard floor, not a tunable.** Unlike the
/// window-scaled edges, whose one-slot floor is deadlock-free by the
/// ordering invariants, shrinking this queue below `FAN` can
/// stall a session (`underbuffered_mirror_stalls` in the capacity tests
/// demonstrates it), which is why the session window deliberately never
/// reaches this constructor.
pub(super) fn assembly_level_returns<B>(
    height: usize,
) -> (
    Sender<Option<B::Erased>>,
    OkReceiverStream<Option<B::Erased>, Error<B::Error>>,
)
where
    B: Backend<Node<Z>: Leaf>,
{
    ok_channel(QueueRole::new(QueueKind::AssemblyLevelReturns, height), FAN)
}

/// Carry the initiator's single root query.
///
/// The opening emits exactly one query for the root scope, so a second slot
/// can never be occupied.
pub(super) fn initiator_root_query<B>() -> (Sender<Query<B::Erased>>, Receiver<Query<B::Erased>>)
where
    B: Backend<Node<Z>: Leaf>,
{
    channel(
        QueueRole::new(QueueKind::InitiatorRootQuery, UnderRoot::HEIGHT),
        1,
    )
}

/// Carry the initiator's single completed root.
///
/// Reconciliation produces exactly one root node and the terminal future
/// consumes it directly.
pub(super) fn initiator_root_return<B>() -> (Sender<Option<B::Erased>>, Receiver<Option<B::Erased>>)
where
    B: Backend<Node<Z>: Leaf>,
{
    channel(
        QueueRole::new(QueueKind::InitiatorRootReturn, Root::HEIGHT),
        1,
    )
}

/// Stream the responder opening's child queries through the window.
///
/// The opening wire reply and root resolution are published before these
/// queries, so one slot is the liveness floor. The window widens it so the next
/// stage can hold a pipeline of disputed children in flight; each buffered
/// [`Query`] may own a fan of node handles, which is priced by the window's
/// node budget.
pub(super) fn responder_child_queries<B>(
    capacity: usize,
    stats: &Recorder,
) -> (WindowSender<Query<B::Erased>>, Receiver<Query<B::Erased>>)
where
    B: Backend<Node<Z>: Leaf>,
{
    let (send, receive) = channel(
        QueueRole::new(QueueKind::ResponderChildQueries, UnderUnderRoot::HEIGHT),
        capacity,
    );
    (WindowSender::new(send, stats), receive)
}

/// Carry the responder's single root resolution.
///
/// The responder processes exactly one opening request and therefore
/// publishes exactly one resolution for the root scope.
pub(super) fn responder_root_resolution<B>() -> (
    Sender<Resolution<B::Erased>>,
    OkReceiverStream<Resolution<B::Erased>, Error<B::Error>>,
)
where
    B: Backend<Node<Z>: Leaf>,
{
    ok_channel(
        QueueRole::new(QueueKind::ResponderRootResolution, UnderRoot::HEIGHT),
        1,
    )
}

/// Buffer the responder opening's completed child scopes.
///
/// The root resolution is visible before its child queries are sent, so its
/// assembler can consume each return as it arrives. No later return is needed
/// to unlock the consumer of the buffered one.
pub(super) fn responder_root_returns<B>() -> (
    Sender<Option<B::Erased>>,
    OkReceiverStream<Option<B::Erased>, Error<B::Error>>,
)
where
    B: Backend<Node<Z>: Leaf>,
{
    ok_channel(
        QueueRole::new(QueueKind::ResponderRootReturns, UnderRoot::HEIGHT),
        1,
    )
}

/// Buffer the child queries emitted by one internal walk, window-wide.
///
/// `height` is the children's height: the walk's dependent queries descend
/// to it, and it labels the edge.
///
/// The corresponding child resolution is published first, so one slot is the
/// liveness floor. This queue is the in-flight question window itself: its
/// occupancy is the number of disputed scopes awaiting wire replies at this
/// height, so its capacity is what lets sibling scopes' round trips overlap.
pub(super) fn internal_child_queries<B>(
    height: usize,
    capacity: usize,
    stats: &Recorder,
) -> (WindowSender<Query<B::Erased>>, Receiver<Query<B::Erased>>)
where
    B: Backend<Node<Z>: Leaf>,
{
    let (send, receive) = channel(
        QueueRole::new(QueueKind::InternalChildQueries, height),
        capacity,
    );
    (WindowSender::new(send, stats), receive)
}

/// Buffer parent-scope resolutions produced by an internal walk, window-wide.
///
/// `height` is the parent resolutions' own height (two above the walk's
/// dependent queries).
///
/// Before each parent resolution is sent, all work capable of fulfilling its
/// `Pending` slots has been launched, so one slot is the liveness floor. But a
/// resolution is consumed only as its subtree completes, so a one-slot edge
/// stalls the walk two scopes in; the window lets it run ahead.
pub(super) fn internal_parent_resolutions<B>(
    height: usize,
    capacity: usize,
    stats: &Recorder,
) -> (
    WindowSender<Resolution<B::Erased>>,
    OkReceiverStream<Resolution<B::Erased>, Error<B::Error>>,
)
where
    B: Backend<Node<Z>: Leaf>,
{
    let (send, receive) = ok_channel(
        QueueRole::new(QueueKind::InternalParentResolutions, height),
        capacity,
    );
    (WindowSender::new(send, stats), receive)
}

/// Buffer child-scope resolutions produced by an internal walk, window-wide.
///
/// `height` is the child resolutions' own height (one above the walk's
/// dependent queries).
///
/// Each resolution is published before its corresponding child queries, so one
/// slot is the liveness floor; the window lets the walk publish a pipeline of
/// them while earlier subtrees are still reconciling.
pub(super) fn internal_child_resolutions<B>(
    height: usize,
    capacity: usize,
    stats: &Recorder,
) -> (
    WindowSender<Resolution<B::Erased>>,
    OkReceiverStream<Resolution<B::Erased>, Error<B::Error>>,
)
where
    B: Backend<Node<Z>: Leaf>,
{
    let (send, receive) = ok_channel(
        QueueRole::new(QueueKind::InternalChildResolutions, height),
        capacity,
    );
    (WindowSender::new(send, stats), receive)
}

/// Buffer the leaf requests emitted by a leaf-parent walk, window-wide.
///
/// The corresponding leaf-scope resolution is published first, so one slot is
/// the liveness floor. This queue is the leaf-height question window: its
/// capacity is how many requested leaves may await the peer's supplies at once.
pub(super) fn leaf_requests(
    capacity: usize,
    stats: &Recorder,
) -> (WindowSender<Prefix<Z>>, Receiver<Prefix<Z>>) {
    let (send, receive) = channel(QueueRole::new(QueueKind::LeafRequests, Z::HEIGHT), capacity);
    (WindowSender::new(send, stats), receive)
}

/// Buffer leaf-parent resolutions awaiting their reconstructed children,
/// window-wide.
///
/// All terminal work for a parent resolution has been launched before it is
/// sent — the one-slot liveness floor; the window lets the walk run ahead while
/// buffered resolutions wait on their leaf exchanges.
pub(super) fn leaf_parent_resolutions<B>(
    capacity: usize,
    stats: &Recorder,
) -> (
    WindowSender<Resolution<B::Erased>>,
    OkReceiverStream<Resolution<B::Erased>, Error<B::Error>>,
)
where
    B: Backend<Node<Z>: Leaf>,
{
    let (send, receive) = ok_channel(
        QueueRole::new(QueueKind::LeafParentResolutions, <S<Z>>::HEIGHT),
        capacity,
    );
    (WindowSender::new(send, stats), receive)
}

/// Buffer leaf-scope resolutions produced within one leaf-parent reply,
/// window-wide.
///
/// Each resolution is published before its leaf requests — the one-slot
/// liveness floor; the window keeps the walk publishing while earlier leaf
/// scopes await their supplies.
pub(super) fn leaf_child_resolutions<B>(
    capacity: usize,
    stats: &Recorder,
) -> (
    WindowSender<Resolution<B::Erased>>,
    OkReceiverStream<Resolution<B::Erased>, Error<B::Error>>,
)
where
    B: Backend<Node<Z>: Leaf>,
{
    let (send, receive) = ok_channel(
        QueueRole::new(QueueKind::LeafChildResolutions, Z::HEIGHT),
        capacity,
    );
    (WindowSender::new(send, stats), receive)
}

/// Stream terminal leaf resolutions, buffered one fan deep.
///
/// Terminal resolutions contain no `Pending` slots, so leaf assembly can
/// consume each immediately; no later item is required to unlock its consumer,
/// and one slot is the liveness floor. But the walk produces one resolution per
/// requested leaf, so a one-slot edge pays a waker round trip per leaf on the
/// compute path. One fan amortizes that; unlike the window edges these items
/// are single-leaf resolutions belonging to scopes the memory model already
/// charges, so no window capacity applies. (Contrast
/// [`assembly_level_returns`], where one fan is a correctness floor rather than
/// an amortization.)
pub(super) fn terminal_leaf_resolutions<B>() -> (
    Sender<Resolution<B::Erased>>,
    OkReceiverStream<Resolution<B::Erased>, Error<B::Error>>,
)
where
    B: Backend<Node<Z>: Leaf>,
{
    ok_channel(
        QueueRole::new(QueueKind::TerminalLeafResolutions, Z::HEIGHT),
        FAN,
    )
}
