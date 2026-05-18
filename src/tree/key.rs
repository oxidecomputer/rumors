use std::fmt::Debug;

use super::typed;

/// An opaque identifier for a message in a local rumor set.
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct Key(pub(crate) [u8; 32]);

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
