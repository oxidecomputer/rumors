//! [`Party`] — a nonzero share of the interval-tree-clock id space.

use core::cmp::Ordering;

use bitvec::prelude::*;

use crate::DecodeError;

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
        todo!()
    }

    /// Split in two; `self` keeps one half, the other is returned.
    pub fn fork(&mut self) -> Party {
        todo!()
    }

    /// Merge a disjoint share into `self`; on overlap, `other` is returned.
    pub fn join(&mut self, other: Party) -> Result<(), Party> {
        let _ = other;
        todo!()
    }

    /// Whether `self` and `other` share no id-space region.
    pub fn is_disjoint(&self, other: &Party) -> bool {
        let _ = other;
        todo!()
    }

    /// The canonical packed byte encoding (preorder, uniform flag).
    pub fn encode(&self) -> Vec<u8> {
        todo!()
    }

    /// Decode a byte string, strictly rejecting malformed or non-canonical input.
    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let _ = bytes;
        todo!()
    }
}

impl PartialOrd for Party {
    /// Descent: an ancestor (larger region) is *less than* its forked descendants;
    /// cousins are incomparable (`None`).
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let _ = other;
        todo!()
    }
}

impl core::fmt::Debug for Party {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let _ = f;
        todo!()
    }
}
