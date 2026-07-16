//! Fixed-size over-the-wire gossip sweeps.
//!
//! Every fixture starts with a total universe size of `N = 10_000` possible
//! actions and varies only where the work lands: shared pre-fork insertions,
//! post-fork divergent insertions, or post-fork redactions. Each benchmark
//! drives [`Rumors::gossip`] over one persistent in-memory duplex, so the timed
//! body pays for the gossip session rather than transport allocation.
//!
//! Each of the four Criterion groups measures [`Protocol::V2`] on the same
//! fixtures — and [`Protocol::V1`] alongside it when the `protocol-v1`
//! feature is enabled (`cargo bench --features protocol-v1 gossip_fixed`),
//! which is the comparative-measurement path that feature exists for:
//!
//! - `gossip_fixed_bidir_insertions`: total post-fork insertions `I`.
//! - `gossip_fixed_bidir_redactions`: total post-fork redactions `R`.
//! - `gossip_fixed_unilateral_insertions`: one-side post-fork insertions `I`.
//! - `gossip_fixed_unilateral_redactions`: one-side post-fork redactions `R`.

use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::{RngCore, SeedableRng};
use rumors::{Key, Peer, Protocol, Rumors};

// The shared grid module exposes a superset of helpers; this bench only needs
// its sample-size policy so fixed-N runs line up with the existing benches.
#[allow(dead_code)]
#[path = "support/grid.rs"]
mod grid;

const N: usize = 10_000;
const INSERT_STEP: usize = 500;
const REDACT_STEP: usize = 250;

/// The protocols under measurement: V1 joins the sweep only when built in.
#[cfg(feature = "protocol-v1")]
const PROTOCOLS: [Protocol; 2] = [Protocol::V1, Protocol::V2];
#[cfg(not(feature = "protocol-v1"))]
const PROTOCOLS: [Protocol; 1] = [Protocol::V2];

#[derive(Clone, Copy)]
enum Scenario {
    BidirInsertions,
    BidirRedactions,
    UnilateralInsertions,
    UnilateralRedactions,
}

impl Scenario {
    fn group_name(self) -> &'static str {
        match self {
            Scenario::BidirInsertions => "gossip_fixed_bidir_insertions",
            Scenario::BidirRedactions => "gossip_fixed_bidir_redactions",
            Scenario::UnilateralInsertions => "gossip_fixed_unilateral_insertions",
            Scenario::UnilateralRedactions => "gossip_fixed_unilateral_redactions",
        }
    }

    fn max_param(self) -> usize {
        match self {
            Scenario::BidirInsertions | Scenario::UnilateralInsertions => N,
            Scenario::BidirRedactions | Scenario::UnilateralRedactions => N / 2,
        }
    }

    fn step(self) -> usize {
        match self {
            Scenario::BidirInsertions | Scenario::UnilateralInsertions => INSERT_STEP,
            Scenario::BidirRedactions | Scenario::UnilateralRedactions => REDACT_STEP,
        }
    }

    fn build(self, protocol: Protocol, param: usize) -> (Rumors<u8>, Rumors<u8>) {
        match self {
            Scenario::BidirInsertions => build_bidir_insertions(protocol, param),
            Scenario::BidirRedactions => build_bidir_redactions(protocol, param),
            Scenario::UnilateralInsertions => build_unilateral_insertions(protocol, param),
            Scenario::UnilateralRedactions => build_unilateral_redactions(protocol, param),
        }
    }
}

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
            group.throughput(Throughput::Elements(param as u64));
            for protocol in PROTOCOLS {
                group.bench_function(BenchmarkId::new(format!("{protocol:?}"), param), |b| {
                    b.iter_batched(
                        || warmed(scenario.build(protocol, param)),
                        |(left, right)| black_box(wire.round_trip(left, right)),
                        BatchSize::PerIteration,
                    )
                });
            }
        }

        group.finish();
    }
}

fn build_bidir_insertions(protocol: Protocol, total_insertions: usize) -> (Rumors<u8>, Rumors<u8>) {
    assert!(total_insertions <= N);
    assert_eq!(total_insertions % 2, 0);

    let left = seeded_with_messages(protocol, N - total_insertions, 0x1189_2d1a_c54f_a94d);
    let right = grid::wire::bootstrap_fork(&left, protocol);
    let per_side = total_insertions / 2;

    send_all(
        &left,
        random_bytes(per_side, 0x7a27_9f20_6c8b_d141 ^ total_insertions as u64),
    );
    send_all(
        &right,
        random_bytes(per_side, 0xc436_90ed_83f6_5b55 ^ total_insertions as u64),
    );

    (left, right)
}

fn build_unilateral_insertions(
    protocol: Protocol,
    total_insertions: usize,
) -> (Rumors<u8>, Rumors<u8>) {
    assert!(total_insertions <= N);

    let left = seeded_with_messages(protocol, N - total_insertions, 0x70e4_a5b8_cce0_25da);
    let right = grid::wire::bootstrap_fork(&left, protocol);

    send_all(
        &left,
        random_bytes(
            total_insertions,
            0xf193_d419_8d66_85d1 ^ total_insertions as u64,
        ),
    );

    (left, right)
}

fn build_bidir_redactions(protocol: Protocol, total_redactions: usize) -> (Rumors<u8>, Rumors<u8>) {
    assert!(total_redactions <= N / 2);
    assert_eq!(total_redactions % 2, 0);

    let (left, keys) = seeded_with_keys(protocol, N, 0xc786_a046_6b7d_c9d3);
    let right = grid::wire::bootstrap_fork(&left, protocol);
    let shuffled = shuffled_keys(keys, 0x84f6_7932_1265_9eec ^ total_redactions as u64);
    let per_side = total_redactions / 2;

    redact_all(&left, &shuffled[..per_side]);
    redact_all(&right, &shuffled[per_side..total_redactions]);

    (left, right)
}

fn build_unilateral_redactions(
    protocol: Protocol,
    total_redactions: usize,
) -> (Rumors<u8>, Rumors<u8>) {
    assert!(total_redactions <= N / 2);

    let (left, keys) = seeded_with_keys(protocol, N, 0x2526_34f4_918f_e1c7);
    let right = grid::wire::bootstrap_fork(&left, protocol);
    let shuffled = shuffled_keys(keys, 0xd4f9_f46b_3c09_1d60 ^ total_redactions as u64);

    redact_all(&left, &shuffled[..total_redactions]);

    (left, right)
}

fn send_all(rumors: &Rumors<u8>, messages: Vec<u8>) {
    let mut batch = rumors.batch();
    for message in messages {
        batch.send(message);
    }
}

fn redact_all(rumors: &Rumors<u8>, keys: &[Key]) {
    let mut batch = rumors.batch();
    for key in keys {
        batch.redact(*key);
    }
}

fn seeded_with_messages(protocol: Protocol, n: usize, seed: u64) -> Rumors<u8> {
    let rumors = Peer::seed().protocol(protocol).into_rumors();
    send_all(&rumors, random_bytes(n, seed));
    rumors
}

fn seeded_with_keys(protocol: Protocol, n: usize, seed: u64) -> (Rumors<u8>, Vec<Key>) {
    let rumors = Peer::seed().protocol(protocol).into_rumors();
    send_all(&rumors, random_bytes(n, seed));
    let keys = rumors.snapshot().iter().map(|(k, _, _)| k).collect();
    (rumors, keys)
}

fn warmed((left, right): (Rumors<u8>, Rumors<u8>)) -> (Rumors<u8>, Rumors<u8>) {
    left.warm_caches();
    right.warm_caches();
    (left, right)
}

fn random_bytes(n: usize, seed: u64) -> Vec<u8> {
    let mut bytes = vec![0; n];
    SmallRng::seed_from_u64(seed).fill_bytes(&mut bytes);
    bytes
}

fn shuffled_keys(mut keys: Vec<Key>, seed: u64) -> Vec<Key> {
    keys.shuffle(&mut SmallRng::seed_from_u64(seed));
    keys
}

criterion_group!(benches, bench_gossip_fixed);
criterion_main!(benches);
