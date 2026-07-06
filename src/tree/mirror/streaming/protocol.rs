#![allow(clippy::type_complexity)]

use crate::{
    Version,
    tree::{
        mirror::streaming::{Backend, BoxMessages, Materiality},
        typed::{
            Prefix,
            height::{Height, Pred, Root, S, UnderRoot, UnderUnderRoot, Z},
        },
    },
};

use super::{backend, message};

use futures::Stream;

mod peer;
pub use peer::{Client, Peer, Server};

pub trait Protocol {
    type Height: Height;
    // `Send + 'static` because every stream these traits exchange is a
    // `Messages<_, Self::Error>`, itself `Send + 'static`.
    type Error: Send + 'static;
    type Output;
}

/// Trait synonym: streams of protocol messages which may error.
pub trait Messages<M, E>: Stream<Item = Result<M, E>> + Send + 'static {}
impl<X, M, E> Messages<M, E> for X where X: Stream<Item = Result<M, E>> + Send + 'static {}

pub trait Connect<B: Backend<T>, T: Send + Sync>: Protocol<Height = Root> + Sized {
    type Next: CompleteConnect<B, T>
        + Protocol<Height = Root, Output = Self::Output, Error = Self::Error>;

    fn connect(
        self,
    ) -> impl Future<Output = Result<(message::Handshake, Self::Next), Self::Error>> + Send;
}

pub trait Accept<B: Backend<T>, T: Send + Sync>: Protocol<Height = Root> + Sized {
    type Next: Initiator<B, T>
        + Responder<B, T>
        + Protocol<Height = Root, Output = Self::Output, Error = Self::Error>;

    fn accept(
        self,
        request: message::Handshake,
    ) -> impl Future<Output = Result<(message::Handshake, Self::Next), Self::Error>> + Send;
}

pub trait CompleteConnect<B: Backend<T>, T: Send + Sync>: Protocol<Height = Root> + Sized {
    type Next: Initiator<B, T>
        + Responder<B, T>
        + Protocol<Height = Root, Output = Self::Output, Error = Self::Error>;

    fn complete_connect(
        self,
        their_version: Version,
    ) -> impl Future<Output = Result<Self::Next, Self::Error>> + Send;
}

pub trait Initiator<B: Backend<T>, T: Send + Sync>: Protocol<Height = Root> + Sized {
    type Next: OpenInitiator<B, T>
        + Protocol<Height = Root, Output = Self::Output, Error = Self::Error>;

    fn initiator(self) -> (impl Messages<message::Initiate, Self::Error>, Self::Next);
}

pub trait Responder<B: Backend<T>, T: Send + Sync>: Protocol<Height = Root> + Sized {
    type Next: Exchange<B, T>
        + Protocol<Height = UnderRoot, Output = Self::Output, Error = Self::Error>;

    fn responder(
        self,
        requests: impl Messages<message::Initiate, Self::Error>,
    ) -> (impl Messages<message::Opening, Self::Error>, Self::Next);
}

pub trait OpenInitiator<B: Backend<T>, T: Send + Sync>: Protocol<Height = Root> + Sized {
    type Next: Exchange<B, T>
        + Protocol<Height = UnderUnderRoot, Output = Self::Output, Error = Self::Error>;

    fn open_initiator(
        self,
        requests: impl Messages<message::Opening, Self::Error>,
    ) -> (
        // IMPORTANT: This must be boxed because otherwise `rustc` explodes on
        // an exponentially-sized type!
        BoxMessages<(Prefix<UnderRoot>, message::Exchange<B, T, UnderRoot>), Self::Error>,
        Self::Next,
    );
}

pub trait Exchange<B: Backend<T>, T: Send + Sync>: Protocol + Sized
where
    Self::Height: Pred,
    <Self::Height as Pred>::Pred: Pred,
{
    type Next: AfterExchange<B, T, <<Self::Height as Pred>::Pred as Pred>::Pred>
        + Protocol<
            Height = <<Self::Height as Pred>::Pred as Pred>::Pred,
            Output = Self::Output,
            Error = Self::Error,
        >;

    fn exchange(
        self,
        requests: impl Messages<
            (Prefix<Self::Height>, message::Exchange<B, T, Self::Height>),
            Self::Error,
        >,
    ) -> (
        // IMPORTANT: This must be boxed because otherwise `rustc` explodes on
        // an exponentially-sized type!
        BoxMessages<
            (
                Prefix<<Self::Height as Pred>::Pred>,
                message::Exchange<B, T, <Self::Height as Pred>::Pred>,
            ),
            Self::Error,
        >,
        Self::Next,
    );
}

pub trait CloseInitiator<B: Backend<T>, T: Send + Sync>:
    Protocol<Height = S<S<Z>>> + Sized
{
    type Next: CompleteInitiator<B, T>
        + Protocol<Height = Z, Output = Self::Output, Error = Self::Error>;

    fn close_initiator(
        self,
        requests: impl Messages<(Prefix<S<S<Z>>>, message::Exchange<B, T, S<S<Z>>>), Self::Error>,
    ) -> (
        // IMPORTANT: This must be boxed because otherwise `rustc` explodes on
        // an exponentially-sized type!
        BoxMessages<(Prefix<S<Z>>, message::Closing<B, T>), Self::Error>,
        Self::Next,
    );
}

pub trait CompleteResponder<B: Backend<T>, T: Send + Sync>:
    Protocol<Height = S<Z>> + Sized
{
    fn complete_responder(
        self,
        requests: impl Messages<(Prefix<S<Z>>, message::Closing<B, T>), Self::Error>,
    ) -> (
        BoxMessages<(Prefix<Z>, message::Complete<B, T>), Self::Error>,
        impl Future<Output = Result<Self::Output, Self::Error>> + Send,
    );
}

pub trait CompleteInitiator<B: Backend<T>, T: Send + Sync>: Protocol<Height = Z> + Sized {
    fn complete_initiator(
        self,
        requests: impl Messages<(Prefix<Z>, message::Complete<B, T>), Self::Error>,
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
pub trait AfterExchange<B: Backend<T>, T: Send + Sync, H: Height>: Sized {}

impl<T: Send + Sync, B: Backend<T>, X: CompleteResponder<B, T>> AfterExchange<B, T, S<Z>> for X {}

impl<T: Send + Sync, B: Backend<T>, X: CloseInitiator<B, T>> AfterExchange<B, T, S<S<Z>>> for X {}

impl<T, B, H, X> AfterExchange<B, T, S<S<S<H>>>> for X
where
    B: Backend<T>,
    T: Send + Sync,
    H: Height,
    S<H>: Height,
    S<S<H>>: Height,
    S<S<S<H>>>: Height,
    X: Exchange<B, T> + Protocol<Height = S<S<S<H>>>>,
{
}
