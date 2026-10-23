//! The storage boundary for canonical party trees.
//!
//! Party algorithms work with nodes, paths, and complete subtrees rather than
//! bit positions. Readers decode those objects without materializing a tree;
//! writers emit them while applying the canonical collapses. The
//! underlying bit stream is confined to this boundary.

#[cfg(test)]
use crate::bits::BitsWriter;
use crate::bits::{Bits, BitsReader};
use crate::error::Decode;
use crate::Party;

impl Party {
    /// Validates exactly one canonical Party encoding and adopts it as the
    /// party's storage.
    pub(crate) fn decode_bytes(bytes: Vec<u8>) -> Result<Self, Decode> {
        let end = validate::prefix(BitsReader::from_bytes(&bytes))?;
        Bits::validate_padding(&bytes, end)?;
        Ok(Self::from_canonical(Bits::from_canonical_vec(bytes)))
    }

    /// Validates the byte-aligned Party at the head of `bytes` and splits it
    /// off into storage of its own.
    ///
    /// Returns the party and the bytes after it, which start the next encoded
    /// field. The two share no allocation.
    pub(crate) fn decode_head(mut bytes: Vec<u8>) -> Result<(Self, Vec<u8>), Decode> {
        let end = validate::prefix(BitsReader::from_bytes(&bytes))?;
        let len = Bits::padded_len(&bytes, end)?;
        let rest = bytes.split_off(len);
        Ok((Self::from_canonical(Bits::from_canonical_vec(bytes)), rest))
    }

    /// Adopt a test-built stream without validating the Party invariants.
    #[cfg(test)]
    pub(crate) fn from_test_bits(bits: BitsWriter) -> Self {
        Party::from_canonical(bits.finalize())
    }

    /// Number of meaningful bits in the canonical ownership tree.
    pub(crate) fn stored_len(&self) -> u64 {
        self.0.reader().len()
    }
}

mod reader;
mod regions;
pub(crate) mod validate;
pub(crate) mod writer;

pub(crate) use reader::{
    BranchChoice, ForkPoint, PartyBranch, PartyNode, PartyPath, PartyReader, PartySnapshot,
    PartySubtree,
};
pub(crate) use regions::PartyRegionReader;
