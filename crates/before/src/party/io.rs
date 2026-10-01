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
    /// Validate and adopt exactly one canonical Party encoding.
    pub(crate) fn decode_bytes(bytes: bytes::Bytes) -> Result<Self, Decode> {
        let end = validate::prefix(BitsReader::from_bytes(&bytes))?;
        Bits::validate_padding(&bytes, end)?;
        Ok(Self::from_canonical(Bits::from_canonical(bytes)))
    }

    /// Validate and adopt the first byte-aligned Party in `bytes`.
    ///
    /// The returned byte count starts the next encoded field. The Party shares
    /// its allocation with `bytes`.
    pub(crate) fn decode_prefix(bytes: &bytes::Bytes) -> Result<(Self, usize), Decode> {
        let end = validate::prefix(BitsReader::from_bytes(bytes))?;
        let encoded_bytes = (end + 1).div_ceil(8);
        if encoded_bytes > bytes.len() as u64 {
            return Err(Decode::Truncated);
        }
        let encoded_bytes =
            usize::try_from(encoded_bytes).expect("the Party prefix ends within the input buffer");
        Bits::validate_padding(&bytes[..encoded_bytes], end)?;
        Ok((
            Self::from_canonical(Bits::from_canonical(bytes.slice(..encoded_bytes))),
            encoded_bytes,
        ))
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
