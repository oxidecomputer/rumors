use std::marker::PhantomData;

use super::height::{Height, Root, S, Z};
use super::path::Path;

/// A typed path through the tree which is always the right height.
#[repr(transparent)]
pub struct Prefix<H: Height = Z> {
    height: PhantomData<H>,
    hash: Vec<u8>,
}

impl Prefix<Root> {
    /// Make a new empty prefix.
    pub fn new() -> Self {
        Prefix {
            height: PhantomData,
            hash: Vec::new(),
        }
    }
}

impl From<Prefix> for Path {
    fn from(value: Prefix) -> Self {
        let array: [u8; 32] = value.hash.try_into().expect("vector must be 32 bytes");
        array.into()
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
    pub fn pop(mut self) -> (u8, Prefix<S<H>>)
    where
        S<H>: Height,
    {
        let byte = self
            .hash
            .pop()
            .expect("internal vector cannot be non-empty");
        (
            byte,
            Prefix {
                height: PhantomData,
                hash: self.hash,
            },
        )
    }
}

// Manual clone impl so we don't require unnecessary bounds on `H`:

impl<H: Height> Clone for Prefix<H> {
    fn clone(&self) -> Self {
        Prefix {
            height: PhantomData,
            hash: self.hash.clone(),
        }
    }
}
