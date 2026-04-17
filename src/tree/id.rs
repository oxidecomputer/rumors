use super::typed;

/// An identifier for a unique item in the tree.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct Id(pub [u8; 32]);

impl From<[u8; 32]> for Id {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl From<Id> for [u8; 32] {
    fn from(id: Id) -> Self {
        id.0
    }
}

impl From<typed::Path> for Id {
    fn from(path: typed::Path) -> Self {
        <[u8; 32]>::from(path).into()
    }
}

impl From<Id> for typed::Path {
    fn from(id: Id) -> Self {
        typed::Path::from(id.0)
    }
}
