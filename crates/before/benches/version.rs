//! `Version` benchmarks: the optimized implementation against the naive
//! recursive oracle, on the same randomized event trees (see `common`).
//! Includes the batch-vs-single-op comparison that motivates the working form,
//! plus the impl-only byte codec.

use before::{Party, Version};
use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use rand::rngs::StdRng;
use rand::SeedableRng;

mod common;
use common::{SEED, SIZES};

fn rng(salt: u64) -> StdRng {
    StdRng::seed_from_u64(SEED.wrapping_add(salt))
}

/// A randomized version paired with the party that owns its id-space — the operand `tick`
/// needs. Returns the impl version's bytes (for fresh `decode`s) and the impl party, plus
/// the oracle version and party.
fn version_and_party(
    r: &mut StdRng,
    n: usize,
) -> (
    Vec<u8>,
    Party,
    before::oracle::Version,
    before::oracle::Party,
) {
    let plan = common::plan(r, n, 1);
    let (iparty, iversion) = common::impl_clocks(&plan, 1).pop().unwrap().into_parts();
    let (oparty, oversion) = common::oracle_clocks(&plan, 1).pop().unwrap().into_parts();
    (iversion.encode(), iparty, oversion, oparty)
}

/// `tick`: advance the owning party's component by one event. Destructive, so the version
/// is rebuilt fresh per iteration; the party is read-only and built once.
fn bench_tick(c: &mut Criterion) {
    let mut g = c.benchmark_group("version/tick");
    let mut r = rng(1);
    for &n in SIZES {
        let (bytes, iparty, oversion, oparty) = version_and_party(&mut r, n);
        g.bench_with_input(BenchmarkId::new("before", n), &bytes, |b, bytes| {
            b.iter_batched(
                || Version::decode(&bytes[..]).unwrap(),
                |mut v| {
                    v.tick(&iparty);
                    black_box(v)
                },
                BatchSize::SmallInput,
            );
        });
        g.bench_with_input(BenchmarkId::new("oracle", n), &oversion, |b, oversion| {
            b.iter_batched(
                || oversion.clone(),
                |mut v| {
                    v.tick(&oparty);
                    black_box(v)
                },
                BatchSize::SmallInput,
            );
        });
    }
    g.finish();
}

/// The headline of the working form: applying `k` ticks. The impl can `batch()` them
/// (one unpack/repack amortized over all `k`); without a batch each tick unpacks and
/// repacks on its own; the oracle has no working form and re-normalizes each tick. Tree
/// size is fixed; `k` is the axis.
fn bench_batch(c: &mut Criterion) {
    let mut g = c.benchmark_group("version/k_ticks");
    let mut r = rng(2);
    const TREE: usize = 64;
    let (bytes, iparty, oversion, oparty) = version_and_party(&mut r, TREE);
    for &k in &[1usize, 4, 16, 64] {
        g.bench_with_input(BenchmarkId::new("before/batched", k), &bytes, |b, bytes| {
            b.iter_batched(
                || Version::decode(&bytes[..]).unwrap(),
                |mut v| {
                    {
                        let mut batch = v.batch();
                        for _ in 0..k {
                            batch.tick(&iparty);
                        }
                    }
                    black_box(v)
                },
                BatchSize::SmallInput,
            );
        });
        g.bench_with_input(
            BenchmarkId::new("before/unbatched", k),
            &bytes,
            |b, bytes| {
                b.iter_batched(
                    || Version::decode(&bytes[..]).unwrap(),
                    |mut v| {
                        for _ in 0..k {
                            v.tick(&iparty);
                        }
                        black_box(v)
                    },
                    BatchSize::SmallInput,
                );
            },
        );
        g.bench_with_input(BenchmarkId::new("oracle", k), &oversion, |b, oversion| {
            b.iter_batched(
                || oversion.clone(),
                |mut v| {
                    for _ in 0..k {
                        v.tick(&oparty);
                    }
                    black_box(v)
                },
                BatchSize::SmallInput,
            );
        });
    }
    g.finish();
}

/// `|` (merge / least-upper-bound) of two histories. Both operands are consumed, so both
/// are rebuilt fresh per iteration.
fn bench_merge(c: &mut Criterion) {
    let mut g = c.benchmark_group("version/merge");
    let mut r = rng(3);
    for &n in SIZES {
        let plan = common::plan(&mut r, n, 2);
        let imp = common::impl_versions(&plan, 2);
        let (ba, bb) = (imp[0].encode(), imp[1].encode());
        let orc = common::oracle_versions(&plan, 2);
        let (oa, ob) = (orc[0].clone(), orc[1].clone());
        g.bench_with_input(BenchmarkId::new("before", n), &(ba, bb), |b, (ba, bb)| {
            b.iter_batched(
                || {
                    (
                        Version::decode(&ba[..]).unwrap(),
                        Version::decode(&bb[..]).unwrap(),
                    )
                },
                |(a, b)| black_box(a | b),
                BatchSize::SmallInput,
            );
        });
        g.bench_with_input(BenchmarkId::new("oracle", n), &(oa, ob), |b, (oa, ob)| {
            b.iter_batched(
                || (oa.clone(), ob.clone()),
                |(a, b)| black_box(a | b),
                BatchSize::SmallInput,
            );
        });
    }
    g.finish();
}

/// `partial_cmp` (the causal order) over the three outcomes the comparison can take, each
/// exercising a different traversal: `concurrent` (two independent histories), `ordered`
/// (one strictly precedes the other), and `equal` (a version against its own clone).
fn bench_partial_cmp(c: &mut Criterion) {
    let mut g = c.benchmark_group("version/partial_cmp");
    let mut r = rng(4);
    for &n in SIZES {
        // Concurrent: two histories grown on disjoint parties.
        let plan2 = common::plan(&mut r, n, 2);
        let iv = common::impl_versions(&plan2, 2);
        let ov = common::oracle_versions(&plan2, 2);

        // Ordered + equal: a single history, plus a strictly later copy of it.
        let plan1 = common::plan(&mut r, n, 1);
        let (iparty, base) = common::impl_clocks(&plan1, 1).pop().unwrap().into_parts();
        let (oparty, obase) = common::oracle_clocks(&plan1, 1).pop().unwrap().into_parts();
        let mut later = base.clone();
        later.tick(&iparty);
        let mut olater = obase.clone();
        olater.tick(&oparty);

        for (kind, ia, ib, oa, ob) in [
            ("concurrent", &iv[0], &iv[1], &ov[0], &ov[1]),
            ("ordered", &base, &later, &obase, &olater),
            ("equal", &base, &base, &obase, &obase),
        ] {
            g.bench_with_input(
                BenchmarkId::new(format!("before/{kind}"), n),
                &(ia, ib),
                |b, (a, c)| {
                    b.iter(|| black_box(a.partial_cmp(c)));
                },
            );
            g.bench_with_input(
                BenchmarkId::new(format!("oracle/{kind}"), n),
                &(oa, ob),
                |b, (a, c)| {
                    b.iter(|| black_box(a.partial_cmp(c)));
                },
            );
        }
    }
    g.finish();
}

/// `encode`/`decode`: the packed byte codec. No oracle equivalent; impl alone.
fn bench_codec(c: &mut Criterion) {
    let mut g = c.benchmark_group("version/codec");
    let mut r = rng(5);
    for &n in SIZES {
        let plan = common::plan(&mut r, n, 1);
        let version = common::impl_versions(&plan, 1).pop().unwrap();
        let bytes = version.encode();
        g.bench_with_input(BenchmarkId::new("before/encode", n), &version, |b, v| {
            b.iter(|| black_box(v.encode()));
        });
        g.bench_with_input(BenchmarkId::new("before/decode", n), &bytes, |b, bytes| {
            b.iter(|| black_box(Version::decode(&bytes[..]).unwrap()));
        });
    }
    g.finish();
}

criterion_group!(
    benches,
    bench_tick,
    bench_batch,
    bench_merge,
    bench_partial_cmp,
    bench_codec
);
criterion_main!(benches);
