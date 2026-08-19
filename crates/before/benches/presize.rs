//! Allocation-strategy record for stored build buffers: wall cost and
//! end-state resident bytes of the shipped output pre-sizing, per site.
//!
//! Every cell here drives a public operation whose output buffer is built
//! once and then *becomes storage* (the freeze adopts the build buffer
//! without copying, so its final capacity — pre-sized or grown — stays
//! resident for the value's whole life). The sites span the pre-sizing
//! regimes the library ships:
//!
//! - **projection** (`(&v / &p).to_version()`): the pre-size is an
//!   *estimate* (operands' summed lengths) and the output is not derivable
//!   from the inputs — it can outgrow them, so this is the one site with a
//!   mid-walk allocation phase. The `projection_outgrow` family drives the
//!   output across successive growth doublings of the pre-sized buffer.
//! - **display** (`v.to_string()`): the pre-size is *exact* — one
//!   allocation, never grown (pinned by an assert in the renderer).
//! - **parse** (`s.parse::<Version>()`): the pre-size is a *heuristic*
//!   (the text length, in bits).
//!
//! The A/B sides are compile-time arms of the library, selected per site by
//! `RUSTFLAGS='--cfg before_alloc_ab="<arm>"'` (the `bench-alloc-ab`
//! recipe): `projection_growth` and `display_growth` start the site's
//! buffer empty, `projection_shrink` adds one exact-size copy where the
//! buffer becomes storage. Nothing in-process distinguishes the sides, so
//! each run saves a criterion baseline named after its arm, and every line
//! of the resident-bytes table below is stamped with the compiled arm.
//!
//! Beside the wall cells, the binary prints one `presize-resident` line per
//! site and input before criterion runs: the live-heap delta of building
//! and holding the operation's result, measured by this binary's counting
//! allocator. That column is deterministic (allocation *requests*, in
//! bytes; the platform allocator's size-class rounding is deliberately out
//! of frame) and is the record's evidence on the copy-vs-stranded-slack
//! trade. The `decoded` rows are the contrast baseline: the same values
//! rebuilt through `decode`, whose buffer is exact by construction.
//!
//! Wall times are compared *within* a machine and build only; the counting
//! allocator adds a small uniform overhead to every allocation, identical
//! across arms, so arm-to-arm deltas stay honest.

use before::{Clock, Party, Version};
use criterion::{black_box, criterion_group, BenchmarkId, Criterion};
use peak_alloc::PeakAlloc;
use rand::rngs::StdRng;
use rand::SeedableRng;

mod common;
use common::{SEED, SIZES};

#[global_allocator]
static HEAP: PeakAlloc = PeakAlloc;

fn rng(salt: u64) -> StdRng {
    StdRng::seed_from_u64(SEED.wrapping_add(salt))
}

/// Tooth count of the `projection_outgrow` family's fragmented party.
///
/// Alternate teeth of a fork comb this long are kept (the rest drop as
/// structural holes), so a projection onto the kept party crosses an
/// ownership boundary at every tooth and must re-emit the absolute height
/// at each crossing — the mechanism whose output outgrows the pre-size.
const OUTGROW_FRAGMENTS: usize = 256;

/// Height exponents the `projection_outgrow` family sweeps: the plateau is
/// `2^k` ticks tall, so each boundary crossing emits an `O(k)`-bit code
/// while the operands stay near-constant in size.
const OUTGROW_HEIGHT_LOG2: &[u32] = &[1, 6, 12, 18, 24, 30];

/// Build and hold a value, returning it with the live-heap delta (bytes
/// requested and still held) its construction left behind.
fn resident<T>(build: impl FnOnce() -> T) -> (T, usize) {
    let baseline = HEAP.current_usage();
    let value = build();
    (value, HEAP.current_usage().saturating_sub(baseline))
}

/// The projection operands for one universe size: the merged two-group
/// version and the first group's (fragmented) party.
fn projection_operands(r: &mut StdRng, n: usize) -> (Version, Party) {
    let plan = common::plan(r, n, 2);
    let mut clocks = common::impl_clocks(&plan, 2);
    let (_, vb) = clocks.pop().expect("two groups").into_parts();
    let (pa, va) = clocks.pop().expect("two groups").into_parts();
    (va | vb, pa)
}

/// The `projection_outgrow` operands, one pair per height exponent: a
/// single plateau `2^k` ticks tall over the whole id space, and a party
/// owning alternate teeth of a [`OUTGROW_FRAGMENTS`]-tooth fork comb.
///
/// The family's design property is asserted on construction: the
/// materialized output must sweep from near the pre-size estimate
/// (operands' summed lengths) to at least 4x past it, so successive cells
/// cross output-buffer growth doublings. Encoded sizes are
/// machine-independent, so a construction drift fails loudly here instead
/// of silently flattening the record's allocation phase.
fn outgrow_family() -> Vec<(u32, Version, Party)> {
    let family: Vec<(u32, Version, Party)> = OUTGROW_HEIGHT_LOG2
        .iter()
        .map(|&k| {
            let (mut party, mut version) = Clock::seed().into_parts();
            version.ticks(&party, 1u64 << k);
            let mut kept: Option<Party> = None;
            for i in 0..OUTGROW_FRAGMENTS - 1 {
                let child = party.fork();
                if i % 2 == 0 {
                    match kept.as_mut() {
                        None => kept = Some(child),
                        Some(kept) => kept.join(child).expect("comb teeth are pairwise disjoint"),
                    }
                }
                // Odd teeth (and the residual sliver) drop: structural
                // holes between the kept teeth, one ownership boundary
                // per tooth.
            }
            (k, version, kept.expect("the comb keeps its even teeth"))
        })
        .collect();

    let ratio = |v: &Version, p: &Party| {
        let out = (v / p).to_version().encoded_bits() as f64;
        out / (v.encoded_bits() + p.encoded_bits()) as f64
    };
    let (_, v0, p0) = family.first().expect("the sweep is nonempty");
    let (_, v1, p1) = family.last().expect("the sweep is nonempty");
    let (first, last) = (ratio(v0, p0), ratio(v1, p1));
    assert!(
        last >= 4.0 * first,
        "the outgrow family must sweep the output across pre-size doublings \
         (output/pre-size ratio moved only {first:.2} -> {last:.2})",
    );
    family
}

/// One deterministic `presize-resident` line: the end-state footprint of a
/// stored result, beside the sizes that contextualize it.
fn resident_line<T>(
    site: &str,
    x: impl std::fmt::Display,
    out_bits: u64,
    build: impl FnOnce() -> T,
) {
    let arm = common::alloc_arms();
    let (value, bytes) = resident(build);
    println!(
        "presize-resident arm={arm} site={site} x={x} resident_bytes={bytes} out_bits={out_bits}"
    );
    drop(value);
}

/// The resident-bytes table: every site x input, one line each, printed
/// before the wall cells so each record run carries both columns.
fn resident_report() {
    let mut r = rng(101);
    for &n in SIZES {
        let (v, pa) = projection_operands(&mut r, n);
        let out = (&v / &pa).to_version();
        resident_line("projection", n, out.encoded_bits(), || {
            (&v / &pa).to_version()
        });
        // The contrast baseline: the same projected value rebuilt through
        // `decode`, whose buffer is exact by construction — the resident
        // difference is the build path's stranded slack alone.
        let bytes = out.encoded_bits();
        let encoded = out.encode();
        resident_line("projection_decoded", n, bytes, || {
            Version::decode(&encoded[..]).expect("a fresh encoding decodes")
        });
    }
    for (k, v, pa) in &outgrow_family() {
        let out_bits = (v / pa).to_version().encoded_bits();
        resident_line("projection_outgrow", k, out_bits, || (v / pa).to_version());
    }
    let mut r = rng(102);
    for &n in SIZES {
        // Merge carries no arm of its own: its wall cells live in the
        // `version/merge` bench, and this row records the end-state
        // footprint of its (estimate-pre-sized) stored result.
        let plan = common::plan(&mut r, n, 2);
        let versions = common::impl_versions(&plan, 2);
        let (ba, bb) = (versions[0].encode(), versions[1].encode());
        let merged_bits = {
            let a = Version::decode(&ba[..]).expect("a fresh encoding decodes");
            let b = Version::decode(&bb[..]).expect("a fresh encoding decodes");
            (a | b).encoded_bits()
        };
        resident_line("merge", n, merged_bits, || {
            let a = Version::decode(&ba[..]).expect("a fresh encoding decodes");
            let b = Version::decode(&bb[..]).expect("a fresh encoding decodes");
            a | b
        });
    }
    let mut r = rng(103);
    for &n in SIZES {
        let plan = common::plan(&mut r, n, 1);
        let v = common::impl_versions(&plan, 1).pop().expect("one group");
        let s = v.to_string();
        resident_line("display", n, s.len() as u64 * 8, || v.to_string());
        resident_line("parse", n, v.encoded_bits(), || {
            s.parse::<Version>().expect("a rendered version parses")
        });
    }
}

/// Projection materialization over the universe-size axis: the per-op wall
/// cost of the estimate-pre-sized site on organically fragmented parties.
fn bench_projection(c: &mut Criterion) {
    let mut g = c.benchmark_group("presize/projection");
    let mut r = rng(1);
    for &n in SIZES {
        let operands = projection_operands(&mut r, n);
        g.bench_with_input(BenchmarkId::new("stored", n), &operands, |b, (v, pa)| {
            b.iter(|| black_box((v / pa).to_version()));
        });
    }
    g.finish();
}

/// Projection materialization over the height axis: the output outgrows
/// the pre-size estimate further at every step.
///
/// The allocation phase — mid-walk growth under the shipped arm, the full
/// doubling ladder under `projection_growth`, one extra copy under
/// `projection_shrink` — is the dominant difference between adjacent
/// cells and arms.
fn bench_projection_outgrow(c: &mut Criterion) {
    let mut g = c.benchmark_group("presize/projection_outgrow");
    for (k, v, pa) in &outgrow_family() {
        g.bench_with_input(BenchmarkId::new("stored", k), &(v, pa), |b, (v, pa)| {
            b.iter(|| black_box((*v / *pa).to_version()));
        });
    }
    g.finish();
}

/// Text rendering: the exact-pre-size site (one allocation, never grown).
///
/// The `display_growth` arm prices the hypothesis that an exact request
/// can lose to doubling growth through the allocator's size-classing on
/// a short-lived request pattern.
fn bench_display(c: &mut Criterion) {
    let mut g = c.benchmark_group("presize/display");
    let mut r = rng(2);
    for &n in SIZES {
        let plan = common::plan(&mut r, n, 1);
        let v = common::impl_versions(&plan, 1).pop().expect("one group");
        g.bench_with_input(BenchmarkId::new("stored", n), &v, |b, v| {
            b.iter(|| black_box(v.to_string()));
        });
    }
    g.finish();
}

/// Text parsing: the heuristic-pre-size site (the text length, in bits,
/// which the built stream may under- or over-run).
///
/// No alternative arm is compiled for this site — the resident table
/// carries its slack evidence, and a seam waits on that evidence.
fn bench_parse(c: &mut Criterion) {
    let mut g = c.benchmark_group("presize/parse");
    let mut r = rng(3);
    for &n in SIZES {
        let plan = common::plan(&mut r, n, 1);
        let v = common::impl_versions(&plan, 1).pop().expect("one group");
        let s = v.to_string();
        g.bench_with_input(BenchmarkId::new("stored", n), &s, |b, s| {
            b.iter(|| black_box(s.parse::<Version>().expect("a rendered version parses")));
        });
    }
    g.finish();
}

criterion_group!(
    benches,
    bench_projection,
    bench_projection_outgrow,
    bench_display,
    bench_parse
);

fn main() {
    resident_report();
    benches();
    Criterion::default().configure_from_args().final_summary();
}
