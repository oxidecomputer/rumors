//! The streaming mirror's wire vocabulary: one reply per question, in
//! chunks.
//!
//! After the handshake, every stream message is one chunk of the reply to
//! a single earlier question. The first [`Reply`] asks the implicit root
//! question, and every later reply answers the k-th question its receiver
//! asked, in order. No message carries a prefix — scope is determined by
//! pairing against the receiver's own query queue — and a reply's last
//! chunk is marked, so completeness is structural: having read the k-th
//! reply's last chunk, the receiver holds *everything* the counterparty
//! will ever say about the k-th scope. That structural completeness is what
//! lets the session resolve a scope the moment its reply ends (see
//! [`materialized`](crate::tree::mirror::streaming::materialized) for the
//! ordering argument).
//!
//! The receiver acts on each chunk as it arrives, answering the questions a
//! reply asks before the rest of the reply arrives. Decoded chunks park
//! until their stage takes them, within the room the question they answer
//! set aside for its reply.

use std::cmp::Ordering;

use crate::{
    Version,
    tree::{
        mirror::streaming::{Backend, ErasedNode, Leaf, byte_set::ByteSet, ledger::Charge},
        typed::{
            Hash,
            height::{Height, Z},
        },
    },
};

/// The greeting exchanged after the fixed transport preamble.
///
/// Three terms, three things: the *preamble* is the fixed transport bytes
/// that precede any message, the *handshake* is the act of exchanging
/// greetings, and a `Greeting` is the message each side contributes to it.
///
/// It carries everything a session must know about a sender before the
/// descent — its causal position, its negotiation inputs, and its opening
/// question's content; the fields below are the inventory.
///
/// The listing is the same radix-keyed hash listing the initiator's opening
/// [`Reaction::Query`] carries — and that is the point: the opening
/// question's content depends only on the sender's own tree, so carrying it
/// here lets the elected responder answer immediately instead of waiting one
/// wire hop for a standalone opening frame. An empty tree carries an empty
/// listing, which at the root means exactly what an empty opening `Query`
/// means: "I lack this node, send everything."
///
/// Both sides carry a listing because neither knows at greeting time whether
/// it will win the initiator election. A divergent session consumes the
/// initiator's, which the elected responder answers; a converged session
/// (equal versions) ends at the greeting and consumes neither.
/// That is the trade, made knowingly: the listing costs at most one root
/// fan of hashes (~4.3 KB, and ~nothing for an empty tree) on a hop that
/// exists anyway, versus saving a full one-way hop on every divergent
/// session. Divergence is not knowable at greeting time, so there is
/// nothing to gate the bytes on.
#[derive(Clone)]
pub struct Greeting {
    /// The sender's set version; equal versions end the session at the greeting.
    pub version: Version,
    /// The sender's live message count, exact (an O(1) read of its set).
    ///
    /// The receiver holds the sender's supplies to it: a conforming peer
    /// supplies each leaf at most once and only leaves its set holds, so
    /// the count bounds the records it can supply. The pair is also the
    /// role election's primary key
    /// ([`RoleKey::initiates`]):
    /// the smaller set initiates, so the bulk holder lands in the
    /// responder role, whose exclusive content ships as whole-subtree
    /// supplies.
    pub set_len: u64,
    /// The largest canonical version-bound encoding the sender's tree
    /// holds — leaf versions and every interior ceiling and floor — in
    /// bytes: exact, a read of a memoized per-node aggregate that
    /// redaction resizes down.
    ///
    /// Every bound a session holds is either a bound one replica already
    /// materializes (covered by that side's aggregate alone) or a
    /// join/meet of the two sides' contributions, whose encoding never
    /// exceeds its inputs' combined (the pinned pairwise lemmas), so the
    /// exchanged pair bounds worst-case version bytes per node.
    ///
    /// The declaration is enforced at ingress: every version the sender
    /// supplies is one its tree materializes, so it must encode within
    /// this bound, and a session that receives one over it fails with a
    /// typed violation
    /// (a protocol violation if exceeded).
    pub max_version_bytes: u64,
    /// The sender's supply-run byte target
    /// ([`Peer::target_message_size`](crate::Peer::target_message_size)).
    ///
    /// The session's encoders on both ends run at the **minimum** of the
    /// two exchanged targets: each side's setting bounds the frames it
    /// builds *and* the frames built for it, so the more
    /// memory-constrained end sets the pace.
    pub target_message_size: u64,
    /// The sender's configured payload nesting-depth limit
    /// ([`Peer::payload_depth_limit`](crate::Peer::payload_depth_limit)),
    /// in decode recursion steps.
    ///
    /// A session proceeds only if the two exchanged values are equal:
    /// the limit is a property of the shared set (every replica must be
    /// able to hold and forward all content), so a mismatch in either
    /// direction is a typed, unconditional abort at the handshake, before
    /// the equal-versions resolution. On the wire the value is authored
    /// by the proxy from the session's payload codec at send; an
    /// in-process participant carries the default.
    pub payload_depth_limit: u64,
    /// The sender's root children in strictly ascending radix order, each
    /// with its grandchild count; empty when the sender's tree is empty.
    pub listing: Vec<Listed>,
    /// The sender's grandchild count of its root: over the listed root
    /// children, each one's own child count, a child at the leaves counting
    /// one; zero for an empty tree.
    ///
    /// Otherwise it lies in 1..=65,536. A sender whose backend reports a count
    /// outside that range fails the session at its greeting
    /// ([`CountError::Grandchildren`]), and a receiver rejects one at decode.
    ///
    /// It bounds the elected responder's reply to the opening question, which
    /// lists the responder's children of its root children. The initiator sizes
    /// its memory for that reply from the responder's greeting, as it sizes
    /// every later reply from a listing's counts.
    pub grandchildren: u64,
}

/// One child a listing names: a question's listing of the asker's children
/// of the node it asks about, or a greeting's listing of the sender's root
/// children.
///
/// A listing names its children in strictly ascending radix order, so a
/// replier compares it with its own children in one merge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Listed {
    /// The child's radix under the listed node.
    pub radix: u8,
    /// The child's subtree hash.
    pub hash: Hash,
    /// The lister's grandchild count of the child.
    ///
    /// It bounds the reply the lister will give to a question about the
    /// child. A peer that asks one sizes its memory for that reply from this
    /// count before asking, and a reply over it fails the session.
    pub grandchildren: Grandchildren,
    /// This side's child count of the child, less one, when this side
    /// lists its own child; zero in a listing this side decoded.
    ///
    /// The wire does not carry it. It is carried by the entry's padding from
    /// the walk, which lists its own children, into the record of the question
    /// the listing asks ([`Scope`](super::remote::Scope)), where the decoder
    /// reads it when the peer matches the child: a matched child is identical
    /// on both sides, so the peer's count of it, which the peer's grandchild
    /// count of the scope includes, is this one, and that many of the reply's
    /// listing entries can never arrive. Wherever a reply can carry listings,
    /// each child a question lists has a child of its own, so the count less
    /// one fits a byte.
    pub(crate) children_less_one: u8,
}

/// The child count riding in a listing entry costs no byte: every listing
/// is priced per entry at its size.
const _: () = assert!(
    size_of::<Listed>() == size_of::<(u8, Hash, Grandchildren)>(),
    "a listing entry's child count must fit its padding",
);

impl Listed {
    /// The entry for `node`, listed by this side at `radix`.
    ///
    /// # Errors
    ///
    /// Returns a [`CountError`] if the backend reports more children or
    /// grandchildren for `node` than any node can have, which no listing
    /// can carry.
    pub(crate) fn of<E: ErasedNode>(radix: u8, node: &E) -> Result<Self, CountError> {
        let children = node.child_count();
        let children_less_one =
            u8::try_from(children.saturating_sub(1)).map_err(|_| CountError::Children(children))?;
        Ok(Self {
            radix,
            hash: node.hash(),
            grandchildren: Grandchildren::of(node)?,
            children_less_one,
        })
    }
}

/// A count the backend reported for a node that no node can have.
///
/// A node has at most a fan of 256 children, and its children at most a fan
/// of fans, 65,536, among them. The in-memory backend keeps both counts by
/// construction, but a backend that loads nodes from storage reads them
/// there, so storage corrupted at rest can report a count outside those
/// ranges. A listing cannot carry such a count, and every price and bound
/// the session derives from one would be wrong, so the session fails
/// instead of listing the node.
///
/// The bytes a backend reports exploding a node allocates
/// ([`ErasedNode::explosion_bytes`]) are bounded the same way, by the
/// backend's own [`ErasedNode::MAX_EXPLOSION_BYTES`]: the session bounds
/// every grant it asks its ledger for by that bound, and a ledger charge
/// counts a grant in four bytes. A report above it breaks the contract as
/// a corrupted count does, so the session fails instead of exploding the
/// node.
///
/// A backend must also list exactly the children it reports: the session
/// lists a node's children into a vector sized by its child count, and a
/// question is priced at that count, so a listing that differs from the
/// report fails the session as well, before the vector can grow past it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CountError {
    /// The node's child count exceeded a fan.
    #[error("the backend reported {0} children for a node, more than a fan of 256")]
    Children(usize),
    /// The node's grandchild count exceeded a fan of fans.
    #[error("the backend reported {0} grandchildren for a node, more than 65,536")]
    Grandchildren(usize),
    /// The backend listed a different number of children for a node than
    /// it reported ([`ErasedNode::child_count`]).
    #[error("the backend reported {reported} children for a node but listed {listed}")]
    ChildrenListed {
        /// The children reported.
        reported: usize,
        /// The children listed, counted up to the first beyond the report,
        /// where listing stops.
        listed: usize,
    },
    /// The node's explosion report exceeded the backend's bound.
    #[error("the backend reported {reported} B to explode a node, more than its bound of {max} B")]
    ExplosionBytes {
        /// The bytes reported.
        reported: usize,
        /// The backend's bound, its `MAX_EXPLOSION_BYTES`.
        max: usize,
    },
}

/// A grandchild count as a listing carries it: how many children a listed
/// child's children have.
///
/// The lister counts as its backend lists them, a child at the leaves
/// counting one ([`ErasedNode::grandchild_count`]). A listed child that is itself a leaf counts one as well. Nothing is ever
/// asked about a leaf but the leaf itself, whose reply supplies at most one
/// node, so one is that reply's bound too. Every count therefore lies in
/// 1..=65,536: at most 256 children of at most 256 children each. The wire
/// spells it as the count less one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Grandchildren(u16);

impl Grandchildren {
    /// The smallest count: a listed leaf, or a child whose one child is a
    /// leaf or itself has one child.
    pub const ONE: Self = Self(0);

    /// The largest count: a full fan of children, each with a full fan.
    pub const MAX: Self = Self(u16::MAX);

    /// The count of `node`'s grandchildren, a leaf counting one.
    ///
    /// # Errors
    ///
    /// Returns [`CountError::Grandchildren`] if the backend reports more
    /// than 65,536, more than any node's children can have.
    pub fn of(node: &impl ErasedNode) -> Result<Self, CountError> {
        Self::counted(node.grandchild_count())
    }

    /// The grandchild count a backend reported for a node, `count`, a leaf
    /// counting one: the check [`of`](Self::of) applies, for counts read
    /// through a node's typed interface, such as the greeting's root.
    ///
    /// # Errors
    ///
    /// Returns [`CountError::Grandchildren`] if `count` exceeds 65,536.
    pub fn counted(count: usize) -> Result<Self, CountError> {
        Self::new(count.max(1)).ok_or(CountError::Grandchildren(count))
    }

    /// The count `count`, or `None` outside 1..=65,536.
    pub fn new(count: usize) -> Option<Self> {
        let less_one = count.checked_sub(1)?;
        u16::try_from(less_one).ok().map(Self)
    }

    /// The count spelled on the wire: the count less one.
    pub const fn from_wire(less_one: u16) -> Self {
        Self(less_one)
    }

    /// The count's wire spelling: the count less one.
    pub const fn wire(self) -> u16 {
        self.0
    }

    /// The count itself.
    pub const fn get(self) -> usize {
        // Widening a `u16`, which `usize::from` does but constants cannot
        // call.
        self.0 as usize + 1
    }
}

/// A listing entry for the child at `radix` with subtree hash `hash` and
/// the smallest grandchild count.
#[cfg(any(test, feature = "test-internals"))]
pub(crate) fn listed(radix: u8, hash: Hash) -> Listed {
    Listed {
        radix,
        hash,
        grandchildren: Grandchildren::ONE,
        children_less_one: 0,
    }
}

/// The greeting fields used to elect the session initiator.
#[derive(Clone)]
pub(crate) struct RoleKey {
    /// The sender's live message count, the primary election key.
    set_len: u64,
    /// The sender's causal version, the deterministic tiebreaker.
    version: Version,
}

impl RoleKey {
    /// Construct a role key from the values a greeting advertises.
    pub(crate) fn new(set_len: u64, version: Version) -> Self {
        Self { set_len, version }
    }

    /// Return the advertised version.
    pub(crate) fn version(&self) -> &Version {
        &self.version
    }

    /// Decide whether this side initiates against `peer`.
    ///
    /// The smaller exchanged set initiates. The initiator's opening act is a
    /// pure question (its listing rides the greeting), while the elected
    /// responder answers that question directly with whole-subtree supplies
    /// for every root child the initiator lacks. This puts the bulk holder in
    /// the responder role, where its exclusive content can move at the
    /// coarsest granularity and on the earliest possible hop.
    ///
    /// Equal sizes fall back to the canonical version encodings' lexicographic
    /// order; the greater encoding initiates. Causal versions are only
    /// partially ordered, so the byte order supplies an arbitrary but total,
    /// deterministic tiebreaker.
    ///
    /// # Panics
    ///
    /// If the versions are equal. A converged session ends during the greeting
    /// exchange and never elects roles.
    pub(crate) fn initiates(&self, peer: &Self) -> bool {
        match self.set_len.cmp(&peer.set_len) {
            Ordering::Less => true,
            Ordering::Greater => false,
            Ordering::Equal => match self.version.as_bytes().cmp(peer.version.as_bytes()) {
                Ordering::Greater => true,
                Ordering::Less => false,
                Ordering::Equal => unreachable!("equal versions do not elect roles"),
            },
        }
    }
}

impl Greeting {
    /// Copy the two fields used for role election.
    pub(crate) fn role_key(&self) -> RoleKey {
        RoleKey::new(self.set_len, self.version.clone())
    }
}

/// The sole stream message: one chunk of the reply to a single previous
/// query.
///
/// A reply is a run of chunks, its reactions in prefix order, ending at the
/// first chunk marked [`last`](Self::last). Its receiver acts on each chunk
/// as it arrives, so a sender may pause between chunks until the receiver
/// has answered the questions sent so far. On the wire, chunk boundaries
/// carry no frame of their own: the receiver's decoder chunks a reply as
/// its frames arrive, and the reply's last reaction carries the end.
pub struct Reply<B: Backend<Node<Z>: Leaf>, H: Height> {
    /// The chunk's reactions, continuing the reply's earlier chunks.
    pub reactions: Vec<Reaction<B, H>>,
    /// What the chunk holds, charged to the endpoint that built it and
    /// returned when its consumer has used it.
    pub(crate) charge: Charge,
    /// Whether this chunk ends the reply.
    pub(crate) last: bool,
    /// What the chunk carries for its consumer beyond its own items.
    pub(crate) carried: Carried,
}

/// What an outgoing chunk carries for its encoder beyond its own items,
/// charged to the endpoint that built the chunk.
///
/// A chunk ends at a question only partway through an answer that explores
/// its scope, and only a reply that answers a whole scope with supplies
/// alone copies nodes for them, so no chunk carries both. A chunk this side
/// decoded carries nothing.
#[derive(Debug, Default)]
pub(crate) enum Carried {
    /// Nothing beyond the chunk's own items.
    #[default]
    Nothing,
    /// The node copies the chunk's supplies made, which return with the
    /// chunk once its encoder has used it.
    Copies(#[expect(dead_code, reason = "held for its bytes, which return when it drops")] Charge),
    /// What the question the chunk ends at sets aside for its reply.
    Question {
        /// The charge for the question's record, which the encoder
        /// publishes the record with.
        record: Charge,
        /// The room the reply's chunks take while they park, which
        /// travels with the record to the decoder: it charges each chunk
        /// its part as the chunk decodes and returns the rest once the
        /// reply is decoded.
        room: Charge,
    },
}

impl Carried {
    /// Take out the record's charge and the reply's room of the question
    /// the chunk ends at, if it ends at one, leaving any node copies to
    /// return with the chunk.
    pub(crate) fn take_question(&mut self) -> Option<(Charge, Charge)> {
        match std::mem::take(self) {
            Self::Question { record, room } => Some((record, room)),
            other => {
                *self = other;
                None
            }
        }
    }
}

/// The charges a chunk carries take two charges' room, what a question's
/// carry needs.
///
/// Every chunk is priced at its size, so a wider variant would raise the
/// price of every chunk, carrying or not.
const _: () = assert!(
    size_of::<Carried>() == 2 * size_of::<Charge>(),
    "a chunk's carried charges must fit in a question's two charges",
);

/// Reactions are positionally keyed against the corresponding
/// [`Reaction::Query`] query.
///
/// The exception is [`Reaction::Supply`], which indicates its radix because
/// it represents information that the counterparty could not have known to
/// ask about.
///
/// A reply carries at most one [`Reaction::Match`]: its first, which names
/// every later listed child that also matches ([`LaterMatches`]). Those
/// children take no reaction of their own; a receiver pairing reactions
/// with its listing passes over them as matched, so the next positional
/// reaction pairs with the next listed child the match did not cover.
pub enum Reaction<B: Backend<Node<Z>: Leaf>, H: Height> {
    /// The sender supplies a node absent from the counterparty's listing, at
    /// this radix.
    ///
    /// The radix is explicit because the counterparty did not know the node
    /// existed and therefore could not ask about it positionally.
    Supply(u8, B::Node<H>),
    /// Both sides hold the node with the same hash, and so they do every
    /// later listed child the reaction covers.
    Match(LaterMatches),
    /// The sender asks to compare the node's children because the hashes
    /// differ, or asks for the whole node with an empty listing.
    ///
    /// The listing informs the counterparty of the hashes of this node's
    /// children, implicitly requesting that they reply about each of those
    /// children (as well as providing any children the sender did not know to
    /// ask about). An empty listing is the request for the whole node: an
    /// internal node always has at least one child, so emptiness is
    /// unambiguous; it can only mean the sender lacks the node.
    Query(Vec<Listed>),
}

/// The later listed children a [`Reaction::Match`] covers: every child the
/// reply's question listed after the match's own position that also
/// matches, by radix.
///
/// Absent when no later child matches, so a reply with one match carries
/// nothing beyond it, and present only when nonempty. The set is boxed: a
/// reaction is a queue element, priced at its size wherever it waits, and
/// an inline set would raise the price of every reaction, the queries and
/// supplies that make up a divergent session's replies included. A box
/// takes the room of the pointer alone, within the room a reaction already
/// has, and is allocated only by a reply with two matches or more, which
/// it saves a reaction for each match after the first. Every place that
/// prices a reaction prices the box as well ([`heap_bytes`](Self::heap_bytes)).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LaterMatches(Option<Box<ByteSet>>);

impl LaterMatches {
    /// No later matches: the match covers no other child.
    pub const NONE: Self = Self(None);

    /// The later matches `set` names; none when it is empty.
    pub fn new(set: ByteSet) -> Self {
        Self((!set.is_empty()).then(|| Box::new(set)))
    }

    /// The radixes covered, or `None` when there are none.
    pub fn get(&self) -> Option<&ByteSet> {
        self.0.as_deref()
    }

    /// The radixes covered, as a set, empty when there are none.
    pub fn set(&self) -> ByteSet {
        self.get().copied().unwrap_or_default()
    }

    /// The bytes the covered set holds on the heap beyond the reaction
    /// carrying it: a set's, when there is one, and none otherwise.
    pub const fn heap_bytes(&self) -> usize {
        match self.0 {
            Some(_) => LATER_MATCHES_HEAP_BYTES,
            None => 0,
        }
    }
}

/// The bytes one present [`LaterMatches`] holds on the heap: one set.
pub(crate) const LATER_MATCHES_HEAP_BYTES: usize = size_of::<ByteSet>();

/// A covered set rides in the room of one pointer, so carrying it raises
/// no reaction's price.
const _: () = assert!(
    size_of::<LaterMatches>() == size_of::<usize>(),
    "the later matches must take one pointer's room",
);

#[cfg(test)]
mod tests;
