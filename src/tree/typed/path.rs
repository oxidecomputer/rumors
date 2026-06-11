use std::{fmt::Debug, marker::PhantomData};

use bytes::Bytes;

use super::hash::{Hash, Hasher};
use super::height::{Height, Root, S};
use crate::version::Version;

/// A typed path through the tree which is always the right height.
///
/// The height marker is `PhantomData<fn() -> H>` rather than
/// `PhantomData<H>` for the same auto-trait reason as
/// [`super::node::Node`]: function pointers are unconditionally
/// `Send + Sync`, so the recursive `S<S<…>>` chain never opens up
/// during auto-trait dispatch on consumers.
#[repr(transparent)]
pub struct Path<H: Height = Root> {
    height: PhantomData<fn() -> H>,
    hash: [u8; 32],
}

impl Path<Root> {
    /// Get a path for the given leaf, incorporating its version and value.
    ///
    /// The version's canonical [`as_bytes`](Version::as_bytes) makes the path
    /// unique per insert: every [`tick`](Version::tick) yields a distinct
    /// canonical encoding, so two content-identical values inserted at
    /// different versions land at distinct paths. Parties descend from a
    /// shared seed by disjoint forks, so their versions are structurally
    /// distinct too; the version alone therefore disambiguates without also
    /// folding in the party.
    pub fn for_leaf(version: &Version, value: &Bytes) -> Self {
        // We form the hash for a value as the binary depth-1 merkle tree of
        // version, value. This ensures no length malleability issues.

        let mut hasher = Hasher::new();
        hasher.update(Hash::of(version.as_bytes()).as_bytes());
        hasher.update(Hash::of(value.as_ref()).as_bytes());

        Self {
            height: PhantomData,
            hash: hasher.finalize().into(),
        }
    }
}

impl<H: Height> Path<S<H>>
where
    S<H>: Height,
{
    /// Pop one hash byte off the path, yielding the byte and the remainder of
    /// the path.
    pub fn pop(self) -> (u8, Path<H>) {
        let byte = self.hash[32 - S::<H>::HEIGHT];
        (
            byte,
            Path {
                height: PhantomData,
                hash: self.hash,
            },
        )
    }
}

// Manual copy/clone impls so we don't require unnecessary bounds on `H`:

impl<H: Height> Copy for Path<H> {}

impl<H: Height> Clone for Path<H> {
    fn clone(&self) -> Self {
        *self
    }
}

// Comparison of paths refers only to the un-consumed portion, even though
// there's still stored hash (inaccessible) in the struct itself:

impl<H: Height> PartialEq for Path<H> {
    fn eq(&self, other: &Self) -> bool {
        self.hash[32 - H::HEIGHT..].eq(&other.hash[32 - H::HEIGHT..])
    }
}

impl<H: Height> PartialOrd for Path<H> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<H: Height> Ord for Path<H> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.hash[32 - H::HEIGHT..].cmp(&other.hash[32 - H::HEIGHT..])
    }
}

impl<H: Height> Eq for Path<H> {}

impl<H: Height> Debug for Path<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.hash.fmt(f)
    }
}

// We can convert any hash-sized array of bytes into a Path:

impl From<[u8; 32]> for Path<Root> {
    fn from(bytes: [u8; 32]) -> Self {
        Self {
            height: PhantomData,
            hash: bytes,
        }
    }
}

impl From<Path<Root>> for [u8; 32] {
    fn from(path: Path<Root>) -> Self {
        path.hash
    }
}

#[cfg(test)]
mod tests;
