//! Where pipelining ends: the divergence the static window predicts must
//! serialize.
//!
//! The window derivation fixes each level's channel capacity, so it also
//! predicts its own knee: a session stays fully pipelined while the
//! disputed scopes at every engaged level fit that level's capacity, and
//! beyond it the descent drains in capacity-sized waves — one round trip
//! per wave at the binding level. These tests read the derived capacities
//! back through `rumors::testing::window_capacities`, place divergences
//! on both sides of the predicted knee — each cell against the binding
//! capacity of its own session shape, the same derivation the session
//! performs at greeting time, so the prediction and the measurement
//! cannot drift apart — and measure serialized wire hops in exact
//! virtual time on a delayed link: flat below the knee, growing with the
//! wave count above it.

// Only the delayed wire is exercised here; the module's pipes and
// conformance surface belong to the benches and `latency_link.rs`.
#[allow(dead_code)]
#[path = "../benches/support/latency.rs"]
mod latency;

use std::time::Duration;

use rand::rngs::SmallRng;
use rand::{RngCore, SeedableRng};
use rumors::testing::window_capacities;
use rumors::{Peer, Rumors};

/// A budget sized so the binding capacity lands in the low hundreds:
/// small enough that a modest divergence crosses the knee at test scale,
/// large enough that below-knee sessions genuinely pipeline.
const BUDGET: usize = 2 << 20;

/// Messages both peers share before the fork.
const COMMON: usize = 2_048;

/// One-way link delay, in whole milliseconds (the timer wheel's grain).
const DELAY: Duration = Duration::from_millis(10);

/// Serialized one-way hops a fully pipelined session may spend.
///
/// The below-knee cells measure 7 exact hops (the phase ladder's few
/// active levels); the bound's headroom admits a deeper engaged ladder,
/// never wave costs — the growth cell already exceeds it.
const PIPELINED_HOPS: u32 = 12;

/// Extra hops the above-knee session must show beyond the below-knee
/// one: the wave prediction at the growth cell is well past this, and a
/// pipelined ladder's shape-to-shape drift is well under it.
const KNEE_MARGIN: u32 = 12;

/// Per-stream in-flight window: far above this test's transfers, so only
/// round-trip structure is measured.
const LINK_CAPACITY: usize = 8 * 1024 * 1024;

/// The linearity suite's cells, in eighths of each cell's own capacity.
///
/// Widely spaced: each marginal spans at least eight capacities of
/// divergence (~16 hops of signal), so the ladder constants that shift
/// between session shapes stay far below the wave signal they are
/// differenced against.
const LINEAR_CELLS: [usize; 3] = [32, 96, 288];

/// The above-knee wave-growth cell, in eighths of its own capacity.
const GROWTH_CELL: usize = 64;

/// The bandwidth-bound comparison cell, in eighths of its own capacity.
const STALL_CELL: usize = 128;

/// The lower below-knee cell: half of its session's capacity, in eighths.
const BELOW_HALF: usize = 4;
/// The upper below-knee cell, in eighths: strictly inside the knee.
///
/// Seven eighths rather than the full capacity: a cell placed exactly on
/// the boundary flips into one serialized wave on a ±1 measurement
/// wobble, so the below-knee claim is stated for the regime's interior.
const BELOW_FULL: usize = 7;

/// The binding capacity for one measured session shape: the minimum over
/// the levels a test-scale divergence engages (depths two through four,
/// heights 30 down to 28).
///
/// The engaged band is a collision expectation: under uniform hashing the
/// expected leaf pairs sharing a depth-`j` slot are `E = C(n,2)/256^j`
/// for a session of `n` leaves, falling ~256x per further depth. At this
/// file's scale (`n` runs a few hundred past [`COMMON`]) that is ~32-40
/// pairs forcing descent to depth three and ~0.13-0.16 expected pairs
/// reaching depth four: depth four's capacity enters the minimum as tail
/// coverage, not typical population, and expected engagement past depth
/// four is below 10^-3.
///
/// This is the derivation the session itself performs at greeting time,
/// evaluated at the sizes the session will actually exchange — both
/// sides hold [`COMMON`] plus their divergence — so every cell's
/// prediction and its measurement rest on the same numbers. A separate
/// band check keeps the knee reachable at test scale.
fn binding_capacity_at(divergent_per_side: usize) -> usize {
    let session_len = (COMMON + divergent_per_side) as u64;
    let capacities = window_capacities(session_len, session_len, BUDGET);
    let binding = capacities[28..=30]
        .iter()
        .copied()
        .min()
        .expect("three engaged heights");
    assert!(
        (4..=256).contains(&binding),
        "the test budget must land every measured cell's binding capacity \
         in a band where both sides of the knee are reachable at test \
         scale; got {binding} at divergence {divergent_per_side}",
    );
    binding
}

/// The smallest divergence at or above `eighths`/8 of its own capacity.
///
/// A cell's placement is self-referential: its target is a multiple of
/// the binding capacity of the session that cell runs, which shrinks as
/// the divergence grows. `8d / capacity(d)` is monotone in `d`, so a
/// binary search solves the placement, and the landing assertion is the
/// coupling between derivation and measurement: a derivation change that
/// displaces a cell from its target multiple fails here, loudly, instead
/// of silently measuring a shape the prediction does not cover.
fn divergence_at_least(eighths: usize) -> usize {
    let reaches = |d: usize| 8 * d >= eighths * binding_capacity_at(d);
    let mut hi = binding_capacity_at(0).max(1);
    while !reaches(hi) {
        hi = hi.checked_mul(2).expect("cell placement must terminate");
    }
    let mut lo = 0;
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if reaches(mid) { hi = mid } else { lo = mid }
    }
    let capacity = binding_capacity_at(hi);
    assert!(
        8 * hi >= eighths * capacity && 8 * hi <= eighths * capacity * 3 / 2 + 8,
        "a measured cell must land near its target multiple of its own \
         session's binding capacity (the derivation/measurement coupling): \
         {hi} divergent, capacity {capacity}, target {eighths}/8",
    );
    hi
}

/// The largest divergence at or below `eighths`/8 of its own capacity.
///
/// The below-knee dual of [`divergence_at_least`], with the same landing
/// assertion coupling the cell to the prediction for its own shape.
fn divergence_at_most(eighths: usize) -> usize {
    let exceeds = |d: usize| 8 * d > eighths * binding_capacity_at(d);
    let mut hi = binding_capacity_at(0).max(2);
    while !exceeds(hi) {
        hi = hi.checked_mul(2).expect("cell placement must terminate");
    }
    let mut lo = 0;
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if exceeds(mid) { hi = mid } else { lo = mid }
    }
    let cell = hi - 1;
    let capacity = binding_capacity_at(cell);
    assert!(
        cell >= 1 && 8 * cell <= eighths * capacity && 16 * cell + 16 >= eighths * capacity,
        "a below-knee cell must land near its target multiple of its own \
         session's binding capacity (the derivation/measurement coupling): \
         {cell} divergent, capacity {capacity}, target {eighths}/8",
    );
    cell
}

/// Serialized one-way hops one divergent session spends on the wire.
fn hops(divergent_per_side: usize) -> u32 {
    hops_over(divergent_per_side, LINK_CAPACITY)
}

/// [`hops`], with the pipe's per-stream in-flight window chosen by the
/// caller.
///
/// Measured in exact virtual time: compute costs zero virtual time on
/// the paused clock, and a tight pipe's transfer time (`bytes × delay /
/// capacity`) is delay-denominated, so it lands in hop units exactly
/// like wave stall. The count is a deterministic function of the
/// session shape — machine load cannot move it.
fn hops_over(divergent_per_side: usize, pipe_capacity: usize) -> u32 {
    latency::session_hops(pipe_capacity, DELAY, diverged(divergent_per_side))
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
            Peer::<u64>::bootstrap().join(&mut newcomer),
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
    rumors.send_all((0..n).map(|_| rng.next_u64())).unwrap();
}

/// Below the predicted knee the session is pipelined: hops are bounded
/// by the phase ladder, not by scope count.
///
/// Widening the divergence
/// toward the knee may engage one more trie level — a couple of ladder
/// hops — but never a per-wave cost, which is what separates it from the
/// above-knee regime.
#[test]
fn below_the_knee_hops_are_flat() {
    let half_cell = divergence_at_most(BELOW_HALF);
    let full_cell = divergence_at_most(BELOW_FULL);
    let half = hops(half_cell);
    let full = hops(full_cell);
    eprintln!(
        "below-knee cells {half_cell} (capacity {}) and {full_cell} \
         (capacity {}): hops {half} at half, {full} near capacity",
        binding_capacity_at(half_cell),
        binding_capacity_at(full_cell),
    );
    assert!(
        half <= PIPELINED_HOPS && full <= PIPELINED_HOPS,
        "below-knee sessions must stay pipelined: {half} and {full} hops \
         against a budget of {PIPELINED_HOPS}",
    );
    assert!(
        full <= half + 4,
        "widening a below-knee divergence toward the knee may deepen the \
         ladder, not serialize: {half} -> {full}",
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
/// latency-only link (roomy pipe), each marginal hops-per-message
/// between consecutive cells must track the wave model's prediction for
/// those cells' own shapes, `Δ(2·d/capacity(d)) / Δd`, within a factor
/// of two either way.
#[test]
fn above_the_knee_cost_is_linear_in_divergence() {
    let divergences = LINEAR_CELLS.map(divergence_at_least);
    let capacities = divergences.map(binding_capacity_at);
    let times: Vec<u32> = divergences.iter().map(|&d| hops(d)).collect();
    let cells: Vec<(usize, usize)> = divergences.into_iter().zip(capacities).collect();
    let slopes: Vec<f64> = times
        .windows(2)
        .zip(cells.windows(2))
        .map(|(t, c)| (f64::from(t[1]) - f64::from(t[0])) / (c[1].0 - c[0].0) as f64)
        .collect();
    // The wave model's marginal between two cells, each at its own
    // shape's capacity: Δ(2·d/capacity) over Δd.
    let predicted: Vec<f64> = cells
        .windows(2)
        .map(|c| {
            let waves = |(d, capacity): (usize, usize)| 2.0 * d as f64 / capacity as f64;
            (waves(c[1]) - waves(c[0])) / (c[1].0 - c[0].0) as f64
        })
        .collect();
    eprintln!(
        "cells (divergence, capacity) {cells:?}, hops {times:?}, marginal \
         hops/message {slopes:?} (wave model predicts {predicted:?})",
    );
    // Each marginal slope must sit in a band around its own predicted
    // marginal — the direct statement of constant-per-message cost. The
    // band absorbs what the exact counts still carry: the ladder
    // constants that shift between the two shapes each marginal
    // differences, and the wave model's own approximation error.
    for (&slope, &predicted) in slopes.iter().zip(&predicted) {
        assert!(
            slope / predicted > 0.5 && slope / predicted < 2.0,
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
    let divergence = divergence_at_least(STALL_CELL);
    let capacity = binding_capacity_at(divergence);
    // 2 KiB in flight at 10 ms one-way models ~200 KB/s per stream: a
    // BDP of ~2 KiB — ~57 messages at this corpus's measured ~36 B each
    // (tests/dispute_wire.rs pins the affine per-message cost) — below
    // the binding capacity, so bandwidth binds before the window does.
    let bandwidth_bound = hops_over(divergence, 2 * 1024);
    let stall_bound = hops(divergence);
    eprintln!(
        "capacity {capacity}, divergence {divergence}: \
         latency-only {stall_bound} hops, bandwidth-bound {bandwidth_bound} hops",
    );
    assert!(
        stall_bound <= bandwidth_bound,
        "window stall ({stall_bound} hops) must hide inside the \
         bandwidth-bound transfer ({bandwidth_bound} hops) once the pipe's \
         BDP in messages is below the window",
    );
}

/// Above the predicted knee the descent serializes into capacity-sized
/// waves.
///
/// A divergence at [`GROWTH_CELL`] eighths of its own session's
/// binding capacity costs measurably more round trips than the
/// pipelined session, in the direction and scale the derivation
/// predicts.
#[test]
fn above_the_knee_hops_grow() {
    let below_cell = divergence_at_most(BELOW_HALF);
    let above_cell = divergence_at_least(GROWTH_CELL);
    let below = hops(below_cell);
    let above = hops(above_cell);
    eprintln!(
        "cells {below_cell} (capacity {}) and {above_cell} (capacity {}): \
         hops {below} below, {above} above",
        binding_capacity_at(below_cell),
        binding_capacity_at(above_cell),
    );
    assert!(
        above >= below + KNEE_MARGIN,
        "a divergence at {GROWTH_CELL} eighths of its own binding capacity \
         must serialize into waves: measured {above} hops vs {below} \
         pipelined",
    );
}
