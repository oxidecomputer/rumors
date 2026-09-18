//! Traversals over the typed tree structure, each inductive over the height.
//!
//! `act` and `join` provide entry functions. Their work, and the internal
//! deletion filter, use height-indexed traits for polymorphic recursion.

use super::*;

// `act` and `unknown` are `pub(crate)` so rustdoc elsewhere in the crate
// can link to the traversal traits inside them: a private `mod` is
// unnameable from outside `traverse`, so the links would not resolve. The
// entry functions below remain the API.
pub(crate) mod act;
pub use act::{Action, act};

pub(crate) mod unknown;

mod join;
pub use join::join;
