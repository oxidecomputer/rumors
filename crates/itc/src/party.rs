//! [`Party`] — a nonzero share of the interval-tree-clock id space.

use core::cmp::Ordering;

use bitvec::prelude::*;

use crate::codec::{self, BitsSlice};
use crate::DecodeError;

mod ops;

#[cfg(test)]
mod tests;

/// A nonzero share of the id space. Not `Clone`. Ordered by descent /
/// reverse-inclusion: `seed` is the minimum, leaves are maximal, cousins are
/// `None`. For disjoint parties, `join` computes the meet under this order.
///
/// At rest, a `Party` holds its canonical packed preorder encoding, so
/// bit-equality is semantic equality.
#[derive(PartialEq, Eq, Hash)]
pub struct Party(BitVec<u8, Msb0>);

impl Party {
    /// The whole id space (the paper's `1`). The only nonzero constructor.
    pub fn seed() -> Self {
        let mut bits = codec::Bits::with_capacity(2);
        bits.push(false); // leaf flag
        bits.push(true); // value 1
        Party(bits)
    }

    /// Split in two; `self` keeps one half, the other is returned.
    pub fn fork(&mut self) -> Party {
        let (keep, give) = ops::split(&self.0);
        self.0 = keep;
        Party(give)
    }

    /// Merge a disjoint share into `self`; on overlap, `other` is returned.
    pub fn join(&mut self, other: Party) -> Result<(), Party> {
        if !self.is_disjoint(&other) {
            return Err(other);
        }
        self.0 = ops::sum(&self.0, &other.0);
        Ok(())
    }

    /// Whether `self` and `other` share no id-space region.
    pub fn is_disjoint(&self, other: &Party) -> bool {
        ops::is_disjoint(&self.0, &other.0)
    }

    /// The canonical packed byte encoding (preorder, uniform flag), zero-padded to
    /// a byte boundary.
    pub fn encode(&self) -> Vec<u8> {
        codec::pack_to_bytes(&self.0)
    }

    /// Decode a byte string, strictly rejecting malformed or non-canonical input.
    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let bits = codec::Bits::from_slice(bytes);
        let end = codec::parse_id(&bits, 0)?;
        codec::require_zero_padding(&bits, end)?;
        Ok(Party(bits[..end].to_bitvec()))
    }

    /// The packed preorder bit stream (no trailing padding). Internal.
    pub(crate) fn as_bits(&self) -> &BitsSlice {
        &self.0
    }

    /// Wrap a canonical packed bit stream. Internal; callers guarantee normal form.
    pub(crate) fn from_bits(bits: codec::Bits) -> Self {
        Party(bits)
    }
}

impl PartialOrd for Party {
    /// Descent: an ancestor (larger region) is *less than* its forked descendants;
    /// cousins are incomparable (`None`). `self < other` ⇔ `self` contains `other`.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (
            ops::contains(&self.0, &other.0),
            ops::contains(&other.0, &self.0),
        ) {
            (true, true) => Some(Ordering::Equal),
            (true, false) => Some(Ordering::Less),
            (false, true) => Some(Ordering::Greater),
            (false, false) => None,
        }
    }
}

impl core::fmt::Debug for Party {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let _ = f;
        todo!("Phase 7: Party Debug")
    }
}
