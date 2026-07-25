use std::fmt::Debug;

use borsh::{BorshDeserialize, BorshSerialize};

use super::typed;

/// An opaque key uniquely identifying a message.
///
/// A key is a unique pointer to the causal moment its message was
/// introduced by the party that sent it: it binds the send's version
/// (fresh on every send) to the message's content. Keys are therefore
/// unique across the entire history of the universe, byte-identical
/// re-sends are distinct messages under distinct keys, and the same
/// *send* has the same key on every replica. Keys come back out of the
/// observers and [`Snapshot`](crate::Snapshot) iteration; they go into
/// [`redact`](crate::Rumors::redact) and [`get`](crate::Snapshot::get). A
/// key is freely persistable as its raw 32 bytes
/// ([`as_bytes`](Self::as_bytes), [`From<[u8; 32]>`](Self#impl-From<[u8;+32]>-for-Key)).
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

/// The same lowercase hex as the [`Debug`] form.
impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&hex::encode(self.0))
    }
}

impl Key {
    /// The raw 32 bytes: the leaf's content-addressed path.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Reconstitutes a key from its raw bytes (for example, one persisted for a
/// later redaction).
///
/// A key that never named a live message is harmless: lookups miss and
/// redactions are no-ops.
impl From<[u8; 32]> for Key {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl From<Key> for [u8; 32] {
    fn from(key: Key) -> Self {
        key.0
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
