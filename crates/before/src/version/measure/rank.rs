//! Computes exact rank and compares ranks without materializing them.

use core::cmp::Ordering;

use crate::accumulator;
use crate::version::io::regions::{RegionReader, VersionRegionReader};
use crate::version::overlay::Side;
use crate::{Rank, Version};

use super::integral::Integrator;

impl Rank {
    /// Integrate a Version's height over the party interval.
    pub(crate) fn of_version(version: &Version) -> Rank {
        let max_depth = VersionRegionReader::max_depth(version);
        let (mut cursor, first) = VersionRegionReader::open(version);
        let mut integral = Integrator::new();
        integral.open_height(&first);

        // At the common depth, a leaf at depth `d` has integer width
        // `2^(max_depth - d)`. Integrating those scaled widths yields the
        // numerator of the exact dyadic rank.
        loop {
            integral.interval(max_depth - cursor.depth());
            if cursor.done() {
                break;
            }
            let (_, step) = cursor.step();
            Side::A.fold(&mut integral.live, &step);
            integral.boundary(accumulator::digit_len(step.magnitude()));
        }

        let (sign, numerator) = integral.finish(max_depth);
        debug_assert_ne!(sign, Ordering::Less, "heights are nonnegative");
        Rank::from_raw(numerator, max_depth)
    }
}

impl Version {
    /// Compare exact ranks without constructing either [`Rank`].
    ///
    /// The integral of the pointwise height difference is `rank(self) -
    /// rank(other)`, so its sign gives the order. A tie must settle that exact
    /// difference to zero and therefore has the same worst-case cost as
    /// materializing the ranks.
    pub(crate) fn rank_cmp(&self, other: &Version) -> Ordering {
        Integrator::pair_order(self, other)
    }
}
