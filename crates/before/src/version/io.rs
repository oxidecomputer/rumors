//! The storage boundary for canonical version trees.
//!
//! A [`Version`] is a step function over party space, represented as a binary
//! tree of constant-height regions. Its canonical stream interleaves preorder
//! node tags with leaf payloads: the first leaf stores an absolute height and
//! each later leaf stores its signed change from the previous height.
//!
//! The IO boundary turns that stream into domain objects—nodes, regions,
//! payloads, and complete subtrees. Version algorithms use those objects, not
//! bit positions or raw bit buffers. This keeps the representation replaceable
//! and makes the algorithms read as tree operations.
//!
//! Canonical streams contain exactly one complete tree, never drive the running
//! height below zero, and never retain equal sibling leaves: such siblings
//! collapse into their parent. The writer performs those collapses as it emits
//! leaves. Validation proves the same invariants before decoded bytes become a
//! [`Version`]. Both walks are iterative and retain only compact path state.

use core::ops::Range;

#[cfg(test)]
use crate::bits::BitsWriter;
use crate::bits::{BitRead, Bits, BitsReader};
use crate::Version;
use num_bigint::BigUint;

impl Version {
    /// Validate and adopt exactly one canonical Version encoding.
    pub(crate) fn decode_bytes(bytes: bytes::Bytes) -> Result<Self, crate::error::Decode> {
        let end = validate::prefix(BitsReader::from_bytes(&bytes))?;
        Bits::validate_padding(&bytes, end)?;
        Ok(Self::from_canonical(Bits::from_canonical(bytes)))
    }

    /// Validate and adopt the first byte-aligned Version in `bytes`.
    ///
    /// The returned byte count starts the next encoded field. The Version
    /// shares its allocation with `bytes`.
    pub(crate) fn decode_prefix(
        bytes: &bytes::Bytes,
    ) -> Result<(Self, usize), crate::error::Decode> {
        let end = validate::prefix(BitsReader::from_bytes(bytes))?;
        let len = Bits::padded_len(bytes, end)?;
        Ok((
            Self::from_canonical(Bits::from_canonical(bytes.slice(..len))),
            len,
        ))
    }

    /// Adopt a test-built stream without validating the Version invariants.
    #[cfg(test)]
    pub(crate) fn from_test_bits(bits: BitsWriter) -> Self {
        Version::from_canonical(bits.finalize())
    }

    /// Number of meaningful bits in the representation.
    pub(crate) fn stored_len(&self) -> u64 {
        self.0.reader().len()
    }

    /// Whether both versions borrow the same storage.
    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }
}

/// A delimited range within one canonical Version representation.
///
/// Algorithms pass ranges rather than the underlying bit storage. Only the IO
/// implementation interprets or copies their bits.
#[derive(Clone)]
pub struct PayloadRange<'a> {
    /// Source Version.
    version: &'a Version,
    /// Half-open bit range within the source.
    range: Range<u64>,
}

impl<'a> PayloadRange<'a> {
    /// Delimit one payload within a Version.
    pub fn new(version: &'a Version, range: Range<u64>) -> Self {
        assert!(range.start <= range.end && range.end <= version.stored_len());
        Self { version, range }
    }

    /// Number of bits in the range.
    pub fn len(&self) -> u64 {
        self.range.end - self.range.start
    }

    /// Decode the range as exactly one gamma-coded payload.
    pub fn decode_payload(&self) -> BigUint {
        let mut reader = BitsReader::at(&self.version.0, self.range.start);
        let value = reader
            .read_gamma()
            .expect("a canonical Version payload is complete");
        debug_assert_eq!(
            reader.position(),
            self.range.end,
            "the range is exactly one payload"
        );
        value
    }

    /// Raw source and positions, available only inside the IO implementation.
    pub fn storage(self) -> (&'a Bits, Range<u64>) {
        (&self.version.0, self.range)
    }
}

pub mod regions;
pub mod splice;
pub mod tree;
pub mod validate;
pub mod writer;

#[cfg(test)]
mod tests;
