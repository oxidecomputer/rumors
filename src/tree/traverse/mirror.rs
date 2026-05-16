//! Bidirectional alternating mirror-sync between two replicas of the typed tree.
//!
//! # Wire format
//!
//! Each message and every typed subtree is borsh-encoded. The encoding is
//! canonical (one byte sequence per value) and reflects the in-memory
//! representation directly — no leaf-vs-branch tag, no redundant version
//! fields above the leaf level. Container lengths are `u32` little-endian.
//!
//! ## Atoms
//!
//! - [`typed::Hash`](crate::tree::typed::Hash): 32 raw bytes.
//! - [`typed::Prefix<H>`](crate::tree::typed::Prefix): exactly `32 −
//!   H::HEIGHT` raw bytes, no length prefix (the type pins the byte count).
//! - [`Version<P>`](crate::Version) and [`Message<T>`](crate::Message):
//!   their existing borsh shapes (see those types).
//! - [`OrdMap<K, V>`](imbl::OrdMap), [`OrdSet<T>`](imbl::OrdSet): `u32`
//!   length followed by every entry in strictly-ascending key order;
//!   decoders reject duplicates and out-of-order keys (see the
//!   `imbl_borsh` module).
//!
//! ## Typed [`Node<P, T, H>`](crate::tree::typed::Node)
//!
//! Encoded in its in-memory layout. The typed `BorshSerialize` impl is a
//! thin delegate over the untyped node's `serialize_to`, which is the
//! canonical encoder:
//!
//! ```text
//! NodeWire ::=
//!     prefix_len: u8                  // path-compressed prefix byte count
//!     [u8; prefix_len]                // head bytes, shallowest first
//!     body                            // dispatched on `children`:
//!         Children::Leaf:   version: Version<P>, message: Message<T>
//!         Children::Branch: count_minus_two: u8, [(radix: u8, NodeWire); count]
//! ```
//!
//! The body's shape is **not** tagged on the wire; the receiver determines
//! it from the typed height (`Z` ⇒ leaf, `S<_>` ⇒ branch) together with
//! the running `prefix_len`. On the decode side, when `prefix_len > 0` we
//! peel one head byte and recurse at the next-finer typed height,
//! synthesizing the `prefix_len − 1` byte for the inner reader via
//! [`borsh::io::Read::chain`] — so the wire carries one `prefix_len` byte
//! per top-of-chain rather than one per typed level.
//!
//! Multi-child branches always carry at least two children; singletons
//! appear on the wire only as `prefix_len > 0` and reconstruct through
//! [`Node::beneath`](crate::tree::typed::Node::beneath). Branch radices
//! are required to be strictly ascending (matching the backing `OrdMap`'s
//! canonical iteration order).
//!
//! ## Messages
//!
//! Each of the five message types (see [`message`]) is the borsh
//! concatenation of its fields in source order. The `providing` /
//! `requested` / `uncertain` channels use the shared `OrdMap`/`OrdSet`
//! encoding. There is no length framing between messages on the wire:
//! the protocol's height schedule names the type each side expects next.
//!
//! ## Test coverage
//!
//! - `tree::typed::test`: round-trip proptests for `Hash`, `Prefix<H>`
//!   at every height, and the typed root `Node<P, T, Root>`. Includes
//!   negative tests for the decoder rejection paths.
//! - `imbl_borsh::test`: negative tests for the `OrdMap`/`OrdSet` helpers.
//! - `mirror::test`: round-trip proptests for each message type.
//! - `mirror::wire_snapshot`: insta snapshots pinning the exact bytes of
//!   representative and corner-case values.

mod local;
mod message;
pub mod protocol;

#[cfg(test)]
mod message_test;

#[cfg(test)]
mod test;

#[cfg(test)]
mod wire_snapshot;
