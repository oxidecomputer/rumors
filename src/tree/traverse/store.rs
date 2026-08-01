//! Backend-generic counterparts of the synchronous traversal towers: the
//! same inductive recursions, awaiting a [`Backend`](crate::tree::backend::Backend)
//! where the in-memory towers dereference an `Arc`.
//!
//! Each tower reproduces its synchronous twin's verdicts node for node —
//! [`mod@act`] mirrors [`fn@crate::tree::traverse::act`], [`mod@join`]
//! mirrors [`fn@crate::tree::traverse::join`], [`mod@unknown`]
//! mirrors [`crate::tree::traverse::unknown`] — and the
//! crate's backend conformance suite pins the agreement by differential
//! proptest. The in-memory [`Local`](crate::tree::backend::Local) backend
//! overrides every [`Store`](crate::tree::backend::Store) seam with the
//! synchronous engines, so the [`mod@act`], [`mod@join`], [`mod@get`], and
//! [`mod@walk`] towers monomorphize only for backends that own storage of
//! their own. The [`mod@unknown`] tower is the exception: it is also the
//! streaming mirror's deletion-honoring filter, so it runs for every
//! session backend — [`Local`](crate::tree::backend::Local) included.
//!
//! Every level returns a [`BoxFuture`](futures::future::BoxFuture) (or a
//! boxed stream): an `impl Future` return would nest each level's `async`
//! type inside the next, ballooning the compiler's type exponentially over
//! the 32-level descent. Where a level carries a per-radix work list, it is
//! collected into a `Vec` before recursing, for the same reason the
//! synchronous [`fn@crate::tree::traverse::act`] tower collects: a lazy
//! iterator would weave each level's closure type into the next level's
//! instantiation.

// The towers speak in per-height projections of generic associated types;
// naming each would mint aliases with no semantic weight.
#![allow(clippy::type_complexity)]

pub mod act;
pub mod get;
pub mod join;
pub mod unknown;
pub mod walk;

pub use act::act;
pub use join::join;
