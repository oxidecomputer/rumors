//! The height-typed veneer over the untyped storage nodes.
//!
//! Storage wants exactly one node representation, so structural sharing,
//! path compression, and the lazy memos work uniformly ([`untyped`]). The
//! traversals want the height in the type, so each inductive step
//! monomorphizes separately and the recursion provably bottoms out at
//! [`Z`](height::Z) instead of trusting a runtime depth counter. This
//! module is the adapter: [`Node<T, H>`](Node) pairs an untyped node with
//! a phantom [`Height`](height::Height), and the typed surface
//! ([`Path`], [`Children`], [`Levels`]) keeps every pop, descent, and
//! reassembly height-correct at compile time.

pub mod hash;
pub mod height;
pub mod levels;
pub mod node;
pub mod path;
pub mod prefix;

// `pub(crate)` so rustdoc elsewhere can link to `typed::untyped::Range`: a
// private `mod` is unnameable from outside `typed`, so the links would not
// resolve. The items below are still re-exported as the canonical paths.
pub(crate) mod untyped;

#[cfg(test)]
mod tests;

pub use hash::Hash;
pub use levels::{Level, Levels};
pub use node::{Children, Node};
pub use path::Path;
pub use prefix::Prefix;
pub use untyped::{RangeOwned, Iter, Leaf};
