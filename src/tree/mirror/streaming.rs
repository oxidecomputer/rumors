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
//! [`Handshaking::start`]. A remote party's implementor — the stage chain
//! that frames messages onto a wire instead of walking a tree — is a later
//! phase.

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

use async_stream::try_stream;
use futures::join;
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
    // `N` names the converted message type rather than projecting it from `M`:
    // the source pins `M`, the receiving stage pins `N`, and inference meets in
    // this bound — a projection in return position would have to be concrete
    // before the receiver constrains it (the boxed wires erase to `dyn`, whose
    // item type is fixed at the coercion).
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

/// Expand the protocol phase schedule into the driver's body: one line per
/// phase, evaluating to both parties' joined terminal results.
///
/// Each line is one phase: the named party's stage consumes the pending stream
/// (`..`) and its own outgoing stream becomes pending in turn. The parties
/// alternate, so a crossing's producer is always the counterparty of the line
/// consuming it, and [`wire`] converts each crossing from the producer's node
/// vocabulary into the consumer's — which is also why a misordered schedule
/// fails to type-check instead of misrouting. `initiator()` — no stream
/// pending yet — opens the schedule; a `for _ in lo..hi { … }` run of phases
/// repeats by delegation to `seq!`; and the final phase closes the schedule,
/// `join!`ing its party's terminal future with the counterparty's into the
/// invocation's value, ordered `(final phase's party, counterparty)`.
///
/// Each party ident stays bound to a `(backend handle, state)` pair, with the
/// pending stream stashed inside the producer's state slot, because the loop
/// body expands to one pasted invocation per iteration and macro hygiene gives
/// every expansion its own locals: only the caller's own idents can carry the
/// stream from one iteration to the next.
macro_rules! mirror {
    // The shared phase step: un-pend the counterparty `$a`, convert the
    // crossing, leave `$b` pending.
    (@one $a:ident >> $b:ident.$m:ident) => {
        let (handle, (msgs, state)) = $a;
        let $a = (handle, state);
        let msgs = wire(&$a.0, &$b.0, msgs);
        let $b = ($b.0, $b.1.$m(msgs));
    };
    // The terminal crossing, matched ahead of the general phase rule: a
    // phase with nothing after it is the schedule's last, and the whole
    // expansion evaluates to the joined `(its party, counterparty)` results.
    (@pending($a:ident) $b:ident.$m:ident(..);) => {{
        let (handle, (msgs, state)) = $a;
        let msgs = wire(&handle, &$b.0, msgs);
        join!($b.1.$m(msgs), state)
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
        let $a = ($a.0, $a.1.$m());
        mirror!(@pending($a) $($rest)*)
    }};
}

/// Drive the full protocol schedule between two connected peers.
///
/// The streaming traits expose each step as `(outgoing_stream, next_state)`, so
/// unlike the alternating driver there is no per-message `Step` to inspect and
/// no early return: the schedule is a straight line, one `mirror!` phase per
/// stage, each stage's outgoing stream [wired](wire) across the party boundary
/// to the counterparty's next stage.
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
    let i = (initiator, i);
    let r = (responder, r);

    let (i, r) = mirror! {
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
    };

    Ok((i?, r.map_err(Error::flip)?))
}

type ClientConnected<C, B, T> = <<C as Connect<B, T>>::Next as CompleteConnect<B, T>>::Next;
type ServerConnected<S, B, T> = <S as Accept<B, T>>::Next;

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
    /// Returns `None` when the handshake versions were equal and the trees are
    /// already converged, in which case each side's root is whatever the caller
    /// already holds.
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
/// The implementors need not be backed by materialized trees — any pair of
/// [`Client`] and [`Server`] stage chains will do, each speaking in its own
/// backend's node types; the backend handles are what the party boundary
/// converts through. The two backends deliberately differ: an implementor
/// fronting a remote peer speaks an *immaterial* backend whose nodes are the
/// wire's own leaf frames, so crossing the boundary toward it is an explode —
/// no node is ever constructed on the wire side — while crossing from it is
/// exactly the assembly the local side needs anyway. Start a tree-backed
/// session with [`Handshaking::start`].
///
/// Returns `None` when the handshake versions were equal and there is nothing
/// to reconcile, in which case each side keeps whatever it came with; a caller
/// holding trees falls back to the roots it started its sessions from.
pub(crate) async fn mirror<P, Q, B, C, T>(
    local_backend: B,
    remote_backend: C,
    client: P,
    server: Q,
) -> Result<Option<(Root<B, T>, Root<C, T>)>, Error<B::Error, C::Error>>
where
    T: Send + Sync + 'static,
    B: Backend<T, Node<Z>: Leaf<T>>,
    C: Backend<T, Node<Z>: Leaf<T>>,
    P: Client<B, T>,
    Q: Server<C, T>,
{
    handshake(local_backend, remote_backend, client, server)
        .await?
        .reconcile()
        .await
}
