//! The operator wave model held against measured sessions.
//!
//! `sync_memory_budget`'s docs publish the closed-form estimate,
//! `slowdown(budget, m) ≈ max(1, BDP × envelope / (budget × (28 + m)))`:
//! the large-window simplification of the exact wave-model form
//! `slowdown = max(1, BDP_messages / K)` with `K` the derived window.
//! The simplification substitutes `K ≈ (budget − fans) / envelope`
//! (`fans` is the flat supply-decode pre-charge) and
//! `BDP_messages = BDP / (28 + m)`, the calibrated per-message wire law
//! `tests/dispute_wire.rs` pins. The `K` substitution overstates the
//! window by roughly `F / budget`, with `F` the corpus-fixed component
//! of the real charge (4.7–7.9 MB at the design corpus; the band and
//! its decomposition are worked at `Peer::sync_memory_budget`) — which
//! is why the committed trade-off table carries the solve's own windows
//! and the pins here hold the exact form, never the scalar.
//!
//! The pin here holds the *exact* wave form against sessions on a
//! genuinely bandwidth-limited pipe, with the link rate self-calibrated
//! from the unbounded-budget transfer (the pipe carries several
//! concurrent streams, so its effective rate is measured, not assumed).

// Only the delayed wire is exercised here; the module's pipes and
// conformance surface belong to the benches and `latency_link.rs`.
#[allow(dead_code)]
#[path = "../benches/support/latency.rs"]
mod latency;

use std::time::Duration;

use rand::rngs::SmallRng;
use rand::{RngCore, SeedableRng};
use rumors::testing::{supply_decode_envelope_bytes, window_capacities};
use rumors::{Peer, Protocol, Rumors};

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

    send_random(&left, DIVERGENT, &mut rng);
    send_random(&right, DIVERGENT, &mut rng);
    (left, right)
}

/// Commit `n` random payloads as one batch.
fn send_random(rumors: &Rumors<u64>, n: usize, rng: &mut SmallRng) {
    rumors
        .batch(|batch| {
            for _ in 0..n {
                batch.send(rng.next_u64())?;
            }
            Ok::<(), rumors::EncodeError>(())
        })
        .expect("flat test payloads are within any depth limit");
}

/// Serialized wire hops of one session at `budget` over the tight pipe.
///
/// Measured in exact virtual time: transfer time (bytes over the
/// capacity-limited pipe) and window stall are both delay-denominated,
/// while compute costs zero virtual time, so the count is a
/// deterministic function of the session shape — machine load cannot
/// move it.
fn wire_hops(budget: usize) -> u32 {
    latency::session_hops(TIGHT_PIPE, DELAY, diverged(budget))
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

/// The exact wave-model equation holds on a bandwidth-limited link.
///
/// The unbounded-budget run measures the link's effective rate (transfer
/// hops for a known divergence), giving the link's BDP in messages
/// without assuming how the session spreads bytes across streams. Two
/// constricted budgets then measure real slowdowns against the exact
/// form `max(1, BDP_messages / K)` with `K` the derived binding window,
/// inside the same accuracy band the knee suite certifies for the wave
/// model. The docs' closed form is this equation with `K` and
/// `BDP_messages` substituted by their large-window laws.
#[test]
fn wave_model_matches_measured_sessions() {
    // Transfer baseline first: it calibrates the effective link rate,
    // so BDP_messages = RTT × rate / wire = 2 × divergence /
    // transfer_hops (both sides' divergences cross).
    let transfer = wire_hops(UNBOUNDED);
    // The calibration divides by this count, so the tight pipe must
    // genuinely bind: a transfer too fast to price the pipe would carry
    // no bandwidth signal into BDP_messages.
    assert!(
        transfer >= 4,
        "self-calibration needs a genuinely bandwidth-limited transfer: \
         {transfer} hops",
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
        let measured = f64::from(wire_hops(budget)) / f64::from(transfer);
        eprintln!(
            "budget {budget}: window {window}, BDP {bdp_messages:.0} messages, \
             predicted {predicted:.1}x, measured {measured:.1}x",
        );
        assert!(
            predicted > 3.0,
            "the cell must predict real constriction to carry signal: {predicted:.1}x",
        );
        assert!(
            measured / predicted > 0.5 && measured / predicted < 2.0,
            "measured slowdown {measured:.1}x outside the accuracy band of \
             predicted {predicted:.1}x",
        );
    }
}

/// The parity pipe: tight enough that its BDP in messages sits inside
/// the window a moderate budget can derive.
///
/// (The near-root structural cap
/// bounds every window at this corpus scale to ~256 scopes, so the
/// link's BDP must measure below that for parity to be reachable.)
const PARITY_PIPE: usize = 4 * 1024;

/// A budget whose derived window is at or above the link's BDP in
/// messages runs at the transfer bound: the parity direction of the
/// inverse form.
#[test]
fn parity_budget_runs_at_the_transfer_bound() {
    let hops_at = |budget: usize| latency::session_hops(PARITY_PIPE, DELAY, diverged(budget));
    let transfer = hops_at(UNBOUNDED);
    // Same binding-pipe guard as the operator-equation cell: the
    // calibration divides by this count, so a transfer too fast to price
    // the pipe would carry no bandwidth signal.
    assert!(
        transfer >= 4,
        "self-calibration needs a genuinely bandwidth-limited transfer: \
         {transfer} hops",
    );
    let bdp_messages = 2.0 * DIVERGENT as f64 / f64::from(transfer);

    // Grow the budget until the derived window clears the measured BDP:
    // the smallest such budget is what the inverse form denotes, exactly.
    let mut budget = 1 << 20;
    while (binding_capacity(budget) as f64) < bdp_messages {
        budget *= 2;
        assert!(budget < 1 << 30, "a parity window must be derivable");
    }
    let measured = hops_at(budget);
    eprintln!(
        "parity budget {budget} (window {} vs BDP {bdp_messages:.0}): \
         {measured} hops vs transfer {transfer}",
        binding_capacity(budget),
    );
    // Measured excess at this shape is 2 hops over a 97-hop transfer;
    // the additive allowance admits ladder drift, never a wave regime
    // (one wave at this window would add tens of hops).
    assert!(
        measured <= transfer + transfer / 8 + 4,
        "a window at the link's BDP must hide its waves under transfer: \
         {measured} hops vs {transfer}",
    );
}
