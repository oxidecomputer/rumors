//! `Version` benchmarks: the optimized implementation against the naive
//! recursive oracle, on the same randomized event trees (see `common`).
//!
//! Includes the repeated-tick comparison (impl vs oracle over `k` ticks),
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

/// Repeated mutation: applying `k` ticks — fused and per-tick.
///
/// The fused series is one `ticks(k)` call (two walks at most, whatever
/// `k`); the per-tick series applies each tick through the fill splice,
/// paying the unpack/repack per call; the oracle re-normalizes each
/// tick. Tree size is fixed; `k` is the axis, so the fused series must
/// read flat where the iterated series scale with `k`.
fn bench_k_ticks(c: &mut Criterion) {
    let mut g = c.benchmark_group("version/k_ticks");
    let mut r = rng(2);
    const TREE: usize = 64;
    let (bytes, iparty, oversion, oparty) = version_and_party(&mut r, TREE);
    for &k in &[1usize, 4, 16, 64] {
        g.bench_with_input(BenchmarkId::new("before", k), &bytes, |b, bytes| {
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
        });
        g.bench_with_input(BenchmarkId::new("before/fused", k), &bytes, |b, bytes| {
            b.iter_batched(
                || Version::decode(&bytes[..]).unwrap(),
                |mut v| {
                    v.ticks(&iparty, k as u64);
                    black_box(v)
                },
                BatchSize::SmallInput,
            );
        });
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

/// `partial_cmp` (the causal order) over the three outcomes the comparison
/// can take, each exercising a different traversal.
///
/// The outcomes: `concurrent` (two independent histories), `ordered`
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

/// The ownership-hole regime: a party owning a vanishing custody
/// fraction of a fully-received version (`common::hole_pair`), the
/// small-custody-peer shape the ownership-gated walks serve.
///
/// Consumers on the same pair: `tick`, the projection
/// (`OwnVersion::to_version`), masked equality of equal projections
/// in distinct buffers (a full co-walk, no early exit), and the
/// asymmetric masked equality against the flat materialization. No
/// oracle row: the regime's anchor is the same pair on the plain
/// walks above.
fn bench_hole(c: &mut Criterion) {
    let mut g = c.benchmark_group("version/hole");
    let mut r = rng(1);
    for &n in SIZES {
        let plan = common::plan(&mut r, n, 1);
        let (party, version) = common::hole_pair(&plan);
        let bytes = version.encode();
        g.bench_with_input(BenchmarkId::new("tick", n), &bytes, |b, bytes| {
            b.iter_batched(
                || Version::decode(&bytes[..]).unwrap(),
                |mut v| {
                    v.tick(&party);
                    black_box(v)
                },
                BatchSize::SmallInput,
            );
        });
        g.bench_with_input(BenchmarkId::new("project", n), &version, |b, v| {
            b.iter(|| black_box((v / &party).to_version()));
        });
        let (party2, version2) = common::hole_pair(&plan);
        g.bench_with_input(
            BenchmarkId::new("masked_eq", n),
            &(&version, &version2),
            |b, (v, v2)| {
                b.iter(|| black_box((*v / &party) == (*v2 / &party2)));
            },
        );
        // The asymmetric form: the projection against its own (small)
        // materialization — one deep masked side against a shallow
        // plain one, the shape whose interior boundaries the masked
        // walk consumes alone.
        let projected = (&version / &party).to_version();
        g.bench_with_input(
            BenchmarkId::new("masked_eq_flat", n),
            &(&version, &projected),
            |b, (v, w)| {
                b.iter(|| black_box((*v / &party) == **w));
            },
        );
    }
    g.finish();
}

criterion_group!(
    benches,
    bench_tick,
    bench_k_ticks,
    bench_merge,
    bench_partial_cmp,
    bench_codec,
    bench_hole
);
criterion_main!(benches);
