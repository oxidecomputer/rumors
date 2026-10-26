//! Tests for the census floor's arithmetic.

use proptest::prelude::*;

use super::{floor, Mass, FALSE_FAILURE_BITS};

proptest! {
    /// The exact-integer floor never exceeds the real Chernoff bound
    /// `μ − sqrt(2 μ ln 2^30)`, and falls short of it by less than three.
    ///
    /// The shortfall is under one for each of its two roundings, and under
    /// one for its bound on `ln 2`, which inflates the deviation by a factor
    /// below `1 + 10⁻⁶`.
    ///
    /// The real bound is computed here in floating point, independently of
    /// the integer search [`floor`] runs, over sample counts and masses far
    /// past any census's.
    #[test]
    fn floor_tracks_the_chernoff_bound(
        samples in 1usize..10_000_000,
        (numerator, denominator) in (1u64..=1_000_000)
            .prop_flat_map(|denominator| (1..=denominator, Just(denominator))),
    ) {
        let got = floor(samples, Mass::new(numerator, denominator)) as f64;
        let mean = samples as f64 * numerator as f64 / denominator as f64;
        let deviation = (2.0 * mean * FALSE_FAILURE_BITS as f64 * core::f64::consts::LN_2).sqrt();
        let bound = (mean - deviation).max(0.0);
        prop_assert!(got <= bound + 1e-6, "floor {} exceeds the bound {}", got, bound);
        prop_assert!(got > bound - 3.0, "floor {} falls three or more below the bound {}", got, bound);
    }
}
