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

pub use crate::peer::{PROTOCOL_MAGIC, PROTOCOL_VERSION};
pub use ::before;
pub use ::borsh;
pub use batch::Batch;
pub use before::{Version, causally};
pub use network::Network;
pub use peer::{Peer, Retire};
pub use rumors::{CausalMessages, Messages, Rumors};
pub use snapshot::Snapshot;
pub use tree::Key;
pub use tree::mirror::remote::Error;

pub(crate) use peer::Inner;
