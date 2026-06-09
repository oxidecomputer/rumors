use std::{fmt::Debug, marker::PhantomData};

use borsh::{BorshDeserialize, BorshSerialize};
use tinyvec::ArrayVec;

use crate::tree::Key;

use super::height::{Height, Root, S, Z};
use super::path::Path;

/// The path bytes accumulated from the root down to height `H`: exactly
/// `32 - H::HEIGHT` of them. The complement of a [`Path<H>`], which holds
/// the bytes still to be consumed below that height.
///
/// `PhantomData<fn() -> H>` rather than `PhantomData<H>` so the
/// auto-trait check on `Prefix` does not recurse through the
/// `S<S<…>>` peano-style height chain; see
/// [`super::node::Node`] for the full rationale.
#[repr(transparent)]
pub struct Prefix<H: Height = Z> {
    height: PhantomData<fn() -> H>,
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
    /// The accumulated path bytes, shallowest-first. Exactly `32 - H::HEIGHT`
    /// long, so appending the remaining `H::HEIGHT` bytes of a descent below
    /// this point reconstructs a full 32-byte [`Key`].
    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.hash
    }

    /// Pop one hash byte off the end of the prefix, yielding the byte and the
    /// remainder of the prefix.
    pub fn pop(mut self) -> (Prefix<S<H>>, u8)
    where
        S<H>: Height,
    {
        let byte = self
            .hash
            .pop()
            .expect("a prefix above height Root has at least one byte to pop");
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

/// On the wire a `Prefix<H>` is exactly `32 - H::HEIGHT` raw bytes. The height
/// is pinned by the type, so no length prefix is transmitted: deserialization
/// reads exactly the byte count the type demands.
impl<H: Height> BorshSerialize for Prefix<H> {
    fn serialize<W: borsh::io::Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        let expected = 32 - H::HEIGHT;
        debug_assert_eq!(
            self.hash.len(),
            expected,
            "Prefix<{}> byte count does not match {}::HEIGHT",
            H::HEIGHT,
            H::HEIGHT,
        );
        writer.write_all(&self.hash)
    }
}

impl<H: Height> BorshDeserialize for Prefix<H> {
    fn deserialize_reader<R: borsh::io::Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let len = 32 - H::HEIGHT;
        let mut hash: ArrayVec<[u8; 32]> = ArrayVec::new();
        // Reserve `len` zero slots so we can read directly into the buffer.
        hash.set_len(len);
        reader.read_exact(&mut hash[..len])?;
        Ok(Prefix {
            height: PhantomData,
            hash,
        })
    }
}
