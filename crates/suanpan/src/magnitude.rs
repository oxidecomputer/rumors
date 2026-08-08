//! The width seam: unsigned operands readable at the width they are
//! stored at.
//!
//! [`Magnitude`] is the caller-facing dispatch between the accumulator's
//! amortized-O(1) small path and its per-limb wide path; the trait's own
//! docs carry the contract, including the self-agreement rule its
//! implementors owe.

use crate::UBig;

/// An unsigned operand readable at the width it is stored at.
///
/// The seam that lets a caller's own stored-magnitude type drive the
/// accumulator's `*_magnitude` entry points without conversion: the operand
/// reports whether it fits a machine word — the dispatch onto the
/// amortized-O(1) small path — and otherwise lends its full value to the
/// wide path. Signedness stays with the caller: route the operand's sign
/// to the `add_*` or `sub_*` entry point. Implementors necessarily own a
/// [`UBig`] to lend from [`as_wide`](Magnitude::as_wide); a type whose
/// values always fit a machine word has nothing to lend and should call
/// [`add_u64`](crate::Accumulator::add_u64)/[`sub_u64`](crate::Accumulator::sub_u64)
/// directly instead of implementing the trait. Implementations must agree
/// with themselves: when [`to_word`](Magnitude::to_word) returns
/// `Some(n)`, [`as_wide`](Magnitude::as_wide) must denote that same `n` —
/// a disagreeing implementation yields an unspecified held value (never a
/// memory-safety problem).
pub trait Magnitude {
    /// The value as a single machine word, or `None` past the `u64` range.
    ///
    /// Must be O(1): this is the dispatch read the small path's amortized
    /// cost accounting assumes is free. Returning `None` for a value that
    /// does fit a word is permitted — it forfeits the small path, never
    /// correctness.
    fn to_word(&self) -> Option<u64>;

    /// The full value, borrowed for the wide path.
    fn as_wide(&self) -> &UBig;
}

/// `to_word` returns `Some` exactly when the value fits a `u64`, so
/// word-sized `UBig`s always take the small path.
///
/// # Complexity
///
/// `to_word` and `as_wide` `O(1)`.
impl Magnitude for UBig {
    fn to_word(&self) -> Option<u64> {
        u64::try_from(self).ok()
    }

    fn as_wide(&self) -> &UBig {
        self
    }
}
