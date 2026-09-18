//! The streaming mirror's wire vocabulary: one reply per question.
//!
//! After the handshake, every stream message is the complete reply to a
//! single earlier question. The first [`Reply`] asks the implicit root
//! question, and every subsequent `Reply` answers the k-th
//! question its receiver asked, in order. No message carries a prefix —
//! scope is determined by pairing against the receiver's own query queue —
//! and a reply is a finite value, so completeness is structural: having
//! read the k-th message, the receiver holds *everything* the counterparty
//! will ever say about the k-th scope. That structural completeness is
//! what lets the session resolve a scope the moment its reply arrives (see
//! [`materialized`](crate::tree::mirror::streaming::materialized) for the ordering argument).
//!
//! The memory unit is one reply: a maximally disputed reply carries one
//! full listing per root radix, or fan² hashes. An encoded reply and its
//! decoded skeleton may coexist transiently, with at most one decoded reply
//! in flight per stage; the adapter's occupancy tests pin that charge.

use std::cmp::Ordering;

use crate::{
    Version,
    tree::{
        mirror::streaming::{Backend, Leaf},
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
/// it will win the initiator election, and a divergent session consumes
/// both: the elected responder answers the initiator's listing, and the
/// initiator merges the responder's against its own fan to ship its
/// exclusive root children as the opening's early supplies. A converged
/// session (equal versions) ends at the greeting and consumes neither.
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
    /// Both sides size their session window from the pair: dispute
    /// populations scale with the *product* of the two sizes (joint
    /// occupancy), so the window needs the peer's size, not an estimate.
    /// The pair is also the role election's primary key
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
    /// exchanged pair bounds worst-case version bytes per node — the
    /// second input a budget-configured window prices nodes with. One
    /// priced residual: deletion-honoring can prune a side's
    /// contribution to a survivor subset whose recomputed bound neither
    /// input materialized; the pair sum there is an envelope, pinned by
    /// the census suite's reconciled-bound measurements.
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
    /// The sender's root children as `(radix, hash)` pairs in strictly
    /// ascending radix order; empty when the sender's tree is empty.
    pub listing: Vec<(u8, Hash)>,
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

/// The sole stream message.
pub struct Reply<B: Backend<Node<Z>: Leaf>, H: Height> {
    /// The reactions to a single previous query.
    pub reactions: Vec<Reaction<B, H>>,
}

/// Reactions are positionally keyed against the corresponding
/// [`Reaction::Query`] query.
///
/// The exception is [`Reaction::Supply`], which indicates its radix because
/// it represents information that the counterparty could not have known to
/// ask about.
pub enum Reaction<B: Backend<Node<Z>: Leaf>, H: Height> {
    /// The sender supplies a node absent from the counterparty's listing, at
    /// this radix.
    ///
    /// The radix is explicit because the counterparty did not know the node
    /// existed and therefore could not ask about it positionally.
    Supply(u8, B::Node<H>),
    /// Both sides hold the node with the same hash.
    Match,
    /// The sender asks to compare the node's children because the hashes
    /// differ, or asks for the whole node with an empty listing.
    ///
    /// The listing informs the counterparty of the hashes of this node's
    /// children, implicitly requesting that they reply about each of those
    /// children (as well as providing any children the sender did not know to
    /// ask about). An empty listing is the request for the whole node: an
    /// internal node always has at least one child, so emptiness is
    /// unambiguous; it can only mean the sender lacks the node.
    Query(Vec<(u8, Hash)>),
}
