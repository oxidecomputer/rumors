//! `Party` benchmarks: the optimized packed-`BitVec` implementation against the naive
//! recursive oracle, on the same randomized trees (see `common`). Codec ops have no
//! oracle counterpart and are timed for the impl alone.

use before::Party;
use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use rand::rngs::StdRng;
use rand::SeedableRng;

mod common;
use common::{SEED, SIZES};

/// A fresh RNG per group, each seeded off [`SEED`], so inputs are reproducible and every
/// group sees an independent (but fixed) stream.
fn rng(salt: u64) -> StdRng {
    StdRng::seed_from_u64(SEED.wrapping_add(salt))
}

/// `fork`: split a party in two. Destructive (mutates self, returns the sibling), so each
/// iteration starts from a fresh value rebuilt in the untimed `iter_batched` setup —
/// `decode` for the impl (not `Clone`), `clone` for the oracle.
fn bench_fork(c: &mut Criterion) {
    let mut g = c.benchmark_group("party/fork");
    let mut r = rng(1);
    for &n in SIZES {
        let plan = common::plan(&mut r, n, 1);
        let bytes = common::impl_parties(&plan, 1).pop().unwrap().encode();
        let orc = common::oracle_parties(&plan, 1).pop().unwrap();
        g.bench_with_input(BenchmarkId::new("before", n), &bytes, |b, bytes| {
            b.iter_batched(
                || Party::decode(&bytes[..]).unwrap(),
                |mut p| black_box(p.fork()),
                BatchSize::SmallInput,
            );
        });
        g.bench_with_input(BenchmarkId::new("oracle", n), &orc, |b, orc| {
            b.iter_batched(
                || orc.clone(),
                |mut p| black_box(p.fork()),
                BatchSize::SmallInput,
            );
        });
    }
    g.finish();
}

/// `join`: merge a disjoint party into self. Destructive in both operands (self mutated,
/// other consumed), so both are rebuilt fresh per iteration.
fn bench_join(c: &mut Criterion) {
    let mut g = c.benchmark_group("party/join");
    let mut r = rng(2);
    for &n in SIZES {
        let plan = common::plan(&mut r, n, 2);
        let imp = common::impl_parties(&plan, 2);
        let (ba, bb) = (imp[0].encode(), imp[1].encode());
        let orc = common::oracle_parties(&plan, 2);
        let (oa, ob) = (orc[0].clone(), orc[1].clone());
        g.bench_with_input(BenchmarkId::new("before", n), &(ba, bb), |b, (ba, bb)| {
            b.iter_batched(
                || {
                    (
                        Party::decode(&ba[..]).unwrap(),
                        Party::decode(&bb[..]).unwrap(),
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

/// `is_disjoint`: read-only overlap check on two disjoint parties. Built once per size.
fn bench_is_disjoint(c: &mut Criterion) {
    let mut g = c.benchmark_group("party/is_disjoint");
    let mut r = rng(3);
    for &n in SIZES {
        let plan = common::plan(&mut r, n, 2);
        let imp = common::impl_parties(&plan, 2);
        let orc = common::oracle_parties(&plan, 2);
        g.bench_with_input(BenchmarkId::new("before", n), &imp, |b, p| {
            b.iter(|| black_box(p[0].is_disjoint(&p[1])));
        });
        g.bench_with_input(BenchmarkId::new("oracle", n), &orc, |b, p| {
            b.iter(|| black_box(p[0].is_disjoint(&p[1])));
        });
    }
    g.finish();
}

/// `partial_cmp`: the descent order, over the two outcomes that force a full traversal.
///
/// - `ancestor`: an ancestor/descendant pair (`Some(Less)`). One direction (`a ⊇ b`)
///   runs to completion; the other is excluded early. The pre-fork whole contains both
///   post-fork halves, so we snapshot it (bytes for the impl, clone for the oracle)
///   before forking off a child.
/// - `equal`: two structurally identical parties (`Some(Equal)`). *Both* directions run
///   to completion — the case the single-pass [`ops::compare`] helps most, since a
///   two-pass containment formulation would traverse the whole tree twice here.
///
/// (Disjoint cousins are omitted: they bail at the first mismatch, so they measure the
/// per-call floor rather than traversal.)
fn bench_partial_cmp(c: &mut Criterion) {
    let mut g = c.benchmark_group("party/partial_cmp");
    let mut r = rng(4);
    for &n in SIZES {
        let plan = common::plan(&mut r, n, 1);

        // The pre-fork whole, snapshotted before forking off a child.
        let mut ip = common::impl_parties(&plan, 1).pop().unwrap();
        let whole_bytes = ip.encode();
        let child = ip.fork();
        let mut op = common::oracle_parties(&plan, 1).pop().unwrap();
        let owhole = op.clone();
        let ochild = op.fork();

        // ancestor/descendant: whole ⊋ child (Some(Less)).
        let anc = Party::decode(&whole_bytes[..]).unwrap();
        // equal: two distinct instances of the whole (Some(Equal)) — both directions full.
        let ieq = (
            Party::decode(&whole_bytes[..]).unwrap(),
            Party::decode(&whole_bytes[..]).unwrap(),
        );
        let oeq = (owhole.clone(), owhole.clone());

        g.bench_with_input(
            BenchmarkId::new("before/ancestor", n),
            &(anc, child),
            |b, (a, c)| {
                b.iter(|| black_box(a.partial_cmp(c)));
            },
        );
        g.bench_with_input(
            BenchmarkId::new("oracle/ancestor", n),
            &(owhole, ochild),
            |b, (a, c)| {
                b.iter(|| black_box(a.partial_cmp(c)));
            },
        );
        g.bench_with_input(BenchmarkId::new("before/equal", n), &ieq, |b, (a, c)| {
            b.iter(|| black_box(a.partial_cmp(c)));
        });
        g.bench_with_input(BenchmarkId::new("oracle/equal", n), &oeq, |b, (a, c)| {
            b.iter(|| black_box(a.partial_cmp(c)));
        });
    }
    g.finish();
}

/// `encode`/`decode`: the packed byte codec. No oracle equivalent (the oracle omits the
/// codec by design), so these are timed for the impl alone.
fn bench_codec(c: &mut Criterion) {
    let mut g = c.benchmark_group("party/codec");
    let mut r = rng(5);
    for &n in SIZES {
        let plan = common::plan(&mut r, n, 1);
        let party = common::impl_parties(&plan, 1).pop().unwrap();
        let bytes = party.encode();
        g.bench_with_input(BenchmarkId::new("before/encode", n), &party, |b, p| {
            b.iter(|| black_box(p.encode()));
        });
        g.bench_with_input(BenchmarkId::new("before/decode", n), &bytes, |b, bytes| {
            b.iter(|| black_box(Party::decode(&bytes[..]).unwrap()));
        });
    }
    g.finish();
}

criterion_group!(
    benches,
    bench_fork,
    bench_join,
    bench_is_disjoint,
    bench_partial_cmp,
    bench_codec
);
criterion_main!(benches);
