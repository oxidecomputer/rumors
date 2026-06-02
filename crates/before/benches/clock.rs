//! `Clock` benchmarks: the optimized implementation against the naive recursive
//! oracle, on the same randomized clocks (see `common`).

use before::Clock;
use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use rand::rngs::StdRng;
use rand::SeedableRng;

mod common;
use common::{SEED, SIZES};

fn rng(salt: u64) -> StdRng {
    StdRng::seed_from_u64(SEED.wrapping_add(salt))
}

/// `tick`: advance this clock's own component. Destructive; fresh per iteration.
fn bench_tick(c: &mut Criterion) {
    let mut g = c.benchmark_group("clock/tick");
    let mut r = rng(1);
    for &n in SIZES {
        let plan = common::plan(&mut r, n, 1);
        let bytes = common::impl_clocks(&plan, 1).pop().unwrap().encode();
        let orc = common::oracle_clocks(&plan, 1).pop().unwrap();
        g.bench_with_input(BenchmarkId::new("before", n), &bytes, |b, bytes| {
            b.iter_batched(
                || Clock::decode(&bytes[..]).unwrap(),
                |mut c| {
                    c.tick();
                    black_box(c)
                },
                BatchSize::SmallInput,
            );
        });
        g.bench_with_input(BenchmarkId::new("oracle", n), &orc, |b, orc| {
            b.iter_batched(
                || orc.clone(),
                |mut c| {
                    c.tick();
                    black_box(c)
                },
                BatchSize::SmallInput,
            );
        });
    }
    g.finish();
}

/// `fork`: split off a child clock. Destructive; fresh per iteration.
fn bench_fork(c: &mut Criterion) {
    let mut g = c.benchmark_group("clock/fork");
    let mut r = rng(2);
    for &n in SIZES {
        let plan = common::plan(&mut r, n, 1);
        let bytes = common::impl_clocks(&plan, 1).pop().unwrap().encode();
        let orc = common::oracle_clocks(&plan, 1).pop().unwrap();
        g.bench_with_input(BenchmarkId::new("before", n), &bytes, |b, bytes| {
            b.iter_batched(
                || Clock::decode(&bytes[..]).unwrap(),
                |mut c| black_box(c.fork()),
                BatchSize::SmallInput,
            );
        });
        g.bench_with_input(BenchmarkId::new("oracle", n), &orc, |b, orc| {
            b.iter_batched(
                || orc.clone(),
                |mut c| black_box(c.fork()),
                BatchSize::SmallInput,
            );
        });
    }
    g.finish();
}

/// `join`: absorb a disjoint clock (party + history). Both operands consumed; fresh pair.
fn bench_join(c: &mut Criterion) {
    let mut g = c.benchmark_group("clock/join");
    let mut r = rng(3);
    for &n in SIZES {
        let plan = common::plan(&mut r, n, 2);
        let imp = common::impl_clocks(&plan, 2);
        let (ba, bb) = (imp[0].encode(), imp[1].encode());
        let orc = common::oracle_clocks(&plan, 2);
        let (oa, ob) = (orc[0].clone(), orc[1].clone());
        g.bench_with_input(BenchmarkId::new("before", n), &(ba, bb), |b, (ba, bb)| {
            b.iter_batched(
                || {
                    (
                        Clock::decode(&ba[..]).unwrap(),
                        Clock::decode(&bb[..]).unwrap(),
                    )
                },
                |(mut a, b)| black_box(a.join(b).is_ok()),
                BatchSize::SmallInput,
            );
        });
        g.bench_with_input(BenchmarkId::new("oracle", n), &(oa, ob), |b, (oa, ob)| {
            b.iter_batched(
                || (oa.clone(), ob.clone()),
                |(mut a, b)| black_box(a.join(b).is_ok()),
                BatchSize::SmallInput,
            );
        });
    }
    g.finish();
}

/// `sync`: reconcile two clocks (merge histories, re-split the merged party). Mutates
/// both; fresh pair per iteration.
fn bench_sync(c: &mut Criterion) {
    let mut g = c.benchmark_group("clock/sync");
    let mut r = rng(4);
    for &n in SIZES {
        let plan = common::plan(&mut r, n, 2);
        let imp = common::impl_clocks(&plan, 2);
        let (ba, bb) = (imp[0].encode(), imp[1].encode());
        let orc = common::oracle_clocks(&plan, 2);
        let (oa, ob) = (orc[0].clone(), orc[1].clone());
        g.bench_with_input(BenchmarkId::new("before", n), &(ba, bb), |b, (ba, bb)| {
            b.iter_batched(
                || {
                    (
                        Clock::decode(&ba[..]).unwrap(),
                        Clock::decode(&bb[..]).unwrap(),
                    )
                },
                |(mut a, mut b)| black_box(a.sync(&mut b).is_ok()),
                BatchSize::SmallInput,
            );
        });
        g.bench_with_input(BenchmarkId::new("oracle", n), &(oa, ob), |b, (oa, ob)| {
            b.iter_batched(
                || (oa.clone(), ob.clone()),
                |(mut a, mut b)| black_box(a.sync(&mut b).is_ok()),
                BatchSize::SmallInput,
            );
        });
    }
    g.finish();
}

/// `send`: tick, then snapshot the history to transmit. Destructive; fresh per iteration.
fn bench_send(c: &mut Criterion) {
    let mut g = c.benchmark_group("clock/send");
    let mut r = rng(5);
    for &n in SIZES {
        let plan = common::plan(&mut r, n, 1);
        let bytes = common::impl_clocks(&plan, 1).pop().unwrap().encode();
        let orc = common::oracle_clocks(&plan, 1).pop().unwrap();
        g.bench_with_input(BenchmarkId::new("before", n), &bytes, |b, bytes| {
            b.iter_batched(
                || Clock::decode(&bytes[..]).unwrap(),
                |mut c| black_box(c.send().clone()),
                BatchSize::SmallInput,
            );
        });
        g.bench_with_input(BenchmarkId::new("oracle", n), &orc, |b, orc| {
            b.iter_batched(
                || orc.clone(),
                |mut c| black_box(c.send()),
                BatchSize::SmallInput,
            );
        });
    }
    g.finish();
}

/// `receive`: merge an incoming message, then tick. The message is consumed; the clock is
/// mutated. Both operands fresh per iteration (the message clones cheaply in setup).
fn bench_receive(c: &mut Criterion) {
    let mut g = c.benchmark_group("clock/receive");
    let mut r = rng(6);
    for &n in SIZES {
        let plan = common::plan(&mut r, n, 2);
        let imp = common::impl_clocks(&plan, 2);
        let bytes = imp[0].encode();
        let msg = imp[1].version();
        let orc = common::oracle_clocks(&plan, 2);
        let oclock = orc[0].clone();
        let omsg = orc[1].version();
        g.bench_with_input(
            BenchmarkId::new("before", n),
            &(bytes, msg),
            |b, (bytes, msg)| {
                b.iter_batched(
                    || (Clock::decode(&bytes[..]).unwrap(), msg),
                    |(mut c, msg)| {
                        c.receive(msg);
                        black_box(c)
                    },
                    BatchSize::SmallInput,
                );
            },
        );
        g.bench_with_input(
            BenchmarkId::new("oracle", n),
            &(oclock, omsg),
            |b, (oc, msg)| {
                b.iter_batched(
                    || (oc.clone(), msg.clone()),
                    |(mut c, msg)| {
                        c.receive(msg);
                        black_box(c)
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    g.finish();
}

/// `encode`/`decode`: the packed byte codec. No oracle equivalent; impl alone.
fn bench_codec(c: &mut Criterion) {
    let mut g = c.benchmark_group("clock/codec");
    let mut r = rng(10);
    for &n in SIZES {
        let plan = common::plan(&mut r, n, 1);
        let clock = common::impl_clocks(&plan, 1).pop().unwrap();
        let bytes = clock.encode();
        g.bench_with_input(BenchmarkId::new("before/encode", n), &clock, |b, c| {
            b.iter(|| black_box(c.encode()));
        });
        g.bench_with_input(BenchmarkId::new("before/decode", n), &bytes, |b, bytes| {
            b.iter(|| black_box(Clock::decode(&bytes[..]).unwrap()));
        });
    }
    g.finish();
}

criterion_group!(
    benches,
    bench_tick,
    bench_fork,
    bench_join,
    bench_sync,
    bench_send,
    bench_receive,
    bench_codec
);
criterion_main!(benches);
