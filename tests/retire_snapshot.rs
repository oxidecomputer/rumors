//! Golden byte-level snapshots of a single retire session: a peer handing its
//! ITC party to a counterparty so the id-region is reclaimed rather than leaked
//! (see `rumors::Peer::retire`).
//!
//! The companion to `gossip_snapshot.rs` for the *retire* leg of the protocol.
//! A retire opens with an ordinary mirror descent — the same reconciliation a
//! plain gossip would run — and then the absorbing peer takes the retiree's
//! party as a trailing frame. Each test stages the pair, drives one retire
//! through the recording connection in
//! [`common::gossip_snapshot::capture_session`], and pins every wire byte.
//! Traffic is grouped by logical stream while preserving exact order within
//! each stream. Drift in reconciliation or the hand-off shows up as a diff.
//! Re-accept only after a deliberate protocol change: a new protocol version,
//! never a mutation of an existing one. The re-accept procedure
//! (`cargo insta review`) is in `AGENTS.md`.
//!
//! Party convention: **A is the absorber** — the counterparty that survives the
//! session and takes the retiree's party — and **B is the retiree**, running
//! [`Peer::retire`]. The absorber's role varies by scenario: plain `gossip` in
//! [`empty_retire`] and [`divergent_retire`], a [`Peer::join`] that
//! inherits the identity in [`retire_into_joiner`]. The exception is
//! [`mutual_retire_declines`], where both sides retire and neither absorbs.
//!
//! As in `gossip_snapshot.rs` the payload is `u64`: a fixed 8 bytes, easy to
//! spot in the hex.

use rumors_testkit::common;

use rumors::{Connection, Peer, Retire};

use crate::common::gossip_snapshot::{capture_session, seeded};
use crate::common::wire::join_fork;

/// Capture one successful retire: `retiree` runs [`Peer::retire`] (party B)
/// while `absorber` drives `gossip` (party A), reconciling content and then
/// taking the retiree's party.
///
/// The retiree is expected to commit, its connection reusable
/// ([`Retire::Retired`] with [`Connection::Reusable`]).
fn capture_retire(absorber: Peer<u64>, retiree: Peer<u64>) -> String {
    capture_session(
        move |mut end, _hook| async move {
            absorber
                .gossip_once(&mut end.read, &mut end.write)
                .await
                .expect("absorber gossip");
        },
        move |mut end, _hook| async move {
            let outcome = retiree.retire(&mut end.read, &mut end.write).await;
            assert!(
                matches!(
                    outcome,
                    Retire::Retired {
                        connection: Connection::Reusable { .. }
                    }
                ),
                "the absorber dominates, so retire must commit; got {outcome:?}",
            );
        },
    )
}

/// Retire into a converged absorber: both sides are empty, so their versions
/// are equal and the absorber dominates reflexively.
///
/// The minimal retire session — a reconciliation round that moves no content,
/// then the bare party hand-off.
#[test]
fn empty_retire() {
    let seed = seeded();
    let retiree = join_fork(&seed);
    insta::assert_snapshot!(capture_retire(seed, retiree));
}

/// Retire across a divergence: the retiree holds `1`, the absorber holds `2`.
///
/// The session's reconciliation round trades the two novel messages — content
/// crossing the wire in *both* directions — before the party changes hands, so
/// this pins a content-bearing retire that the converged case never reaches.
#[test]
fn divergent_retire() {
    let seed = seeded();
    let retiree = join_fork(&seed);
    retiree.send(1).unwrap();
    seed.send(2).unwrap();
    insta::assert_snapshot!(capture_retire(seed, retiree));
}

/// Both sides try to retire into each other: each reads the other's
/// retire-intent from the preamble and refuses to absorb a peer that is itself
/// leaving, so both decline and are handed back intact.
///
/// The capture pins the bytes of that mutual stand-down. (The symmetric
/// exception to this file's A-absorbs/B-retires convention: here both parties
/// retire.)
#[test]
fn mutual_retire_declines() {
    let seed = seeded();
    let a = join_fork(&seed);
    let b = seed;
    a.send_all([1, 2]).unwrap();
    b.send_all([3, 4]).unwrap();

    let capture = capture_session(
        move |mut end, _hook| async move {
            let outcome = a.retire(&mut end.read, &mut end.write).await;
            assert!(
                matches!(outcome, Retire::Declined { .. }),
                "mutual retirement must decline; got {outcome:?}",
            );
        },
        move |mut end, _hook| async move {
            let outcome = b.retire(&mut end.read, &mut end.write).await;
            assert!(
                matches!(outcome, Retire::Declined { .. }),
                "mutual retirement must decline; got {outcome:?}",
            );
        },
    );
    insta::assert_snapshot!(capture);
}

/// Retire into a *joining* counterparty: the newcomer (party A) pulls the
/// retiree's whole tree through the descent and then receives its whole party
/// as the trailing frame.
///
/// The newcomer *becomes* the retiree's successor in the same network. The
/// cross of the join and retire legs: one side joins, the other
/// retires, and the identity is handed off rather than reclaimed by an
/// established peer.
#[test]
fn retire_into_joiner() {
    let seed = seeded();
    let retiree = join_fork(&seed);
    retiree.send_all([1, 2]).unwrap();

    let capture = capture_session(
        |mut end, hook| async move {
            match Peer::<u64>::builder()
                .observe(hook)
                .join(&mut end.read, &mut end.write)
                .await
            {
                rumors::Join::Connected {
                    peer,
                    connection: rumors::Connection::Reusable { .. },
                } => peer,
                _ => panic!("the retiree served the join"),
            };
        },
        move |mut end, _hook| async move {
            let outcome = retiree.retire(&mut end.read, &mut end.write).await;
            assert!(
                matches!(
                    outcome,
                    Retire::Retired {
                        connection: Connection::Reusable { .. }
                    }
                ),
                "a joiner absorbs the retiree; got {outcome:?}",
            );
        },
    );
    insta::assert_snapshot!(capture);
}
