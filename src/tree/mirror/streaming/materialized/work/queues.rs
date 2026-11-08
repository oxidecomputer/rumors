//! Channel constructors for the materialized walk.
//!
//! Each function names one edge in the protocol dataflow. Keeping capacity
//! choices here makes them reviewable alongside the exact item type and keeps
//! queue arithmetic out of the walk itself. Every edge carries the erased
//! payload vocabulary — one channel-machinery instantiation per backend — and
//! keeps its height as the runtime [`QueueRole`] label the walk threads through
//! (the instrumented diagnostics and capacity tests key on it). The one typed
//! exception is [`leaf_requests`], whose item is a leaf's single-height path
//! with its charge ([`LeafRequest`]).
//!
//! Recursive query and resolution queues rely on two halves of the walk's
//! progress invariant: publish a scope's resolution before sending the work
//! that fulfills its `Pending` slots, then launch all such work before
//! publishing the enclosing parent resolution. That ordering makes one slot
//! *sufficient* for those queues — the liveness floor — but a bounded edge only
//! slows the walk: everything they hold is charged on the endpoint's ledger
//! when admission grants it
//! ([`ledger`](crate::tree::mirror::streaming::ledger)), so they are
//! [`UNBOUNDED`]. The constructors below document the separate cardinality or
//! flow argument for every remaining edge that one slot would serve. An edge
//! that hands its consumer one item at a time has [`HANDOFF_CAPACITY`] slots,
//! so that a send into an empty edge does not park its producer. The outgoing
//! replies' edge holds as many chunks as one taken chunk can make its stage
//! hand on ([`OUTGOING_REPLY_CHUNKS`]), for batching alone. Only the
//! inter-level return boundary needs a fan for progress.

use futures::{StreamExt as _, stream};

use super::LeafRequest;
use crate::tree::{
    mirror::streaming::{
        Backend, FAN, Leaf,
        channel::{HANDOFF_CAPACITY, QueueKind, QueueRole, Receiver, Sender, UNBOUNDED, channel},
        erased::{self, Reply, ReplyResultStream},
        materialized::{Error, Query, Resolution},
    },
    typed::height::{Height, Root, S, UnderRoot, UnderUnderRoot, Z},
};

/// A channel receiver stream whose items are wrapped in `Ok`.
pub(super) type OkReceiverStream<T, E> = stream::Map<Receiver<T>, fn(T) -> Result<T, E>>;

/// Create a channel whose receiver presents every item as `Ok`.
fn ok_channel<T, E>(role: QueueRole, capacity: usize) -> (Sender<T>, OkReceiverStream<T, E>) {
    let (sender, receiver) = channel(role, capacity);
    (sender, receiver.map(Ok))
}

/// The reply chunks the outgoing replies' edge holds ([`outgoing_responses`]):
/// a fan, [`FAN`].
///
/// The capacity follows from what one input makes a walk stage produce. A stage
/// takes one chunk of the peer's reply at a time, at most a fan of reactions,
/// and answers every reaction of it that disputes a child before it takes the
/// next chunk, so it hands on at most a fan of answers per taken chunk. Each
/// answer ends in one last chunk, and an answer that asks nothing is that chunk
/// alone, so with a fan of slots a stage hands on all of a taken chunk's
/// answers of that kind without waiting on the edge's consumer. An answer that
/// asks questions ends a chunk at each question, handed on once its question is
/// admitted. A smaller edge would hand a taken chunk's answers over in several
/// waves, each a few chunks for the encoder and a small write for the pipe.
///
/// A channel allocates a node for each chunk it holds, never for a slot it
/// leaves empty, so the capacity reserves no memory by itself.
///
/// The remote proxy's scope queue counts these beside a fan: its encoder
/// dequeues a reply's scope before it takes the reply's chunks from this edge,
/// so each chunk queued here can stand for one scope not yet dequeued.
pub(crate) const OUTGOING_REPLY_CHUNKS: usize = FAN;

/// Buffer outgoing protocol replies, [`OUTGOING_REPLY_CHUNKS`] chunks deep.
///
/// A blocked producer has already made a reply available to the counterparty,
/// and consuming that reply is sufficient to release the producer, so one slot
/// is the liveness floor. More slots retain whole messages without breaking
/// another dependency, so the edge holds as many chunks as one taken chunk's
/// answers can hand on together.
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
        OUTGOING_REPLY_CHUNKS,
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
/// finishes the reaction loop. When admission takes one answer of a depth at a
/// time, as it does in the serial configuration, the walk admits each answer
/// only once the previous answered child is assembled, so every child but the
/// last does complete first; the capacity tests' witness
/// (`capacity_stress_witness_requires_inter_level_fan`) stalls one slot short
/// of that. Once the parent resolution arrives, assembly drains the completions
/// in order, so the bound does not multiply with tree width or depth.
///
/// **One full fan is this edge's hard floor, not a tunable.** Unlike the
/// unbounded edges, whose one-slot floor is deadlock-free by the ordering
/// invariants, shrinking this queue below what a fan's completions need stalls
/// a session.
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
/// The opening emits exactly one query for the root scope, so the edge never
/// holds a second item; its [`HANDOFF_CAPACITY`] slots leave room behind that
/// one send, which therefore does not park.
pub(super) fn initiator_root_query<B>() -> (Sender<Query<B::Erased>>, Receiver<Query<B::Erased>>)
where
    B: Backend<Node<Z>: Leaf>,
{
    channel(
        QueueRole::new(QueueKind::InitiatorRootQuery, UnderRoot::HEIGHT),
        HANDOFF_CAPACITY,
    )
}

/// Carry the initiator's single completed root.
///
/// Reconciliation produces exactly one root node and the terminal future
/// consumes it directly; the edge's [`HANDOFF_CAPACITY`] slots leave room
/// behind that one send, which therefore does not park.
pub(super) fn initiator_root_return<B>() -> (Sender<Option<B::Erased>>, Receiver<Option<B::Erased>>)
where
    B: Backend<Node<Z>: Leaf>,
{
    channel(
        QueueRole::new(QueueKind::InitiatorRootReturn, Root::HEIGHT),
        HANDOFF_CAPACITY,
    )
}

/// Stream the responder opening's child queries.
///
/// The opening's first chunk and root resolution are published before these
/// queries, so one slot is the liveness floor; each buffered [`Query`] was
/// admitted, and its handles charged, before it was built.
pub(super) fn responder_child_queries<B>() -> (Sender<Query<B::Erased>>, Receiver<Query<B::Erased>>)
where
    B: Backend<Node<Z>: Leaf>,
{
    channel(
        QueueRole::new(QueueKind::ResponderChildQueries, UnderUnderRoot::HEIGHT),
        UNBOUNDED,
    )
}

/// Carry the responder's single root resolution.
///
/// The responder processes exactly one opening request and therefore publishes
/// exactly one resolution for the root scope; the edge's [`HANDOFF_CAPACITY`]
/// slots leave room behind that one send, which therefore does not park.
pub(super) fn responder_root_resolution<B>() -> (
    Sender<Resolution<B::Erased>>,
    OkReceiverStream<Resolution<B::Erased>, Error<B::Error>>,
)
where
    B: Backend<Node<Z>: Leaf>,
{
    ok_channel(
        QueueRole::new(QueueKind::ResponderRootResolution, UnderRoot::HEIGHT),
        HANDOFF_CAPACITY,
    )
}

/// Buffer the responder opening's completed child scopes.
///
/// The root resolution is visible before its child queries are sent, so its
/// assembler can consume each return as it arrives. No later return is needed
/// to unlock the consumer of the buffered one, so one slot is the liveness
/// floor; with [`HANDOFF_CAPACITY`] slots, a return sent while the assembler
/// keeps up does not park.
pub(super) fn responder_root_returns<B>() -> (
    Sender<Option<B::Erased>>,
    OkReceiverStream<Option<B::Erased>, Error<B::Error>>,
)
where
    B: Backend<Node<Z>: Leaf>,
{
    ok_channel(
        QueueRole::new(QueueKind::ResponderRootReturns, UnderRoot::HEIGHT),
        HANDOFF_CAPACITY,
    )
}

/// Buffer the child queries emitted by one internal walk.
///
/// `height` is the children's height: the walk's dependent queries descend to
/// it, and it labels the edge.
///
/// The corresponding child resolution is published first, so one slot is the
/// liveness floor. Its occupancy is the number of disputed scopes awaiting wire
/// replies at this height, which admission, not this queue, bounds.
pub(super) fn internal_child_queries<B>(
    height: usize,
) -> (Sender<Query<B::Erased>>, Receiver<Query<B::Erased>>)
where
    B: Backend<Node<Z>: Leaf>,
{
    channel(
        QueueRole::new(QueueKind::InternalChildQueries, height),
        UNBOUNDED,
    )
}

/// Buffer parent-scope resolutions produced by an internal walk.
///
/// `height` is the parent resolutions' own height (two above the walk's
/// dependent queries).
///
/// Before each parent resolution is sent, all work capable of fulfilling its
/// `Pending` slots has been launched, so one slot is the liveness floor. But a
/// resolution is consumed only as its subtree completes, so a one-slot edge
/// would stall the walk two scopes in.
pub(super) fn internal_parent_resolutions<B>(
    height: usize,
) -> (
    Sender<Resolution<B::Erased>>,
    OkReceiverStream<Resolution<B::Erased>, Error<B::Error>>,
)
where
    B: Backend<Node<Z>: Leaf>,
{
    ok_channel(
        QueueRole::new(QueueKind::InternalParentResolutions, height),
        UNBOUNDED,
    )
}

/// Buffer child-scope resolutions produced by an internal walk.
///
/// `height` is the child resolutions' own height (one above the walk's
/// dependent queries).
///
/// Each resolution is published before its corresponding child queries, so one
/// slot is the liveness floor; the walk publishes a pipeline of them while
/// earlier subtrees are still reconciling.
pub(super) fn internal_child_resolutions<B>(
    height: usize,
) -> (
    Sender<Resolution<B::Erased>>,
    OkReceiverStream<Resolution<B::Erased>, Error<B::Error>>,
)
where
    B: Backend<Node<Z>: Leaf>,
{
    ok_channel(
        QueueRole::new(QueueKind::InternalChildResolutions, height),
        UNBOUNDED,
    )
}

/// Buffer the leaf requests emitted by a leaf-parent walk.
///
/// The corresponding leaf-scope resolution is published first, so one slot is
/// the liveness floor; how many requested leaves await the peer's supplies at
/// once is admission's to bound.
pub(super) fn leaf_requests() -> (Sender<LeafRequest>, Receiver<LeafRequest>) {
    channel(
        QueueRole::new(QueueKind::LeafRequests, Z::HEIGHT),
        UNBOUNDED,
    )
}

/// Buffer leaf-parent resolutions awaiting their reconstructed children.
///
/// All terminal work for a parent resolution has been launched before it is
/// sent — the one-slot liveness floor; the walk runs ahead while buffered
/// resolutions wait on their leaf exchanges.
pub(super) fn leaf_parent_resolutions<B>() -> (
    Sender<Resolution<B::Erased>>,
    OkReceiverStream<Resolution<B::Erased>, Error<B::Error>>,
)
where
    B: Backend<Node<Z>: Leaf>,
{
    ok_channel(
        QueueRole::new(QueueKind::LeafParentResolutions, <S<Z>>::HEIGHT),
        UNBOUNDED,
    )
}

/// Buffer leaf-scope resolutions produced within one leaf-parent reply.
///
/// Each resolution is published before its leaf requests — the one-slot
/// liveness floor; the walk keeps publishing while earlier leaf scopes await
/// their supplies.
pub(super) fn leaf_child_resolutions<B>() -> (
    Sender<Resolution<B::Erased>>,
    OkReceiverStream<Resolution<B::Erased>, Error<B::Error>>,
)
where
    B: Backend<Node<Z>: Leaf>,
{
    ok_channel(
        QueueRole::new(QueueKind::LeafChildResolutions, Z::HEIGHT),
        UNBOUNDED,
    )
}

/// Stream terminal leaf resolutions, buffered one fan deep.
///
/// Terminal resolutions contain no `Pending` slots, so leaf assembly can
/// consume each immediately; no later item is required to unlock its consumer,
/// and one slot is the liveness floor. But the walk produces one resolution per
/// requested leaf, so a one-slot edge pays a waker round trip per leaf on the
/// compute path. One fan amortizes that; these items are single-leaf
/// resolutions belonging to scopes the ledger already charges. (Contrast
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
