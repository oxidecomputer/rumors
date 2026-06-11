//! Fixed-size over-the-wire gossip sweeps.
//!
//! Every fixture starts with a total universe size of `N = 10_000` possible
//! actions and varies only where the work lands: shared pre-fork insertions,
//! post-fork divergent insertions, or post-fork redactions. Each benchmark
//! drives [`Rumors::gossip`] over persistent in-memory pipes, so the timed body
//! pays for the gossip session rather than pipe allocation or thread spawn.
//!
//! The four Criterion groups are:
//!
//! - `gossip_fixed_bidir_insertions`: total post-fork insertions `I`.
//! - `gossip_fixed_bidir_redactions`: total post-fork redactions `R`.
//! - `gossip_fixed_unilateral_insertions`: one-side post-fork insertions `I`.
//! - `gossip_fixed_unilateral_redactions`: one-side post-fork redactions `R`.

use std::hint::black_box;
use std::io::{PipeReader, PipeWriter, pipe};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::{self, JoinHandle};

use borsh::{BorshDeserialize, BorshSerialize};

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::{RngCore, SeedableRng};
use rumors::sync::{Key, Peer, Rumors};

// The shared grid module exposes a superset of helpers; this bench only needs
// its sample-size policy so fixed-N runs line up with the existing benches.
#[allow(dead_code)]
#[path = "support/grid.rs"]
mod grid;

const N: usize = 10_000;
const INSERT_STEP: usize = 500;
const REDACT_STEP: usize = 250;

/// Mint a genuine party-disjoint originator that inherits `parent`'s content.
///
/// A peer that will independently `send`/`redact` (as both sides of every
/// fixture here do) is minted by serving a bootstrap from `parent` over a
/// pair of pipes: the newcomer pulls `parent`'s whole tree through the
/// ordinary mirror descent and is handed a fresh disjoint party, forked in
/// the same critical section that snapshots the served tree. The fixtures
/// only need the data plane, so the lifecycle handle is collapsed to
/// [`Rumors`] right away.
fn bootstrap_fork<T>(parent: &mut Rumors<T>) -> Rumors<T>
where
    T: BorshSerialize + BorshDeserialize + Clone + Send + Sync + 'static,
{
    let (mut p2n_r, mut p2n_w) = pipe().expect("pipe parent→newcomer");
    let (mut n2p_r, mut n2p_w) = pipe().expect("pipe newcomer→parent");
    thread::scope(|s| {
        let newcomer = s.spawn(move || {
            Peer::<T>::bootstrap(&mut p2n_r, &mut n2p_w)
                .expect("bootstrap newcomer")
                .expect("provider served bootstrap")
                .into_rumors()
        });
        parent
            .gossip(&mut n2p_r, &mut p2n_w)
            .expect("serve bootstrap");
        newcomer.join().expect("join bootstrap thread")
    })
}

/// A reusable in-memory "wire": two OS pipes plus a persistent worker thread
/// driving peer B.
struct Wire {
    a_read: PipeReader,
    a_write: PipeWriter,
    work: Option<Sender<Rumors<u8>>>,
    done: Receiver<Rumors<u8>>,
    worker: Option<JoinHandle<()>>,
}

impl Wire {
    fn new() -> Self {
        let (a_to_b_r, a_to_b_w) = pipe().expect("pipe a_to_b");
        let (b_to_a_r, b_to_a_w) = pipe().expect("pipe b_to_a");
        let (work_tx, work_rx) = channel::<Rumors<u8>>();
        let (done_tx, done_rx) = channel::<Rumors<u8>>();

        let worker = thread::spawn(move || {
            let mut read = a_to_b_r;
            let mut write = b_to_a_w;
            while let Ok(mut b) = work_rx.recv() {
                b.gossip(&mut read, &mut write).expect("worker peer gossip");
                if done_tx.send(b).is_err() {
                    break;
                }
            }
        });

        Wire {
            a_read: b_to_a_r,
            a_write: a_to_b_w,
            work: Some(work_tx),
            done: done_rx,
            worker: Some(worker),
        }
    }

    fn round_trip(&mut self, mut a: Rumors<u8>, b: Rumors<u8>) -> (Rumors<u8>, Rumors<u8>) {
        self.work
            .as_ref()
            .expect("worker still running")
            .send(b)
            .expect("hand peer B to worker");
        a.gossip(&mut self.a_read, &mut self.a_write)
            .expect("peer A gossip");
        let b_out = self.done.recv().expect("recv reconciled peer B");
        (a, b_out)
    }
}

impl Drop for Wire {
    fn drop(&mut self) {
        self.work.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

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

    fn build(self, param: usize) -> (Rumors<u8>, Rumors<u8>) {
        match self {
            Scenario::BidirInsertions => build_bidir_insertions(param),
            Scenario::BidirRedactions => build_bidir_redactions(param),
            Scenario::UnilateralInsertions => build_unilateral_insertions(param),
            Scenario::UnilateralRedactions => build_unilateral_redactions(param),
        }
    }
}

fn bench_gossip_fixed(c: &mut Criterion) {
    let mut wire = Wire::new();

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
            group.bench_function(BenchmarkId::from_parameter(param), |b| {
                b.iter_batched(
                    || warmed(scenario.build(param)),
                    |(left, right)| black_box(wire.round_trip(left, right)),
                    BatchSize::PerIteration,
                )
            });
        }

        group.finish();
    }
}

fn build_bidir_insertions(total_insertions: usize) -> (Rumors<u8>, Rumors<u8>) {
    assert!(total_insertions <= N);
    assert_eq!(total_insertions % 2, 0);

    let mut left = seeded_with_messages(N - total_insertions, 0x1189_2d1a_c54f_a94d);
    let right = bootstrap_fork(&mut left);
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

fn build_unilateral_insertions(total_insertions: usize) -> (Rumors<u8>, Rumors<u8>) {
    assert!(total_insertions <= N);

    let mut left = seeded_with_messages(N - total_insertions, 0x70e4_a5b8_cce0_25da);
    let right = bootstrap_fork(&mut left);

    send_all(
        &left,
        random_bytes(
            total_insertions,
            0xf193_d419_8d66_85d1 ^ total_insertions as u64,
        ),
    );

    (left, right)
}

fn build_bidir_redactions(total_redactions: usize) -> (Rumors<u8>, Rumors<u8>) {
    assert!(total_redactions <= N / 2);
    assert_eq!(total_redactions % 2, 0);

    let (mut left, keys) = seeded_with_keys(N, 0xc786_a046_6b7d_c9d3);
    let right = bootstrap_fork(&mut left);
    let shuffled = shuffled_keys(keys, 0x84f6_7932_1265_9eec ^ total_redactions as u64);
    let per_side = total_redactions / 2;

    redact_all(&left, &shuffled[..per_side]);
    redact_all(&right, &shuffled[per_side..total_redactions]);

    (left, right)
}

fn build_unilateral_redactions(total_redactions: usize) -> (Rumors<u8>, Rumors<u8>) {
    assert!(total_redactions <= N / 2);

    let (mut left, keys) = seeded_with_keys(N, 0x2526_34f4_918f_e1c7);
    let right = bootstrap_fork(&mut left);
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

fn seeded_with_messages(n: usize, seed: u64) -> Rumors<u8> {
    let rumors = Peer::seed().into_rumors();
    send_all(&rumors, random_bytes(n, seed));
    rumors
}

fn seeded_with_keys(n: usize, seed: u64) -> (Rumors<u8>, Vec<Key>) {
    let rumors = Peer::seed().into_rumors();
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
