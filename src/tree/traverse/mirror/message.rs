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
//! - [`Version`](crate::Version) and [`Message<T>`](crate::Message):
//!   their existing borsh shapes (see those types).
//! - [`BTreeMap<K, V>`](std::collections::BTreeMap),
//!   [`BTreeSet<T>`](std::collections::BTreeSet): `u32` length followed by
//!   every entry in strictly-ascending key order. These channels are
//!   single-use and never forked, so a plain ordered map suffices — no
//!   persistence needed. `borsh`'s `de_strict_order` feature makes the
//!   decoders reject duplicates and out-of-order keys, giving each value one
//!   canonical encoding.
//!
//! ## Typed [`Node<T, H>`](crate::tree::typed::Node)
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
//!         Children::Leaf:   version: Version, message: Message<T>
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
//! `requested` / `uncertain` channels use the standard `BTreeMap`/`BTreeSet`
//! encoding. There is no length framing between messages on the wire:
//! the protocol's height schedule names the type each side expects next.

use std::collections::{BTreeMap, BTreeSet};

use borsh::{BorshDeserialize, BorshSerialize};

use crate::tree::typed::{
    Hash, Node, Prefix,
    height::{Height, Pred, Root, S, Z},
};

/// The initiator's opening message: a single hash at the empty (root) prefix,
/// namely our root hash.
///
/// Carries the same shape as [`Opening`]: an `uncertain` map at `Root` height,
/// populated with one entry. Distinct from `Opening` only by height -- and from
/// [`Exchange`] by the absence of `providing` / `requested`, which can't be
/// populated until at least one round has passed.
#[derive(Clone, Default, BorshSerialize, BorshDeserialize)]
pub struct Initiate {
    pub uncertain: BTreeMap<Prefix<Root>, Hash>,
}

/// The responder's opening message: one hash per child of the responder's root,
/// listed unconditionally because the responder has not yet learned what the
/// initiator holds.
///
/// Distinct from [`Exchange`] by the absence of `providing` and `requested`:
/// the responder has not yet been asked for anything, nor seen any of the
/// initiator's `uncertain` to react to. Encoding the asymmetry in the type
/// system makes the initiator's first call
/// ([`super::exchange::Exchange::open_initiator`]) a separate entry point from
/// the steady-state `exchange`, so the latter can assume every uncertain hash
/// describes a parent the receiver has already acknowledged.
#[derive(Clone, Default, BorshSerialize, BorshDeserialize)]
pub struct Opening {
    pub uncertain: BTreeMap<Prefix<UnderRoot>, Hash>,
}

/// The steady-state message: carries all three channels (see the
/// asymmetry-matrix table in the [`super`] module docs).
#[derive(Clone)]
pub struct Exchange<T, H>
where
    S<H>: Height,
    H: Height,
{
    /// Subtrees the counterparty does not have. Populated from two sources:
    /// nodes they `requested` in the previous round, and nodes we unilaterally
    /// know they lack (because they did not list them in the previous round's
    /// `uncertain`).
    ///
    /// In both cases the subtrees are filtered against the counterparty's
    /// version vector: anything causally `<=` their version has either been
    /// already-seen or already-forgotten on their side, so the receiver's view
    /// must agree with ours by treating the absence as a deletion.
    pub providing: BTreeMap<Prefix<S<H>>, Node<T, S<H>>>,
    /// Prefixes the counterparty listed in the previous round's `uncertain`
    /// that we lack entirely. We ask them to send the subtrees so we can insert
    /// them into our zipper.
    pub requested: BTreeSet<Prefix<S<H>>>,
    /// Hashes of our subtrees at this round's frontier, for the counterparty
    /// to compare against their own. Each entry routes to one cell of the
    /// asymmetry matrix on the receiving side.
    pub uncertain: BTreeMap<Prefix<H>, Hash>,
}

impl<T, H> BorshSerialize for Exchange<T, H>
where
    S<H>: Height,
    H: Height,
{
    fn serialize<W: borsh::io::Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        self.providing.serialize(writer)?;
        self.requested.serialize(writer)?;
        self.uncertain.serialize(writer)?;
        Ok(())
    }
}

// `Node<T, S<H>>: BorshDeserialize` reduces inductively to
// `Node<T, H>: BorshDeserialize` and bottoms at `Z`, so with `H` left
// generic the proof obligation doesn't terminate during inference. We
// thread `Node<T, S<H>>: BorshDeserialize` through as an explicit
// bound so the caller — who knows `H` concretely — discharges it.
impl<T, H> BorshDeserialize for Exchange<T, H>
where
    T: BorshDeserialize,
    S<H>: Height,
    H: Height,
    Node<T, S<H>>: BorshDeserialize,
{
    fn deserialize_reader<R: borsh::io::Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let providing = BorshDeserialize::deserialize_reader(reader)?;
        let requested = BorshDeserialize::deserialize_reader(reader)?;
        let uncertain = BorshDeserialize::deserialize_reader(reader)?;
        Ok(Self {
            providing,
            requested,
            uncertain,
        })
    }
}

impl<T> From<Opening> for Exchange<T, UnderRoot> {
    fn from(Opening { uncertain }: Opening) -> Self {
        Exchange {
            uncertain,
            ..Default::default()
        }
    }
}

impl<T, H> Default for Exchange<T, H>
where
    S<H>: Height,
    H: Height,
{
    fn default() -> Self {
        Self {
            providing: Default::default(),
            requested: Default::default(),
            uncertain: Default::default(),
        }
    }
}

/// The initiator's closing message: a final `providing`/`requested` pair at
/// `S<Z>`, emitted by [`super::exchange::Exchange::close_initiator`].
///
/// Distinct from [`Exchange`] by the absence of `uncertain`: at leaf height,
/// any two parties either have a leaf at the same path (in which case the leaf
/// hashes match: they are the same all-ones sentinel) or one of them lacks it
/// (in which case the receiver routes the missing prefix to `requested`, never
/// `uncertain`). Encoding the vacuity in the type system lets
/// [`super::exchange::Exchange::complete_responder`] consume `Closing`
/// directly, without a runtime check against an out-of-spec initiator.
#[derive(Clone)]
pub struct Closing<T> {
    pub providing: BTreeMap<Prefix<S<Z>>, Node<T, S<Z>>>,
    pub requested: BTreeSet<Prefix<S<Z>>>,
}

impl<T> BorshSerialize for Closing<T> {
    fn serialize<W: borsh::io::Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        self.providing.serialize(writer)?;
        self.requested.serialize(writer)?;
        Ok(())
    }
}

impl<T> BorshDeserialize for Closing<T>
where
    T: BorshDeserialize,
{
    fn deserialize_reader<R: borsh::io::Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let providing = BorshDeserialize::deserialize_reader(reader)?;
        let requested = BorshDeserialize::deserialize_reader(reader)?;
        Ok(Self {
            providing,
            requested,
        })
    }
}

impl<T> From<Exchange<T, Z>> for Closing<T> {
    fn from(
        Exchange {
            providing,
            requested,
            uncertain: _,
        }: Exchange<T, Z>,
    ) -> Self {
        Closing {
            providing,
            requested,
        }
    }
}

impl<T> Default for Closing<T> {
    fn default() -> Self {
        Self {
            providing: Default::default(),
            requested: Default::default(),
        }
    }
}

/// The responder's closing message: the final `providing` at leaf height,
/// emitted by [`super::exchange::Exchange::complete_responder`] for the
/// initiator to absorb in [`super::exchange::Exchange::complete_initiator`].
///
/// No `requested` (the initiator never replies after this) and no `uncertain`
/// (vacuous at leaf height, same reasoning as [`Closing`]).
#[derive(Clone)]
pub struct Complete<T> {
    pub providing: BTreeMap<Prefix<Z>, Node<T, Z>>,
}

impl<T> BorshSerialize for Complete<T> {
    fn serialize<W: borsh::io::Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        self.providing.serialize(writer)
    }
}

impl<T> BorshDeserialize for Complete<T>
where
    T: BorshDeserialize,
{
    fn deserialize_reader<R: borsh::io::Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let providing = BorshDeserialize::deserialize_reader(reader)?;
        Ok(Self { providing })
    }
}

impl<T> Default for Complete<T> {
    fn default() -> Self {
        Self {
            providing: Default::default(),
        }
    }
}

/// The height just under the root, i.e. 31. The responder's opening message
/// carries hashes at this height -- one for each child of its root.
pub type UnderRoot = <Root as Pred>::Pred;

/// The height two levels under the root, i.e. 30.
pub type UnderUnderRoot = <UnderRoot as Pred>::Pred;
