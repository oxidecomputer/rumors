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

pub enum Opening {
    /// The initiator's unconditional listing of its root's children.
    Uncertain(Vec<(u8, Hash)>),
}

/// One steady-state reaction about the subtree at the paired key.
pub enum Exchange<B: Backend<T, Node<Z>: Leaf<T>>, T, H: Height> {
    /// The subtree which the counterparty asked for or provably lacks.
    Providing(Prefix<H>, B::Node<H>),
    /// The sender matches the subtree: it need not be transferred, because the
    /// receiver already has it.
    Matched,
    /// The sender lacks the subtree: it asks the counterparty to provide.
    Requested,
    /// The sender disputes the subtree: its children's hashes, each child below
    /// the prefix, ascending by radix.
    Uncertain(Vec<(u8, Hash)>),
}

/// The initiator's closing reply: leaf-height words answering the
/// responder's leaf-parent verdicts.
///
/// This is [`Exchange`] at leaf height minus `Uncertain`, which is
/// structurally vacuous: leaves never dispute, because two parties holding
/// a leaf at one path hold the same leaf. `Matched` and `Requested` are
/// positional — each pairs, in order, with one leaf of the `uncertain`
/// listing the receiver spoke and still holds — while `Providing` carries
/// its prefix: it is the one word the receiver cannot anticipate, naming a
/// leaf only the sender holds.
pub enum Closing<B: Backend<T, Node<Z>: Leaf<T>>, T> {
    /// A leaf the responder provably lacks and has not deleted.
    Providing(Prefix<Z>, B::Node<Z>),
    /// The next listed leaf is shared: the responder keeps its copy.
    Matched,
    /// The sender lacks the next listed leaf: asks the responder to provide
    /// it in [`Complete`].
    ///
    /// The answer prunes against the sender's version first: a requested
    /// leaf causally at or before it was deleted by the sender, and drops on
    /// the responder's side instead of shipping.
    Requested,
}

/// The responder's final word: the leaves the initiator requested.
///
/// Keyed rather than positional even though each item answers a specific
/// `Requested`: answers prune against the requester's version, so this
/// stream is a subsequence of the questions, and position cannot carry the
/// pairing.
pub enum Complete<B: Backend<T, Node<Z>: Leaf<T>>, T> {
    Providing(Prefix<Z>, B::Node<Z>),
}
