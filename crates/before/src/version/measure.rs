//! Exact numeric measures of causal history.
//!
//! A Version is a nonnegative step function over the party interval. Its rank
//! is the integral of that function; distance and lag integrate differences
//! between two Versions. `min_ticks` instead finds the shortest event history
//! consistent with one Version.
//!
//! Each operation has its own module. [`integral`] contains the shared
//! arithmetic that prevents narrow encoded changes from repeatedly provoking
//! work over a wide running height. The operation modules construct the final
//! domain type—[`Rank`](crate::Rank) or [`Ticks`](crate::Ticks)—from Version
//! readers, so the public methods need no representation-level helper methods.
//!
//! Rank, distance, and lag take `O(M(n))` time in `n` input bits, where `M(n)`
//! is the cost of multiplying `n`-bit integers. An exact result can itself
//! contain such a product. `min_ticks` is linear in its input. Every traversal
//! retains only its cursor paths and the arithmetic state needed for its
//! result; none constructs a recursive tree or an intermediate Version.

/// Extra base-2^32 digits tolerated before an old height change is frozen.
///
/// Eight digits give a 256-bit cushion. This avoids moving arithmetic state for
/// small width fluctuations; the fixed cushion does not affect the
/// input-proportional bound.
const HEIGHT_FREEZE_ALLOWANCE_DIGITS: usize = 8;

mod distance;
pub mod integral;
mod lag;
mod min_ticks;
mod rank;

#[cfg(test)]
mod tests;
