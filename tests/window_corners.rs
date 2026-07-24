//! Boundary honesty of the sync-budget interface.
//!
//! `sync_memory_budget` promises degradation to latency only — never
//! deadlock, never memory growth — and promises that the pair-based
//! window prices sessions by their *disputes*, not their sizes. These
//! tests probe the corners where those promises are easiest to break:
//! asymmetric catch-up, the zero-budget floor, compounded backpressure
//! (one-byte link windows under floor dispute windows), sets that grow
//! while a session runs, and sessions timed on a real clock rather than
//! the paused one.

// Only the delayed wire is exercised here; the module's pipes and
// conformance surface belong to the benches and `latency_link.rs`.
#[allow(dead_code)]
#[path = "../benches/support/latency.rs"]
mod latency;

use std::time::Duration;

use rand::rngs::SmallRng;
use rand::{RngCore, SeedableRng};
use rumors::testing::run_to_quiescence;
use rumors::{Peer, Protocol, Rumors};

/// One-way delay for the virtual-time measurements (the timer grain).
const DELAY: Duration = Duration::from_millis(10);

/// Roomy per-stream pipe buffering: only round-trip structure is measured.
const LINK_CAPACITY: usize = 8 * 1024 * 1024;

/// Build a bootstrapped pair, then commit `left_extra`/`right_extra`
/// further messages on the respective sides, all under `budget`.
fn pair(
    budget: usize,
    common: usize,
    left_extra: usize,
    right_extra: usize,
) -> (Rumors<u64>, Rumors<u64>) {
    let left = Peer::seed().sync_memory_budget(budget).into_rumors();
    let mut rng = SmallRng::seed_from_u64(0x0b05_2026_c07e_0003);
    send_random(&left, common.max(1), &mut rng);

    let right = pollster::block_on(async {
        let (mut provider, mut newcomer) = rumors::link::memory_with_capacity(LINK_CAPACITY);
        let (served, joined) = tokio::join!(
            left.gossip(&mut provider),
            Peer::<u64>::bootstrap()
                .protocol(Protocol::V2)
                .join(&mut newcomer),
        );
        served.expect("serve bootstrap");
        joined
            .expect("bootstrap newcomer")
            .expect("provider is established")
            .sync_memory_budget(budget)
            .into_rumors()
    });

    send_random(&left, left_extra, &mut rng);
    send_random(&right, right_extra, &mut rng);
    (left, right)
}

/// Commit `n` random payloads as one batch.
fn send_random(rumors: &Rumors<u64>, n: usize, rng: &mut SmallRng) {
    if n == 0 {
        return;
    }
    let mut batch = rumors.batch();
    for _ in 0..n {
        batch.send(rng.next_u64());
    }
}

/// Serialized one-way hops one session spends on the wire, by the
/// delay-sweep slope: the same shape runs at two delays, and the time
/// difference divided by the delay difference isolates wire structure
/// from compute (the harness reports their sum, and a session that
/// moves tens of thousands of messages spends real milliseconds
/// computing — dividing a single point by the delay would count that
/// compute as phantom hops).
fn hops(shape: impl Fn() -> (Rumors<u64>, Rumors<u64>)) -> u32 {
    let elapsed_at = |delay: Duration| {
        let (left, right) = shape();
        let mut wire = latency::DelayedWire::new(LINK_CAPACITY, delay);
        let (_pair, elapsed) = wire.round_trip(left, right);
        elapsed
    };
    let (short, long) = (elapsed_at(DELAY), elapsed_at(2 * DELAY));
    u32::try_from(long.saturating_sub(short).as_millis() / DELAY.as_millis())
        .expect("bounded hop count")
}

/// Asymmetric catch-up is priced by its disputes, not its size: a nearly
/// empty replica pulling twenty thousand messages completes in ladder
/// hops even at the zero-budget floor, because the pair of exchanged
/// sizes derives floor dispute windows *and* the session genuinely has
/// almost nothing to dispute — the transfer is supply, which streams
/// outside the window. This is the honesty of the pair-based choice: if
/// catch-up ever became wave-bound, the smaller-corpus bound would be
/// mispricing real sessions.
#[test]
fn asymmetric_catch_up_is_ladder_bound_at_the_floor() {
    let measured = hops(|| pair(0, 1, 20_000, 0));
    eprintln!("asymmetric catch-up at budget 0: {measured} hops");
    // Ladder hops: the dispute chain prunes within a few levels (the one
    // shared key's subtree thins to exactly that leaf and matches), and
    // the supply is one unidirectional stream. A size-priced session
    // would cost ~2 hops per message — three orders of magnitude past
    // this bound.
    assert!(
        measured <= 24,
        "a one-common-message catch-up must cost ladder hops, not waves: \
         {measured} hops",
    );
}

/// The same catch-up serves in the other direction: the large side
/// initiating against the small side derives the same pair, prices the
/// same near-nothing, and stays ladder-bound.
#[test]
fn asymmetric_catch_up_is_direction_independent() {
    let measured = hops(|| pair(0, 1, 0, 20_000));
    eprintln!("reverse asymmetric catch-up at budget 0: {measured} hops");
    assert!(
        measured <= 24,
        "catch-up direction must not change the dispute price: {measured} hops",
    );
}

/// The zero-budget floor keeps its promise at wide mutual divergence:
/// the session completes and converges — slowly, in waves, which the hop
/// count must actually show (a fast floor would mean the capacities are
/// not being applied), and boundedly (linear in the divergence, nothing
/// worse).
#[test]
fn zero_budget_serializes_but_completes() {
    let divergence = 2_000;
    let measured = hops(|| pair(0, 2_048, divergence, divergence));
    let (left, right) = pair(0, 2_048, divergence, divergence);
    eprintln!("zero budget at {divergence} mutual divergence: {measured} hops");
    assert!(
        measured > 64,
        "the floor must genuinely serialize a wide divergence: {measured} hops",
    );
    assert!(
        measured <= 8 * 2 * divergence as u32,
        "floor cost must stay linear in divergence: {measured} hops",
    );
    let mut wire = latency::DelayedWire::new(LINK_CAPACITY, DELAY);
    let ((left, right), _elapsed) = wire.round_trip(left, right);
    assert_eq!(
        left.snapshot().len(),
        right.snapshot().len(),
        "the serialized session still converges",
    );
}

/// Compounded backpressure cannot deadlock: one-byte link windows under
/// zero-budget dispute windows — every buffer in the system at its
/// minimum — still complete, witnessed deterministically (a `Pending`
/// with no wake arranged would fail the run, not hang it).
#[test]
fn one_byte_pipes_at_the_floor_stay_live() {
    let (left, right) = pair(0, 64, 64, 64);
    run_to_quiescence(async {
        let (mut a, mut b) = rumors::link::memory_with_capacity(1);
        let (left_result, right_result) = tokio::join!(left.gossip(&mut a), right.gossip(&mut b));
        left_result.expect("gossip left");
        right_result.expect("gossip right");
    })
    .expect("every buffer at its floor stays live");
}

/// A set that grows mid-session cannot break the session it grows under.
///
/// The window derives from the sizes exchanged at the greeting; commits
/// racing the session make those sizes stale in the direction of more
/// population, which may only serialize. The racing session completes,
/// and the next session converges whatever it missed.
#[test]
fn growth_during_a_session_only_serializes() {
    let (left, right) = pair(64 * 1024, 2_048, 2_000, 2_000);
    let racer = left.clone();
    pollster::block_on(async {
        let (mut a, mut b) = rumors::link::memory_with_capacity(LINK_CAPACITY);
        let mut rng = SmallRng::seed_from_u64(0x0b05_2026_c07e_0004);
        let race = async {
            for _ in 0..64 {
                let mut batch = racer.batch();
                for _ in 0..32 {
                    batch.send(rng.next_u64());
                }
                drop(batch);
                tokio::task::yield_now().await;
            }
        };
        let (left_result, right_result, ()) =
            tokio::join!(left.gossip(&mut a), right.gossip(&mut b), race);
        left_result.expect("gossip left under concurrent growth");
        right_result.expect("gossip right");

        let (mut a, mut b) = rumors::link::memory_with_capacity(LINK_CAPACITY);
        let (left_result, right_result) = tokio::join!(left.gossip(&mut a), right.gossip(&mut b));
        left_result.expect("follow-up gossip left");
        right_result.expect("follow-up gossip right");
    });
    assert_eq!(
        left.snapshot().len(),
        right.snapshot().len(),
        "the follow-up session converges everything the race added",
    );
}

/// The honesty claims survive a real clock. On genuinely elapsing timers,
/// an asymmetric catch-up stays fast (ladder hops of real milliseconds),
/// and a serialized session's waves burn real time in at least the
/// quantity the virtual model predicts — the model may overcharge,
/// never undercharge.
#[test]
fn real_clock_corners_match_the_virtual_model() {
    let delay = Duration::from_millis(2);

    // Catch-up: ladder-bound, so a real link finishes promptly.
    let (left, right) = pair(0, 1, 10_000, 0);
    let mut wire = latency::DelayedWire::new_wall_clock(LINK_CAPACITY, delay);
    let start = std::time::Instant::now();
    let _ = wire.round_trip(left, right);
    let catch_up = start.elapsed();
    eprintln!("real-clock catch-up: {catch_up:?}");
    assert!(
        catch_up < Duration::from_secs(5),
        "a ladder-bound catch-up must stay prompt on a real link: {catch_up:?}",
    );

    // Serialized divergence: the waves must actually cost wall time.
    let divergence = 512;
    let (left, right) = pair(0, 2_048, divergence, divergence);
    let mut wire = latency::DelayedWire::new_wall_clock(LINK_CAPACITY, delay);
    let start = std::time::Instant::now();
    let _ = wire.round_trip(left, right);
    let serialized = start.elapsed();
    eprintln!("real-clock serialized floor session: {serialized:?}");
    assert!(
        serialized >= 64 * delay,
        "a floor session over {divergence} disputes must pay real wave \
         time: {serialized:?}",
    );
}
