//! Connection reuse: back-to-back gossip sessions on one transport.
//!
//! A [`rumors::Rumors::gossip_once`] session that returns `Ok` leaves the stream
//! at a session boundary, so a single connection can host any number of
//! sequential sessions. These tests pin that promise at the two
//! interleavings that matter: rounds separated by a cross-peer barrier, and
//! rounds where one side eagerly begins its next session while the other is
//! still draining the last frames of the previous one — the shape of any
//! loop that re-gossips a long-lived connection whenever local content
//! changes.

use rumors_testkit::common;
use rumors_testkit::common::wire::ROOMY_CAPACITY;

use std::time::Duration;

use rumors::testing::{IoPlan, IoSide, wrap_io};
use rumors::{Peer, Rumors};
use tokio::io::AsyncWriteExt;
use tokio::time::timeout;

use crate::common::wire::{assert_connection_drained, bootstrap_fork_async};

/// Generous wall-clock bound: these sessions are in-memory and finish in
/// microseconds, so hitting the deadline means lost bytes wedged a session,
/// not a slow machine.
const DEADLINE: Duration = Duration::from_secs(10);

/// How many sequential sessions each test drives over the one connection.
const ROUNDS: u64 = 3;

/// Create a connected, party-disjoint pair: a freshly seeded peer and a
/// bootstrap fork of it. The two ends of one connection they will keep
/// reusing are created per test.
async fn pair() -> (Rumors<u64>, Rumors<u64>) {
    let a: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
    let b = bootstrap_fork_async(&a).await;
    (a, b)
}

/// Sessions separated by a barrier reuse the connection: once a round's two
/// `gossip` calls have both returned `Ok`, the same reader/writer pair hosts
/// the next round, and every round converges the pair.
#[tokio::test(flavor = "current_thread")]
async fn barriered_sessions_reuse_the_connection() {
    let (a, b) = pair().await;

    let (mut a_end, mut b_end) = rumors::testing::memory_connection(ROOMY_CAPACITY);

    for round in 0..ROUNDS {
        a.send(round).unwrap();
        b.send(round + 100).unwrap();
        let (a_out, b_out) = timeout(DEADLINE, async {
            tokio::join!(
                a.gossip_once(&mut a_end.read, &mut a_end.write),
                b.gossip_once(&mut b_end.read, &mut b_end.write)
            )
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
    // Reuse rests on each round ending at a clean boundary; after the last
    // round the connection must hold nothing in either direction.
    assert_connection_drained(a_end, b_end);
}

/// An eagerly re-initiating side loses nothing: each peer runs its
/// `send; gossip` rounds on its own schedule with no cross-peer barrier.
///
/// The faster side's next preamble goes on the wire while the slower side is
/// still consuming the previous session's trailing frames.
/// Those preamble
/// bytes must survive to start the next session — a session reader that
/// buffers past the frames it consumes would swallow them and wedge both
/// peers.
#[tokio::test(flavor = "current_thread")]
async fn eager_reinitiation_reuses_the_connection() {
    let (a, b) = pair().await;

    let (mut a_end, mut b_end) = rumors::testing::memory_connection(ROOMY_CAPACITY);

    let drive_a = async {
        for round in 0..ROUNDS {
            a.send(round).unwrap();
            a.gossip_once(&mut a_end.read, &mut a_end.write)
                .await
                .expect("A's session");
        }
    };
    let drive_b = async {
        for round in 0..ROUNDS {
            b.send(round + 100).unwrap();
            b.gossip_once(&mut b_end.read, &mut b_end.write)
                .await
                .expect("B's session");
        }
    };
    timeout(DEADLINE, async { tokio::join!(drive_a, drive_b) })
        .await
        .expect("eager rounds deadlocked: a next-session preamble was lost");

    // Both sides ran the same number of sessions, so the last session paired
    // the final states: converged, holding every message both sides sent.
    assert_eq!(a.snapshot().hash(), b.snapshot().hash());
    assert_eq!(a.snapshot().len(), 2 * ROUNDS as usize);
    assert_connection_drained(a_end, b_end);
}

/// Items each side flushes in a session that carries no frame: the
/// preamble, the greeting, and the completion marker, one flush each.
const FRAMELESS_FLUSHES: usize = 3;

/// A session that carries no frame leaves the connection reusable.
///
/// A converged pair runs a session of control items alone (nothing differs,
/// so no frame crosses — asserted through the wrapped halves' flush counts,
/// not assumed), then diverges and reconciles again over the same
/// connection, which converges only if the frameless session left both
/// halves exactly at its boundary.
#[tokio::test(flavor = "current_thread")]
async fn a_frameless_session_leaves_the_connection_reusable() {
    let (a, b) = pair().await;
    let (a_end, b_end) = rumors::testing::memory_connection(ROOMY_CAPACITY);
    let (mut a_read, mut a_write, a_report) =
        wrap_io(IoSide::Left, IoPlan::default(), a_end.read, a_end.write);
    let (mut b_read, mut b_write, b_report) =
        wrap_io(IoSide::Right, IoPlan::default(), b_end.read, b_end.write);

    // Session 1: the pair is converged, so this is preamble, greeting, and
    // epilogue only.
    let (a_out, b_out) = timeout(DEADLINE, async {
        tokio::join!(
            a.gossip_once(&mut a_read, &mut a_write),
            b.gossip_once(&mut b_read, &mut b_write)
        )
    })
    .await
    .expect("the converged session deadlocked");
    a_out.expect("A's frameless session");
    b_out.expect("B's frameless session");

    // The premise everything above rests on, asserted rather than assumed:
    // the converged session flushed no frame. If a future change gives
    // converged sessions a frame (an unconditional probe, say), the
    // "frameless session" coverage silently disappears — this catches that
    // rot.
    let (a_empty, b_empty) = (a_report.snapshot(), b_report.snapshot());
    assert_eq!(
        (a_empty.flushes, b_empty.flushes),
        (FRAMELESS_FLUSHES, FRAMELESS_FLUSHES),
        "the converged session must flush no frame in either direction",
    );

    // Session 2, same connection, real divergence.
    a.send(1).unwrap();
    b.send(2).unwrap();
    let (a_out, b_out) = timeout(DEADLINE, async {
        tokio::join!(
            a.gossip_once(&mut a_read, &mut a_write),
            b.gossip_once(&mut b_read, &mut b_write)
        )
    })
    .await
    .expect("the divergent session deadlocked");
    a_out.expect("A's divergent session");
    b_out.expect("B's divergent session");
    assert_eq!(a.snapshot().hash(), b.snapshot().hash());
    assert_eq!(a.snapshot().len(), 2);
}

/// Negative control for `assert_connection_drained`: the assert must catch the
/// class it gates.
///
/// A pre-drain bug leaves peer-sent bytes, never consumed by the session,
/// buffered ahead of the next read — a class that otherwise only
/// hand-tracing every early-out path catches. This test manufactures
/// exactly that shape: a clean session runs to `Ok` on both sides, then
/// one stray byte is planted on B's write half (landing unread in front of
/// A), and the drain assert must fail. If this test ever passes
/// with the helper silent, the gate's drain coverage has rotted to a no-op.
#[tokio::test(flavor = "current_thread")]
async fn drain_assert_catches_a_planted_leftover_byte() {
    let (a, b) = pair().await;
    let (mut a_end, mut b_end) = rumors::testing::memory_connection(ROOMY_CAPACITY);
    let (a_out, b_out) = timeout(DEADLINE, async {
        tokio::join!(
            a.gossip_once(&mut a_end.read, &mut a_end.write),
            b.gossip_once(&mut b_end.read, &mut b_end.write)
        )
    })
    .await
    .expect("the clean session deadlocked");
    a_out.expect("A's session");
    b_out.expect("B's session");

    // Plant one unread byte toward A: what a session that failed to drain
    // a peer item would leave resting on the connection.
    b_end
        .write
        .write_all(&[0xAA])
        .await
        .expect("planting the leftover byte");

    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        assert_connection_drained(a_end, b_end);
    }));
    assert!(
        caught.is_err(),
        "the drain assert must fail on a planted leftover byte"
    );
}
