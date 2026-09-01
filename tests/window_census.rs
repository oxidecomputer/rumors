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
use rumors::{Peer, Rumors};

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
            Peer::<u64>::bootstrap().join(&mut newcomer),
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
    rumors
        .batch(|batch| {
            for _ in 0..n {
                batch.send(rng.next_u64())?;
            }
            Ok::<(), rumors::EncodeError>(())
        })
        .expect("flat test payloads are within any depth limit");
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

/// The lemma-slack pin: measured version bounds stay inside the priced
/// pair bound.
///
/// The window prices every version a session can hold — including the
/// bounds the merged tree assembles from *both* replicas' surviving
/// leaves — at `local_max + remote_max`, the sum of the exchanged
/// per-node aggregates. Each aggregate covers the bounds its own
/// replica materializes, and a cross-side assembly is priced by the
/// pairwise join/meet lemmas — but deletion-honoring can recompute a
/// merged bound over a survivor subset neither input materialized, so
/// this is where the model could silently under-price; measuring a
/// reconciled tree's every bound against the pre-session exchange pins
/// the slack to reality.
#[test]
fn version_bounds_stay_inside_the_priced_pair_bound() {
    let (left, right) = diverged(TIGHT_BUDGET, DIVERGENT_WIDE);

    // A third concurrent history: interior joins over two parties stay
    // near either input's size, so a third is what gives the many-leaf
    // join room to outgrow the pairwise bound if it ever could.
    let third = pollster::block_on(async {
        let (mut provider, mut newcomer) = rumors::link::memory_with_capacity(LINK_CAPACITY);
        let (served, joined) = tokio::join!(
            right.gossip(&mut provider),
            Peer::<u64>::bootstrap().join(&mut newcomer),
        );
        served.expect("serve third bootstrap");
        joined
            .expect("bootstrap third")
            .expect("provider is established")
            .sync_memory_budget(TIGHT_BUDGET)
            .into_rumors()
    });
    let mut rng = SmallRng::seed_from_u64(0x0b05_2026_1e77_a51a);
    send_random(&third, 2_048, &mut rng);
    reconcile(&third, &left);

    let local_max = rumors::testing::max_version_bytes(&left.snapshot());
    let remote_max = rumors::testing::max_version_bytes(&right.snapshot());
    reconcile(&left, &right);

    let measured = rumors::testing::max_bound_bytes(&left.snapshot())
        .max(rumors::testing::max_bound_bytes(&right.snapshot()));
    eprintln!("priced bound {local_max} + {remote_max}, measured max node bound {measured}");
    assert!(
        measured <= local_max + remote_max,
        "a reconciled tree holds a {measured}-byte version bound; the \
         session priced every bound within {local_max} + {remote_max}",
    );
}

/// One newcomer bootstrapped from `provider` over a roomy link.
fn bootstrap_from(provider: &Rumors<u64>) -> Rumors<u64> {
    pollster::block_on(async {
        let (mut serving, mut joining) = rumors::link::memory_with_capacity(LINK_CAPACITY);
        let (served, joined) = tokio::join!(
            provider.gossip(&mut serving),
            Peer::<u64>::bootstrap().join(&mut joining),
        );
        served.expect("serve swarm bootstrap");
        joined
            .expect("bootstrap swarm member")
            .expect("provider is established")
            .sync_window_floor()
            .into_rumors()
    })
}

/// A wide concurrent frontier stays inside the exchanged version bound.
///
/// The session memory model prices every bound it can hold within the
/// exchanged pair sum, so the aggregate each greeting carries must cover
/// interior ceilings and floors, not just leaf stamps. This corpus is
/// the shape that separates the two: parties forked in doubling
/// generations (so each interval sits shallow) each stamp one message
/// concurrently, and one replica gathers all of them — every leaf
/// version is a small single-spike stamp, while the gathered tree's
/// interior ceilings join *all* the frontiers and encode several times
/// larger than any leaf. A leaf-denominated exchange under-prices
/// exactly here; the bound-covering aggregate holds.
#[test]
fn wide_concurrent_frontiers_stay_inside_the_exchanged_bound() {
    // Doubling generations: every fork halves a *different* interval, so
    // party intervals stay shallow and stamps stay small — the many
    // frontiers accumulate in the join, not in any one leaf.
    let seed = Peer::seed().sync_window_floor().into_rumors();
    let mut rng = SmallRng::seed_from_u64(0x0b05_2026_f207_713a);
    send_random(&seed, 4, &mut rng);
    let mut swarm = vec![seed];
    for _ in 0..5 {
        let next: Vec<_> = swarm.iter().map(bootstrap_from).collect();
        swarm.extend(next);
    }

    // Every member stamps concurrently, each a *different* number of
    // times: no cross-sync, so the stamps are mutually concurrent, and
    // the ragged counts keep the joined frontier from saturating into a
    // uniform plateau — the join must carry one distinct count per
    // interval, while each member's own stamps refine only its own.
    for (ticks, member) in swarm.iter().enumerate() {
        member
            .batch(|batch| {
                for _ in 0..=ticks {
                    batch.send(rng.next_u64())?;
                }
                Ok::<(), rumors::EncodeError>(())
            })
            .expect("flat test payloads are within any depth limit");
    }

    // One replica gathers the whole frontier; the last member stays
    // un-gathered as the session counterparty.
    let (gatherer, rest) = swarm.split_first().expect("swarm is non-empty");
    let (remote, gathered) = rest.split_last().expect("swarm has members");
    for member in gathered {
        reconcile(gatherer, member);
    }

    let local_max = rumors::testing::max_version_bytes(&gatherer.snapshot());
    let remote_max = rumors::testing::max_version_bytes(&remote.snapshot());
    reconcile(gatherer, remote);

    let measured = rumors::testing::max_bound_bytes(&gatherer.snapshot())
        .max(rumors::testing::max_bound_bytes(&remote.snapshot()));
    eprintln!("priced bound {local_max} + {remote_max}, measured max node bound {measured}");
    assert!(
        measured <= local_max + remote_max,
        "a gathered concurrent frontier holds a {measured}-byte version \
         bound; the session priced every bound within {local_max} + {remote_max}",
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
