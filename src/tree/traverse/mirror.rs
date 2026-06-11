//! Bidirectional alternating mirror-sync between two replicas of the typed
//! tree.
//!
//! See [`local`] for the protocol's state machine and asymmetry matrix,
//! [`protocol`] for the type-level phase schedule, [`message`] for the wire
//! format, and [`remote`] for the wire-bound proxy and framing.
//!
//! # Cost
//!
//! Write `N = C + D` for two replicas sharing `C` leaves and differing in
//! `D`. Content addressing spreads leaves uniformly through the 256-ary,
//! 32-deep trie, and each exchange descends two heights. Per session, in
//! expectation:
//!
//! - **Round trips:** ≈ `½·log₂₅₆(2·D·N)` exchanges after the opening —
//!   the descent runs until the disputed paths separate pairwise, and path
//!   compression does *not* shorten it (the descent pops one prefix byte
//!   per level). The fixed schedule caps every session at 36 frames (two
//!   handshake frames plus a 34-message descent covering all 32 levels,
//!   ≈ 18 round trips), and in-band termination (the emptiness predicates
//!   in [`protocol`]'s table) ends it the moment nothing remains disputed.
//! - **Computation, per side:** the disputed frontier, not the tree. Each
//!   round merge-joins the counterparty's disputed sets against level maps
//!   that hold nothing already agreed ([`local`]'s zipper). The frontier
//!   is the union of root-to-leaf paths to the `D` differing leaves —
//!   `Θ(D·(1 + log₂₅₆(N/D)))` disputed nodes — but each disputed node
//!   ships and compares its full child list, a ×256 sibling fan through
//!   the dense levels: `Θ(min(256·D, N)·(1 + log₂₅₆(N/D)))` in all,
//!   collapsing to `Θ(N)` once `D` exceeds `N/256`.
//! - **Bytes:** the same frontier in prefix-and-hash records, plus the `D`
//!   differing message bodies. For small payloads the hash records
//!   dominate, not the bodies. A redacted leaf drives the descent like any
//!   other difference but ships no body at all.
//!
//! Two boundary cases sit outside the formula. A bootstrap (`D = N`)
//! bypasses the dispute machinery entirely: the provider drains its root
//! in one shot — `Θ(N)` work and bytes in `O(1)` rounds. And the first
//! session after local changes also pays the lazy hash memoization along
//! the changed paths: divergence-shaped, charged once.
//!
//! These constants are a deliberate tilt toward latency-dominated links.
//! The 256 fanout and the two-height stride narrow the search space by
//! 256² per round — while shipping only the disputed frontier's actual
//! children, pruned by hash agreement every half-round, up to ~9 KB per
//! disputed node — finishing the descent in ~2 exchanges at scales where
//! a binary Merkle descent would take ~30 rounds. The protocol assumes the link's
//! bandwidth-delay product dwarfs `r̄·W` per session; on a bandwidth-bound
//! link the trade runs backwards (the crate docs' "Should you use it?"
//! says so to users). With the descent this short, a session's remaining
//! latency sits mostly in the fixed phases — preamble, handshake, open,
//! close — which is where any future round-trip work should aim
//! (piggybacking the root fan on the handshake, pipelining the
//! alternation) rather than at the tree.

use std::cmp::Ordering;

use seq_macro::seq;

pub mod local;
pub mod protocol;
pub mod remote;

pub(crate) mod message;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod wire_snapshot;

use protocol::*;

use crate::Version;
use message::Handshake;

// This macro allows defining one communication step of the inner protocol
// between initiator <==> responder (once the client and server have determined
// who plays which role).
macro_rules! x {
    // Any unconditional initiator step that must continue looks like this:
    //
    // ```
    // x! { let message = initiator.method(...) }
    // ```
    //
    // This elides the error handling and irrefutable pattern match.
    (let $msg:pat = $initiator:ident . $initiator_method:ident ( $($arg:expr)* ) ) => {
        let Step::Continue {
            msg: $msg,
            next: $initiator,
        } = $initiator.$initiator_method($($arg)*).await.map_err(Error::Client)?;
    };
    // An initiator step in the protocol:
    //
    // ```
    // x! { initiator.method ==message==> responder.method }
    // ```
    //
    // This feeds the existing binding of `message` into the initiator method,
    // and rebinds `message` to the output. The expected next responder method
    // is specified so that if the initiator signals it is done, the responder
    // can be immediately be given the final message and closed out.
    ($initiator:ident . $initiator_method:ident == $msg:ident => $responder:ident . $responder_method:ident) => {
        #[allow(unused)]
        let ($msg, $responder, $initiator) =
            match $initiator.$initiator_method($msg).await.map_err(Error::Client)? {
                Step::Continue { msg, next } => (msg, $responder, next),
                Step::Done {
                    msg,
                    output: initiator_output,
                } => {
                    #[allow(irrefutable_let_patterns)]
                    let Step::Done {
                        output: responder_output,
                        ..
                    } = $responder.$responder_method(msg).await.map_err(Error::Server)?
                    else {
                        // The protocol is designed so that the two sides will
                        // *always* agree on when the protocol is complete.
                        unreachable!("responder did not finish after initiator was finished")
                    };
                    return Ok((initiator_output, responder_output));
                }
            };
    };
    // An responder step in the protocol:
    //
    // ```
    // x! { initiator.method <=message== responder.method }
    // ```
    //
    // This feeds the existing binding of `message` into the responder method
    // (on the *RIGHT HAND SIDE*), and rebinds `message` to the output. The
    // expected next initiator method is specified so that if the responder
    // signals it is done, the initiator can be immediately be given the final
    // message and closed out.
    ($initiator:ident . $initiator_method:ident <= $msg:ident == $responder:ident . $responder_method:ident) => {
        #[allow(unused)]
        let ($msg, $initiator, $responder) =
            match $responder.$responder_method($msg).await.map_err(Error::Server)? {
                Step::Continue { msg, next } => (msg, $initiator, next),
                Step::Done {
                    msg,
                    output: responder_output,
                } => {
                    #[allow(irrefutable_let_patterns)]
                    let Step::Done {
                        output: initiator_output,
                        ..
                    } = $initiator.$initiator_method(msg).await.map_err(Error::Client)?
                    else {
                        // The protocol is designed so that the two sides will
                        // *always* agree on when the protocol is complete.
                        unreachable!("initiator did not finish after responder was finished");
                    };
                    return Ok((initiator_output, responder_output));
                }
            };
    };
}

// The inner mirror protocol, between an initiator and a responder (who may or
// may not correspond with the original client/server distinction).
async fn mirror_connected<I, R, T>(
    i: I,
    r: R,
) -> Result<(I::Output, R::Output), Error<I::Error, R::Error>>
where
    T: Send + Sync,
    I: Peer<T>,
    R: Peer<T>,
{
    x! { let x = i.initiator() }
    x! { i.open_initiator <=x== r.responder }
    x! { i.open_initiator ==x=> r.exchange  }
    seq!(_ in 0..14 {
        x! { i.exchange <=x== r.exchange }
        x! { i.exchange ==x=> r.exchange }
    });
    x! { i.close_initiator    <=x== r.exchange           }
    x! { i.close_initiator    ==x=> r.complete_responder }
    x! { i.complete_initiator <=x== r.complete_responder }

    match r {}
}

/// An error which can occur during mirroring: either a client error or a server one.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error<C, S> {
    Client(C),
    Server(S),
}

/// The client's exchange after the connect phase: the [`Peer`] it has descended
/// to once `connect` then `complete_connect` have run.
pub(crate) type ClientConnected<C, T> = <<C as Connect<T>>::Next as CompleteConnect<T>>::Next;

/// The server's exchange after the connect phase: the [`Peer`] it has descended
/// to once `accept` has run.
pub(crate) type ServerConnected<S, T> = <S as Accept<T>>::Next;

/// The result of the connect phase ([`handshake`]): the [`Handshake`]s have
/// been exchanged, so the caller can inspect the peer's
/// `network`/`version`/`intent` and decide whether to descend, absorb a
/// retiree, serve a bootstrapper, and so on.
pub(crate) enum Handshaken<C, S, T>
where
    T: Send + Sync,
    C: Client<T>,
    S: Server<T>,
{
    /// The two versions were equal: already converged, no descent. Carries the
    /// client's reconciled root, the server's output (the remote side's framed
    /// halves over the wire), and the peer's [`Handshake`].
    Converged {
        local_root: C::Output,
        remote_out: S::Output,
        peer: Handshake,
    },
    /// The versions differ: the connected exchanges are ready for [`descend`].
    /// Carries our version and the peer's [`Handshake`] for the caller's
    /// dispatch and the descent's role tiebreak.
    Diverged {
        local: ClientConnected<C, T>,
        remote: ServerConnected<S, T>,
        our_version: Version,
        peer: Handshake,
    },
}

impl<C, S, T> Handshaken<C, S, T>
where
    T: Send + Sync,
    C: Client<T>,
    S: Server<T>,
{
    /// The peer's [`Handshake`], available whichever way the connect phase
    /// went.
    ///
    /// Lets the caller dispatch on the peer's `network`/`version`/`intent`,
    /// deciding whether to [`reconcile`](Self::reconcile) or to drop the
    /// exchange unreconciled, before committing to (or skipping) the descent.
    pub(crate) fn peer(&self) -> &Handshake {
        match self {
            Handshaken::Converged { peer, .. } | Handshaken::Diverged { peer, .. } => peer,
        }
    }

    /// Reconcile the two trees to convergence, **descending** the divergent
    /// tries if the versions differ (a no-op if they already converged).
    ///
    /// Hands back the reconciled root, the remote wire halves (for any trailing
    /// party hand-off the caller appends), and the peer's [`Handshake`]. This is
    /// the path that moves content: a steady-state gossip, or serving a
    /// bootstrapper its first copy.
    pub(crate) async fn reconcile(
        self,
    ) -> Result<(C::Output, S::Output), Error<C::Error, S::Error>> {
        match self {
            Handshaken::Converged {
                local_root,
                remote_out,
                ..
            } => Ok((local_root, remote_out)),
            Handshaken::Diverged {
                local,
                remote,
                our_version,
                peer,
            } => {
                let (root, remote_out) =
                    descend(local, remote, our_version, peer.version.clone()).await?;
                Ok((root, remote_out))
            }
        }
    }
}

/// Run the connect phase: the client emits its [`Handshake`], the server
/// ships it and replies with the peer's, and the client absorbs the peer's
/// version. Stops there, handing the outcome back for dispatch (see
/// [`Handshaken`]).
pub(crate) async fn handshake<C, S, T>(
    c: C,
    s: S,
) -> Result<Handshaken<C, S, T>, Error<C::Error, S::Error>>
where
    T: Send + Sync,
    C: Client<T>,
    S: Server<T>,
{
    // The client emits its handshake. `connect` is statically `Continue` (its
    // `Done` carries `Infallible`), so this `let` is irrefutable.
    x! { let our_handshake = c.connect() }
    let our_version = our_handshake.version.clone();

    // The server ships our handshake and replies with the peer's.
    match s.accept(our_handshake).await.map_err(Error::Server)? {
        Step::Continue { msg: peer, next: s } => match c
            .complete_connect(peer.version.clone())
            .await
            .map_err(Error::Client)?
        {
            Step::Continue { msg: (), next: c } => Ok(Handshaken::Diverged {
                local: c,
                remote: s,
                our_version,
                peer,
            }),
            Step::Done { .. } => {
                unreachable!("client and server disagree about whether versions match")
            }
        },
        Step::Done {
            msg: peer,
            output: remote_out,
        } => match c
            .complete_connect(peer.version.clone())
            .await
            .map_err(Error::Client)?
        {
            Step::Done {
                msg: (),
                output: local_root,
            } => {
                debug_assert!(
                    our_version == peer.version,
                    "server and client must agree on version to quit early"
                );
                Ok(Handshaken::Converged {
                    local_root,
                    remote_out,
                    peer,
                })
            }
            Step::Continue { .. } => {
                unreachable!("client and server disagree about whether versions match")
            }
        },
    }
}

/// Run the steady-state descent between two connected [`Peer`]s, choosing the
/// initiator by the canonical-byte tiebreak on the two (necessarily distinct)
/// versions, and returning the outputs in `(local, remote)` order.
pub(crate) async fn descend<I, R, T>(
    local: I,
    remote: R,
    local_version: Version,
    remote_version: Version,
) -> Result<(I::Output, R::Output), Error<I::Error, R::Error>>
where
    T: Send + Sync,
    I: Peer<T>,
    R: Peer<T>,
{
    // Their causal order is only partial (they may be concurrent), so to pick
    // an initiator we compare canonical bytes lexicographically: an arbitrary
    // but total, deterministic tiebreak (not a causal order). Distinct versions
    // have distinct canonical bytes, so `Equal` is impossible.
    match remote_version.as_bytes().cmp(local_version.as_bytes()) {
        // If the remote version is less, the local side is the initiator.
        Ordering::Less => mirror_connected(local, remote).await,
        // Running the remote as initiator, rearrange the result back to (local, remote).
        Ordering::Greater => match mirror_connected(remote, local).await {
            Ok((r, l)) => Ok((l, r)),
            Err(e) => Err(match e {
                Error::Server(l) => Error::Client(l),
                Error::Client(r) => Error::Server(r),
            }),
        },
        Ordering::Equal => unreachable!("distinct versions have distinct canonical bytes"),
    }
}

/// Drive a mirror protocol client against a server to synchronize both of
/// them. A test convenience: the wire entry points
/// ([`Peer::bootstrap`](crate::Peer::bootstrap),
/// [`Rumors::gossip`](crate::Rumors::gossip),
/// [`Peer::retire`](crate::Peer::retire)) drive
/// [`handshake`] and [`Handshaken::reconcile`] directly so they can dispatch
/// on the peer's [`Handshake`] in between, so this whole-session shortcut is
/// only used by the in-process protocol tests.
#[cfg(test)]
pub async fn mirror<'a, C, S, T>(
    c: C,
    s: S,
) -> Result<(C::Output, S::Output), Error<C::Error, S::Error>>
where
    T: Send + Sync + 'a,
    C: Client<T> + 'a,
    S: Server<T> + 'a,
{
    // Box the future so that callers don't need to handle its big future type.
    Box::pin(async move {
        let (root, remote_out) = handshake(c, s).await?.reconcile().await?;
        Ok((root, remote_out))
    })
    .await
}
