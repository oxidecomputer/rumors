use std::convert::Infallible;

use crate::{
    Version,
    tree::{
        mirror::streaming::{Backend, Leaf},
        typed::height::{Height, Pred, Root, S, UnderRoot, UnderUnderRoot, Z},
    },
};

use super::message;

use futures::Stream;

pub trait Stage {
    type Height: Height;
    type Error;
}

pub enum Error<I, E> {
    Internal(I),
    External(E),
}

// Trait synonym for messages of type `M` sent between `S` and `B`, carrying type `T` leaves.
pub trait Messages<M, S: Stage + ?Sized, B: Backend<T, Node<Z>: Leaf<T>>, T>:
    Stream<Item = Result<M, Error<S::Error, B::Error>>> + Send
{
}
impl<X, M, S: Stage + ?Sized, B: Backend<T, Node<Z>: Leaf<T>>, T> Messages<M, S, B, T> for X where
    X: Stream<Item = Result<M, Error<S::Error, B::Error>>> + Send
{
}

pub trait Connect<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>: Stage<Height = Root> {
    async fn connect(
        self,
    ) -> Result<
        (
            message::Handshake,
            impl CompleteConnect<B, T> + Stage<Height = Root, Error = Self::Error>,
        ),
        Error<Self::Error, B::Error>,
    >;
}

pub trait Accept<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>: Stage<Height = Root> {
    async fn accept(
        self,
        request: message::Handshake,
    ) -> Result<
        (
            message::Handshake,
            impl Initiator<B, T> + Responder<B, T> + Stage<Height = Root, Error = Self::Error>,
        ),
        Error<Self::Error, B::Error>,
    >;
}

pub trait CompleteConnect<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>:
    Stage<Height = Root>
{
    async fn complete_connect(
        self,
        their_version: Version,
    ) -> Result<
        impl Initiator<B, T> + Responder<B, T> + Stage<Height = Root, Error = Self::Error>,
        Error<Self::Error, B::Error>,
    >;
}

pub trait Initiator<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>: Stage<Height = Root> {
    fn initiator(
        self,
    ) -> (
        impl Messages<message::Initiate, Self, B, T>,
        impl OpenInitiator<B, T> + Stage<Height = Root, Error = Self::Error>,
    );
}

pub trait Responder<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>: Stage<Height = Root> {
    fn responder(
        self,
        requests: impl Messages<message::Initiate, Self, B, T>,
    ) -> (
        impl Messages<message::Opening, Self, B, T>,
        impl Exchange<B, T> + Stage<Height = UnderRoot, Error = Self::Error>,
    );
}

pub trait OpenInitiator<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>:
    Stage<Height = Root> + Sized
{
    fn open_initiator(
        self,
        requests: impl Messages<message::Opening, Self, B, T>,
    ) -> (
        impl Messages<message::Exchange<B, T, UnderUnderRoot>, Self, B, T>,
        impl Exchange<B, T> + Stage<Height = UnderUnderRoot, Error = Self::Error>,
    );
}

pub trait Exchange<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>: Stage + Sized
where
    Self::Height: Pred,
    <Self::Height as Pred>::Pred: Pred,
    S<<Self::Height as Pred>::Pred>: Height,
    S<<<Self::Height as Pred>::Pred as Pred>::Pred>: Height,
{
    fn exchange(
        self,
        requests: impl Messages<message::Exchange<B, T, <Self::Height as Pred>::Pred>, Self, B, T>,
    ) -> (
        impl Messages<message::Exchange<B, T, <<Self::Height as Pred>::Pred as Pred>::Pred>, Self, B, T>,
        impl AfterExchange<B, T, <<Self::Height as Pred>::Pred as Pred>::Pred>
        + Stage<Height = <<Self::Height as Pred>::Pred as Pred>::Pred, Error = Self::Error>,
    );
}

pub trait CloseInitiator<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>:
    Stage<Height = S<S<Z>>> + Sized
{
    fn close_initiator(
        self,
        requests: impl Messages<message::Exchange<B, T, S<Z>>, Self, B, T>,
    ) -> (
        impl Messages<message::Closing<B, T>, Self, B, T>,
        impl CompleteInitiator<B, T> + Stage<Height = Z, Error = Self::Error>,
    );
}

pub trait CompleteResponder<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>:
    Stage<Height = S<Z>> + Sized
{
    fn complete_responder(
        self,
        requests: impl Messages<message::Closing<B, T>, Self, B, T>,
    ) -> impl Messages<message::Complete<B, T>, Self, B, T>;
}

pub trait CompleteInitiator<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>:
    Stage<Height = Z> + Sized
{
    fn complete_initiator(
        self,
        requests: impl Messages<message::Complete<B, T>, Self, B, T>,
    ) -> impl Future<Output = Result<(), Error<Self::Error, B::Error>>> + Send;
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
pub trait AfterExchange<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync, H: Height>: Sized {}

impl<T: Send + Sync, B: Backend<T, Node<Z>: Leaf<T>>, X: CompleteResponder<B, T>>
    AfterExchange<B, T, S<Z>> for X
{
}

impl<T: Send + Sync, B: Backend<T, Node<Z>: Leaf<T>>, X: CloseInitiator<B, T>>
    AfterExchange<B, T, S<S<Z>>> for X
{
}

impl<T, B, H, X> AfterExchange<B, T, S<S<S<H>>>> for X
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync,
    H: Height,
    S<H>: Height,
    S<S<H>>: Height,
    S<S<S<H>>>: Height,
    X: Exchange<B, T> + Stage<Height = S<S<S<H>>>>,
{
}
