//! The shape leg's own tripwires: the trend judgment must fire on a
//! quadratic reading, stay quiet on a noisy linear one, and abstain
//! without evidence — proven on synthetic samples before any fuzzing
//! counts on it.

use super::{local_slope_excess, MIN_PER_BUCKET, SLOPE_ALLOWANCE};
use crate::bands::Band;

/// A pinned-linear synthetic band: fuel ≈ 100 · d over 128..100k bits.
fn linear_band() -> Band {
    Band {
        kernel: "synthetic_linear",
        rejected: false,
        slope: 1.0,
        intercept: 2.0,
        width_above: 0.3,
        width_below: 0.3,
        min_denom: 128,
        max_denom: 100_000,
        samples: 1000,
        constant: false,
    }
}

/// Samples along `fuel = f(d)` at `MIN_PER_BUCKET` points per half-decade
/// bucket across 128..~100k bits.
fn sampled(f: impl Fn(u64) -> u64) -> Vec<(u64, u64)> {
    let mut out = Vec::new();
    let mut d = 128u64;
    while d < 100_000 {
        for i in 0..MIN_PER_BUCKET as u64 {
            let dd = d + i * (d / 16).max(1);
            out.push((dd, f(dd)));
        }
        // Two buckets per decade: step by √10.
        d = (d as f64 * 3.163) as u64;
    }
    out
}

/// A quadratic mechanism must trip the trend: its within-case local
/// slope reads ~2 against a pinned slope of 1, far beyond the allowance.
///
/// This is the leg's reason to exist — a quadratic that tilts into a
/// wide band keeps every point residual small, and only the trend sees
/// it.
#[test]
fn quadratic_readings_exceed_the_allowance() {
    let band = linear_band();
    let excess = local_slope_excess(&band, &sampled(|d| d * d / 100))
        .expect("full-span synthetic corpus has evidence");
    assert!(
        excess > SLOPE_ALLOWANCE,
        "quadratic local slope excess {excess:.3} did not exceed the allowance"
    );
}

/// An honest linear mechanism, even under 2x multiplicative noise, stays
/// inside the allowance: the leg flags trends, not spread.
#[test]
fn noisy_linear_readings_stay_inside_the_allowance() {
    let band = linear_band();
    // Deterministic 1x..2x jitter, uncorrelated with size.
    let excess = local_slope_excess(&band, &sampled(|d| d * 100 * (1 + d % 2)))
        .expect("full-span synthetic corpus has evidence");
    assert!(
        excess.abs() < SLOPE_ALLOWANCE,
        "linear local slope excess {excess:.3} left the allowance"
    );
}

/// Without evidence — thin buckets, few buckets, or a short span — the
/// leg abstains rather than judging a dart throw.
#[test]
fn under_evidenced_cases_abstain() {
    let band = linear_band();
    // One sample per bucket: thin.
    let thin: Vec<(u64, u64)> = (0..8).map(|i| (128u64 << i, (128u64 << i) * 100)).collect();
    assert!(local_slope_excess(&band, &thin).is_none());
    // Full buckets, but everything below the judgment floor.
    let floored = sampled(|d| d * 100)
        .into_iter()
        .map(|(d, f)| (d.min(127), f))
        .collect::<Vec<_>>();
    assert!(local_slope_excess(&band, &floored).is_none());
    // Three populated buckets whose endpoint medians still sit under a
    // decade apart (each cluster hugs the bucket edge nearest the middle).
    let mut short = Vec::new();
    for d in [300u64, 320, 1010] {
        for i in 0..MIN_PER_BUCKET as u64 {
            short.push((d + i, (d + i) * 100));
        }
    }
    assert!(local_slope_excess(&band, &short).is_none());
}
