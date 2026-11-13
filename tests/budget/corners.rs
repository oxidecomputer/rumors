//! The sync budget's boundary cases.
//!
//! `sync_memory_budget` promises that a smaller budget costs a session time,
//! never completion. The liveness argument behind that promise uses no budget
//! at all: it rests on admission granting a unit whose kind and depth hold
//! nothing. In the serial configuration, a ledger budget of zero with every
//! kind and depth's bound zero, that is the only grant there is, so a session
//! admits one unit of each kind and depth at a time, and still finishes and
//! converges. The public setter keeps every session far above that, at
//! [`MIN_SYNC_MEMORY_BUDGET`](rumors::testing::MIN_SYNC_MEMORY_BUDGET), so these tests
//! set the serial configuration through the test path
//! ([`BudgetChoice::Serial`]) and probe the corners where the argument is
//! easiest to break: asymmetric catch-up and wide mutual divergence, compounded
//! backpressure (one-byte pipe buffers), sets that grow while a session runs,
//! and sessions timed on a real clock rather than the paused one.
//!
//! A serial test asserts that its session completes and converges, and prints
//! its hop count; it asserts nothing about speed.

use std::time::Duration;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use rumors::testing::run_to_quiescence;
use rumors::{Peer, Rumors};
use rumors_testkit::bench::latency;
use rumors_testkit::common::budget::BudgetChoice;
use rumors_testkit::common::property::FLEET_DIVISOR;
use rumors_testkit::common::seed::{NetworkSeed, network_seed};
use rumors_testkit::divided_proptest;

/// One-way delay for the virtual-time measurements (the timer grain).
const DELAY: Duration = Duration::from_millis(10);

/// Roomy per-direction pipe buffering: only round-trip structure is measured.
const LINK_CAPACITY: usize = 8 * 1024 * 1024;

/// Messages both peers share in the two-sided fixtures below.
const COMMON: usize = 2_048;

/// Messages each peer adds alone in the two-sided fixtures below.
const EXTRA: usize = 2_000;

/// The divisor of the case count of a property here that reconciles thousands
/// of messages: one case costs seconds in a debug build, so the default count
/// runs one case.
const CORNER_DIVISOR: u32 = 256;

/// Build a bootstrapped pair of the network `network` names, then commit
/// `left_extra`/`right_extra` further messages on the respective sides, both
/// under `budget`.
fn pair(
    network: NetworkSeed,
    budget: BudgetChoice,
    common: usize,
    left_extra: usize,
    right_extra: usize,
) -> (Rumors<u64>, Rumors<u64>) {
    let left = budget.apply(network.peer()).into_rumors();
    // Fixed, so every run reconciles the same corpus.
    let mut rng = ChaChaRng::seed_from_u64(0x0b05_2026_c07e_0003);
    send_random(&left, common.max(1), &mut rng);

    let right = pollster::block_on(async {
        let (mut provider, mut newcomer) = rumors::testing::memory_connection(LINK_CAPACITY);
        let (served, joined) = tokio::join!(
            left.gossip_once(&mut provider.read, &mut provider.write),
            Peer::<u64>::bootstrap().join(&mut newcomer.read, &mut newcomer.write),
        );
        served.expect("serve bootstrap");
        budget
            .apply(match joined {
                rumors::Joined::Joined {
                    peer,
                    connection: rumors::Connection::Reusable { .. },
                } => peer,
                _ => panic!("provider is established"),
            })
            .into_rumors()
    });

    send_random(&left, left_extra, &mut rng);
    send_random(&right, right_extra, &mut rng);
    (left, right)
}

/// Commit `n` random payloads as one batch.
fn send_random(rumors: &Rumors<u64>, n: usize, rng: &mut ChaChaRng) {
    rumors.send_all((0..n).map(|_| rng.next_u64())).unwrap();
}

/// Reconciles `pair` once over the delayed link on a paused clock, returning
/// the reconciled pair and the one-way hops the session spent.
///
/// Compute costs no virtual time on the paused clock, so the hop count is a
/// deterministic function of the session's shape.
fn reconcile(pair: (Rumors<u64>, Rumors<u64>)) -> ((Rumors<u64>, Rumors<u64>), u32) {
    let mut wire = latency::DelayedWire::new(LINK_CAPACITY, DELAY);
    let (pair, elapsed) = wire.round_trip_virtual(pair.0, pair.1);
    (pair, latency::hops_on_lattice(elapsed, DELAY))
}

/// Asserts that one session over `pair` leaves both sides holding the same
/// messages, printing the hops it spent under `label`.
fn assert_converges(label: &str, pair: (Rumors<u64>, Rumors<u64>)) {
    let ((left, right), hops) = reconcile(pair);
    eprintln!("{label}: {hops} hops");
    assert_eq!(
        left.snapshot(),
        right.snapshot(),
        "{label}: one session converges the pair",
    );
}

divided_proptest! {
    #![divisor(CORNER_DIVISOR)]

    /// A nearly empty replica catches up on twenty thousand messages in the
    /// serial configuration: the session completes and both sides then hold
    /// the same messages.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn asymmetric_catch_up_converges_serially(network in network_seed()) {
        assert_converges(
            "serial asymmetric catch-up",
            pair(network, BudgetChoice::Serial, 1, 20_000, 0),
        );
    }

    /// The same catch-up converges when the large side is the initiator.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn reverse_asymmetric_catch_up_converges_serially(network in network_seed()) {
        assert_converges(
            "serial reverse asymmetric catch-up",
            pair(network, BudgetChoice::Serial, 1, 0, 20_000),
        );
    }

    /// Two replicas sharing 2,048 messages, each holding 2,000 the other lacks,
    /// converge in one serial session.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn wide_divergence_converges_serially(network in network_seed()) {
        assert_converges(
            "2,000 messages a side of divergence, serially",
            pair(network, BudgetChoice::Serial, 2_048, 2_000, 2_000),
        );
    }
}

divided_proptest! {
    #![divisor(FLEET_DIVISOR)]

    /// Compounded backpressure cannot deadlock: one-byte pipe buffers with
    /// both sides serial, every buffer in the system at its minimum, still
    /// complete.
    ///
    /// Witnessed deterministically: a `Pending`
    /// with no wake arranged would fail the run, not hang it.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn one_byte_pipes_stay_live_serially(network in network_seed()) {
        let (left, right) = pair(network, BudgetChoice::Serial, 64, 64, 64);
        run_to_quiescence(async {
            let (mut a, mut b) = rumors::testing::memory_connection(1);
            let (left_result, right_result) = tokio::join!(
                left.gossip_once(&mut a.read, &mut a.write),
                right.gossip_once(&mut b.read, &mut b.write)
            );
            left_result.expect("gossip left");
            right_result.expect("gossip right");
        })
        .expect("every buffer at its floor stays live");
    }
}

divided_proptest! {
    #![divisor(CORNER_DIVISOR)]

    /// A set that grows mid-session cannot break the session it grows under.
    ///
    /// Commits racing the session make the sizes its greeting exchanged stale.
    /// The racing session completes, at least one racing commit is witnessed
    /// to have landed after the greeting's snapshot (the left replica has
    /// grown past what the session gave the right one), and the next session
    /// converges whatever it missed.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn growth_during_a_session_completes_and_the_next_converges(network in network_seed()) {
        let (left, right) = pair(network, BudgetChoice::Default, COMMON, EXTRA, EXTRA);
        let racer = left.clone();
        pollster::block_on(async {
            let (mut a, mut b) = rumors::testing::memory_connection(LINK_CAPACITY);
            // Fixed, so every run races the same commits.
            let mut rng = ChaChaRng::seed_from_u64(0x0b05_2026_c07e_0004);
            let race = async {
                for _ in 0..64 {
                    racer.send_all((0..32).map(|_| rng.next_u64())).unwrap();
                    tokio::task::yield_now().await;
                }
            };
            let (left_result, right_result, ()) = tokio::join!(
                left.gossip_once(&mut a.read, &mut a.write),
                right.gossip_once(&mut b.read, &mut b.write),
                race
            );
            left_result.expect("gossip left under concurrent growth");
            right_result.expect("gossip right");
            // The mid-session-growth witness: a commit that landed after the
            // greeting's snapshot is on the left replica and was never
            // offered to the right one, so the left has grown past the right.
            assert!(
                left.snapshot().len() > right.snapshot().len(),
                "no racing commit landed after the greeting: the left replica holds {} \
                 messages and the right {}, so the session raced nothing",
                left.snapshot().len(),
                right.snapshot().len(),
            );

            let (mut a, mut b) = rumors::testing::memory_connection(LINK_CAPACITY);
            let (left_result, right_result) = tokio::join!(
                left.gossip_once(&mut a.read, &mut a.write),
                right.gossip_once(&mut b.read, &mut b.write)
            );
            left_result.expect("follow-up gossip left");
            right_result.expect("follow-up gossip right");
        });
        assert_eq!(
            left.snapshot(),
            right.snapshot(),
            "the follow-up session converges everything the race added",
        );
    }

    /// Sessions on a real clock pay real delay and stay live.
    ///
    /// A catch-up depends on a reply to a delivered message, so at least two
    /// causally chained one-way hops of real delay elapse before it completes.
    /// A wide serial divergence, with every pipe delay burning wall
    /// time, completes and converges as on the paused clock. Only floors are
    /// asserted on elapsed time, because machine load inflates real elapsed
    /// time and can never compress it, so the assertions read the same under
    /// any suite parallelism.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn real_clock_sessions_pay_real_delay_and_stay_live(network in network_seed()) {
        let delay = Duration::from_millis(2);

        let (left, right) = pair(network, BudgetChoice::Serial, 1, 10_000, 0);
        let mut wire = latency::DelayedWire::new_wall_clock(LINK_CAPACITY, delay);
        let start = std::time::Instant::now();
        let _ = wire.round_trip(left, right);
        let catch_up = start.elapsed();
        eprintln!("real-clock catch-up: {catch_up:?}");
        assert!(
            catch_up >= 2 * delay,
            "a catch-up session must pay the wire's real round trip: {catch_up:?}",
        );

        let divergence = 512;
        let (left, right) = pair(network, BudgetChoice::Serial, 2_048, divergence, divergence);
        let mut wire = latency::DelayedWire::new_wall_clock(LINK_CAPACITY, delay);
        let ((left, right), elapsed) = wire.round_trip(left, right);
        eprintln!("serial real-clock session over {divergence} messages a side: {elapsed:?}");
        assert_eq!(
            left.snapshot(),
            right.snapshot(),
            "a serial session on a real clock converges",
        );
    }
}
