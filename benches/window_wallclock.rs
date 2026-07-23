//! Wall-clock cross-check of the window trade-off's virtual-time model.
//!
//! Runs a small budget × divergence grid over the same delayed pipes as
//! `examples/window_tradeoff.rs`, but on a running clock, so pipe delays
//! burn real time under real timer scheduling. The virtual model predicts
//! each cell's time as `hops × delay` plus compute; criterion's mean for a
//! cell should land within a few percent of the generator's virtual figure
//! for the same shape. Compare by eye after `just bench window_wallclock`;
//! the grid is small because every wave here costs real milliseconds.

#[allow(dead_code)]
#[path = "support/latency.rs"]
mod latency;

use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};
use rand::rngs::SmallRng;
use rand::{RngCore, SeedableRng};
use rumors::{Peer, Protocol, Rumors};

/// One-way delay: small, so a serialized cell still finishes in tens of
/// milliseconds of real time.
const DELAY: Duration = Duration::from_millis(2);

/// Per-stream pipe buffering: roomy, matching the generator's
/// latency-only regime.
const LINK_CAPACITY: usize = 8 * 1024 * 1024;

/// Messages both peers share before the fork.
const COMMON: usize = 2_048;

/// The grid: three cells over two budgets.
///
/// The tight budget runs below and above its serialization knee (1k
/// pipelined, 10k serialized); the roomy budget runs the same large
/// divergence as the pipelined contrast at identical work.
const CELLS: &[(&str, usize, usize)] = &[
    ("256KiB-1k", 256 << 10, 1_000),
    ("256KiB-10k", 256 << 10, 10_000),
    ("16MiB-10k", 16 << 20, 10_000),
];

fn window_wallclock(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("window_wallclock");
    group.sample_size(10);
    for &(name, budget, divergence) in CELLS {
        group.bench_function(name, |bencher| {
            bencher.iter_batched(
                || diverged(budget, divergence),
                |(left, right)| {
                    let mut wire = latency::DelayedWire::new_wall_clock(LINK_CAPACITY, DELAY);
                    wire.round_trip(left, right)
                },
                criterion::BatchSize::PerIteration,
            );
        });
    }
    group.finish();
}

/// Two peers with a shared prefix, diverged by `divergent` messages each.
fn diverged(budget: usize, divergent: usize) -> (Rumors<u64>, Rumors<u64>) {
    let left = Peer::seed().sync_memory_budget(budget).into_rumors();
    let mut rng = SmallRng::seed_from_u64(0x0b05_2026_0d0c_0002);
    send_random(&left, COMMON, &mut rng);

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

criterion_group!(benches, window_wallclock);
criterion_main!(benches);
