//! Computes directed causal lag between two Versions.

use crate::{Rank, Version};

use super::integral::{Integrator, NegativePart};

impl Rank {
    /// Integrate the history present in `other` but absent from `self_`.
    pub(crate) fn lag_between(self_: &Version, other: &Version) -> Rank {
        // The negative part of `self - other` is the history present only
        // in `other`.
        Integrator::pair_rank::<NegativePart>(self_, other)
    }
}
