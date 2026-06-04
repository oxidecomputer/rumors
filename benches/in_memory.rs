//! In-memory benchmarks for the public `rumors::sync` surface.
//!
//! These cover the operations that mutate or read a rumor set entirely in
//! memory: everything except [`gossip`](rumors::sync::Known::gossip), which
//! serializes onto the wire. The message payload is `()`, which borsh-encodes
//! to zero bytes, so each measurement reflects the tree / clock / hashing work
//! rather than the cost of serializing a payload.
//!
//! # Fixture discipline
//!
//! Inserting a message ticks an Interval Tree Clock party, and `before`
//! documents that repeatedly [`fork`](before::Party::fork)ing *the same* party
//! deepens its id tree linearly (worse memory and per-op cost). To keep that
//! out of the measurements, every fixture is rebuilt from a fresh
//! [`Known::seed`](rumors::sync::Known::seed) in untimed setup and forked at
//! most once: no party accumulates depth across Criterion iterations.
//!
//! # What's measured
//!
//! - `message_insert`: build a rumor set of size N from empty (insert
//!   throughput, averaged over the 0..N growth curve).
//! - `iter`: a full live-message traversal of a size-N set.
//! - `redact`: forget all N keys of a size-N set in one call.
//! - `join_{disjoint,small_delta,identical}`: reconcile two peers whose
//!   histories differ by everything / a fixed handful / nothing.

use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rumors::sync::{Key, Known};

/// Live message counts spanning three orders of magnitude.
const SIZES: &[usize] = &[100, 10_000, 1_000_000];

/// Extra messages each side originates after the fork in the `small_delta`
/// merge shape: enough to exercise real reconciliation work without swamping
/// the shared-prefix comparison the benchmark is meant to isolate.
const DELTA: usize = 16;

/// Criterion samples per size. The million-message fixtures are expensive to
/// (re)build in setup, so the larger sizes take Criterion's floor of 10.
fn sample_size_for(n: usize) -> usize {
    match n {
        n if n >= 1_000_000 => 10,
        n if n >= 10_000 => 20,
        _ => 100,
    }
}

/// An iterator yielding `n` unit payloads.
fn units(n: usize) -> impl Iterator<Item = ()> + Send {
    std::iter::repeat_n((), n)
}

/// A function building the two peers for one `join` divergence shape.
type ShapeBuilder = fn(usize) -> (Known<()>, Known<()>);

/// A freshly seeded rumor set holding `n` messages, paired with the keys minted
/// for each, in insertion order.
fn build(n: usize) -> (Known<()>, Vec<Key>) {
    let mut known: Known<()> = Known::seed();
    let mut keys = Vec::with_capacity(n);
    known.message_then(units(n), |k, _, _| keys.push(k));
    (known, keys)
}

/// `message`: insert N messages into an empty set.
///
/// `b.iter` builds and drops one set per iteration, so peak memory stays at a
/// single tree even at N = 1M. The trivial `seed()` is inside the timed body,
/// but its cost is negligible against N inserts.
fn bench_message(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_insert");
    for &n in SIZES {
        group.sample_size(sample_size_for(n));
        group.throughput(Throughput::Elements(n as u64));
        group.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter(|| {
                let mut known: Known<()> = Known::seed();
                known.message(units(black_box(n)));
                known
            })
        });
    }
    group.finish();
}

/// `iter`: traverse every live message in a size-N set.
///
/// The set is built once (untimed) and shared across iterations; `iter` takes
/// `&self`, so no rebuild is needed.
fn bench_iter(c: &mut Criterion) {
    let mut group = c.benchmark_group("iter");
    for &n in SIZES {
        group.sample_size(sample_size_for(n));
        group.throughput(Throughput::Elements(n as u64));
        let (known, _keys) = build(n);
        group.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter(|| {
                let mut count = 0usize;
                for entry in known.iter() {
                    black_box(entry);
                    count += 1;
                }
                black_box(count)
            })
        });
    }
    group.finish();
}

/// `redact`: forget all N keys of a size-N set in a single call.
///
/// Each iteration redacts a fresh set built in untimed setup. `PerIteration`
/// keeps only one tree alive at a time, which matters at N = 1M.
fn bench_redact(c: &mut Criterion) {
    let mut group = c.benchmark_group("redact");
    for &n in SIZES {
        group.sample_size(sample_size_for(n));
        group.throughput(Throughput::Elements(n as u64));
        group.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter_batched(
                || build(n),
                |(mut known, keys)| {
                    known.redact(black_box(keys));
                    known
                },
                BatchSize::PerIteration,
            )
        });
    }
    group.finish();
}

/// Two peers, each holding N messages the other has never seen: worst-case
/// reconciliation, where `join` must transfer everything.
fn build_disjoint(n: usize) -> (Known<()>, Known<()>) {
    let mut left: Known<()> = Known::seed();
    let mut right = left.fork();
    left.message(units(n));
    right.message(units(n));
    (left, right)
}

/// Two peers sharing N messages (inserted before the fork), each then
/// originating [`DELTA`] of its own: steady-state gossip, where the shared
/// prefix should short-circuit by hash and only the deltas transfer.
fn build_small_delta(n: usize) -> (Known<()>, Known<()>) {
    let mut left: Known<()> = Known::seed();
    left.message(units(n));
    let mut right = left.fork();
    left.message(units(DELTA));
    right.message(units(DELTA));
    (left, right)
}

/// Two peers with identical histories: `join` compares two equal roots and
/// transfers nothing, measuring the structural-equality fast path.
fn build_identical(n: usize) -> (Known<()>, Known<()>) {
    let mut left: Known<()> = Known::seed();
    left.message(units(n));
    let right = left.fork();
    (left, right)
}

/// `join` across the three divergence shapes. `Elements(n)` reports against
/// the shared tree size in every shape, so the three groups are comparable at
/// equal N.
fn bench_join(c: &mut Criterion) {
    let shapes: [(&str, ShapeBuilder); 3] = [
        ("disjoint", build_disjoint),
        ("small_delta", build_small_delta),
        ("identical", build_identical),
    ];
    for (name, builder) in shapes {
        let mut group = c.benchmark_group(format!("join_{name}"));
        for &n in SIZES {
            group.sample_size(sample_size_for(n));
            group.throughput(Throughput::Elements(n as u64));
            group.bench_function(BenchmarkId::from_parameter(n), |b| {
                b.iter_batched(
                    || builder(n),
                    |(mut left, right)| {
                        left.join(right).unwrap();
                        left
                    },
                    BatchSize::PerIteration,
                )
            });
        }
        group.finish();
    }
}

criterion_group!(benches, bench_message, bench_iter, bench_redact, bench_join);
criterion_main!(benches);
