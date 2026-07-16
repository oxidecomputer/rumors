//! Conversion between scoped protocol replies and prefix-free wire frames.
//!
//! The materialized protocol and the wire deliberately speak at different
//! levels. In memory, one [`Reply`](super::super::message::Reply) contains
//! backend node handles and omits its prefix because the receiver already knows
//! which earlier question it answers. On the wire, a supplied node is flattened
//! into backend-neutral `(Version, Message<T>)` leaves and still carries neither
//! prefix nor radix. This module is the lossless boundary between them:
//!
//! ```text
//! Reply<B, T, H> -- encode + explode --> Frame<T> leaves
//! Reply<B, T, H> <-- decode + assemble -- Frame<T> leaves
//! ```
//!
//! # Recovering omitted scope
//!
//! A [`Scope`] is the durable part of one sent `Query`: its parent prefix and
//! the listed child radices. `Match` and `Query` reactions consume those radices
//! positionally; a nested `Query` thereby creates the lower scope which will
//! interpret its future reply. `Supply` does not consume the positional cursor:
//! its content-derived path recovers its child radix independently. The
//! leaf-height exception is an empty `Query`: it consumes its leaf position and
//! requests that leaf itself, creating a terminal scope at the same height
//! rather than descending. The initiator's opening query is the sole exception
//! to “one reply answers one earlier question,” so it seeds the root scope
//! directly.
//!
//! Encoding attaches a newly created scope to the exact frame containing its
//! `Query`. [`Encoded::write_with`] releases that scope only after the supplied
//! writer reports success, making the materialized walk's “wire before internal
//! publication” liveness rule the natural API order.
//!
//! # Supplying backend nodes as leaves
//!
//! Encoding asks [`Backend::leaves`](super::super::Backend::leaves) to flatten
//! each `Supply(radix, node)`. Decoding recomputes every leaf's full path from
//! its version and serialized message, rejects paths outside the retained
//! scope, and groups consecutive leaves by their height-`H` prefix. Strict path
//! and run ordering make those group boundaries unambiguous without another
//! delimiter or a trusted peer-supplied key.
//!
//! The decoder feeds leaves through a one-slot channel into the existing
//! [`Convert::assemble`](super::super::convert::Convert::assemble) fold. While
//! that fold rebuilds backend nodes, the reader retains only the reply skeleton
//! (`Match`, `Query`, or a supplied-prefix placeholder). Completed nodes fill
//! those placeholders after the reply end arrives. Thus memory remains one
//! finite reply, its completed node handles, and one leaf in flight; no subtree
//! payload is accumulated merely to cross the backend boundary.
//!
//! # Why this is sufficient
//!
//! Four protocol properties make the conversion invertible: questions and
//! replies are paired in order; leaf paths are functions of leaf contents;
//! supplied paths are strictly ascending; and every reply has an explicit end.
//! The adapter adds no new identity or ordering authority of its own.

mod decode;
mod encode;
mod error;
mod scope;

pub use decode::{Decoded, decode_leaf_reply, decode_opening, decode_reply};
pub use encode::{Encoded, encode_leaf_reply, encode_opening, encode_reply};
pub use error::{DecodeError, EncodeError, OpeningError, ScopeError};
pub use scope::Scope;

#[cfg(test)]
mod tests;
