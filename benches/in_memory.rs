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
//! - `join_grid` / `join_identical`: reconcile two peers across the
//!   `(common, differing, redacted)` divergence grid (see [`grid`]).
//!   `gossip_grid.rs` runs the same grid over the wire for comparison.

use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rumors::sync::{Key, Known, Rumors};

// The shared grid module exposes a superset of helpers; each bench binary uses
// a subset, so the unused remainder is expected per-binary.
#[allow(dead_code)]
#[path = "support/grid.rs"]
mod grid;

use grid::{SIZES, sample_size_for, units};

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

/// `join` across the divergence grid.
///
/// Throughput is reported against each cell's [`divergence`](grid::Cell::divergence)
/// — the `differing` messages plus `redacted` deletions a peer must reconcile —
/// not the shared tree size: `join`'s cost tracks the difference between the
/// peers, not how much they already agree on, so charging it against the shared
/// prefix would understate the small-delta cells (near-constant work, growing
/// prefix) by orders of magnitude.
///
/// The identical corner (`differing = 0, redacted = 0`) reconciles nothing, so
/// it lands in a separate `join_identical` group that reports plain latency
/// rather than a meaningless zero-element throughput.
///
/// Each fixture is built *and its lazy hash/ceiling/floor memos warmed* in the
/// untimed setup, so the timed body measures `join`'s own traversal — the hash
/// short-circuit over the shared prefix, plus reconciling the divergence — and
/// not the one-time cost of first computing those memos (which, cold, would
/// charge an `O(n)` rehash of the whole shared subtree to every `join`).
fn bench_join(c: &mut Criterion) {
    // Cells that transfer something: throughput is charged against divergence.
    let mut grid_group = c.benchmark_group("join_grid");
    for cell in grid::cells().filter(|cell| cell.divergence() > 0) {
        grid_group.sample_size(sample_size_for(cell.build_magnitude()));
        grid_group.throughput(Throughput::Elements(cell.divergence()));
        grid_group.bench_function(BenchmarkId::from_parameter(cell.id()), |b| {
            b.iter_batched(
                || warmed(cell),
                |(mut left, right)| {
                    left.join(right).unwrap();
                    left
                },
                BatchSize::PerIteration,
            )
        });
    }
    grid_group.finish();

    // The identical corner reconciles nothing; report latency only.
    let mut identical_group = c.benchmark_group("join_identical");
    for cell in grid::cells().filter(|cell| cell.divergence() == 0) {
        identical_group.sample_size(sample_size_for(cell.build_magnitude()));
        identical_group.bench_function(BenchmarkId::from_parameter(cell.id()), |b| {
            b.iter_batched(
                || warmed(cell),
                |(mut left, right)| {
                    left.join(right).unwrap();
                    left
                },
                BatchSize::PerIteration,
            )
        });
    }
    identical_group.finish();
}

/// Build a cell's peers and warm both trees' lazy memos in untimed setup, so
/// the timed body is not charged for first-touch memoization (see [`bench_join`]).
///
/// `join` now consumes a [`rumors`](Known::rumors) snapshot, so `right` is
/// snapshotted here in the untimed setup; the timed body stays a bare
/// `left.join(right)`.
fn warmed(cell: grid::Cell) -> (Known<()>, Known<(), Rumors>) {
    let (left, right) = grid::build(cell);
    left.warm_caches();
    right.warm_caches();
    (left, right.rumors())
}

criterion_group!(benches, bench_message, bench_iter, bench_redact, bench_join);
criterion_main!(benches);
