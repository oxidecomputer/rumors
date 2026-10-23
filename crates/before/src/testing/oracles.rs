//! Independent semantic models used to check the production implementation.
//!
//! [`tree`] directly follows the paper's recursive tree definitions. The
//! test-only function oracle realizes parties and versions as functions over the unit
//! interval, so it shares neither representation nor traversal with the tree
//! implementation. Tests use agreement among all three as stronger evidence
//! than any pair alone can provide.
//!
//! The test-only codec module models the wire formats rather than the
//! algebra. It shares no code with production's writers, so the near-valid
//! generators can spell the non-canonical encodings no production writer
//! emits; its canonical spellings are calibrated against production, which
//! is the only authority for the version and party bit layouts.
//!
//! The test-only dyadic oracle models an exact rank as an unnormalized dyadic
//! rational, so rank values, order, and text are judged without production's
//! arithmetic or codecs.

#[cfg(test)]
pub(crate) mod codec;
#[cfg(test)]
pub(crate) mod dyadic;
#[cfg(test)]
pub(crate) mod function;

#[cfg(any(test, feature = "oracle"))]
pub mod tree;
