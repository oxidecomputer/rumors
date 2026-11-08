//! Conversion between scoped protocol replies and prefix-free wire frames,
//! chunk by chunk.
//!
//! The materialized protocol and the wire deliberately speak at different
//! levels. In memory, one [`Reply`](super::super::erased::Reply) contains
//! backend node handles and omits its prefix because the receiver already knows
//! which earlier question it answers. On the wire, a supplied node is flattened
//! into runs of backend-neutral `(Version, Message)` leaf records which
//! still carry neither prefix nor radix. This module is the lossless boundary
//! between them:
//!
//! ```text
//! Reply<E> -- encode + explode --> Frame leaves
//! Reply<E> <-- decode + assemble -- Frame leaves
//! ```
//!
//! # Recovering omitted scope
//!
//! A [`Scope`] is the durable part of one sent `Query`: its parent prefix,
//! the listed child radices with the counts their lister gave them, and the
//! bound on the reply it awaits. `Match` and `Query` reactions consume those radices
//! positionally; a nested `Query` thereby creates the lower scope which will
//! interpret its future reply. A reply's one `Match` also names the later
//! radices it covers, which take no reaction: each positional reaction
//! passes over the covered radices before it, and [`Replay`] carries how far
//! a reply has replayed, covered radices included, from one chunk to the
//! next. `Supply` does not consume the positional cursor:
//! its version-derived path recovers its child radix independently. The
//! leaf-height exception is an empty `Query`: it consumes its leaf position and
//! requests that leaf itself, creating a terminal scope at the same height
//! rather than descending. The initiator's opening reply is the sole
//! exception to “one reply answers one earlier question”: it seeds the
//! root scope directly, and its question occupies no wire frame because
//! the listing rides the greeting
//! ([`Greeting`](super::super::message::Greeting)). [`opening_listing`]
//! validates the local opening's one-query shape and returns its listing;
//! [`opening_reply`] replays the peer's listing as the message the
//! responder answers.
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
//! leaf's full path from its version and serialized message, rejects a leaf
//! whose version breaks the session's [`SupplyBounds`] (one outside the
//! peer's declared version or size, or one this side has already seen),
//! rejects paths
//! outside the retained scope, and groups consecutive leaves by their
//! prefix at the scope's children height. Strict path and run ordering make
//! those group
//! boundaries unambiguous without another delimiter or a trusted
//! peer-supplied key.
//!
//! The decoder yields each record as it is decoded through a fan-bounded
//! channel into [`Backend::assemble`](super::super::Backend::assemble),
//! passing custody of the payload to the backend at construction
//! ([`Leaf::leaf`](super::super::Leaf::leaf)) before the leaf enters the
//! channel. A group of consecutive supplies ends at the next positional
//! reaction or at the reply's end, and the assembler then finishes the
//! group's last node and sends the group's nodes back. Meanwhile the reader
//! retains only the reactions it has read and not yet delivered, and one
//! placeholder per run of the open group; it delivers the complete
//! reactions in a chunk at each question the reply asks before its last
//! reaction, and the rest with the reply's end. Thus
//! memory remains those reactions, their completed node handles, one
//! encoded run, and the buffered fan of decoded leaves; no subtree payload
//! is accumulated merely to cross the backend boundary.
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

pub(crate) use decode::decode_ahead_bytes;
#[cfg(test)]
pub use decode::decode_whole_reply;
pub use decode::{LeafEdge, SupplyBounds, decode_reply, opening_reply};
pub use encode::{Encoded, ReplyEncoder, opening_listing};
#[cfg(test)]
pub use encode::{encode_leaf_reply, encode_reply};
#[cfg(test)]
pub use error::BoundedPart;
pub use error::{DecodeError, EncodeError, OpeningError, ScopeError};
pub(crate) use scope::RecordedChild;
pub use scope::{Replay, ReplyLevel, Scope, derive_questions};

#[cfg(test)]
mod tests;
