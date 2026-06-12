//! Connection reuse: back-to-back gossip sessions on one transport.
//!
//! A [`rumors::Rumors::gossip`] session that returns `Ok` leaves the stream
//! at a session boundary, so a single connection can host any number of
//! sequential sessions. These tests pin that promise at the two
//! interleavings that matter: rounds separated by a cross-peer barrier, and
//! rounds where one side eagerly begins its next session while the other is
//! still draining the last frames of the previous one — the shape of any
//! loop that re-gossips a long-lived connection whenever local content
//! changes.

mod common;

use std::time::Duration;

use rumors::{Peer, Rumors};
use tokio::io::duplex;
use tokio::time::timeout;

use crate::common::wire::bootstrap_fork_async;

/// Generous wall-clock bound: these sessions are in-memory and finish in
/// microseconds, so hitting the deadline means lost bytes wedged a session,
/// not a slow machine.
const DEADLINE: Duration = Duration::from_secs(10);

/// Duplex capacity, comfortably larger than everything a round ships, so an
/// eager side can finish a session and write its next preamble without
/// waiting on the laggard — the exact interleaving the eager test pins.
const DUPLEX_BUF: usize = 64 * 1024;

/// How many sequential sessions each test drives over the one connection.
const ROUNDS: u64 = 3;

/// Mint a connected, party-disjoint pair: a freshly seeded peer and a
/// bootstrap fork of it, plus the two ends of one duplex they will keep
/// reusing.
async fn pair() -> (Rumors<u64>, Rumors<u64>) {
    let a: Rumors<u64> = Peer::seed().into_rumors();
    let b = bootstrap_fork_async(&a).await;
    (a, b)
}

/// Sessions separated by a barrier reuse the connection: once a round's two
/// `gossip` calls have both returned `Ok`, the same reader/writer pair hosts
/// the next round, and every round converges the pair.
#[tokio::test(flavor = "current_thread")]
async fn barriered_sessions_reuse_the_connection() {
    let (a, b) = pair().await;

    let (a_side, b_side) = duplex(DUPLEX_BUF);
    let (mut a_r, mut a_w) = tokio::io::split(a_side);
    let (mut b_r, mut b_w) = tokio::io::split(b_side);

    for round in 0..ROUNDS {
        a.send(round);
        b.send(round + 100);
        let (a_out, b_out) = timeout(DEADLINE, async {
            tokio::join!(a.gossip(&mut a_r, &mut a_w), b.gossip(&mut b_r, &mut b_w),)
        })
        .await
        .expect("barriered round deadlocked");
        a_out.expect("A's session");
        b_out.expect("B's session");
        assert_eq!(
            a.snapshot().hash(),
            b.snapshot().hash(),
            "round {round} did not converge the pair"
        );
    }
}

/// An eagerly re-initiating side loses nothing: each peer runs its
/// `send; gossip` rounds on its own schedule with no cross-peer barrier, so
/// the faster side's next preamble goes on the wire while the slower side is
/// still consuming the previous session's trailing frames. Those preamble
/// bytes must survive to start the next session — a session reader that
/// buffers past the frames it consumes would swallow them and wedge both
/// peers.
#[tokio::test(flavor = "current_thread")]
async fn eager_reinitiation_reuses_the_connection() {
    let (a, b) = pair().await;

    let (a_side, b_side) = duplex(DUPLEX_BUF);
    let (mut a_r, mut a_w) = tokio::io::split(a_side);
    let (mut b_r, mut b_w) = tokio::io::split(b_side);

    let drive_a = async {
        for round in 0..ROUNDS {
            a.send(round);
            a.gossip(&mut a_r, &mut a_w).await.expect("A's session");
        }
    };
    let drive_b = async {
        for round in 0..ROUNDS {
            b.send(round + 100);
            b.gossip(&mut b_r, &mut b_w).await.expect("B's session");
        }
    };
    timeout(DEADLINE, async { tokio::join!(drive_a, drive_b) })
        .await
        .expect("eager rounds deadlocked: a next-session preamble was lost");

    // Both sides ran the same number of sessions, so the last session paired
    // the final states: converged, holding every message both sides sent.
    assert_eq!(a.snapshot().hash(), b.snapshot().hash());
    assert_eq!(a.snapshot().len(), 2 * ROUNDS as usize);
}
