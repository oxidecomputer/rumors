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
//! sufficient for those queues. The constructors below document the separate
//! cardinality or flow argument for every other one-slot edge; only the
//! inter-level return boundary needs a fan.

#[cfg(not(test))]
use tokio_stream::wrappers::ReceiverStream;

use crate::tree::{
    mirror::streaming::{
        Backend, Leaf, Node,
        materialized::{
            Error, OkReceiverStream, Query, Resolution,
            channel::{QueueKind, QueueRole, Receiver, Sender, channel},
            ok_channel,
        },
        message::Reply,
        protocol::BoxResponses,
    },
    typed::{
        Prefix,
        height::{Height, Root, S, UnderRoot, UnderUnderRoot, Z},
    },
};

/// The tree's maximum branching factor.
const FAN: usize = 256;

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

/// Stream the responder opening's child queries through one slot.
///
/// The opening wire reply and root resolution are published before these
/// queries. The next stage can therefore accept and resolve each buffered query
/// while the root assembler consumes its return through
/// [`responder_root_returns`]. One slot streams the whole fan without retaining
/// a fan of [`Query`] values, each of which may itself own a fan of node handles.
pub(super) fn responder_child_queries<B, T>() -> (
    Sender<Query<B, T, UnderUnderRoot>>,
    Receiver<Query<B, T, UnderUnderRoot>>,
)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    channel(
        QueueRole::new(QueueKind::ResponderChildQueries, UnderUnderRoot::HEIGHT),
        1,
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

/// Buffer the child queries emitted by one internal walk.
///
/// The corresponding child resolution is published first. If this sender
/// blocks, one query is already available to the next stage, whose return can
/// advance the assembler waiting on that resolution.
pub(super) fn internal_child_queries<B, T, H>() -> (Sender<Query<B, T, H>>, Receiver<Query<B, T, H>>)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
{
    channel(
        QueueRole::new(QueueKind::InternalChildQueries, H::HEIGHT),
        1,
    )
}

/// Buffer parent-scope resolutions produced by an internal walk.
///
/// Before each parent resolution is sent, all work capable of fulfilling its
/// `Pending` slots has been launched. If this sender blocks, the older buffered
/// resolution can therefore complete without the newer one being accepted.
pub(super) fn internal_parent_resolutions<B, T, H>() -> (
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
        1,
    )
}

/// Buffer child-scope resolutions produced by an internal walk.
///
/// Each resolution is published before its corresponding child queries. By
/// the time a later resolution can block behind it, all work needed by the
/// buffered resolution has been launched.
pub(super) fn internal_child_resolutions<B, T, H>() -> (
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
        1,
    )
}

/// Buffer the leaf requests emitted by a leaf-parent walk.
///
/// The corresponding leaf-scope resolution is published first. One buffered
/// request can therefore advance the terminal stage and the assembler waiting
/// on that resolution.
pub(super) fn leaf_requests() -> (Sender<Prefix<Z>>, Receiver<Prefix<Z>>) {
    channel(QueueRole::new(QueueKind::LeafRequests, Z::HEIGHT), 1)
}

/// Buffer leaf-parent resolutions awaiting their reconstructed children.
///
/// All terminal work for a parent resolution has been launched before it is
/// sent. A buffered older resolution can therefore complete even while this
/// sender is blocked on the next one.
pub(super) fn leaf_parent_resolutions<B, T>() -> (
    Sender<Resolution<B, T, S<Z>>>,
    OkReceiverStream<Resolution<B, T, S<Z>>, Error<B::Error>>,
)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    ok_channel(
        QueueRole::new(QueueKind::LeafParentResolutions, <S<Z>>::HEIGHT),
        1,
    )
}

/// Buffer leaf-scope resolutions produced within one leaf-parent reply.
///
/// Each resolution is published before its leaf requests. By the time a later
/// resolution can block behind it, the terminal work needed by the buffered
/// resolution has been launched.
pub(super) fn leaf_child_resolutions<B, T>() -> (
    Sender<Resolution<B, T, Z>>,
    OkReceiverStream<Resolution<B, T, Z>, Error<B::Error>>,
)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    ok_channel(
        QueueRole::new(QueueKind::LeafChildResolutions, Z::HEIGHT),
        1,
    )
}

/// Stream terminal leaf resolutions one at a time.
///
/// Terminal resolutions contain no `Pending` slots, so leaf assembly can
/// consume each immediately; no later item is required to unlock its consumer.
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
        1,
    )
}
