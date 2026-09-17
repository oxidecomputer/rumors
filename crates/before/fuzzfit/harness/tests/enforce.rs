//! Checks generated programs against their pinned fuel bounds.
//!
//! Each case generates one single-universe program, replays it natively and
//! in the wasm guest, and checks two properties. Every operation's fuel must
//! remain inside the committed band for its operation and outcome at that
//! operand size. Within a case, each operation's bucket-median trend must
//! also remain below its pinned slope. The trend check catches growth that
//! a wide pointwise band could hide. Fuel is deterministic, so proptest can
//! shrink failures to a reproducible program; every resulting seed file
//! must be committed.
//!
//! The deterministic corpus prefix rides the same judgment, program by
//! program, every run: the random draws probe novelty, and the prefix
//! leg makes every kernel × size-decade region the corpus reaches an
//! enforced verdict rather than a sampled one. A region drawn with
//! probability `q` still escapes `n` random cases with probability
//! `(1 − q)^n`; the deterministic prefix removes that uncertainty for the
//! regions it covers.
//!
//! Standing self-checks ride along: the meter's liveness (`ff_nop`), the
//! detection path's adequacy (a deliberately quadratic guest burner must
//! read ABOVE a linear band), the pin's provenance (the building toolchain
//! must match the pinning one), the pin's staleness (a fresh refit of the
//! deterministic corpus prefix must agree with the committed lines on
//! every covered band key), and the escalated regime itself (two fixed
//! reach-family programs — mid-depth and the depth cap, distinct seeds —
//! replay on every run, so the deep rejection and overlap cases do not
//! depend on rare escalation draws).
//!
//! The property test runs proptest's default case count, raised with
//! `PROPTEST_CASES`; the calibration corpus is the big sweep.

use std::collections::BTreeMap;

use proptest::prelude::*;

use fuzzfit_harness::bands::{
    band_for, judge_against, judge_small, Verdict, BANDS, PINNED_RUSTC, REFIT_COVERAGE,
    REFIT_PREFIX_PROGRAMS, REFIT_TOLERANCE, SMALL_BANDS, SMALL_BAND_KERNELS,
};
use fuzzfit_harness::curve::{local_slope_excess, SHAPE_EXEMPT, SLOPE_ALLOWANCE};
use fuzzfit_harness::drive::{
    for_each_bootstrap_program, for_each_deterministic_program, run_program, Sample,
};
use fuzzfit_harness::strategies::{any_program, build, Family, ESCALATION_REPLAYS};
use fuzzfit_harness::wasm::Guest;

/// Checks one program's pointwise fuel and within-case growth, panicking on
/// any violation.
fn judge(samples: &[Sample]) {
    let mut by_key: BTreeMap<(&'static str, bool), Vec<(u64, u64)>> = BTreeMap::new();
    for s in samples {
        let band = band_for(s.kernel, s.rejected).unwrap_or_else(|| {
            panic!(
                "{}{} has no pinned band: re-run `just fuzzfit-calibrate`",
                s.kernel,
                if s.rejected { " [err]" } else { "" },
            )
        });
        let arm = if s.rejected { " [err]" } else { "" };
        match judge_against(band, s.denom_bits, s.fuel) {
            Verdict::InBand => {}
            // Below the size-law floor, the small-operand roster takes
            // over where it prices the key: sub-floor steps of the
            // bootstrap-hot kernels are judged against their constant
            // band instead of skipped.
            Verdict::BelowFloor => {
                if let Some((small, verdict)) =
                    judge_small(s.kernel, s.rejected, s.denom_bits, s.fuel)
                {
                    match verdict {
                        Verdict::InBand | Verdict::BelowFloor => {}
                        Verdict::Above => panic!(
                            "ABOVE SMALL BAND (sub-floor regression): {}{arm} at {} bits \
                             consumed {} fuel; the pinned constant level is ~10^{:.3} \
                             +{:.3}/-{:.3} over {}..{} bits",
                            s.kernel,
                            s.denom_bits,
                            s.fuel,
                            small.intercept,
                            small.width_above,
                            small.width_below,
                            small.min_denom,
                            small.max_denom,
                        ),
                        Verdict::Below => panic!(
                            "BELOW SMALL BAND (liveness): {}{arm} at {} bits consumed \
                             only {} fuel; the pinned constant level is ~10^{:.3} \
                             +{:.3}/-{:.3} over {}..{} bits",
                            s.kernel,
                            s.denom_bits,
                            s.fuel,
                            small.intercept,
                            small.width_above,
                            small.width_below,
                            small.min_denom,
                            small.max_denom,
                        ),
                    }
                }
            }
            Verdict::Above => panic!(
                "ABOVE BAND (asymptotic regression): {}{arm} at {} bits consumed {} fuel; \
                 the pinned law predicts ~10^{:.3} +{:.3}/-{:.3}",
                s.kernel,
                s.denom_bits,
                s.fuel,
                band.intercept + band.slope * (s.denom_bits as f64).log10(),
                band.width_above,
                band.width_below,
            ),
            Verdict::Below => panic!(
                "BELOW BAND (liveness): {}{arm} at {} bits consumed only {} fuel; \
                 the pinned law predicts ~10^{:.3} +{:.3}/-{:.3}",
                s.kernel,
                s.denom_bits,
                s.fuel,
                band.intercept + band.slope * (s.denom_bits as f64).log10(),
                band.width_above,
                band.width_below,
            ),
        }
        by_key
            .entry((s.kernel, s.rejected))
            .or_default()
            .push((s.denom_bits, s.fuel));
    }
    // The shape leg: within one case the population is family-pure,
    // so a rising bucket-median trend is the mechanism's own
    // curvature, not mixture tilt. Fold operations are exempt because
    // their expected cost grows along the width axis; their pointwise
    // bands enforce the bound instead.
    for (&(kernel, rejected), group) in &by_key {
        if SHAPE_EXEMPT.contains(&kernel) {
            continue;
        }
        let band = band_for(kernel, rejected).expect("checked above");
        if let Some(excess) = local_slope_excess(band, group) {
            assert!(
                excess <= SLOPE_ALLOWANCE,
                "RISING TREND (shape leg): {kernel}{} climbs {excess:+.3} past its pinned \
                 slope {:.3} within one case (allowance {SLOPE_ALLOWANCE})",
                if rejected { " [err]" } else { "" },
                band.slope,
            );
        }
    }
}

/// The pinned bands must exist before enforcement means anything: an empty
/// roster would let every program "pass" vacuously.
#[test]
fn bands_are_pinned() {
    assert!(
        !BANDS.is_empty(),
        "no pinned bands: run `just fuzzfit-calibrate` and commit src/bands.rs"
    );
}

/// The small-operand roster is pinned exactly: one constant-classified
/// band per [`SMALL_BAND_KERNELS`] entry (success arm).
///
/// Each band judges strictly below the size-law fit floor, and no
/// small band prices a kernel off the roster.
///
/// The roster is the committed expectation list: a calibration that
/// drops a kernel's small band (a generator regression starving the
/// sub-floor samples) fails here by name instead of silently reopening
/// the sub-floor blind spot the bands exist to close.
#[test]
fn small_bands_are_pinned_for_the_bootstrap_kernels() {
    for &kernel in SMALL_BAND_KERNELS {
        let band = SMALL_BANDS
            .iter()
            .find(|b| b.kernel == kernel && !b.rejected)
            .unwrap_or_else(|| {
                panic!(
                    "{kernel} has no pinned small-operand band: re-run \
                     `just fuzzfit-calibrate` and commit src/bands.rs"
                )
            });
        assert!(
            band.constant,
            "{kernel}: a small band is constant-classified"
        );
        assert!(
            band.max_denom < fuzzfit_harness::fit::FIT_FLOOR_BITS,
            "{kernel}: a small band judges strictly below the fit floor"
        );
    }
    for band in SMALL_BANDS {
        assert!(
            SMALL_BAND_KERNELS.contains(&band.kernel) && !band.rejected,
            "small band {} prices no rostered kernel: a stale pin; re-run \
             `just fuzzfit-calibrate` or extend SMALL_BAND_KERNELS",
            band.kernel
        );
    }
}

/// The deterministic bootstrap corpus lands every step in its band,
/// and each rostered kernel is actually judged sub-floor at least
/// once.
///
/// The sub-floor steps of the rostered kernels judge against their
/// small bands, everything else against the main roster; the
/// at-least-once demand is the leg's own liveness floor (a small band
/// no program ever lands in is decoration).
///
/// This is the deterministic verdict over rumors' production-hot
/// operand region: bootstrap and per-message stamping live below the
/// size-law fit floor, where the point, shape, and refit legs are all
/// structurally out of range, so this replay is the region's only
/// total judgment.
#[test]
fn the_bootstrap_regime_stays_in_the_pinned_small_bands() {
    let mut judged_small: BTreeMap<&'static str, usize> = BTreeMap::new();
    for_each_bootstrap_program(|_, samples| {
        judge(samples);
        for s in samples {
            if !s.rejected && judge_small(s.kernel, s.rejected, s.denom_bits, s.fuel).is_some() {
                *judged_small.entry(s.kernel).or_default() += 1;
            }
        }
    });
    for &kernel in SMALL_BAND_KERNELS {
        assert!(
            judged_small.get(kernel).copied().unwrap_or(0) > 0,
            "{kernel}: the bootstrap corpus never landed a step inside its small \
             band's calibrated span — the leg is decoration for this kernel; \
             re-derive the corpus or the band"
        );
    }
}

/// The fuel meter itself is alive: an empty kernel call costs a small,
/// positive, exact amount.
///
/// A zero here means fuel accounting is off (every ceiling would pass
/// vacuously); a large value means call overhead has grown into the
/// measurements.
#[test]
fn fuel_metering_is_live() {
    let mut guest = Guest::new();
    let nop = guest.call("ff_nop", &[]);
    assert!(nop.fuel > 0, "nop consumed no fuel: metering is dead");
    assert!(
        nop.fuel < 100,
        "nop consumed {} fuel: call overhead has grown",
        nop.fuel
    );
}

/// The detection path is live end to end: a genuinely superlinear
/// mechanism must read ABOVE through the same judge the real kernels face.
///
/// The guest ships a deliberately quadratic self-test burner (not a
/// kernel: no `before` operation runs in it and no strategy emits it; its
/// loop is `black_box`-pinned so codegen cannot strength-reduce it into a
/// closed form). A slope-1 band anchored on its small-input cost must
/// judge its at-scale cost Above, and a stalled reading Below. This
/// proves the wasm-execution → fuel-metering → judgment path can flag a
/// quadratic at all; whether the *generators* place real kernels where a
/// regression must flag is the reach families' and the demonstrations
/// ledger's business, not this check's.
#[test]
fn a_live_quadratic_reads_above_a_linear_band() {
    let mut guest = Guest::new();
    let small = guest.call("ff_selftest_quadratic", &[64]).fuel;
    let mid = guest.call("ff_selftest_quadratic", &[128]).fuel;
    // The mechanism itself is alive and superlinear in this codegen: a
    // doubled input must cost well over double (a strength-reduced
    // closed form would read ~flat and fail here).
    assert!(
        mid > small * 3,
        "self-test burner is not superlinear: {small} -> {mid} fuel"
    );
    // The most charitable linear law the anchor supports: slope 1 through
    // the mid reading, with a generous width.
    let band = fuzzfit_harness::bands::Band {
        kernel: "ff_selftest_quadratic",
        rejected: false,
        slope: 1.0,
        intercept: (mid as f64).log10() - 128f64.log10(),
        width_above: 0.5,
        width_below: 0.5,
        min_denom: 64,
        max_denom: 8192,
        samples: 2,
        constant: false,
    };
    assert_eq!(judge_against(&band, 128, mid), Verdict::InBand);
    // 64x the anchor: quadratic growth stands ~1.8 decades above the
    // linear prediction, far past width + margin.
    let big = guest.call("ff_selftest_quadratic", &[8192]).fuel;
    assert_eq!(
        judge_against(&band, 8192, big),
        Verdict::Above,
        "quadratic growth did not read ABOVE: {big} fuel at 8192"
    );
    // And the liveness direction through the same band: a reading that
    // stopped growing reads Below.
    assert_eq!(judge_against(&band, 8192, mid), Verdict::Below);
}

/// Ensures the current compiler matches the one used to derive the bands.
///
/// Fuel constants are a function of the guest codegen, which is a
/// function of the compiler, so judging against bands pinned under a
/// different toolchain compares incommensurable numbers. A toolchain
/// bump fails here until the bands are re-pinned
/// (`just fuzzfit-calibrate`).
#[test]
fn building_toolchain_matches_the_pin() {
    assert_eq!(
        PINNED_RUSTC,
        env!("FUZZFIT_RUSTC_VERSION"),
        "toolchain drift: re-pin the bands with `just fuzzfit-calibrate`"
    );
}

/// Ensures the deterministic corpus fits every covered pinned band.
///
/// Every step in the first [`REFIT_PREFIX_PROGRAMS`] programs receives the
/// same pointwise and trend checks as a generated case. This makes the
/// corpus's operation-by-size coverage deterministic while generated cases
/// continue exploring new shapes.
///
/// The test also refits the corpus. Every key in [`REFIT_COVERAGE`] must
/// remain measurable, retain its constant-or-linear classification, and
/// agree with its pinned line within [`REFIT_TOLERANCE`].
#[test]
fn the_deterministic_prefix_is_judged_total_and_matches_the_pin() {
    let mut by_key: BTreeMap<(&'static str, bool), Vec<(u64, u64)>> = BTreeMap::new();
    for_each_deterministic_program(REFIT_PREFIX_PROGRAMS, |_, _, samples| {
        judge(samples);
        for s in samples {
            by_key
                .entry((s.kernel, s.rejected))
                .or_default()
                .push((s.denom_bits, s.fuel));
        }
    });
    for &(kernel, rejected) in REFIT_COVERAGE {
        let arm = if rejected { " [err]" } else { "" };
        let f = by_key
            .get(&(kernel, rejected))
            .and_then(|samples| fuzzfit_harness::fit::fit(samples))
            .unwrap_or_else(|| {
                panic!(
                    "STALE PIN (coverage decay): {kernel}{arm} no longer fits in the \
                     deterministic prefix; re-pin with `just fuzzfit-calibrate` and \
                     annotate the movement"
                )
            });
        let band = band_for(kernel, rejected)
            .unwrap_or_else(|| panic!("{kernel}{arm} is covered but has no pinned band"));
        assert_eq!(
            f.constant, band.constant,
            "STALE PIN (classification flip): {kernel}{arm}'s prefix refit reads \
             constant={} against the pin's constant={} — a reach regression; re-pin \
             with `just fuzzfit-calibrate` and annotate the movement",
            f.constant, band.constant,
        );
        let d = fuzzfit_harness::fit::line_divergence(&f, band);
        assert!(
            d <= REFIT_TOLERANCE,
            "STALE PIN: {kernel}{arm}'s prefix refit diverges from its pin by {d:.3} \
             (tolerance {REFIT_TOLERANCE}); re-pin with `just fuzzfit-calibrate` \
             and annotate the movement"
        );
    }
}

/// Ensures a fixed mid-depth escalation program satisfies every fuel bound.
///
/// Generated cases select this family only about once in 137 draws. The
/// fixed replay deterministically exercises the family's large operands,
/// rejection outcomes, and overlap scans.
#[test]
fn the_escalated_regime_stays_in_the_pinned_bands() {
    let (depth, seed) = ESCALATION_REPLAYS[0];
    let program = build(&Family::Escalation { depth }, seed);
    let samples = run_program(&program)
        .unwrap_or_else(|m| panic!("malformed escalation program at {}", m.op));
    judge(&samples);
}

/// Ensures a fixed depth-cap escalation program satisfies every fuel bound.
///
/// This second replay covers the deepest constructible spine with a
/// different seed, complementing the mid-depth replay above.
#[test]
fn the_escalation_depth_cap_stays_in_the_pinned_bands() {
    let (depth, seed) = ESCALATION_REPLAYS[1];
    let program = build(&Family::Escalation { depth }, seed);
    let samples = run_program(&program)
        .unwrap_or_else(|m| panic!("malformed escalation program at {}", m.op));
    judge(&samples);
}

proptest! {
    /// Every public operation stays inside its pinned fuel band on
    /// shapes nobody chose, and no band key's within-case cost trend
    /// out-climbs its pinned slope.
    ///
    /// Cases draw random programs over the whole vocabulary within one
    /// universe. Success and rejection outcomes are checked against their
    /// respective pinned laws.
    ///
    /// Above-band is an asymptotic regression; below-band is a liveness
    /// failure; a band key with no band is an unpriced operation
    /// (totality); a rising within-case trend is a superlinear mechanism
    /// hiding inside a wide band (the shape leg).
    #[test]
    fn fuel_stays_in_the_pinned_bands(program in any_program()) {
        let samples = run_program(&program)
            .unwrap_or_else(|m| panic!("malformed program at {}", m.op));
        judge(&samples);
    }
}
