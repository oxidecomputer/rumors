//! The staleness comparator's own tripwires: [`line_divergence`] must
//! read zero against an agreeing pin and read the exact perturbation
//! against a hand-perturbed one — proven before the cross-check's
//! tolerance means anything.

use super::{fit, line_divergence, Fit};
use crate::bands::Band;

/// A band transcribing `f` exactly (the agreeing pin).
fn band_of(f: &Fit) -> Band {
    Band {
        kernel: "synthetic",
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

/// A clean linear corpus to fit: fuel = 100 · d over 128..131072 bits.
fn linear_fit() -> Fit {
    let samples: Vec<(u64, u64)> = (7..=17)
        .flat_map(|k| {
            let d = 1u64 << k;
            [(d, d * 100), (d + d / 2, (d + d / 2) * 100)]
        })
        .collect();
    fit(&samples).expect("enough samples to fit")
}

/// Against a pin that transcribes the fit, the divergence is zero: the
/// comparator cannot false-flag an honest pin.
#[test]
fn agreeing_pin_reads_zero() {
    let f = linear_fit();
    assert!(line_divergence(&f, &band_of(&f)) < 1e-12);
}

/// A perturbed intercept reads back as exactly the perturbation, and a
/// perturbed slope as its full effect at the range's far endpoint: the
/// comparator is alive on both parameters, so a stale pin cannot slip
/// under a tolerance smaller than its drift.
#[test]
fn perturbed_pin_reads_its_perturbation() {
    let f = linear_fit();
    let mut shifted = band_of(&f);
    shifted.intercept += 0.4;
    let d = line_divergence(&f, &shifted);
    assert!((d - 0.4).abs() < 1e-12, "intercept shift read {d}");

    let mut tilted = band_of(&f);
    tilted.slope += 0.1;
    let d = line_divergence(&f, &tilted);
    // The tilt pivots at d = 1 bit, so the endpoint far from the pivot
    // carries the larger disagreement.
    let expected = 0.1 * (f.max_denom as f64).log10();
    assert!(
        (d - expected).abs() < 1e-12,
        "slope tilt read {d}, expected {expected}"
    );
}
