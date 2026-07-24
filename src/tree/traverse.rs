//! Traversals over the typed tree structure, each inductive over the height.
//!
//! Each traversal is exposed as a free function so callers need not import a
//! trait, though under the hood all are implemented by polymorphic recursion
//! through traits.

use super::*;

// `act` and `unknown` are `pub(crate)` so rustdoc elsewhere (e.g. the
// `Levels` docs) can link to the traversal traits inside them: a private
// `mod` is unnameable from outside `traverse`, so the links would not
// resolve. The free-function facade below remains the API.
pub(crate) mod act;
pub use act::{Action, act};

pub(crate) mod unknown;

mod join;
pub use join::join;
