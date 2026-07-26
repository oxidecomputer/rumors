//! The enforcement sentry: fuzzed programs must land every measured step
//! inside its pinned fuel band.
//!
//! Each case draws one program from the full family roster (shape-biased,
//! combination, and cross-universe regimes), replays it natively and in the
//! wasm guest, and judges every step's fuel against the committed band for
//! its kernel at its denominated size. Fuel determinism makes a failure
//! replay exactly: proptest shrinks to a minimal out-of-band shape and
//! writes a seed file next to this binary — commit any seed that appears
//! (repo hard rule); it is an out-of-band-shape finding of record.
//!
//! Case count: 48 by default (the calibration corpus is the big sweep; this
//! is the sentry); override with `PROPTEST_CASES`.

use proptest::prelude::*;

use fuzzfit_harness::bands::{band_for, judge_against, Verdict, BANDS};
use fuzzfit_harness::drive::run_program;
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
/// positive, exact amount. A zero here means fuel accounting is off (every
/// ceiling would pass vacuously); a large value means call overhead has
/// grown into the measurements.
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

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Every public operation stays inside its pinned fuel band on shapes
    /// nobody chose: random programs over the whole vocabulary, coupled and
    /// cross-universe operand regimes alike. Above-band is an asymptotic
    /// regression; below-band is a liveness failure; a kernel with no band
    /// is an unpriced operation (totality).
    #[test]
    fn fuel_stays_in_the_pinned_bands(program in any_program()) {
        let samples = run_program(&program)
            .unwrap_or_else(|m| panic!("malformed program at {}", m.op));
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
                     the pinned law predicts ~10^{:.3}±{:.3}",
                    s.kernel,
                    s.denom_bits,
                    s.fuel,
                    band.intercept + band.slope * (s.denom_bits as f64).log10(),
                    band.width,
                ),
                Verdict::Below => prop_assert!(
                    false,
                    "BELOW BAND (liveness): {} at {} bits consumed only {} fuel; \
                     the pinned law predicts ~10^{:.3}±{:.3}",
                    s.kernel,
                    s.denom_bits,
                    s.fuel,
                    band.intercept + band.slope * (s.denom_bits as f64).log10(),
                    band.width,
                ),
            }
        }
    }
}
