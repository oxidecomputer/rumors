//! Over-the-wire reconciliation benchmark: the divergence grid from
//! [`grid`], reconciled through [`Rumors::gossip`] over a simulated wire —
//! the protocol's full cost per cell: handshake, framing, and the
//! round-trip exchange chain over the divergence it must move.
//!
//! # The wire
//!
//! Two peers gossip concurrently on the two ends of one bounded in-memory
//! asynchronous connection, driven directly by [`pollster`] without a runtime.
//!
//! # Why a persistent connection
//!
//! A naive harness would allocate a fresh transport per Criterion iteration.
//! The
//! gossip exchange chain is statically bounded and self-delimiting (it closes
//! on its own counter, never on EOF), so a completed session leaves the duplex
//! at its next boundary. [`Wire::new`] therefore allocates once for all cells.

use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rumors::Rumors;

// The shared grid module exposes a superset of helpers; each bench binary uses
// a subset, so the unused remainder is expected per-binary.
#[allow(dead_code)]
#[path = "support/grid.rs"]
mod grid;

use grid::sample_size_for;

/// `gossip` across the divergence grid.
///
/// The grid and throughput accounting mirror `in_memory.rs`'s `join` bench
/// exactly (see [`grid`]); the only difference is that each pair reconciles
/// over the wire via [`Wire::round_trip`] rather than in-process. The identical
/// corner reports latency in a separate `gossip_identical` group.
///
/// Both peers' lazy memos are warmed in untimed setup, so the timed body
/// measures the steady-state protocol rather than first-touch memoization.
fn bench_gossip(c: &mut Criterion) {
    let mut wire = grid::wire::Wire::new();

    // Cells that transfer something: throughput is charged against divergence.
    let mut grid_group = c.benchmark_group("gossip_grid");
    for cell in grid::cells().filter(|cell| cell.divergence() > 0) {
        grid_group.sample_size(sample_size_for(cell.build_magnitude()));
        grid_group.throughput(Throughput::Elements(cell.divergence()));
        grid_group.bench_function(BenchmarkId::from_parameter(cell.id()), |b| {
            b.iter_batched(
                || warmed(cell),
                |(left, right)| black_box(wire.round_trip(left, right)),
                BatchSize::PerIteration,
            )
        });
    }
    grid_group.finish();

    // The identical corner reconciles nothing; report latency only.
    let mut identical_group = c.benchmark_group("gossip_identical");
    for cell in grid::cells().filter(|cell| cell.divergence() == 0) {
        identical_group.sample_size(sample_size_for(cell.build_magnitude()));
        identical_group.bench_function(BenchmarkId::from_parameter(cell.id()), |b| {
            b.iter_batched(
                || warmed(cell),
                |(left, right)| black_box(wire.round_trip(left, right)),
                BatchSize::PerIteration,
            )
        });
    }
    identical_group.finish();
}

/// Build a cell's peers and warm both trees' lazy memos in untimed setup.
fn warmed(cell: grid::Cell) -> (Rumors<()>, Rumors<()>) {
    let (left, right) = grid::build(cell);
    left.warm_caches();
    right.warm_caches();
    (left, right)
}

criterion_group!(benches, bench_gossip);
criterion_main!(benches);
