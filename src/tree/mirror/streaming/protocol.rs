//! The streaming protocol's traits, generic over the session's backend.
//!
//! Both parties of a session speak one backend `I`: the messages a stage
//! consumes and the messages it produces are keyed by the same node types.
//! A party that owns no tree — the [`remote`](super::remote) proxy — is
//! parameterized by its local counterparty's backend rather than defining one
//! of its own, which is what lets the node types meet in the middle without a
//! conversion anywhere in the schedule.

#![allow(clippy::type_complexity)]

use std::pin::Pin;

use crate::{
    Version,
    tree::{
        mirror::streaming::{Backend, Leaf},
        typed::{
            Prefix,
            height::{Height, Pred, Root, S, UnderRoot, UnderUnderRoot, Z},
        },
    },
};

use super::message;

use futures::Stream;

mod peer;
pub use peer::{Client, Peer, Server};

pub trait Protocol: Send {
    type Height: Height;
    // `Send + 'static` because these traits' outgoing streams carry it, both
    // bare and inside an `OutputError`, and the driver moves it into its
    // error slot.
    type Error: Send + 'static;
    type Output;
}

/// Trait synonym: non-erroring message streams, the shape of incoming streams.
pub trait Requests<M>: Stream<Item = M> + Send + 'static {}
impl<X, M> Requests<M> for X where X: Stream<Item = M> + Send + 'static {}

/// Trait synonym: fallible message streams, the shape of outgoing streams.
pub trait Responses<M, E>: Stream<Item = Result<M, E>> + Send + 'static {}
impl<X, M, E> Responses<M, E> for X where X: Stream<Item = Result<M, E>> + Send + 'static {}

/// A boxed [`Responses`] stream.
pub type BoxResponses<M, E> = Pin<Box<dyn Responses<M, E>>>;

pub trait Connect<I: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>:
    Protocol<Height = Root> + Sized
{
    type Next: CompleteConnect<I, T>
        + Protocol<Height = Root, Output = Self::Output, Error = Self::Error>;

    fn connect(
        self,
    ) -> impl Future<Output = Result<(message::Handshake, Self::Next), Self::Error>> + Send;
}

pub trait Accept<I: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>:
    Protocol<Height = Root> + Sized
{
    type Next: Initiator<I, T>
        + OpenResponder<I, T>
        + Protocol<Height = Root, Output = Self::Output, Error = Self::Error>;

    fn accept(
        self,
        request: message::Handshake,
    ) -> impl Future<Output = Result<(message::Handshake, Self::Next), Self::Error>> + Send;
}

pub trait CompleteConnect<I: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>:
    Protocol<Height = Root> + Sized
{
    type Next: Initiator<I, T>
        + OpenResponder<I, T>
        + Protocol<Height = Root, Output = Self::Output, Error = Self::Error>;

    fn complete_connect(
        self,
        their_version: Version,
    ) -> impl Future<Output = Result<Self::Next, Self::Error>> + Send;
}

/// The opening burst: the initiator speaks first, and unprompted.
///
/// Nothing precedes this stage on the wire. A root hash would be the natural
/// thing to send, and it is exactly what the session never needs: two roots
/// hash equal only when their versions are equal, and equal versions
/// short-circuit the session before it reaches the protocol at all. So the
/// initiator skips straight to its root's children.
pub trait Initiator<I: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>:
    Protocol<Height = Root> + Sized
{
    type Next: Protocol<Height = UnderRoot, Output = Self::Output, Error = Self::Error>;

    fn initiator(self) -> (impl Responses<message::Initiate, Self::Error>, Self::Next);
}

pub trait OpenResponder<I: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>:
    Protocol<Height = Root> + Sized
{
    // Like [`Initiator::Next`], this is left un-bounded by [`Exchange`]: both
    // openings hand off to the descent, but only [`Peer`] spells the chain out.
    // Naming `Exchange` here instead would make it a bound `Accept::Next` and
    // `CompleteConnect::Next` must discharge, which no generic wrapper can do
    // without the concrete height.
    type Next: Protocol<Height = UnderUnderRoot, Output = Self::Output, Error = Self::Error>;

    fn responder(
        self,
        requests: impl Requests<message::Initiate>,
    ) -> (
        // IMPORTANT: This must be boxed because otherwise `rustc` explodes on
        // an exponentially-sized type!
        BoxResponses<message::Reply<I, T, UnderRoot>, Self::Error>,
        Self::Next,
    );
}

pub trait Exchange<I: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>: Protocol + Sized
where
    Self::Height: Pred,
    <Self::Height as Pred>::Pred: Pred,
{
    type Next: AfterExchange<I, T, <<Self::Height as Pred>::Pred as Pred>::Pred>
        + Protocol<
            Height = <<Self::Height as Pred>::Pred as Pred>::Pred,
            Output = Self::Output,
            Error = Self::Error,
        >;

    fn reply(
        self,
        requests: impl Requests<message::Reply<I, T, Self::Height>>,
    ) -> (
        // IMPORTANT: This must be boxed because otherwise `rustc` explodes on
        // an exponentially-sized type!
        BoxResponses<message::Reply<I, T, <Self::Height as Pred>::Pred>, Self::Error>,
        Self::Next,
    );
}

pub trait CloseResponder<I: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>:
    Protocol<Height = S<S<Z>>> + Sized
{
    type Next: CompleteResponder<I, T>
        + Protocol<Height = Z, Output = Self::Output, Error = Self::Error>;

    fn close_responder(
        self,
        requests: impl Requests<message::Reply<I, T, S<S<Z>>>>,
    ) -> (
        // IMPORTANT: This must be boxed because otherwise `rustc` explodes on
        // an exponentially-sized type!
        BoxResponses<message::Reply<I, T, S<Z>>, Self::Error>,
        Self::Next,
    );
}

/// The initiator's closing reply: consume the responder's leaf-parent
/// verdicts, answer their `uncertain` leaf listings through the degenerate
/// leaf-height matrix, and emit [`message::Closing`].
///
/// An incoming `uncertain` here lists the responder's leaves under a parent
/// both sides hold; leaves never dispute, so each one resolves in place —
/// ours-only pruned and provided, theirs-only requested, shared kept in
/// silence. The kept leaves descend to the terminal, where the responder's
/// answers join them.
pub trait CloseInitiator<I: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>:
    Protocol<Height = S<Z>> + Sized
{
    type Next: CompleteInitiator<I, T>
        + Protocol<Height = Z, Output = Self::Output, Error = Self::Error>;

    fn close_initiator(
        self,
        requests: impl Requests<message::Reply<I, T, S<Z>>>,
    ) -> (BoxResponses<message::Close<I, T>, Self::Error>, Self::Next);
}

/// The responder's terminal: absorb the initiator's [`message::Closing`]
/// and answer it with the final [`message::Complete`].
///
/// Each `Requested` leaf is answered pruned against the initiator's
/// version, so a leaf the initiator deleted drops here instead of shipping.
pub trait CompleteResponder<I: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>:
    Protocol<Height = Z> + Sized
{
    fn complete_responder(
        self,
        requests: impl Requests<message::Close<I, T>>,
    ) -> (
        BoxResponses<message::Complete<I, T>, Self::Error>,
        impl Future<Output = Result<Self::Output, Self::Error>> + Send,
    );
}

/// The initiator's terminal: absorb the responder's final
/// [`message::Complete`] and resolve to the reconciled root.
pub trait CompleteInitiator<I: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>:
    Protocol<Height = Z> + Sized
{
    fn complete_initiator(
        self,
        requests: impl Requests<message::Complete<I, T>>,
    ) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send;
}

/// Blanket marker trait keyed by the height `H` just produced by an exchange:
///
/// - `H = S<Z>`: must impl [`CloseInitiator`].
/// - `H = S<S<Z>>`: must impl [`CloseResponder`].
/// - `H = S<S<S<_>>>`: must impl [`Exchange`] at two heights finer.
///
/// Heights `S<Z>` and `S<S<Z>>` are handled via the blanket impls below,
/// keyed off the appropriate closing trait.
///
/// Height `Z` is never reached as the result of an exchange (the closing
/// stages name their `Z`-height terminals directly through their `Next`
/// bounds), so there is no `AfterExchange<Z>` impl.
pub trait AfterExchange<I: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync, H: Height>: Sized {}

impl<T: Send + Sync, I: Backend<T, Node<Z>: Leaf<T>>, X: CloseInitiator<I, T>>
    AfterExchange<I, T, S<Z>> for X
{
}

impl<T: Send + Sync, I: Backend<T, Node<Z>: Leaf<T>>, X: CloseResponder<I, T>>
    AfterExchange<I, T, S<S<Z>>> for X
{
}

impl<T, I, H, X> AfterExchange<I, T, S<S<S<H>>>> for X
where
    I: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync,
    H: Height,
    S<H>: Height,
    S<S<H>>: Height,
    S<S<S<H>>>: Height,
    X: Exchange<I, T> + Protocol<Height = S<S<S<H>>>>,
{
}
