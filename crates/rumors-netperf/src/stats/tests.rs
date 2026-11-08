use proptest::prelude::*;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

use super::*;

/// The binomial distribution function matches exact counts: for n = 5,
/// `P(X ≤ k)` is the sum of `C(5, i)` for i ≤ k over 32.
#[test]
fn binomial_matches_exact_counts() {
    let exact = [1.0, 6.0, 16.0, 26.0, 31.0, 32.0].map(|c| c / 32.0);
    for (k, expected) in exact.iter().enumerate() {
        assert!((binomial_half_cdf(5, k) - expected).abs() < 1e-12);
    }
}

proptest! {
    /// Every summary orders its interval around its median, and from five
    /// samples on reaches the requested coverage.
    #[test]
    fn interval_brackets_median(samples in proptest::collection::vec(-1e6f64..1e6, 1..200)) {
        let summary = summarize(&samples, CONFIDENCE).unwrap();
        prop_assert!(summary.low <= summary.median && summary.median <= summary.high);
        if samples.len() >= 5 {
            prop_assert!(summary.coverage >= CONFIDENCE);
        }
    }
}

/// The declared coverage is the realized one.
///
/// Over 20,000 draws of seven exponential samples, whose population median is
/// ln 2, the interval covers ln 2 as often as declared, within 1.5 percentage
/// points (about seven standard errors).
#[test]
fn declared_coverage_is_realized() {
    let mut rng = StdRng::seed_from_u64(5);
    let median = std::f64::consts::LN_2;
    let trials = 20_000;
    let mut covered = 0;
    let mut declared = 0.0;
    for _ in 0..trials {
        let samples: Vec<f64> = (0..7).map(|_| -(1.0 - rng.random::<f64>()).ln()).collect();
        let summary = summarize(&samples, CONFIDENCE).unwrap();
        declared = summary.coverage;
        covered += usize::from(summary.low <= median && median <= summary.high);
    }
    let realized = covered as f64 / trials as f64;
    assert!(
        (realized - declared).abs() < 0.015,
        "realized {realized}, declared {declared}"
    );
}
