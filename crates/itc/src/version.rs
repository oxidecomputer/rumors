//! [`Version`] — an interval-tree-clock event tree / message, and its working-form
//! mutation [`Batch`].

use core::cmp::Ordering;
use core::marker::PhantomData;
use core::ops::{BitOr, BitOrAssign};

use bitvec::prelude::*;

use crate::codec::{self, BitsSlice};
use crate::{DecodeError, Party};

/// An event tree / message; an anonymous clock. `Eq`/`Hash` are structural over
/// the canonical encoding; `PartialOrd` is the causal order (`None` ⇔ concurrent),
/// consistent with `Eq` because normal form is canonical.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Version(BitVec<u8, Msb0>);

impl Version {
    /// The empty history (identity for `|`).
    pub fn new() -> Self {
        todo!()
    }

    /// Advance `party`'s component by one event. Single-op batch.
    pub fn tick(&mut self, party: &Party) {
        self.batch().tick(party);
    }

    /// Begin a working-form session over this version.
    pub fn batch(&mut self) -> Batch<'_> {
        todo!()
    }

    /// The canonical packed byte encoding (preorder, uniform flag), zero-padded to
    /// a byte boundary.
    pub fn encode(&self) -> Vec<u8> {
        codec::pack_to_bytes(&self.0)
    }

    /// Decode a byte string, strictly rejecting malformed or non-canonical input.
    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let bits = codec::Bits::from_slice(bytes);
        let end = codec::parse_ev(&bits, 0)?;
        codec::require_zero_padding(&bits, end)?;
        Ok(Version(bits[..end].to_bitvec()))
    }

    /// The packed preorder bit stream (no trailing padding). Internal.
    pub(crate) fn as_bits(&self) -> &BitsSlice {
        &self.0
    }

    /// Wrap a canonical packed bit stream. Internal; callers guarantee normal form.
    pub(crate) fn from_bits(bits: codec::Bits) -> Self {
        Version(bits)
    }
}

impl Default for Version {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialOrd for Version {
    /// The causal order; `None` means the two versions are concurrent.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let _ = other;
        todo!()
    }
}

impl core::fmt::Debug for Version {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let _ = f;
        todo!()
    }
}

/// A working-form session over a [`Version`]. The event-tree complexity
/// (fill/grow) lives in [`tick`](Self::tick). Repacks on drop.
pub struct Batch<'v> {
    _p: PhantomData<&'v mut Version>,
}

impl Batch<'_> {
    /// Advance `party`'s component. Chainable. **Core event operation.**
    pub fn tick(&mut self, party: &Party) -> &mut Self {
        let _ = party;
        todo!()
    }

    /// Merge another history in place. Chainable.
    pub fn merge(&mut self, other: &Version) -> &mut Self {
        let _ = other;
        todo!()
    }
}

impl Drop for Batch<'_> {
    fn drop(&mut self) {
        // Repack into *version if the working form was materialized.
    }
}

impl<'a> From<&'a mut Version> for Batch<'a> {
    fn from(v: &'a mut Version) -> Self {
        v.batch()
    }
}

impl BitOr<Version> for Version {
    type Output = Version;
    fn bitor(self, r: Version) -> Version {
        let _ = r;
        todo!()
    }
}

impl BitOrAssign<Version> for Version {
    fn bitor_assign(&mut self, r: Version) {
        let _ = r;
        todo!()
    }
}

impl BitOrAssign<&Version> for Batch<'_> {
    fn bitor_assign(&mut self, r: &Version) {
        self.merge(r);
    }
}

// Causal comparison across {Version, Batch}², reading current state in place.

impl PartialEq<Batch<'_>> for Version {
    fn eq(&self, o: &Batch<'_>) -> bool {
        let _ = o;
        todo!()
    }
}

impl PartialOrd<Batch<'_>> for Version {
    fn partial_cmp(&self, o: &Batch<'_>) -> Option<Ordering> {
        let _ = o;
        todo!()
    }
}

impl PartialEq<Version> for Batch<'_> {
    fn eq(&self, o: &Version) -> bool {
        let _ = o;
        todo!()
    }
}

impl PartialOrd<Version> for Batch<'_> {
    fn partial_cmp(&self, o: &Version) -> Option<Ordering> {
        let _ = o;
        todo!()
    }
}

impl<'b> PartialEq<Batch<'b>> for Batch<'_> {
    fn eq(&self, o: &Batch<'b>) -> bool {
        let _ = o;
        todo!()
    }
}

impl<'b> PartialOrd<Batch<'b>> for Batch<'_> {
    fn partial_cmp(&self, o: &Batch<'b>) -> Option<Ordering> {
        let _ = o;
        todo!()
    }
}
