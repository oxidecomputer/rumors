//! The window's memory bound checked against measured node residency.
//!
//! The crate counts every live tree-node handle under `test-internals`
//! ([`node_census`]): constructions and clones check in, drops check out,
//! and the high-water mark is exact concurrent residency. A session's
//! peak has four parts: the pre-session generation (alive until the
//! atomic replace), the reconciled generation, the session's output tree
//! (transiently coexisting with both at the commit join), and the
//! window's in-flight work. The first three are content and scale with
//! the divergence; only the fourth is the window's to bound. These tests
//! isolate it by differencing runs of the *identical* divergence under
//! different budgets, then hold it against the admittance the derived
//! capacities state.
//!
//! The census is process-global, which is sound here because nextest runs
//! each test in its own process.

use rand::rngs::SmallRng;
use rand::{RngCore, SeedableRng};
use rumors::testing::{node_census, node_census_reset, window_capacities};
use rumors::{Peer, Protocol, Rumors};

/// Per-stream in-memory link buffering: far above every transfer here, so
/// link backpressure never shapes residency.
const LINK_CAPACITY: usize = 8 * 1024 * 1024;

/// A budget that binds at test scale: a few scopes per level.
const TIGHT_BUDGET: usize = 64 * 1024;

/// Messages each side originates beyond the common prefix.
const DIVERGENT_WIDE: usize = 20_000;

/// Handles one buffered scope can pin at most: a full fan of child
/// references plus its own bookkeeping.
const HANDLES_PER_SCOPE: usize = 256 + 2;

/// Handles the assembly fan queues can hold beyond the window: one full
/// fan per active level (their capacity is a correctness floor the window
/// never scales; see the window module docs).
const ASSEMBLY_FAN_HANDLES: usize = 33 * 256;

/// Transient slack: conversion buffers, in-hand replies, and the join's
/// working set, all bounded per session rather than per divergence.
const TRANSIENT_SLACK: usize = 8 * 1024;

/// Two peers sharing a bootstrap, then diverged by `divergent` random
/// messages on each side, both configured with `budget`.
fn diverged(budget: usize, divergent: usize) -> (Rumors<u64>, Rumors<u64>) {
    let left = Peer::seed().sync_memory_budget(budget).into_rumors();
    let mut rng = SmallRng::seed_from_u64(0x00c0_ffee_0b05_cafe);
    send_random(&left, 1_024, &mut rng);

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
            .sync_memory_budget(budget)
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

/// One reconciliation session over a roomy in-memory link.
fn reconcile(a: &Rumors<u64>, b: &Rumors<u64>) {
    pollster::block_on(async {
        let (mut left, mut right) = rumors::link::memory_with_capacity(LINK_CAPACITY);
        let (a_result, b_result) = tokio::join!(a.gossip(&mut left), b.gossip(&mut right));
        a_result.expect("gossip a");
        b_result.expect("gossip b");
    });
}

/// The session-peak handles a run holds above its two resting
/// generations: content double-existence plus window in-flight.
fn overhead(budget: usize, divergent: usize) -> usize {
    let (left, right) = diverged(budget, divergent);
    let before = node_census().live;
    node_census_reset();
    reconcile(&left, &right);
    let peak = node_census().peak;
    let after = node_census().live;
    let overhead = peak.saturating_sub(before + after);
    eprintln!(
        "budget {budget}, divergence {divergent}: peak {peak}, \
         generations {before}+{after}, overhead {overhead}",
    );
    overhead
}

/// Window-attributable residency stays inside the derived admittance.
///
/// The identical divergence runs once at the zero-budget floor and once
/// at a budget that binds at test scale; the content components (both
/// generations and the output tree) are the same trees in both runs, so
/// the peak difference is the window's own buffering — which must stay
/// inside what the derived capacities admit: each scope a full fan of
/// handles, plus the assembly fans and bounded per-session slack. The
/// admittance is denominated in the same capacities `sync_memory_budget`
/// derives, so a regression that buffers past the window moves the
/// measurement, not the bound.
#[test]
fn window_attributable_residency_stays_inside_admittance() {
    // The sizes the session itself will exchange: both replicas hold the
    // common prefix plus their own divergence when they reconcile.
    let session_len = (1_024 + DIVERGENT_WIDE) as u64;
    let capacities = window_capacities(session_len, session_len, TIGHT_BUDGET);
    let admitted: usize = capacities.iter().sum::<usize>() * HANDLES_PER_SCOPE
        + ASSEMBLY_FAN_HANDLES
        + TRANSIENT_SLACK;
    let floor = overhead(0, DIVERGENT_WIDE);
    let windowed = overhead(TIGHT_BUDGET, DIVERGENT_WIDE);
    eprintln!(
        "admittance {admitted} (capacities sum {})",
        capacities.iter().sum::<usize>(),
    );
    assert!(
        windowed <= floor + admitted,
        "widening the window from the floor added {} handles at peak; \
         the derived capacities admit {admitted}",
        windowed.saturating_sub(floor),
    );
}

/// Content overhead at the floor is the output tree, not the divergence.
///
/// At the one-slot floor the window holds almost nothing, so a session's
/// peak above its generations is the commit's double-existence: the
/// output tree alive beside the joining result. That is bounded by the
/// reconciled content itself — it cannot silently grow into a multiple
/// of it.
#[test]
fn floor_overhead_is_bounded_by_content() {
    let (left, right) = diverged(0, DIVERGENT_WIDE);
    let before = node_census().live;
    node_census_reset();
    reconcile(&left, &right);
    let peak = node_census().peak;
    let after = node_census().live;
    let overhead = peak.saturating_sub(before + after);
    eprintln!("floor: peak {peak}, generations {before}+{after}, overhead {overhead}");
    assert!(
        overhead <= after + TRANSIENT_SLACK,
        "floor session held {overhead} handles above its generations, \
         more than one output tree ({after}) can explain",
    );
}
