use std::fmt::Debug;

use borsh::{BorshDeserialize, BorshSerialize};

use super::typed;

/// The borsh encoding is 32 raw bytes (no length prefix), matching the internal
/// content-address hash: a key *is* a leaf's content-addressed path, and the
/// mirror protocol's `providing` channel ships it alongside the
/// `(version, value)` so the receiver can place the leaf without re-hashing
/// during reassembly.
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct Key(pub(crate) [u8; 32]);

/// Hex-encodes the 32-byte key as a lowercase string, with no surrounding
/// punctuation. Convenient in logs and assertion messages.
impl Debug for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        hex::encode(self.0).fmt(f)
    }
}

impl From<typed::Path> for Key {
    fn from(path: typed::Path) -> Self {
        Self(<[u8; 32]>::from(path))
    }
}

impl From<Key> for typed::Path {
    fn from(id: Key) -> Self {
        typed::Path::from(id.0)
    }
}
