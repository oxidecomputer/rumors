//! The width of the counts the library accumulates.
//!
//! A *counter* accumulates events, bytes, or items as they happen: frames
//! sent, bytes read from a transport, messages gained, leaves a backend
//! yields. Library code holds every counter as a `u64` (an `AtomicU64` when
//! shared) on every target, so none wraps at four billion on a target whose
//! `usize` is 32 bits.
//!
//! A length or index bounded by memory stays a `usize`: a `Vec`'s length, a
//! position in a buffer, the number of items a queue holds at once. Such a
//! value cannot outgrow the address space it measures.
//!
//! [`widen`] carries a `usize` into a counter. A counter read back where a
//! `usize` is needed goes through `usize::try_from`, with its failure
//! handled at that site.

// `widen` relies on this: every `usize` value fits a `u64`. A target with a
// wider `usize` fails to compile here rather than truncate a count.
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
