//! Adversarial-shape benchmarks: the `before::meter` generator inputs, timed
//! at small sizes so the resource-proportionality paths register as
//! wall-clock numbers.
//!
//! No oracle comparison: these rows exist to make a superlinear regression
//! on a worst-case shape visible in `cargo bench`, complementing the
//! deterministic envelopes in `tests/meter.rs`.

use before::{meter, Party, Version};
use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};

/// Magnitudes (bits) of the hugeleaf rows: one gamma code as wide as the
/// whole input, the shape that exposes any cost superlinear in a single
/// code's width.
const HUGELEAF_BITS: &[usize] = &[8_192, 32_768];

/// `(root magnitude bits, spine depth)` of the bigroot rows: a big base over
/// a long spine, the shape that exposes per-node costs scaled by magnitude.
const BIGROOT_SIZES: &[(usize, usize)] = &[(4_096, 256), (16_384, 1_024)];

/// Depths of the id-spine rows: a unary chain, the shape that drives the
/// one-sided id walks to their full depth.
const ID_SPINE_DEPTH: &[usize] = &[8_192, 32_768];

/// `Version::decode` of hugeleaf: the whole input is one wide gamma code, so
/// this times the mantissa accumulation undiluted.
fn bench_decode_hugeleaf(c: &mut Criterion) {
    let mut g = c.benchmark_group("amplify/version_decode_hugeleaf");
    for &b in HUGELEAF_BITS {
        // The generator's construction language is transcoded to the stored
        // coding by `Packed::version`; decode times its packed bytes.
        let bytes = meter::hugeleaf(b).version().encode();
        g.bench_with_input(BenchmarkId::new("before", b), &bytes, |bench, bytes| {
            bench.iter(|| black_box(Version::decode(&bytes[..]).unwrap()));
        });
    }
    g.finish();
}

/// `join` of bigroot with a one-tick version: reading the stored big base
/// re-runs the wide-gamma decode, and the emit path rebuilds the spine.
fn bench_join_bigroot(c: &mut Criterion) {
    let mut g = c.benchmark_group("amplify/version_join_bigroot");
    for &(b, d) in BIGROOT_SIZES {
        let bytes = meter::bigroot(b, d).version().encode();
        let one = Version::try_from(1u64).expect("a one-tick version is valid");
        g.bench_with_input(
            BenchmarkId::new("before", format!("{b}x{d}")),
            &bytes,
            |bench, bytes| {
                bench.iter_batched(
                    || {
                        (
                            Version::decode(&bytes[..]).unwrap(),
                            Version::decode(&one.encode()[..]).unwrap(),
                        )
                    },
                    |(a, b)| black_box(a | b),
                    BatchSize::SmallInput,
                );
            },
        );
    }
    g.finish();
}

/// `Party::without` subtracting an id spine from the seed: the one-sided
/// complement walk runs to the subtrahend's full depth.
fn bench_without_spine(c: &mut Criterion) {
    let mut g = c.benchmark_group("amplify/party_without_spine");
    for &d in ID_SPINE_DEPTH {
        let p = meter::id_spine(d, true);
        let spine = Party::decode(&p.bytes[..]).expect("generated shape is strict normal form");
        g.bench_with_input(BenchmarkId::new("before", d), &spine, |bench, spine| {
            bench.iter_batched(
                Party::seed,
                |seed| black_box(seed.without(spine)),
                BatchSize::SmallInput,
            );
        });
    }
    g.finish();
}

criterion_group!(
    benches,
    bench_decode_hugeleaf,
    bench_join_bigroot,
    bench_without_spine
);
criterion_main!(benches);
