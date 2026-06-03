//! The crate's causal [`Version`] type.
//!
//! This is a re-export of [`before::Version`], the Interval Tree Clock event
//! tree: a causal timestamp partially ordered by `<=` (causal containment),
//! joined by `|` (least upper bound), and advanced by ticking a
//! [`before::Party`]. See the [`before`] crate for the full semantics.

pub use before::Version;
