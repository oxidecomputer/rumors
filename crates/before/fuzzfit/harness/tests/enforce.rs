//! The enforcement sentry: fuzzed programs must land every measured step
//! inside its pinned fuel band, and no kernel's within-case trend may
//! out-climb its pinned slope.
//!
//! Each case draws one program from the full family roster (shape-biased,
//! combination, cross-universe, and reach regimes), replays it natively and
//! in the wasm guest, and judges two legs: every step's fuel against the
//! committed band for its kernel at its denominated size (the point leg),
//! and every kernel's within-case bucket-median trend against its pinned
//! slope (the shape leg — a mechanism that tilts into a wide band keeps
//! its point residuals small, and only the trend sees it). Fuel
//! determinism makes a failure replay exactly: proptest shrinks to a
//! minimal out-of-band shape and writes a seed file next to this binary —
//! commit any seed that appears (repo hard rule); it is an
//! out-of-band-shape finding of record.
//!
//! Standing self-checks ride along: the meter's liveness (`ff_nop`), the
//! detection path's adequacy (a deliberately quadratic guest burner must
//! read ABOVE a linear band), the pin's provenance (the building toolchain
//! must match the pinning one), and the pin's staleness (a fresh refit of
//! the deterministic corpus prefix must agree with the committed lines).
//!
//! Case count: 48 by default (the calibration corpus is the big sweep; this
//! is the sentry); override with `PROPTEST_CASES`.

use std::collections::BTreeMap;

use proptest::prelude::*;

use fuzzfit_harness::bands::{
    band_for, judge_against, Verdict, BANDS, PINNED_RUSTC, REFIT_MIN_KERNELS,
    REFIT_PREFIX_PROGRAMS, REFIT_TOLERANCE,
};
use fuzzfit_harness::curve::{local_slope_excess, SHAPE_EXEMPT, SLOPE_ALLOWANCE};
use fuzzfit_harness::drive::{for_each_deterministic_program, run_program};
use fuzzfit_harness::fit::{fit, line_divergence};
use fuzzfit_harness::strategies::any_program;
use fuzzfit_harness::wasm::Guest;

/// The pinned bands must exist before enforcement means anything: an empty
/// roster would let every program "pass" vacuously.
#[test]
fn bands_are_pinned() {
    assert!(
        !BANDS.is_empty(),
        "no pinned bands: run `just fuzzfit-calibrate` and commit src/bands.rs"
    );
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

/// The detection path is adequate: a genuinely superlinear mechanism must
/// read ABOVE through the same judge the real kernels face.
///
/// The guest ships a deliberately quadratic self-test burner (not a
/// kernel: no `before` operation runs in it and no strategy emits it; its
/// loop is `black_box`-pinned so codegen cannot strength-reduce it into a
/// closed form). A slope-1 band anchored on its small-input cost must
/// judge its at-scale cost Above, and a stalled reading Below. If this
/// fails, the instrument is blind and every green band is decoration.
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

/// The pin's provenance: fuel constants are a function of the guest
/// codegen, which is a function of the compiler, so judging against bands
/// pinned under a different toolchain compares incommensurable numbers.
/// A toolchain bump reads red here until the bands are re-pinned
/// (`just fuzzfit-calibrate`).
#[test]
fn building_toolchain_matches_the_pin() {
    assert_eq!(
        PINNED_RUSTC,
        env!("FUZZFIT_RUSTC_VERSION"),
        "toolchain drift: re-pin the bands with `just fuzzfit-calibrate`"
    );
}

/// The pin's staleness cross-check: the bands are computable two ways —
/// the committed constants and a fresh fit of the same deterministic
/// sample stream — so the two get compared, and disagreement beyond the
/// measured tolerance demands a deliberate re-pin with a movement
/// annotation, never silent drift.
///
/// Refits the first [`REFIT_PREFIX_PROGRAMS`] programs of the calibration
/// stream and compares each kernel's fresh line against its pin
/// ([`line_divergence`]); kernels whose prefix sample is too thin to
/// share the pin's classification are skipped, and the coverage floor
/// keeps that skipping from hollowing the check out.
#[test]
fn refit_of_the_deterministic_prefix_matches_the_pin() {
    let mut by_kernel: BTreeMap<&'static str, Vec<(u64, u64)>> = BTreeMap::new();
    for_each_deterministic_program(REFIT_PREFIX_PROGRAMS, |_, _, samples| {
        for s in samples {
            by_kernel
                .entry(s.kernel)
                .or_default()
                .push((s.denom_bits, s.fuel));
        }
    });
    let mut compared = 0usize;
    for (kernel, samples) in &by_kernel {
        let Some(f) = fit(samples) else { continue };
        let band = band_for(kernel)
            .unwrap_or_else(|| panic!("{kernel} has no pinned band: re-pin (totality)"));
        if f.constant != band.constant {
            continue;
        }
        let d = line_divergence(&f, band);
        assert!(
            d <= REFIT_TOLERANCE,
            "STALE PIN: {kernel}'s prefix refit diverges from its pin by {d:.3} \
             (tolerance {REFIT_TOLERANCE}); re-pin with `just fuzzfit-calibrate` \
             and annotate the movement"
        );
        compared += 1;
    }
    assert!(
        compared >= REFIT_MIN_KERNELS,
        "staleness check compared only {compared} kernels (floor {REFIT_MIN_KERNELS}): \
         the prefix no longer covers the surface"
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Every public operation stays inside its pinned fuel band on shapes
    /// nobody chose — random programs over the whole vocabulary, coupled
    /// and cross-universe operand regimes alike — and no kernel's
    /// within-case cost trend out-climbs its pinned slope.
    ///
    /// Above-band is an asymptotic regression; below-band is a liveness
    /// failure; a kernel with no band is an unpriced operation (totality);
    /// a rising within-case trend is a superlinear mechanism hiding inside
    /// a wide band (the shape leg).
    #[test]
    fn fuel_stays_in_the_pinned_bands(program in any_program()) {
        let samples = run_program(&program)
            .unwrap_or_else(|m| panic!("malformed program at {}", m.op));
        let mut by_kernel: BTreeMap<&'static str, Vec<(u64, u64)>> = BTreeMap::new();
        for s in &samples {
            let band = band_for(s.kernel).unwrap_or_else(|| {
                panic!(
                    "{} has no pinned band: re-run `just fuzzfit-calibrate`",
                    s.kernel
                )
            });
            match judge_against(band, s.denom_bits, s.fuel) {
                Verdict::InBand | Verdict::BelowFloor => {}
                Verdict::Above => prop_assert!(
                    false,
                    "ABOVE BAND (asymptotic regression): {} at {} bits consumed {} fuel; \
                     the pinned law predicts ~10^{:.3} +{:.3}/-{:.3}",
                    s.kernel,
                    s.denom_bits,
                    s.fuel,
                    band.intercept + band.slope * (s.denom_bits as f64).log10(),
                    band.width_above,
                    band.width_below,
                ),
                Verdict::Below => prop_assert!(
                    false,
                    "BELOW BAND (liveness): {} at {} bits consumed only {} fuel; \
                     the pinned law predicts ~10^{:.3} +{:.3}/-{:.3}",
                    s.kernel,
                    s.denom_bits,
                    s.fuel,
                    band.intercept + band.slope * (s.denom_bits as f64).log10(),
                    band.width_above,
                    band.width_below,
                ),
            }
            by_kernel.entry(s.kernel).or_default().push((s.denom_bits, s.fuel));
        }
        // The shape leg: within one case the population is family-pure,
        // so a rising bucket-median trend is the mechanism's own
        // curvature, not mixture tilt. The fold rows are exempt (their
        // honest law trends along the width axis; the point leg owns
        // them).
        for (kernel, group) in &by_kernel {
            if SHAPE_EXEMPT.contains(kernel) {
                continue;
            }
            let band = band_for(kernel).expect("checked above");
            if let Some(excess) = local_slope_excess(band, group) {
                prop_assert!(
                    excess <= SLOPE_ALLOWANCE,
                    "RISING TREND (shape leg): {} climbs {:+.3} past its pinned slope \
                     {:.3} within one case (allowance {})",
                    kernel,
                    excess,
                    band.slope,
                    SLOPE_ALLOWANCE,
                );
            }
        }
    }
}
