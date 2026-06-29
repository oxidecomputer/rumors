use std::convert::Infallible;

use crate::{
    Version,
    tree::typed::height::{Height, Pred, Root, S, UnderRoot, UnderUnderRoot, Z},
};

use super::message;

mod peer;
pub use peer::Peer;

pub trait Stage {
    type Height: Height;
}

pub trait Connect<T: Send + Sync>: Stage<Height = Root> + Sized {
    type Next: CompleteConnect<T> + Stage<Height = Root>;

    // async fn connect(self)
    // -> Result<Step<message::Handshake, Self::Next, Infallible>, Self::Error>;
}

pub trait CompleteConnect<T: Send + Sync>: Stage<Height = Root> + Sized {
    type Next: Initiator<T> + Responder<T> + Stage<Height = Root>;

    // async fn complete_connect(
    //     self,
    //     their_version: Version,
    // ) -> Result<Step<(), Self::Next, Self::Output>, Self::Error>;
}

pub trait Accept<T: Send + Sync>: Stage<Height = Root> + Sized {
    type Next: Initiator<T> + Responder<T> + Stage<Height = Root>;

    // async fn accept(
    //     self,
    //     request: message::Handshake,
    // ) -> Result<Step<message::Handshake, Self::Next, Self::Output>, Self::Error>;
}

pub trait Initiator<T: Send + Sync>: Stage<Height = Root> + Sized {
    type Next: OpenInitiator<T> + Stage<Height = Root>;

    // async fn initiator(
    //     self,
    // ) -> Result<Step<message::Initiate, Self::Next, Infallible>, Self::Error>;
}

pub trait Responder<T: Send + Sync>: Stage<Height = Root> + Sized {
    type Next: Exchange<T> + Stage<Height = UnderRoot>;

    // async fn responder(
    //     self,
    //     request: message::Initiate,
    // ) -> Result<Step<message::Opening, Self::Next, Self::Output>, Self::Error>;
}

pub trait OpenInitiator<T: Send + Sync>: Stage<Height = Root> + Sized {
    type Next: Exchange<T> + Stage<Height = UnderUnderRoot>;

    // async fn open_initiator(
    //     self,
    //     request: message::Opening,
    // ) -> Result<Step<message::Exchange<T, UnderUnderRoot>, Self::Next, Self::Output>, Self::Error>;
}

pub trait Exchange<T: Send + Sync>: Stage + Sized
where
    Self::Height: Pred,
    <Self::Height as Pred>::Pred: Pred,
    S<<Self::Height as Pred>::Pred>: Height,
    S<<<Self::Height as Pred>::Pred as Pred>::Pred>: Height,
{
    type Next: AfterExchange<T, <<Self::Height as Pred>::Pred as Pred>::Pred>
        + Stage<Height = <<Self::Height as Pred>::Pred as Pred>::Pred>;

    // async fn exchange(
    //     self,
    //     request: message::Exchange<T, <Self::Height as Pred>::Pred>,
    // ) -> Result<
    //     Step<
    //         message::Exchange<T, <<Self::Height as Pred>::Pred as Pred>::Pred>,
    //         Self::Next,
    //         Self::Output,
    //     >,
    //     Self::Error,
    // >;
}

pub trait CloseInitiator<T: Send + Sync>: Stage<Height = S<S<Z>>> + Sized {
    type Next: CompleteInitiator<T> + Stage<Height = Z>;

    // async fn close_initiator(
    //     self,
    //     request: message::Exchange<T, S<Z>>,
    // ) -> Result<Step<message::Closing<T>, Self::Next, Self::Output>, Self::Error>;
}

pub trait CompleteResponder<T: Send + Sync>: Stage<Height = S<Z>> + Sized {
    // async fn complete_responder(
    //     self,
    //     request: message::Closing<T>,
    // ) -> Result<Step<message::Complete<T>, Infallible, Self::Output>, Self::Error>;
}

pub trait CompleteInitiator<T: Send + Sync>: Stage<Height = Z> + Sized {
    // async fn complete_initiator(
    //     self,
    //     request: message::Complete<T>,
    // ) -> Result<Step<(), Infallible, Self::Output>, Self::Error>;
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
pub trait AfterExchange<T: Send + Sync, H: Height>: Sized {}

impl<T: Send + Sync, X: CompleteResponder<T>> AfterExchange<T, S<Z>> for X {}

impl<T: Send + Sync, X: CloseInitiator<T>> AfterExchange<T, S<S<Z>>> for X {}

impl<T, H, X> AfterExchange<T, S<S<S<H>>>> for X
where
    T: Send + Sync,
    H: Height,
    S<H>: Height,
    S<S<H>>: Height,
    S<S<S<H>>>: Height,
    X: Exchange<T> + Stage<Height = S<S<S<H>>>>,
{
}
