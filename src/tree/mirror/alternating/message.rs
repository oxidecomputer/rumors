//! # Wire format
//!
//! Each message is encoded by the tree's [`wire`]
//! codec: explicit structural framing whose variable-width atoms are
//! single CBOR values. Container lengths are `u32` little-endian.
//!
//! ## Atoms
//!
//! - [`typed::Hash`](crate::tree::typed::Hash): its raw bytes at the
//!   truncated Merkle width
//!   ([`MERKLE_HASH_LEN`](crate::tree::typed::hash::MERKLE_HASH_LEN)),
//!   no length prefix.
//! - [`typed::Prefix<H>`](crate::tree::typed::Prefix): exactly `32 −
//!   H::HEIGHT` raw bytes, no length prefix (the type pins the byte count).
//! - [`Version`] and [`Message`](crate::message::Message): one CBOR
//!   value each — a byte string wrapping the version's canonical encoding,
//!   and a byte string wrapping the message's cached CBOR payload —
//!   self-delimiting by CBOR's own length headers.
//! - `Vec<_>`: `u32` length followed by each element in order. Every channel
//!   is a length-prefixed `Vec`; on deserialize the decoder rejects any
//!   frame whose entries are not strictly ascending order (which also
//!   rejects duplicates).
//!
//! ## Typed [`Node<H>`](crate::tree::typed::Node)
//!
//! Encoded in its in-memory layout. The typed node's wire impl is a thin
//! delegate over the untyped node's `serialize_to`, which is the canonical
//! encoder:
//!
//! ```text
//! NodeWire ::=
//!     prefix_len: u8                  // path-compressed prefix byte count
//!     [u8; prefix_len]                // head bytes, shallowest first
//!     body                            // dispatched on `children`:
//!         Children::Leaf:   version: Version, message: Message
//!         Children::Branch: count_minus_two: u8, [(radix: u8, NodeWire); count]
//! ```
//!
//! The body's shape is not tagged on the wire; the receiver determines it
//! from the typed height (`Z` ⇒ leaf, `S<_>` ⇒ branch) together with the
//! running `prefix_len`. On the decode side, when `prefix_len > 0` the
//! decoder peels one head byte and recurses at the next-finer typed height,
//! synthesizing the `prefix_len − 1` byte for the inner reader via
//! [`std::io::Read::chain`], so the wire carries one `prefix_len` byte
//! per top-of-chain rather than one per typed level.
//!
//! Multi-child branches always carry at least two children; singletons
//! appear on the wire only as `prefix_len > 0` and reconstruct through
//! [`Node::beneath`](crate::tree::typed::Node::beneath). Branch radices
//! are required to be strictly ascending (matching the backing fan's
//! canonical iteration order).
//!
//! ## The three channels
//!
//! - **`providing`**: `Vec<(Prefix<_>, Node<_>)>` — the subtrees being
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
//! Each of the five message types below is the concatenation of its
//! fields in source order. There is no length framing between messages on
//! the wire: the protocol's height schedule names the type each side expects
//! next.

use crate::message::PayloadDeserializer;
use crate::tree::wire::{self, Decode, Encode};

/// Wire decode for the payload-bearing protocol messages: parses each
/// `providing` channel's leaf payloads through the peer's deserializer
/// (see [`read_providing`]); the payload-free messages stay on the plain
/// [`Decode`].
pub trait DecodeWith: Sized {
    fn read_wire_with<R: std::io::Read>(
        reader: &mut R,
        deserializer: PayloadDeserializer,
    ) -> std::io::Result<Self>;
}

use crate::Version;
use crate::tree::typed::{
    Hash, Node, Prefix,
    height::{Height, Root, S, UnderRoot, Z},
    node::DecodeNode,
};

#[cfg(test)]
mod tests;

/// The `providing` channel's payload at height `H`: the subtrees being provided,
/// each paired with the prefix it lands at, in ascending prefix order. The
/// receiver inserts each node directly at its named prefix.
pub type Providing<H> = Vec<(Prefix<H>, Node<H>)>;

/// Decode one `providing` channel: a `u32` count, then `(prefix, node)`
/// pairs, the nodes' leaf payloads parsed through the peer's
/// deserializer (the protocol's typed ingress; see [`DecodeNode`]).
fn read_providing<H, R>(
    reader: &mut R,
    deserializer: PayloadDeserializer,
) -> std::io::Result<Providing<H>>
where
    H: DecodeNode,
    R: std::io::Read,
{
    let count = u32::read_wire(reader)? as usize;
    // Grow as elements arrive rather than trusting the declared count for
    // the allocation (the same discipline as the framing reader).
    let mut items = Vec::new();
    for _ in 0..count {
        let prefix = Prefix::<H>::read_wire(reader)?;
        let node = H::read_node(reader, deserializer)?;
        items.push((prefix, node));
    }
    Ok(items)
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

impl Encode for Handshake {
    fn write_wire<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.version.write_wire(writer)
    }
}

impl Decode for Handshake {
    fn read_wire<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let version = Version::read_wire(reader)?;
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
#[derive(Clone, Default)]
pub struct Initiate {
    pub uncertain: Vec<(Prefix<Root>, Hash)>,
}

impl Encode for Initiate {
    fn write_wire<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.uncertain.write_wire(writer)
    }
}

impl Decode for Initiate {
    fn read_wire<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let uncertain = Vec::read_wire(reader)?;
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
#[derive(Clone, Default)]
pub struct Opening {
    pub uncertain: Vec<(Prefix<UnderRoot>, Hash)>,
}

impl Encode for Opening {
    fn write_wire<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.uncertain.write_wire(writer)
    }
}

impl Decode for Opening {
    fn read_wire<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let uncertain = Vec::read_wire(reader)?;
        verify_pairs_canonical(&uncertain, "Opening.uncertain")?;
        Ok(Self { uncertain })
    }
}

/// The steady-state message: carries all three channels (see the
/// asymmetry-matrix table in the [`super::local`] module docs).
#[derive(Clone)]
pub struct Exchange<H>
where
    S<H>: Height,
    H: Height,
{
    /// Subtrees the counterparty does not have.
    ///
    /// Populated from two sources: nodes they `requested` in the previous
    /// round, and nodes we unilaterally know they lack (because they did not
    /// list them in the previous round's `uncertain`).
    ///
    /// In both cases the subtrees are filtered against the counterparty's
    /// version vector: anything causally `<=` their version has either been
    /// already-seen or already-forgotten on their side, so the receiver's view
    /// must agree with ours by treating the absence as a deletion.
    ///
    /// On the wire each subtree travels as a whole `(prefix, node)` pair in
    /// ascending prefix order; the receiver inserts it directly at the named
    /// prefix. Strictly ascending by prefix; duplicates are rejected.
    pub providing: Providing<S<H>>,
    /// Prefixes the counterparty listed in the previous round's `uncertain`
    /// that we lack entirely. We ask them to send the subtrees so we can insert
    /// them into our zipper. Strictly ascending; duplicates are rejected.
    pub requested: Vec<Prefix<S<H>>>,
    /// Hashes of our subtrees at this round's frontier, for the counterparty
    /// to compare against their own.
    ///
    /// Each entry routes to one cell of the asymmetry matrix (see the
    /// [`super::local`] module docs) on the receiving side. Strictly ascending
    /// by prefix.
    pub uncertain: Vec<(Prefix<H>, Hash)>,
}

impl<H> Encode for Exchange<H>
where
    S<H>: Height,
    H: Height,
{
    fn write_wire<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.providing.write_wire(writer)?;
        self.requested.write_wire(writer)?;
        self.uncertain.write_wire(writer)
    }
}

impl<H> DecodeWith for Exchange<H>
where
    S<H>: DecodeNode,
    H: Height,
{
    fn read_wire_with<R: std::io::Read>(
        reader: &mut R,
        deserializer: PayloadDeserializer,
    ) -> std::io::Result<Self> {
        let providing: Providing<S<H>> = read_providing(reader, deserializer)?;
        verify_pairs_canonical(&providing, "Exchange.providing")?;
        let requested: Vec<Prefix<S<H>>> = Decode::read_wire(reader)?;
        verify_keys_canonical(&requested, "Exchange.requested")?;
        let uncertain: Vec<(Prefix<H>, Hash)> = Decode::read_wire(reader)?;
        verify_pairs_canonical(&uncertain, "Exchange.uncertain")?;
        Ok(Self {
            providing,
            requested,
            uncertain,
        })
    }
}

impl From<Opening> for Exchange<UnderRoot> {
    fn from(Opening { uncertain }: Opening) -> Self {
        Exchange {
            uncertain,
            ..Default::default()
        }
    }
}

impl<H> Default for Exchange<H>
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

/// The responder's closing message: a leaf-height `providing`/`requested`
/// pair.
///
/// Emitted by
/// [`close_responder`](super::protocol::CloseResponder::close_responder) in
/// answer to the initiator's final [`Exchange`], whose `uncertain` lists the
/// leaves under still-disputed leaf-parents.
///
/// Distinct from [`Exchange`] by the absence of `uncertain`: at leaf height
/// the dispute cell of the asymmetry matrix is empty — two parties holding a
/// leaf at the same path hold the same leaf, because leaves are
/// content-addressed and the path *is* the content commitment — so every
/// leaf routes to `providing`, `requested`, or silence,
/// never to a finer round. Encoding the vacuity in the type system lets
/// [`complete_initiator`](super::protocol::CompleteInitiator::complete_initiator)
/// consume `Closing` directly, without a runtime check against an
/// out-of-spec responder.
#[derive(Clone, Default)]
pub struct Closing {
    /// Leaves only the responder holds that the initiator has not deleted:
    /// answers to the initiator's final `requested`, plus leaves the
    /// initiator's `uncertain` listing proved it lacks.
    pub providing: Providing<Z>,
    /// Leaves the initiator listed under disputed parents that the responder
    /// lacks entirely.
    ///
    /// The initiator answers in [`Complete`], pruning first: a requested
    /// leaf at or before the responder's version was deleted there, and
    /// drops on both sides instead of shipping.
    pub requested: Vec<Prefix<Z>>,
}

impl Encode for Closing {
    fn write_wire<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.providing.write_wire(writer)?;
        self.requested.write_wire(writer)
    }
}

impl DecodeWith for Closing {
    fn read_wire_with<R: std::io::Read>(
        reader: &mut R,
        deserializer: PayloadDeserializer,
    ) -> std::io::Result<Self> {
        let providing: Providing<Z> = read_providing(reader, deserializer)?;
        verify_pairs_canonical(&providing, "Closing.providing")?;
        let requested: Vec<Prefix<Z>> = Decode::read_wire(reader)?;
        verify_keys_canonical(&requested, "Closing.requested")?;
        Ok(Self {
            providing,
            requested,
        })
    }
}

/// The initiator's terminal message: the final `providing` at leaf height,
/// answering the responder's closing `requested`.
///
/// Emitted by
/// [`complete_initiator`](super::protocol::CompleteInitiator::complete_initiator)
/// for the responder to absorb in
/// [`complete_responder`](super::protocol::CompleteResponder::complete_responder).
///
/// No `requested` (the responder never replies after this) and no `uncertain`
/// (vacuous at leaf height, same reasoning as [`Closing`]).
#[derive(Clone, Default)]
pub struct Complete {
    pub providing: Providing<Z>,
}

impl Encode for Complete {
    fn write_wire<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.providing.write_wire(writer)
    }
}

impl DecodeWith for Complete {
    fn read_wire_with<R: std::io::Read>(
        reader: &mut R,
        deserializer: PayloadDeserializer,
    ) -> std::io::Result<Self> {
        let providing: Providing<Z> = read_providing(reader, deserializer)?;
        verify_pairs_canonical(&providing, "Complete.providing")?;
        Ok(Self { providing })
    }
}

/// An out-of-order or duplicated wire channel: the canonical encoding admits
/// exactly one byte sequence per value, so a peer that reorders or pads is
/// rejected before its content is acted on.
fn not_canonical(what: &'static str) -> std::io::Error {
    wire::invalid(format!("{what} not in strictly ascending order"))
}

/// Require key→value pairs to be in strictly ascending key order (rejecting
/// duplicate keys): the `uncertain` channel.
pub(crate) fn verify_pairs_canonical<K: Ord, V>(
    pairs: &[(K, V)],
    what: &'static str,
) -> std::io::Result<()> {
    if pairs.windows(2).any(|w| w[0].0 >= w[1].0) {
        return Err(not_canonical(what));
    }
    Ok(())
}

/// Require keys to be in strictly ascending order (rejecting duplicates): the
/// `requested` channel.
pub(crate) fn verify_keys_canonical<K: Ord>(keys: &[K], what: &'static str) -> std::io::Result<()> {
    if keys.windows(2).any(|w| w[0] >= w[1]) {
        return Err(not_canonical(what));
    }
    Ok(())
}
