use std::{fmt::Debug, marker::PhantomData};

use super::hash::ContentHash;
use super::height::{Height, Root, S};
use crate::Version;

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
    /// Get the path for a leaf stamped with `version`: the full-width hash
    /// of the version's canonical bytes, and nothing else.
    ///
    /// Versions are unique per send — locally by [`tick`](Version::tick)
    /// (each tick changes the canonical [`as_bytes`](Version::as_bytes)),
    /// globally by party disjointness — an invariant the protocol already
    /// rests on everywhere, so version-derived identity adds no assumption.
    /// Message bytes enter no path and no digest: no actor can steer where
    /// anything lands by choosing content.
    ///
    /// The path is the full-width 32-byte `ContentHash`, never the
    /// truncated Merkle `Hash`: a path collision is permanent split-brain
    /// (see `ContentHash`). The preimage is one self-delimiting canonical
    /// byte string, so no concatenation ambiguity arises.
    pub fn for_leaf(version: &Version) -> Self {
        Self {
            height: PhantomData,
            hash: ContentHash::of(version.as_bytes()).into(),
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
