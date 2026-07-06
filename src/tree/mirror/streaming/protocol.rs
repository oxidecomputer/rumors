//! The streaming protocol's phase traits, generic over both parties' backends.
//!
//! Two backend parameters run through these traits. *`I`* is the backend of
//! an implementor's incoming streams: an implementor requires this to be its
//! own backend, because a message's nodes are useless to it in any other
//! representation. *`O`* is the backend of its outgoing streams: the
//! counterparty's, which the counterparty requires to be *its* own for the
//! same reason. The phases whose output carries nodes convert as they
//! produce (see [`convert`](super::convert)) — an implementation that needs
//! an `O` handle to do so holds one internally, as the tree session does —
//! while the node-free phases never name `O` at all. Both are trait
//! parameters, so an implementation may be generic over either or pin
//! either.
//!
//! Errors never cross the party boundary. Incoming streams are non-erroring
//! [`Requests`] streams: whoever produced one (the in-process driver, a wire
//! transport) has already stripped the producer's errors out of band.
//! Outgoing streams do error, in the producer's frame: node-free phases in
//! the implementor's own [`Protocol::Error`], node-carrying phases in
//! [`OutputError`] — own errors first, faults from assembling into `O`'s
//! node types second.

#![allow(clippy::type_complexity)]

use crate::{
    Version,
    tree::{
        mirror::{
            Error,
            streaming::{Backend, BoxMessages},
        },
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

pub trait Protocol {
    type Height: Height;
    // `Send + 'static` because these traits' outgoing streams carry it, both
    // bare and inside an `OutputError`, and the driver moves it into its
    // error slot.
    type Error: Send + 'static;
    type Output;
}

/// Trait synonym: fallible message streams, the shape of outgoing streams.
pub trait Messages<M, E>: Stream<Item = Result<M, E>> + Send + 'static {}
impl<X, M, E> Messages<M, E> for X where X: Stream<Item = Result<M, E>> + Send + 'static {}

/// Trait synonym: non-erroring message streams, the shape of incoming streams.
///
/// Whoever produced the stream (the in-process driver, a wire transport)
/// strips the producer's errors out of band before it crosses the party
/// boundary, so a consumer never has to represent — or even be able to
/// represent — its counterparty's failures. End-of-stream therefore always
/// means the phase completed: a production failure parks the stream forever
/// instead of ending it, and the harness abandons the session around it.
pub trait Requests<M>: Stream<Item = M> + Send + 'static {}
impl<X, M> Requests<M> for X where X: Stream<Item = M> + Send + 'static {}

/// The error of node-carrying outgoing streams, in the producer's frame.
///
/// The implementor's own faults come first, faults from assembling into the
/// output backend `O`'s node types second.
pub type OutputError<P, O, T> = Error<<P as Protocol>::Error, <O as Backend<T>>::Error>;

pub trait Connect<I: Backend<T>, T: Send + Sync>: Protocol<Height = Root> + Sized {
    type Next: CompleteConnect<I, T>
        + Protocol<Height = Root, Output = Self::Output, Error = Self::Error>;

    fn connect(
        self,
    ) -> impl Future<Output = Result<(message::Handshake, Self::Next), Self::Error>> + Send;
}

pub trait Accept<I: Backend<T>, T: Send + Sync>: Protocol<Height = Root> + Sized {
    type Next: Initiator<I, T>
        + Responder<I, T>
        + Protocol<Height = Root, Output = Self::Output, Error = Self::Error>;

    fn accept(
        self,
        request: message::Handshake,
    ) -> impl Future<Output = Result<(message::Handshake, Self::Next), Self::Error>> + Send;
}

pub trait CompleteConnect<I: Backend<T>, T: Send + Sync>: Protocol<Height = Root> + Sized {
    type Next: Initiator<I, T>
        + Responder<I, T>
        + Protocol<Height = Root, Output = Self::Output, Error = Self::Error>;

    fn complete_connect(
        self,
        their_version: Version,
    ) -> impl Future<Output = Result<Self::Next, Self::Error>> + Send;
}

pub trait Initiator<I: Backend<T>, T: Send + Sync>: Protocol<Height = Root> + Sized {
    // The successor phase (`OpenInitiator`) emits node-carrying output and
    // so is generic in an output backend this node-free phase never names;
    // the `Peer` chains (see [`peer`]) restate the successor bound with `O`
    // threaded through.
    type Next: Protocol<Height = Root, Output = Self::Output, Error = Self::Error>;

    fn initiator(self) -> (impl Messages<message::Initiate, Self::Error>, Self::Next);
}

pub trait Responder<I: Backend<T>, T: Send + Sync>: Protocol<Height = Root> + Sized {
    // As with [`Initiator::Next`]: the successor (`Exchange`) needs `O`, so
    // the `Peer` chains carry its bound.
    type Next: Protocol<Height = UnderRoot, Output = Self::Output, Error = Self::Error>;

    fn responder(
        self,
        requests: impl Requests<message::Initiate>,
    ) -> (impl Messages<message::Opening, Self::Error>, Self::Next);
}

pub trait OpenInitiator<I: Backend<T>, O: Backend<T>, T: Send + Sync>:
    Protocol<Height = Root> + Sized
{
    type Next: Exchange<I, O, T>
        + Protocol<Height = UnderUnderRoot, Output = Self::Output, Error = Self::Error>;

    fn open_initiator(
        self,
        requests: impl Requests<message::Opening>,
    ) -> (
        // IMPORTANT: This must be boxed because otherwise `rustc` explodes on
        // an exponentially-sized type!
        BoxMessages<message::Exchanged<O, T, UnderRoot>, OutputError<Self, O, T>>,
        Self::Next,
    );
}

pub trait Exchange<I: Backend<T>, O: Backend<T>, T: Send + Sync>: Protocol + Sized
where
    Self::Height: Pred,
    <Self::Height as Pred>::Pred: Pred,
{
    type Next: AfterExchange<I, O, T, <<Self::Height as Pred>::Pred as Pred>::Pred>
        + Protocol<
            Height = <<Self::Height as Pred>::Pred as Pred>::Pred,
            Output = Self::Output,
            Error = Self::Error,
        >;

    fn exchange(
        self,
        requests: impl Requests<message::Exchanged<I, T, Self::Height>>,
    ) -> (
        // IMPORTANT: This must be boxed because otherwise `rustc` explodes on
        // an exponentially-sized type!
        BoxMessages<
            message::Exchanged<O, T, <Self::Height as Pred>::Pred>,
            OutputError<Self, O, T>,
        >,
        Self::Next,
    );
}

pub trait CloseInitiator<I: Backend<T>, O: Backend<T>, T: Send + Sync>:
    Protocol<Height = S<S<Z>>> + Sized
{
    type Next: CompleteInitiator<I, T>
        + Protocol<Height = Z, Output = Self::Output, Error = Self::Error>;

    fn close_initiator(
        self,
        requests: impl Requests<message::Exchanged<I, T, S<S<Z>>>>,
    ) -> (
        // IMPORTANT: This must be boxed because otherwise `rustc` explodes on
        // an exponentially-sized type!
        BoxMessages<(Prefix<S<Z>>, message::Closing<O, T>), OutputError<Self, O, T>>,
        Self::Next,
    );
}

pub trait CompleteResponder<I: Backend<T>, O: Backend<T>, T: Send + Sync>:
    Protocol<Height = S<Z>> + Sized
{
    fn complete_responder(
        self,
        requests: impl Requests<(Prefix<S<Z>>, message::Closing<I, T>)>,
    ) -> (
        BoxMessages<(Prefix<Z>, message::Complete<O, T>), OutputError<Self, O, T>>,
        impl Future<Output = Result<Self::Output, Self::Error>> + Send,
    );
}

pub trait CompleteInitiator<I: Backend<T>, T: Send + Sync>: Protocol<Height = Z> + Sized {
    fn complete_initiator(
        self,
        requests: impl Requests<(Prefix<Z>, message::Complete<I, T>)>,
    ) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send;
}

/// Blanket marker trait keyed by the height `H` just produced by an exchange:
///
/// - `H = S<Z>`: must impl [`CompleteResponder`].
/// - `H = S<S<Z>>`: must impl [`CloseInitiator`].
/// - `H = S<S<S<_>>>`: must impl [`Exchange`] at two heights finer.
///
/// Heights `S<Z>` and `S<S<Z>>` are handled via the blanket impls below,
/// keyed off the appropriate terminal trait.
///
/// Height `Z` is never reached as the result of an exchange (the leaf-height
/// uncertain map would be vacuous), so there is no `AfterExchange<Z>` impl.
pub trait AfterExchange<I: Backend<T>, O: Backend<T>, T: Send + Sync, H: Height>: Sized {}

impl<T: Send + Sync, I: Backend<T>, O: Backend<T>, X: CompleteResponder<I, O, T>>
    AfterExchange<I, O, T, S<Z>> for X
{
}

impl<T: Send + Sync, I: Backend<T>, O: Backend<T>, X: CloseInitiator<I, O, T>>
    AfterExchange<I, O, T, S<S<Z>>> for X
{
}

impl<T, I, O, H, X> AfterExchange<I, O, T, S<S<S<H>>>> for X
where
    I: Backend<T>,
    O: Backend<T>,
    T: Send + Sync,
    H: Height,
    S<H>: Height,
    S<S<H>>: Height,
    S<S<S<H>>>: Height,
    X: Exchange<I, O, T> + Protocol<Height = S<S<S<H>>>>,
{
}
