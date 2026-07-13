//! A collection of traversals over the typed tree structure, each of which is
//! inductive over the height.
//!
//! Traversals are exposed as free functions to avoid the necessity of the
//! caller importing a trait, even though under the hood they are implemented
//! using polymorphic recursion through traits.

use super::*;

// `act` and `unknown` are `pub(crate)` so rustdoc elsewhere (e.g. the
// `Levels` docs) can link to the traversal traits inside them: a private
// `mod` is unnameable from outside `traverse`, so the links would not
// resolve. The free-function facade below remains the API.
pub(crate) mod act;
pub use act::{Action, act};

pub(crate) mod enumerate;
pub(crate) mod unknown;

mod join;
pub use join::join;
