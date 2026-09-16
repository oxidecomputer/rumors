//! Operations over the id encoding.
//!
//! Each node is a 2-bit presence tag (see [`idbits`](crate::idbits)): a `0` is
//! the absence of a child, never a node. Consuming cursors keep each traversal
//! linear and iterative: a pruned subtree is skipped once, and a deep id grows
//! explicit state rather than the call stack. Canonical form makes emptiness
//! and fullness constant-time leaf checks.

mod build;
mod compare;
mod diff;
mod split;
mod sum;
mod sum_split;
