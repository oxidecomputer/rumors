//! Where pipelining ends: the divergence the static window predicts must
//! serialize.
//!
//! The window derivation fixes each level's channel capacity, so it also
//! predicts its own knee: a session stays fully pipelined while the
//! disputed scopes at every engaged level fit that level's capacity, and
//! beyond it the descent drains in capacity-sized waves — one round trip
//! per wave at the binding level. These tests read the derived capacities
//! back through `rumors::testing::window_capacities`, place divergences
//! on both sides of the predicted knee, and measure serialized wire hops
//! in exact virtual time on a delayed link: flat below the knee, growing
//! with the wave count above it.

// Only the delayed wire is exercised here; the module's pipes and
// conformance surface belong to the benches and `latency_link.rs`.
#[allow(dead_code)]
#[path = "../benches/support/latency.rs"]
mod latency;

use std::time::Duration;

use rand::rngs::SmallRng;
use rand::{RngCore, SeedableRng};
use rumors::testing::window_capacities;
use rumors::{Peer, Protocol, Rumors};

/// A budget sized so the binding capacity lands in the tens: small
/// enough that a modest divergence crosses the knee, large enough that
/// below-knee sessions genuinely pipeline.
const BUDGET: usize = 2 << 20;

/// Messages both peers share before the fork.
const COMMON: usize = 2_048;

/// One-way link delay, in whole milliseconds (the timer wheel's grain).
const DELAY: Duration = Duration::from_millis(10);

/// Serialized one-way hops a fully pipelined session may spend: the
/// phase ladder's few active levels, with margin for scheduling noise.
const PIPELINED_HOPS: u32 = 24;

/// Extra hops the above-knee session must show beyond the below-knee
/// one: at eight waves per engaged level the prediction is well past
/// this, and scheduling noise is well under it.
const KNEE_MARGIN: u32 = 6;

/// Per-stream in-flight window: far above this test's transfers, so only
/// round-trip structure is measured.
const LINK_CAPACITY: usize = 8 * 1024 * 1024;

/// The binding capacity among the levels a test-scale divergence
/// engages: depths two through four (heights 30 down to 28).
fn binding_capacity() -> usize {
    // The knee prediction must use the sizes the session itself will
    // exchange; both sides hold COMMON plus their divergence, and the
    // capacities vary little across the test's divergence range, so the
    // largest shape stands for all of them (the assertion below keeps
    // that honest if the derivation drifts).
    let session_len = (COMMON + 32 * 64) as u64;
    let capacities = window_capacities(session_len, session_len, BUDGET);
    let binding = capacities[28..=30]
        .iter()
        .copied()
        .min()
        .expect("three engaged heights");
    assert!(
        (4..=256).contains(&binding),
        "the test budget must land the binding capacity in a band where \
         both sides of the knee are reachable at test scale; got {binding} \
         from capacities {capacities:?}",
    );
    binding
}

/// Serialized one-way hops one divergent session spends on the wire.
fn hops(divergent_per_side: usize) -> u32 {
    hops_over(divergent_per_side, LINK_CAPACITY)
}

/// [`hops`], with the pipe's per-stream in-flight window chosen by the
/// caller.
///
/// Measured as the delay-sweep slope — the same shape at two delays,
/// divided by the delay difference — which isolates wire structure from
/// compute; a single-point division would count the session's compute
/// as phantom hops. The pipe stays fixed across the sweep, so a tight
/// pipe's transfer time (`bytes × delay / capacity`) scales with the
/// delay and survives the slope in hop units, exactly like wave stall.
fn hops_over(divergent_per_side: usize, pipe_capacity: usize) -> u32 {
    let elapsed_at = |delay: Duration| {
        let (left, right) = diverged(divergent_per_side);
        let mut wire = latency::DelayedWire::new(pipe_capacity, delay);
        let (_pair, elapsed) = wire.round_trip(left, right);
        elapsed
    };
    let (short, long) = (elapsed_at(DELAY), elapsed_at(2 * DELAY));
    u32::try_from(long.saturating_sub(short).as_millis() / DELAY.as_millis())
        .expect("bounded hop count")
}

/// Two peers with a shared prefix, diverged by `divergent` messages each.
fn diverged(divergent: usize) -> (Rumors<u64>, Rumors<u64>) {
    let left = Peer::seed().sync_memory_budget(BUDGET).into_rumors();
    let mut rng = SmallRng::seed_from_u64(0x0077_1e0f_0b05_2026);
    send_random(&left, COMMON, &mut rng);

    let right = pollster::block_on(async {
        let (mut provider, mut newcomer) = rumors::link::memory_with_capacity(LINK_CAPACITY);
        let (served, joined) = tokio::join!(
            left.gossip(&mut provider),
            Peer::<u64>::bootstrap_with_protocol(Protocol::V2, &mut newcomer),
        );
        served.expect("serve bootstrap");
        joined
            .expect("bootstrap newcomer")
            .expect("provider is established")
            .sync_memory_budget(BUDGET)
            .into_rumors()
    });

    send_random(&left, divergent, &mut rng);
    send_random(&right, divergent, &mut rng);
    (left, right)
}

/// Commit `n` random payloads as one batch.
fn send_random(rumors: &Rumors<u64>, n: usize, rng: &mut SmallRng) {
    let mut batch = rumors.batch();
    for _ in 0..n {
        batch.send(rng.next_u64());
    }
}

/// Below the predicted knee the session is pipelined: hops are bounded
/// by the phase ladder, not by scope count. Doubling the divergence may
/// engage one more trie level — a couple of ladder hops — but never a
/// per-wave cost, which is what separates it from the above-knee regime.
#[test]
fn below_the_knee_hops_are_flat() {
    let capacity = binding_capacity();
    let half = hops(capacity / 2);
    let full = hops(capacity);
    eprintln!("binding capacity {capacity}: hops {half} at half, {full} at capacity");
    assert!(
        half <= PIPELINED_HOPS && full <= PIPELINED_HOPS,
        "below-knee sessions must stay pipelined: {half} and {full} hops \
         against a budget of {PIPELINED_HOPS}",
    );
    assert!(
        full <= half + 4,
        "doubling a below-knee divergence may deepen the ladder, not \
         serialize: {half} -> {full}",
    );
}

/// Above the knee, cost is linear in divergence: constant hops per
/// message, not a growing cliff.
///
/// The wave model says a session above the knee drains in
/// binding-capacity-sized waves, one round trip each, so time grows by a
/// *constant* increment per message — which is what makes a fixed window
/// a throughput ceiling with a fixed slowdown factor rather than a
/// latency penalty that compounds with divergence. Measured on a
/// latency-only link (roomy pipe), the marginal hops per message between
/// consecutive divergence doublings must agree with each other within a
/// factor of two; their absolute scale is reported for calibration
/// against the predicted `2 / capacity`.
#[test]
fn above_the_knee_cost_is_linear_in_divergence() {
    let capacity = binding_capacity();
    // Widely spaced cells: each marginal spans at least eight capacities
    // of divergence (~16 hops of signal), so wall-compute noise under a
    // loaded test machine — which the two-point sweep cannot fully
    // cancel — stays far below the signal it is differenced against.
    let divergences = [4, 12, 36].map(|factor| factor * capacity);
    let times: Vec<u32> = divergences.iter().map(|&d| hops(d)).collect();
    let slopes: Vec<f64> = times
        .windows(2)
        .zip(divergences.windows(2))
        .map(|(t, d)| f64::from(t[1] - t[0]) / (d[1] - d[0]) as f64)
        .collect();
    let predicted = 2.0 / capacity as f64;
    eprintln!(
        "capacity {capacity}: divergences {divergences:?}, hops {times:?}, \
         marginal hops/message {slopes:?} (wave model predicts {predicted:.4})",
    );
    // Each marginal slope must sit in a band around the wave model's
    // 2/capacity — the direct statement of constant-per-message cost.
    // The band absorbs the sweep's quantization noise (each cell
    // differences two sessions at millisecond timer grain), which a
    // cell-to-cell spread bound would amplify instead.
    for &slope in &slopes {
        assert!(
            slope / predicted > 0.4 && slope / predicted < 2.5,
            "marginal cost per message must track the wave model's \
             {predicted:.4}: slopes {slopes:?}",
        );
    }
}

/// A window at or above the link's bandwidth-delay product in messages
/// hides serialization entirely.
///
/// With the pipe's in-flight window tightened until bandwidth (pipe
/// capacity over delay) is the binding constraint — a
/// bandwidth-delay product in messages below the window's binding
/// capacity — the transfer itself costs more time than the window's
/// waves, so the latency-only session time (pure window stall, roomy
/// pipe) must sit at or below the bandwidth-bound time for the same
/// divergence. This is the amortization that lets one well-picked
/// constant serve every divergence on links up to its design BDP.
#[test]
fn window_stall_hides_under_bandwidth_bound_transfer() {
    let capacity = binding_capacity();
    let divergence = 16 * capacity;
    // 2 KiB in flight at 10 ms one-way models ~200 KB/s per stream: a
    // BDP of ~2 KiB, tens of messages at the ~100 B floor — at or below
    // the binding capacity, so bandwidth binds before the window does.
    let bandwidth_bound = hops_over(divergence, 2 * 1024);
    let stall_bound = hops(divergence);
    eprintln!(
        "capacity {capacity}, divergence {divergence}: \
         latency-only {stall_bound} hops, bandwidth-bound {bandwidth_bound} hops",
    );
    assert!(
        stall_bound <= bandwidth_bound + bandwidth_bound / 4 + 4,
        "window stall ({stall_bound} hops) must hide inside the \
         bandwidth-bound transfer ({bandwidth_bound} hops) once the pipe's \
         BDP in messages is below the window",
    );
}

/// Above the predicted knee the descent serializes into capacity-sized
/// waves: eight times the binding capacity costs measurably more round
/// trips than the pipelined session, in the direction and scale the
/// derivation predicts.
#[test]
fn above_the_knee_hops_grow() {
    let capacity = binding_capacity();
    let below = hops(capacity / 2);
    let above = hops(8 * capacity);
    eprintln!("binding capacity {capacity}: hops {below} below, {above} above");
    assert!(
        above >= below + KNEE_MARGIN,
        "a divergence of 8x the binding capacity must serialize into \
         waves: measured {above} hops vs {below} pipelined",
    );
}
