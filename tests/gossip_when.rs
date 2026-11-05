//! Continuous gossip under per-connection initiation policies.
//!
//! Hand-driven policy streams exercise suppression, remote serving, connection reuse,
//! and cancellation. Closed in-memory schedules use the quiescence detector, including
//! every assertion that a driver has gone quiet. Tokio tasks carry a wall-clock
//! backstop only when the test expects progress.

use rumors_testkit::common;
use rumors_testkit::common::wire::ROOMY_CAPACITY;

use std::{
    future::{Future, poll_fn},
    task::Poll,
    time::Duration,
};

use futures::channel::mpsc::{UnboundedSender, unbounded};
use futures::stream;
use futures::{FutureExt, StreamExt};
use proptest::prelude::*;
use rumors::{
    Error, Gossip, Gossiped, Led, Rumors,
    testing::{Quiescence, run_to_quiescence},
};
use tokio::io::AsyncWriteExt;
use tokio::time::timeout;

use crate::common::fault::{self, FaultPlan, faulty};
use crate::common::wire::{bootstrap_fork_async, tokio_block_on as block_on, wire_gossip_async};
use rumors_testkit::common::property::{LOCAL_DIVISOR, SESSION_DIVISOR};
use rumors_testkit::common::seed::{NetworkSeed, network_seed};
use rumors_testkit::divided_proptest;

/// Generous wall-clock bound: everything here is in-memory and finishes in
/// microseconds, so hitting the deadline means a wedged driver, not a slow
/// machine.
const DEADLINE: Duration = Duration::from_secs(10);

/// A connected, party-disjoint pair: the first peer of the network `network`
/// names and a bootstrap fork of it.
async fn pair(network: NetworkSeed) -> (Rumors<u64>, Rumors<u64>) {
    let a: Rumors<u64> = network.peer().sync_memory_budget(0).into_rumors();
    let b = bootstrap_fork_async(&a).await;
    (a, b)
}

/// The two ends of one in-memory connection between two drivers.
fn connection() -> (rumors::testing::MemoryEnd, rumors::testing::MemoryEnd) {
    rumors::testing::memory_connection(ROOMY_CAPACITY)
}

/// Send policy items manually. Unit items request gossip when the set changes;
/// probe tests send explicit [`Gossip`] requests.
fn ticks<I>() -> (UnboundedSender<I>, impl stream::Stream<Item = I>) {
    let (tx, rx) = unbounded();
    (tx, rx)
}

/// Assert that a closed in-memory future has parked without producing an item.
fn assert_stalled(future: impl Future, failure: &str) {
    match run_to_quiescence(future) {
        Err(Quiescence::Stalled) => {}
        Err(Quiescence::PollBudget) => {
            panic!("{failure}: the future kept waking without completing")
        }
        Ok(_) => panic!("{failure}"),
    }
}

/// Install a hand-driven stream on a fixture that owns its sole gossip handle.
/// The fixture permits exactly one driver to consume this stream.
fn with_policy_stream<S>(rumors: Rumors<u64>, policy: S) -> Rumors<u64>
where
    S: stream::Stream + Send + 'static,
    S::Item: Into<Gossip>,
{
    let policy = std::sync::Mutex::new(Some(policy));
    rumors
        .try_into_peer()
        .now_or_never()
        .expect("fixture owns the only gossip handle")
        .expect("fixture reclaims its peer")
        .gossip_when(move |_| {
            policy
                .lock()
                .unwrap()
                .take()
                .expect("one driver per policy stream")
        })
        .into_rumors()
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

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// A policy with one immediate item requests one session, like `gossip_once`:
    /// converged replicas, exactly one `Ok` item per side, then a
    /// clean end — and the suppression token both sides report is the same
    /// frontier.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn single_tick_reduces_to_gossip(network in network_seed()) {
        block_on(async {
            let (a, b) = pair(network).await;
            a.send(1).unwrap();
            b.send(2).unwrap();

            let (mut a_end, mut b_end) = connection();
            let once = || stream::once(std::future::ready(()));
            let a = with_policy_stream(a, once());
            let mut a_sessions = a.gossip(&mut a_end.read, &mut a_end.write);
            let b = with_policy_stream(b, once());
            let mut b_sessions = b.gossip(&mut b_end.read, &mut b_end.write);

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
        });
    }

    /// A pending `when` stream is a pure responder: it never initiates, serves
    /// every remote-led session, and attributes leadership correctly on both
    /// sides.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn pending_when_serves_remote_initiations(network in network_seed()) {
        block_on(async {
            let (a, b) = pair(network).await;
            let (mut a_end, mut b_end) = connection();

            let (a_tx, a_when) = ticks();
            let a = with_policy_stream(a, a_when);
            let mut a_sessions = a.gossip(&mut a_end.read, &mut a_end.write);
            let b = with_policy_stream(b, stream::pending::<()>());
            let mut b_sessions = b.gossip(&mut b_end.read, &mut b_end.write);

            for round in 0..3u64 {
                a.send(round).unwrap();
                a_tx.unbounded_send(()).expect("driver alive");
                let (a_session, b_session) = one_round(&mut a_sessions, &mut b_sessions).await;
                assert_eq!(a_session.led, Led::Local, "round {round}: A initiated");
                assert_eq!(b_session.led, Led::Remote, "round {round}: B responded");
                assert_eq!(a.snapshot().hash(), b.snapshot().hash());
            }
        });
    }

    /// Suppression is exact, both ways.
    ///
    /// A tick with nothing new since this
    /// connection converged initiates nothing — the echo tick that
    /// [`rumors::Rumors::changes`] fires after a session's own join produces no
    /// second session — while a tick after a real change always initiates.
    /// Wired with `changes()` itself, the way a real driver is.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn suppression_swallows_echoes_not_news(network in network_seed()) {
        block_on(async {
            let (a, b) = pair(network).await;
            let (mut a_end, mut b_end) = connection();

            // Real drivers: each side's policy stream is its own change signal.
            let mut a_sessions = a.gossip(&mut a_end.read, &mut a_end.write);
            let mut b_sessions = b.gossip(&mut b_end.read, &mut b_end.write);

            // Round 1: the initial `changes()` yield on both sides drives the
            // reconnect-convergence session (both led locally; one session total).
            a.send(1).unwrap();
            let (a_session, b_session) = one_round(&mut a_sessions, &mut b_sessions).await;
            assert_eq!(a.snapshot().hash(), b.snapshot().hash());

            // Both replicas converged on the same frontier.
            assert_eq!(a_session.converged, b_session.converged);

            // The session's join advanced each side's frontier, so each side's
            // `changes()` has an echo tick queued. Suppression must swallow it:
            // polling both drivers now yields nothing — no echo session — and the
            // drivers go quiet.
            assert_stalled(
                futures::future::join(a_sessions.next(), b_sessions.next()),
                "an echo session ran on a converged connection",
            );

            // But real news still initiates: a change on B drives exactly one more
            // session, B-led.
            b.send(2).unwrap();
            let (a_session, b_session) = one_round(&mut a_sessions, &mut b_sessions).await;
            assert_eq!(b_session.led, Led::Local, "B had the news");
            assert_eq!(a_session.led, Led::Remote, "A only served");
            assert_eq!(a.snapshot().hash(), b.snapshot().hash());
            assert_eq!(a.snapshot().len(), 2);
        });
    }

    /// An interval-style `()` tick — the when-changed default — on a converged
    /// connection costs nothing, and the same tick stream initiates again as
    /// soon as the local frontier has really moved.
    ///
    /// This is the anti-entropy property change-conditional heartbeats rely
    /// on, pinned with hand-fed ticks instead of a timer.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn heartbeat_ticks_are_free_until_divergence(network in network_seed()) {
        block_on(async {
            let (a, b) = pair(network).await;
            let (mut a_end, mut b_end) = connection();

            let (a_tx, a_when) = ticks();
            let a = with_policy_stream(a, a_when);
            let mut a_sessions = a.gossip(&mut a_end.read, &mut a_end.write);
            let b = with_policy_stream(b, stream::pending::<()>());
            let mut b_sessions = b.gossip(&mut b_end.read, &mut b_end.write);

            // First tick: fresh driver, no token yet — the unconditional first
            // session (reconnect convergence).
            a_tx.unbounded_send(()).expect("driver alive");
            one_round(&mut a_sessions, &mut b_sessions).await;

            // Converged: heartbeat ticks are suppressed, nothing crosses the wire.
            for _ in 0..3 {
                a_tx.unbounded_send(()).expect("driver alive");
            }
            assert_stalled(
                futures::future::join(a_sessions.next(), b_sessions.next()),
                "a heartbeat tick initiated a session on a converged connection",
            );

            // Diverged, with a tick stream that carries no change information (an
            // interval stream never does): the next heartbeat tick fires a real
            // session. This recovery is what makes an interval policy a liveness
            // net for whatever a chosen `when` stream stays quiet about.
            a.send(7).unwrap();
            a_tx.unbounded_send(()).expect("driver alive");
            let (a_session, _) = one_round(&mut a_sessions, &mut b_sessions).await;
            assert_eq!(a_session.led, Led::Local);
            assert_eq!(a.snapshot().hash(), b.snapshot().hash());
        });
    }

    /// An unconditional request initiates on a converged connection where a
    /// when-changed request stays suppressed.
    ///
    /// The probe session is a full end-to-end round-trip, it re-converges on
    /// the same frontier, and it wakes no `changes()` observer on either
    /// side. That last clause is the invariant that keeps unconditional probing
    /// from ever feeding an echo loop through a change-driven policy stream:
    /// a no-news session's join is a no-op at both ends, so no change notification is emitted.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn unconditional_requests_probe_a_converged_connection(network in network_seed()) {
        block_on(async {
            let (a, b) = pair(network).await;
            let (mut a_end, mut b_end) = connection();

            let (a_tx, a_when) = ticks();
            let a = with_policy_stream(a, a_when);
            let mut a_sessions = a.gossip(&mut a_end.read, &mut a_end.write);
            let b = with_policy_stream(b, stream::pending::<Gossip>());
            let mut b_sessions = b.gossip(&mut b_end.read, &mut b_end.write);

            // Converge the pair: the fresh driver's first policy item always initiates
            // (reconnect convergence).
            a.send(1).unwrap();
            a_tx.unbounded_send(Gossip::WhenChanged)
                .expect("driver alive");
            let (first, _) = one_round(&mut a_sessions, &mut b_sessions).await;

            // Fresh observers owe an immediate first yield; drain it so a later
            // yield means a genuine post-convergence advance.
            let mut a_changes = a.changes();
            let mut b_changes = b.changes();
            a_changes.next().await.expect("set open");
            b_changes.next().await.expect("set open");

            // Unconditional requests on the converged connection: each initiates a
            // real session (A initiated it, B only served), and each re-converges on
            // the same frontier — the round-trip carried no news.
            for probe in 0..2 {
                a_tx.unbounded_send(Gossip::Unconditionally)
                    .expect("driver alive");
                let (a_session, b_session) = one_round(&mut a_sessions, &mut b_sessions).await;
                assert_eq!(a_session.led, Led::Local, "probe {probe}: A initiated");
                assert_eq!(b_session.led, Led::Remote, "probe {probe}: B served");
                assert_eq!(a_session.converged, first.converged, "probe {probe}");
                assert_eq!(b_session.converged, first.converged, "probe {probe}");
            }

            // No observer woke: the probes' joins were no-ops at both ends.
            assert_stalled(
                futures::future::join(a_changes.next(), b_changes.next()),
                "a no-news probe session woke a changes() observer",
            );

            // A when-changed request on the same converged connection still initiates
            // nothing: the two policies differ exactly in the suppression check.
            a_tx.unbounded_send(Gossip::WhenChanged)
                .expect("driver alive");
            assert_stalled(
                futures::future::join(a_sessions.next(), b_sessions.next()),
                "a when-changed request initiated a session on a converged connection",
            );
        });
    }

    /// Suppression is per connection, not per set: a change at A crosses B and
    /// reaches C through B's two independent drivers (B's join on the A-side
    /// connection is exactly the news its C-side driver must push).
    ///
    /// The claim holds for every network seed.
    #[test]
    fn changes_propagate_transitively_through_a_chain(network in network_seed()) {
        block_on(async {
            let a: Rumors<u64> = network.peer().sync_memory_budget(0).into_rumors();
            let b = bootstrap_fork_async(&a).await;
            let c = bootstrap_fork_async(&b).await;

            let (mut ab_a_end, mut ab_b_end) = connection();
            let (mut bc_b_end, mut bc_c_end) = connection();

            a.send(42).unwrap();

            // Four drivers, every policy stream a real change signal. Consume
            // session items in the background of the convergence check: the
            // drivers only progress while polled.
            let a_drv = a.gossip(&mut ab_a_end.read, &mut ab_a_end.write);
            let b_ab_drv = b.gossip(&mut ab_b_end.read, &mut ab_b_end.write);
            let b_bc_drv = b.gossip(&mut bc_b_end.read, &mut bc_b_end.write);
            let c_drv = c.gossip(&mut bc_c_end.read, &mut bc_c_end.write);
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
                    if snapshot.iter().any(|(_, m)| *m == 42) {
                        return;
                    }
                }
            };

            tokio::select! {
                _ = drive_all => unreachable!("changes()-fed drivers never end while handles live"),
                out = timeout(DEADLINE, converged) => out.expect("A's change never reached C"),
            }
        });
    }

    /// Transitive propagation holds for a redaction that reaches the middle
    /// peer as pure frontier news: the advance B absorbs on its A-side
    /// connection is exactly the news its C-side driver must push.
    ///
    /// This is the same relay contract
    /// `changes_propagate_transitively_through_a_chain` pins for content, on
    /// an A–B–C chain of `changes()`-fed drivers. The frontier *is* the
    /// redaction — a peer whose ceiling covers a message's version drops that
    /// message on contact — so a chain that relays content but not frontiers
    /// propagates sends while stalling redactions.
    ///
    /// A sends and redacts between driver polls, so the A–B session hands B a
    /// ceiling-only advance (B never held the message, and both trees are
    /// empty). The gossip write-back's notification counts that frontier
    /// advance, so B's `changes()` ticks and B's C-side driver relays it —
    /// while frontier-based suppression keeps the same wake from echoing a
    /// session back on the A-side connection, so the relay initiates once and
    /// the chain quiesces.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn a_redaction_frontier_propagates_transitively_through_a_chain(network in network_seed()) {
        block_on(async {
            let a: Rumors<u64> = network.peer().sync_memory_budget(0).into_rumors();
            let b = bootstrap_fork_async(&a).await;
            let c = bootstrap_fork_async(&b).await;

            let (mut ab_a_end, mut ab_b_end) = connection();
            let (mut bc_b_end, mut bc_c_end) = connection();

            let mut a_drv = a.gossip(&mut ab_a_end.read, &mut ab_a_end.write);
            let mut b_ab_drv = b.gossip(&mut ab_b_end.read, &mut ab_b_end.write);
            let mut b_bc_drv = b.gossip(&mut bc_b_end.read, &mut bc_b_end.write);
            let mut c_drv = c.gossip(&mut bc_c_end.read, &mut bc_c_end.write);

            // The unconditional first sessions converge both connections.
            let initial =
                futures::future::join4(a_drv.next(), b_ab_drv.next(), b_bc_drv.next(), c_drv.next());
            let (a0, b0, b1, c0) = timeout(DEADLINE, initial)
                .await
                .expect("initial sessions deadlocked");
            for item in [a0, b0, b1, c0] {
                item.expect("driver running").expect("initial session");
            }

            // Converged and quiet: every echo tick is suppressed.
            assert_stalled(
                futures::future::join4(a_drv.next(), b_ab_drv.next(), b_bc_drv.next(), c_drv.next()),
                "a converged chain must be quiet",
            );

            // A sends and redacts between driver polls: the commit's only surviving
            // trace is A's advanced causal frontier.
            let version = a.send(42).unwrap();
            a.redact(&version);

            // The A-side connection carries the news to B...
            let ab = futures::future::join(a_drv.next(), b_ab_drv.next());
            let (a1, b2) = timeout(DEADLINE, ab)
                .await
                .expect("the A-led session never ran");
            a1.expect("driver running").expect("A-led session");
            b2.expect("driver running").expect("B's serving session");
            assert_eq!(
                b.snapshot().latest(),
                a.snapshot().latest(),
                "B absorbed the redaction frontier"
            );

            // ...and B's C-side driver must relay it: transitive propagation.
            let bc = futures::future::join(b_bc_drv.next(), c_drv.next());
            let (b3, c1) = timeout(DEADLINE, bc).await.expect(
                "the redaction frontier never crossed the B-C connection: \
                 B's ceiling-only advance did not tick its changes()",
            );
            b3.expect("driver running").expect("B-led session");
            c1.expect("driver running").expect("C's serving session");
            assert_eq!(
                c.snapshot().latest(),
                a.snapshot().latest(),
                "C absorbed the redaction frontier"
            );
        });
    }

    /// When the `when` stream ends with nothing in flight, the driver ends
    /// cleanly — and its still-running counterparty sees the dropped connection
    /// as a clean goodbye (end-of-stream at a session boundary), not an error.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn when_exhaustion_then_hangup_both_end_cleanly(network in network_seed()) {
        block_on(async {
            let (a, b) = pair(network).await;
            let (mut a_end, mut b_end) = connection();

            // A's `when` is already exhausted: its driver ends without a session.
            let a = with_policy_stream(a, stream::empty::<()>());
            let mut a_sessions = a.gossip(&mut a_end.read, &mut a_end.write);
            assert!(
                timeout(DEADLINE, a_sessions.next())
                    .await
                    .expect("A's shutdown deadlocked")
                    .is_none()
            );
            drop(a_sessions);
            let b = with_policy_stream(b, stream::pending::<()>());
            let mut b_sessions = b.gossip(&mut b_end.read, &mut b_end.write);

            // Dropping A's end hangs the connection up at a session
            // boundary; B's responder ends cleanly.
            drop(a_end);
            assert!(
                timeout(DEADLINE, b_sessions.next())
                    .await
                    .expect("B's shutdown deadlocked")
                    .is_none()
            );
        });
    }

    /// Dropping a driver mid-session commits nothing: both replicas are
    /// byte-identical to their pre-session state, and a fresh connection
    /// afterwards converges the pair from scratch.
    ///
    /// The drop forfeits the connection — the price the crate docs' "What a
    /// session promises" section documents: its position is unknown, so it is
    /// discarded.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn dropping_a_driver_mid_session_commits_nothing(network in network_seed()) {
        pollster::block_on(async {
            let (a, b) = pair(network).await;
            a.send(1).unwrap();
            b.send(2).unwrap();
            let a_before = (a.snapshot().hash(), a.snapshot().latest().clone());
            let b_before = (b.snapshot().hash(), b.snapshot().latest().clone());

            let (mut a_end, mut b_end) = connection();
            let (a_tx, a_when) = ticks();
            let a = with_policy_stream(a, a_when);
            let b = with_policy_stream(b, stream::pending::<()>());
            {
                let mut a_sessions = a.gossip(&mut a_end.read, &mut a_end.write);
                let mut b_sessions = b.gossip(&mut b_end.read, &mut b_end.write);

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

            // A fresh connection converges the pair as if nothing had happened.
            wire_gossip_async(&a, &b).await;
            assert_eq!(a.snapshot().hash(), b.snapshot().hash());
            assert_eq!(a.snapshot().len(), 2);
        });
    }

    /// Polling is cancel-safe, as documented: a consumer that creates and
    /// drops a fresh `next()` future on every poll — never holding one across
    /// an await — still drives sessions to completion with nothing lost.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn dropping_next_futures_loses_nothing(network in network_seed()) {
        // Deliberately use a minimal executor rather than Tokio: besides avoiding
        // Tokio's cooperative task budget in this manual-poll test, this pins the
        // public driver's promise that it does not require a Tokio runtime.
        let (a, b) = pollster::block_on(pair(network));
        let (mut a_end, mut b_end) = connection();
        let (a_tx, a_when) = ticks();
        let a = with_policy_stream(a, a_when);
        let mut a_sessions = a.gossip(&mut a_end.read, &mut a_end.write);
        let b = with_policy_stream(b, stream::pending::<()>());
        let mut b_sessions = b.gossip(&mut b_end.read, &mut b_end.write);

        a.send(1).unwrap();
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

    /// A cleanly ended driver leaves the connection usable for a one-shot exchange
    /// and a later driver, while its counterparty keeps responding.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn a_clean_end_leaves_the_connection_reusable(network in network_seed()) {
        block_on(async {
            let (a, b) = pair(network).await;
            let a = a
                .try_into_peer()
                .await
                .unwrap()
                .gossip_when(|_| stream::once(std::future::ready(())))
                .into_rumors();
            let (mut a_end, mut b_end) = connection();
            let b = with_policy_stream(b, stream::pending::<()>());
            let mut b_sessions = b.gossip(&mut b_end.read, &mut b_end.write);

            // Phase 1: a single-tick driver runs one session and ends cleanly.
            a.send(1).unwrap();
            {
                let mut a_sessions = a.gossip(&mut a_end.read, &mut a_end.write);
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

            // Phase 2: the same connection hosts a one-shot `gossip_once`.
            a.send(2).unwrap();
            let (a_out, b_item) = timeout(
                DEADLINE,
                futures::future::join(
                    a.gossip_once(&mut a_end.read, &mut a_end.write),
                    b_sessions.next(),
                ),
            )
            .await
            .expect("phase 2 deadlocked");
            a_out.expect("one-shot gossip on the reused connection");
            b_item.expect("B's driver running").expect("B's session");

            // Phase 3: and then a second driver.
            a.send(3).unwrap();
            {
                let mut a_sessions = a.gossip(&mut a_end.read, &mut a_end.write);
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
        });
    }
}

divided_proptest! {
    #![divisor(LOCAL_DIVISOR)]

    /// Dropping a driver releases its `Rumors` clone like any other: the
    /// remaining handle reclaims the `Peer`.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn a_dropped_driver_does_not_block_peer_reclaim(network in network_seed()) {
        pollster::block_on(async {
            let rumors: Rumors<u64> = network.peer().sync_memory_budget(0).into_rumors();
            let (mut a_end, _b_end) = connection();
            let rumors = with_policy_stream(rumors, stream::pending::<()>());
            {
                let _driver = rumors.gossip(&mut a_end.read, &mut a_end.write);
            }
            assert!(rumors.try_into_peer().await.is_some());
        });
    }
}

/// Upper bound on the byte offset at which a severed connection's cut
/// can land.
///
/// Derived from measurement: a clean run of the severed-connection
/// fixture ([`run_severed`]) moves fewer bytes per endpoint than this
/// bound, and the bound stays within twice that, so generated cuts reach
/// every byte of the session and keep landing inside it. The two-sided
/// pin is [`max_sever_cut_spans_the_fixture_session`]; re-measure there
/// before touching this number.
const MAX_SEVER_CUT: usize = 256;

/// What one severed-connection run of the fixture leaves behind: each
/// driver's items in order, and the two replicas for the checks.
struct Severed {
    a_items: Vec<Result<Gossiped, Error>>,
    b_items: Vec<Result<Gossiped, Error>>,
    a: Rumors<u64>,
    b: Rumors<u64>,
}

/// Run the severed-connection fixture: the [`pair`] with one send on each
/// side, A driven by a single tick and B by remote-led serving, over one
/// in-memory connection with `a_fault` and `b_fault` applied to the two
/// ends.
///
/// Each side's end is owned by its future, so the failing side's drop
/// surfaces as EOF to the other rather than deadlocking the join.
async fn run_severed(network: NetworkSeed, a_fault: FaultPlan, b_fault: FaultPlan) -> Severed {
    let (a, b) = pair(network).await;
    a.send(1).unwrap();
    b.send(2).unwrap();

    let (a_side, b_side) = connection();
    let a_end = faulty(a_side, a_fault);
    let b_end = faulty(b_side, b_fault);
    let a = with_policy_stream(a, stream::once(std::future::ready(())));
    let b = with_policy_stream(b, stream::pending::<()>());
    let a_task = async {
        let mut a_end = a_end;
        a.gossip(&mut a_end.read, &mut a_end.write)
            .collect::<Vec<_>>()
            .await
    };
    let b_task = async {
        let mut b_end = b_end;
        b.gossip(&mut b_end.read, &mut b_end.write)
            .collect::<Vec<_>>()
            .await
    };
    let (a_items, b_items) = timeout(DEADLINE, futures::future::join(a_task, b_task))
        .await
        .expect("a severed connection wedged a driver");
    Severed {
        a_items,
        b_items,
        a,
        b,
    }
}

/// The severed-connection contract, checked on one run's outcome:
/// terminal shape, atomicity, the epilogue's certification, and recovery
/// over a fresh clean connection.
async fn check_severed(run: &Severed) {
    let Severed {
        a_items,
        b_items,
        a,
        b,
    } = run;

    // Terminal shape: zero or more Ok items, then at most one Err.
    for items in [a_items, b_items] {
        if let Some(err_at) = items.iter().position(|i| i.is_err()) {
            assert_eq!(err_at, items.len() - 1, "Err must be terminal");
        }
    }

    // Atomicity: whatever happened, each side holds its own send,
    // nothing beyond the union, and never a torn intermediate.
    let (a_snapshot, b_snapshot) = (a.snapshot(), b.snapshot());
    assert!(a_snapshot.iter().any(|(_, m)| *m == 1));
    assert!(b_snapshot.iter().any(|(_, m)| *m == 2));
    assert!(a_snapshot.len() <= 2);
    assert!(b_snapshot.len() <= 2);

    // Certification: the epilogue's central promise, under a cut at
    // an arbitrary offset. A driver yields `Ok` only after reading
    // the peer's completion marker, which the peer writes only
    // after its own commit -- so any `Ok` on either side means both
    // replicas already hold the session's full union, here, before
    // the recovery gossip has run.
    if a_items.iter().any(|i| i.is_ok()) || b_items.iter().any(|i| i.is_ok()) {
        assert_eq!(
            a_snapshot.hash(),
            b_snapshot.hash(),
            "a session Ok certifies the peer committed: the pair must already be converged",
        );
        assert_eq!(
            a_snapshot.len(),
            2,
            "the certified session moved both sends"
        );
    }

    // Recovery: a fresh, clean connection converges the pair.
    wire_gossip_async(a, b).await;
    let (a_snapshot, b_snapshot) = (a.snapshot(), b.snapshot());
    assert_eq!(a_snapshot.hash(), b_snapshot.hash());
    assert_eq!(a_snapshot.len(), 2);
}

/// Byte extent of the severed-connection fixture on a clean run, per
/// endpoint: `(a_written, b_written, b_read)`.
///
/// The same pair, sends, drivers, and connection capacity as [`run_severed`],
/// metered with the counters the fault cuts spend. The in-memory connection and
/// the single-threaded driver make the run byte-identical across
/// invocations from one network seed, which lets [`a_lost_marker_certifies_one_side`] place a
/// cut one byte short of a clean session.
async fn fixture_session_bytes(network: NetworkSeed) -> (usize, usize, usize) {
    let (a, b) = pair(network).await;
    a.send(1).unwrap();
    b.send(2).unwrap();

    let (a_side, b_side) = connection();
    let (a_end, a_meter) = fault::metered(a_side);
    let (b_end, b_meter) = fault::metered(b_side);
    let a = with_policy_stream(a, stream::once(std::future::ready(())));
    let b = with_policy_stream(b, stream::pending::<()>());
    let a_task = async {
        let mut a_end = a_end;
        a.gossip(&mut a_end.read, &mut a_end.write)
            .collect::<Vec<_>>()
            .await
    };
    let b_task = async {
        let mut b_end = b_end;
        b.gossip(&mut b_end.read, &mut b_end.write)
            .collect::<Vec<_>>()
            .await
    };
    let (a_items, b_items) = timeout(DEADLINE, futures::future::join(a_task, b_task))
        .await
        .expect("the clean fixture session wedged a driver");
    for items in [&a_items, &b_items] {
        assert!(
            items.iter().all(|i| i.is_ok()),
            "the clean fixture session must end without error: {items:?}"
        );
    }
    assert_eq!(a.snapshot().hash(), b.snapshot().hash());
    (a_meter.written(), b_meter.written(), b_meter.read())
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// Pins `MAX_SEVER_CUT` to the fixture session's measured byte extent,
    /// from both sides.
    ///
    /// Every byte of the clean session is a reachable cut offset
    /// (`measured <= MAX_SEVER_CUT`), and the cut range is not vacuously wide
    /// (`MAX_SEVER_CUT <= 2 * measured`), so generated cuts keep landing
    /// inside the session rather than past its end.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn max_sever_cut_spans_the_fixture_session(network in network_seed()) {
        let (a_written, b_written, b_read) = block_on(fixture_session_bytes(network));
        println!("fixture session bytes: A wrote {a_written}, B wrote {b_written}, B read {b_read}");
        let measured = a_written.max(b_written);
        assert!(
            measured <= MAX_SEVER_CUT,
            "the fixture session moves {measured} bytes per endpoint, beyond \
             MAX_SEVER_CUT ({MAX_SEVER_CUT}): deep cut offsets are unreachable"
        );
        assert!(
            MAX_SEVER_CUT <= 2 * measured,
            "MAX_SEVER_CUT ({MAX_SEVER_CUT}) is more than twice the fixture \
             session's {measured} bytes: most generated cuts would land past the \
             end of the session and never fire"
        );
    }

    /// The certification's asymmetric case: A `Ok`, B `Err`, both replicas
    /// converged.
    ///
    /// B's read budget is one byte short of a clean session (measured from a
    /// byte-identical metered run), so the byte B never reads is A's
    /// completion marker: A yields `Ok`; B yields no `Ok` and fails with the
    /// failure in [`rumors::error::Phase::Completion`]. The class places the cut: one
    /// byte more and B completes the session, failing only on its next
    /// read after an `Ok`; any earlier and B fails pre-commit. A
    /// generated read cut reaches this case only by landing exactly here, so
    /// this deterministic witness holds the certification assertions to the
    /// case they exist for.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn a_lost_marker_certifies_one_side(network in network_seed()) {
        let (_, _, b_read) = block_on(fixture_session_bytes(network));
        let run = block_on(run_severed(
            network,
            FaultPlan::NONE,
            FaultPlan {
                write_cut: None,
                read_cut: Some(b_read - 1),
                vanish: None,
            },
        ));
        assert!(
            run.a_items.iter().any(|i| i.is_ok()),
            "A read B's marker and certifies the session: {:?}",
            run.a_items
        );
        assert!(
            run.b_items.iter().all(|i| i.is_err()),
            "B never read A's marker, so it certifies nothing: {:?}",
            run.b_items
        );
        assert!(
            matches!(run.b_items.last(), Some(Err(Error::Transport(error))) if error.context.phase == rumors::error::Phase::Completion),
            "B's lost marker must surface as the completion-phase transport failure: {:?}",
            run.b_items
        );
        block_on(check_severed(&run));
    }
}

proptest! {
    /// A connection severed at arbitrary byte offsets -- both sides' write
    /// directions, any budget including zero, and either side's read
    /// direction as well -- fails loudly and recoverably.
    ///
    /// That is: no hang, each driver ends after at most one terminal
    /// `Err`, each replica still holds its own sends and nothing beyond
    /// the union (a torn session commits all of a reconciliation or none
    /// of it), a session `Ok` from either driver certifies the pair had
    /// already converged before any recovery (the epilogue's certification,
    /// held under arbitrary cut geometry rather than only at pinned byte
    /// boundaries), and a fresh clean connection converges the pair fully.
    /// The one-`Ok`-one-`Err` case that certification exists for has its
    /// deterministic witness in [`a_lost_marker_certifies_one_side`].
    #[test]
    fn severed_connections_fail_loudly_and_recover(
        a_write_cut in 0..MAX_SEVER_CUT,
        b_write_cut in 0..MAX_SEVER_CUT,
        a_read_cut in prop::option::of(0..MAX_SEVER_CUT),
        b_read_cut in prop::option::of(0..MAX_SEVER_CUT),
        network in network_seed(),
    ) {
        block_on(async {
            let run = run_severed(
                network,
                FaultPlan {
                    write_cut: Some(a_write_cut),
                    read_cut: a_read_cut,
                    vanish: None,
                },
                FaultPlan {
                    write_cut: Some(b_write_cut),
                    read_cut: b_read_cut,
                    vanish: None,
                },
            )
            .await;
            check_severed(&run).await;
        });
    }
}

/// One step of a chaos script: a send or redaction on either side, a tick to
/// either driver, or an opportunity for the schedulers to make progress.
///
/// Clock ticks and commits are deliberately decoupled: a tick may arrive with
/// nothing new (must suppress), late (covering several commits), or while a
/// session is already in flight on the same or the opposite side.
#[derive(Debug, Clone, Copy)]
enum Op {
    /// Send a new message from A.
    SendA,
    /// Send a new message from B.
    SendB,
    /// Redact A's most recent unredacted local message, if any.
    RedactA,
    /// Redact B's most recent unredacted local message, if any.
    RedactB,
    /// Ask A's driver to gossip if its set changed.
    TickA,
    /// Ask B's driver to gossip if its set changed.
    TickB,
    /// Yield to the drivers this many times, letting in-flight sessions
    /// progress (or not) between script steps.
    Pump(u8),
}

/// Generate local edits, initiation requests, and opportunities for network progress.
fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        Just(Op::SendA),
        Just(Op::SendB),
        Just(Op::RedactA),
        Just(Op::RedactB),
        Just(Op::TickA),
        Just(Op::TickB),
        (1u8..6).prop_map(Op::Pump),
    ]
}

proptest! {
    /// Chaos: under *any* interleaving of commits, ticks, and scheduler
    /// progress on both sides of one connection, no session errors and
    /// full convergence on exactly the surviving messages.
    ///
    /// The interleavings include simultaneous initiations, ticks racing
    /// in-flight sessions, suppressed ticks, and idle pumps. Every
    /// driver ends cleanly once its tick source closes, and the pair
    /// converges on exactly the union of both sides' sends minus redactions.
    #[test]
    fn chaotic_tick_interleavings_converge_without_error(
        script in proptest::collection::vec(op_strategy(), 0..48),
        network in network_seed(),
    ) {
        block_on(async {
            let (a, b) = pair(network).await;
            let (mut a_end, mut b_end) = connection();
            let (a_tx, a_when) = ticks();
            let (b_tx, b_when) = ticks();

            let a = with_policy_stream(a, a_when);
            let a_sessions = a.gossip(&mut a_end.read, &mut a_end.write);
            let b = with_policy_stream(b, b_when);
            let b_sessions = b.gossip(&mut b_end.read, &mut b_end.write);

            // Collectors poll the drivers to completion; any session error
            // panics, which is the test's core assertion.
            let collect_a = a_sessions.for_each(|item| async move {
                item.expect("A session errored");
            });
            let collect_b = b_sessions.for_each(|item| async move {
                item.expect("B session errored");
            });

            let mut sent = 0u64;
            let mut redacted = 0u64;
            let mut a_live = Vec::new();
            let mut b_live = Vec::new();
            let feeder = async {
                for op in &script {
                    match op {
                        Op::SendA => {
                            a_live.push(a.send(sent).unwrap());
                            sent += 1;
                        }
                        Op::SendB => {
                            b_live.push(b.send(1_000_000 + sent).unwrap());
                            sent += 1;
                        }
                        Op::RedactA => {
                            if let Some(version) = a_live.pop() {
                                a.redact(&version);
                                redacted += 1;
                            }
                        }
                        Op::RedactB => {
                            if let Some(version) = b_live.pop() {
                                b.redact(&version);
                                redacted += 1;
                            }
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
            assert_eq!(
                a_snapshot.len() as u64,
                sent - redacted,
                "the union of all sends minus redactions"
            );
        });
    }

    /// A peer that dies mid-preamble is an error, not a goodbye: the driver
    /// yields one terminal `Err` and then ends.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn truncated_initiation_is_a_terminal_error(network in network_seed()) {
        block_on(async {
            let a: Rumors<u64> = network.peer().sync_memory_budget(0).into_rumors();
            let (mut a_end, b) = rumors::testing::memory_connection(ROOMY_CAPACITY);
            // Drive the counterparty by hand: keep its read half alive so A's own
            // writes succeed, while its write half toward A carries only a partial
            // preamble.
            let mut b_write = b.write;
            let _b_read = b.read;

            let a = with_policy_stream(a, stream::pending::<()>());
            let mut a_sessions = a.gossip(&mut a_end.read, &mut a_end.write);

            // Four bytes of a preamble, then hang up mid-preamble. Closing the
            // write half toward A signals end-of-stream on A's read half.
            b_write.write_all(b"RUMO").await.expect("partial write");
            drop(b_write);

            let item = timeout(DEADLINE, a_sessions.next())
                .await
                .expect("error never surfaced")
                .expect("driver yielded its terminal item");
            match item {
                Err(Error::Transport(error)) => {
                    assert_eq!(error.context.phase, rumors::error::Phase::Preamble);
                    assert_eq!(error.source.kind(), std::io::ErrorKind::UnexpectedEof);
                }
                other => panic!("expected preamble transport failure, got {other:?}"),
            }
            assert!(
                timeout(DEADLINE, a_sessions.next())
                    .await
                    .expect("end after error deadlocked")
                    .is_none(),
                "the stream must end after its terminal error"
            );
        });
    }

    /// A transport error on the idle boundary — the read fails before one
    /// preamble byte arrives — is the driver's terminal `Err`, after which the
    /// stream ends.
    ///
    /// Zero session bytes moved, so this is distinct from the hang-up between
    /// sessions that ends a driver cleanly: an error is never a goodbye. (The
    /// in-memory duplex can only end, so the read error comes from the fault
    /// harness with a zero-byte budget.)
    ///
    /// The claim holds for every network seed.
    #[test]
    fn a_read_error_on_the_idle_boundary_is_terminal(network in network_seed()) {
        block_on(async {
            let a: Rumors<u64> = network.peer().sync_memory_budget(0).into_rumors();
            let (a_end, _b_end) = connection();
            let mut a_end = faulty(
                a_end,
                FaultPlan {
                    write_cut: None,
                    read_cut: Some(0),
                    vanish: None,
                },
            );

            let a = with_policy_stream(a, stream::pending::<()>());
            {
                let mut a_sessions = a.gossip(&mut a_end.read, &mut a_end.write);
                let item = timeout(DEADLINE, a_sessions.next())
                    .await
                    .expect("error never surfaced")
                    .expect("driver yielded its terminal item");
                assert!(
                    matches!(item, Err(Error::Transport(_))),
                    "a read error is the driver's terminal Err, got {item:?}",
                );
                assert!(
                    timeout(DEADLINE, a_sessions.next())
                        .await
                        .expect("end after error deadlocked")
                        .is_none(),
                    "the stream must end after its terminal error"
                );
            }
        });
    }
}
