use std::{fmt::Debug, marker::PhantomData};

use super::hash::PathHash;
use super::height::{Height, Root, S};
use crate::Version;

/// The remaining suffix of a leaf address, with `H` bytes left to traverse.
///
/// Popping a byte advances the height marker. Equality and ordering compare
/// only the remaining bytes, though the full address stays in memory.
#[repr(transparent)]
pub struct Path<H: Height = Root> {
    /// Remaining height; the function marker avoids `Send`/`Sync` bounds on `H`.
    height: PhantomData<fn() -> H>,
    /// Full leaf address, including bytes already consumed by traversal.
    hash: [u8; 32],
}

impl Path<Root> {
    /// Hash the leaf's canonical version bytes into its full 32-byte address.
    ///
    /// Each insert has a unique version: successive inserts tick their party,
    /// and concurrent parties are disjoint. Message contents do not affect
    /// the address.
    ///
    /// [`PathHash`] preserves full collision resistance because the path
    /// identifies the leaf. The shorter Merkle hash only compares subtrees.
    pub fn for_leaf(version: &Version) -> Self {
        #[cfg(test)]
        if let Some(path) = fixture::get(version) {
            return path;
        }

        Self {
            height: PhantomData,
            hash: PathHash::of(version.as_bytes()).into(),
        }
    }
}

impl<H: Height> Path<S<H>>
where
    S<H>: Height,
{
    /// Split off the next address byte, leaving a path one level shorter.
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

// Manual implementations avoid requiring `H: Copy + Clone`.

impl<H: Height> Copy for Path<H> {}

impl<H: Height> Clone for Path<H> {
    /// Copy the address and its height marker.
    fn clone(&self) -> Self {
        *self
    }
}

impl<H: Height> PartialEq for Path<H> {
    /// Compare only the unconsumed suffixes.
    fn eq(&self, other: &Self) -> bool {
        self.hash[32 - H::HEIGHT..].eq(&other.hash[32 - H::HEIGHT..])
    }
}

impl<H: Height> PartialOrd for Path<H> {
    /// Order the unconsumed suffixes lexicographically.
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<H: Height> Ord for Path<H> {
    /// Order the unconsumed suffixes lexicographically.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.hash[32 - H::HEIGHT..].cmp(&other.hash[32 - H::HEIGHT..])
    }
}

impl<H: Height> Eq for Path<H> {}

impl<H: Height> Debug for Path<H> {
    /// Show the full address, including its consumed prefix.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.hash.fmt(f)
    }
}

impl From<[u8; 32]> for Path<Root> {
    /// Use all the supplied bytes as a leaf address.
    fn from(bytes: [u8; 32]) -> Self {
        Self {
            height: PhantomData,
            hash: bytes,
        }
    }
}

impl From<Path<Root>> for [u8; 32] {
    /// Recover the full leaf address.
    fn from(path: Path<Root>) -> Self {
        path.hash
    }
}

#[cfg(test)]
mod fixture;
#[cfg(test)]
mod tests;
