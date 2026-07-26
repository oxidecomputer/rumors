//! The calibration sweep: sample every family, fit per-kernel bands, and
//! rewrite `src/bands.rs` (atomically) with the pinned result.
//!
//! Deterministic end to end: family draws come from proptest's
//! deterministic runner, program seeds are the case index, and fuel is
//! wasmtime's deterministic count — so two sweeps at the same code produce
//! byte-identical bands, and a bands diff always means a real change
//! (guest codegen, kernels, strategies), never noise.
//!
//! Alongside the pins, the sweep prints (to stderr) the measurements the
//! committed judgment constants are pinned from: the corpus's maximum
//! healthy within-case slope excess (the shape leg's `SLOPE_ALLOWANCE`)
//! and the prefix-refit-vs-pin line divergence (the staleness check's
//! `REFIT_TOLERANCE` and its coverage floor) — so a re-pin re-derives the
//! constants' evidence instead of trusting last time's.
//!
//! Usage: `calibrate [programs]` (default 1536, the corpus of record; the
//! committed pins state their corpus size per kernel).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use fuzzfit_harness::bands::{Band, REFIT_PREFIX_PROGRAMS};
use fuzzfit_harness::curve::{local_slope_excess, MIN_BUCKETS, MIN_PER_BUCKET, SHAPE_EXEMPT};
use fuzzfit_harness::drive::for_each_deterministic_program;
use fuzzfit_harness::fit::{fit, line_divergence, Fit};

/// A [`Band`] transcribing a fresh [`Fit`] (what the pin will say).
fn band_of(kernel: &'static str, f: &Fit) -> Band {
    Band {
        kernel,
        slope: f.slope,
        intercept: f.intercept,
        width_above: f.width_above,
        width_below: f.width_below,
        min_denom: f.min_denom,
        max_denom: f.max_denom,
        samples: f.samples,
        constant: f.constant,
    }
}

fn main() {
    let programs: usize = std::env::args()
        .nth(1)
        .map(|s| s.parse().expect("programs must be a number"))
        .unwrap_or(1536);

    let mut by_kernel: BTreeMap<&'static str, Vec<(u64, u64)>> = BTreeMap::new();
    let mut prefix_by_kernel: BTreeMap<&'static str, Vec<(u64, u64)>> = BTreeMap::new();
    // Per-case sample groups big enough to possibly carry shape evidence
    // — (case, family, kernel, samples) — held back so the shape
    // diagnostic can judge them against the *new* fits once those exist.
    #[allow(clippy::type_complexity)]
    let mut shape_cases: Vec<(usize, String, &'static str, Vec<(u64, u64)>)> = Vec::new();
    let mut total_steps = 0usize;
    for_each_deterministic_program(programs, |case, family, samples| {
        total_steps += samples.len();
        let mut case_kernels: BTreeMap<&'static str, Vec<(u64, u64)>> = BTreeMap::new();
        for s in samples {
            by_kernel
                .entry(s.kernel)
                .or_default()
                .push((s.denom_bits, s.fuel));
            if case < REFIT_PREFIX_PROGRAMS {
                prefix_by_kernel
                    .entry(s.kernel)
                    .or_default()
                    .push((s.denom_bits, s.fuel));
            }
            case_kernels
                .entry(s.kernel)
                .or_default()
                .push((s.denom_bits, s.fuel));
        }
        for (kernel, group) in case_kernels {
            if !SHAPE_EXEMPT.contains(&kernel) && group.len() >= MIN_BUCKETS * MIN_PER_BUCKET {
                shape_cases.push((case, format!("{family:?}"), kernel, group));
            }
        }
        if (case + 1) % 32 == 0 {
            eprintln!("… {}/{programs} programs, {total_steps} samples", case + 1);
        }
    });

    let mut rows = String::new();
    let mut table = String::new();
    let mut fits: BTreeMap<&'static str, Fit> = BTreeMap::new();
    for (kernel, samples) in &by_kernel {
        let f = fit(samples).expect("every kernel has ≥ 2 samples");
        fits.insert(kernel, f);
        writeln!(
            rows,
            "    Band {{\n        kernel: \"{kernel}\",\n        slope: {:.6},\n        \
             intercept: {:.6},\n        width_above: {:.6},\n        width_below: {:.6},\n        \
             min_denom: {},\n        max_denom: {},\n        samples: {},\n        \
             constant: {},\n    }},",
            f.slope,
            f.intercept,
            f.width_above,
            f.width_below,
            f.min_denom,
            f.max_denom,
            f.samples,
            f.constant
        )
        .expect("String write cannot fail");
        writeln!(
            table,
            "{kernel:26} slope {:+.3} width +{:.3}/-{:.3} n={:6} denom {}..{} bits{}",
            f.slope,
            f.width_above,
            f.width_below,
            f.samples,
            f.min_denom,
            f.max_denom,
            if f.constant { " [constant]" } else { "" }
        )
        .expect("String write cannot fail");
    }

    // ── judgment-constant evidence (stderr; never part of the pin file) ──
    // The shape leg's ceiling: the maximum healthy within-case slope
    // excess anywhere in the corpus, judged against the *new* fits.
    let mut shape_max: Option<(f64, &'static str, usize, String)> = None;
    for (case, family, kernel, group) in &shape_cases {
        let band = band_of(kernel, &fits[kernel]);
        if let Some(excess) = local_slope_excess(&band, group) {
            if shape_max.as_ref().is_none_or(|(m, ..)| excess > *m) {
                shape_max = Some((excess, kernel, *case, family.clone()));
            }
        }
    }
    match &shape_max {
        Some((excess, kernel, case, family)) => eprintln!(
            "shape-leg evidence: max healthy within-case slope excess {excess:+.3} \
             ({kernel}, case {case}, {family})"
        ),
        None => eprintln!("shape-leg evidence: no case met the evidence requirements"),
    }
    // The staleness check's evidence: prefix refit vs the new pin.
    let mut refit_max: Option<(f64, &'static str)> = None;
    let mut refit_compared = 0usize;
    for (kernel, samples) in &prefix_by_kernel {
        let Some(pf) = fit(samples) else { continue };
        let f = &fits[kernel];
        if pf.constant != f.constant {
            continue;
        }
        let d = line_divergence(&pf, &band_of(kernel, f));
        refit_compared += 1;
        if refit_max.as_ref().is_none_or(|(m, _)| d > *m) {
            refit_max = Some((d, kernel));
        }
    }
    match &refit_max {
        Some((d, kernel)) => eprintln!(
            "refit evidence: prefix ({REFIT_PREFIX_PROGRAMS} programs) vs pin, \
             max line divergence {d:.3} ({kernel}), {refit_compared} kernels compared"
        ),
        None => eprintln!("refit evidence: prefix compared no kernels"),
    }

    let bands_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/bands.rs");
    let current = std::fs::read_to_string(&bands_path).expect("bands.rs exists");
    let marker = "/// The toolchain that pinned";
    let head = current
        .split(marker)
        .next()
        .expect("split always yields a first piece");
    let rustc = env!("FUZZFIT_RUSTC_VERSION");
    // A plain multi-line literal (continuation lines at column zero): the
    // emitted `///` lines keep their paragraph structure both in the
    // written file and here in the source, where the doc-summary linter
    // reads them too.
    let new = format!(
        "{head}{marker} the constants below, as `rustc --version`
/// reports it.
///
/// Generated by `just fuzzfit-calibrate` alongside [`BANDS`]: guest
/// codegen (and so every fuel constant) is a function of this compiler,
/// and the suite asserts the building toolchain matches, so a toolchain
/// bump reads red until the bands are re-pinned. wasmtime (the fuel
/// schedule's other half) is pinned exactly by the workspace
/// `Cargo.lock`; bumping it there is likewise a re-pin event.
pub const PINNED_RUSTC: &str = \"{rustc}\";

/// The pinned bands.
///
/// Generated by `just fuzzfit-calibrate` — review the diff like a
/// snapshot, commit with a dated movement annotation.
pub const BANDS: &[Band] = &[
{rows}];
"
    );
    let tmp = bands_path.with_extension("rs.tmp");
    std::fs::write(&tmp, &new).expect("write bands.rs.tmp");
    std::fs::rename(&tmp, &bands_path).expect("move bands.rs into place");

    println!(
        "calibrated {} kernels from {programs} programs ({total_steps} steps):",
        by_kernel.len()
    );
    print!("{table}");
    println!("wrote {}", bands_path.display());
}
