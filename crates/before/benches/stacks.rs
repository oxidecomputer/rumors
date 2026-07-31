//! Allocation-strategy A/B for the parsers' transient explicit stacks:
//! the shipped inline-capacity `SmallVec` against a heap-only `Vec`, per
//! site, on identical walks.
//!
//! The library holds exactly two such stacks, and each is a separate site
//! with its own seam and its own cells, so the record can rank
//! representations site by site:
//!
//! - `stacks/id_parse` drives the packed id-tree parser (`Party::decode`),
//!   the site the `id_stack_vec` arm switches;
//! - `stacks/text_parse` drives the text id parser (`Party::from_str`),
//!   the site the `text_stack_vec` arm switches.
//!
//! Both stacks are *transient* — built, walked, and dropped inside one
//! parse — which is what makes `SmallVec` a candidate at all: inline
//! capacity on stored data would stay resident for the value's lifetime,
//! but here it lives exactly as long as the walk. Each site runs two input
//! regimes: `random` (organically fork-shaped parties, the wire path's
//! typical depths, where the shipped stack parses with no heap at all) and
//! `spine` (left-spine parties parameterized by depth, sweeping from
//! inside the inline capacity into the deep spill tail).
//!
//! The A/B sides are compile-time builds of the same cells, selected by
//! `RUSTFLAGS='--cfg before_alloc_ab="<arm>"'` (the `bench-alloc-ab`
//! recipe), one site's arm per run. Nothing in-process distinguishes the
//! sides, so each run saves a criterion baseline named after its arm; the
//! binary prints the compiled arm once at startup as provenance.

use before::Party;
use criterion::{black_box, criterion_group, BenchmarkId, Criterion};
use rand::rngs::StdRng;
use rand::SeedableRng;

mod common;
use common::{SEED, SIZES};

fn rng(salt: u64) -> StdRng {
    StdRng::seed_from_u64(SEED.wrapping_add(salt))
}

/// Universe sizes for the `random` regime: a head slice of the shared
/// [`SIZES`] axis.
///
/// Parse cost scales with node count, but the stack regime (inline vs
/// spill) is governed by depth, which random shapes keep shallow at any
/// of these sizes.
const RANDOM_SIZES: &[usize] = &[SIZES[0], SIZES[2], SIZES[4]];

/// Spine depths for the `spine` regime: one explicit-stack frame per
/// level.
///
/// Swept from well inside the shipped inline capacity to a deep
/// heap-spill tail, so the record locates the crossover empirically
/// rather than assuming the capacity constant.
const SPINE_DEPTHS: &[usize] = &[4, 12, 18, 64, 1024];

/// A party owning one sliver at depth `d`: a left spine, costing the
/// parsers one stack frame per level — the depth axis in its purest
/// shape.
fn spine_party(d: usize) -> Party {
    let mut p = Party::seed();
    for _ in 0..d {
        // The forked halves drop: their regions become structural holes,
        // and `p` narrows one level deeper.
        drop(p.fork());
    }
    p
}

/// A random party from the shared fork-a-universe recipe: the typical,
/// shallow shape the wire path decodes constantly.
fn random_party(r: &mut StdRng, n: usize) -> Party {
    let plan = common::plan(r, n, 1);
    common::impl_parties(&plan, 1).pop().expect("one group")
}

/// `Party::decode`: the packed id-tree parser, whose explicit stack the
/// `id_stack_vec` arm switches to a heap-only `Vec`.
fn bench_id_parse(c: &mut Criterion) {
    let mut g = c.benchmark_group("stacks/id_parse");
    let mut r = rng(1);
    for &n in RANDOM_SIZES {
        let bytes = random_party(&mut r, n).encode();
        g.bench_with_input(
            BenchmarkId::new("transient/random", n),
            &bytes,
            |b, bytes| {
                b.iter(|| black_box(Party::decode(&bytes[..]).expect("a fresh encoding decodes")));
            },
        );
    }
    for &d in SPINE_DEPTHS {
        let bytes = spine_party(d).encode();
        g.bench_with_input(
            BenchmarkId::new("transient/spine", d),
            &bytes,
            |b, bytes| {
                b.iter(|| black_box(Party::decode(&bytes[..]).expect("a fresh encoding decodes")));
            },
        );
    }
    g.finish();
}

/// `Party::from_str`: the text id parser, whose explicit stack the
/// `text_stack_vec` arm switches to a heap-only `Vec`.
fn bench_text_parse(c: &mut Criterion) {
    let mut g = c.benchmark_group("stacks/text_parse");
    let mut r = rng(2);
    for &n in RANDOM_SIZES {
        let text = random_party(&mut r, n).to_string();
        g.bench_with_input(BenchmarkId::new("transient/random", n), &text, |b, s| {
            b.iter(|| black_box(s.parse::<Party>().expect("a rendered party parses")));
        });
    }
    for &d in SPINE_DEPTHS {
        let text = spine_party(d).to_string();
        g.bench_with_input(BenchmarkId::new("transient/spine", d), &text, |b, s| {
            b.iter(|| black_box(s.parse::<Party>().expect("a rendered party parses")));
        });
    }
    g.finish();
}

criterion_group!(benches, bench_id_parse, bench_text_parse);

fn main() {
    println!("stacks-arm arm={}", common::alloc_arms());
    benches();
    Criterion::default().configure_from_args().final_summary();
}
