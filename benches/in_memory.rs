// `criterion_group!` generates a public registrar with no documentation.
#![allow(missing_docs)]

//! In-memory benchmarks for the public single-set surface.
//!
//! These cover the operations that mutate or read a rumor set entirely in
//! memory: everything except [`gossip`](rumors::Rumors::gossip), which
//! serializes onto the wire (see `gossip_grid.rs` and `gossip_fixed.rs`).
//! The message payload is `()`, whose encoding is one CBOR null byte, so each
//! measurement reflects the tree / clock / hashing work rather than the cost
//! of serializing a payload.
//!
//! Although [`rumors::Rumors`] also drives asynchronous synchronization, every
//! operation measured here completes synchronously and needs no runtime.
//!
//! # What's measured
//!
//! - `batch_insert`: build a rumor set of size N from empty in one batch
//!   commit (insert throughput, averaged over the 0..N growth curve).
//! - `iter`: a full live-message traversal of a size-N snapshot.
//! - `redact`: forget all N messages of a size-N set in one batch commit.
//! - `range_delta`: iterate the causal delta of size D above a checkpoint in a
//!   size-N set — the version-bounds pruning claim: cost should track D
//!   plus the pruning frontier, not N.
//! - `observer_replay`: drain a fresh [`UnorderedMessages`] observer over a
//!   size-N set (the genesis-replay pass every new observer pays).
//! - `observer_delta`: one observer pass over a size-D delta in a size-N
//!   set (the steady-state cost of an up-to-date observer catching up).
//! - `causal_replay` / `causal_delta`: the same two sweeps through a
//!   [`CausalMessages`] observer — the column-for-column price of causal
//!   delivery's rank-ordered staging over the plain passes.
//! - `get`: a point lookup by [`Version`] in a size-N set.

use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use futures::{FutureExt, Stream, StreamExt};
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use rumors::{Peer, Rumors, Version, causally};
use rumors_testkit::bench::grid;

use grid::{SIZES, sample_size_for, send_units};

/// Causal-delta sizes for the `range_delta` / `observer_delta` sweeps.
const DELTAS: &[usize] = &[1, 100, 10_000];

/// Create an empty set with a stable network identifier.
fn empty_set() -> Rumors<()> {
    let mut rng = ChaChaRng::seed_from_u64(0x8c6c_c850_62c7_c5b6);
    Peer::seed_rng(&mut rng).into_rumors()
}

/// Build a deterministic rumor set holding `n` messages.
fn build(n: usize) -> Rumors<()> {
    let rumors = empty_set();
    send_units(&rumors, n);
    rumors
}

/// Copy the live versions from `rumors` for a consuming benchmark.
fn versions_of(rumors: &Rumors<()>) -> Vec<Version> {
    rumors.snapshot().versions().cloned().collect()
}

/// Drain everything `observer` has pending, without blocking, returning how
/// many messages were yielded.
fn drain<S: Stream + Unpin>(observer: &mut S) -> usize {
    let mut count = 0usize;
    while let Some(Some(item)) = observer.next().now_or_never() {
        black_box(item);
        count += 1;
    }
    count
}

/// `batch_insert`: insert N messages into an empty set in one batch commit.
///
/// Set construction and destruction are untimed. The measured body contains
/// one `send_all` call that inserts all N messages into an empty set.
fn bench_batch_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_insert");
    for &n in SIZES {
        group.sample_size(sample_size_for(n));
        group.throughput(Throughput::Elements(n as u64));
        group.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter_batched(
                empty_set,
                |rumors| {
                    send_units(&rumors, black_box(n));
                    rumors
                },
                BatchSize::PerIteration,
            )
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
        let rumors = build(n);
        let snapshot = rumors.snapshot();
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

/// `redact`: forget all N messages of a size-N set in a single batch commit.
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
                || {
                    let rumors = build(n);
                    let versions = versions_of(&rumors);
                    (rumors, versions)
                },
                |(rumors, versions)| {
                    rumors.redact_all(versions.iter().map(black_box));
                    rumors
                },
                BatchSize::PerIteration,
            )
        });
    }
    group.finish();
}

/// A size-`n` set whose last `delta` messages sit above the returned checkpoint.
fn build_with_checkpoint(n: usize, delta: usize) -> (Rumors<()>, Version) {
    let rumors = empty_set();
    send_units(&rumors, n - delta);
    let checkpoint = rumors.snapshot().latest().clone();
    send_units(&rumors, delta);
    (rumors, checkpoint)
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
            let (rumors, checkpoint) = build_with_checkpoint(n, delta);
            let snapshot = rumors.snapshot();
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
        let rumors = build(n);
        group.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter(|| {
                let mut observer = rumors.unordered_messages();
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
            let (rumors, checkpoint) = build_with_checkpoint(n, delta);
            group.bench_function(
                BenchmarkId::from_parameter(format!("n={n},delta={delta}")),
                |b| {
                    b.iter(|| {
                        let mut observer = rumors.unordered_messages_since(checkpoint.clone());
                        black_box(drain(&mut observer))
                    })
                },
            );
        }
    }
    group.finish();
}

/// `causal_replay`: a fresh causal observer's genesis pass over a size-N
/// set: the price of causal delivery on top of [`bench_observer_replay`]'s
/// plain pass.
///
/// Reordering must buffer, so the causal pass stages every leaf in a
/// rank-ordered map before the first item comes out; comparing the two
/// groups column-for-column is the cost of that staging.
fn bench_causal_replay(c: &mut Criterion) {
    let mut group = c.benchmark_group("causal_replay");
    for &n in SIZES {
        group.sample_size(sample_size_for(n));
        group.throughput(Throughput::Elements(n as u64));
        let rumors = build(n);
        group.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter(|| {
                let mut observer = rumors.causal_messages();
                black_box(drain(&mut observer))
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
            let (rumors, checkpoint) = build_with_checkpoint(n, delta);
            group.bench_function(
                BenchmarkId::from_parameter(format!("n={n},delta={delta}")),
                |b| {
                    b.iter(|| {
                        let mut observer = rumors.causal_messages_since(checkpoint.clone());
                        black_box(drain(&mut observer))
                    })
                },
            );
        }
    }
    group.finish();
}

/// `get`: a point lookup by version — one `O(depth)` descent, never a scan.
///
/// Lookups go through [`snapshot`](Rumors::snapshot), so the timed body pays
/// for acquiring the root handle plus the descent: the whole per-call cost
/// of a point read through the public API.
fn bench_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("get");
    for &n in SIZES {
        group.sample_size(sample_size_for(n));
        let rumors = build(n);
        let versions = versions_of(&rumors);
        // A fixed version from the middle of the stable iteration order;
        // any live version costs the same depth-bounded descent.
        let version = versions[versions.len() / 2].clone();
        group.bench_function(BenchmarkId::from_parameter(n), |b| {
            b.iter(|| {
                let snapshot = rumors.snapshot();
                black_box(snapshot.get(black_box(&version)));
            })
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
