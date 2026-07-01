// TODO-integration: the streaming mirror is complete and oracle-tested but
// not yet wired into the crate's session layer, so outside `cfg(test)` the
// whole module is dead code. Remove this allow when `Peer` adopts it.
#![allow(unused)]

mod backend;
mod convert;
mod dispute;
mod merge;
mod message;
mod protocol;
mod session;
mod unknown;

pub use backend::{Backend, Leaf, Local, Node, Root};
pub use session::Handshaking;

use std::cmp::Ordering;

use crate::{Version, tree::typed::height::Z};
use protocol::*;
use seq_macro::seq;

#[cfg(test)]
mod tests;

/// Erase a message stream's concrete type at a stage boundary.
///
/// Each stage's outgoing stream type wraps its input stream's type; handed
/// around unboxed, the driver below would nest thirty stages of stream
/// machinery into one type and stall the compiler's trait solving. Boxing at
/// every hop keeps each stage's monomorphized type one stage deep — the same
/// reason every height-recursive transducer here returns a boxed stream.
fn boxed<'a, M: 'a, E: 'a>(
    messages: impl Messages<M, E> + 'a,
) -> std::pin::Pin<Box<dyn futures::Stream<Item = Result<M, E>> + Send + 'a>> {
    Box::pin(messages)
}

// Wire one streaming protocol step between initiator <==> responder.
//
// The streaming traits expose each step as `(outgoing_stream, next_state)`,
// so unlike the alternating driver there is no per-message `Step` to inspect:
// each stream is handed (boxed) directly to the counterparty's next method.
macro_rules! x {
    (let $msgs:pat = $peer:ident . $method:ident ( $($arg:expr),* $(,)? ) ) => {
        let (msgs, $peer) = $peer.$method::<E>($($arg),*);
        let $msgs = boxed(msgs);
    };
    ($sender:ident . $sender_method:ident == $msgs:ident => $receiver:ident . $receiver_method:ident) => {
        #[allow(unused)]
        let ($msgs, $sender, $receiver) = {
            let (msgs, next) = $sender.$sender_method::<E>($msgs);
            (boxed(msgs), next, $receiver)
        };
    };
    // The session's terminal: the initiator's completion and the responder's
    // drive future are joined, so every pump on both sides is polled
    // unconditionally until the whole session is done (see
    // `CompleteResponder::complete_responder` for why the responder's wire
    // stream alone is not enough).
    ($initiator:ident . complete_initiator <= $msgs:ident == $responder:ident . complete_responder) => {{
        let (msgs, drive) = $responder.complete_responder::<E>($msgs);
        let msgs = boxed(msgs);
        let (initiated, driven) =
            futures::future::join($initiator.complete_initiator::<E>(msgs), drive).await;
        initiated.and(driven)
    }};
    ($receiver:ident . $receiver_method:ident <= $msgs:ident == $sender:ident . $sender_method:ident) => {
        #[allow(unused)]
        let ($msgs, $receiver, $sender) = {
            let (msgs, next) = $sender.$sender_method::<E>($msgs);
            (boxed(msgs), $receiver, next)
        };
    };
}

async fn mirror_connected<B, I, R, T, E>(i: I, r: R) -> Result<(), E>
where
    T: Send + Sync + 'static,
    B: Backend<T, Node<Z>: Leaf<T>> + 'static,
    I: Peer<B, T>,
    R: Peer<B, T>,
    E: From<I::Error> + From<R::Error> + From<B::Error> + Send + 'static,
{
    x! { let x = i.initiator() }
    x! { i.open_initiator <=x== r.responder }
    x! { i.open_initiator ==x=> r.exchange }
    seq!(_ in 0..14 {
        x! { i.exchange <=x== r.exchange }
        x! { i.exchange ==x=> r.exchange }
    });
    x! { i.close_initiator    <=x== r.exchange }
    x! { i.close_initiator    ==x=> r.complete_responder }
    x! { i.complete_initiator <=x== r.complete_responder }
}

pub(crate) type ClientConnected<C, B, T> =
    <<C as Connect<B, T>>::Next as CompleteConnect<B, T>>::Next;
pub(crate) type ServerConnected<S, B, T> = <S as Accept<B, T>>::Next;

pub(crate) struct Handshaken<C, S, B, T>
where
    T: Send + Sync + 'static,
    B: Backend<T, Node<Z>: Leaf<T>> + 'static,
    C: Client<B, T>,
    S: Server<B, T>,
{
    local: ClientConnected<C, B, T>,
    remote: ServerConnected<S, B, T>,
    our_version: Version,
    peer: message::Handshake,
}

impl<C, S, B, T> Handshaken<C, S, B, T>
where
    T: Send + Sync + 'static,
    B: Backend<T, Node<Z>: Leaf<T>> + 'static,
    C: Client<B, T>,
    S: Server<B, T>,
{
    pub(crate) fn peer(&self) -> &message::Handshake {
        let Handshaken { peer, .. } = self;
        peer
    }

    pub(crate) async fn reconcile<E>(self) -> Result<(), E>
    where
        E: From<C::Error> + From<S::Error> + From<B::Error> + Send + 'static,
    {
        let Handshaken {
            local,
            remote,
            our_version,
            peer,
        } = self;
        descend(local, remote, our_version, peer.version).await
    }
}

pub(crate) async fn handshake<C, S, B, T, E>(c: C, s: S) -> Result<Handshaken<C, S, B, T>, E>
where
    T: Send + Sync + 'static,
    B: Backend<T, Node<Z>: Leaf<T>> + 'static,
    C: Client<B, T>,
    S: Server<B, T>,
    E: From<C::Error> + From<S::Error> + From<B::Error> + Send + 'static,
{
    let (our_handshake, c) = c.connect::<E>().await?;
    let our_version = our_handshake.version.clone();
    let (peer, s) = s.accept::<E>(our_handshake).await?;
    let c = c.complete_connect::<E>(peer.version.clone()).await?;

    Ok(Handshaken {
        local: c,
        remote: s,
        our_version,
        peer,
    })
}

pub(crate) async fn descend<L, R, B, T, E>(
    local: L,
    remote: R,
    local_version: Version,
    remote_version: Version,
) -> Result<(), E>
where
    T: Send + Sync + 'static,
    B: Backend<T, Node<Z>: Leaf<T>> + 'static,
    L: Peer<B, T>,
    R: Peer<B, T>,
    E: From<L::Error> + From<R::Error> + From<B::Error> + Send + 'static,
{
    match remote_version.as_bytes().cmp(local_version.as_bytes()) {
        Ordering::Less => mirror_connected::<B, L, R, T, E>(local, remote).await,
        Ordering::Greater => mirror_connected::<B, R, L, T, E>(remote, local).await,
        Ordering::Equal => Ok(()),
    }
}

/// Reconcile two in-memory trees through the full streaming protocol, both
/// parties polled concurrently on the current task, returning both sides'
/// reconciled roots.
///
/// This is the local-to-local harness the streaming proptests drive against
/// the alternating oracle. The generic drivers above stay `()`-returning and
/// wire-agnostic; the reconciled roots come back through the recovery slots
/// [`Handshaking::start`] hands out. A session that never reaches
/// reconciliation (the versions were equal) drops those slots, in which case
/// each side's tree is already converged and returned unchanged.
#[cfg(test)]
pub(crate) async fn mirror<T>(
    network: crate::Network,
    local: crate::tree::Root<T>,
    remote: crate::tree::Root<T>,
) -> (crate::tree::Root<T>, crate::tree::Root<T>)
where
    T: Send + Sync + 'static,
{
    use std::convert::Infallible;

    let (client, local_recovered) = Handshaking::start(
        Local,
        network,
        message::Intent::Remain,
        local.clone().into(),
    );
    let (server, remote_recovered) = Handshaking::start(
        Local,
        network,
        message::Intent::Remain,
        remote.clone().into(),
    );

    let handshaken = handshake::<_, _, Local, T, Infallible>(client, server)
        .await
        .expect("local handshake is infallible");
    handshaken
        .reconcile::<Infallible>()
        .await
        .expect("local reconciliation is infallible");

    (
        local_recovered.await.map(Into::into).unwrap_or(local),
        remote_recovered.await.map(Into::into).unwrap_or(remote),
    )
}
