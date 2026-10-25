//! Shared traversal for measures of two Versions.

use core::cmp::Ordering;

use num_bigint::BigUint;
use suanpan::Accumulator;

use crate::version::io::regions::{RegionReader, VersionRegionReader};
use crate::version::overlay::{advance_diff, OpenedPair, Side};
use crate::{Rank, Version};

use super::Integrator;

/// A function of the pointwise difference `D = a - b` that a pair measure
/// integrates: `sigma(sign(D)) * D`, for a coefficient `sigma` in `{-1, 0, 1}`.
///
/// Each integrand is a type, not a value, so that the traversal is compiled
/// once per integrand with its coefficient known: the rank comparison's
/// constant coefficient folds away, and so does every test of it.
///
/// Every implementor's `sigma` must be monotone over `Less`, `Equal`,
/// `Greater`, so that when `D` changes sign the correction
/// `(sigma' - sigma) * D'` that [`Integrator::jump`] adds is never negative.
/// The trait is private to the measures, and a test checks every sign change
/// of each implementor it lists; a new implementor joins that list.
pub trait Integrand {
    /// Return `sigma`, the coefficient this integrand applies to a
    /// difference of sign `sign`.
    fn sigma(sign: Ordering) -> i8;
}

/// `|D|`, which distance integrates.
pub struct Absolute;

impl Integrand for Absolute {
    fn sigma(sign: Ordering) -> i8 {
        match sign {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }
}

/// The negative part of `D`, `max(-D, 0)`, which lag integrates.
pub struct NegativePart;

impl Integrand for NegativePart {
    fn sigma(sign: Ordering) -> i8 {
        if sign == Ordering::Less {
            -1
        } else {
            0
        }
    }
}

/// `D` itself, whose integral's sign compares ranks.
pub struct Signed;

impl Integrand for Signed {
    fn sigma(_: Ordering) -> i8 {
        1
    }
}

/// Return the side from which a delta of `side` folds into an integrand of
/// coefficient `sigma`: `side` itself when `sigma` is positive, the other
/// side when it is negative, and none when it is zero.
fn oriented(side: Side, sigma: i8) -> Option<Side> {
    match sigma.cmp(&0) {
        Ordering::Greater => Some(side),
        Ordering::Less => Some(side.other()),
        Ordering::Equal => None,
    }
}

/// Drive the integral from two Versions while retaining their running height difference.
impl Integrator {
    /// Integrate a nonnegative function of the pointwise height difference.
    pub fn pair_rank<I: Integrand>(a: &Version, b: &Version) -> Rank {
        let (total, scale) = Self::pair::<I>(a, b);
        let total = BigUint::try_from(&total)
            .expect("a nonnegative integrand keeps every integrated difference nonnegative");
        Rank::from_raw(total, scale)
    }

    /// Compare ranks by integrating the signed pointwise difference.
    pub fn pair_order(a: &Version, b: &Version) -> Ordering {
        Self::pair::<Signed>(a, b).0.sign()
    }

    /// Integrate `I` over `a - b` in one overlay traversal.
    fn pair<I: Integrand>(a: &Version, b: &Version) -> (Accumulator, u64) {
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
        let mut sigma = I::sigma(diff.sign());
        let mut integral = Integrator::new();

        if let Some(toward) = oriented(Side::A, sigma) {
            integral.open_difference(&diff, toward);
        }

        loop {
            integral.interval(scale - cursor_a.depth().max(cursor_b.depth()));
            if cursor_a.done() && cursor_b.done() {
                break;
            }

            let (step_a, step_b) = advance_diff(&mut cursor_a, &mut cursor_b, &mut diff);
            let diff_order = diff.sign();
            let new_sigma = I::sigma(diff_order);

            // While the selected part is active, fold each input delta into the
            // integrand. A negative coefficient reverses which operand adds and
            // which subtracts.
            for (side, step) in [(Side::A, &step_a), (Side::B, &step_b)] {
                if let (Some(step), Some(toward)) = (step, oriented(side, sigma)) {
                    integral.fold_change(toward, step);
                }
            }

            if new_sigma != sigma {
                // The delta folds above use the old coefficient. This correction
                // applies the change of coefficient to the updated difference.
                // A sign crossing bounds its magnitude by the deltas just consumed.
                integral.jump(new_sigma - sigma, diff_order, &diff);
                sigma = new_sigma;
            }

            // At least one stream advances at every boundary. Its widest decoded
            // delta pays for the recent arithmetic retained until the next region.
            let funded_bits = step_a
                .iter()
                .chain(step_b.iter())
                .map(|step| step.magnitude().bits())
                .max()
                .expect("the overlay advances at least one Version per boundary");
            integral.boundary(funded_bits);
        }

        (integral.finish(scale), scale)
    }
}

#[cfg(test)]
mod tests;
