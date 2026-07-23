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
//! (≈ 2 MB while an encoded and a decoded copy coexist), transient, at
//! most one in flight per stage.

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

/// The greeting exchanged after the fixed transport preamble: the sender's
/// causal [`Version`] plus its root-fan listing.
///
/// The listing is the same radix-keyed hash listing the initiator's opening
/// [`Reaction::Query`] carries — and that is the point: the opening
/// question's content depends only on the sender's own tree, so carrying it
/// here lets the elected responder answer immediately instead of waiting one
/// wire hop for a standalone opening frame. An empty tree carries an empty listing, which at the root means
/// exactly what an empty opening `Query` means: "I lack this node, send
/// everything."
///
/// Both sides carry a listing because neither knows at greeting time whether
/// it will win the initiator election; the elected responder consumes the
/// initiator's, and the responder's own listing is deliberately dead weight.
/// Likewise a converged session (equal versions) ends at the greeting and
/// consumes neither. That is the trade, made knowingly: the listing costs at
/// most one root fan of hashes (~4.3 KB, and ~nothing for an empty tree) on
/// a hop that exists anyway, versus saving a full one-way hop on every
/// divergent session. Divergence is not knowable at greeting time, so there
/// is nothing sound to gate the bytes on.
#[derive(Clone)]
pub struct Handshake {
    pub version: Version,
    /// The sender's live message count, exact (an O(1) read of its set).
    ///
    /// Both sides size their session window from the pair: dispute
    /// populations scale with the *product* of the two sizes (joint
    /// occupancy), so the window needs the peer's size, not an estimate.
    pub set_len: u64,
    /// The largest canonical version-bound encoding the sender's tree
    /// holds — leaf versions and every interior ceiling and floor — in
    /// bytes: exact, a read of a memoized per-node aggregate that
    /// redaction resizes down.
    ///
    /// Every bound a session holds is either a bound one replica already
    /// materializes (covered by that side's aggregate alone) or a
    /// join/meet of the two sides' surviving contributions, whose
    /// encoding never exceeds its inputs' combined, so the exchanged
    /// pair bounds worst-case version bytes per node — the second input
    /// a budget-configured window prices nodes with.
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
