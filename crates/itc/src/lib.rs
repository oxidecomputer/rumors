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
//!
//! # Example
//!
//! ```
//! use itc::Clock;
//!
//! // One process forks another; both carry the shared history so far.
//! let mut a = Clock::seed();
//! let mut b = a.fork();
//!
//! // `a` records an event: now `a` strictly dominates `b`.
//! a.tick();
//! assert!(b.happens_before(&a));
//!
//! // `b` records its own event: now the two are concurrent.
//! b.tick();
//! assert!(a.concurrent_with(&b));
//!
//! // Reconcile them: after `sync`, both agree on history again.
//! a.sync(&mut b).unwrap();
//! assert!(!a.concurrent_with(&b) && !a.happens_before(&b) && !b.happens_before(&a));
//! ```
//!
//! Values print and parse in the paper's notation — handy for tests and literals:
//!
//! ```
//! use itc::{Clock, Party, Version};
//!
//! assert_eq!(Clock::seed().to_string(), "(1, 0)");
//!
//! let id: Party = "(1, (0, 1))".parse().unwrap();
//! assert_eq!(id.to_string(), "(1, (0, 1))");
//!
//! // The same id as an embedded literal, checked at parse time.
//! let lit = Party::try_from((1u8, (0u8, 1u8))).unwrap();
//! assert_eq!(lit, id);
//!
//! let ev = Version::try_from((1u64, 0u64, 1u64)).unwrap();
//! assert_eq!(ev.to_string(), "(1, 0, 1)");
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod clock;
mod codec;
mod idbits;
pub mod party;
pub mod version;

#[cfg(feature = "serde")]
mod serde_impls;

#[cfg(test)]
mod metrics;
/// Reference oracle — the paper's trees as plain recursive enums; ground truth for the
/// differential tests. Public under the `oracle` feature so the benchmark suite can time
/// it against the optimized implementation; not part of the production surface.
#[cfg(any(test, feature = "oracle"))]
pub mod oracle;
#[cfg(test)]
mod test_support;

/// Record one traversal step. Expands to a counter bump under `cfg(test)` (see the
/// test-only [`metrics`](crate::metrics) module) and to nothing otherwise, so
/// production traversals pay zero cost.
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

/// Why a string (or a literal tuple) failed to parse into a `Party`, `Version`, or
/// `Clock`. Parsing uses the paper's notation and, like [`DecodeError`], strictly
/// rejects non-canonical input.
#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    /// The input is not well-formed paper notation (bad token, unbalanced parens,
    /// non-`0`/`1` id leaf, malformed integer, or trailing input).
    Syntax,
    /// The input is well-formed but does not denote a value in canonical normal form
    /// (e.g. a collapsible `(1, 1)` id or `(n, m, m)` event, or an event node with no
    /// zero-base child).
    NotCanonical,
    /// The input denotes the anonymous identity `0` (an id owning no region). A
    /// standalone [`Party`]/[`Clock`] must be a nonzero share, so this is rejected —
    /// though `0` is valid as a sub-tree inside a larger id (e.g. `(0, 1)`).
    Anonymous,
}

impl core::fmt::Display for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let s = match self {
            ParseError::Syntax => "input is not well-formed paper notation",
            ParseError::NotCanonical => "input is well-formed but not in canonical normal form",
            ParseError::Anonymous => "input denotes the anonymous identity 0, not a nonzero share",
        };
        f.write_str(s)
    }
}

impl std::error::Error for ParseError {}
