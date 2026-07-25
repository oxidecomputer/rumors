//! Typed channel constructors for the materialized walk.
//!
//! Each function names one edge in the protocol dataflow. Keeping capacity
//! choices here makes them reviewable alongside the exact item type and keeps
//! queue arithmetic out of the walk itself.
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

#[cfg(not(test))]
use tokio_stream::wrappers::ReceiverStream;

use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        materialized::{
            Error, OkReceiverStream, Query, Resolution,
            channel::{QueueKind, QueueRole, Receiver, Sender, channel},
            ok_channel,
        },
        message::Reply,
        protocol::BoxResponses,
        window::FAN,
    },
    typed::{
        Prefix,
        height::{Height, Root, S, UnderRoot, UnderUnderRoot, Z},
    },
};

/// Buffer outgoing protocol replies one at a time.
///
/// A blocked producer has already made one reply available to the counterparty,
/// and consuming that reply is sufficient to release the producer. More slots
/// retain whole messages without breaking another dependency.
pub(super) fn outgoing_responses<B, T, H>() -> (
    Sender<Result<Reply<B, T, H>, Error<B::Error>>>,
    BoxResponses<B, T, H, Error<B::Error>>,
)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
{
    let (sender, receiver) = channel(QueueRole::new(QueueKind::OutgoingResponses, H::HEIGHT), 1);
    #[cfg(test)]
    let responses = Box::pin(receiver);
    #[cfg(not(test))]
    let responses = Box::pin(ReceiverStream::new(receiver));
    (sender, responses)
}

/// Buffer lower-level completions until their enclosing resolution arrives.
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
/// ordering invariants, shrinking this queue below `FAN` can genuinely
/// stall a session (`underbuffered_mirror_stalls` in the capacity tests
/// demonstrates it), which is why the session window deliberately never
/// reaches this constructor.
pub(super) fn assembly_level_returns<B, T, H>() -> (
    Sender<Option<B::Node<H>>>,
    OkReceiverStream<Option<B::Node<H>>, Error<B::Error>>,
)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
{
    ok_channel(
        QueueRole::new(QueueKind::AssemblyLevelReturns, H::HEIGHT),
        FAN,
    )
}

/// Carry the initiator's single root query.
///
/// The opening emits exactly one query for the root scope, so a second slot
/// can never be occupied.
pub(super) fn initiator_root_query<B, T>() -> (
    Sender<Query<B, T, UnderRoot>>,
    Receiver<Query<B, T, UnderRoot>>,
)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
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
pub(super) fn initiator_root_return<B, T>() -> (
    Sender<Option<B::Node<Root>>>,
    Receiver<Option<B::Node<Root>>>,
)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
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
pub(super) fn responder_child_queries<B, T>(
    capacity: usize,
) -> (
    Sender<Query<B, T, UnderUnderRoot>>,
    Receiver<Query<B, T, UnderUnderRoot>>,
)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    channel(
        QueueRole::new(QueueKind::ResponderChildQueries, UnderUnderRoot::HEIGHT),
        capacity,
    )
}

/// Carry the responder's single root resolution.
///
/// The responder processes exactly one opening request and therefore
/// publishes exactly one resolution for the root scope.
pub(super) fn responder_root_resolution<B, T>() -> (
    Sender<Resolution<B, T, UnderRoot>>,
    OkReceiverStream<Resolution<B, T, UnderRoot>, Error<B::Error>>,
)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
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
pub(super) fn responder_root_returns<B, T>() -> (
    Sender<Option<B::Node<UnderRoot>>>,
    OkReceiverStream<Option<B::Node<UnderRoot>>, Error<B::Error>>,
)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    ok_channel(
        QueueRole::new(QueueKind::ResponderRootReturns, UnderRoot::HEIGHT),
        1,
    )
}

/// Buffer the child queries emitted by one internal walk, window-wide.
///
/// The corresponding child resolution is published first, so one slot is the
/// liveness floor. This queue is the in-flight question window itself: its
/// occupancy is the number of disputed scopes awaiting wire replies at this
/// height, so its capacity is what lets sibling scopes' round trips overlap.
pub(super) fn internal_child_queries<B, T, H>(
    capacity: usize,
) -> (Sender<Query<B, T, H>>, Receiver<Query<B, T, H>>)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
{
    channel(
        QueueRole::new(QueueKind::InternalChildQueries, H::HEIGHT),
        capacity,
    )
}

/// Buffer parent-scope resolutions produced by an internal walk, window-wide.
///
/// Before each parent resolution is sent, all work capable of fulfilling its
/// `Pending` slots has been launched, so one slot is the liveness floor. But a
/// resolution is consumed only as its subtree completes, so a one-slot edge
/// stalls the walk two scopes in; the window lets it run ahead.
pub(super) fn internal_parent_resolutions<B, T, H>(
    capacity: usize,
) -> (
    Sender<Resolution<B, T, S<S<H>>>>,
    OkReceiverStream<Resolution<B, T, S<S<H>>>, Error<B::Error>>,
)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
    S<S<H>>: Height,
    S<S<S<H>>>: Height,
{
    ok_channel(
        QueueRole::new(QueueKind::InternalParentResolutions, <S<S<H>>>::HEIGHT),
        capacity,
    )
}

/// Buffer child-scope resolutions produced by an internal walk, window-wide.
///
/// Each resolution is published before its corresponding child queries, so one
/// slot is the liveness floor; the window lets the walk publish a pipeline of
/// them while earlier subtrees are still reconciling.
pub(super) fn internal_child_resolutions<B, T, H>(
    capacity: usize,
) -> (
    Sender<Resolution<B, T, S<H>>>,
    OkReceiverStream<Resolution<B, T, S<H>>, Error<B::Error>>,
)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
    S<S<H>>: Height,
{
    ok_channel(
        QueueRole::new(QueueKind::InternalChildResolutions, <S<H>>::HEIGHT),
        capacity,
    )
}

/// Buffer the leaf requests emitted by a leaf-parent walk, window-wide.
///
/// The corresponding leaf-scope resolution is published first, so one slot is
/// the liveness floor. This queue is the leaf-height question window: its
/// capacity is how many requested leaves may await the peer's supplies at once.
pub(super) fn leaf_requests(capacity: usize) -> (Sender<Prefix<Z>>, Receiver<Prefix<Z>>) {
    channel(QueueRole::new(QueueKind::LeafRequests, Z::HEIGHT), capacity)
}

/// Buffer leaf-parent resolutions awaiting their reconstructed children,
/// window-wide.
///
/// All terminal work for a parent resolution has been launched before it is
/// sent — the one-slot liveness floor; the window lets the walk run ahead while
/// buffered resolutions wait on their leaf exchanges.
pub(super) fn leaf_parent_resolutions<B, T>(
    capacity: usize,
) -> (
    Sender<Resolution<B, T, S<Z>>>,
    OkReceiverStream<Resolution<B, T, S<Z>>, Error<B::Error>>,
)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    ok_channel(
        QueueRole::new(QueueKind::LeafParentResolutions, <S<Z>>::HEIGHT),
        capacity,
    )
}

/// Buffer leaf-scope resolutions produced within one leaf-parent reply,
/// window-wide.
///
/// Each resolution is published before its leaf requests — the one-slot
/// liveness floor; the window keeps the walk publishing while earlier leaf
/// scopes await their supplies.
pub(super) fn leaf_child_resolutions<B, T>(
    capacity: usize,
) -> (
    Sender<Resolution<B, T, Z>>,
    OkReceiverStream<Resolution<B, T, Z>, Error<B::Error>>,
)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    ok_channel(
        QueueRole::new(QueueKind::LeafChildResolutions, Z::HEIGHT),
        capacity,
    )
}

/// Stream terminal leaf resolutions, buffered one fan deep.
///
/// Terminal resolutions contain no `Pending` slots, so leaf assembly can
/// consume each immediately; no later item is required to unlock its
/// consumer, and one slot is the liveness floor. But the walk produces one
/// resolution per requested leaf, so a one-slot edge pays a waker round
/// trip per leaf on the compute path. One fan amortizes that; unlike the
/// window edges these items are single-leaf resolutions belonging to
/// scopes the memory model already charges, so no knob applies. (Contrast
/// [`assembly_level_returns`], where one fan is a correctness floor rather
/// than an amortization.)
pub(super) fn terminal_leaf_resolutions<B, T>() -> (
    Sender<Resolution<B, T, Z>>,
    OkReceiverStream<Resolution<B, T, Z>, Error<B::Error>>,
)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    ok_channel(
        QueueRole::new(QueueKind::TerminalLeafResolutions, Z::HEIGHT),
        FAN,
    )
}
