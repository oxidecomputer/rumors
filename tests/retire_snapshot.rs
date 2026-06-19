//! Golden byte-level snapshots of a single retire session: a peer handing its
//! ITC party to a counterparty so the id-region is reclaimed rather than leaked
//! (see `rumors::Peer::retire`).
//!
//! The companion to `gossip_snapshot.rs` for the *retire* leg of the protocol.
//! A retire opens with an ordinary mirror descent — the same reconciliation a
//! plain gossip would run — and then the absorbing peer takes the retiree's
//! party as a trailing frame. Each test stages the pair, drives one retire
//! through the recording duplex in
//! [`common::gossip_snapshot::capture_session`], and pins the exact
//! byte-by-byte conversation against an `insta` snapshot. Drift in the
//! reconciliation round or the party hand-off shows up here as a diff;
//! re-accept only after a deliberate protocol change.
//!
//! Party convention: **A is the absorber** — the counterparty that survives the
//! session and takes the retiree's party — and **B is the retiree**, running
//! [`Peer::retire`]. The absorber's role varies by scenario: plain `gossip` in
//! [`empty_retire`] and [`divergent_retire`], a [`Peer::bootstrap`] that
//! inherits the identity in [`retire_into_bootstrapper`]. The exception is
//! [`mutual_retire_declines`], where both sides retire and neither absorbs.
//!
//! As in `gossip_snapshot.rs` the payload is `u64`: a fixed 8 bytes, easy to
//! spot in the hex.

mod common;

use rumors::{Peer, Retire, Rumors};

use crate::common::gossip_snapshot::capture_session;
use crate::common::wire::bootstrap_fork;

/// A seed universe from a fixed RNG, so the [`rumors::Network`] id and every
/// party forked from it are deterministic and these captures stay reproducible.
/// The retiree is always a [`bootstrap_fork`] of this seed: a genuine disjoint
/// originator, which is what retirement reclaims.
fn seeded() -> Rumors<u64> {
    use rand::SeedableRng;
    use rand::rngs::SmallRng;
    Peer::seed_rng(&mut SmallRng::seed_from_u64(0)).into_rumors()
}

/// Capture one successful retire: `retiree` runs [`Peer::retire`] (party B)
/// while `absorber` drives `gossip` (party A), reconciling content and then
/// taking the retiree's party. The retiree is expected to commit
/// ([`Retire::Retired`]).
fn capture_retire(absorber: Rumors<u64>, retiree: Rumors<u64>) -> String {
    capture_session(
        move |mut r, mut w| async move {
            absorber
                .gossip(&mut r, &mut w)
                .await
                .expect("absorber gossip");
        },
        move |mut r, mut w| async move {
            let retiree = retiree
                .try_into_peer()
                .await
                .expect("the sole handle reclaims the Peer");
            let outcome = retiree.retire(&mut r, &mut w).await;
            assert!(
                matches!(outcome, Retire::Retired),
                "the absorber dominates, so retire must commit; got {outcome:?}",
            );
        },
    )
}

/// Retire into a converged absorber: both sides are empty, so their versions
/// are equal and the absorber dominates reflexively. The minimal retire session
/// — a reconciliation round that moves no content, then the bare party
/// hand-off.
#[test]
fn empty_retire() {
    let seed = seeded();
    let retiree = bootstrap_fork(&seed);
    insta::assert_snapshot!(capture_retire(seed, retiree));
}

/// Retire across a divergence: the retiree holds `1`, the absorber holds `2`.
/// The session's reconciliation round trades the two novel messages — content
/// crossing the wire in *both* directions — before the party changes hands, so
/// this pins a content-bearing retire that the converged case never reaches.
#[test]
fn divergent_retire() {
    let seed = seeded();
    let retiree = bootstrap_fork(&seed);
    retiree.send(1);
    seed.send(2);
    insta::assert_snapshot!(capture_retire(seed, retiree));
}

/// Both sides try to retire into each other: each reads the other's
/// retire-intent from the preamble and refuses to absorb a peer that is itself
/// leaving, so both decline and are handed back intact. The capture pins the
/// bytes of that mutual stand-down. (The symmetric exception to this file's
/// A-absorbs/B-retires convention: here both parties retire.)
#[test]
fn mutual_retire_declines() {
    let seed = seeded();
    let a = bootstrap_fork(&seed);
    let b = seed;
    a.batch().send(1).send(2);
    b.batch().send(3).send(4);

    let capture = capture_session(
        move |mut r, mut w| async move {
            let a = a.try_into_peer().await.expect("a's sole handle");
            let outcome = a.retire(&mut r, &mut w).await;
            assert!(
                matches!(outcome, Retire::Declined { .. }),
                "mutual retirement must decline; got {outcome:?}",
            );
        },
        move |mut r, mut w| async move {
            let b = b.try_into_peer().await.expect("b's sole handle");
            let outcome = b.retire(&mut r, &mut w).await;
            assert!(
                matches!(outcome, Retire::Declined { .. }),
                "mutual retirement must decline; got {outcome:?}",
            );
        },
    );
    insta::assert_snapshot!(capture);
}

/// Retire into a *bootstrapping* counterparty: the newcomer (party A) pulls the
/// retiree's whole tree through the descent and then receives its whole party
/// as the trailing frame — it *becomes* the retiree's successor in the same
/// universe. The cross of the bootstrap and retire legs: one side bootstraps,
/// the other retires, and the identity is handed off rather than reclaimed by
/// an established peer.
#[test]
fn retire_into_bootstrapper() {
    let seed = seeded();
    let retiree = bootstrap_fork(&seed);
    retiree.batch().send(1).send(2);

    let capture = capture_session(
        |mut r, mut w| async move {
            Peer::<u64>::bootstrap(&mut r, &mut w)
                .await
                .expect("bootstrap handshake")
                .expect("the retiree served the bootstrap");
        },
        move |mut r, mut w| async move {
            let retiree = retiree
                .try_into_peer()
                .await
                .expect("the sole handle reclaims the Peer");
            let outcome = retiree.retire(&mut r, &mut w).await;
            assert!(
                matches!(outcome, Retire::Retired),
                "a bootstrapper absorbs the retiree; got {outcome:?}",
            );
        },
    );
    insta::assert_snapshot!(capture);
}
