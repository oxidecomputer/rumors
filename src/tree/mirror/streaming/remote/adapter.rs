//! Conversion between scoped protocol replies and prefix-free wire frames.
//!
//! The materialized protocol and the wire deliberately speak at different
//! levels. In memory, one [`Reply`](super::super::message::Reply) contains
//! backend node handles and omits its prefix because the receiver already knows
//! which earlier question it answers. On the wire, a supplied node is flattened
//! into runs of backend-neutral `(Version, Message)` leaf records which
//! still carry neither prefix nor radix. This module is the lossless boundary
//! between them:
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
//! its version-derived path recovers its child radix independently. The
//! leaf-height exception is an empty `Query`: it consumes its leaf position and
//! requests that leaf itself, creating a terminal scope at the same height
//! rather than descending. The initiator's opening reply is the sole
//! exception to “one reply answers one earlier question”: it seeds the
//! root scope directly, and its question occupies no wire frame because
//! the listing rides the greeting
//! ([`Greeting`](super::super::message::Greeting)). [`opening_parts`]
//! validates the local reply's query-then-supplies shape and splits off
//! the early supplies that do cross; [`opening_reply`] replays the
//! peer's listing as the message the responder answers. The early
//! supplies travel as one supplies-only reply, decoded incrementally by
//! [`early_supplies`] so each whole root child surfaces the moment its
//! records complete.
//!
//! Encoding attaches a newly created scope to the exact frame containing its
//! `Query`. [`Encoded::write_with`] releases that scope only after the supplied
//! writer reports success, making the materialized walk's “wire before internal
//! publication” liveness rule the natural API order.
//!
//! # Supplying backend nodes as leaf runs
//!
//! Encoding asks [`Backend::leaves`](super::super::Backend::leaves) to flatten
//! each `Supply(radix, node)`, and batches the enumerated leaves into wire
//! runs chunked by the session's [`RunBudget`](super::codec::RunBudget): a
//! run flushes when the next record would overflow the budget, always holds
//! at least one record, and never spans reactions. Decoding is
//! batching-agnostic — it walks records, not frames: it recomputes every
//! leaf's full path from its version and serialized message, rejects paths
//! outside the retained scope, and groups consecutive leaves by their
//! height-`H` prefix. Strict path and run ordering make those group
//! boundaries unambiguous without another delimiter or a trusted
//! peer-supplied key.
//!
//! The decoder yields each record as it is decoded through a fan-bounded
//! channel into the existing
//! [`Convert::assemble`](super::super::convert::Convert::assemble) fold,
//! passing custody of the payload to the backend at construction
//! ([`Leaf::leaf`](super::super::Leaf::leaf)) before the leaf enters the
//! channel. While that fold rebuilds backend nodes, the reader retains only
//! the reply skeleton (`Match`, `Query`, or a supplied-prefix placeholder).
//! Completed nodes fill those placeholders after the reply end arrives. Thus
//! memory remains one finite reply, its completed node handles, one encoded
//! run, and the buffered fan of backend-priced leaves — the session budget's
//! supply-decode charge; no subtree payload is accumulated merely to cross
//! the backend boundary.
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

pub use decode::{Decoded, decode_leaf_reply, decode_reply, early_supplies, opening_reply};
pub use encode::{Encoded, encode_leaf_reply, encode_reply, opening_parts};
pub use error::{DecodeError, EncodeError, OpeningError, ScopeError};
pub use scope::Scope;

#[cfg(test)]
mod tests;
