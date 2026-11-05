//! The crate's lossless conversion from `usize` to `u64`, and the width of
//! its counters.
//!
//! A *counter* accumulates a count as events happen, with no bound set by
//! memory. Library code holds every counter as a `u64` (an `AtomicU64` when
//! shared) on every target, so that no counter wraps at four billion on a
//! target whose `usize` is 32 bits.
//!
//! A length or index bounded by memory stays a `usize`: a `Vec`'s length, a
//! position in a buffer, the number of items a queue holds at once. Such a
//! value cannot outgrow the address space it measures.
//!
//! [`widen`] is the crate's conversion from a `usize` to a `u64`, used
//! wherever a `usize` becomes a `u64`, whether it enters a counter or a
//! wider field. It cannot lose information, because the crate compiles only
//! for targets whose `usize` is at most 64 bits wide: a compile-time
//! assertion in this module rejects any other target. A counter read back
//! where a `usize` is needed goes through `usize::try_from`, with its
//! failure handled at that site.

// `widen` relies on this: every `usize` value fits a `u64`. A target with a
// wider `usize` fails to compile here rather than truncate a value.
const _: () = assert!(usize::BITS <= u64::BITS);

/// Widen a `usize` to a `u64` counter, losslessly.
///
/// The conversion cannot truncate: the crate does not compile for a target
/// whose `usize` is wider than 64 bits.
#[inline]
pub(crate) const fn widen(len: usize) -> u64 {
    // Lossless: the assertion above guarantees a `u64` holds every `usize`.
    len as u64
}
