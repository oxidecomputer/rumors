#![allow(clippy::type_complexity)]

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

use super::{backend, message};

use futures::Stream;

pub trait Stage {
    type Height: Height;
}

/// Trait synonym: streams of protocol messages which may error.
pub trait Messages<M, E>: Stream<Item = Result<M, E>> + Send {}
impl<X, M, E> Messages<M, E> for X where X: Stream<Item = Result<M, E>> + Send {}

pub trait Connect<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>:
    Stage<Height = Root> + Sized
{
    type Next: CompleteConnect<B, T> + Stage<Height = Root>;

    fn connect<E: From<B::Error> + Send + 'static>(
        self,
    ) -> impl Future<Output = Result<(message::Handshake, Self::Next), E>> + Send;
}

pub trait Accept<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>:
    Stage<Height = Root> + Sized
{
    type Next: Initiator<B, T> + Responder<B, T> + Stage<Height = Root>;

    fn accept<E: From<B::Error> + Send + 'static>(
        self,
        request: message::Handshake,
    ) -> impl Future<Output = Result<(message::Handshake, Self::Next), E>> + Send;
}

pub trait CompleteConnect<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>:
    Stage<Height = Root> + Sized
{
    type Next: Initiator<B, T> + Responder<B, T> + Stage<Height = Root>;

    fn complete_connect<E: From<B::Error> + Send + 'static>(
        self,
        their_version: Version,
    ) -> impl Future<Output = Result<Self::Next, E>> + Send;
}

pub trait Initiator<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>:
    Stage<Height = Root> + Sized
{
    type Next: OpenInitiator<B, T> + Stage<Height = Root>;

    fn initiator<E: From<B::Error> + Send + 'static>(
        self,
    ) -> (impl Messages<message::Initiate, E> + 'static, Self::Next);
}

pub trait Responder<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>:
    Stage<Height = Root> + Sized
{
    type Next: Exchange<B, T> + Stage<Height = UnderRoot>;

    fn responder<E: From<B::Error> + Send + 'static>(
        self,
        requests: impl Messages<message::Initiate, E> + 'static,
    ) -> (impl Messages<message::Opening, E> + 'static, Self::Next);
}

pub trait OpenInitiator<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>:
    Stage<Height = Root> + Sized
{
    type Next: Exchange<B, T> + Stage<Height = UnderUnderRoot>;

    fn open_initiator<E: From<B::Error> + Send + 'static>(
        self,
        requests: impl Messages<message::Opening, E> + 'static,
    ) -> (
        impl Messages<(Prefix<UnderRoot>, message::Exchange<B, T, UnderRoot>), E> + 'static,
        Self::Next,
    );
}

pub trait Exchange<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>: Stage + Sized
where
    Self::Height: Pred,
    <Self::Height as Pred>::Pred: Pred,
{
    type Next: AfterExchange<B, T, <<Self::Height as Pred>::Pred as Pred>::Pred>
        + Stage<Height = <<Self::Height as Pred>::Pred as Pred>::Pred>;

    fn exchange<E: From<B::Error> + Send + 'static>(
        self,
        requests: impl Messages<(Prefix<Self::Height>, message::Exchange<B, T, Self::Height>), E>
        + 'static,
    ) -> (
        impl Messages<
            (
                Prefix<<Self::Height as Pred>::Pred>,
                message::Exchange<B, T, <Self::Height as Pred>::Pred>,
            ),
            E,
        > + 'static,
        Self::Next,
    );
}

pub trait CloseInitiator<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>:
    Stage<Height = S<S<Z>>> + Sized
{
    type Next: CompleteInitiator<B, T> + Stage<Height = Z>;

    fn close_initiator<E: From<B::Error> + Send + 'static>(
        self,
        requests: impl Messages<(Prefix<S<S<Z>>>, message::Exchange<B, T, S<S<Z>>>), E> + 'static,
    ) -> (
        impl Messages<(Prefix<S<Z>>, message::Closing<B, T>), E> + 'static,
        Self::Next,
    );
}

pub trait CompleteResponder<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>:
    Stage<Height = S<Z>> + Sized
{
    fn complete_responder<E: From<B::Error> + Send + 'static>(
        self,
        requests: impl Messages<(Prefix<S<Z>>, message::Closing<B, T>), E> + 'static,
    ) -> (
        impl Messages<(Prefix<Z>, message::Complete<B, T>), E> + 'static,
        impl Future<Output = Result<backend::Root<B, T>, E>> + Send,
    );
}

pub trait CompleteInitiator<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync>:
    Stage<Height = Z> + Sized
{
    fn complete_initiator<E: From<B::Error> + Send + 'static>(
        self,
        requests: impl Messages<(Prefix<Z>, message::Complete<B, T>), E> + 'static,
    ) -> impl Future<Output = Result<backend::Root<B, T>, E>> + Send;
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

macro_rules! define_peer {
    (
        init: [$($init_count:tt)*],
        resp: [$($resp_count:tt)*],
        $(,)?
    ) => {
        define_peer!(@step
            init: [$($init_count)*],
            resp: [$($resp_count)*],
            init_chain: (CloseInitiator<B, T>),
            resp_chain: (CompleteResponder<B, T>),
        );
    };

    (@step
        init: [_ $($init_rest:tt)*],
        resp: [$($resp_count:tt)*],
        init_chain: ($($init_chain:tt)*),
        resp_chain: ($($resp_chain:tt)*) $(,)?
    ) => {
        define_peer!(@step
            init: [$($init_rest)*],
            resp: [$($resp_count)*],
            init_chain: (Exchange<B, T, Next: $($init_chain)*>),
            resp_chain: ($($resp_chain)*),
        );
    };

    (@step
        init: [],
        resp: [_ $($resp_rest:tt)*],
        init_chain: ($($init_chain:tt)*),
        resp_chain: ($($resp_chain:tt)*) $(,)?
    ) => {
        define_peer!(@step
            init: [],
            resp: [$($resp_rest)*],
            init_chain: ($($init_chain)*),
            resp_chain: (Exchange<B, T, Next: $($resp_chain)*>),
        );
    };

    (@step
        init: [],
        resp: [],
        init_chain: ($($init_chain:tt)*),
        resp_chain: ($($resp_chain:tt)*) $(,)?
    ) => {
        pub trait Peer<B, T>:
            Initiator<B, T, Next: OpenInitiator<B, T, Next: $($init_chain)*>>
            + Responder<B, T, Next: $($resp_chain)*>
        where
            B: Backend<T, Node<Z>: Leaf<T>>,
            T: Send + Sync,
        {
        }

        impl<X, B, T> Peer<B, T> for X
        where
            B: Backend<T, Node<Z>: Leaf<T>>,
            T: Send + Sync,
            X: Initiator<B, T, Next: OpenInitiator<B, T, Next: $($init_chain)*>>
                + Responder<B, T, Next: $($resp_chain)*>,
        {
        }

        pub trait Server<B, T>:
            Accept<B, T, Next: Initiator<B, T, Next: OpenInitiator<B, T, Next: $($init_chain)*>> + Responder<B, T, Next: $($resp_chain)*>>
        where
            B: Backend<T, Node<Z>: Leaf<T>>,
            T: Send + Sync,
        {
        }

        impl<X, B, T> Server<B, T> for X
        where
            B: Backend<T, Node<Z>: Leaf<T>>,
            T: Send + Sync,
            X: Accept<B, T, Next: Initiator<B, T, Next: OpenInitiator<B, T, Next: $($init_chain)*>> + Responder<B, T, Next: $($resp_chain)*>>,
        {
        }

        pub trait Client<B, T>:
            Connect<B, T, Next: CompleteConnect<B, T, Next: Initiator<B, T, Next: OpenInitiator<B, T, Next: $($init_chain)*>> + Responder<B, T, Next: $($resp_chain)*>>>
        where
            B: Backend<T, Node<Z>: Leaf<T>>,
            T: Send + Sync,
        {
        }

        impl<X, B, T> Client<B, T> for X
        where
            B: Backend<T, Node<Z>: Leaf<T>>,
            T: Send + Sync,
            X: Connect<B, T, Next: CompleteConnect<B, T, Next: Initiator<B, T, Next: OpenInitiator<B, T, Next: $($init_chain)*>> + Responder<B, T, Next: $($resp_chain)*>>>,
        {
        }
    };
}

define_peer! {
    init: [_ _ _ _ _ _ _ _ _ _ _ _ _ _],
    resp: [_ _ _ _ _ _ _ _ _ _ _ _ _ _ _],
}
