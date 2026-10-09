//! Shared traversal for measures of two Versions.

use core::cmp::Ordering;

use num_bigint::BigUint;

use crate::accumulator;
use crate::version::io::regions::{RegionReader, VersionRegionReader};
use crate::version::overlay::{advance_diff, OpenedPair, Side};
use crate::{Rank, Version};

use super::Integrator;

/// Drive the integral from two Versions while retaining their running height difference.
impl Integrator {
    /// Integrate a nonnegative function of the pointwise height difference.
    ///
    /// `orientation` chooses which signed part of `a - b` contributes. Its
    /// result must be monotone over `Less`, `Equal`, `Greater` and lie in
    /// `-1..=1`; this ensures that crossing zero adds a nonnegative correction.
    pub fn pair_rank(a: &Version, b: &Version, orientation: impl Fn(Ordering) -> i8) -> Rank {
        let (sign, total, scale) = Self::pair(a, b, orientation);
        debug_assert_ne!(sign, Ordering::Less, "the measure is nonnegative");
        Rank::from_raw(total, scale)
    }

    /// Compare ranks by integrating the signed pointwise difference.
    pub fn pair_order(a: &Version, b: &Version) -> Ordering {
        Self::pair(a, b, |_| 1).0
    }

    /// Integrate `orientation(sign(a - b)) * (a - b)` in one overlay walk.
    fn pair(
        a: &Version,
        b: &Version,
        orientation: impl Fn(Ordering) -> i8,
    ) -> (Ordering, BigUint, u64) {
        // Every overlap region lies inside a leaf of each operand. Scaling both
        // streams to the deeper maximum depth therefore gives every region an
        // integer power-of-two width and one common denominator.
        let scale = VersionRegionReader::max_depth(a).max(VersionRegionReader::max_depth(b));
        let OpenedPair {
            a: mut cursor_a,
            b: mut cursor_b,
            mut diff,
            ..
        } = OpenedPair::open(a, b);
        let mut current_orientation = orientation(diff.cmp_zero());
        let mut integral = Integrator::new();

        if current_orientation != 0 {
            integral.open_difference(&diff, current_orientation);
        }

        loop {
            integral.interval(scale - cursor_a.depth().max(cursor_b.depth()));
            if cursor_a.done() && cursor_b.done() {
                break;
            }

            let (step_a, step_b) = advance_diff(&mut cursor_a, &mut cursor_b, &mut diff);
            let diff_order = diff.cmp_zero();
            let new_orientation = orientation(diff_order);

            // While the selected part is active, fold each input delta into the
            // integrand. Reversing orientation reverses which operand adds and
            // which subtracts.
            if current_orientation != 0 {
                for (side, step) in [(Side::A, &step_a), (Side::B, &step_b)] {
                    if let Some(step) = step {
                        let toward = if current_orientation > 0 {
                            side
                        } else {
                            side.other()
                        };
                        toward.fold(&mut integral.live, step);
                    }
                }
            }

            if new_orientation != current_orientation {
                // The delta folds above use the old orientation. This correction
                // applies the orientation change to the updated signed difference.
                // A sign crossing bounds its magnitude by the deltas just consumed.
                integral.jump(new_orientation - current_orientation, diff_order, &mut diff);
                current_orientation = new_orientation;
            }

            // At least one stream advances at every boundary. Its widest decoded
            // delta pays for the recent arithmetic retained until the next region.
            let funded_digits = step_a
                .iter()
                .chain(step_b.iter())
                .map(|step| accumulator::digit_len(step.magnitude()))
                .max()
                .expect("the overlay advances at least one Version per boundary");
            integral.boundary(funded_digits);
        }

        let (sign, total) = integral.finish(scale);
        (sign, total, scale)
    }
}
