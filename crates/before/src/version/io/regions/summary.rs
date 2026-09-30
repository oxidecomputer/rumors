//! State carried while summarizing leaves that an operation can skip.

use num_bigint::BigInt;
use suanpan::Accumulator;

/// Which extreme an [`Extremum`] tracks.
pub enum Direction {
    /// The maximum leaf height.
    Max,
    /// The minimum leaf height.
    Min,
}

/// A streaming extremum of the leaf heights a walk consumes.
///
/// The register stores `extremum - current height`. Crossing the tracked
/// extreme resets it to zero, making the current leaf the new extreme.
pub struct Extremum {
    /// The tracked extreme relative to the current height.
    pub register: Accumulator,
    /// Whether the first leaf has established the initial extreme.
    pub armed: bool,
    /// Whether the register tracks a minimum or maximum.
    pub direction: Direction,
}

/// The information needed to resume after skipping a range of leaves.
pub struct RegionSkip {
    /// Net height movement across the range.
    pub net: BigInt,
    /// Minimum height relative to the exit height; always nonpositive.
    pub min_from_exit: BigInt,
    /// Depth of the range's final leaf below the scanned subtree root.
    pub last_depth: u64,
    /// Encoded length of the final leaf's payload.
    pub last_code_len: u64,
}
