//! # Wire format
//!
//! Each message is borsh-encoded. The encoding is canonical (one byte sequence
//! per value) and reflects the in-memory representation directly. Container
//! lengths are `u32` little-endian.
//!
//! ## Atoms
//!
//! - [`typed::Hash`](crate::tree::typed::Hash): 32 raw bytes.
//! - [`typed::Prefix<H>`](crate::tree::typed::Prefix): exactly `32 −
//!   H::HEIGHT` raw bytes, no length prefix (the type pins the byte count).
//! - [`Version`](crate::Version) and [`Message<T>`](crate::Message):
//!   their existing borsh shapes (see those types). A `Message<T>` serializes
//!   byte-identically to its inner `T`.
//! - `Vec<_>`: `u32` length followed by each element in order. Every channel is
//!   a length-prefixed `Vec`; on deserialize the decoder rejects any frame
//!   whose entries are not strictly ascending in canonical order (which also
//!   rejects duplicates), reimposing the one-encoding-per-value guarantee the
//!   old `de_strict_order` `BTreeMap`/`BTreeSet` channels gave (see
//!   [`super::reassemble`]).
//!
//! ## The three channels
//!
//! - **`providing`**: `Vec<(Key, Version, Message<T>)>` — the *leaves* of the
//!   subtrees being provided, in ascending content-addressed-path order. The
//!   prefixes and tree structure are **elided**: each leaf carries its own
//!   [`Key`] — which *is* its content-addressed path
//!   `blake3(blake3(version) ‖ blake3(value))` ([`Path::for_leaf`]) — so the
//!   receiver re-materializes the subtrees ([`reassemble_providing`]) from the
//!   transmitted key without re-hashing the `(version, value)`. The provider
//!   already holds the hash; sending it spares the receiver the recompute (up
//!   to ~4× the cost of placement) at the price of 32 bytes per leaf. Release
//!   builds *trust* the key; debug builds recompute the path and assert it
//!   matches (see [`reassemble_providing`]). Rejected unless strictly ascending
//!   by transmitted key.
//! - **`requested`**: `Vec<Prefix<_>>` — prefixes the peer should send next
//!   round. Rejected unless strictly ascending.
//! - **`uncertain`**: `Vec<(Prefix<_>, Hash)>` — frontier subtree hashes for the
//!   peer to compare against its own. Rejected unless strictly ascending by
//!   prefix. (Prefixes here cannot be elided: the peer has no content from which
//!   to re-derive them, so these bytes are byte-identical to the old map
//!   encoding.)
//!
//! ## Messages
//!
//! Each of the five message types (see [`message`]) is the borsh concatenation
//! of its fields in source order. There is no length framing between messages
//! on the wire: the protocol's height schedule names the type each side expects
//! next.
//!
//! [`Path::for_leaf`]: crate::tree::typed::Path::for_leaf
//! [`reassemble_providing`]: super::reassemble::reassemble_providing

use borsh::{BorshDeserialize, BorshSerialize};

use crate::message::Message;
use crate::tree::key::Key;
use crate::tree::typed::{
    Hash, Prefix,
    height::{Height, Pred, Root, S, Z},
};
use crate::version::Version;

use super::reassemble::{
    verify_keys_canonical, verify_pairs_canonical, verify_providing_canonical,
};

/// The initiator's opening message: a single hash at the empty (root) prefix,
/// namely our root hash.
///
/// Carries the same shape as [`Opening`]: an `uncertain` map at `Root` height,
/// populated with one entry. Distinct from `Opening` only by height -- and from
/// [`Exchange`] by the absence of `providing` / `requested`, which can't be
/// populated until at least one round has passed.
#[derive(Clone, Default, BorshSerialize)]
pub struct Initiate {
    pub uncertain: Vec<(Prefix<Root>, Hash)>,
}

impl BorshDeserialize for Initiate {
    fn deserialize_reader<R: borsh::io::Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let uncertain = Vec::deserialize_reader(reader)?;
        verify_pairs_canonical(&uncertain, "Initiate.uncertain")?;
        Ok(Self { uncertain })
    }
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
#[derive(Clone, Default, BorshSerialize)]
pub struct Opening {
    pub uncertain: Vec<(Prefix<UnderRoot>, Hash)>,
}

impl BorshDeserialize for Opening {
    fn deserialize_reader<R: borsh::io::Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let uncertain = Vec::deserialize_reader(reader)?;
        verify_pairs_canonical(&uncertain, "Opening.uncertain")?;
        Ok(Self { uncertain })
    }
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
    ///
    /// On the wire this is just the *leaves* of those subtrees — a flat list of
    /// `(key, version, value)` triples in ascending key order, with every prefix
    /// and structural byte elided. Each [`Key`] is the leaf's content-addressed
    /// path, which the receiver uses directly to re-materialize the subtrees
    /// without re-hashing; debug builds recompute it and assert the match (see
    /// [`super::reassemble`]).
    pub providing: Vec<(Key, Version, Message<T>)>,
    /// Prefixes the counterparty listed in the previous round's `uncertain`
    /// that we lack entirely. We ask them to send the subtrees so we can insert
    /// them into our zipper. Strictly ascending; duplicates are rejected.
    pub requested: Vec<Prefix<S<H>>>,
    /// Hashes of our subtrees at this round's frontier, for the counterparty
    /// to compare against their own. Each entry routes to one cell of the
    /// asymmetry matrix on the receiving side. Strictly ascending by prefix.
    pub uncertain: Vec<(Prefix<H>, Hash)>,
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

impl<T, H> BorshDeserialize for Exchange<T, H>
where
    T: BorshDeserialize,
    S<H>: Height,
    H: Height,
{
    fn deserialize_reader<R: borsh::io::Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let providing: Vec<(Key, Version, Message<T>)> =
            BorshDeserialize::deserialize_reader(reader)?;
        verify_providing_canonical(&providing)?;
        let requested: Vec<Prefix<S<H>>> = BorshDeserialize::deserialize_reader(reader)?;
        verify_keys_canonical(&requested, "Exchange.requested")?;
        let uncertain: Vec<(Prefix<H>, Hash)> = BorshDeserialize::deserialize_reader(reader)?;
        verify_pairs_canonical(&uncertain, "Exchange.uncertain")?;
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
    pub providing: Vec<(Key, Version, Message<T>)>,
    pub requested: Vec<Prefix<S<Z>>>,
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
        let providing: Vec<(Key, Version, Message<T>)> =
            BorshDeserialize::deserialize_reader(reader)?;
        verify_providing_canonical(&providing)?;
        let requested: Vec<Prefix<S<Z>>> = BorshDeserialize::deserialize_reader(reader)?;
        verify_keys_canonical(&requested, "Closing.requested")?;
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
    pub providing: Vec<(Key, Version, Message<T>)>,
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
        let providing: Vec<(Key, Version, Message<T>)> =
            BorshDeserialize::deserialize_reader(reader)?;
        verify_providing_canonical(&providing)?;
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
