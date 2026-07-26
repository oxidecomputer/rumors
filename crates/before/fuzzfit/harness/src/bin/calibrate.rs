//! The calibration sweep: sample every family, fit per-band-key fuel lines,
//! and rewrite `src/bands.rs` (atomically) with the pinned result.
//!
//! Deterministic end to end: family draws come from proptest's
//! deterministic runner, program seeds are the case index, and fuel is
//! wasmtime's deterministic count — so two sweeps at the same code produce
//! byte-identical bands, and a bands diff always means a real change
//! (guest codegen, kernels, strategies), never noise.
//!
//! The band key is kernel × outcome: rejection arms (`ERR_OP` outcomes)
//! are fitted and pinned separately from success paths, so a regression in
//! either arm is judged against its own law.
//!
//! Alongside the pins, the sweep prints (to stderr) the measurements the
//! committed judgment constants are pinned from: the corpus's maximum
//! healthy within-case slope excess (the shape leg's `SLOPE_ALLOWANCE`),
//! the prefix-refit-vs-pin line divergence (the staleness check's
//! `REFIT_TOLERANCE`), and the band keys the prefix leaves uncovered
//! (the complement of the generated `REFIT_COVERAGE` list) — so a re-pin
//! re-derives the constants' evidence instead of trusting last time's.
//!
//! Usage: `calibrate [programs]` (default 1536, the corpus of record; the
//! committed pins state their corpus size per band).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use fuzzfit_harness::bands::{Band, REFIT_PREFIX_PROGRAMS};
use fuzzfit_harness::curve::{local_slope_excess, MIN_BUCKETS, MIN_PER_BUCKET, SHAPE_EXEMPT};
use fuzzfit_harness::drive::for_each_deterministic_program;
use fuzzfit_harness::fit::{fit, line_divergence, Fit};

/// A [`Band`] transcribing a fresh [`Fit`] (what the pin will say).
fn band_of(kernel: &'static str, rejected: bool, f: &Fit) -> Band {
    Band {
        kernel,
        rejected,
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

    type Key = (&'static str, bool);
    let mut by_key: BTreeMap<Key, Vec<(u64, u64)>> = BTreeMap::new();
    let mut prefix_by_key: BTreeMap<Key, Vec<(u64, u64)>> = BTreeMap::new();
    // Per-case sample groups big enough to possibly carry shape evidence
    // — (case, family, key, samples) — held back so the shape
    // diagnostic can judge them against the *new* fits once those exist.
    #[allow(clippy::type_complexity)]
    let mut shape_cases: Vec<(usize, String, Key, Vec<(u64, u64)>)> = Vec::new();
    let mut total_steps = 0usize;
    for_each_deterministic_program(programs, |case, family, samples| {
        total_steps += samples.len();
        let mut case_keys: BTreeMap<Key, Vec<(u64, u64)>> = BTreeMap::new();
        for s in samples {
            let key = (s.kernel, s.rejected);
            by_key.entry(key).or_default().push((s.denom_bits, s.fuel));
            if case < REFIT_PREFIX_PROGRAMS {
                prefix_by_key
                    .entry(key)
                    .or_default()
                    .push((s.denom_bits, s.fuel));
            }
            case_keys
                .entry(key)
                .or_default()
                .push((s.denom_bits, s.fuel));
        }
        for (key, group) in case_keys {
            if !SHAPE_EXEMPT.contains(&key.0) && group.len() >= MIN_BUCKETS * MIN_PER_BUCKET {
                shape_cases.push((case, format!("{family:?}"), key, group));
            }
        }
        if (case + 1) % 32 == 0 {
            eprintln!("… {}/{programs} programs, {total_steps} samples", case + 1);
        }
    });

    let mut rows = String::new();
    let mut table = String::new();
    let mut fits: BTreeMap<Key, Fit> = BTreeMap::new();
    for (&(kernel, rejected), samples) in &by_key {
        let f = fit(samples).expect("every band key has ≥ 2 samples");
        fits.insert((kernel, rejected), f);
        writeln!(
            rows,
            "    Band {{\n        kernel: \"{kernel}\",\n        rejected: {rejected},\n        \
             slope: {:.6},\n        \
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
            "{:32} slope {:+.3} width +{:.3}/-{:.3} n={:6} denom {}..{} bits{}",
            format!("{kernel}{}", if rejected { " [err]" } else { "" }),
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
    let mut shape_max: Option<(f64, Key, usize, String)> = None;
    for (case, family, key, group) in &shape_cases {
        let band = band_of(key.0, key.1, &fits[key]);
        if let Some(excess) = local_slope_excess(&band, group) {
            if shape_max.as_ref().is_none_or(|(m, ..)| excess > *m) {
                shape_max = Some((excess, *key, *case, family.clone()));
            }
        }
    }
    match &shape_max {
        Some((excess, (kernel, rejected), case, family)) => eprintln!(
            "shape-leg evidence: max healthy within-case slope excess {excess:+.3} \
             ({kernel}{}, case {case}, {family})",
            if *rejected { " [err]" } else { "" }
        ),
        None => eprintln!("shape-leg evidence: no case met the evidence requirements"),
    }
    // The staleness check's evidence: prefix refit vs the new pin, and the
    // committed coverage list (the keys whose prefix classification matches
    // the pin's; everything else is printed as uncovered for review).
    let mut refit_max: Option<(f64, Key)> = None;
    let mut coverage = String::new();
    let mut covered = 0usize;
    for (&(kernel, rejected), f) in &fits {
        let flip = match prefix_by_key.get(&(kernel, rejected)).and_then(|s| fit(s)) {
            Some(pf) if pf.constant == f.constant => {
                let d = line_divergence(&pf, &band_of(kernel, rejected, f));
                if refit_max.as_ref().is_none_or(|(m, _)| d > *m) {
                    refit_max = Some((d, (kernel, rejected)));
                }
                writeln!(coverage, "    (\"{kernel}\", {rejected}),")
                    .expect("String write cannot fail");
                covered += 1;
                continue;
            }
            Some(_) => "classification flip",
            None => "too few prefix samples",
        };
        eprintln!(
            "refit coverage: {kernel}{} UNCOVERED ({flip})",
            if rejected { " [err]" } else { "" }
        );
    }
    match &refit_max {
        Some((d, (kernel, rejected))) => eprintln!(
            "refit evidence: prefix ({REFIT_PREFIX_PROGRAMS} programs) vs pin, \
             max line divergence {d:.3} ({kernel}{}), {covered} of {} band keys covered",
            if *rejected { " [err]" } else { "" },
            fits.len()
        ),
        None => eprintln!("refit evidence: prefix covered no band keys"),
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

/// The band keys the pin-time prefix refit covered: the staleness
/// cross-check's committed expectation list.
///
/// Generated by `just fuzzfit-calibrate` alongside [`BANDS`]: every key
/// listed here had, at pin time, a prefix refit whose classification
/// matched its pin. The enforcement suite requires each listed key to
/// still fit, still match its pin's classification, and still agree
/// within [`REFIT_TOLERANCE`] — so coverage decay, a classification flip
/// (the reach-regression tell), and line drift each fail by name instead
/// of hollowing the check out silently. Keys not listed are outside the
/// staleness detector's reach at pin time; calibration prints them for
/// the re-pinner to review.
pub const REFIT_COVERAGE: &[(&str, bool)] = &[
{coverage}];
"
    );
    let tmp = bands_path.with_extension("rs.tmp");
    std::fs::write(&tmp, &new).expect("write bands.rs.tmp");
    std::fs::rename(&tmp, &bands_path).expect("move bands.rs into place");

    println!(
        "calibrated {} band keys from {programs} programs ({total_steps} steps):",
        by_key.len()
    );
    print!("{table}");
    println!("wrote {}", bands_path.display());
}
