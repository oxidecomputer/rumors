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
//! reactions × a 256-entry listing ≈ fan² hashes ≈ 2 MB, transient, at
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

/// The version greeting exchanged after the fixed transport preamble.
pub struct Handshake {
    pub version: Version,
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
