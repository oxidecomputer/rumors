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

pub use backend::{Backend, Immaterial, Leaf, Local, Material, Materiality, Node, Root};
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

/// The bound on every internal channel: one node's child fan (the radix).
///
/// The walk's producers and consumers advance in lockstep per parent: the merge
/// operation holds one item of lookahead per input, and a parent contributes at
/// most one fan of children before its walk must pull its inputs again, so a
/// single fan's worth of slack absorbs the maximum skew between the wire, the
/// descending frontier, and the upward reassembly. This bound is what makes
/// reconciliation fixed-memory regardless of diff size.
pub(super) const FAN: usize = u8::MAX as usize;

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

/// Map a fault from the client party's producer frame into the session sum.
///
/// The client's own errors keep their side; an assembly fault belongs to the
/// server, whose backend failed to represent its own incoming nodes, and lifts
/// into the server's error vernacular.
fn client<C, S, E>(fault: Error<C, E>) -> Error<C, S>
where
    S: From<E>,
{
    match fault {
        Error::Client(own) => Error::Client(own),
        Error::Server(theirs) => Error::Server(theirs.into()),
    }
}

/// [`client`] from the server party's frame: the sides flip as the fault
/// crosses into the session sum.
fn server<C, S, E>(fault: Error<S, E>) -> Error<C, S>
where
    C: From<E>,
{
    match fault {
        Error::Client(own) => Error::Server(own),
        Error::Server(theirs) => Error::Client(theirs.into()),
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
macro_rules! mirror {
    (@one $a:ident >> $b:ident.$m:ident) => {
        let ((msgs, state), (tx, wrap)) = $a;
        let msgs = divert(msgs, tx.clone(), move |e| wrap(e.into()));
        let $a = (state, (tx, wrap));
        let $b = ($b.0.$m(msgs), $b.1);
    };
    (@pending($a:ident) $b:ident.$m:ident;) => {{
        let ((msgs, state), (tx, wrap)) = $a;
        let msgs = divert(msgs, tx, move |e| wrap(e.into()));
        join!($b.0.$m(msgs), state)
    }};
    (@pending($a:ident) for _ in $lo:tt..$hi:tt { $($body:tt)* } $($rest:tt)*) => {{
        seq!(_ in $lo..$hi {
            mirror!(@step($a) $($body)*);
        });
        mirror!(@pending($a) $($rest)*)
    }};
    (@pending($a:ident) $b:ident.$m:ident; $($rest:tt)*) => {{
        mirror!(@one $a >> $b.$m);
        mirror!(@pending($b) $($rest)*)
    }};
    (@step($a:ident) $b:ident.$m:ident; $($rest:tt)*) => {
        mirror!(@one $a >> $b.$m);
        mirror!(@step($b) $($rest)*);
    };
    (@step($a:ident)) => {};
    (@run $a:ident.$m:ident; $($rest:tt)*) => {{
        let $a = ($a.0.$m(), $a.1);
        mirror!(@pending($a) $($rest)*)
    }};
    ($a:ident.$m:ident; $b:ident.$n:ident; $($rest:tt)*) => {{
        let (errors, mut first_error) = mpsc::channel(1);
        let $a = ($a, (errors.clone(), client));
        let $b = ($b, (errors, server));
        let session = async { mirror!(@run $a.$m; $b.$n; $($rest)*) };
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
/// The parties may speak different backends: each one emits its node-carrying
/// output already converted into its counterparty's node types (a session holds
/// its counterparty's backend handle for exactly this), so messages cross the
/// party boundary unchanged, and errors never cross at all: each crossing
/// [`divert`]s the producer's errors out of band into a shared one-error slot
/// and the driver races the session against the slot.
async fn mirror_connected<BI, BR, I, R, T>(
    i: I,
    r: R,
) -> Result<(I::Output, R::Output), Error<I::Error, R::Error>>
where
    T: Send + Sync + 'static,
    BI: Backend<T>,
    BR: Backend<T>,
    I: Peer<BI, BR, T>,
    R: Peer<BR, BI, T>,
    I::Error: From<BI::Error>,
    R::Error: From<BR::Error>,
{
    mirror! {
        i.initiator;
        r.responder;
        i.open_initiator;
        for _ in 0..14 {
            r.exchange;
            i.exchange;
        }
        r.exchange;
        i.close_initiator;
        r.complete_responder;
        i.complete_initiator;
    }
}

type ClientConnected<C, B, T> = <<C as Connect<B, T>>::Next as CompleteConnect<B, T>>::Next;
type ServerConnected<S, B, T> = <S as Accept<B, T>>::Next;

pub(crate) struct Handshaken<C, S, BC, BS, T>
where
    T: Send + Sync + 'static,
    BC: Backend<T>,
    BS: Backend<T>,
    C: Client<BC, BS, T>,
    S: Server<BS, BC, T>,
{
    client: ClientConnected<C, BC, T>,
    server: ServerConnected<S, BS, T>,
    our_version: Version,
    peer: message::Handshake,
}

impl<C, S, BC, BS, T> Handshaken<C, S, BC, BS, T>
where
    T: Send + Sync + 'static,
    BC: Backend<T>,
    BS: Backend<T>,
    C: Client<BC, BS, T>,
    S: Server<BS, BC, T>,
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
    ) -> Result<Option<(C::Output, S::Output)>, Error<C::Error, S::Error>>
    where
        C::Error: From<BC::Error>,
        S::Error: From<BS::Error>,
    {
        let Handshaken {
            client: local,
            server: remote,
            our_version,
            peer,
        } = self;
        descend(local, remote, our_version, peer.version).await
    }
}

pub(crate) async fn handshake<C, S, BC, BS, T>(
    c: C,
    s: S,
) -> Result<Handshaken<C, S, BC, BS, T>, Error<C::Error, S::Error>>
where
    T: Send + Sync + 'static,
    BC: Backend<T>,
    BS: Backend<T>,
    C: Client<BC, BS, T>,
    S: Server<BS, BC, T>,
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

pub(crate) async fn descend<L, R, BL, BR, T>(
    local: L,
    remote: R,
    local_version: Version,
    remote_version: Version,
) -> Result<Option<(L::Output, R::Output)>, Error<L::Error, R::Error>>
where
    T: Send + Sync + 'static,
    BL: Backend<T>,
    BR: Backend<T>,
    L: Peer<BL, BR, T>,
    R: Peer<BR, BL, T>,
    L::Error: From<BL::Error>,
    R::Error: From<BR::Error>,
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
/// The parties may implement different backends: each one holds its
/// counterparty's backend handle and emits its node-carrying output already
/// converted into that backend's node types.
///
/// Returns `None` when the handshake versions were equal and there is nothing
/// to reconcile, in which case each side keeps whatever it came with.
pub(crate) async fn mirror<C, S, BC, BS, T>(
    client: C,
    server: S,
) -> Result<Option<(C::Output, S::Output)>, Error<C::Error, S::Error>>
where
    T: Send + Sync + 'static,
    BC: Backend<T>,
    BS: Backend<T>,
    C: Client<BC, BS, T>,
    S: Server<BS, BC, T>,
    C::Error: From<BC::Error>,
    S::Error: From<BS::Error>,
{
    handshake(client, server).await?.reconcile().await
}
