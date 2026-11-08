use proptest::prelude::*;
use rand::SeedableRng;
use rand::rngs::StdRng;

use super::*;

proptest! {
    /// The log-space parameters reproduce the mean and CV they came from:
    /// `exp(μ + σ²/2)` is the mean and `sqrt(exp(σ²) − 1)` the CV.
    #[test]
    fn log_space_inverts(mean in 1.0f64..1e7, cv in 0.0f64..10.0) {
        let LogSpace { mu, sigma } = SizeDistribution { mean, cv, max: u64::MAX }.log_space();
        let back_mean = (mu + sigma * sigma / 2.0).exp();
        let back_cv = (sigma * sigma).exp_m1().sqrt();
        prop_assert!((back_mean - mean).abs() <= 1e-9 * mean);
        prop_assert!((back_cv - cv).abs() <= 1e-9 * cv.max(1.0));
    }

    /// Every draw of a valid distribution is an encoded size from 1 byte to
    /// the cap that a CBOR byte string can take.
    #[test]
    fn draws_stay_in_bounds(
        mean in 10.0f64..10_000.0,
        cv in 0.0f64..2.0,
        cap_factor in 4.0f64..20.0,
        seed in any::<u64>(),
    ) {
        // A cap at least four means above a mean of ten bytes or more keeps
        // more than nine draws in ten in bounds for every CV up to 2, so
        // every generated distribution is valid.
        let max = (mean * cap_factor).ceil() as u64;
        let distribution = SizeDistribution { mean, cv, max };
        prop_assert!(distribution.validate().is_ok());
        let mut sampler = distribution.sampler();
        let mut rng = StdRng::seed_from_u64(seed);
        for _ in 0..64 {
            let size = sampler.draw(&mut rng);
            prop_assert!((1..=max).contains(&size));
            prop_assert!(content_len(size).is_some());
        }
    }

    /// `content_len` and `byte_string_head` are inverse: a byte string of
    /// the returned length encodes in exactly the asked size.
    #[test]
    fn content_len_inverts_head(size in 1u64..1 << 34) {
        if let Some(len) = content_len(size) {
            prop_assert_eq!(len + byte_string_head(len), size);
        } else {
            prop_assert!(encodable_size_at_most(size) < size);
        }
    }
}

/// A CV of zero is a fixed size: every draw is the mean, and nothing is
/// redrawn.
#[test]
fn zero_cv_is_fixed() {
    let mut sampler = SizeDistribution {
        mean: 100.0,
        cv: 0.0,
        max: 1 << 16,
    }
    .sampler();
    let mut rng = StdRng::seed_from_u64(1);
    assert!((0..1000).all(|_| sampler.draw(&mut rng) == 100));
    assert_eq!(
        sampler.redrawn_above_cap + sampler.redrawn_below_one_byte,
        0
    );
}

/// Far below the cap, the sample mean of 100,000 draws at CV 1 is within 2% of
/// the requested mean, about seven standard errors.
#[test]
fn uncapped_sample_mean_matches() {
    let mut sampler = SizeDistribution {
        mean: 1000.0,
        cv: 1.0,
        max: 1 << 30,
    }
    .sampler();
    let mut rng = StdRng::seed_from_u64(2);
    let draws = 100_000;
    let total: u64 = (0..draws).map(|_| sampler.draw(&mut rng)).sum();
    let mean = total as f64 / draws as f64;
    assert!((mean - 1000.0).abs() < 20.0, "sample mean {mean}");
}

/// A cap that cuts into the tail redraws rather than clamps: no draw equals the
/// cap more often than its neighbours would, and the redraws are counted.
#[test]
fn cap_redraws_instead_of_clamping() {
    let distribution = SizeDistribution {
        mean: 1000.0,
        cv: 1.0,
        max: 2000,
    };
    let mut sampler = distribution.sampler();
    let mut rng = StdRng::seed_from_u64(3);
    let draws: Vec<u64> = (0..100_000).map(|_| sampler.draw(&mut rng)).collect();
    let at_cap = draws.iter().filter(|&&size| size == 2000).count();
    let near_cap = draws.iter().filter(|&&size| size == 1990).count();
    assert!(
        at_cap < 3 * near_cap.max(10),
        "{at_cap} draws at the cap against {near_cap} near it"
    );
    // Each accepted draw follows a geometric number of redraws, with mean
    // (1 − p) / p for acceptance chance p.
    let p = distribution.acceptance();
    let expected = (1.0 - p) / p * 100_000.0;
    let redrawn = sampler.redrawn_above_cap as f64;
    assert!(redrawn > expected * 0.8 && redrawn < expected * 1.25 + 50.0);
}

/// Distributions the sampler cannot meet as asked are refused with advice: a
/// mean above the cap, a mean below one byte, and a distribution so skewed that
/// most draws round to no bytes at all.
#[test]
fn unmeetable_distributions_are_refused() {
    assert!(
        SizeDistribution {
            mean: 100.0,
            cv: 0.0,
            max: 50
        }
        .validate()
        .is_err()
    );
    assert!(
        SizeDistribution {
            mean: 2.0,
            cv: 50.0,
            max: 1 << 16
        }
        .validate()
        .is_err()
    );
    assert!(
        SizeDistribution {
            mean: 0.0,
            cv: 0.0,
            max: 50
        }
        .validate()
        .is_err()
    );
}
