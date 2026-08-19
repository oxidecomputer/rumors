use std::{fmt::Debug, marker::PhantomData};

use tinyvec::ArrayVec;

#[cfg(any(test, feature = "protocol-v1"))]
use crate::tree::wire;

use super::height::{Height, Root, S, Z};
use super::path::Path;

/// The path bytes accumulated from the root down to height `H`.
///
/// Exactly `32 - H::HEIGHT` bytes: the complement of a [`Path<H>`], which
/// holds the bytes still to be consumed below that height.
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

/// A prefix with its height tag forgotten: the same accumulated path
/// bytes, whose length *is* the height (`32 - height` bytes at `height`).
///
/// The typed [`Prefix<H>`] wraps runtime bytes in a compile-time tag; this
/// is those bytes without the tag, for plumbing that carries prefixes of
/// every height through one instantiation. [`Prefix::erase`] forgets the
/// tag and [`ErasedPrefix::assume`] restores it, checking the length
/// against the claimed height in debug builds.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ErasedPrefix {
    hash: ArrayVec<[u8; 32]>,
}

impl ErasedPrefix {
    /// Re-tag this prefix at height `H`.
    ///
    /// `H` must be the height the prefix was erased at — equivalently,
    /// `32 - H::HEIGHT` must be its byte length, debug-asserted here. A
    /// cross-height re-tag is a programmer error in the erased plumbing,
    /// never a consequence of peer input: every wire prefix decodes
    /// through the typed reader, which fixes the length from the type.
    pub fn assume<H: Height>(self) -> Prefix<H> {
        debug_assert_eq!(
            self.hash.len(),
            32 - H::HEIGHT,
            "an erased prefix re-tags at the height it was erased at",
        );
        Prefix {
            height: PhantomData,
            hash: self.hash,
        }
    }

    /// The height this prefix sits at: its byte length's complement,
    /// exactly the `H::HEIGHT` of the [`Prefix<H>`] it erases.
    pub fn height(&self) -> usize {
        32 - self.hash.len()
    }

    /// The accumulated path bytes, shallowest-first ([`Prefix::as_bytes`]).
    pub fn as_bytes(&self) -> &[u8] {
        &self.hash
    }

    /// Push one hash byte onto the end of the prefix, descending one
    /// height ([`Prefix::push`]).
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
}

impl Debug for ErasedPrefix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.hash.fmt(f)
    }
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

impl From<Prefix> for Path {
    fn from(value: Prefix) -> Self {
        value.hash.into_inner().into()
    }
}

impl From<Prefix> for [u8; 32] {
    fn from(value: Prefix) -> Self {
        Path::from(value).into()
    }
}

impl From<[u8; 32]> for Prefix {
    fn from(value: [u8; 32]) -> Self {
        Self {
            height: PhantomData,
            hash: value.into(),
        }
    }
}

impl From<Path> for Prefix {
    fn from(value: Path) -> Self {
        Self {
            height: PhantomData,
            hash: <[u8; 32]>::from(value).into(),
        }
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
    /// this point reconstructs a full 32-byte path.
    pub fn as_bytes(&self) -> &[u8] {
        &self.hash
    }

    /// Forget this prefix's height tag; [`ErasedPrefix::assume`] restores it.
    pub(crate) fn erase(self) -> ErasedPrefix {
        ErasedPrefix { hash: self.hash }
    }

    /// The prefix naming the height-`H` subtree that contains `path`: its
    /// first `32 - H::HEIGHT` bytes.
    pub fn containing(path: &Path) -> Self {
        Prefix {
            height: PhantomData,
            hash: <[u8; 32]>::from(*path)[..32 - H::HEIGHT]
                .iter()
                .copied()
                .collect(),
        }
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
#[cfg(any(test, feature = "protocol-v1"))]
impl<H: Height> wire::Encode for Prefix<H> {
    fn write_wire<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
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

#[cfg(any(test, feature = "protocol-v1"))]
impl<H: Height> wire::Decode for Prefix<H> {
    fn read_wire<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
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
