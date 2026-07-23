//! The pipeline window keeps wire stalls off the per-scope round-trip path.
//!
//! With one-slot channels, the streaming descent pays one wire round trip per
//! disputed scope, so a session's wire-stall time scales with divergence
//! instead of tree depth. The window (set through
//! [`Peer::sync_memory_budget`]) is the fix, and this test asserts it
//! end-to-end — from the public knob, through both protocol implementations, to
//! the channels — by gossiping over a delayed-pipe link on a paused-clock
//! runtime, where wire stalls are measured in exact virtual time.

// Only the delayed wire is exercised here; the module's pipes and
// conformance surface belong to the benches and `latency_link.rs`.
#[allow(dead_code)]
#[path = "../benches/support/latency.rs"]
mod latency;

use std::time::Duration;

use rand::rngs::SmallRng;
use rand::{RngCore, SeedableRng};
use rumors::{DEFAULT_SYNC_MEMORY_BUDGET, Peer, Protocol, Rumors};

/// Messages both peers share before the fork.
const COMMON: usize = 2_048;

/// Messages each peer originates after the fork: enough divergence to
/// dispute nearly all 256 root-child scopes.
const DIVERGENT_PER_SIDE: usize = 512;

/// One-way link delay, in whole milliseconds (the timer wheel's grain).
const DELAY: Duration = Duration::from_millis(10);

/// Serialized one-way hops a pipelined session may spend, with margin.
///
/// A pipelined descent costs a handful of hops (~7 measured; the phase
/// ladder's few active levels). The pre-window behavior paid one round
/// trip per disputed scope — here ≥ ~250 scopes, hence ≥ 2.8 s of stall —
/// so the bound separates the regimes by well over 2× in both directions.
const HOP_BUDGET: u32 = 64;

/// Per-stream in-flight window: far above this session's transfers, so
/// only round-trip structure is measured (see the module docs).
const LINK_CAPACITY: usize = 8 * 1024 * 1024;

/// With the production window, a divergent session's wire-stall time is
/// bounded by tree depth times the round trip — not by the number of
/// disputed scopes, which is an order of magnitude larger here.
#[test]
fn window_pipelines_disputed_scopes() {
    // The delay-sweep slope isolates wire hops from compute: the same
    // shape runs at two delays, and their difference is pure wire
    // structure.
    let elapsed_at = |delay: Duration| {
        let (left, right) = diverged_pair();
        let mut wire = latency::DelayedWire::new(LINK_CAPACITY, delay);
        let (_pair, elapsed) = wire.round_trip(left, right);
        elapsed
    };
    let (short, long) = (elapsed_at(DELAY), elapsed_at(2 * DELAY));
    let wire_cost = long.saturating_sub(short);

    let budget = DELAY * HOP_BUDGET;
    assert!(
        wire_cost < budget,
        "session wire cost {wire_cost:?} exceeds the pipelined budget {budget:?}: \
         the descent is serializing per disputed scope again"
    );
}

/// Two production-window peers with a shared prefix and heavy divergence.
fn diverged_pair() -> (Rumors<u64>, Rumors<u64>) {
    let left = Peer::seed()
        .sync_memory_budget(DEFAULT_SYNC_MEMORY_BUDGET)
        .into_rumors();
    let mut rng = SmallRng::seed_from_u64(0x9e37_79b9_7f4a_7c15);
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
            .sync_memory_budget(DEFAULT_SYNC_MEMORY_BUDGET)
            .into_rumors()
    });

    send_random(&left, DIVERGENT_PER_SIDE, &mut rng);
    send_random(&right, DIVERGENT_PER_SIDE, &mut rng);
    (left, right)
}

/// Commit `n` random payloads as one batch.
fn send_random(rumors: &Rumors<u64>, n: usize, rng: &mut SmallRng) {
    let mut batch = rumors.batch();
    for _ in 0..n {
        batch.send(rng.next_u64());
    }
}
