use std::{fmt::Debug, marker::PhantomData};

use tinyvec::ArrayVec;

use crate::Key;

use super::height::{Height, Root, S, Z};
use super::path::Path;

/// A typed path through the tree which is always the right height.
#[repr(transparent)]
pub struct Prefix<H: Height = Z> {
    height: PhantomData<H>,
    hash: ArrayVec<[u8; 32]>,
}

impl Prefix<Root> {
    /// Make a new empty prefix.
    pub fn new() -> Self {
        Prefix {
            height: PhantomData,
            hash: ArrayVec::new(),
        }
    }
}

impl From<Prefix<Z>> for Path {
    fn from(value: Prefix) -> Self {
        value.hash.into_inner().into()
    }
}

impl From<Prefix<Z>> for Key {
    fn from(value: Prefix) -> Self {
        Path::from(value).into()
    }
}

impl<H: Height> Prefix<S<H>>
where
    S<H>: Height,
{
    /// Push one hash byte onto the end of the prefix.
    pub fn push(mut self, byte: u8) -> Prefix<H> {
        self.hash.push(byte);
        Prefix {
            height: PhantomData,
            hash: self.hash,
        }
    }
}

impl<H: Height> Prefix<H> {
    /// Pop one hash byte off the end of the prefix, yielding the byte and the
    /// remainder of the prefix.
    pub fn pop(mut self) -> (Prefix<S<H>>, u8)
    where
        S<H>: Height,
    {
        let byte = self
            .hash
            .pop()
            .expect("internal vector cannot be non-empty");
        (
            Prefix {
                height: PhantomData,
                hash: self.hash,
            },
            byte,
        )
    }
}

// Manual clone/comparison impls so we don't require unnecessary bounds on `H`.
// Comparison refers only to the accumulated path bytes; the phantom height is
// already pinned by the type.

impl<H: Height> Copy for Prefix<H> {}

impl<H: Height> Clone for Prefix<H> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<H: Height> PartialEq for Prefix<H> {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl<H: Height> Eq for Prefix<H> {}

impl<H: Height> PartialOrd for Prefix<H> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<H: Height> Ord for Prefix<H> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.hash.cmp(&other.hash)
    }
}

impl<H: Height> Debug for Prefix<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.hash.fmt(f)
    }
}
