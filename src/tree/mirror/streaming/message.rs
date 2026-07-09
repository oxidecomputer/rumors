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

/// One keyed steady-state wire item: the prefix of the subtree an
/// [`Exchange`] concerns, paired with it.
pub type Exchanged<B, T, H> = (Prefix<H>, Exchange<B, T, H>);

/// One steady-state reaction about the subtree at the paired key.
pub enum Exchange<B: Backend<T>, T, H: Height> {
    /// The subtree which the counterparty asked for or provably lacks.
    Providing(B::Node<H>),
    /// The sender lacks the subtree: it asks the counterparty to provide.
    Requested,
    /// The sender disputes the subtree: its children's hashes, each child below
    /// the prefix, ascending by radix.
    Uncertain(Vec<(u8, Hash)>),
}

/// The responder's leaf-parent-height reaction: [`Exchange`] minus `Uncertain`,
/// which is unused at leaf height.
pub enum Closing<B: Backend<T>, T> {
    Providing(B::Node<S<Z>>),
    Requested,
}

/// The initiator's final word: the leaves the responder requested.
pub enum Complete<B: Backend<T>, T> {
    Providing(B::Node<Z>),
}
