//! [`Party`] — a nonzero share of the interval-tree-clock id space.

use core::cmp::Ordering;
use core::fmt::Display;

use bitvec::prelude::*;

use crate::codec::{self, BitsSlice};
use crate::{DecodeError, ParseError};

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

    /// Merge a disjoint share into `self`; on overlap, `other` is returned unchanged.
    /// `sum` detects the overlap directly, so there is no separate disjointness scan.
    pub fn join(&mut self, other: Party) -> Result<(), Party> {
        match ops::sum(&self.0, &other.0) {
            Some(bits) => {
                self.0 = bits;
                Ok(())
            }
            None => Err(other),
        }
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

    /// The empty (zero) id, `Leaf(false)`. Internal transient only — never a public
    /// value (a `Party` is a nonzero share). Used as a placeholder when moving a party
    /// out of a `&mut` during `sync`, immediately overwritten by the re-split half.
    pub(crate) fn empty() -> Party {
        let mut bits = codec::Bits::with_capacity(2);
        bits.push(false); // leaf flag
        bits.push(false); // value 0
        Party(bits)
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

/// Paper notation: `0` / `1` leaves, `(l, r)` nodes. E.g. `(1, (0, 1))`.
impl core::fmt::Display for Party {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        codec::write_id(&self.0, f, ", ")
    }
}

/// `Party(<paper notation, space-separated>)`, e.g. `Party(1, (0, 1))`.
impl core::fmt::Debug for Party {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        <Self as Display>::fmt(self, f)
    }
}

/// Parse paper notation (`0 | 1 | (i1, i2)`), strictly rejecting non-normal-form input
/// and the anonymous identity `0` (a standalone `Party` must be a nonzero share).
impl core::str::FromStr for Party {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, ParseError> {
        finish_id(codec::parse_id_str(s)?)
    }
}

/// Wrap validated id bits as a `Party`, rejecting the anonymous (empty) identity. The
/// single gate through which every parsed/built top-level `Party` passes.
fn finish_id(bits: codec::Bits) -> Result<Party, ParseError> {
    if codec::id_is_empty(&bits) {
        Err(ParseError::Anonymous)
    } else {
        Ok(Party::from_bits(bits))
    }
}

/// An id literal that can ground out a [`Party`] tuple: the `u8` leaves `0`/`1` and
/// nested `(left, right)` tuples. Sealed and hidden — an implementation detail enabling
/// `Party::try_from(..)` literals. Unlike the public `TryFrom`, an `IdLit` leaf of `0`
/// is allowed (it is a valid *sub-tree*); the anonymous check happens only once the
/// whole id is assembled (see [`finish_id`]).
mod sealed {
    pub trait Sealed {}
    impl Sealed for u8 {}
    impl<T, S> Sealed for (T, S) {}
}

#[doc(hidden)]
pub trait PartyLiteral: sealed::Sealed {
    #[doc(hidden)]
    fn into_id_bits(self) -> Result<codec::Bits, ParseError>;
}

impl PartyLiteral for u8 {
    fn into_id_bits(self) -> Result<codec::Bits, ParseError> {
        match self {
            0 => Ok(codec::id_leaf(false)),
            1 => Ok(codec::id_leaf(true)),
            _ => Err(ParseError::Syntax),
        }
    }
}

impl<T: PartyLiteral, S: PartyLiteral> PartyLiteral for (T, S) {
    fn into_id_bits(self) -> Result<codec::Bits, ParseError> {
        let l = self.0.into_id_bits()?;
        let r = self.1.into_id_bits()?;
        codec::id_node(&l, &r) // assembles + validates normal form
    }
}

/// An id leaf from a single bit: `1` (full) is a valid `Party`; `0` is the anonymous
/// identity and is rejected here, though it is allowed as a sub-tree in the tuple form.
impl TryFrom<u8> for Party {
    type Error = ParseError;
    fn try_from(v: u8) -> Result<Self, ParseError> {
        finish_id(v.into_id_bits()?)
    }
}

/// An id node from a `(left, right)` literal, e.g. `Party::try_from((1u8, (0u8, 1u8)))`.
/// Rejects a collapsible `(v, v)` (non-canonical) and an all-`0` (anonymous) result.
impl<T: PartyLiteral, S: PartyLiteral> TryFrom<(T, S)> for Party {
    type Error = ParseError;
    fn try_from(t: (T, S)) -> Result<Self, ParseError> {
        finish_id(t.into_id_bits()?)
    }
}
