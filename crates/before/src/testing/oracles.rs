//! Independent semantic models used to check the production implementation.
//!
//! [`tree`] directly follows the paper's recursive tree definitions. The
//! test-only function oracle realizes parties and versions as functions over the unit
//! interval, so it shares neither representation nor traversal with the tree
//! implementation. Tests use agreement among all three as stronger evidence
//! than any pair alone can provide.

#[cfg(test)]
pub(crate) mod function;

#[cfg(any(test, feature = "oracle"))]
pub mod tree;
