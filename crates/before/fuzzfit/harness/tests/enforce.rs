//! Checks generated programs against their pinned fuel bounds.
//!
//! Generated programs, the deterministic calibration prefix, and fixed deep
//! replays receive the same pointwise and within-program trend checks. The
//! bootstrap corpus separately exercises the small-operand pins. Measurements
//! below a main band's floor are judged only where a small band applies.
//!
//! The prefix must refit every pinned operation/outcome pair, preserving its
//! classification and staying within the line-divergence tolerance. Additional
//! tests check compiler provenance, fresh-instance fuel determinism, no-op
//! overhead, and detection of a deliberately quadratic guest operation.
//!
//! The property test runs proptest's default case count, raised with
//! `PROPTEST_CASES`; the calibration corpus is the big sweep.

use std::collections::{BTreeMap, BTreeSet};

use before::Clock;
use proptest::prelude::*;

use fuzzfit_harness::bands::{
    band_for, judge_against, judge_small, Band, Verdict, BANDS, PINNED_RUSTC,
    REFIT_PREFIX_PROGRAMS, REFIT_TOLERANCE, SMALL_BANDS,
};
use fuzzfit_harness::curve::{local_slope_excess, SHAPE_EXEMPT, SLOPE_ALLOWANCE};
use fuzzfit_harness::drive::{
    for_each_bootstrap_program, for_each_deterministic_program, run_program, Sample,
};
use fuzzfit_harness::fit::{fit, line_divergence, FIT_FLOOR_BITS};
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
            // Below the size-law floor, an applicable small band takes over.
            // The bootstrap test requires one for every operation that its
            // small-input corpus reaches below the main floor.
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

/// Every bootstrap sample is judged by a main or small band, and every small
/// band receives a bootstrap sample within its calibrated range.
#[test]
fn the_bootstrap_regime_stays_in_the_pinned_small_bands() {
    assert!(
        !SMALL_BANDS.is_empty(),
        "no bootstrap fuel bands are pinned"
    );
    let mut judged_small = BTreeSet::new();
    for_each_bootstrap_program(|_, samples| {
        judge(samples);
        for s in samples {
            let main = band_for(s.kernel, s.rejected).expect("judged main key");
            if s.denom_bits < main.min_denom {
                let (_, verdict) = judge_small(s.kernel, s.rejected, s.denom_bits, s.fuel)
                    .unwrap_or_else(|| {
                        panic!(
                            "{} at {} bits has no applicable bootstrap band",
                            s.kernel, s.denom_bits
                        )
                    });
                assert_eq!(verdict, Verdict::InBand, "{} bootstrap fuel", s.kernel);
                judged_small.insert((s.kernel, s.rejected));
            }
        }
    });
    for band in SMALL_BANDS {
        let kernel = band.kernel;
        assert!(
            band.constant && !band.rejected,
            "{kernel}: expected a constant success band"
        );
        assert!(
            band.max_denom < FIT_FLOOR_BITS,
            "{kernel}: small band reaches the fit floor"
        );
        assert!(
            judged_small.contains(&(kernel, band.rejected)),
            "{kernel}: the bootstrap corpus never exercised its small band"
        );
    }
}

/// An empty guest call consumes positive fuel below the overhead ceiling.
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

/// A version tick after a fixed exchange has identical fuel and result bytes
/// in fresh guest instances, including the input codecs and no-op overhead.
#[test]
fn version_tick_fuel_is_deterministic_across_fresh_guests() {
    let mut alice = Clock::seed();
    let mut bob = alice.fork();
    let mut carol = bob.fork();
    for _ in 0..100 {
        alice.tick();
        carol.tick();
        bob.recv(alice.send());
    }
    let (party, mut version) = bob.into_parts();
    let version_bytes = version.encode();
    let party_bytes = party.encode();
    party.tick(&mut version);
    let expected = version.encode();

    let run = || {
        let mut guest = Guest::new();
        let nop = guest.call("ff_nop", &[]);
        guest.stage_write(&version_bytes);
        let decode_version = guest.call("ff_version_decode", &[0]);
        guest.stage_write(&party_bytes);
        let decode_party = guest.call("ff_party_decode", &[1]);
        let tick = guest.call("ff_version_tick", &[0, 1]);
        let encode = guest.call("ff_version_encode", &[0]);
        let calls = [nop, decode_version, decode_party, tick, encode];
        for call in calls {
            assert_eq!(call.ret, 0, "guest call failed");
        }
        assert_eq!(guest.stage_read(), expected, "guest/native tick mismatch");
        calls
    };
    assert_eq!(run(), run(), "fresh guests measured different fuel");
}

/// A quadratic guest loop exceeds a linear band, and a stalled reading falls
/// below it, exercising execution, fuel metering, and judgment together.
///
/// The loop must also retain superlinear growth after compilation. This checks
/// the measurement path; it does not establish the generators' input coverage.
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
    let band = Band {
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

/// The deterministic prefix exercises and refits every main pinned band.
///
/// Every step in the first [`REFIT_PREFIX_PROGRAMS`] programs receives the
/// same pointwise and trend checks as a generated case. This makes the
/// corpus's operation-by-size coverage deterministic while generated cases
/// continue exploring new shapes.
///
/// The test also refits the corpus. Every key in [`BANDS`] must
/// remain measurable, retain its constant-or-linear classification, and
/// agree with its pinned line within [`REFIT_TOLERANCE`].
#[test]
fn the_deterministic_prefix_exercises_every_band_and_matches_the_pin() {
    assert!(!BANDS.is_empty(), "no operation fuel bands are pinned");
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
    for band in BANDS {
        let (kernel, rejected) = (band.kernel, band.rejected);
        let arm = if rejected { " [err]" } else { "" };
        let f = by_key
            .get(&(kernel, rejected))
            .and_then(|samples| fit(samples))
            .unwrap_or_else(|| {
                panic!(
                    "STALE PIN (coverage decay): {kernel}{arm} no longer fits in the \
                     deterministic prefix; re-pin with `just fuzzfit-calibrate` and \
                     annotate the movement"
                )
            });
        assert_eq!(
            f.constant, band.constant,
            "STALE PIN (classification flip): {kernel}{arm}'s prefix refit reads \
             constant={} against the pin's constant={} — a reach regression; re-pin \
             with `just fuzzfit-calibrate` and annotate the movement",
            f.constant, band.constant,
        );
        let d = line_divergence(&f, band);
        assert!(
            d <= REFIT_TOLERANCE,
            "STALE PIN: {kernel}{arm}'s prefix refit diverges from its pin by {d:.3} \
             (tolerance {REFIT_TOLERANCE}); re-pin with `just fuzzfit-calibrate` \
             and annotate the movement"
        );
    }
}

/// Every fixed escalation replay satisfies the applicable fuel bounds,
/// covering the mid-depth and depth-cap inputs with distinct seeds.
#[test]
fn the_escalated_regime_stays_in_the_pinned_bands() {
    for (depth, seed) in ESCALATION_REPLAYS {
        let program = build(&Family::Escalation { depth }, seed);
        let samples = run_program(&program)
            .unwrap_or_else(|m| panic!("malformed escalation program at {}", m.op));
        judge(&samples);
    }
}

proptest! {
    /// Generated operations satisfy the applicable pointwise fuel bounds
    /// and within-program trend limits on randomly chosen shapes.
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
