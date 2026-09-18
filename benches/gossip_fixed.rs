// `criterion_group!` generates a public registrar with no documentation.
#![allow(missing_docs)]

//! Fixed-size over-the-wire gossip sweeps.
//!
//! Every fixture starts with a total universe size of `N = 10_000` possible
//! actions and varies only where the work lands: shared pre-fork insertions,
//! post-fork divergent insertions, or post-fork redactions. Each benchmark
//! drives [`Rumors::gossip`] over one persistent in-memory link, so the timed
//! body pays for the gossip session rather than transport allocation.
//!
//! The fixed-size groups measure the wire protocol on the same fixtures. Their
//! series retain the `V2` label used by existing Criterion baselines:
//!
//! - `gossip_fixed_bidir_insertions`: total post-fork insertions `I`.
//! - `gossip_fixed_bidir_redactions`: total post-fork redactions `R`.
//! - `gossip_fixed_unilateral_insertions`: one-side post-fork insertions `I`.
//! - `gossip_fixed_unilateral_redactions`: one-side post-fork redactions `R`.
//!
//! # The latency knob
//!
//! Two further groups reuse the bidirectional fixtures but add a third
//! dimension: one-way link latency, swept as the x-axis at a few fixed
//! divergence points (the full divergence sweep times a latency sweep
//! would be quadratic in wall time for no added signal):
//!
//! - `gossip_latency_bidir_insertions`: `I ∈ {0, 5000}`.
//! - `gossip_latency_bidir_redactions`: `R = 2500`.
//!
//! Sessions run over the test kit's delayed-pipe link, which
//! charges wire delay to a paused runtime clock, so the sweep costs
//! wall-clock compute only and the reported duration is elapsed compute time
//! plus virtual wire stall. Read each line's intercept as the protocol's
//! computational cost and its slope as latency sensitivity: the slope at
//! one-way delay `d`, divided by `d`, is the session's serialized one-way
//! hop count. `I = 0` is the identical corner — the steady-state "nothing
//! changed" handshake a production peer pays every gossip round.
//!
//! Read the slopes for regressions: a hop count scaling with disputed
//! scopes rather than tree depth is the serialized inter-stage regime the
//! session window exists to prevent, and it shows up here as slope growing
//! with divergence.

use std::hint::black_box;
use std::time::Duration;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rand::seq::SliceRandom;
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaChaRng;
use rumors::{Peer, Rumors, Version};
use rumors_testkit::bench::{grid, latency};

/// Total actions represented by every fixed-size fixture.
const N: usize = 10_000;
/// Distance between insertion-divergence samples.
const INSERT_STEP: usize = 500;
/// Distance between redaction-divergence samples.
const REDACT_STEP: usize = 250;

/// One-way link delays for the latency sweep, in whole milliseconds:
/// Tokio's timer wheel rounds sub-millisecond deadlines up, so finer
/// values would be quantized anyway.
///
/// Zero anchors each line's intercept at the pure-compute cost; the
/// wire-stall component scales linearly in the delay, so three nonzero
/// decades pin the slope.
const LATENCY_SWEEP_MS: &[u64] = &[0, 1, 10, 100];

/// Divergence points measured under latency: the identical corner (the
/// steady-state handshake) and a large delta. Intermediate points add
/// fixture-build wall time without bending the latency lines.
const LATENCY_INSERTIONS: &[usize] = &[0, 5_000];

/// The single redaction divergence measured under latency; the insertion
/// sweep already carries the size-scaling signal.
const LATENCY_REDACTIONS: &[usize] = &[2_500];

/// Per-stream in-flight window for the latency sweep: far above any
/// session's per-stream transfer at `N = 10_000`.
///
/// Measurements therefore isolate round-trip serialization from
/// bandwidth-delay throttling (the window caps per-stream throughput at
/// `capacity / delay`).
const LATENCY_CAPACITY: usize = 8 * 1024 * 1024;

/// How the two peers diverge after their shared history.
#[derive(Clone, Copy)]
enum Scenario {
    /// Both peers add messages.
    BidirInsertions,
    /// Both peers redact distinct messages.
    BidirRedactions,
    /// One peer adds messages.
    UnilateralInsertions,
    /// One peer redacts messages.
    UnilateralRedactions,
}

/// Describe and build one divergence family.
impl Scenario {
    /// Criterion group for the fixed-size sweep.
    fn group_name(self) -> &'static str {
        match self {
            Scenario::BidirInsertions => "gossip_fixed_bidir_insertions",
            Scenario::BidirRedactions => "gossip_fixed_bidir_redactions",
            Scenario::UnilateralInsertions => "gossip_fixed_unilateral_insertions",
            Scenario::UnilateralRedactions => "gossip_fixed_unilateral_redactions",
        }
    }

    /// Largest divergence represented by this family.
    fn max_param(self) -> usize {
        match self {
            Scenario::BidirInsertions | Scenario::UnilateralInsertions => N,
            Scenario::BidirRedactions | Scenario::UnilateralRedactions => N / 2,
        }
    }

    /// Distance between samples in this family.
    fn step(self) -> usize {
        match self {
            Scenario::BidirInsertions | Scenario::UnilateralInsertions => INSERT_STEP,
            Scenario::BidirRedactions | Scenario::UnilateralRedactions => REDACT_STEP,
        }
    }

    /// Build peers with `param` total divergent actions.
    fn build(self, param: usize) -> (Rumors<u8>, Rumors<u8>) {
        match self {
            Scenario::BidirInsertions => build_bidir_insertions(param),
            Scenario::BidirRedactions => build_bidir_redactions(param),
            Scenario::UnilateralInsertions => build_unilateral_insertions(param),
            Scenario::UnilateralRedactions => build_unilateral_redactions(param),
        }
    }
}

/// Measure gossip as divergence grows within a fixed total population.
fn bench_gossip_fixed(c: &mut Criterion) {
    let mut wire = grid::wire::Wire::new();

    for scenario in [
        Scenario::BidirInsertions,
        Scenario::BidirRedactions,
        Scenario::UnilateralInsertions,
        Scenario::UnilateralRedactions,
    ] {
        let mut group = c.benchmark_group(scenario.group_name());
        group.sample_size(grid::sample_size_for(N));

        for param in (0..=scenario.max_param()).step_by(scenario.step()) {
            if param > 0 {
                group.throughput(Throughput::Elements(param as u64));
            }
            group.bench_function(BenchmarkId::new("V2", param), |b| {
                b.iter_batched(
                    || scenario.build(param),
                    |(left, right)| black_box(wire.round_trip(left, right)),
                    BatchSize::PerIteration,
                )
            });
        }

        group.finish();
    }
}

/// Measure how serialized wire dependencies amplify one-way latency.
fn bench_gossip_latency(c: &mut Criterion) {
    let sweeps: [(&str, Scenario, &[usize]); 2] = [
        (
            "gossip_latency_bidir_insertions",
            Scenario::BidirInsertions,
            LATENCY_INSERTIONS,
        ),
        (
            "gossip_latency_bidir_redactions",
            Scenario::BidirRedactions,
            LATENCY_REDACTIONS,
        ),
    ];

    for (name, scenario, params) in sweeps {
        let mut group = c.benchmark_group(name);
        // The virtual component is deterministic and the wall component is
        // the same magnitude the fixed groups already sample heavily, so a
        // small sample count and short windows suffice. The windows bound
        // *reported* (largely virtual) time, while the real wall cost is
        // the untimed fixture rebuild every iteration pays — keeping them
        // tight is what keeps this group's wall time in check.
        group.sample_size(10);
        group.warm_up_time(Duration::from_millis(150));
        group.measurement_time(Duration::from_millis(300));

        for &latency_ms in LATENCY_SWEEP_MS {
            let delay = Duration::from_millis(latency_ms);
            let mut wire = latency::DelayedWire::new(LATENCY_CAPACITY, delay);
            for &param in params {
                group.bench_function(
                    BenchmarkId::new(format!("V2/divergence={param}"), latency_ms),
                    |b| {
                        b.iter_custom(|iters| {
                            let mut total = Duration::ZERO;
                            for _ in 0..iters {
                                let (left, right) = scenario.build(param);
                                let (pair, elapsed) = wire.round_trip(left, right);
                                black_box(pair);
                                total += elapsed;
                            }
                            total
                        })
                    },
                );
            }
        }

        group.finish();
    }
}

/// Build peers that independently add half of `total_insertions` each.
fn build_bidir_insertions(total_insertions: usize) -> (Rumors<u8>, Rumors<u8>) {
    assert!(total_insertions <= N);
    assert_eq!(total_insertions % 2, 0);

    let left = seeded_with_messages(N - total_insertions, 0x1189_2d1a_c54f_a94d);
    let right = grid::wire::bootstrap_fork(&left);
    let per_side = total_insertions / 2;

    left.send_all(random_bytes(
        per_side,
        0x7a27_9f20_6c8b_d141 ^ total_insertions as u64,
    ))
    .expect("flat test payloads are within any depth limit");
    right
        .send_all(random_bytes(
            per_side,
            0xc436_90ed_83f6_5b55 ^ total_insertions as u64,
        ))
        .expect("flat test payloads are within any depth limit");

    (left, right)
}

/// Build peers where only the left adds `total_insertions` messages.
fn build_unilateral_insertions(total_insertions: usize) -> (Rumors<u8>, Rumors<u8>) {
    assert!(total_insertions <= N);

    let left = seeded_with_messages(N - total_insertions, 0x70e4_a5b8_cce0_25da);
    let right = grid::wire::bootstrap_fork(&left);

    left.send_all(random_bytes(
        total_insertions,
        0xf193_d419_8d66_85d1 ^ total_insertions as u64,
    ))
    .expect("flat test payloads are within any depth limit");

    (left, right)
}

/// Build peers that redact disjoint halves of `total_redactions`.
fn build_bidir_redactions(total_redactions: usize) -> (Rumors<u8>, Rumors<u8>) {
    assert!(total_redactions <= N / 2);
    assert_eq!(total_redactions % 2, 0);

    let (left, versions) = seeded_with_versions(N, 0xc786_a046_6b7d_c9d3);
    let right = grid::wire::bootstrap_fork(&left);
    let shuffled = shuffled_versions(versions, 0x84f6_7932_1265_9eec ^ total_redactions as u64);
    let per_side = total_redactions / 2;

    left.redact_all(&shuffled[..per_side]);
    right.redact_all(&shuffled[per_side..total_redactions]);

    (left, right)
}

/// Build peers where only the left redacts `total_redactions` messages.
fn build_unilateral_redactions(total_redactions: usize) -> (Rumors<u8>, Rumors<u8>) {
    assert!(total_redactions <= N / 2);

    let (left, versions) = seeded_with_versions(N, 0x2526_34f4_918f_e1c7);
    let right = grid::wire::bootstrap_fork(&left);
    let shuffled = shuffled_versions(versions, 0xd4f9_f46b_3c09_1d60 ^ total_redactions as u64);

    left.redact_all(&shuffled[..total_redactions]);

    (left, right)
}

/// A seed peer measuring shipped behavior: the default pipeline window is
/// the production budget in every build shape (see `support/wire.rs`).
fn production_seed() -> Rumors<u8> {
    let mut rng = ChaChaRng::seed_from_u64(0xe9f4_fc18_d119_7299);
    Peer::seed_rng(&mut rng).into_rumors()
}

/// Create a production-configured set containing `n` deterministic bytes.
fn seeded_with_messages(n: usize, seed: u64) -> Rumors<u8> {
    let rumors = production_seed();
    rumors
        .send_all(random_bytes(n, seed))
        .expect("flat test payloads are within any depth limit");
    rumors
}

/// Create a deterministic set and copy its live versions for redaction.
fn seeded_with_versions(n: usize, seed: u64) -> (Rumors<u8>, Vec<Version>) {
    let rumors = production_seed();
    rumors
        .send_all(random_bytes(n, seed))
        .expect("flat test payloads are within any depth limit");
    let versions = rumors.snapshot().versions().cloned().collect();
    (rumors, versions)
}

/// Generate `n` reproducible payload bytes.
fn random_bytes(n: usize, seed: u64) -> Vec<u8> {
    let mut bytes = vec![0; n];
    ChaChaRng::seed_from_u64(seed).fill_bytes(&mut bytes);
    bytes
}

/// Reorder versions reproducibly so redactions do not follow tree order.
fn shuffled_versions(mut versions: Vec<Version>, seed: u64) -> Vec<Version> {
    versions.shuffle(&mut ChaChaRng::seed_from_u64(seed));
    versions
}

criterion_group!(benches, bench_gossip_fixed, bench_gossip_latency);
criterion_main!(benches);
