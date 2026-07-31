//! The span-band discriminator: per-pair native measurements of the
//! pair operations over the `version_span` input space, as raw CSV.
//!
//! The atlas's `version_span` heatmap shows banded conditional work
//! distributions, but a heatmap cannot say *which coordinate of the
//! pair* separates the bands — it plots work against total size alone.
//! This runner re-draws the same input space (total packed bytes, split
//! uniformly across two version operands, each drawn exactly uniformly
//! at its size) and records, per pair, the candidate discriminating
//! coordinates beside the work: the split (lopsidedness), the pair's
//! causal relation (comparable or concurrent), and the deterministic
//! native meters — scanned bits, limb operations, accumulator digit
//! touches — for each of the comparison, the meet, the join, the span
//! ladder, and the raw fused hull kernel. Native counters stand in for
//! guest fuel deliberately: the walks' instruction counts are affine in
//! exactly these counted primitives, so a band separation of the
//! atlas's magnitude survives the change of currency, and the native
//! run needs no wasm guest.
//!
//! Flags (all optional): `--samples <n>` per size column (default 500),
//! `--sizes <a,b,c>` total packed bytes per column (default
//! `64,256,1024,4096`), `--seed <u64>` base seed (default `0xa71a5`),
//! `--out <path>` CSV destination (default `target/spanbands.csv`).
//!
//! The run is a pure function of the flags: every pair's RNG is seeded
//! from (`spanbands`, size, sample index), the loop is single-threaded
//! (the counters are process-global), and no entropy comes from time or
//! the OS. Wall time is never read.
//!
//! CSV columns: `size,split_a,split_b,relation,rejected`, then
//! `<op>_{scan,limb,touch}` for `cmp`, `meet`, `join`, `span`, `hull`.
//! `relation` is `lt`/`gt`/`eq`/`conc` (the stored-form comparison's
//! verdict). `span` is the public ladder (classify, then hand back or
//! emit); `hull` is the raw fused emission kernel on the same operands,
//! so the two arms of the classify-first-versus-emit-always trade are
//! both on every row.

use std::io::Write;
use std::path::PathBuf;

use before::meter::{self, skyline, Packed};
use before::Version;
use before_fuelscape::sample::{cell_rng, VersionSampler};
use rand::Rng;

/// One measured call's readings: (scanned bits, limb ops, digit touches).
fn metered<R>(f: impl FnOnce() -> R) -> (R, u64, u64, u64) {
    meter::reset_scan_bits();
    meter::reset_limb_ops();
    suanpan::touch_meter::reset();
    let out = f();
    (
        out,
        meter::scan_bits(),
        meter::limb_ops(),
        suanpan::touch_meter::touches(),
    )
}

fn main() {
    let mut samples: usize = 500;
    let mut sizes: Vec<usize> = vec![64, 256, 1024, 4096];
    let mut seed: u64 = 0xa71a5;
    let mut out = PathBuf::from("target/spanbands.csv");
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let mut value = || {
            args.next()
                .unwrap_or_else(|| panic!("flag {flag} needs a value"))
        };
        match flag.as_str() {
            "--samples" => samples = value().parse().expect("--samples <count>"),
            "--sizes" => {
                sizes = value()
                    .split(',')
                    .map(|s| s.parse().expect("--sizes <bytes,bytes,...>"))
                    .collect();
            }
            "--seed" => seed = value().parse().expect("--seed <u64>"),
            "--out" => out = PathBuf::from(value()),
            other => panic!("unknown flag {other} (see the module doc for the flag list)"),
        }
    }
    let max = sizes.iter().copied().max().expect("at least one size");
    assert!(
        sizes.iter().all(|&s| s >= 2),
        "a two-operand column needs at least 2 bytes"
    );

    println!("building the count table to {max} bytes...");
    let sampler = VersionSampler::new(max);

    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir).expect("output directory must be creatable");
    }
    let mut csv = std::io::BufWriter::new(std::fs::File::create(&out).expect("CSV must open"));
    writeln!(
        csv,
        "size,split_a,split_b,relation,rejected,\
         cmp_scan,cmp_limb,cmp_touch,\
         meet_scan,meet_limb,meet_touch,\
         join_scan,join_limb,join_touch,\
         span_scan,span_limb,span_touch,\
         hull_scan,hull_limb,hull_touch"
    )
    .expect("CSV header writes");

    for &size in &sizes {
        let mut rejected_total = 0u64;
        for index in 0..samples {
            let mut rng = cell_rng(seed, "spanbands", size, index);
            // The binary split rule of the atlas's two-operand rows: one
            // uniform cut in `1..size`.
            let split_a = rng.gen_range(1..size);
            let split_b = size - split_a;
            let da = sampler
                .sample_bytes(split_a, &mut rng)
                .expect("every byte size down to 1 has canonical versions");
            let db = sampler
                .sample_bytes(split_b, &mut rng)
                .expect("every byte size down to 1 has canonical versions");
            let rejected = da.rejected + db.rejected;
            rejected_total += rejected;
            let a = Version::decode(&da.bytes[..]).expect("a sampled version decodes");
            let b = Version::decode(&db.bytes[..]).expect("a sampled version decodes");
            let pa = Packed {
                bytes: da.bytes,
                bits: da.bits,
            };
            let pb = Packed {
                bytes: db.bytes,
                bits: db.bits,
            };

            let (verdict, cmp_scan, cmp_limb, cmp_touch) = metered(|| a.partial_cmp(&b));
            let relation = match verdict {
                Some(core::cmp::Ordering::Less) => "lt",
                Some(core::cmp::Ordering::Greater) => "gt",
                Some(core::cmp::Ordering::Equal) => "eq",
                None => "conc",
            };
            let (_, meet_scan, meet_limb, meet_touch) = metered(|| &a & &b);
            let (_, join_scan, join_limb, join_touch) = metered(|| &a | &b);
            let (_, span_scan, span_limb, span_touch) = metered(|| a.span(&b));
            let (hulled, hull_scan, hull_limb, hull_touch) =
                metered(|| skyline::emit::hull(pa.as_bits(), pb.as_bits()));
            assert_eq!(
                hulled.relation, verdict,
                "the fused verdict must match the stored-form comparison"
            );

            writeln!(
                csv,
                "{size},{split_a},{split_b},{relation},{rejected},\
                 {cmp_scan},{cmp_limb},{cmp_touch},\
                 {meet_scan},{meet_limb},{meet_touch},\
                 {join_scan},{join_limb},{join_touch},\
                 {span_scan},{span_limb},{span_touch},\
                 {hull_scan},{hull_limb},{hull_touch}"
            )
            .expect("CSV row writes");
        }
        println!("size {size}: {samples} pairs, {rejected_total} rejection draws");
    }
    csv.into_inner()
        .expect("CSV flushes")
        .sync_all()
        .expect("CSV reaches disk");
    println!("→ {}", out.display());
}
