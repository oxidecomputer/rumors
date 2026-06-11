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
//! - [`Version`] and [`Message<T>`](crate::message::Message):
//!   their existing borsh shapes (see those types). A `Message<T>` serializes
//!   byte-identically to its inner `T`.
//! - `Vec<_>`: `u32` length followed by each element in order. Every channel
//!   is a length-prefixed `Vec`; on deserialize the decoder rejects any
//!   frame whose entries are not strictly ascending in canonical order
//!   (which also rejects duplicates), so each value has exactly one
//!   encoding.
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
//! The body's shape is not tagged on the wire; the receiver determines it
//! from the typed height (`Z` ⇒ leaf, `S<_>` ⇒ branch) together with the
//! running `prefix_len`. On the decode side, when `prefix_len > 0` the
//! decoder peels one head byte and recurses at the next-finer typed height,
//! synthesizing the `prefix_len − 1` byte for the inner reader via
//! [`borsh::io::Read::chain`], so the wire carries one `prefix_len` byte
//! per top-of-chain rather than one per typed level.
//!
//! Multi-child branches always carry at least two children; singletons
//! appear on the wire only as `prefix_len > 0` and reconstruct through
//! [`Node::beneath`](crate::tree::typed::Node::beneath). Branch radices
//! are required to be strictly ascending (matching the backing `OrdMap`'s
//! canonical iteration order).
//!
//! ## The three channels
//!
//! - **`providing`**: `Vec<(Prefix<_>, Node<T, _>)>` — the subtrees being
//!   provided, each paired with the prefix it lands at, in ascending prefix
//!   order. Each node carries its full structure on the wire (path-compression
//!   bytes, branch radices, child counts); the receiver inserts it directly at
//!   the named prefix. This trades the bandwidth of the elided-leaf encoding for
//!   placement without a per-leaf re-hash. Rejected unless strictly ascending by
//!   prefix.
//! - **`requested`**: `Vec<Prefix<_>>` — prefixes the peer should send next
//!   round. Rejected unless strictly ascending.
//! - **`uncertain`**: `Vec<(Prefix<_>, Hash)>` — frontier subtree hashes for the
//!   peer to compare against its own. Rejected unless strictly ascending by
//!   prefix.
//!
//! ## Messages
//!
//! Each of the five message types below is the borsh concatenation of its
//! fields in source order. There is no length framing between messages on
//! the wire: the protocol's height schedule names the type each side expects
//! next.

use borsh::{BorshDeserialize, BorshSerialize};

use crate::Version;
use crate::tree::typed::{
    Hash, Node, Prefix,
    height::{Height, Pred, Root, S, Z},
};

#[cfg(test)]
mod tests;

/// The `providing` channel's payload at height `H`: the subtrees being provided,
/// each paired with the prefix it lands at, in ascending prefix order. The
/// receiver inserts each node directly at its named prefix.
pub type Providing<T, H> = Vec<(Prefix<H>, Node<T, H>)>;

/// A peer's declared session intent, carried in its [`Handshake`] greeting.
///
/// This is strictly about the *party hand-off*: it tells the receiver whether
/// a trailing party frame will follow reconciliation. (Bootstrapping is the
/// other special intent, but it is signalled by the placeholder
/// [`Network::ZERO`](crate::Network), not here: a bootstrapper participates in
/// an ordinary session and *receives* a party, so it greets with [`Remain`].)
///
/// On the wire it is a borsh unit-enum: a single `u8` tag, `0x00` for
/// [`Remain`] and `0x01` for [`Retire`].
///
/// [`Remain`]: Intent::Remain
/// [`Retire`]: Intent::Retire
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum Intent {
    /// The sender stays in (or, bootstrapping, joins) the universe: no party
    /// will be handed over. Ordinary gossip and bootstrap greet with this.
    Remain,
    /// The sender is retiring: once reconciliation completes, it will ship its
    /// party as a single trailing frame for the receiver to absorb (see
    /// [`Peer::retire`](crate::Peer::retire)).
    Retire,
}

impl Intent {
    /// True iff this is a [`Retire`](Intent::Retire) greeting: the sender will
    /// hand its party over after reconciliation.
    pub fn retiring(self) -> bool {
        self == Intent::Retire
    }
}

/// The opening message of every session, exchanged by the `connect`/`accept`
/// steps. It carries the sender's causal [`Version`].
///
/// On the wire this frame follows the raw `magic + proto_version + network +
/// intent` preamble, which is validated before this body is ever parsed (see
/// [`super::remote`]), so the magic bytes are not part of this struct.
pub struct Handshake {
    /// The sender's latest causal [`Version`].
    pub version: Version,
}

impl BorshSerialize for Handshake {
    fn serialize<W: borsh::io::Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        self.version.serialize(writer)?;
        Ok(())
    }
}

impl BorshDeserialize for Handshake {
    fn deserialize_reader<R: borsh::io::Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let version = Version::deserialize_reader(reader)?;
        Ok(Self { version })
    }
}

/// The initiator's opening message: our root hash at the empty (root)
/// prefix.
///
/// Carries the same shape as [`Opening`]: an `uncertain` map at `Root`
/// height, with at most one entry (none when the initiator's tree is
/// empty). Distinct from `Opening` only by height,
/// and from [`Exchange`] by the absence of `providing`/`requested`, which
/// cannot be populated until at least one round has passed.
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
/// ([`open_initiator`](super::protocol::OpenInitiator::open_initiator)) a
/// separate entry point from the steady-state `exchange`, so the latter can
/// assume every uncertain hash describes a parent the receiver has already
/// acknowledged.
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
/// asymmetry-matrix table in the [`super::local`] module docs).
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
    /// On the wire each subtree travels as a whole `(prefix, node)` pair in
    /// ascending prefix order; the receiver inserts it directly at the named
    /// prefix. Strictly ascending by prefix; duplicates are rejected.
    pub providing: Providing<T, S<H>>,
    /// Prefixes the counterparty listed in the previous round's `uncertain`
    /// that we lack entirely. We ask them to send the subtrees so we can insert
    /// them into our zipper. Strictly ascending; duplicates are rejected.
    pub requested: Vec<Prefix<S<H>>>,
    /// Hashes of our subtrees at this round's frontier, for the counterparty
    /// to compare against their own. Each entry routes to one cell of the
    /// asymmetry matrix (see the [`super::local`] module docs) on the
    /// receiving side. Strictly ascending by prefix.
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
        let providing: Providing<T, S<H>> = BorshDeserialize::deserialize_reader(reader)?;
        verify_pairs_canonical(&providing, "Exchange.providing")?;
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
/// `S<Z>`, emitted by
/// [`close_initiator`](super::protocol::CloseInitiator::close_initiator).
///
/// Distinct from [`Exchange`] by the absence of `uncertain`: at leaf height,
/// any two parties either have a leaf at the same path (in which case the
/// leaf hashes match, both being the constant leaf hash) or one of them
/// lacks it (in which case the receiver routes the missing prefix to
/// `requested`, never `uncertain`). Encoding the vacuity in the type system
/// lets
/// [`complete_responder`](super::protocol::CompleteResponder::complete_responder)
/// consume `Closing` directly, without a runtime check against an
/// out-of-spec initiator.
#[derive(Clone)]
pub struct Closing<T> {
    pub providing: Providing<T, S<Z>>,
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
        let providing: Providing<T, S<Z>> = BorshDeserialize::deserialize_reader(reader)?;
        verify_pairs_canonical(&providing, "Closing.providing")?;
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
/// emitted by
/// [`complete_responder`](super::protocol::CompleteResponder::complete_responder)
/// for the initiator to absorb in
/// [`complete_initiator`](super::protocol::CompleteInitiator::complete_initiator).
///
/// No `requested` (the initiator never replies after this) and no `uncertain`
/// (vacuous at leaf height, same reasoning as [`Closing`]).
#[derive(Clone)]
pub struct Complete<T> {
    pub providing: Providing<T, Z>,
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
        let providing: Providing<T, Z> = BorshDeserialize::deserialize_reader(reader)?;
        verify_pairs_canonical(&providing, "Complete.providing")?;
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

/// An out-of-order or duplicated wire channel: the canonical encoding admits
/// exactly one byte sequence per value, so a peer that reorders or pads is
/// rejected before its content is acted on.
fn not_canonical(what: &'static str) -> borsh::io::Error {
    borsh::io::Error::new(
        borsh::io::ErrorKind::InvalidData,
        format!("{what} not in strictly ascending order"),
    )
}

/// Require key→value pairs to be in strictly ascending key order (rejecting
/// duplicate keys): the `uncertain` channel.
pub(crate) fn verify_pairs_canonical<K: Ord, V>(
    pairs: &[(K, V)],
    what: &'static str,
) -> borsh::io::Result<()> {
    if pairs.windows(2).any(|w| w[0].0 >= w[1].0) {
        return Err(not_canonical(what));
    }
    Ok(())
}

/// Require keys to be in strictly ascending order (rejecting duplicates): the
/// `requested` channel.
pub(crate) fn verify_keys_canonical<K: Ord>(
    keys: &[K],
    what: &'static str,
) -> borsh::io::Result<()> {
    if keys.windows(2).any(|w| w[0] >= w[1]) {
        return Err(not_canonical(what));
    }
    Ok(())
}
