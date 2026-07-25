//! The [`rumors::Rumors::gossip_when`] driver: change-driven gossip over a
//! long-lived connection.
//!
//! Every test drives the policy stream by hand — a `futures` mpsc channel
//! whose receiver is the `when` stream — so initiation timing is fully
//! deterministic with no timers anywhere. The suite pins the driver's whole
//! contract:
//!
//! - the reduction to one-shot `gossip`, and remote-led serving;
//! - suppression exactness: the echo a naive driver would produce does not
//!   happen, while real changes always do;
//! - transitive propagation across a chain of connections, and who-led
//!   attribution;
//! - clean shutdown on both the `when` stream ending and the peer hanging
//!   up, and the error terminal.
//!
//! The second half pins the cancellation and reuse contract from the
//! misbehaving side:
//!
//! - a driver dropped mid-session commits nothing and forfeits the
//!   connection — the price the crate docs' "What a session promises"
//!   section documents, enforced by link poisoning;
//! - a driver started on an already-poisoned link fails fast without
//!   waiting for a tick;
//! - a consumer that drops every `next()` future loses nothing (poll
//!   cancel-safety);
//! - a cleanly ended driver hands the connection back usable;
//! - two proptest suites — random tick/commit/yield interleavings, and
//!   connections severed at arbitrary byte offsets — require error-free
//!   convergence and loud-but-recoverable failure respectively, under
//!   every sequencing.

mod common;

use std::{future::poll_fn, task::Poll, time::Duration};

use futures::channel::mpsc::{UnboundedSender, unbounded};
use futures::stream;
use futures::{FutureExt, StreamExt};
use proptest::prelude::*;
use rumors::{Error, Gossiped, Led, Peer, Rumors, testing::run_to_quiescence};
use tokio::io::AsyncWriteExt;
use tokio::time::timeout;

use crate::common::fault::{FaultPlan, faulty};
use crate::common::wire::{bootstrap_fork_async, tokio_block_on as block_on, wire_gossip_async};

/// Generous wall-clock bound: everything here is in-memory and finishes in
/// microseconds, so hitting the deadline means a wedged driver, not a slow
/// machine.
const DEADLINE: Duration = Duration::from_secs(10);

/// Link stream capacity, comfortably larger than anything a session here ships.
const LINK_BUF: usize = 64 * 1024;

/// A connected, party-disjoint pair: a freshly seeded peer and a bootstrap
/// fork of it.
async fn pair() -> (Rumors<u64>, Rumors<u64>) {
    let a: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
    let b = bootstrap_fork_async(&a).await;
    (a, b)
}

/// The two ends of one in-memory link connection between two drivers.
fn links() -> (rumors::link::MemoryLink, rumors::link::MemoryLink) {
    rumors::link::memory_with_capacity(LINK_BUF)
}

/// A hand-driven tick source: send `()` into the sender to tick the stream.
fn ticks() -> (UnboundedSender<()>, impl stream::Stream<Item = ()>) {
    let (tx, rx) = unbounded();
    (tx, rx)
}

/// One session under two hand-driven drivers: tick one (or both) sides,
/// then await one item from each driver, asserting both are `Ok`.
async fn one_round(
    a_sessions: &mut (impl stream::Stream<Item = Result<Gossiped, Error>> + Unpin),
    b_sessions: &mut (impl stream::Stream<Item = Result<Gossiped, Error>> + Unpin),
) -> (Gossiped, Gossiped) {
    let (a_item, b_item) = timeout(
        DEADLINE,
        futures::future::join(a_sessions.next(), b_sessions.next()),
    )
    .await
    .expect("round deadlocked");
    (
        a_item.expect("A's driver ended").expect("A's session"),
        b_item.expect("B's driver ended").expect("B's session"),
    )
}

/// `gossip_when` with a single immediate tick reduces to `gossip`: one
/// session, converged replicas, exactly one `Ok` item per side, then a
/// clean end — and the suppression token both sides report is the same
/// frontier.
#[tokio::test(flavor = "current_thread")]
async fn single_tick_reduces_to_gossip() {
    let (a, b) = pair().await;
    a.send(1);
    b.send(2);

    let (mut a_link, mut b_link) = links();
    let once = || stream::once(std::future::ready(()));
    let mut a_sessions = a.gossip_when(once(), &mut a_link);
    let mut b_sessions = b.gossip_when(once(), &mut b_link);

    let (a_session, b_session) = one_round(&mut a_sessions, &mut b_sessions).await;
    assert_eq!(a_session.converged, b_session.converged);
    assert_eq!(a.snapshot().hash(), b.snapshot().hash());
    assert_eq!(a.snapshot().len(), 2);

    // The `when` streams are exhausted with nothing in flight: clean end.
    let (a_end, b_end) = timeout(
        DEADLINE,
        futures::future::join(a_sessions.next(), b_sessions.next()),
    )
    .await
    .expect("shutdown deadlocked");
    assert!(a_end.is_none());
    assert!(b_end.is_none());
}

/// A pending `when` stream is a pure responder: it never initiates, serves
/// every remote-led session, and attributes leadership correctly on both
/// sides.
#[tokio::test(flavor = "current_thread")]
async fn pending_when_serves_remote_initiations() {
    let (a, b) = pair().await;
    let (mut a_link, mut b_link) = links();

    let (a_tx, a_when) = ticks();
    let mut a_sessions = a.gossip_when(a_when, &mut a_link);
    let mut b_sessions = b.gossip_when(stream::pending::<()>(), &mut b_link);

    for round in 0..3u64 {
        a.send(round);
        a_tx.unbounded_send(()).expect("driver alive");
        let (a_session, b_session) = one_round(&mut a_sessions, &mut b_sessions).await;
        assert_eq!(a_session.led, Led::Local, "round {round}: A initiated");
        assert_eq!(b_session.led, Led::Remote, "round {round}: B responded");
        assert_eq!(a.snapshot().hash(), b.snapshot().hash());
    }
}

/// Suppression is exact, both ways.
///
/// A tick with nothing new since this
/// connection converged initiates nothing — the echo tick that
/// [`rumors::Rumors::changes`] fires after a session's own join produces no
/// second session — while a tick after a real change always initiates.
/// Wired with `changes()` itself, the way a real driver is.
#[tokio::test(flavor = "current_thread")]
async fn suppression_swallows_echoes_not_news() {
    let (a, b) = pair().await;
    let (mut a_link, mut b_link) = links();

    // Real drivers: each side's policy stream is its own change signal.
    let mut a_sessions = a.gossip_when(a.changes(), &mut a_link);
    let mut b_sessions = b.gossip_when(b.changes(), &mut b_link);

    // Round 1: the initial `changes()` yield on both sides drives the
    // reconnect-convergence session (both led locally; one session total).
    a.send(1);
    let (a_session, b_session) = one_round(&mut a_sessions, &mut b_sessions).await;
    assert_eq!(a.snapshot().hash(), b.snapshot().hash());

    // Both replicas converged on the same frontier.
    assert_eq!(a_session.converged, b_session.converged);

    // The session's join advanced each side's frontier, so each side's
    // `changes()` has an echo tick queued. Suppression must swallow it:
    // polling both drivers now yields nothing — no echo session — and the
    // drivers go quiet.
    let echo = futures::future::join(a_sessions.next(), b_sessions.next());
    assert!(
        timeout(Duration::from_millis(100), echo).await.is_err(),
        "an echo session ran on a converged connection"
    );

    // But real news still initiates: a change on B drives exactly one more
    // session, B-led.
    b.send(2);
    let (a_session, b_session) = one_round(&mut a_sessions, &mut b_sessions).await;
    assert_eq!(b_session.led, Led::Local, "B had the news");
    assert_eq!(a_session.led, Led::Remote, "A only served");
    assert_eq!(a.snapshot().hash(), b.snapshot().hash());
    assert_eq!(a.snapshot().len(), 2);
}

/// An interval-style tick on a converged connection costs nothing, and the
/// same tick stream initiates again as soon as the local frontier has
/// really moved.
///
/// This is the anti-entropy property heartbeats rely on, pinned
/// with hand-fed ticks instead of a timer.
#[tokio::test(flavor = "current_thread")]
async fn heartbeat_ticks_are_free_until_divergence() {
    let (a, b) = pair().await;
    let (mut a_link, mut b_link) = links();

    let (a_tx, a_when) = ticks();
    let mut a_sessions = a.gossip_when(a_when, &mut a_link);
    let mut b_sessions = b.gossip_when(stream::pending::<()>(), &mut b_link);

    // First tick: fresh driver, no token yet — the unconditional first
    // session (reconnect convergence).
    a_tx.unbounded_send(()).expect("driver alive");
    one_round(&mut a_sessions, &mut b_sessions).await;

    // Converged: heartbeat ticks are suppressed, nothing crosses the wire.
    for _ in 0..3 {
        a_tx.unbounded_send(()).expect("driver alive");
    }
    let idle = futures::future::join(a_sessions.next(), b_sessions.next());
    assert!(
        timeout(Duration::from_millis(100), idle).await.is_err(),
        "a heartbeat tick initiated a session on a converged connection"
    );

    // Diverged (the lost-wakeup scenario): the next heartbeat tick fires a
    // real session.
    a.send(7);
    a_tx.unbounded_send(()).expect("driver alive");
    let (a_session, _) = one_round(&mut a_sessions, &mut b_sessions).await;
    assert_eq!(a_session.led, Led::Local);
    assert_eq!(a.snapshot().hash(), b.snapshot().hash());
}

/// Suppression is per connection, not per set: a change at A crosses B and
/// reaches C through B's two independent drivers (B's join on the A-side
/// connection is exactly the news its C-side driver must push).
#[tokio::test(flavor = "current_thread")]
async fn changes_propagate_transitively_through_a_chain() {
    let a: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
    let b = bootstrap_fork_async(&a).await;
    let c = bootstrap_fork_async(&b).await;

    let (mut ab_a_link, mut ab_b_link) = links();
    let (mut bc_b_link, mut bc_c_link) = links();

    a.send(42);

    // Four drivers, every policy stream a real change signal. Consume
    // session items in the background of the convergence check: the
    // drivers only progress while polled.
    let a_drv = a.gossip_when(a.changes(), &mut ab_a_link);
    let b_ab_drv = b.gossip_when(b.changes(), &mut ab_b_link);
    let b_bc_drv = b.gossip_when(b.changes(), &mut bc_b_link);
    let c_drv = c.gossip_when(c.changes(), &mut bc_c_link);
    let drive_all = futures::future::join4(
        a_drv.for_each(|item| async move {
            item.expect("A driver session");
        }),
        b_ab_drv.for_each(|item| async move {
            item.expect("B/AB driver session");
        }),
        b_bc_drv.for_each(|item| async move {
            item.expect("B/BC driver session");
        }),
        c_drv.for_each(|item| async move {
            item.expect("C driver session");
        }),
    );

    // C's own change signal announces every advance of C's replica; wait on
    // it until A's message has arrived.
    let mut c_changes = c.changes();
    let converged = async {
        loop {
            c_changes.next().await.expect("set still open");
            let snapshot = c.snapshot();
            if snapshot.iter().any(|(_, _, m)| **m == 42) {
                return;
            }
        }
    };

    tokio::select! {
        _ = drive_all => unreachable!("changes()-fed drivers never end while handles live"),
        out = timeout(DEADLINE, converged) => out.expect("A's change never reached C"),
    }
}

/// When the `when` stream ends with nothing in flight, the driver ends
/// cleanly — and its still-running counterparty sees the dropped connection
/// as a clean goodbye (end-of-stream at a session boundary), not an error.
#[tokio::test(flavor = "current_thread")]
async fn when_exhaustion_then_hangup_both_end_cleanly() {
    let (a, b) = pair().await;
    let (mut a_link, mut b_link) = links();

    // A's `when` is already exhausted: its driver ends without a session.
    let mut a_sessions = a.gossip_when(stream::empty::<()>(), &mut a_link);
    assert!(
        timeout(DEADLINE, a_sessions.next())
            .await
            .expect("A's shutdown deadlocked")
            .is_none()
    );
    drop(a_sessions);
    let mut b_sessions = b.gossip_when(stream::pending::<()>(), &mut b_link);

    // Dropping A's link hangs the connection up at a session
    // boundary; B's responder ends cleanly.
    drop(a_link);
    assert!(
        timeout(DEADLINE, b_sessions.next())
            .await
            .expect("B's shutdown deadlocked")
            .is_none()
    );
}

/// Dropping a driver mid-session commits nothing: both replicas are
/// byte-identical to their pre-session state, and a fresh connection
/// afterwards converges the pair from scratch.
///
/// The drop forfeits the
/// connection — the price the crate docs' "What a session promises"
/// section documents — and poisoning enforces it: a reuse attempt on
/// either end's link fails fast with [`Error::LinkPoisoned`].
#[pollster::test]
async fn dropping_a_driver_mid_session_commits_nothing() {
    let (a, b) = pair().await;
    a.send(1);
    b.send(2);
    let a_before = (a.snapshot().hash(), a.snapshot().latest().clone());
    let b_before = (b.snapshot().hash(), b.snapshot().latest().clone());

    let (mut a_link, mut b_link) = links();
    {
        let (a_tx, a_when) = ticks();
        let mut a_sessions = a.gossip_when(a_when, &mut a_link);
        let mut b_sessions = b.gossip_when(stream::pending::<()>(), &mut b_link);

        // Freeze the session mid-flight: a couple of single polls per side
        // get the preambles (and the first protocol frames) onto the wire,
        // well short of completion.
        a_tx.unbounded_send(()).expect("driver alive");
        for _ in 0..2 {
            assert!(
                a_sessions.next().now_or_never().is_none(),
                "session must still be in flight"
            );
            assert!(
                b_sessions.next().now_or_never().is_none(),
                "session must still be in flight"
            );
        }
        // Both drivers drop here, mid-session, forfeiting the connection.
    }

    assert_eq!(
        (a.snapshot().hash(), a.snapshot().latest().clone()),
        a_before
    );
    assert_eq!(
        (b.snapshot().hash(), b.snapshot().latest().clone()),
        b_before
    );

    // The forfeit is enforced, not just documented: both ends' links are
    // poisoned, so reuse fails fast — with no counterparty driving, which
    // the closed-world harness itself proves the fail-fast does not need.
    let retry = run_to_quiescence(a.gossip(&mut a_link)).expect("fail-fast needs no peer");
    assert!(
        matches!(retry, Err(Error::LinkPoisoned)),
        "reusing A's forfeited link must fail fast, got {retry:?}"
    );
    let retry = run_to_quiescence(b.gossip(&mut b_link)).expect("fail-fast needs no peer");
    assert!(
        matches!(retry, Err(Error::LinkPoisoned)),
        "reusing B's forfeited link must fail fast, got {retry:?}"
    );

    // A fresh connection converges the pair as if nothing had happened.
    wire_gossip_async(&a, &b).await;
    assert_eq!(a.snapshot().hash(), b.snapshot().hash());
    assert_eq!(a.snapshot().len(), 2);
}

/// A driver started on an already-poisoned link yields its terminal
/// [`Error::LinkPoisoned`] immediately, even though its policy stream
/// never ticks and no peer drives the other end.
///
/// The poison check precedes
/// the driver's idle select, so a dead link cannot masquerade as a live,
/// quietly parked driver.
#[test]
fn a_driver_on_a_poisoned_link_fails_fast() {
    let a: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
    let (mut a_link, _b_link) = links();

    // Poison the link: cancel a one-shot session stalled on its silent
    // counterparty (the bounded-poll harness reports the stall and drops
    // the session future on the way out).
    assert!(
        run_to_quiescence(a.gossip(&mut a_link)).is_err(),
        "the session must stall against a silent peer, then cancel"
    );

    // The policy stream is pending forever, and nothing drives the other
    // end: only the pre-select fail-fast can resolve this, which the
    // closed-world harness proves happens promptly rather than hanging.
    let mut sessions = a.gossip_when(stream::pending::<()>(), &mut a_link);
    let item = run_to_quiescence(sessions.next())
        .expect("the fail-fast must not wait for a tick or a peer");
    assert!(
        matches!(item, Some(Err(Error::LinkPoisoned))),
        "a driver on a poisoned link must yield LinkPoisoned, got {item:?}"
    );
    assert!(
        run_to_quiescence(sessions.next())
            .expect("the driver must end after its terminal error")
            .is_none(),
        "the stream must end after its terminal error"
    );
}

/// Polling is cancel-safe, as documented: a consumer that creates and
/// drops a fresh `next()` future on every poll — never holding one across
/// an await — still drives sessions to completion with nothing lost.
#[test]
fn dropping_next_futures_loses_nothing() {
    // Deliberately use a minimal executor rather than Tokio: besides avoiding
    // Tokio's cooperative task budget in this manual-poll test, this pins the
    // public driver's promise that it does not require a Tokio runtime.
    let (a, b) = pollster::block_on(pair());
    let (mut a_link, mut b_link) = links();
    let (a_tx, a_when) = ticks();
    let mut a_sessions = a.gossip_when(a_when, &mut a_link);
    let mut b_sessions = b.gossip_when(stream::pending::<()>(), &mut b_link);

    a.send(1);
    a_tx.unbounded_send(()).expect("driver alive");

    // Every poll creates each driver's `next()` future afresh, polls it once,
    // and drops it. Self-waking asks the quiescence detector for another poll;
    // the detector supplies the deterministic progress guard for this closed
    // in-memory system.
    let (mut a_item, mut b_item) = (None, None);
    run_to_quiescence(poll_fn(|cx| {
        if a_item.is_none() {
            a_item = a_sessions.next().now_or_never().flatten();
        }
        if b_item.is_none() {
            b_item = b_sessions.next().now_or_never().flatten();
        }
        if a_item.is_some() && b_item.is_some() {
            Poll::Ready(())
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }))
    .expect("dropped next futures stalled the session");
    a_item.unwrap().expect("A's session");
    b_item.expect("B's session completed").expect("B's session");
    assert_eq!(a.snapshot().hash(), b.snapshot().hash());
}

/// A driver that ended cleanly leaves the connection at a session
/// boundary, as documented: the same link then hosts a
/// one-shot `gossip`, and after that a second driver, against the
/// counterparty's still-running responder.
#[tokio::test(flavor = "current_thread")]
async fn a_clean_end_leaves_the_connection_reusable() {
    let (a, b) = pair().await;
    let (mut a_link, mut b_link) = links();
    let mut b_sessions = b.gossip_when(stream::pending::<()>(), &mut b_link);

    // Phase 1: a single-tick driver runs one session and ends cleanly.
    a.send(1);
    {
        let once = stream::once(std::future::ready(()));
        let mut a_sessions = a.gossip_when(once, &mut a_link);
        let (a_item, b_item) = timeout(
            DEADLINE,
            futures::future::join(a_sessions.next(), b_sessions.next()),
        )
        .await
        .expect("phase 1 deadlocked");
        a_item.expect("A's driver running").expect("A's session");
        b_item.expect("B's driver running").expect("B's session");
        assert!(
            timeout(DEADLINE, a_sessions.next())
                .await
                .expect("phase 1 shutdown deadlocked")
                .is_none(),
            "single-tick driver ends after its session"
        );
    }

    // Phase 2: the same link hosts a one-shot `gossip`.
    a.send(2);
    let (a_out, b_item) = timeout(
        DEADLINE,
        futures::future::join(a.gossip(&mut a_link), b_sessions.next()),
    )
    .await
    .expect("phase 2 deadlocked");
    a_out.expect("one-shot gossip on the reused connection");
    b_item.expect("B's driver running").expect("B's session");

    // Phase 3: and then a second driver.
    a.send(3);
    {
        let once = stream::once(std::future::ready(()));
        let mut a_sessions = a.gossip_when(once, &mut a_link);
        let (a_item, b_item) = timeout(
            DEADLINE,
            futures::future::join(a_sessions.next(), b_sessions.next()),
        )
        .await
        .expect("phase 3 deadlocked");
        a_item.expect("A's driver running").expect("A's session");
        b_item.expect("B's driver running").expect("B's session");
    }

    assert_eq!(a.snapshot().hash(), b.snapshot().hash());
    assert_eq!(a.snapshot().len(), 3);
}

/// Dropping a driver releases its `Rumors` clone like any other: the
/// remaining handle reclaims the `Peer`.
#[pollster::test]
async fn a_dropped_driver_does_not_block_peer_reclaim() {
    let rumors: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
    let (mut a_link, _b_link) = links();
    {
        let _driver = rumors.gossip_when(stream::pending::<()>(), &mut a_link);
    }
    assert!(rumors.try_into_peer().await.is_some());
}

proptest! {
    /// A connection severed at an arbitrary byte offset — either side's
    /// write direction, any budget, including zero — fails loudly and
    /// recoverably.
    ///
    /// That is: no hang, each driver ends after at most one terminal
    /// `Err`, each replica still holds its own sends and nothing beyond
    /// the union (a torn session commits all of a reconciliation or none
    /// of it), a session `Ok` from either driver certifies the pair had
    /// already converged before any recovery (the epilogue's certification,
    /// held under arbitrary cut geometry rather than only at pinned byte
    /// boundaries), and a fresh clean connection converges the pair fully.
    #[test]
    fn severed_connections_fail_loudly_and_recover(
        a_write_cut in 0usize..400,
        b_write_cut in 0usize..400,
    ) {
        block_on(async {
            let (a, b) = pair().await;
            a.send(1);
            b.send(2);

            let (a_side, b_side) = rumors::link::memory_with_capacity(LINK_BUF);
            let a_link = faulty(a_side, FaultPlan {
                write_cut: Some(a_write_cut),
                read_cut: None,
            });
            let b_link = faulty(b_side, FaultPlan {
                write_cut: Some(b_write_cut),
                read_cut: None,
            });

            // Each side's link is owned by its future, so the failing
            // side's drop surfaces as EOF to the other rather than
            // deadlocking the join.
            let a_task = async {
                let mut a_link = a_link;
                let once = stream::once(std::future::ready(()));
                a.gossip_when(once, &mut a_link)
                    .collect::<Vec<_>>()
                    .await
            };
            let b_task = async {
                let mut b_link = b_link;
                b.gossip_when(stream::pending::<()>(), &mut b_link)
                    .collect::<Vec<_>>()
                    .await
            };
            let (a_items, b_items) = timeout(DEADLINE, futures::future::join(a_task, b_task))
                .await
                .expect("a severed connection wedged a driver");

            // Terminal shape: zero or more Ok items, then at most one Err.
            for items in [&a_items, &b_items] {
                if let Some(err_at) = items.iter().position(|i| i.is_err()) {
                    assert_eq!(err_at, items.len() - 1, "Err must be terminal");
                }
            }

            // Atomicity: whatever happened, each side holds its own send,
            // nothing beyond the union, and never a torn intermediate.
            let (a_snapshot, b_snapshot) = (a.snapshot(), b.snapshot());
            assert!(a_snapshot.iter().any(|(_, _, m)| **m == 1));
            assert!(b_snapshot.iter().any(|(_, _, m)| **m == 2));
            assert!(a_snapshot.len() <= 2);
            assert!(b_snapshot.len() <= 2);

            // Certification: the epilogue's central promise, under a cut at
            // an arbitrary offset. A driver yields `Ok` only after reading
            // the peer's completion marker, which the peer writes only
            // after its own commit — so any `Ok` on either side means both
            // replicas already hold the session's full union, here, before
            // the recovery gossip has run.
            if a_items.iter().any(|i| i.is_ok()) || b_items.iter().any(|i| i.is_ok()) {
                assert_eq!(
                    a_snapshot.hash(),
                    b_snapshot.hash(),
                    "a session Ok certifies the peer committed: the pair must already be converged",
                );
                assert_eq!(a_snapshot.len(), 2, "the certified session moved both sends");
            }

            // Recovery: a fresh, clean connection converges the pair.
            wire_gossip_async(&a, &b).await;
            let (a_snapshot, b_snapshot) = (a.snapshot(), b.snapshot());
            assert_eq!(a_snapshot.hash(), b_snapshot.hash());
            assert_eq!(a_snapshot.len(), 2);
        });
    }
}

/// One step of a chaos script: a commit on either side, a tick to either
/// driver, or letting the schedulers run for a few polls.
///
/// Ticks and commits
/// are deliberately decoupled — a tick may arrive with nothing new (must
/// suppress), late (covering several commits), or while a session is
/// already in flight on the same or the opposite side.
#[derive(Debug, Clone, Copy)]
enum Op {
    SendA,
    SendB,
    TickA,
    TickB,
    /// Yield to the drivers this many times, letting in-flight sessions
    /// progress (or not) between script steps.
    Pump(u8),
}

fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        Just(Op::SendA),
        Just(Op::SendB),
        Just(Op::TickA),
        Just(Op::TickB),
        (1u8..6).prop_map(Op::Pump),
    ]
}

proptest! {
    /// Chaos: under *any* interleaving of commits, ticks, and scheduler
    /// progress on both sides of one connection, no session errors and
    /// full convergence.
    ///
    /// The interleavings include simultaneous initiations, ticks racing
    /// in-flight sessions, suppressed ticks, and idle pumps. Every
    /// driver ends cleanly once its tick source closes, and the pair
    /// converges on exactly the union of both sides' sends.
    #[test]
    fn chaotic_tick_interleavings_converge_without_error(
        script in proptest::collection::vec(op_strategy(), 0..48),
    ) {
        block_on(async {
            let (a, b) = pair().await;
            let (mut a_link, mut b_link) = links();
            let (a_tx, a_when) = ticks();
            let (b_tx, b_when) = ticks();

            let a_sessions = a.gossip_when(a_when, &mut a_link);
            let b_sessions = b.gossip_when(b_when, &mut b_link);

            // Collectors poll the drivers to completion; any session error
            // panics, which is the test's core assertion.
            let collect_a = a_sessions.for_each(|item| async move {
                item.expect("A session errored");
            });
            let collect_b = b_sessions.for_each(|item| async move {
                item.expect("B session errored");
            });

            let mut sent = 0u64;
            let feeder = async {
                for op in &script {
                    match op {
                        Op::SendA => {
                            a.send(sent);
                            sent += 1;
                        }
                        Op::SendB => {
                            b.send(1_000_000 + sent);
                            sent += 1;
                        }
                        Op::TickA => a_tx.unbounded_send(()).expect("A driver holds its rx"),
                        Op::TickB => b_tx.unbounded_send(()).expect("B driver holds its rx"),
                        Op::Pump(n) => {
                            for _ in 0..*n {
                                tokio::task::yield_now().await;
                            }
                        }
                    }
                }

                // Epilogue: one final tick each guarantees every side pushes
                // whatever the script left unsynced (a suppressed final tick
                // means there was nothing), then wait for the frontiers to
                // meet before closing the tick sources — a driver must never
                // end while its counterparty could still initiate.
                a_tx.unbounded_send(()).expect("A driver holds its rx");
                b_tx.unbounded_send(()).expect("B driver holds its rx");
                while a.snapshot().latest() != b.snapshot().latest() {
                    tokio::task::yield_now().await;
                }
                drop(a_tx);
                drop(b_tx);
            };

            timeout(
                DEADLINE,
                futures::future::join3(collect_a, collect_b, feeder),
            )
            .await
            .expect("chaos script deadlocked");

            let (a_snapshot, b_snapshot) = (a.snapshot(), b.snapshot());
            assert_eq!(a_snapshot.hash(), b_snapshot.hash());
            assert_eq!(a_snapshot.latest(), b_snapshot.latest());
            assert_eq!(a_snapshot.len() as u64, sent, "the union of all sends");
        });
    }
}

/// A peer that dies mid-preamble is an error, not a goodbye: the driver
/// yields one terminal `Err` and then ends.
#[tokio::test(flavor = "current_thread")]
async fn truncated_initiation_is_a_terminal_error() {
    let a: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
    let (mut a_link, b) = rumors::link::memory_with_capacity(LINK_BUF);
    // Drive the counterparty's control stream by hand: keep its read half (and
    // the data-stream connector/acceptor) alive so A's own writes succeed,
    // while its control-write half toward A carries only a partial preamble.
    let b = b.into_parts();
    let mut b_control_write = b.control_write;

    let mut a_sessions = a.gossip_when(stream::pending::<()>(), &mut a_link);

    // Four bytes of a preamble, then hang up mid-preamble. Closing the
    // control-write half toward A signals end-of-stream on A's control read.
    b_control_write
        .write_all(b"RUMO")
        .await
        .expect("partial write");
    drop(b_control_write);

    let item = timeout(DEADLINE, a_sessions.next())
        .await
        .expect("error never surfaced")
        .expect("driver yielded its terminal item");
    match item {
        Err(Error::Io(e)) => assert_eq!(e.kind(), std::io::ErrorKind::UnexpectedEof),
        other => panic!("expected Io(UnexpectedEof), got {other:?}"),
    }
    assert!(
        timeout(DEADLINE, a_sessions.next())
            .await
            .expect("end after error deadlocked")
            .is_none(),
        "the stream must end after its terminal error"
    );
}

/// A transport error on the idle boundary — the control read fails before
/// one preamble byte arrives — is a terminal `Err` that poisons the link.
///
/// This is the error terminal's poisoning promise on its subtlest path:
/// zero session bytes moved, so no misframing is even possible, but a
/// transport that errored is not a link a later session should trust.
/// Reuse must fail fast with [`Error::LinkPoisoned`] rather than retrying
/// I/O against the dead transport. (The in-memory duplex can only EOF, so
/// the read error comes from the fault harness with a zero-byte budget.)
#[tokio::test(flavor = "current_thread")]
async fn a_control_read_error_on_the_idle_boundary_poisons_the_link() {
    let a: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
    let (a_link, _b_link) = links();
    let mut a_link = faulty(
        a_link,
        FaultPlan {
            write_cut: None,
            read_cut: Some(0),
        },
    );

    {
        let mut a_sessions = a.gossip_when(stream::pending::<()>(), &mut a_link);
        let item = timeout(DEADLINE, a_sessions.next())
            .await
            .expect("error never surfaced")
            .expect("driver yielded its terminal item");
        assert!(
            matches!(item, Err(Error::Io(_))),
            "a control read error is the driver's terminal Err, got {item:?}",
        );
        assert!(
            timeout(DEADLINE, a_sessions.next())
                .await
                .expect("end after error deadlocked")
                .is_none(),
            "the stream must end after its terminal error"
        );
    }

    // The staging buffer never held a byte, yet the link is poisoned: the
    // fail-fast happens before any I/O, so no counterparty is needed.
    let retry = timeout(DEADLINE, a.gossip(&mut a_link))
        .await
        .expect("the fail-fast must not wait for a peer");
    assert!(
        matches!(retry, Err(Error::LinkPoisoned)),
        "reuse after an error terminal must fail fast, got {retry:?}",
    );
}
