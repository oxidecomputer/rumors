use super::typed;

/// An identifier for a unique item in the tree.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct Key(pub [u8; 32]);

impl From<[u8; 32]> for Key {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl From<Key> for [u8; 32] {
    fn from(id: Key) -> Self {
        id.0
    }
}

impl From<typed::Path> for Key {
    fn from(path: typed::Path) -> Self {
        <[u8; 32]>::from(path).into()
    }
}

impl From<Key> for typed::Path {
    fn from(id: Key) -> Self {
        typed::Path::from(id.0)
    }
}
