//! Computes causal distance between two Versions.

use core::cmp::Ordering;

use crate::{Rank, Version};

use super::integral::Integrator;

impl Rank {
    /// Integrate the absolute pointwise difference between two Versions.
    pub(crate) fn distance_between(a: &Version, b: &Version) -> Rank {
        // Integrating `sign(D) * D` gives `|D|`, where `D = a - b`.
        Integrator::pair_rank(a, b, |sign| match sign {
            Ordering::Greater => 1,
            Ordering::Equal => 0,
            Ordering::Less => -1,
        })
    }
}
