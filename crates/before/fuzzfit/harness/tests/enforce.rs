//! The enforcement sentry: fuzzed programs must land every measured step
//! inside its pinned fuel band, and no band key's within-case trend may
//! out-climb its pinned slope.
//!
//! Each case draws one program from the full family roster (shape-biased,
//! combination, cross-universe, and reach regimes), replays it natively and
//! in the wasm guest, and judges two legs: every step's fuel against the
//! committed band for its key (kernel × outcome) at its denominated size
//! (the point leg), and every key's within-case bucket-median trend against
//! its pinned slope (the shape leg — a mechanism that tilts into a wide
//! band keeps its point residuals small, and only the trend sees it). Fuel
//! determinism makes a failure replay exactly: proptest shrinks to a
//! minimal out-of-band shape and writes a seed file next to this binary —
//! commit any seed that appears (repo hard rule); it is an
//! out-of-band-shape finding of record.
//!
//! Standing self-checks ride along: the meter's liveness (`ff_nop`), the
//! detection path's adequacy (a deliberately quadratic guest burner must
//! read ABOVE a linear band), the pin's provenance (the building toolchain
//! must match the pinning one), the pin's staleness (a fresh refit of the
//! deterministic corpus prefix must agree with the committed lines on
//! every covered band key), and the escalated regime itself (two fixed
//! reach-family programs — mid-depth and the depth cap, distinct seeds —
//! replay on every run, so the instrument's deep reach — including the
//! at-scale rejection arms — never rides on the sentry's rare escalation
//! draws alone).
//!
//! Case count: 48 by default (the calibration corpus is the big sweep; this
//! is the sentry); override with `PROPTEST_CASES`.

use std::collections::BTreeMap;

use proptest::prelude::*;

use fuzzfit_harness::bands::{
    band_for, judge_against, Verdict, BANDS, PINNED_RUSTC, REFIT_COVERAGE, REFIT_PREFIX_PROGRAMS,
    REFIT_TOLERANCE,
};
use fuzzfit_harness::curve::{local_slope_excess, SHAPE_EXEMPT, SLOPE_ALLOWANCE};
use fuzzfit_harness::drive::{for_each_deterministic_program, run_program, Sample};
use fuzzfit_harness::strategies::{any_program, build, Family, ESCALATION_REPLAYS};
use fuzzfit_harness::wasm::Guest;

/// Judge one program's samples on both enforcement legs, panicking with
/// the finding on any violation (the proptest sentry and the deterministic
/// escalation replay share this judgment verbatim).
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
            Verdict::InBand | Verdict::BelowFloor => {}
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
    // curvature, not mixture tilt. The fold rows are exempt (their
    // honest law trends along the width axis; the point leg owns
    // them).
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

/// The pin's provenance: the building toolchain must be the pinning one.
///
/// Fuel constants are a function of the guest codegen, which is a
/// function of the compiler, so judging against bands pinned under a
/// different toolchain compares incommensurable numbers. A toolchain
/// bump reads red here until the bands are re-pinned
/// (`just fuzzfit-calibrate`).
#[test]
fn building_toolchain_matches_the_pin() {
    assert_eq!(
        PINNED_RUSTC,
        env!("FUZZFIT_RUSTC_VERSION"),
        "toolchain drift: re-pin the bands with `just fuzzfit-calibrate`"
    );
}

/// The pin's staleness cross-check: a fresh fit of the deterministic
/// stream must agree with the committed constants on every band key the
/// pin-time prefix covered.
///
/// The bands are computable two ways — the pin and the refit — so the
/// two get compared, and disagreement beyond the measured tolerance
/// demands a deliberate re-pin with a movement annotation, never silent
/// drift.
///
/// Refits the first [`REFIT_PREFIX_PROGRAMS`] programs of the calibration
/// stream and walks the committed [`REFIT_COVERAGE`] list: every listed
/// key must still have a prefix fit (coverage decay fails by name), must
/// still match its pin's classification (a constant/linear flip is a
/// reach regression — the generators stopped placing that key where its
/// slope is measurable — and fails as a stale pin, never a skip), and
/// must agree with the pinned line within [`REFIT_TOLERANCE`].
#[test]
fn refit_of_the_deterministic_prefix_matches_the_pin() {
    let mut by_key: BTreeMap<(&'static str, bool), Vec<(u64, u64)>> = BTreeMap::new();
    for_each_deterministic_program(REFIT_PREFIX_PROGRAMS, |_, _, samples| {
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

/// The escalated regime rides in every suite run: a fixed reach-family
/// program must land every step in its band and every trend under its
/// slope, deterministically.
///
/// The sentry's random draws pick the escalation family about once in 137
/// cases, so a 48-case run usually never leaves the small-operand regime —
/// and an instrument whose deep reach is exercised only by rare draws has
/// no standing proof its at-scale bands (the seven single-operand rows,
/// the rejection arms, the deep-overlap scans) still bite. This replay is
/// that proof: one escalation program at fixed depth and seed, judged on
/// both legs like any sentry case. (The cross-universe rejection arms
/// need no fixed replay: the sentry's family roster draws the independent
/// regime nearly three times per default run.)
#[test]
fn the_escalated_regime_stays_in_the_pinned_bands() {
    let (depth, seed) = ESCALATION_REPLAYS[0];
    let program = build(&Family::Escalation { depth }, seed);
    let samples = run_program(&program)
        .unwrap_or_else(|m| panic!("malformed escalation program at {}", m.op));
    judge(&samples);
}

/// The escalated regime's far end, on an independent seed: a second fixed
/// reach-family program at the family's depth cap must land every step in
/// its band and every trend under its slope, deterministically.
///
/// The depth-1024 replay alone would leave the family's upper depth range
/// (1025..=1792) riding on sentry draws that arrive about once in five
/// runs, and would hang the whole deterministic reach proof on a single
/// (depth, seed) point. This replay pins the other end of the reach: the
/// deepest constructible spine, a different seed, the same judgment.
#[test]
fn the_escalation_depth_cap_stays_in_the_pinned_bands() {
    let (depth, seed) = ESCALATION_REPLAYS[1];
    let program = build(&Family::Escalation { depth }, seed);
    let samples = run_program(&program)
        .unwrap_or_else(|m| panic!("malformed escalation program at {}", m.op));
    judge(&samples);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Every public operation stays inside its pinned fuel band on
    /// shapes nobody chose, and no band key's within-case cost trend
    /// out-climbs its pinned slope.
    ///
    /// Cases draw random programs over the whole vocabulary, coupled
    /// and cross-universe operand regimes alike, success and rejection
    /// arms judged each against their own pinned law.
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
