//! A collection of traversals over the typed tree structure, each of which is
//! inductive over the height. Traversals are exposed as free functions to avoid
//! the necessity of the caller importing a trait, even though under the hood
//! they are implemented using polymorphic recursion through traits.

use super::*;

mod act;
pub use act::{Action, act};

mod get;
pub use get::get;

mod unknown;
pub use unknown::unknown;

mod mirror;
pub use mirror::mirror;
