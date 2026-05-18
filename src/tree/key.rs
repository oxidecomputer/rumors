use borsh::{BorshDeserialize, BorshSerialize};

use super::typed;

/// An identifier for a unique item in the tree.
#[derive(
    Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, BorshSerialize, BorshDeserialize,
)]
#[repr(transparent)]
pub struct Key(pub(crate) [u8; 32]);

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
