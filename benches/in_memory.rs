//! In-memory benchmarks for the public single-set surface.
//!
//! These cover the operations that mutate or read a rumor set entirely in
//! memory: everything except [`gossip`](rumors::Known::gossip), which
//! serializes onto the wire (see `gossip_grid.rs` and `gossip_fixed.rs`).
//! The message payload is `()`, which borsh-encodes to zero bytes, so each
//! measurement reflects the tree / clock / hashing work rather than the cost
//! of serializing a payload.
//!
//! The handles here are the asynchronous [`rumors::Known`] and its
//! [`Messages`] observer: every operation measured is synchronous on that
//! surface (batches commit on drop; observer drains are polled without an
//! executor via `now_or_never`), so no runtime is involved.
//!
//! # Fixture discipline
//!
//! Inserting a message ticks an Interval Tree Clock party, and `before`
//! documents that repeatedly [`fork`](before::Party::fork)ing *the same*
//! party deepens its id tree linearly (worse memory and per-op cost). To
//! keep that out of the measurements, every fixture is rebuilt from a fresh
//! [`Known::seed`](rumors::Known::seed) in untimed setup: no party
//! accumulates depth across Criterion iterations.
//!
//! # What's measured
//!
//! - `batch_insert`: build a rumor set of size N from empty in one batch
//!   commit (insert throughput, averaged over the 0..N growth curve).
//! - `iter`: a full live-message traversal of a size-N snapshot.
//! - `redact`: forget all N keys of a size-N set in one batch commit.
//! - `range_delta`: iterate the causal delta of size D above a checkpoint in a
//!   size-N set — the version-bounds pruning claim: cost should track D
//!   plus the pruning frontier, not N.
//! - `observer_replay`: drain a fresh [`Messages`] observer over a size-N
//!   set (the genesis-replay pass every new observer pays).
//! - `observer_delta`: one observer pass over a size-D delta in a size-N
//!   set (the steady-state cost of an up-to-date observer catching up).
//! - `causal_replay` / `causal_delta`: the same two sweeps through a
//!   [`CausalMessages`] observer — the column-for-column price of causal
//!   delivery's rank-ordered staging over the plain passes.
//! - `get`: a point lookup by [`Key`] in a size-N set.

use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use futures::FutureExt;
use rumors::{CausalMessages, Key, Known, Messages, causally};

// The shared grid module exposes a superset of helpers; each bench binary uses
// a subset, so the unused remainder is expected per-binary.
#[allow(dead_code)]
#[path = "support/grid.rs"]
mod grid;

use grid::{SIZES, sample_size_for};

/// Causal-delta sizes for the `range_delta` / `observer_delta` sweeps.
const DELTAS: &[usize] = &[1, 100, 10_000];

/// Commit `n` unit payloads to `known` as one batch.
fn send_units(known: &Known<()>, n: usize) {
    let mut batch = known.batch();
    for _ in 0..n {
        batch.send(());
    }
}

/// A freshly seeded rumor set holding `n` messages, paired with its live
/// keys (in the snapshot's stable order).
fn build(n: usize) -> (Known<()>, Vec<Key>) {
    let known: Known<()> = Known::seed();
    send_units(&known, n);
    let keys = known.snapshot().iter().map(|(k, _, _)| k).collect();
    (known, keys)
}

/// Drain everything `observer` has pending, without blocking, returning how
/// many messages were yielded.
fn drain(observer: &mut Messages<()>) -> usize {
    let mut count = 0usize;
    while let Some(Some(item)) = observer.borrow_next().now_or_never() {
        black_box(item);
        count += 1;
    }
    count
}

/// `batch_insert`: insert N messages into an empty set in one batch commit.
///
/// `b.iter` builds and drops one set per iteration, so peak memory stays at a
/// single tree even at N = 1M. The trivial `seed()` is inside the timed body,
/// but its cost is negligible against N inserts.
fn bench_batch_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_insert");
    for &n in SIZES {
        group.sample_size(sample_size_for(n));
        group.throughput(Throughput::Elements(n as u64));
        group.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter(|| {
                let known: Known<()> = Known::seed();
                send_units(&known, black_box(n));
                known
            })
        });
    }
    group.finish();
}

/// `iter`: traverse every live message in a size-N snapshot.
///
/// The set and snapshot are built once (untimed) and shared across
/// iterations; the snapshot is a cheap copy-on-write view, so this measures
/// the walk itself.
fn bench_iter(c: &mut Criterion) {
    let mut group = c.benchmark_group("iter");
    for &n in SIZES {
        group.sample_size(sample_size_for(n));
        group.throughput(Throughput::Elements(n as u64));
        let (known, _keys) = build(n);
        let snapshot = known.snapshot();
        group.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter(|| {
                let mut count = 0usize;
                for entry in snapshot.iter() {
                    black_box(entry);
                    count += 1;
                }
                black_box(count)
            })
        });
    }
    group.finish();
}

/// `redact`: forget all N keys of a size-N set in a single batch commit.
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
                |(known, keys)| {
                    let mut batch = known.batch();
                    for key in keys {
                        batch.redact(black_box(key));
                    }
                    drop(batch);
                    known
                },
                BatchSize::PerIteration,
            )
        });
    }
    group.finish();
}

/// A size-`n` set whose last `delta` messages sit above the returned checkpoint,
/// with the tree's lazy memos warmed so the timed body measures the
/// version-bounded walk itself rather than first-touch memoization.
fn build_with_checkpoint(n: usize, delta: usize) -> (Known<()>, rumors::Version) {
    let known: Known<()> = Known::seed();
    send_units(&known, n - delta);
    let checkpoint = known.latest();
    send_units(&known, delta);
    known.warm_caches();
    (known, checkpoint)
}

/// `range_delta`: iterate the causal delta above a checkpoint.
///
/// Throughput is charged against the delta, not the set size: the
/// memoized version bounds let the walk prune everything the checkpoint
/// dominates, so a small delta against a large snapshot should cost the
/// delta plus the pruning frontier, not the tree. Comparing one column
/// (fixed delta) across set sizes is exactly that claim under measurement.
fn bench_range_delta(c: &mut Criterion) {
    let mut group = c.benchmark_group("range_delta");
    for &n in SIZES {
        for &delta in DELTAS {
            if delta > n {
                continue;
            }
            group.sample_size(sample_size_for(n));
            group.throughput(Throughput::Elements(delta as u64));
            let (known, checkpoint) = build_with_checkpoint(n, delta);
            let snapshot = known.snapshot();
            group.bench_function(
                BenchmarkId::from_parameter(format!("n={n},delta={delta}")),
                |b| {
                    b.iter(|| {
                        let mut count = 0usize;
                        for entry in snapshot.range(causally::since(black_box(&checkpoint))) {
                            black_box(entry);
                            count += 1;
                        }
                        black_box(count)
                    })
                },
            );
        }
    }
    group.finish();
}

/// `observer_replay`: a fresh observer's genesis pass over a size-N set.
///
/// Subscribing is cheap; the cost is the first drain, which walks every
/// live leaf once. This is what a new consumer pays to catch up.
fn bench_observer_replay(c: &mut Criterion) {
    let mut group = c.benchmark_group("observer_replay");
    for &n in SIZES {
        group.sample_size(sample_size_for(n));
        group.throughput(Throughput::Elements(n as u64));
        let (known, _keys) = build(n);
        known.warm_caches();
        group.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter(|| {
                let mut observer = known.messages();
                black_box(drain(&mut observer))
            })
        });
    }
    group.finish();
}

/// `observer_delta`: one pass over a size-D delta in a size-N set.
///
/// The observer subscribes from the pre-delta checkpoint, so each iteration's
/// drain is the steady-state cost of an up-to-date observer catching up on
/// D new messages — like `range_delta`, this should track D, not N.
fn bench_observer_delta(c: &mut Criterion) {
    let mut group = c.benchmark_group("observer_delta");
    for &n in SIZES {
        for &delta in DELTAS {
            if delta > n {
                continue;
            }
            group.sample_size(sample_size_for(n));
            group.throughput(Throughput::Elements(delta as u64));
            let (known, checkpoint) = build_with_checkpoint(n, delta);
            group.bench_function(
                BenchmarkId::from_parameter(format!("n={n},delta={delta}")),
                |b| {
                    b.iter(|| {
                        let mut observer = known.messages_from(checkpoint.clone());
                        black_box(drain(&mut observer))
                    })
                },
            );
        }
    }
    group.finish();
}

/// Drain everything `observer` has staged, without blocking, returning how
/// many messages were yielded: [`drain`]'s twin for the causal face.
fn drain_causal(observer: &mut CausalMessages<()>) -> usize {
    let mut count = 0usize;
    while let Some(Some(item)) = observer.borrow_next().now_or_never() {
        black_box(item);
        count += 1;
    }
    count
}

/// `causal_replay`: a fresh causal observer's genesis pass over a size-N
/// set: the price of causal delivery on top of [`bench_observer_replay`]'s
/// plain pass. Reordering must buffer, so the causal pass stages every leaf
/// in a rank-ordered map before the first item comes out; comparing the two
/// groups column-for-column is the cost of that staging.
fn bench_causal_replay(c: &mut Criterion) {
    let mut group = c.benchmark_group("causal_replay");
    for &n in SIZES {
        group.sample_size(sample_size_for(n));
        group.throughput(Throughput::Elements(n as u64));
        let (known, _keys) = build(n);
        known.warm_caches();
        group.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter(|| {
                let mut observer = known.causal_messages();
                black_box(drain_causal(&mut observer))
            })
        });
    }
    group.finish();
}

/// `causal_delta`: one causal pass over a size-D delta in a size-N set —
/// the steady-state twin of [`bench_observer_delta`], staging only the
/// delta, so this too should track D, not N.
fn bench_causal_delta(c: &mut Criterion) {
    let mut group = c.benchmark_group("causal_delta");
    for &n in SIZES {
        for &delta in DELTAS {
            if delta > n {
                continue;
            }
            group.sample_size(sample_size_for(n));
            group.throughput(Throughput::Elements(delta as u64));
            let (known, checkpoint) = build_with_checkpoint(n, delta);
            group.bench_function(
                BenchmarkId::from_parameter(format!("n={n},delta={delta}")),
                |b| {
                    b.iter(|| {
                        let mut observer = known.causal_messages_from(checkpoint.clone());
                        black_box(drain_causal(&mut observer))
                    })
                },
            );
        }
    }
    group.finish();
}

/// `get`: a point lookup by key — one `O(depth)` descent, never a scan.
fn bench_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("get");
    for &n in SIZES {
        group.sample_size(sample_size_for(n));
        let (known, keys) = build(n);
        known.warm_caches();
        // A fixed key from the middle of the stable iteration order; any
        // live key costs the same depth-bounded descent.
        let key = keys[keys.len() / 2];
        group.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter(|| black_box(known.get(black_box(&key))))
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_batch_insert,
    bench_iter,
    bench_redact,
    bench_range_delta,
    bench_observer_replay,
    bench_observer_delta,
    bench_causal_replay,
    bench_causal_delta,
    bench_get,
);
criterion_main!(benches);
