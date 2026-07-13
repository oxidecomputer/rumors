//! The streaming mirror's wire vocabulary.
//!
//! Every stream message rides as a `(Prefix<H>, kind)` pair: the prefix is
//! the stream's key — the protocol keeps each stream strictly
//! prefix-ascending, and the walk merge-joins it against the frontier
//! directly — so it lives in the pair, not the payload. `H` is the key's
//! height; a message about a subtree's *children* ([`Exchange::Uncertain`],
//! [`Opening::Uncertain`]) identifies each child by the single hash byte
//! below the key.

use crate::Version;
use crate::tree::mirror::streaming::Leaf;
use crate::tree::typed::{
    Hash, Prefix,
    height::{Height, S, Z},
};

use super::Backend;

// The initial handshake message:

pub enum Intent {
    Remain,
    Retire,
}

pub struct Handshake {
    pub version: Version,
}

// The four kinds of stream messages:

pub struct Initiate {
    /// The hashes of all the children of the initiator's root.
    uncertain: Vec<(u8, Hash)>,
}

pub struct Reply<B: Backend<T, Node<Z>: Leaf<T>>, T, H: Height> {
    /// The reactions to a single previous query.
    pub replies: Vec<Reaction<B, T, H>>,
}

/// Reactions are positionally keyed against the corresponding
/// [`Reaction::Uncertain`] query, with the exception of
/// [`Reaction::Providing`], which indicates its radix because it represents
/// information that the counterparty could not have known to ask about.
pub enum Reaction<B: Backend<T, Node<Z>: Leaf<T>>, T, H: Height> {
    /// Having inferred that the counterparty lacks this node through its
    /// absence in the counterparty's listing of hashes, we provide it, at this
    /// radix (the counterparty cannot infer the radix because only we know it
    /// exists in the first place).
    Providing(u8, B::Node<H>),
    /// Having inferred that we and the counterparty agree about this node, as
    /// its hash is the same on both sides, we indicate such.
    Matched,
    /// Having inferred that we ourselves lack this node through its presence in
    /// the counterparty's listing of hashes (and its absence in our own tree),
    /// we request it.
    Requested,
    /// Having inferred that we both have this node but disagree about its
    /// contents, we recur, informing the counterparty about the hashes of this
    /// node's children and implicitly requesting that they reply about each of
    /// those children (as well as providing any children which we didn't know
    /// to ask about).
    Uncertain(Vec<(u8, Hash)>),
}

pub struct Close<B: Backend<T, Node<Z>: Leaf<T>>, T> {
    /// The reactions to a single previous bottom-level query, which statically
    /// cannot be [`Reaction::Uncertain`], as content never differs at a full
    /// leaf path since leaves are content-addressed.
    pub replies: Vec<CloseReaction<B, T>>,
}

pub enum CloseReaction<B: Backend<T, Node<Z>: Leaf<T>>, T> {
    /// See [`Reaction::Providing`].
    Providing(u8, B::Node<Z>),
    /// See [`Reaction::Matched`].
    Matched,
    /// See [`Reaction::Requested`].
    Requested,
}

pub struct Complete<B: Backend<T, Node<Z>: Leaf<T>>, T> {
    /// A [`ClosingReaction::Requested`] can still occur during closing, meaning
    /// that the counterparty needs to provide the leaf; this is that final
    /// provision. It is optional because the counterparty may discover, during
    /// the course of providing the leaf, that the leaf ought to have been
    /// causally pruned.
    providing: Option<B::Node<Z>>,
}
