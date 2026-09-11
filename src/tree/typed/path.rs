use std::{fmt::Debug, marker::PhantomData};

use super::hash::{PATH_LEN, PathHash};
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
    hash: [u8; PATH_LEN],
}

/// Derive full leaf addresses from versions.
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

/// Read the part of an address remaining below this height.
impl<H: Height> Path<H> {
    /// The unconsumed address bytes, in traversal order.
    fn as_bytes(&self) -> &[u8] {
        &self.hash[PATH_LEN - H::HEIGHT..]
    }
}

/// Descend one level of a nonempty path.
impl<H: Height> Path<S<H>>
where
    S<H>: Height,
{
    /// Split off the next address byte, leaving a path one level shorter.
    pub fn pop(self) -> (u8, Path<H>) {
        let byte = self.hash[PATH_LEN - S::<H>::HEIGHT];
        (
            byte,
            Path {
                height: PhantomData,
                hash: self.hash,
            },
        )
    }
}

/// Copy the address without requiring a copyable height marker.
impl<H: Height> Copy for Path<H> {}

/// Clone by copying the address; no bound on `H` is needed.
impl<H: Height> Clone for Path<H> {
    /// Copy the address and its height marker.
    fn clone(&self) -> Self {
        *self
    }
}

/// Equality concerns only the remaining suffix.
impl<H: Height> PartialEq for Path<H> {
    /// Compare only the unconsumed suffixes.
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

/// Use the remaining suffix's total ordering.
impl<H: Height> PartialOrd for Path<H> {
    /// Order the unconsumed suffixes lexicographically.
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Sort paths by their remaining bytes.
impl<H: Height> Ord for Path<H> {
    /// Order the unconsumed suffixes lexicographically.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_bytes().cmp(other.as_bytes())
    }
}

/// Suffix equality is total.
impl<H: Height> Eq for Path<H> {}

/// Display the same suffix that equality and ordering compare.
impl<H: Height> Debug for Path<H> {
    /// Show only the unconsumed bytes.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_bytes().fmt(f)
    }
}

/// Wrap a full address at root height.
impl From<[u8; PATH_LEN]> for Path<Root> {
    /// Use all the supplied bytes as a leaf address.
    fn from(bytes: [u8; PATH_LEN]) -> Self {
        Self {
            height: PhantomData,
            hash: bytes,
        }
    }
}

/// Recover the array from a full, unconsumed path.
impl From<Path<Root>> for [u8; PATH_LEN] {
    /// Recover the full leaf address.
    fn from(path: Path<Root>) -> Self {
        path.hash
    }
}

#[cfg(test)]
mod fixture;
#[cfg(test)]
mod tests;
