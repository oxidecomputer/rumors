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
use std::pin::Pin;

use async_stream::try_stream;
use seq_macro::seq;

use super::Error;
use crate::{Version, tree::typed::height::Z};
use convert::Convertible;
use protocol::*;

#[cfg(test)]
mod tests;

/// The two-sided session [`Error`], seen from one party's perspective.
type CombinedError<B, C, T> = Error<<B as Backend<T>>::Error, <C as Backend<T>>::Error>;

/// A boxed [`Messages`] stream: what [`wire`] hands between stages.
type BoxMessages<M, E> = Pin<Box<dyn Messages<M, E>>>;

/// One direction of the boundary between participants: re-represent each
/// crossing message from `from`'s node types into `to`'s ([`Convertible`]), and
/// each crossing error from `from`'s [`CombinedError`] frame into `to`'s.
///
/// Boxed because each wire wraps the stage stream it converts: handed around
/// unboxed, the driver would nest thirty stages of stream machinery into one
/// type and stall the compiler's trait solving.
fn wire<F, G, T, M, N>(
    from: &F,
    to: &G,
    messages: impl Messages<M, CombinedError<F, G, T>> + 'static,
) -> BoxMessages<N, CombinedError<G, F, T>>
where
    F: Backend<T, Node<Z>: Leaf<T>>,
    G: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    // `N` names the converted message type rather than projecting it from
    // `M`: the source pins `M`, the receiving stage pins `N`, and inference
    // meets in this bound — a projection in return position would have to be
    // concrete before the receiver constrains it (the boxed wires erase to
    // `dyn`, whose item type is fixed at the coercion).
    M: Convertible<F, G, T, Converted = N> + 'static,
    N: Send + 'static,
{
    let (from, to) = (from.clone(), to.clone());
    Box::pin(try_stream! {
        for await item in messages {
            match item {
                Ok(message) => yield message.convert(&from, &to).await?,
                Err(error) => Err(error.flip())?,
            }
        }
    })
}

/// Drive the full protocol schedule between two connected peers.
///
/// The streaming traits expose each step as `(outgoing_stream, next_state)`,
/// so unlike the alternating driver there is no per-message `Step` to
/// inspect and no early return: the schedule is a straight line, each stage's
/// outgoing stream [wired](wire) across the party boundary to the
/// counterparty's next stage. Each party runs at its own [`CombinedError`]
/// frame (the protocol traits' `E`), so every error travels in-band to
/// whichever terminal it interrupts. `B` is the initiator's backend and `C`
/// the responder's; the result is the initiator's frame, and [`descend`]
/// flips it back when the version tiebreak swapped the roles.
async fn mirror_connected<B, C, I, R, T>(
    initiator: B,
    responder: C,
    i: I,
    r: R,
) -> Result<(Root<B, T>, Root<C, T>), Error<B::Error, C::Error>>
where
    T: Send + Sync + 'static,
    B: Backend<T, Node<Z>: Leaf<T>>,
    C: Backend<T, Node<Z>: Leaf<T>>,
    I: Peer<B, T>,
    R: Peer<C, T>,
{
    let (msgs, i) = i.initiator::<CombinedError<B, C, T>>();
    let msgs = wire(&initiator, &responder, msgs);
    let (msgs, r) = r.responder::<CombinedError<C, B, T>>(msgs);
    let msgs = wire(&responder, &initiator, msgs);
    let (msgs, i) = i.open_initiator::<CombinedError<B, C, T>>(msgs);
    let msgs = wire(&initiator, &responder, msgs);
    seq!(_ in 0..14 {
        let (msgs, r) = r.exchange::<CombinedError<C, B, T>>(msgs);
        let msgs = wire(&responder, &initiator, msgs);
        let (msgs, i) = i.exchange::<CombinedError<B, C, T>>(msgs);
        let msgs = wire(&initiator, &responder, msgs);
    });
    let (msgs, r) = r.exchange::<CombinedError<C, B, T>>(msgs);
    let msgs = wire(&responder, &initiator, msgs);
    let (msgs, i) = i.close_initiator::<CombinedError<B, C, T>>(msgs);
    let msgs = wire(&initiator, &responder, msgs);
    let (msgs, drive) = r.complete_responder::<CombinedError<C, B, T>>(msgs);
    let msgs = wire(&responder, &initiator, msgs);
    let (initiated, driven) =
        futures::future::join(i.complete_initiator::<CombinedError<B, C, T>>(msgs), drive).await;
    Ok((initiated?, driven.map_err(Error::flip)?))
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

    /// Reconcile the two connected sessions, returning both sides' reconciled
    /// roots.
    ///
    /// Returns `None` when the handshake versions were equal and the trees
    /// are already converged, in which case each side's root is whatever the
    /// caller already holds.
    pub(crate) async fn reconcile(
        self,
    ) -> Result<Option<(Root<B, T>, Root<C, T>)>, Error<B::Error, C::Error>> {
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
) -> Result<Option<(Root<B, T>, Root<C, T>)>, Error<B::Error, C::Error>>
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
        Ordering::Less => mirror_connected(local_backend, remote_backend, local, remote)
            .await
            .map(Some),
        // Running the remote as initiator, flip the roots and the error's
        // sides back.
        Ordering::Greater => mirror_connected(remote_backend, local_backend, remote, local)
            .await
            .map(|(theirs, ours)| Some((ours, theirs)))
            .map_err(Error::flip),
        // Equal versions mean already-converged trees: nothing to reconcile,
        // and each side keeps the root it came with.
        Ordering::Equal => Ok(None),
    }
}

/// Reconcile two trees held by (possibly different) backends through the full
/// streaming protocol, both parties polled concurrently on the current task,
/// returning both sides' reconciled roots.
///
/// [`wire`] re-represents each message's nodes across the two backends — the
/// in-process analog of a wire transport's serialization. A session that
/// never reaches reconciliation (the versions were equal) returns both trees
/// unchanged: they are already converged.
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
    let client = Handshaking::start(local_backend.clone(), local.clone());
    let server = Handshaking::start(remote_backend.clone(), remote.clone());

    let reconciled = handshake(local_backend, remote_backend, client, server)
        .await?
        .reconcile()
        .await?;

    Ok(reconciled.unwrap_or((local, remote)))
}
