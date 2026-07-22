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

/// The declared set size: far above test scale, so capacities are
/// budget-bound and the knee is the budget's, not the data's.
const DECLARED_MESSAGES: u64 = 10_000_000_000;

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
    let capacities = window_capacities(DECLARED_MESSAGES, BUDGET);
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
    let (left, right) = diverged(divergent_per_side);
    let mut wire = latency::DelayedWire::new(LINK_CAPACITY, DELAY);
    let (_pair, elapsed) = wire.round_trip(left, right);
    u32::try_from(elapsed.as_millis() / DELAY.as_millis()).expect("bounded hop count")
}

/// Two peers with a shared prefix, diverged by `divergent` messages each.
fn diverged(divergent: usize) -> (Rumors<u64>, Rumors<u64>) {
    let left = Peer::seed()
        .sync_memory_budget(DECLARED_MESSAGES, BUDGET)
        .into_rumors();
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
            .sync_memory_budget(DECLARED_MESSAGES, BUDGET)
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
