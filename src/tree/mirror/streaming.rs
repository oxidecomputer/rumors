//! The streaming mirror: fixed-memory reconciliation over lazy node streams.
//!
//! The drivers here run any two protocol implementors against each other
//! ([`mirror`], or [`handshake`] then [`Handshaken::reconcile`] separately
//! around the version exchange); implementors backed by trees start with
//! [`Handshaking::start`].

// TODO: remove this when integrated
#![allow(dead_code, unused_imports)]

mod backend;
mod convert;
mod materialized;
mod message;
mod protocol;
mod remote;

pub use backend::{Backend, Group, Leaf, Local, Node, Root};
pub use materialized::Handshaking;

use std::cmp::Ordering;
use std::pin::{Pin, pin};

use async_stream::stream;
use futures::{StreamExt, join};
use seq_macro::seq;
use tokio::sync::mpsc;

use super::Error;
use crate::{Version, tree::typed::height::Z};
use protocol::*;

#[cfg(test)]
mod tests;

pub(super) const FAN: usize = 256;

/// One direction of the party boundary: messages pass through unchanged,
/// while the producer's errors leave the schedule out of band, into the
/// driver's error slot.
///
/// This is what makes the incoming [`Requests`] streams structurally
/// non-erroring: the consumer never has to represent the producer's error
/// type. On an error the stream parks (`Pending` forever) rather than
/// ending: end-of-stream means phase completion to the consumer, and a
/// truncated phase would be misread, whereas a parked consumer merely stops —
/// the driver has already been handed the error and abandons the session.
fn divert<M, E, D, W>(
    messages: impl Responses<M, E>,
    slot: mpsc::Sender<D>,
    wrap: W,
) -> impl Requests<M>
where
    M: Send + 'static,
    E: Send + 'static,
    D: Send + 'static,
    W: Fn(E) -> D + Send + 'static,
{
    stream! {
        let mut messages = pin!(messages);
        while let Some(item) = messages.next().await {
            match item {
                Ok(message) => yield message,
                Err(error) => {
                    // First error wins; a later crossing finding the slot
                    // already claimed loses the race and is dropped.
                    let _ = slot.try_send(wrap(error));
                    std::future::pending::<()>().await;
                }
            }
        }
    }
}

/// Expand the protocol phase schedule into the driver's whole body: one line
/// per phase.
///
/// The expansion creates the shared one-error slot, threads each phase's
/// outgoing stream to the counterparty's next phase, races the schedule against
/// the first diverted fault, and resolves both terminals into the session's
/// `Result`, early-returning the fault if the slot fires, so it must expand in
/// tail position of a function with the session's return type.
///
/// Which party holds the last stage depends on the parity of the schedule, so
/// `parties` carries the two party identifiers down the recursion: the terminal
/// binds each result back to its own party's name, and the expansion yields
/// them in the order the caller wrote them rather than the order they finish.
macro_rules! mirror {
    (@one $a:ident >> $b:ident.$m:ident) => {
        let ((msgs, state), (tx, wrap)) = $a;
        let msgs = divert(msgs, tx.clone(), wrap);
        let $a = (state, (tx, wrap));
        let $b = ($b.0.$m(msgs), $b.1);
    };
    (@pending($a:ident) parties($p:ident, $q:ident) $b:ident.$m:ident;) => {{
        let ((msgs, state), (tx, wrap)) = $a;
        let msgs = divert(msgs, tx, wrap);
        let ($b, $a) = join!($b.0.$m(msgs), state);
        ($p, $q)
    }};
    (@pending($a:ident) parties($p:ident, $q:ident) for _ in $lo:tt..$hi:tt { $($body:tt)* } $($rest:tt)*) => {{
        seq!(_ in $lo..$hi {
            mirror!(@step($a) $($body)*);
        });
        mirror!(@pending($a) parties($p, $q) $($rest)*)
    }};
    (@pending($a:ident) parties($p:ident, $q:ident) $b:ident.$m:ident; $($rest:tt)*) => {{
        mirror!(@one $a >> $b.$m);
        mirror!(@pending($b) parties($p, $q) $($rest)*)
    }};
    (@step($a:ident) $b:ident.$m:ident; $($rest:tt)*) => {
        mirror!(@one $a >> $b.$m);
        mirror!(@step($b) $($rest)*);
    };
    (@step($a:ident)) => {};
    (@run parties($p:ident, $q:ident) $a:ident.$m:ident; $($rest:tt)*) => {{
        let $a = ($a.0.$m(), $a.1);
        mirror!(@pending($a) parties($p, $q) $($rest)*)
    }};
    ($a:ident.$m:ident; $b:ident.$n:ident; $($rest:tt)*) => {{
        let (errors, mut first_error) = mpsc::channel(1);
        // Each party's faults enter the session sum on its own side; nothing
        // crosses, so the two variant constructors are the whole mapping.
        let $a = ($a, (errors.clone(), Error::Client));
        let $b = ($b, (errors, Error::Server));
        let session = async { mirror!(@run parties($a, $b) $a.$m; $b.$n; $($rest)*) };
        let (client, server) = tokio::select! {
            results = session => results,
            Some(error) = first_error.recv() => return Err(error),
        };
        Ok((
            client.map_err(Error::Client)?,
            server.map_err(Error::Server)?,
        ))
    }};
}

/// Drive the full protocol schedule between two connected peers.
///
/// The streaming traits expose each step as `(outgoing_stream, next_state)`, so
/// unlike the alternating driver there is no per-message `Step` to inspect and
/// no early return: the schedule is a straight line, each stage's outgoing
/// stream handed to the counterparty's next one.
///
/// Both parties speak one backend `B`, so a message crosses the party boundary
/// in the node types the receiver already reads. Errors never cross at all:
/// each crossing [`divert`]s the producer's errors out of band into a shared
/// one-error slot, and the driver races the session against the slot.
async fn mirror_connected<B, I, R, T>(
    i: I,
    r: R,
) -> Result<(I::Output, R::Output), Error<I::Error, R::Error>>
where
    T: Send + Sync + 'static,
    B: Backend<T, Node<Z>: Leaf<T>>,
    I: Peer<B, T>,
    R: Peer<B, T>,
{
    mirror! {
        i.initiator;
        r.open_responder;
        for _ in 0..14 {
            i.exchange;
            r.exchange;
        }
        i.exchange;
        r.close_responder;
        i.close_initiator;
        r.complete_responder;
        i.complete_initiator;
    }
}

type ClientConnected<C, B, T> = <<C as Connect<B, T>>::Next as CompleteConnect<B, T>>::Next;
type ServerConnected<S, B, T> = <S as Accept<B, T>>::Next;

pub(crate) struct Handshaken<C, S, B, T>
where
    T: Send + Sync + 'static,
    B: Backend<T, Node<Z>: Leaf<T>>,
    C: Client<B, T>,
    S: Server<B, T>,
{
    client: ClientConnected<C, B, T>,
    server: ServerConnected<S, B, T>,
    our_version: Version,
    peer: message::Handshake,
}

impl<C, S, B, T> Handshaken<C, S, B, T>
where
    T: Send + Sync + 'static,
    B: Backend<T, Node<Z>: Leaf<T>>,
    C: Client<B, T>,
    S: Server<B, T>,
{
    pub(crate) fn peer(&self) -> &message::Handshake {
        let Handshaken { peer, .. } = self;
        peer
    }

    /// Reconcile the two connected sessions, returning both sides' reconciled
    /// roots.
    ///
    /// Returns `None` when the handshake versions were equal and the trees are
    /// already converged, in which case each side's root is whatever the caller
    /// already holds.
    pub(crate) async fn reconcile(
        self,
    ) -> Result<Option<(C::Output, S::Output)>, Error<C::Error, S::Error>> {
        let Handshaken {
            client: local,
            server: remote,
            our_version,
            peer,
        } = self;
        descend(local, remote, our_version, peer.version).await
    }
}

pub(crate) async fn handshake<C, S, B, T>(
    c: C,
    s: S,
) -> Result<Handshaken<C, S, B, T>, Error<C::Error, S::Error>>
where
    T: Send + Sync + 'static,
    B: Backend<T, Node<Z>: Leaf<T>>,
    C: Client<B, T>,
    S: Server<B, T>,
{
    // The handshake carries only versions, so it crosses the party boundary
    // without conversion; each side's errors wrap into its own arm here.
    let (our_handshake, c) = c.connect().await.map_err(Error::Client)?;
    let our_version = our_handshake.version.clone();
    let (peer, s) = s.accept(our_handshake).await.map_err(Error::Server)?;
    let c = c
        .complete_connect(peer.version.clone())
        .await
        .map_err(Error::Client)?;

    Ok(Handshaken {
        client: c,
        server: s,
        our_version,
        peer,
    })
}

pub(crate) async fn descend<L, R, B, T>(
    local: L,
    remote: R,
    local_version: Version,
    remote_version: Version,
) -> Result<Option<(L::Output, R::Output)>, Error<L::Error, R::Error>>
where
    T: Send + Sync + 'static,
    B: Backend<T, Node<Z>: Leaf<T>>,
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

/// Run two arbitrary protocol implementors against each other through the full
/// streaming protocol, both parties polled concurrently on the current task,
/// returning both sides' reconciled roots.
///
/// The two implementors share one backend `B`, whose node types are the wire
/// vocabulary between them; they need not be the same implementor, only agree
/// on how a node is represented.
///
/// Returns `None` when the handshake versions were equal and there is nothing
/// to reconcile, in which case each side keeps whatever it came with.
pub(crate) async fn mirror<C, S, B, T>(
    client: C,
    server: S,
) -> Result<Option<(C::Output, S::Output)>, Error<C::Error, S::Error>>
where
    T: Send + Sync + 'static,
    B: Backend<T, Node<Z>: Leaf<T>>,
    C: Client<B, T>,
    S: Server<B, T>,
{
    handshake(client, server).await?.reconcile().await
}
