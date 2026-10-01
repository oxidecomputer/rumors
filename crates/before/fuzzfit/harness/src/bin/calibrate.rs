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
//! The sweep reports within-program slope excess, prefix-refit divergence,
//! floor distance from a no-op, and fixed escalation replays' ceiling excess.
//! Review these measurements against the committed enforcement tolerances
//! when re-pinning. Every fitted key must also be refittable from the prefix
//! with the same constant-or-linear classification.
//!
//! Usage: `calibrate [programs]` (default 4096, the corpus of record; the
//! committed pins state their corpus size per band).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use fuzzfit_harness::bands::{Band, ENFORCE_MARGIN, ENFORCE_MARGIN_BELOW, REFIT_PREFIX_PROGRAMS};
use fuzzfit_harness::curve::{local_slope_excess, MIN_BUCKETS, MIN_PER_BUCKET, SHAPE_EXEMPT};
use fuzzfit_harness::drive::{
    for_each_bootstrap_program, for_each_deterministic_program, run_program,
};
use fuzzfit_harness::fit::{fit, fit_constant, line_divergence, Fit, FIT_FLOOR_BITS};
use fuzzfit_harness::strategies::{build, Family, ESCALATION_REPLAYS};
use fuzzfit_harness::wasm::Guest;

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

/// Fits the deterministic corpora and writes the reviewed band snapshot.
fn main() {
    let programs: usize = std::env::args()
        .nth(1)
        .map(|s| s.parse().expect("programs must be a number"))
        .unwrap_or(4096);

    /// An operation and its success or rejection outcome.
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

    // Bootstrap steps below their main fit's floor determine which success
    // paths need a small band. Pool their main-corpus sub-floor samples with
    // this separate stream; bootstrap samples never enter the main fits.
    let mut small_by_kernel: BTreeMap<&'static str, Vec<(u64, u64)>> = BTreeMap::new();
    for_each_bootstrap_program(|_, samples| {
        for s in samples {
            let key = (s.kernel, s.rejected);
            if s.denom_bits < fits[&key].min_denom {
                assert!(!s.rejected, "bootstrap small bands cover success paths");
                assert!(
                    s.denom_bits < FIT_FLOOR_BITS,
                    "bootstrap operand reaches the fit floor"
                );
                let pooled = small_by_kernel.entry(s.kernel).or_insert_with(|| {
                    by_key[&key]
                        .iter()
                        .copied()
                        .filter(|&(denom, _)| denom < FIT_FLOOR_BITS)
                        .collect()
                });
                pooled.push((s.denom_bits, s.fuel));
            }
        }
    });
    let mut small_rows = String::new();
    let mut small_fits: BTreeMap<&'static str, Fit> = BTreeMap::new();
    for (&kernel, samples) in &small_by_kernel {
        let f = fit_constant(samples)
            .unwrap_or_else(|| panic!("small-band kernel {kernel} needs >= 2 sub-floor samples"));
        writeln!(
            small_rows,
            "    Band {{\n        kernel: \"{kernel}\",\n        rejected: false,\n        \
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
            "{:32} level {:+.3} width +{:.3}/-{:.3} n={:6} denom {}..{} bits [small]",
            format!("{kernel} [small]"),
            f.intercept,
            f.width_above,
            f.width_below,
            f.samples,
            f.min_denom,
            f.max_denom,
        )
        .expect("String write cannot fail");
        small_fits.insert(kernel, f);
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
    // A pin must remain refittable by the enforcement prefix; compare every
    // fitted key directly instead of maintaining another coverage list.
    let mut refit_max: Option<(f64, Key)> = None;
    for (&(kernel, rejected), f) in &fits {
        let prefix = prefix_by_key
            .get(&(kernel, rejected))
            .and_then(|samples| fit(samples))
            .unwrap_or_else(|| panic!("{kernel} (rejected={rejected}): too few prefix samples"));
        assert_eq!(
            prefix.constant, f.constant,
            "{kernel} (rejected={rejected}): prefix classification differs from the full fit"
        );
        let divergence = line_divergence(&prefix, &band_of(kernel, rejected, f));
        if refit_max.as_ref().is_none_or(|(max, _)| divergence > *max) {
            refit_max = Some((divergence, (kernel, rejected)));
        }
    }
    if let Some((divergence, (kernel, rejected))) = refit_max {
        eprintln!(
            "refit evidence: prefix ({REFIT_PREFIX_PROGRAMS} programs) vs pin, maximum divergence {divergence:.3} ({kernel}, rejected={rejected})"
        );
    }
    // The liveness margin's evidence: the narrowest gap, over every band
    // key, between the effective floor (line − width_below −
    // ENFORCE_MARGIN_BELOW, at min_denom — the floor's lowest judged
    // point for a non-negative slope) and the nop-level reading a dead
    // meter produces. The liveness claim rests on every gap staying
    // positive.
    let nop = Guest::new().call("ff_nop", &[]).fuel;
    let nop_log = (nop.max(1) as f64).log10();
    let mut floor_min: Option<(f64, Key)> = None;
    for (&key, f) in &fits {
        let floor = f.intercept + f.slope * (f.min_denom as f64).log10()
            - f.width_below
            - ENFORCE_MARGIN_BELOW;
        let gap = floor - nop_log;
        if floor_min.as_ref().is_none_or(|(m, _)| gap < *m) {
            floor_min = Some((gap, key));
        }
    }
    if let Some((gap, (kernel, rejected))) = &floor_min {
        eprintln!(
            "floor evidence: min floor-vs-nop gap {gap:.3} decades ({kernel}{}) \
             with ENFORCE_MARGIN_BELOW {ENFORCE_MARGIN_BELOW} already subtracted",
            if *rejected { " [err]" } else { "" }
        );
    }
    // The ceiling margin's evidence: the enforcement suite's fixed
    // escalation replays are deterministic enforcement-context programs
    // outside the calibration corpus (different seeds), so their worst
    // ceiling excess — over judged steps, residual minus the fitted
    // ceiling width — is the observed calibration-vs-enforcement gap
    // ENFORCE_MARGIN must absorb. The replays' samples never enter the
    // fits: they are evidence, not pin input.
    let mut ceiling_max: Option<(f64, Key, u32)> = None;
    for (depth, seed) in ESCALATION_REPLAYS {
        let program = build(&Family::Escalation { depth }, seed);
        let samples = run_program(&program)
            .unwrap_or_else(|m| panic!("malformed escalation replay at {}", m.op));
        for s in &samples {
            let key = (s.kernel, s.rejected);
            // Sub-floor steps of the small-band kernels are judged by the
            // small bands in enforcement, so their replay excess is
            // measured against the small ceiling here.
            if s.denom_bits < FIT_FLOOR_BITS && !s.rejected {
                if let Some(f) = small_fits.get(&s.kernel) {
                    if s.denom_bits >= f.min_denom && s.denom_bits <= f.max_denom {
                        let excess = (s.fuel.max(1) as f64).log10() - f.intercept - f.width_above;
                        if ceiling_max.as_ref().is_none_or(|(m, ..)| excess > *m) {
                            ceiling_max = Some((excess, key, depth));
                        }
                    }
                }
            }
            let Some(f) = fits.get(&key) else { continue };
            if s.denom_bits < f.min_denom {
                continue;
            }
            let line = f.intercept + f.slope * (s.denom_bits as f64).log10();
            let excess = (s.fuel.max(1) as f64).log10() - line - f.width_above;
            if ceiling_max.as_ref().is_none_or(|(m, ..)| excess > *m) {
                ceiling_max = Some((excess, key, depth));
            }
        }
    }
    match &ceiling_max {
        Some((excess, (kernel, rejected), depth)) => eprintln!(
            "ceiling evidence: max enforcement-replay ceiling excess {excess:+.3} \
             ({kernel}{}, escalation depth {depth}) against ENFORCE_MARGIN {ENFORCE_MARGIN}",
            if *rejected { " [err]" } else { "" }
        ),
        None => eprintln!("ceiling evidence: the replays produced no judged steps"),
    }

    let bands_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/bands.rs");
    let current = std::fs::read_to_string(&bands_path).expect("bands.rs exists");
    let marker = "/// The toolchain that pinned";
    // The marker line is prose in a generated region: a rewording that
    // loses it must fail here by name, never silently splice the whole
    // file into the head.
    let head = &current[..current
        .find(marker)
        .expect("bands.rs splice marker present")];
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

/// Bootstrap success paths below their main bands' floors, selected from the
/// executed bootstrap corpus and judged over each band's calibrated span.
///
/// Generated by `just fuzzfit-calibrate` alongside [`BANDS`], from the
/// pooled sub-floor samples of the calibration corpus and the
/// deterministic bootstrap stream — review the diff like a snapshot,
/// commit with a dated movement annotation.
pub const SMALL_BANDS: &[Band] = &[
{small_rows}];

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
