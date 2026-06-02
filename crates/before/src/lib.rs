//! Interval Tree Clocks.
//!
//! This crate implements Interval Tree Clocks (Almeida, Baquero & Fonte, 2008)
//! with an alternate packed bit-vector representation.
//!
//! ```
//! use before::Clock;
//!
//! let mut alice = Clock::seed();
//! let mut bob = alice.fork(); // hand Bob a disjoint clock
//!
//! alice.tick();
//! let msg = alice.send().clone(); // Alice sends her current version
//! bob.recv(&msg); // Bob incorporates it, then ticks
//!
//! assert!(*bob.version() > msg); // Bob's clock now dominates the message
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod clock;
mod codec;
mod idbits;
mod party;
mod version;

// The whole public API:
pub use clock::Clock;
pub mod error;
pub use party::Party;
pub use version::Version;
pub mod batch {
    //! [`batch::Clock`](Clock) and [`batch::Version`](Version) amortize
    //! operation costs to improve performance on [`Clock`](crate::Clock)s and
    //! [`Version`](crate::Version)s.
    //!
    //! ```
    //! use before::{batch, Clock};
    //! let mut clock = Clock::seed();
    //! {
    //!     let mut batch: batch::Clock = clock.batch();
    //!     batch.tick().tick(); // amortized; repacked when the batch drops
    //! }
    //! assert_eq!(clock.version().to_string(), "2");
    //! ```
    pub use crate::{clock::Batch as Clock, version::Batch as Version};
}

/// Stack-growth guard shared by the recursive traversals.
mod recurse;

/// Reference oracle: the paper's recursive trees; ground truth for the
/// differential tests. Public under the `oracle` feature so the benchmark suite
/// can time it against the optimized implementation.
#[cfg(any(test, feature = "oracle"))]
pub mod oracle;

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
