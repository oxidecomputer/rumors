// TODO: Under construction, remove when done!
#![allow(unused)]

mod backend;
mod convert;
mod message;
mod protocol;

pub use backend::{Backend, Leaf, Local, Node};

use std::cmp::Ordering;

use crate::{Version, tree::typed::height::Z};
use protocol::*;
use seq_macro::seq;

// Wire one streaming protocol step between initiator <==> responder.
//
// The streaming traits expose each step as `(outgoing_stream, next_state)`,
// so unlike the alternating driver there is no per-message `Step` to inspect:
// each stream is handed directly to the counterparty's next method.
macro_rules! x {
    (let $msgs:pat = $peer:ident . $method:ident ( $($arg:expr),* $(,)? ) ) => {
        let ($msgs, $peer) = $peer.$method::<E>($($arg),*);
    };
    ($sender:ident . $sender_method:ident == $msgs:ident => $receiver:ident . $receiver_method:ident) => {
        #[allow(unused)]
        let ($msgs, $sender, $receiver) = {
            let (msgs, next) = $sender.$sender_method::<E>($msgs);
            (msgs, next, $receiver)
        };
    };
    ($initiator:ident . complete_initiator <= $msgs:ident == $responder:ident . complete_responder) => {{
        let msgs = $responder.complete_responder::<E>($msgs);
        $initiator.complete_initiator::<E>(msgs).await
    }};
    ($receiver:ident . $receiver_method:ident <= $msgs:ident == $sender:ident . $sender_method:ident) => {
        #[allow(unused)]
        let ($msgs, $receiver, $sender) = {
            let (msgs, next) = $sender.$sender_method::<E>($msgs);
            (msgs, $receiver, next)
        };
    };
}

async fn mirror_connected<B, I, R, T, E>(i: I, r: R) -> Result<(), E>
where
    T: Send + Sync,
    B: Backend<T, Node<Z>: Leaf<T>>,
    I: Peer<B, T>,
    R: Peer<B, T>,
    E: From<I::Error> + From<R::Error> + From<B::Error>,
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
    T: Send + Sync,
    B: Backend<T, Node<Z>: Leaf<T>>,
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
    T: Send + Sync,
    B: Backend<T, Node<Z>: Leaf<T>>,
    C: Client<B, T>,
    S: Server<B, T>,
{
    pub(crate) fn peer(&self) -> &message::Handshake {
        match self {
            Handshaken { peer, .. } => peer,
        }
    }

    pub(crate) async fn reconcile<E>(self) -> Result<(), E>
    where
        E: From<C::Error> + From<S::Error> + From<B::Error>,
    {
        match self {
            Handshaken {
                local,
                remote,
                our_version,
                peer,
            } => descend(local, remote, our_version, peer.version).await,
        }
    }
}

pub(crate) async fn handshake<C, S, B, T, E>(c: C, s: S) -> Result<Handshaken<C, S, B, T>, E>
where
    T: Send + Sync,
    B: Backend<T, Node<Z>: Leaf<T>>,
    C: Client<B, T>,
    S: Server<B, T>,
    E: From<C::Error> + From<S::Error> + From<B::Error>,
{
    let (our_handshake, c) = c.connect::<E>().await.map_err(E::from)?;
    let our_version = our_handshake.version.clone();
    let (peer, s) = s.accept::<E>(our_handshake).await.map_err(E::from)?;
    let c = c
        .complete_connect::<E>(peer.version.clone())
        .await
        .map_err(E::from)?;

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
    T: Send + Sync,
    B: Backend<T, Node<Z>: Leaf<T>>,
    L: Peer<B, T>,
    R: Peer<B, T>,
    E: From<L::Error> + From<R::Error> + From<B::Error>,
{
    match remote_version.as_bytes().cmp(local_version.as_bytes()) {
        Ordering::Less => mirror_connected::<B, L, R, T, E>(local, remote).await,
        Ordering::Greater => mirror_connected::<B, R, L, T, E>(remote, local).await,
        Ordering::Equal => Ok(()),
    }
}
