//! Interval Tree Clocks.
//!
//! This crate implements Interval Tree Clocks (Almeida, Baquero & Fonte, 2008)
//! with an alternate packed bit-vector representation.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod codec;
mod error;
mod idbits;

pub mod clock;
pub mod party;
pub mod version;

/// Reference oracle: the paper's recursive trees; ground truth for the
/// differential tests. Public under the `oracle` feature so the benchmark suite
/// can time it against the optimized implementation.
#[cfg(any(test, feature = "oracle"))]
pub mod oracle;

pub use clock::Clock;
pub use error::{DecodeError, OverlapError, ParseError};
pub use party::Party;
pub use version::Version;

#[cfg(feature = "serde")]
mod serde_impls;

#[cfg(test)]
mod testing;

/// Record one traversal step. Expands to a counter bump under `cfg(test)` (see the
/// test-only [`metrics`](crate::testing::metrics) module) and to nothing otherwise.
///
/// This is used to deterministically test asymptotic traversal cost to prevent
/// accidental quadraticity.
#[cfg(test)]
macro_rules! step {
    () => {
        $crate::testing::metrics::bump()
    };
}
#[cfg(not(test))]
macro_rules! step {
    () => {};
}
pub(crate) use step;
