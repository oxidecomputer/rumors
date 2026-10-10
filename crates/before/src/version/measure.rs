//! Exact numeric measures of causal history.
//!
//! A [`Version`](crate::Version) is a nonnegative step function over the party
//! interval. Its rank is the integral of that function; distance and lag
//! integrate differences between two [`Version`](crate::Version)s. `min_ticks`
//! instead finds the shortest event history consistent with one
//! [`Version`](crate::Version).
//!
//! Each operation has its own module. [`integral`] contains the shared
//! arithmetic that prevents narrow encoded changes from repeatedly provoking
//! work over a wide running height. The operation modules construct the final
//! domain type—[`Rank`](crate::Rank) or [`Count`](crate::Count)—from version
//! readers, so the public methods need no representation-level helper methods.
//!
//! Rank, distance, and lag take `O(M(n))` time in `n` input bits, where `M(n)`
//! is the cost of multiplying `n`-bit integers. An exact result can itself
//! contain such a product. `min_ticks` is linear in its input. Every traversal
//! retains only its cursor paths and the arithmetic state needed for its
//! result; none constructs a recursive tree or an intermediate Version.

/// Extra bits of working width a height change may hold beyond the newest
/// delta's bit length before it is frozen.
///
/// The cushion avoids moving arithmetic state for small width fluctuations;
/// being fixed, it does not affect the input-proportional bound.
pub(super) const HEIGHT_FREEZE_ALLOWANCE_BITS: u64 = 256;

mod distance;
pub mod integral;
mod lag;
mod min_ticks;
mod rank;

#[cfg(test)]
mod tests;
