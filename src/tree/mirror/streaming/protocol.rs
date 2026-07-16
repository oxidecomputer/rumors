//! The streaming protocol's traits, generic over the session's backend.
//!
//! Both parties of a session speak one backend `I`: the messages a stage
//! consumes and the messages it produces are keyed by the same node types.
//! A party that owns no tree — the [`remote`](crate::tree::mirror::streaming::remote) proxy — is
//! parameterized by its local counterparty's backend rather than defining one
//! of its own, which is what lets the node types meet in the middle without a
//! conversion anywhere in the schedule.

#![allow(clippy::type_complexity)]

use std::pin::Pin;

use futures::Stream;

use crate::{
    Version,
    tree::{
        mirror::streaming::{Backend, Leaf, message},
        typed::height::{Height, Root, S, UnderRoot, UnderUnderRoot, Z},
    },
};

mod peer;
pub use peer::{Client, Peer, Server};

pub trait Protocol: Send {
    type Height: Height;
    // `Send + 'static` because these traits' outgoing streams carry it, both
    // bare and inside an `OutputError`, and the driver moves it into its
    // error slot.
    type Error: Send + 'static;
    type Output: Send;
}

/// Trait synonym: non-erroring message streams, the shape of incoming streams.
pub trait Requests<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static, H: Height>:
    Stream<Item = message::Reply<B, T, H>> + Send + 'static
{
}
impl<X, B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static, H: Height> Requests<B, T, H>
    for X
where
    X: Stream<Item = message::Reply<B, T, H>> + Send + 'static,
{
}

/// Trait synonym: fallible message streams, the shape of outgoing streams.
pub trait Responses<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static, H: Height, E>:
    Stream<Item = Result<message::Reply<B, T, H>, E>> + Send + 'static
{
}
impl<X, B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static, H: Height, E>
    Responses<B, T, H, E> for X
where
    X: Stream<Item = Result<message::Reply<B, T, H>, E>> + Send + 'static,
{
}

/// A boxed [`Responses`] stream.
pub type BoxResponses<B, T, H, E> = Pin<Box<dyn Responses<B, T, H, E>>>;

pub trait Connect<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static>:
    Protocol<Height = Root> + Sized
{
    type Next: CompleteConnect<B, T>
        + Protocol<Height = Root, Output = Self::Output, Error = Self::Error>;

    fn connect(
        self,
    ) -> impl Future<Output = Result<(message::Handshake, Self::Next), Self::Error>> + Send;
}

pub trait Accept<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static>:
    Protocol<Height = Root> + Sized
{
    type Next: CompleteEqual<B, T>
        + Initiator<B, T>
        + Responder<B, T>
        + Protocol<Height = Root, Output = Self::Output, Error = Self::Error>;

    fn accept(
        self,
        request: message::Handshake,
    ) -> impl Future<Output = Result<(message::Handshake, Self::Next), Self::Error>> + Send;
}

pub trait CompleteConnect<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static>:
    Protocol<Height = Root> + Sized
{
    type Next: CompleteEqual<B, T>
        + Initiator<B, T>
        + Responder<B, T>
        + Protocol<Height = Root, Output = Self::Output, Error = Self::Error>;

    fn complete_connect(
        self,
        their_version: Version,
    ) -> impl Future<Output = Result<Self::Next, Self::Error>> + Send;
}

/// Resolve a connected session directly when the handshake versions match.
///
/// Equal versions prove that no descent is needed, but each connected state
/// must still be converted into its normal output. A materialized state returns
/// the root it already holds; a remote proxy returns its transport halves so
/// the caller can continue with trailing session frames.
pub trait CompleteEqual<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static>:
    Protocol<Height = Root> + Sized
{
    fn complete_equal(self) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send;
}

/// The opening burst: the initiator speaks first, and unprompted.
///
/// Nothing precedes this stage on the wire. A root hash would be the natural
/// thing to send, and it is exactly what the session never needs: two roots
/// hash equal only when their versions are equal, and equal versions
/// short-circuit the session before it reaches the protocol at all. So the
/// initiator skips straight to its root's children.
pub trait Initiator<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static>:
    Protocol<Height = Root> + Sized
{
    type Next: Protocol<Height = UnderRoot, Output = Self::Output, Error = Self::Error>;

    fn initiator(self) -> (impl Responses<B, T, UnderRoot, Self::Error>, Self::Next);
}

pub trait Responder<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static>:
    Protocol<Height = Root> + Sized
{
    // Like [`Initiator::Next`], this is left un-bounded by [`Reply`]: both
    // openings hand off to the descent, but only [`Peer`] spells the chain out.
    // Naming `Reply` here instead would make it a bound `Accept::Next` and
    // `CompleteConnect::Next` must discharge, which no generic wrapper can do
    // without the concrete height.
    type Next: Protocol<Height = UnderUnderRoot, Output = Self::Output, Error = Self::Error>;

    fn responder(
        self,
        requests: impl Requests<B, T, UnderRoot>,
    ) -> (
        // IMPORTANT: This must be boxed because otherwise `rustc` explodes on
        // an exponentially-sized type!
        BoxResponses<B, T, UnderRoot, Self::Error>,
        Self::Next,
    );
}

/// The height transition of a reply phase.
pub trait ReplyHeight: Height {
    type Output: Height;
    type Next: Height;
}

impl ReplyHeight for S<Z> {
    type Output = Z;
    type Next = Z;
}

impl<H> ReplyHeight for S<S<H>>
where
    H: Height,
    S<H>: Height,
    S<S<H>>: Height,
{
    type Output = S<H>;
    type Next = H;
}

pub trait Reply<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static>:
    Protocol<Height: ReplyHeight> + Sized
{
    type Next: Protocol<
            Height = <Self::Height as ReplyHeight>::Next,
            Output = Self::Output,
            Error = Self::Error,
        >;

    fn reply(
        self,
        requests: impl Requests<B, T, Self::Height>,
    ) -> (
        // IMPORTANT: This must be boxed because otherwise `rustc` explodes on
        // an exponentially-sized type!
        BoxResponses<B, T, <Self::Height as ReplyHeight>::Output, Self::Error>,
        Self::Next,
    );
}

/// The responder's terminal: absorb the initiator's leaf replies and answer
/// each requested leaf with one leaf-height [`Reply`].
///
/// Each requested leaf is answered pruned against the initiator's version,
/// so a leaf the initiator deleted drops here instead of shipping.
pub trait CompleteResponder<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static>:
    Protocol<Height = Z> + Sized
{
    fn complete_responder(
        self,
        requests: impl Requests<B, T, Z>,
    ) -> (
        BoxResponses<B, T, Z, Self::Error>,
        impl Future<Output = Result<Self::Output, Self::Error>> + Send,
    );
}

/// The initiator's terminal: absorb the responder's final leaf replies and
/// resolve to the reconciled root.
pub trait CompleteInitiator<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static>:
    Protocol<Height = Z> + Sized
{
    fn complete_initiator(
        self,
        requests: impl Requests<B, T, Z>,
    ) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send;
}
