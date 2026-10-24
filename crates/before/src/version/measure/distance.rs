//! Computes causal distance between two Versions.

use crate::{Rank, Version};

use super::integral::{Absolute, Integrator};

impl Rank {
    /// Integrate the absolute pointwise difference between two Versions.
    pub(crate) fn distance_between(a: &Version, b: &Version) -> Rank {
        Integrator::pair_rank::<Absolute>(a, b)
    }
}
