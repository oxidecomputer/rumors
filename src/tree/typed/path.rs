use std::marker::PhantomData;

use super::height::{Height, Root, S};

/// A typed path through the tree which is always the right height.
pub struct Path<H: Height> {
    height: PhantomData<H>,
    hash: blake3::Hash,
}

impl Path<Root> {
    /// Get a path for the given bytes by taking their hash.
    pub fn for_bytes(bytes: &[u8]) -> Self {
        Self {
            height: PhantomData,
            hash: blake3::hash(bytes),
        }
    }

    /// Get a path for the supplied hash.
    pub fn for_hash(hash: blake3::Hash) -> Self {
        hash.into()
    }
}

impl<H: Height> Path<S<H>>
where
    S<H>: Height,
{
    /// Pop one hash byte off the path, yielding the byte and the remainder of
    /// the path.
    pub fn pop(self) -> (u8, Path<H>) {
        let byte = self.hash.as_bytes()[32 - S::<H>::HEIGHT];
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
        self.hash.as_bytes()[32 - H::HEIGHT..].eq(&other.hash.as_bytes()[32 - H::HEIGHT..])
    }
}

impl<H: Height> PartialOrd for Path<H> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.hash.as_bytes()[32 - H::HEIGHT..].partial_cmp(&other.hash.as_bytes()[32 - H::HEIGHT..])
    }
}

impl<H: Height> Ord for Path<H> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.hash.as_bytes()[32 - H::HEIGHT..].cmp(&other.hash.as_bytes()[32 - H::HEIGHT..])
    }
}

impl<H: Height> Eq for Path<H> {}

// We can convert any hash and any hash-sized array of bytes into a Path:

impl From<blake3::Hash> for Path<Root> {
    fn from(hash: blake3::Hash) -> Self {
        Self {
            height: PhantomData,
            hash,
        }
    }
}

impl From<[u8; 32]> for Path<Root> {
    fn from(bytes: [u8; 32]) -> Self {
        Self {
            height: PhantomData,
            hash: blake3::Hash::from(bytes),
        }
    }
}

#[cfg(test)]
mod test;
