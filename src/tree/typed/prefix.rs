use std::{fmt::Debug, marker::PhantomData};

use tinyvec::ArrayVec;

use super::hash::PATH_LEN;
use super::height::{Height, Root, S, Z};
use super::path::Path;

/// The path bytes accumulated from the root down to height `H`.
///
/// Exactly `PATH_LEN - H::HEIGHT` bytes: the complement of a [`Path<H>`], which
/// holds the bytes still to be consumed below that height.
///
/// The function marker avoids recursive auto-trait bounds on `H`; see [`S`].
#[repr(transparent)]
pub struct Prefix<H: Height = Z> {
    /// Height reached by these bytes, recorded only in the type.
    height: PhantomData<fn() -> H>,
    /// Accumulated address bytes, shallowest first.
    hash: ArrayVec<[u8; PATH_LEN]>,
}

/// Accumulated path bytes without a type-level height tag.
///
/// The byte length records depth from the root; the remaining height is
/// `PATH_LEN - len`. [`Prefix::erase`] removes the tag, and [`Self::assume`]
/// restores it. This lets one runtime queue carry prefixes at any height.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ErasedPrefix {
    /// Accumulated bytes; their length determines the remaining height.
    hash: ArrayVec<[u8; PATH_LEN]>,
}

/// Navigate prefixes whose height is known at runtime.
impl ErasedPrefix {
    /// Re-tag this prefix at height `H`.
    ///
    /// `H` must match [`Self::height`]. A mismatch
    /// indicates an internal dispatch error: wire decoding obtains prefix
    /// lengths from the typed context before erasing them.
    ///
    /// # Panics
    ///
    /// If `H` differs from the prefix's remaining height.
    pub fn assume<H: Height>(self) -> Prefix<H> {
        assert_eq!(
            self.hash.len(),
            PATH_LEN - H::HEIGHT,
            "prefix length must match the requested height",
        );
        Prefix {
            height: PhantomData,
            hash: self.hash,
        }
    }

    /// Remaining height, equal to the `H::HEIGHT` of the erased [`Prefix<H>`].
    pub fn height(&self) -> usize {
        PATH_LEN - self.hash.len()
    }

    /// The accumulated path bytes, shallowest-first ([`Prefix::as_bytes`]).
    pub fn as_bytes(&self) -> &[u8] {
        &self.hash
    }

    /// Append one address byte and descend a level ([`Prefix::push`]).
    ///
    /// # Panics
    ///
    /// If the prefix is already at height zero (a full 32-byte path).
    pub fn push(mut self, byte: u8) -> ErasedPrefix {
        assert!(
            self.height() > 0,
            "a leaf-height prefix has no level to descend into",
        );
        self.hash.push(byte);
        self
    }

    /// Ascend one level, returning the parent prefix and removed byte.
    ///
    /// # Panics
    ///
    /// If the prefix is empty (the root has no parent).
    pub fn pop(mut self) -> (ErasedPrefix, u8) {
        let byte = self
            .hash
            .pop()
            .expect("a prefix below the root has at least one byte to pop");
        (self, byte)
    }
}

/// Display accumulated bytes in path order.
impl Debug for ErasedPrefix {
    /// Format the accumulated address bytes.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.hash.fmt(f)
    }
}

/// Start a prefix at the root, before any address bytes are consumed.
impl Prefix<Root> {
    /// Make a new empty prefix.
    pub fn new() -> Self {
        Prefix {
            height: PhantomData,
            hash: ArrayVec::new(),
        }
    }
}

/// Turn a full leaf prefix into a root-height path over the same address.
impl From<Prefix> for Path {
    /// Retain every address byte and reset traversal to the root.
    fn from(value: Prefix) -> Self {
        value.hash.into_inner().into()
    }
}

/// Recover all bytes from a height-zero prefix.
impl From<Prefix> for [u8; PATH_LEN] {
    /// A leaf prefix fills its backing array exactly.
    fn from(value: Prefix) -> Self {
        value.hash.into_inner()
    }
}

/// Wrap a full address as a height-zero prefix.
impl From<[u8; PATH_LEN]> for Prefix {
    /// Mark every supplied byte as part of the prefix.
    fn from(value: [u8; PATH_LEN]) -> Self {
        Self {
            height: PhantomData,
            hash: value.into(),
        }
    }
}

/// Treat an unconsumed full path as a completed leaf prefix.
impl From<Path> for Prefix {
    /// Keep the address and move its height tag from root to leaf.
    fn from(value: Path) -> Self {
        Self {
            height: PhantomData,
            hash: <[u8; PATH_LEN]>::from(value).into(),
        }
    }
}

/// Extend a prefix that has at least one level left below it.
impl<H: Height> Prefix<S<H>>
where
    S<H>: Height,
{
    /// Append one address byte and descend a level.
    pub fn push(mut self, byte: u8) -> Prefix<H> {
        self.hash.push(byte);
        Prefix {
            height: PhantomData,
            hash: self.hash,
        }
    }
}

/// Read a prefix, forget its height tag, or move to its parent.
impl<H: Height> Prefix<H> {
    /// Accumulated bytes in path order, of length `PATH_LEN - H::HEIGHT`.
    pub fn as_bytes(&self) -> &[u8] {
        &self.hash
    }

    /// Forget this prefix's height tag; [`ErasedPrefix::assume`] restores it.
    pub(crate) fn erase(self) -> ErasedPrefix {
        ErasedPrefix { hash: self.hash }
    }

    /// The first `PATH_LEN - H::HEIGHT` bytes of `path`, locating its subtree.
    pub fn containing(path: &Path) -> Self {
        Prefix {
            height: PhantomData,
            hash: ArrayVec::from_array_len((*path).into(), PATH_LEN - H::HEIGHT),
        }
    }

    /// Ascend one level, returning the parent prefix and removed byte.
    pub fn pop(mut self) -> (Prefix<S<H>>, u8)
    where
        S<H>: Height,
    {
        let byte = self
            .hash
            .pop()
            .expect("a prefix below the root has at least one byte to pop");
        (
            Prefix {
                height: PhantomData,
                hash: self.hash,
            },
            byte,
        )
    }
}

/// Copy the prefix without requiring a copyable height marker.
impl<H: Height> Copy for Prefix<H> {}

/// Clone by copying the prefix; no bound on `H` is needed.
impl<H: Height> Clone for Prefix<H> {
    /// Copy the accumulated bytes and their height tag.
    fn clone(&self) -> Self {
        *self
    }
}

/// Compare accumulated bytes, independent of the phantom marker.
impl<H: Height> PartialEq for Prefix<H> {
    /// Compare only bytes within each prefix's length.
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

/// Prefix equality is total.
impl<H: Height> Eq for Prefix<H> {}

/// Use the accumulated bytes' total ordering.
impl<H: Height> PartialOrd for Prefix<H> {
    /// Delegate to the lexicographic ordering.
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Sort prefixes lexicographically by their accumulated bytes.
impl<H: Height> Ord for Prefix<H> {
    /// Compare only bytes within each prefix's length.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.hash.cmp(&other.hash)
    }
}

/// Display accumulated bytes in path order.
impl<H: Height> Debug for Prefix<H> {
    /// Format the prefix's occupied bytes.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.hash.fmt(f)
    }
}
