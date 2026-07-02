//! The streaming mirror: fixed-memory reconciliation over lazy node streams.
//!
//! The pieces, bottom up:
//!
//! - [`backend`]: what a party must provide. [`Backend`] itself asks only
//!   for prefix-ordered re-chunking of opaque nodes — weak enough for a
//!   wire party whose "nodes" are framed leaf sequences — with the
//!   inspection operations dispatched by [`Materiality`]: the session's
//!   walks demand `Materialized = `[`Material`], the layers above accept
//!   either. [`Leaf`] is the crossing currency every party represents
//!   faithfully.
//! - [`protocol`]: the type-level phase schedule both parties advance
//!   through, generic over any backend.
//! - [`session`]: the schedule implemented once for every *material*
//!   backend, as concurrent walks over lazy streams.
//! - [`convert`]: the party boundary, where one side's nodes re-represent
//!   in the other's types by meeting at the leaves — what a wire transport
//!   will do implicitly when it serializes one side and deserializes into
//!   the other.
//!
//! The drivers here run any two protocol implementors against each other
//! ([`mirror`], or [`handshake`] and [`Handshaken::reconcile`] separately
//! around the version exchange); implementors backed by trees start with
//! [`Handshaking::start`].

// TODO: remove this when integrated
#![allow(dead_code, unused_imports)]

mod backend;
mod convert;
mod dispute;
mod merge;
mod message;
mod protocol;
mod session;
mod unknown;

pub use backend::{Backend, Immaterial, Leaf, Local, Material, Materiality, Node, Root};
pub use session::Handshaking;

use std::cmp::Ordering;
use std::pin::Pin;

use futures::{StreamExt, join};
use seq_macro::seq;

use super::Error;
use crate::Version;
use protocol::*;

#[cfg(test)]
mod tests;

/// The two-sided session [`Error`], seen from one party's perspective.
type CombinedError<B, C, T> = Error<<B as Backend<T>>::Error, <C as Backend<T>>::Error>;

/// A boxed [`Messages`] stream: what [`flip`] hands between stages.
type BoxMessages<M, E> = Pin<Box<dyn Messages<M, E>>>;

/// The boundary between participants: messages cross unchanged, errors
/// [flip](Error::flip) from the sender's frame into the receiver's.
///
/// Messages need no translation because both implementors speak the same
/// backend's node types; the flip keeps every error reporting in the arm of
/// the party that raised it.
///
/// Boxed because each crossing wraps the stage stream before it: handed
/// around unboxed, the driver would nest thirty stages of stream machinery
/// into one type and stall the compiler's trait solving.
fn flip<M, C, S>(messages: impl Messages<M, Error<C, S>> + 'static) -> BoxMessages<M, Error<S, C>>
where
    M: Send + 'static,
    C: Send + 'static,
    S: Send + 'static,
{
    Box::pin(messages.map(|item| item.map_err(Error::flip)))
}

/// Drive the full protocol schedule between two connected peers.
///
/// The streaming traits expose each step as `(outgoing_stream, next_state)`,
/// so unlike the alternating driver there is no per-message `Step` to
/// inspect and no early return: the schedule is a straight line, each stage's
/// outgoing stream handed across the party boundary ([`flip`]) to the
/// counterparty's next stage.
async fn mirror_connected<B, I, R, T>(
    i: I,
    r: R,
) -> Result<(Root<B, T>, Root<B, T>), Error<B::Error, B::Error>>
where
    T: Send + Sync + 'static,
    B: Backend<T>,
    I: Peer<B, T>,
    R: Peer<B, T>,
{
    let (msgs, i) = i.initiator();
    let msgs = flip(msgs);

    let (msgs, r) = r.responder(msgs);
    let msgs = flip(msgs);

    let (msgs, i) = i.open_initiator(msgs);
    let msgs = flip(msgs);

    seq!(_ in 0..14 {
        let (msgs, r) = r.exchange(msgs);
        let msgs = flip(msgs);

        let (msgs, i) = i.exchange(msgs);
        let msgs = flip(msgs);
    });

    let (msgs, r) = r.exchange(msgs);
    let msgs = flip(msgs);

    let (msgs, i) = i.close_initiator(msgs);
    let msgs = flip(msgs);

    let (msgs, r) = r.complete_responder(msgs);
    let msgs = flip(msgs);

    let (i, r) = join!(i.complete_initiator(msgs), r);
    Ok((i?, r.map_err(Error::flip)?))
}

type ClientConnected<C, B, T> = <<C as Connect<B, T>>::Next as CompleteConnect<B, T>>::Next;
type ServerConnected<S, B, T> = <S as Accept<B, T>>::Next;

pub(crate) struct Handshaken<P, Q, B, T>
where
    T: Send + Sync + 'static,
    B: Backend<T>,
    P: Client<B, T>,
    Q: Server<B, T>,
{
    local: ClientConnected<P, B, T>,
    remote: ServerConnected<Q, B, T>,
    our_version: Version,
    peer: message::Handshake,
}

impl<P, Q, B, T> Handshaken<P, Q, B, T>
where
    T: Send + Sync + 'static,
    B: Backend<T>,
    P: Client<B, T>,
    Q: Server<B, T>,
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
    ) -> Result<Option<(Root<B, T>, Root<B, T>)>, Error<B::Error, B::Error>> {
        let Handshaken {
            local,
            remote,
            our_version,
            peer,
        } = self;
        descend(local, remote, our_version, peer.version).await
    }
}

pub(crate) async fn handshake<P, Q, B, T>(
    c: P,
    s: Q,
) -> Result<Handshaken<P, Q, B, T>, Error<B::Error, B::Error>>
where
    T: Send + Sync + 'static,
    B: Backend<T>,
    P: Client<B, T>,
    Q: Server<B, T>,
{
    // The handshake carries only versions; each side's errors wrap into its
    // own arm here.
    let (our_handshake, c) = c.connect::<B::Error>().await.map_err(Error::Client)?;
    let our_version = our_handshake.version.clone();
    let (peer, s) = s
        .accept::<B::Error>(our_handshake)
        .await
        .map_err(Error::Server)?;
    let c = c
        .complete_connect::<B::Error>(peer.version.clone())
        .await
        .map_err(Error::Client)?;

    Ok(Handshaken {
        local: c,
        remote: s,
        our_version,
        peer,
    })
}

pub(crate) async fn descend<L, R, B, T>(
    local: L,
    remote: R,
    local_version: Version,
    remote_version: Version,
) -> Result<Option<(Root<B, T>, Root<B, T>)>, Error<B::Error, B::Error>>
where
    T: Send + Sync + 'static,
    B: Backend<T>,
    L: Peer<B, T>,
    R: Peer<B, T>,
{
    // Their causal order is only partial (they may be concurrent), so to pick
    // an initiator we compare canonical bytes lexicographically: an arbitrary
    // but total, deterministic tiebreak (not a causal order).
    match remote_version.as_bytes().cmp(local_version.as_bytes()) {
        Ordering::Less => mirror_connected(local, remote).await.map(Some),
        // Running the remote as initiator, flip the roots and the error's
        // sides back.
        Ordering::Greater => mirror_connected(remote, local)
            .await
            .map(|(theirs, ours)| Some((ours, theirs)))
            .map_err(Error::flip),
        // Equal versions mean already-converged trees: nothing to reconcile,
        // and each side keeps the root it came with. Both parties compare the
        // same two versions, so a remote counterparty concludes this
        // identically on its own side: no message needs to say so.
        Ordering::Equal => Ok(None),
    }
}

/// Run two arbitrary protocol implementors against each other through the
/// full streaming protocol, both parties polled concurrently on the current
/// task, returning both sides' reconciled roots.
///
/// The implementors need not be backed by materialized trees; any pair of
/// [`Client`] and [`Server`] sharing a backend type will do. Messages cross
/// between them unchanged — an implementor that fronts a remote peer is
/// generic over the backend, decoding what arrives on its wire into that
/// backend's nodes — so the driver interposes nothing but the error-frame
/// [`flip`]. Start a tree-backed session with [`Handshaking::start`].
///
/// Returns `None` when the handshake versions were equal and there is
/// nothing to reconcile, in which case each side keeps its state untouched.
pub(crate) async fn mirror<P, Q, B, T>(
    client: P,
    server: Q,
) -> Result<Option<(Root<B, T>, Root<B, T>)>, Error<B::Error, B::Error>>
where
    T: Send + Sync + 'static,
    B: Backend<T>,
    P: Client<B, T>,
    Q: Server<B, T>,
{
    handshake(client, server).await?.reconcile().await
}
