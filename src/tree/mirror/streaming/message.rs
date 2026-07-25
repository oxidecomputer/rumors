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
//! The memory unit is one reply: a maximally disputed reply is 256
//! reactions × a 256-entry listing ≈ fan² hashes ≈ 1.1 MB encoded
//! (≈ 2.2 MB while an encoded and a decoded copy coexist), transient, at
//! most one in flight per stage.

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
/// nothing sound to gate the bytes on.
#[derive(Clone)]
pub struct Greeting {
    pub version: Version,
    /// The sender's live message count, exact (an O(1) read of its set).
    ///
    /// Both sides size their session window from the pair: dispute
    /// populations scale with the *product* of the two sizes (joint
    /// occupancy), so the window needs the peer's size, not an estimate.
    /// The pair is also the role election's primary key ([`initiates`]):
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
    /// ([`DecodeError::OversizedVersion`](crate::tree::mirror::streaming::remote::DecodeError::OversizedVersion)).
    pub max_version_bytes: u64,
    /// The sender's supply-run byte target
    /// ([`Peer::target_message_size`](crate::Peer::target_message_size)).
    ///
    /// The session's encoders on both ends run at the **minimum** of the
    /// two exchanged targets: each side's setting bounds the frames it
    /// builds *and* the frames built for it, so the more
    /// memory-constrained end sets the pace.
    pub target_message_size: u64,
    /// The sender's root children as `(radix, hash)` pairs in strictly
    /// ascending radix order; empty when the sender's tree is empty.
    pub listing: Vec<(u8, Hash)>,
}

/// Elect roles from the exchanged greetings: does the side advertising
/// `(len, version)` initiate against a peer advertising `(peer_len,
/// peer_version)`?
///
/// The smaller exchanged set initiates. The initiator's opening act is a
/// pure question (its listing rides the greeting), while the elected
/// responder answers that question directly with whole-subtree supplies
/// for every root child the initiator lacks — so routing the bulk holder
/// into the responder role ships its exclusive content at the coarsest
/// granularity and on the earliest possible hop. Equal sizes fall back
/// to the canonical version encodings' lexicographic order (the greater
/// encoding initiates): causal versions are only partially ordered, and
/// the byte order is an arbitrary but total, deterministic tiebreak.
/// Both keys ride every greeting, so the two sides always elect
/// complementary roles from the same exchanged pair.
///
/// # Panics
///
/// If the versions are equal: a converged session short-circuits at the
/// greeting and never elects roles.
pub(crate) fn initiates(
    len: u64,
    version: &Version,
    peer_len: u64,
    peer_version: &Version,
) -> bool {
    match len.cmp(&peer_len) {
        Ordering::Less => true,
        Ordering::Greater => false,
        Ordering::Equal => match version.as_bytes().cmp(peer_version.as_bytes()) {
            Ordering::Greater => true,
            Ordering::Less => false,
            Ordering::Equal => unreachable!("equal versions do not elect roles"),
        },
    }
}

/// The sole stream message.
pub struct Reply<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static, H: Height> {
    /// The reactions to a single previous query.
    pub replies: Vec<Reaction<B, T, H>>,
}

/// Reactions are positionally keyed against the corresponding
/// [`Reaction::Query`] query.
///
/// The exception is [`Reaction::Supply`], which indicates its radix because
/// it represents information that the counterparty could not have known to
/// ask about.
pub enum Reaction<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static, H: Height> {
    /// Having inferred that the counterparty lacks this node through its
    /// absence in the counterparty's listing of hashes, we provide it, at
    /// this radix.
    ///
    /// The counterparty cannot infer the radix because only we know the node
    /// exists in the first place.
    Supply(u8, B::Node<H>),
    /// Having inferred that we and the counterparty agree about this node, as
    /// its hash is the same on both sides, we indicate such.
    Match,
    /// Having inferred that we both have this node but disagree about its
    /// contents (or that we lack the node entirely), we recur.
    ///
    /// The listing informs the counterparty of the hashes of this node's
    /// children, implicitly requesting that they reply about each of those
    /// children (as well as providing any children which we didn't know to
    /// ask about). An empty listing is the request for the whole node: an
    /// internal node always has at least one child, so emptiness is
    /// unambiguous; it can only mean we lack the node.
    Query(Vec<(u8, Hash)>),
}
