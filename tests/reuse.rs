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

use rumors::testing::{IoPlan, IoSide, wrap_link};
use rumors::{Peer, Rumors};
use tokio::time::timeout;

use crate::common::wire::bootstrap_fork_async;

/// Generous wall-clock bound: these sessions are in-memory and finish in
/// microseconds, so hitting the deadline means lost bytes wedged a session,
/// not a slow machine.
const DEADLINE: Duration = Duration::from_secs(10);

/// Link stream capacity, comfortably larger than everything a round ships, so
/// an eager side can finish a session and write its next preamble without
/// waiting on the laggard — the exact interleaving the eager test pins.
const LINK_BUF: usize = 64 * 1024;

/// How many sequential sessions each test drives over the one connection.
const ROUNDS: u64 = 3;

/// Converged no-op sessions run before the divergent rounds in the epoch
/// wrap test: with the [`WRAP_ROUNDS`] divergent rounds after them, the
/// link's u8 session counter crosses 255 and wraps to 0 mid-way through
/// the divergent rounds.
const PRE_WRAP_SESSIONS: usize = 253;

/// Divergent sessions bracketing the epoch wrap: with
/// [`PRE_WRAP_SESSIONS`] before them they run at epochs 253, 254, 255,
/// and the wrapped 0, 1, 2.
const WRAP_ROUNDS: u64 = 6;

/// Mint a connected, party-disjoint pair: a freshly seeded peer and a
/// bootstrap fork of it. The two ends of one link they will keep reusing are
/// minted per test.
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

    let (mut a_link, mut b_link) = rumors::link::memory_with_capacity(LINK_BUF);

    for round in 0..ROUNDS {
        a.send(round);
        b.send(round + 100);
        let (a_out, b_out) = timeout(DEADLINE, async {
            tokio::join!(a.gossip(&mut a_link), b.gossip(&mut b_link))
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

    let (mut a_link, mut b_link) = rumors::link::memory_with_capacity(LINK_BUF);

    let drive_a = async {
        for round in 0..ROUNDS {
            a.send(round);
            a.gossip(&mut a_link).await.expect("A's session");
        }
    };
    let drive_b = async {
        for round in 0..ROUNDS {
            b.send(round + 100);
            b.gossip(&mut b_link).await.expect("B's session");
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

/// An epilogue-only session still advances the link's session epoch on
/// both ends: a converged pair runs a zero-data-stream session (nothing
/// differs, so reconciliation opens no streams — asserted through the
/// wrapped links' stream counters, not assumed), then diverges and
/// reconciles again over the same link. The second session's data streams
/// are labeled with each end's next epoch, so it converges only if the
/// empty session advanced both counters in lockstep — catching any future
/// "advance the epoch only when streams open" optimization on either end.
#[tokio::test(flavor = "current_thread")]
async fn empty_sessions_advance_epochs_in_lockstep() {
    let (a, b) = pair().await;
    let (a_link, b_link) = rumors::link::memory_with_capacity(LINK_BUF);
    let (mut a_link, a_report) = wrap_link(IoSide::Left, IoPlan::default(), a_link);
    let (mut b_link, b_report) = wrap_link(IoSide::Right, IoPlan::default(), b_link);

    // Session 1: the pair is converged, so this is preamble, greeting, and
    // epilogue only — no data stream opens in either direction.
    let (a_out, b_out) = timeout(DEADLINE, async {
        tokio::join!(a.gossip(&mut a_link), b.gossip(&mut b_link))
    })
    .await
    .expect("the converged session deadlocked");
    a_out.expect("A's empty session");
    b_out.expect("B's empty session");

    // The premise everything above rests on, asserted rather than assumed:
    // the converged session opened no data streams. If a future change
    // gives converged sessions a stream (an unconditional probe, say), the
    // "epilogue-only session" coverage silently disappears — this catches
    // that rot.
    let (a_empty, b_empty) = (a_report.snapshot(), b_report.snapshot());
    assert_eq!(
        (
            a_empty.connects,
            a_empty.accepts,
            b_empty.connects,
            b_empty.accepts,
        ),
        (0, 0, 0, 0),
        "the converged session must open no data streams in either direction",
    );

    // Session 2, same link, real divergence: its streams carry epoch 1.
    a.send(1);
    b.send(2);
    let (a_out, b_out) = timeout(DEADLINE, async {
        tokio::join!(a.gossip(&mut a_link), b.gossip(&mut b_link))
    })
    .await
    .expect("the divergent session deadlocked");
    a_out.expect("A's divergent session");
    b_out.expect("B's divergent session");
    assert_eq!(a.snapshot().hash(), b.snapshot().hash());
    assert_eq!(a.snapshot().len(), 2);
}

/// The u8 session epoch wraps while both ends stay in lockstep: after
/// enough sessions on one link the counter crosses 255 back to 0, and
/// sessions spanning the wrap still label their streams consistently and
/// converge. Strategy: converged no-op sessions burn epochs cheaply (they
/// open no data streams but still count — the lockstep the test above
/// pins), then divergent rounds bracket the wrap itself, running at
/// epochs 253 through 255 and the wrapped 0 through 2.
#[tokio::test(flavor = "current_thread")]
async fn epoch_wrap_keeps_the_pair_in_lockstep() {
    let (a, b) = pair().await;
    let (mut a_link, mut b_link) = rumors::link::memory_with_capacity(LINK_BUF);

    for session in 0..PRE_WRAP_SESSIONS {
        let (a_out, b_out) = timeout(DEADLINE, async {
            tokio::join!(a.gossip(&mut a_link), b.gossip(&mut b_link))
        })
        .await
        .unwrap_or_else(|_| panic!("no-op session {session} deadlocked"));
        a_out.expect("A's no-op session");
        b_out.expect("B's no-op session");
    }

    for round in 0..WRAP_ROUNDS {
        a.send(round);
        b.send(100 + round);
        let (a_out, b_out) = timeout(DEADLINE, async {
            tokio::join!(a.gossip(&mut a_link), b.gossip(&mut b_link))
        })
        .await
        .unwrap_or_else(|_| panic!("wrap round {round} deadlocked"));
        a_out.expect("A's wrap-spanning session");
        b_out.expect("B's wrap-spanning session");
        assert_eq!(
            a.snapshot().hash(),
            b.snapshot().hash(),
            "wrap round {round} did not converge the pair"
        );
    }
    assert_eq!(a.snapshot().len(), 2 * WRAP_ROUNDS as usize);
}
