//! The operator equations held against measured sessions.
//!
//! `sync_memory_budget`'s docs publish two closed forms over a link's
//! bandwidth-delay product, with `ratio = envelope / wire = 22` and
//! `fans` the flat supply-decode envelope that comes off every budget
//! before the dispute solve: worst-case slowdown under a budget,
//! `max(1, 22 × BDP / (budget − fans))`, and the smallest budget for an
//! acceptable slowdown, `fans + 22 × BDP / slowdown`. They are the
//! large-window simplification of the exact wave-model form
//! `slowdown = max(1, BDP_messages / K)` with `K` the derived window —
//! the simplification substitutes `K ≈ (budget − fans) / envelope`,
//! which holds once the window is past the near-root structural band (a
//! few hundred scopes; small budgets pay full-fan prices the scalar
//! undercounts).
//!
//! Two pins split the claim at that seam:
//!
//! - the *exact* form is held against sessions on a genuinely
//!   bandwidth-limited pipe, with the link rate self-calibrated from the
//!   unbounded-budget transfer (the pipe carries several concurrent
//!   streams, so its effective rate is measured, not assumed);
//! - the *scalar* form is held by identity at the design point, where
//!   the default budget is its own inverse: `22 × BDP / 1` at the design
//!   link is exactly `DEFAULT_SYNC_MEMORY_BUDGET`.

// Only the delayed wire is exercised here; the module's pipes and
// conformance surface belong to the benches and `latency_link.rs`.
#[allow(dead_code)]
#[path = "../benches/support/latency.rs"]
mod latency;

use std::time::Duration;

use rand::rngs::SmallRng;
use rand::{RngCore, SeedableRng};
use rumors::testing::{envelope_and_wire_bytes, supply_decode_envelope_bytes, window_capacities};
use rumors::{DEFAULT_SYNC_MEMORY_BUDGET, Peer, Protocol, Rumors};

/// One-way delay for the virtual-time measurements (the timer grain).
const DELAY: Duration = Duration::from_millis(10);

/// The measured pipe's per-stream in-flight byte limit: tight, so
/// transfer time is real and the link rate genuinely binds.
const TIGHT_PIPE: usize = 25 * 1024;

/// Roomy buffering for corpus construction only.
const BUILD_CAPACITY: usize = 8 * 1024 * 1024;

/// Messages both peers share before the fork.
const COMMON: usize = 2_048;

/// Messages each side originates alone: deep enough above the
/// constricted knees that wave structure dominates ladder constants.
const DIVERGENT: usize = 8_192;

/// An effectively unbounded budget: the transfer-bound baseline.
const UNBOUNDED: usize = 8 << 30;

/// Two peers sharing a bootstrap, then diverged by [`DIVERGENT`] random
/// messages on each side, both configured with `budget`.
fn diverged(budget: usize) -> (Rumors<u64>, Rumors<u64>) {
    let left = Peer::seed().sync_memory_budget(budget).into_rumors();
    let mut rng = SmallRng::seed_from_u64(0x0b05_2026_09e7_a707);
    send_random(&left, COMMON, &mut rng);

    let right = pollster::block_on(async {
        let (mut provider, mut newcomer) = rumors::link::memory_with_capacity(BUILD_CAPACITY);
        let (served, joined) = tokio::join!(
            left.gossip(&mut provider),
            Peer::<u64>::bootstrap_with_protocol(Protocol::V2, &mut newcomer),
        );
        served.expect("serve bootstrap");
        joined
            .expect("bootstrap newcomer")
            .expect("provider is established")
            .sync_memory_budget(budget)
            .into_rumors()
    });

    send_random(&left, DIVERGENT, &mut rng);
    send_random(&right, DIVERGENT, &mut rng);
    (left, right)
}

/// Commit `n` random payloads as one batch.
fn send_random(rumors: &Rumors<u64>, n: usize, rng: &mut SmallRng) {
    let mut batch = rumors.batch();
    for _ in 0..n {
        batch.send(rng.next_u64());
    }
}

/// Serialized wire time of one session at `budget` over the tight pipe,
/// by the delay-sweep slope in units of the one-way delay.
///
/// The pipe stays fixed across the sweep, so transfer time (bytes over
/// the capacity-limited pipe) and window stall both scale with the delay
/// and survive the slope, while compute — which does not scale — is
/// differenced away.
fn wire_slope(budget: usize) -> u32 {
    let elapsed_at = |delay: Duration| {
        let (left, right) = diverged(budget);
        let mut wire = latency::DelayedWire::new(TIGHT_PIPE, delay);
        let (_pair, elapsed) = wire.round_trip(left, right);
        elapsed
    };
    let (short, long) = (elapsed_at(DELAY), elapsed_at(2 * DELAY));
    u32::try_from(long.saturating_sub(short).as_millis() / DELAY.as_millis())
        .expect("bounded hop count")
}

/// The binding window a budget derives at this suite's session shape:
/// the narrowest capacity among the engaged dispute stages (depths two
/// through four; heights 30 down to 28), as the knee suite reads it.
fn binding_capacity(budget: usize) -> usize {
    let session_len = (COMMON + DIVERGENT) as u64;
    let capacities = window_capacities(session_len, session_len, budget);
    capacities[28..=30]
        .iter()
        .copied()
        .min()
        .expect("three engaged heights")
}

/// The exact operator equation holds on a bandwidth-limited link, and
/// the scalar form is the design point's own identity.
///
/// The unbounded-budget run measures the link's effective rate (transfer
/// hops for a known divergence), giving the link's BDP in messages
/// without assuming how the session spreads bytes across streams. Two
/// constricted budgets then measure real slowdowns against the exact
/// form `max(1, BDP_messages / K)` with `K` the derived binding window,
/// inside the same accuracy band the knee suite certifies for the wave
/// model. The scalar form is pinned where it is exact: the inverse at
/// slowdown one and the design link's BDP reproduces the default budget
/// to the byte, because the default is that expression.
#[test]
fn operator_equations_match_measured_sessions() {
    // The scalar identity: budget(slowdown = 1) at the design link IS
    // the default. 12.5 MB of BDP per millisecond of RTT is the design
    // link's product; the docs' `fans + 22 × BDP / slowdown` is this
    // expression with the ratio left as a quotient.
    let (envelope, wire) = envelope_and_wire_bytes();
    let design_bdp = 12_500_000usize;
    assert_eq!(
        supply_decode_envelope_bytes() + design_bdp / wire * envelope,
        DEFAULT_SYNC_MEMORY_BUDGET,
        "the inverse form at slowdown 1 must reproduce the default budget",
    );

    // The exact form, measured. Transfer baseline first: it calibrates
    // the effective link rate, so BDP_messages = RTT × rate / wire
    // = 2 × divergence / transfer_hops (both sides' divergences cross).
    let transfer = wire_slope(UNBOUNDED);
    // A degenerate slope means the delay sweep was swamped: compute time
    // under machine load dwarfed the virtual delay and the difference
    // collapsed. Fail by name here rather than letting BDP go infinite
    // and the accuracy band compare NaN.
    assert!(
        transfer >= 4,
        "self-calibration measured a degenerate transfer slope ({transfer} hops): \
         the delay sweep was swamped by machine load; rerun on a quieter machine",
    );
    let bdp_messages = 2.0 * DIVERGENT as f64 / f64::from(transfer);

    // The cells are denominated in the budget's *dispute share*: the flat
    // supply-decode envelope comes off every budget before the solve, so
    // it is added back here and the cells keep the calibrated binding
    // windows they were tuned for.
    for dispute_share in [1_200_000usize, 430 * 1024] {
        let budget = supply_decode_envelope_bytes() + dispute_share;
        let window = binding_capacity(budget);
        let predicted = (bdp_messages / window as f64).max(1.0);
        let measured = f64::from(wire_slope(budget)) / f64::from(transfer);
        eprintln!(
            "budget {budget}: window {window}, BDP {bdp_messages:.0} messages, \
             predicted {predicted:.1}x, measured {measured:.1}x",
        );
        assert!(
            predicted > 3.0,
            "the cell must predict real constriction to carry signal: {predicted:.1}x",
        );
        assert!(
            measured / predicted > 0.4 && measured / predicted < 2.5,
            "measured slowdown {measured:.1}x outside the accuracy band of \
             predicted {predicted:.1}x",
        );
    }
}

/// The parity pipe: tight enough that its BDP in messages sits inside
/// the window a moderate budget can derive (the near-root structural cap
/// bounds every window at this corpus scale to ~256 scopes, so the
/// link's BDP must measure below that for parity to be reachable).
const PARITY_PIPE: usize = 4 * 1024;

/// A budget whose derived window is at or above the link's BDP in
/// messages runs at the transfer bound: the parity direction of the
/// inverse form.
#[test]
fn parity_budget_runs_at_the_transfer_bound() {
    let slope_at = |budget: usize| {
        let elapsed_at = |delay: Duration| {
            let (left, right) = diverged(budget);
            let mut wire = latency::DelayedWire::new(PARITY_PIPE, delay);
            let (_pair, elapsed) = wire.round_trip(left, right);
            elapsed
        };
        let (short, long) = (elapsed_at(DELAY), elapsed_at(2 * DELAY));
        u32::try_from(long.saturating_sub(short).as_millis() / DELAY.as_millis())
            .expect("bounded hop count")
    };
    let transfer = slope_at(UNBOUNDED);
    // Same degenerate-slope guard as the operator-equation cell: a
    // load-swamped sweep must fail by name, not divide toward infinity.
    assert!(
        transfer >= 4,
        "self-calibration measured a degenerate transfer slope ({transfer} hops): \
         the delay sweep was swamped by machine load; rerun on a quieter machine",
    );
    let bdp_messages = 2.0 * DIVERGENT as f64 / f64::from(transfer);

    // Grow the budget until the derived window clears the measured BDP:
    // the smallest such budget is what the inverse form denotes, exactly.
    let mut budget = 1 << 20;
    while (binding_capacity(budget) as f64) < bdp_messages {
        budget *= 2;
        assert!(budget < 1 << 30, "a parity window must be derivable");
    }
    let measured = slope_at(budget);
    eprintln!(
        "parity budget {budget} (window {} vs BDP {bdp_messages:.0}): \
         {measured} hops vs transfer {transfer}",
        binding_capacity(budget),
    );
    assert!(
        measured <= transfer + transfer / 2 + 4,
        "a window at the link's BDP must hide its waves under transfer: \
         {measured} hops vs {transfer}",
    );
}
