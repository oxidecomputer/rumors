//! Interval Tree Clocks.
//!
//! [`party::Party`] is a nonzero share of the id space (ordered by descent: an
//! ancestor is *less than* its forked descendants). [`version::Version`] is an
//! event tree / message, also serving as the paper's anonymous stamp.
//! [`clock::Clock`] is a `Party` paired with a `Version` — purely a convenience;
//! `into_parts`/`from_parts` move between them, and the whole `Clock` API can be
//! reconstructed by hand from the `Party` and `Version` APIs.
//!
//! Linearity: `Party`/`Clock` are not `Clone`; `Version` clones freely.
//!
//! All mutation goes through a batch ([`version::Batch`], [`clock::Batch`]) that
//! unpacks the version to a fast fixed-width working form lazily, applies a run
//! of operations, and repacks once on drop. Value-level methods are single-op
//! batches. Comparison reads the current state in place — no repack — so batches
//! are compared directly rather than peeked. All traversals are iterative.
//!
//! This crate implements Interval Tree Clocks (Almeida, Baquero & Fonte, 2008)
//! with a packed [`bitvec`] storage form and a transient fixed-width working form
//! for mutation. See `IMPLEMENTATION_PLAN.md` for the full, frozen design.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod clock;
mod codec;
mod idbits;
pub mod party;
pub mod version;

#[cfg(test)]
mod oracle;
#[cfg(test)]
mod test_support;

/// Test-only traversal step counter. Every node-header read calls [`step!`]; the
/// complexity tests reset it, run one operation, and assert the count is `O(n + m)`
/// — proving no traversal re-scans (which would be `O(n²)` on a deep spine). A
/// deterministic stand-in for wall-clock timing, which would be flaky.
#[cfg(test)]
pub(crate) mod metrics {
    use std::cell::Cell;

    thread_local! {
        static STEPS: Cell<u64> = const { Cell::new(0) };
    }

    /// Reset the step counter to zero.
    pub(crate) fn reset() {
        STEPS.with(|c| c.set(0));
    }

    /// The number of traversal steps recorded since the last [`reset`].
    pub(crate) fn taken() -> u64 {
        STEPS.with(|c| c.get())
    }

    /// Record one traversal step (one node-header read).
    pub(crate) fn bump() {
        STEPS.with(|c| c.set(c.get() + 1));
    }
}

/// Record one traversal step. Expands to a counter bump under `cfg(test)` (see
/// [`metrics`]) and to nothing otherwise, so production traversals pay zero cost.
#[cfg(test)]
macro_rules! step {
    () => {
        $crate::metrics::bump()
    };
}
#[cfg(not(test))]
macro_rules! step {
    () => {};
}
pub(crate) use step;

pub use clock::Clock;
pub use party::Party;
pub use version::Version;

/// Two parties were not disjoint. (`join` instead hands the clock back.)
#[derive(Debug)]
pub struct OverlapError;

impl core::fmt::Display for OverlapError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("parties are not disjoint")
    }
}

impl std::error::Error for OverlapError {}

/// Why a byte string failed to decode into a `Party`, `Version`, or `Clock`.
#[derive(Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// The bit stream ended mid-tree (or mid-integer).
    Truncated,
    /// Non-padding bits remained after a complete tree, or the padding was nonzero.
    TrailingBits,
    /// The structure is well-formed but not in canonical normal form.
    NotCanonical,
}

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let s = match self {
            DecodeError::Truncated => "input ended mid-tree",
            DecodeError::TrailingBits => "trailing or nonzero padding bits after a complete tree",
            DecodeError::NotCanonical => "input is well-formed but not in canonical normal form",
        };
        f.write_str(s)
    }
}

impl std::error::Error for DecodeError {}
