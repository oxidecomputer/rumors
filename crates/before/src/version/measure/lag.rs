//! Computes directed causal lag between two Versions.

use core::cmp::Ordering;

use crate::{Rank, Version};

use super::integral::Integrator;

impl Rank {
    /// Integrate the history present in `other` but absent from `self_`.
    pub(crate) fn lag_between(self_: &Version, other: &Version) -> Rank {
        // Integrate only the negative part of `self - other`: multiplying by
        // `-1` there measures the history present only in `other`.
        Integrator::pair_rank(self_, other, |sign| match sign {
            Ordering::Less => -1,
            _ => 0,
        })
    }
}
