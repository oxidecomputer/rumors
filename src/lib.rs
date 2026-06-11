//! Unordered gossip with redaction.

// Static assertions uses #[allow(unsafe_code)], so we allow it only in tests
#![cfg_attr(not(test), forbid(unsafe_code))]
// Programmer error in recursive async traits can create large futures, so we
// check to make sure it's not an issue
#![deny(clippy::large_futures)]

pub mod sync;

mod batch;
mod bookmark;
mod message;
mod network;
mod peer;
mod rumors;
mod snapshot;
mod tree;

#[cfg(test)]
mod tests;

pub use batch::Batch;
pub use network::Network;
pub use peer::{PROTOCOL_MAGIC, PROTOCOL_VERSION, Peer, Retire};
pub use rumors::{CausalMessages, Messages, Rumors};
pub use snapshot::{Iter, Snapshot};

pub(crate) use peer::Inner;

/// The error type returned by [`Rumors::gossip`].
pub use tree::mirror::remote::Error;

/// An opaque identifier for a single message in a rumor set.
pub use tree::Key;

/// A causal version vector tagging when a message was observed.
///
/// This is a re-export of [`before::Version`], the Interval Tree Clock event
/// tree: a causal timestamp partially ordered by `<=` (causal containment),
/// joined by `|` (least upper bound), and advanced by ticking a
/// [`before::Party`]. See the [`before`] crate for the full semantics.
pub use before::Version;

/// Named, composable constructors for causal [`Version`] ranges
/// (re-exported from [`before`]): the vocabulary for
/// [`Snapshot::range`] and [`Rumors::messages_since`] — e.g.
/// `causally::since(&checkpoint)` or `causally::not_before(&s).known_at(&e)`.
pub use before::causally;

/// The [`borsh`] crate, re-exported.
///
/// Message types must implement [`BorshSerialize`](borsh::BorshSerialize) and
/// [`BorshDeserialize`](borsh::BorshDeserialize); re-exporting borsh here lets
/// callers derive both without a separate dependency.
pub use ::borsh;
