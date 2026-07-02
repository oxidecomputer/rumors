// TODO-integration: the streaming mirror is complete and oracle-tested but
// not yet wired into the crate's session layer, so outside `cfg(test)` the
// whole module is dead code (imports are still checked). Remove this allow
// and the re-export allows below when `Peer` adopts it.
#![allow(dead_code)]

mod backend;
mod convert;
mod dispute;
mod merge;
mod message;
mod protocol;
mod session;
mod unknown;

#[allow(unused_imports)]
pub use backend::{Backend, Leaf, Local, Node, Root};
#[allow(unused_imports)]
pub use session::Handshaking;

use std::cmp::Ordering;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use async_stream::try_stream;
use seq_macro::seq;

use super::Error;
use crate::{Version, tree::typed::height::Z};
use convert::Convertible;
use protocol::*;

#[cfg(test)]
mod tests;

/// Erase a message stream's concrete type at a party boundary.
///
/// Each boundary's hop stream wraps the stage stream it converts; handed
/// around unboxed, the driver below would nest thirty stages of stream
/// machinery into one type and stall the compiler's trait solving. Boxing at
/// every hop keeps each stage's monomorphized type one stage deep — the same
/// reason every height-recursive transducer here returns a boxed stream.
fn boxed<'a, M: 'a, E: 'a>(
    messages: impl Messages<M, E> + 'a,
) -> std::pin::Pin<Box<dyn futures::Stream<Item = Result<M, E>> + Send + 'a>> {
    Box::pin(messages)
}

/// The session's fault slot: where the party boundary deposits an error that
/// cannot cross it.
///
/// A stream error originating on one side has no representation in the
/// counterparty's error vocabulary, so the boundary diverts it here and ends
/// the stream instead; the counterparty observes an ordinary early
/// end-of-stream and winds down. The driver checks the slot once the session
/// settles — before trusting either terminal's result, since a diverted fault
/// explains any downstream oddity. The first fault wins.
struct Fault<E>(Arc<Mutex<Option<E>>>);

impl<E> Fault<E> {
    fn new() -> Self {
        Self(Arc::default())
    }

    fn set(&self, error: E) {
        self.0
            .lock()
            .expect("fault slot poisoned")
            .get_or_insert(error);
    }

    fn take(&self) -> Option<E> {
        self.0.lock().expect("fault slot poisoned").take()
    }
}

impl<E> Clone for Fault<E> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// One direction of the party boundary: re-represent each crossing message
/// from `from`'s node types into `to`'s ([`Convertible`]).
///
/// Every `from`-side error — stream or conversion — diverts into the fault
/// slot. `to`-side conversion errors flow in-band: they are the output
/// stream's own error type, and the receiving stage owns them like any other
/// backend failure.
fn hop<F, G, T, M, N, X>(
    from: F,
    to: G,
    fault: Fault<X>,
    wrap: impl Fn(F::Error) -> X + Clone + Send + 'static,
    messages: impl Messages<M, F::Error> + 'static,
) -> impl Messages<N, G::Error> + 'static
where
    F: Backend<T, Node<Z>: Leaf<T>>,
    G: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    // `N` names the converted message type rather than projecting it from
    // `M`: the source pins `M`, the receiving stage pins `N`, and inference
    // meets in this bound — a projection in return position would have to be
    // concrete before the receiver constrains it (the boxed hops erase to
    // `dyn Stream`, whose item type is fixed at the coercion).
    M: Convertible<F, G, T, Converted = N> + 'static,
    N: Send + 'static,
    X: Send + 'static,
{
    let divert = {
        let (fault, wrap) = (fault.clone(), wrap.clone());
        move |error| fault.set(wrap(error))
    };
    try_stream! {
        for await item in messages {
            match item {
                Ok(message) => match message.convert(&from, &to, divert.clone()).await? {
                    Some(message) => yield message,
                    // A source-side conversion failure went to the fault
                    // slot; end the stream as if the source had stopped.
                    None => return,
                },
                Err(error) => {
                    fault.set(wrap(error));
                    return;
                }
            }
        }
    }
}

/// The party boundary of an in-process session: both backends, the fault
/// slot, and the initiator/responder orientation of the current descent.
///
/// This is the in-process analog of a wire transport: where a socket
/// serializes one side's nodes and deserializes them into the other's, the
/// boundary [hops](hop) each message stream across the two node vocabularies.
/// `I` is the initiator's backend and `R` the responder's; faults wrap into
/// the [`Error`] frame where [`Client`](Error::Client) is the initiator, and
/// [`descend`] flips the frame back when the version tiebreak swapped the
/// roles.
struct Boundary<I, R, T>
where
    I: Backend<T, Node<Z>: Leaf<T>>,
    R: Backend<T, Node<Z>: Leaf<T>>,
{
    initiator: I,
    responder: R,
    fault: Fault<Error<I::Error, R::Error>>,
    messages: PhantomData<fn() -> T>,
}

impl<I, R, T> Boundary<I, R, T>
where
    I: Backend<T, Node<Z>: Leaf<T>>,
    R: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    fn new(initiator: I, responder: R) -> Self {
        Self {
            initiator,
            responder,
            fault: Fault::new(),
            messages: PhantomData,
        }
    }

    /// Carry an initiator-side wire stream across to the responder.
    fn to_responder<M, N>(
        &self,
        messages: impl Messages<M, I::Error> + 'static,
    ) -> impl Messages<N, R::Error> + 'static
    where
        M: Convertible<I, R, T, Converted = N> + 'static,
        N: Send + 'static,
    {
        hop(
            self.initiator.clone(),
            self.responder.clone(),
            self.fault.clone(),
            Error::Client,
            messages,
        )
    }

    /// Carry a responder-side wire stream across to the initiator.
    fn to_initiator<M, N>(
        &self,
        messages: impl Messages<M, R::Error> + 'static,
    ) -> impl Messages<N, I::Error> + 'static
    where
        M: Convertible<R, I, T, Converted = N> + 'static,
        N: Send + 'static,
    {
        hop(
            self.responder.clone(),
            self.initiator.clone(),
            self.fault.clone(),
            Error::Server,
            messages,
        )
    }
}

// Wire one streaming protocol step between initiator <==> responder.
//
// The streaming traits expose each step as `(outgoing_stream, next_state)`,
// so unlike the alternating driver there is no per-message `Step` to inspect:
// each stream crosses the party boundary (boxed) on its way to the
// counterparty's next method. Each party runs in its own error type (the
// protocol traits' `E` is instantiated at its backend's error); the boundary
// converts message payloads and diverts errors, so the sum [`Error`] appears
// only at the driver level.
macro_rules! x {
    ($b:ident: let $msgs:pat = $peer:ident . $method:ident ( $($arg:expr),* $(,)? ) ) => {
        let (msgs, $peer) = $peer.$method::<B::Error>($($arg),*);
        let $msgs = boxed($b.to_responder(msgs));
    };
    ($b:ident: $sender:ident . $sender_method:ident == $msgs:ident => $receiver:ident) => {
        #[allow(unused)]
        let ($msgs, $sender, $receiver) = {
            let (msgs, next) = $sender.$sender_method::<B::Error>($msgs);
            (boxed($b.to_responder(msgs)), next, $receiver)
        };
    };
    // The session's terminal: the initiator's completion and the responder's
    // drive future are joined, so every pump on both sides is polled
    // unconditionally until the whole session is done (see
    // `CompleteResponder::complete_responder` for why the responder's wire
    // stream alone is not enough). A diverted boundary fault outranks either
    // terminal's result: downstream of a fault, both sides wind down early.
    ($b:ident: $initiator:ident . complete_initiator <= $msgs:ident == $responder:ident . complete_responder) => {{
        let (msgs, drive) = $responder.complete_responder::<C::Error>($msgs);
        let msgs = boxed($b.to_initiator(msgs));
        let (initiated, driven) =
            futures::future::join($initiator.complete_initiator::<B::Error>(msgs), drive).await;
        if let Some(fault) = $b.fault.take() {
            return Err(fault);
        }
        initiated.map_err(Error::Client)?;
        driven.map_err(Error::Server)
    }};
    ($b:ident: $receiver:ident <= $msgs:ident == $sender:ident . $sender_method:ident) => {
        #[allow(unused)]
        let ($msgs, $receiver, $sender) = {
            let (msgs, next) = $sender.$sender_method::<C::Error>($msgs);
            (boxed($b.to_initiator(msgs)), $receiver, next)
        };
    };
}

async fn mirror_connected<B, C, I, R, T>(
    boundary: Boundary<B, C, T>,
    i: I,
    r: R,
) -> Result<(), Error<B::Error, C::Error>>
where
    T: Send + Sync + 'static,
    B: Backend<T, Node<Z>: Leaf<T>>,
    C: Backend<T, Node<Z>: Leaf<T>>,
    I: Peer<B, T>,
    R: Peer<C, T>,
{
    x! { boundary: let x = i.initiator() }
    x! { boundary: i <=x== r.responder }
    x! { boundary: i.open_initiator ==x=> r }
    seq!(_ in 0..14 {
        x! { boundary: i <=x== r.exchange }
        x! { boundary: i.exchange ==x=> r }
    });
    x! { boundary: i <=x== r.exchange }
    x! { boundary: i.close_initiator ==x=> r }
    x! { boundary: i.complete_initiator <=x== r.complete_responder }
}

pub(crate) type ClientConnected<C, B, T> =
    <<C as Connect<B, T>>::Next as CompleteConnect<B, T>>::Next;
pub(crate) type ServerConnected<S, B, T> = <S as Accept<B, T>>::Next;

pub(crate) struct Handshaken<P, Q, B, C, T>
where
    T: Send + Sync + 'static,
    B: Backend<T, Node<Z>: Leaf<T>>,
    C: Backend<T, Node<Z>: Leaf<T>>,
    P: Client<B, T>,
    Q: Server<C, T>,
{
    local_backend: B,
    remote_backend: C,
    local: ClientConnected<P, B, T>,
    remote: ServerConnected<Q, C, T>,
    our_version: Version,
    peer: message::Handshake,
}

impl<P, Q, B, C, T> Handshaken<P, Q, B, C, T>
where
    T: Send + Sync + 'static,
    B: Backend<T, Node<Z>: Leaf<T>>,
    C: Backend<T, Node<Z>: Leaf<T>>,
    P: Client<B, T>,
    Q: Server<C, T>,
{
    pub(crate) fn peer(&self) -> &message::Handshake {
        let Handshaken { peer, .. } = self;
        peer
    }

    pub(crate) async fn reconcile(self) -> Result<(), Error<B::Error, C::Error>> {
        let Handshaken {
            local_backend,
            remote_backend,
            local,
            remote,
            our_version,
            peer,
        } = self;
        descend(
            local_backend,
            remote_backend,
            local,
            remote,
            our_version,
            peer.version,
        )
        .await
    }
}

pub(crate) async fn handshake<P, Q, B, C, T>(
    local_backend: B,
    remote_backend: C,
    c: P,
    s: Q,
) -> Result<Handshaken<P, Q, B, C, T>, Error<B::Error, C::Error>>
where
    T: Send + Sync + 'static,
    B: Backend<T, Node<Z>: Leaf<T>>,
    C: Backend<T, Node<Z>: Leaf<T>>,
    P: Client<B, T>,
    Q: Server<C, T>,
{
    // The handshake carries only versions, so it crosses the party boundary
    // without conversion; each side's errors wrap into its own arm here.
    let (our_handshake, c) = c.connect::<B::Error>().await.map_err(Error::Client)?;
    let our_version = our_handshake.version.clone();
    let (peer, s) = s
        .accept::<C::Error>(our_handshake)
        .await
        .map_err(Error::Server)?;
    let c = c
        .complete_connect::<B::Error>(peer.version.clone())
        .await
        .map_err(Error::Client)?;

    Ok(Handshaken {
        local_backend,
        remote_backend,
        local: c,
        remote: s,
        our_version,
        peer,
    })
}

pub(crate) async fn descend<L, R, B, C, T>(
    local_backend: B,
    remote_backend: C,
    local: L,
    remote: R,
    local_version: Version,
    remote_version: Version,
) -> Result<(), Error<B::Error, C::Error>>
where
    T: Send + Sync + 'static,
    B: Backend<T, Node<Z>: Leaf<T>>,
    C: Backend<T, Node<Z>: Leaf<T>>,
    L: Peer<B, T>,
    R: Peer<C, T>,
{
    // Their causal order is only partial (they may be concurrent), so to pick
    // an initiator we compare canonical bytes lexicographically: an arbitrary
    // but total, deterministic tiebreak (not a causal order).
    match remote_version.as_bytes().cmp(local_version.as_bytes()) {
        Ordering::Less => {
            mirror_connected(Boundary::new(local_backend, remote_backend), local, remote).await
        }
        // Running the remote as initiator, flip the error's sides back.
        Ordering::Greater => {
            mirror_connected(Boundary::new(remote_backend, local_backend), remote, local)
                .await
                .map_err(Error::flip)
        }
        Ordering::Equal => Ok(()),
    }
}

/// Reconcile two trees held by (possibly different) backends through the full
/// streaming protocol, both parties polled concurrently on the current task,
/// returning both sides' reconciled roots.
///
/// The party [`Boundary`] re-represents each message's nodes across the two
/// backends — the in-process analog of a wire transport's serialization. The
/// reconciled roots come back through the recovery slots
/// [`Handshaking::start`] hands out. A session that never reaches
/// reconciliation (the versions were equal) drops those slots, in which case
/// each side's tree is already converged and returned unchanged.
#[cfg(test)]
pub(crate) async fn mirror<B, C, T>(
    local_backend: B,
    remote_backend: C,
    local: Root<B, T>,
    remote: Root<C, T>,
) -> Result<(Root<B, T>, Root<C, T>), Error<B::Error, C::Error>>
where
    T: Send + Sync + 'static,
    B: Backend<T, Node<Z>: Leaf<T>>,
    C: Backend<T, Node<Z>: Leaf<T>>,
{
    let (client, local_recovered) = Handshaking::start(local_backend.clone(), local.clone());
    let (server, remote_recovered) = Handshaking::start(remote_backend.clone(), remote.clone());

    handshake(local_backend, remote_backend, client, server)
        .await?
        .reconcile()
        .await?;

    Ok((
        local_recovered.await.unwrap_or(local),
        remote_recovered.await.unwrap_or(remote),
    ))
}
