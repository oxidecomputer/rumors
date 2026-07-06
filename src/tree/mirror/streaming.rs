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
mod dispute;
mod merge;
mod message;
mod protocol;
mod session;
mod unknown;

pub use backend::{Backend, Immaterial, Leaf, Local, Material, Materiality, Node, Root};
pub use session::Handshaking;

use std::cmp::Ordering;
use std::pin::{Pin, pin};

use async_stream::try_stream;
use futures::{StreamExt, join};
use seq_macro::seq;
use tokio::sync::mpsc;

use super::Error;
use crate::{Version, tree::typed::height::Z};
use convert::Convertible;
use protocol::*;

#[cfg(test)]
mod tests;

/// The two-sided session [`Error`], seen from one party's perspective.
type CombinedError<B, C, T> = Error<<B as Backend<T>>::Error, <C as Backend<T>>::Error>;

/// The root returned, if the backend is materialized.
type RootIfMaterial<B, T> =
    <<B as Backend<T>>::Materialized as Materiality>::Materialized<Root<B, T>>;

/// A boxed [`Messages`] stream.
type BoxMessages<M, E> = Pin<Box<dyn Messages<M, E>>>;

/// One direction of the party boundary: messages pass through unchanged
/// (both parties speak the same backend's node types), while the producer's
/// errors leave the schedule out of band, into the driver's error slot.
///
/// The consumer never has to represent the producer's error type, so the
/// returned stream is infallible at any error type it's asked for — which is
/// why `F` is free. On an error the stream parks (`Pending` forever) rather
/// than ending: end-of-stream means phase completion to the consumer, and a
/// truncated phase would be misread, whereas a parked consumer merely stops —
/// the driver has already been handed the error and abandons the session.
fn divert<M, E, D, W, F>(
    messages: impl Messages<M, E>,
    slot: mpsc::Sender<D>,
    wrap: W,
) -> impl Messages<M, F>
where
    M: Send + 'static,
    E: Send + 'static,
    D: Send + 'static,
    F: Send + 'static,
    W: Fn(E) -> D + Send + 'static,
{
    try_stream! {
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

/// Expand the protocol phase schedule into the driver's body: one line per
/// phase, evaluating to both parties' joined terminal results.
///
/// Each party ident stays bound to a `(state, error slot)` pair: the slot
/// (sender, wrapper) travels with its party so every crossing can [`divert`]
/// the producer's errors, tagged with the producer's side of the sum, out to
/// the driver.
macro_rules! mirror {
    // The shared phase step: un-pend the counterparty `$a`, divert the
    // crossing's errors out of band (the messages themselves cross
    // unchanged: both parties speak the same backend's node types), leave
    // `$b` pending.
    (@one $a:ident >> $b:ident.$m:ident) => {
        let ((msgs, state), (tx, wrap)) = $a;
        let msgs = divert(msgs, tx.clone(), wrap);
        let $a = (state, (tx, wrap));
        let $b = ($b.0.$m(msgs), $b.1);
    };
    // The terminal crossing, matched ahead of the general phase rule: a
    // phase with nothing after it is the schedule's last, and the whole
    // expansion evaluates to the joined `(its party, counterparty)` results.
    (@pending($a:ident) $b:ident.$m:ident(..);) => {{
        let ((msgs, state), (tx, wrap)) = $a;
        let msgs = divert(msgs, tx, wrap);
        join!($b.0.$m(msgs), state)
    }};
    // A repeated run of phases, one statement-form (`@step`) expansion pasted
    // per iteration. The body must leave pending the party it began from —
    // the alternation does, and the types check it.
    (@pending($a:ident) for _ in $lo:tt..$hi:tt { $($body:tt)* } $($rest:tt)*) => {{
        seq!(_ in $lo..$hi {
            mirror!(@step($a) $($body)*);
        });
        mirror!(@pending($a) $($rest)*)
    }};
    // One phase, continuing in tail position so the terminal value bubbles
    // out through the nested blocks.
    (@pending($a:ident) $b:ident.$m:ident(..); $($rest:tt)*) => {{
        mirror!(@one $a >> $b.$m);
        mirror!(@pending($b) $($rest)*)
    }};
    // Statement-form phases for loop bodies: rebindings must outlive their
    // expansion so the next pasted iteration sees them.
    (@step($a:ident) $b:ident.$m:ident(..); $($rest:tt)*) => {
        mirror!(@one $a >> $b.$m);
        mirror!(@step($b) $($rest)*);
    };
    (@step($a:ident)) => {};
    // Opening the schedule: no stream is pending yet.
    ($a:ident.$m:ident(); $($rest:tt)*) => {{
        let $a = ($a.0.$m(), $a.1);
        mirror!(@pending($a) $($rest)*)
    }};
}

/// Drive the full protocol schedule between two connected peers.
///
/// The streaming traits expose each step as `(outgoing_stream, next_state)`, so
/// unlike the alternating driver there is no per-message `Step` to inspect and
/// no early return: the schedule is a straight line, each stage's outgoing
/// stream handed to the counterparty's next one.
///
/// Both parties speak the same backend's node types, so messages cross the
/// party boundary unchanged, and errors never cross at all: each crossing
/// [`divert`]s the producer's errors out of band into a shared one-error
/// slot, already tagged with the producer's side of the sum, and the driver
/// races the session against the slot. A party therefore never has to
/// represent — or even be able to represent — its counterparty's errors,
/// which is what lets an [`Infallible`](std::convert::Infallible)-erroring
/// peer pair with a fallible one.
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
    let (errors, mut first_error) = mpsc::channel(1);
    let i = (i, (errors.clone(), Error::Client));
    let r = (r, (errors, Error::Server));

    // This is a fancy macro that makes it easy to see the stages without any
    // noise. The `..` is an anaphora for the stream being threaded between the
    // stages, with it implicitly bound as the result of each method call.
    let session = async {
        mirror! {
            i.initiator();
            r.responder(..);
            i.open_initiator(..);

            for _ in 0..14 {
                r.exchange(..);
                i.exchange(..);
            }

            r.exchange(..);
            i.close_initiator(..);
            r.complete_responder(..);
            i.complete_initiator(..);
        }
    };

    // A diverted error parks its consumer, so the session can no longer
    // finish once the slot has fired: the error branch abandons it. If
    // instead every slot sender drops without an error, the boundary is
    // quiet for good: the `Some` pattern disables the branch and the
    // session runs out on its own.
    let (i, r) = tokio::select! {
        results = session => results,
        Some(error) = first_error.recv() => return Err(error),
    };

    // The terminals resolve in their own parties' error types; each lifts
    // into its own side of the sum.
    Ok((i.map_err(Error::Client)?, r.map_err(Error::Server)?))
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
