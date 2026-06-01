//! [`Version`] — an interval-tree-clock event tree / message, and its working-form
//! mutation [`Batch`].

use core::cmp::Ordering;
use core::fmt::Display;
use core::ops::{BitOr, BitOrAssign};

use bitvec::prelude::*;

use crate::codec::{self, BitsSlice};
use crate::{DecodeError, ParseError, Party};

use self::compare::EvView;
use self::working::WorkingVersion;

mod compare;
mod event;
mod working;

#[cfg(test)]
mod tests;

/// An event tree / message; an anonymous clock. `Eq`/`Hash` are structural over
/// the canonical encoding; `PartialOrd` is the causal order (`None` ⇔ concurrent),
/// consistent with `Eq` because normal form is canonical.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Version(BitVec<u8, Msb0>);

impl Version {
    /// The empty history (identity for `|`): the event tree `Leaf(0)`.
    pub fn new() -> Self {
        let mut bits = codec::Bits::new();
        bits.push(false); // leaf flag
        codec::encode_int(&mut bits, &codec::Base::ZERO);
        Version(bits)
    }

    /// Advance `party`'s component by one event. Single-op batch.
    pub fn tick(&mut self, party: &Party) {
        self.batch().tick(party);
    }

    /// Begin a batch of operations on this [`Version`].
    ///
    /// The same operations are available on a [`Batch`] as on a [`Version`],
    /// but multiple sequential operations within a [`Batch`] are more
    /// efficient.
    pub fn batch(&mut self) -> Batch<'_> {
        Batch {
            version: self,
            work: None,
        }
    }

    /// A read-only view of this version's event tree.
    fn view(&self) -> EvView<'_> {
        EvView::Packed(&self.0)
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
        self.view().causal_cmp(&other.view())
    }
}

/// Paper notation: `n` leaves, `(n, e1, e2)` nodes. E.g. `(1, 2, (0, (1, 0, 2), 0))`.
impl core::fmt::Display for Version {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        codec::write_ev(&self.0, f, ", ")
    }
}

/// `Version(<paper notation, space-separated>)`, e.g. `Version(1, 2, (0, (1, 0, 2), 0))`.
impl core::fmt::Debug for Version {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        <Self as Display>::fmt(self, f)
    }
}

/// Parse paper notation (`n | (n, e1, e2)`), strictly rejecting non-normal-form input.
impl core::str::FromStr for Version {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, ParseError> {
        Ok(Version::from_bits(codec::parse_ev_str(s)?))
    }
}

/// An event leaf from its base value, e.g. `Version::try_from(3u64)`.
impl TryFrom<u64> for Version {
    type Error = ParseError;
    fn try_from(n: u64) -> Result<Self, ParseError> {
        Ok(Version::from_bits(codec::ev_leaf(n)))
    }
}

/// An event node from an `(n, left, right)` literal, e.g.
/// `Version::try_from((1u64, 0u64, (2u64, 0u64, 1u64)))`. Rejects non-normal-form nodes
/// (no zero-base child, or a collapsible `(n, m, m)`).
impl<T, S> TryFrom<(u64, T, S)> for Version
where
    Version: TryFrom<T, Error = ParseError> + TryFrom<S, Error = ParseError>,
{
    type Error = ParseError;
    fn try_from((n, l, r): (u64, T, S)) -> Result<Self, ParseError> {
        let l = Version::try_from(l)?;
        let r = Version::try_from(r)?;
        Ok(Version::from_bits(codec::ev_node(n, &l.0, &r.0)?))
    }
}

/// A working-form session over a [`Version`]. The event-tree complexity
/// (fill/grow) lives in [`tick`](Self::tick). The working form is materialized
/// lazily and repacked into the borrowed version on drop.
pub struct Batch<'v> {
    version: &'v mut Version,
    work: Option<WorkingVersion>,
}

impl Batch<'_> {
    /// Advance `party`'s component. Chainable. **Core event operation.**
    pub fn tick(&mut self, party: &Party) -> &mut Self {
        let work = self
            .work
            .take()
            .unwrap_or_else(|| WorkingVersion::unpack(self.version.as_bits()));
        self.work = Some(event::tick(party.as_bits(), &work));
        self
    }

    /// Merge another history in place. Chainable.
    pub fn merge(&mut self, other: &Version) -> &mut Self {
        let work = self.view().join(&other.view());
        self.work = Some(work);
        self
    }

    /// Snapshot the in-progress history as an owned, canonical [`Version`] — without
    /// committing or forcing materialization. Used by `clock::Batch` for `fork`/`sync`,
    /// which must hand a concrete version to another clock mid-session.
    pub(crate) fn snapshot(&self) -> Version {
        match &self.work {
            Some(work) => Version::from_bits(work.repack()),
            None => self.version.clone(),
        }
    }

    /// A read-only view of the in-progress event tree (working form if
    /// materialized, otherwise the borrowed version's packed bits).
    fn view(&self) -> EvView<'_> {
        match &self.work {
            Some(work) => EvView::Working(work),
            None => EvView::Packed(self.version.as_bits()),
        }
    }
}

impl Drop for Batch<'_> {
    fn drop(&mut self) {
        if let Some(work) = self.work.take() {
            *self.version = Version::from_bits(work.repack());
        }
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
        let work = self.view().join(&r.view());
        Version::from_bits(work.repack())
    }
}

impl BitOrAssign<Version> for Version {
    fn bitor_assign(&mut self, r: Version) {
        let work = self.view().join(&r.view());
        *self = Version::from_bits(work.repack());
    }
}

impl BitOrAssign<&Version> for Batch<'_> {
    fn bitor_assign(&mut self, r: &Version) {
        self.merge(r);
    }
}

// Causal comparison across {Version, Batch}², reading current state in place.
// `Version`/`Version` lives separately (derived `PartialEq` + the `PartialOrd`
// above); this macro fills in the remaining three off-diagonal/`Batch` cells so
// the comparison matrix reads as a matrix. Each cell delegates to `causal_cmp`,
// with `eq` defined as `partial_cmp == Some(Equal)`.
macro_rules! causal_cmp_impls {
    ($($lhs:ty, $rhs:ty);* $(;)?) => {
        $(
            impl PartialEq<$rhs> for $lhs {
                fn eq(&self, o: &$rhs) -> bool {
                    self.view().causal_cmp(&o.view()) == Some(Ordering::Equal)
                }
            }
            impl PartialOrd<$rhs> for $lhs {
                fn partial_cmp(&self, o: &$rhs) -> Option<Ordering> {
                    self.view().causal_cmp(&o.view())
                }
            }
        )*
    };
}

causal_cmp_impls! {
    Version, Batch<'_>;
    Batch<'_>, Version;
    Batch<'_>, Batch<'_>;
}
