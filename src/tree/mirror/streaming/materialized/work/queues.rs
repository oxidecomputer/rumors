//! Typed channel constructors for the materialized walk.
//!
//! Each function names one edge in the protocol dataflow. Keeping capacity
//! choices here makes them reviewable alongside the exact item type and keeps
//! queue arithmetic out of the walk itself. The channels beneath carry the
//! height-erased payload twins — one channel-machinery instantiation per
//! backend rather than one per height — behind typed facades minted per
//! edge (see [`erased`]).
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

use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        erased::{
            self, QueryReceiver, QuerySender, ResolutionOkStream, ResolutionSender, ReturnOkStream,
            ReturnReceiver, ReturnSender,
        },
        materialized::{
            Error,
            channel::{QueueKind, QueueRole, Receiver, Sender, channel},
        },
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
    erased::ReplyResultSender<B, T, H>,
    BoxResponses<B, T, H, Error<B::Error>>,
)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
{
    let (sender, responses) =
        erased::reply_channel(QueueRole::new(QueueKind::OutgoingResponses, H::HEIGHT), 1);
    (sender, Box::pin(responses))
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
pub(super) fn assembly_level_returns<B, T, H>() -> (ReturnSender<B, T, H>, ReturnOkStream<B, T, H>)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
{
    erased::return_ok_channel::<B, T, H>(
        QueueRole::new(QueueKind::AssemblyLevelReturns, H::HEIGHT),
        FAN,
    )
}

/// Carry the initiator's single root query.
///
/// The opening emits exactly one query for the root scope, so a second slot
/// can never be occupied.
pub(super) fn initiator_root_query<B, T>()
-> (QuerySender<B, T, UnderRoot>, QueryReceiver<B, T, UnderRoot>)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    erased::query_channel(
        QueueRole::new(QueueKind::InitiatorRootQuery, UnderRoot::HEIGHT),
        1,
    )
}

/// Carry the initiator's single completed root.
///
/// Reconciliation produces exactly one root node and the terminal future
/// consumes it directly.
pub(super) fn initiator_root_return<B, T>() -> (ReturnSender<B, T, Root>, ReturnReceiver<B, T, Root>)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    erased::return_channel::<B, T, Root>(
        QueueRole::new(QueueKind::InitiatorRootReturn, Root::HEIGHT),
        1,
    )
}

/// Stream the responder opening's child queries through the window.
///
/// The opening wire reply and root resolution are published before these
/// queries, so one slot is the liveness floor. The window widens it so the next
/// stage can hold a pipeline of disputed children in flight; each buffered
/// [`Query`](crate::tree::mirror::streaming::materialized::Query) may own a
/// fan of node handles, which is priced by the window's node budget.
pub(super) fn responder_child_queries<B, T>(
    capacity: usize,
) -> (
    QuerySender<B, T, UnderUnderRoot>,
    QueryReceiver<B, T, UnderUnderRoot>,
)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    erased::query_channel(
        QueueRole::new(QueueKind::ResponderChildQueries, UnderUnderRoot::HEIGHT),
        capacity,
    )
}

/// Carry the responder's single root resolution.
///
/// The responder processes exactly one opening request and therefore
/// publishes exactly one resolution for the root scope.
pub(super) fn responder_root_resolution<B, T>() -> (
    ResolutionSender<B, T, UnderRoot>,
    ResolutionOkStream<B, T, UnderRoot>,
)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    erased::resolution_ok_channel(
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
    ReturnSender<B, T, UnderRoot>,
    ReturnOkStream<B, T, UnderRoot>,
)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    erased::return_ok_channel::<B, T, UnderRoot>(
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
) -> (QuerySender<B, T, H>, QueryReceiver<B, T, H>)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
{
    erased::query_channel(
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
    ResolutionSender<B, T, S<S<H>>>,
    ResolutionOkStream<B, T, S<S<H>>>,
)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
    S<S<H>>: Height,
    S<S<S<H>>>: Height,
{
    erased::resolution_ok_channel(
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
) -> (ResolutionSender<B, T, S<H>>, ResolutionOkStream<B, T, S<H>>)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
    S<S<H>>: Height,
{
    erased::resolution_ok_channel(
        QueueRole::new(QueueKind::InternalChildResolutions, <S<H>>::HEIGHT),
        capacity,
    )
}

/// Buffer the leaf requests emitted by a leaf-parent walk, window-wide.
///
/// The corresponding leaf-scope resolution is published first, so one slot is
/// the liveness floor. This queue is the leaf-height question window: its
/// capacity is how many requested leaves may await the peer's supplies at once.
///
/// The one materialized edge with no erased twin: its item is already the
/// single-height [`Prefix<Z>`].
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
) -> (ResolutionSender<B, T, S<Z>>, ResolutionOkStream<B, T, S<Z>>)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    erased::resolution_ok_channel(
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
) -> (ResolutionSender<B, T, Z>, ResolutionOkStream<B, T, Z>)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    erased::resolution_ok_channel(
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
pub(super) fn terminal_leaf_resolutions<B, T>()
-> (ResolutionSender<B, T, Z>, ResolutionOkStream<B, T, Z>)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    erased::resolution_ok_channel(
        QueueRole::new(QueueKind::TerminalLeafResolutions, Z::HEIGHT),
        FAN,
    )
}
