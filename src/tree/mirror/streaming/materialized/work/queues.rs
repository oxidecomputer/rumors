//! Typed channel constructors for the materialized walk.
//!
//! Each function names one edge in the protocol dataflow. Keeping capacity
//! choices here makes them reviewable alongside the exact item type and keeps
//! queue arithmetic out of the walk itself.
//!
//! Query and resolution queues rely on the walk's progress invariant: publish
//! a scope's resolution before sending the dependent work that fulfills its
//! `Pending` slots, and launch all such work before publishing its parent
//! resolution. That ordering makes one buffered item sufficient everywhere
//! except the inter-level return boundary documented below.

#[cfg(not(test))]
use tokio_stream::wrappers::ReceiverStream;

use crate::tree::{
    mirror::streaming::{
        Backend, Leaf, Node,
        materialized::{
            Error, OkReceiverStream, Query, Resolution,
            channel::{Receiver, Sender, channel},
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
    let (sender, receiver) = channel(1);
    #[cfg(test)]
    let responses = Box::pin(receiver);
    #[cfg(not(test))]
    let responses = Box::pin(ReceiverStream::new(receiver));
    (sender, responses)
}

/// Buffer completed lower-level nodes until their parent resolution arrives.
///
/// One parent reply may start a full fan of lower scopes, all of which can
/// finish before that reply publishes the resolution whose `Pending` slots
/// consume them. The whole fan must fit to let the resolution be published.
pub(super) fn assembly_level_returns<B, T, H>() -> (
    Sender<Option<B::Node<H>>>,
    OkReceiverStream<Option<B::Node<H>>, Error<B::Error>>,
)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
{
    ok_channel(FAN)
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
    channel(1)
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
    channel(1)
}

/// Stream the responder opening's child queries through one slot.
///
/// The opening reply is published before these queries, so its counterparty
/// can answer each query and let the next stage drain it. Early completed
/// children are absorbed by [`responder_root_returns`] instead of accumulating
/// here as query values which may each own a fan.
pub(super) fn responder_child_queries<B, T>() -> (
    Sender<Query<B, T, UnderUnderRoot>>,
    Receiver<Query<B, T, UnderUnderRoot>>,
)
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    channel(1)
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
    ok_channel(1)
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
    ok_channel(1)
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
    channel(1)
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
    ok_channel(1)
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
    ok_channel(1)
}

/// Buffer the leaf requests emitted by a leaf-parent walk.
///
/// The corresponding leaf-scope resolution is published first. One buffered
/// request can therefore advance the terminal stage and the assembler waiting
/// on that resolution.
pub(super) fn leaf_requests() -> (Sender<Prefix<Z>>, Receiver<Prefix<Z>>) {
    channel(1)
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
    ok_channel(1)
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
    ok_channel(1)
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
    ok_channel(1)
}
