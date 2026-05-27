use std::cmp::Ordering;
use std::fmt::Debug;
use std::mem;
use std::num::NonZeroU64;
use std::ops::{BitOr, BitOrAssign};

use borsh::{BorshDeserialize, BorshSerialize};
use bytes::Bytes;
use imbl::OrdMap;

use crate::imbl_borsh::{deserialize_ordmap, serialize_ordmap};

#[derive(Clone, PartialEq, Eq)]
pub struct Version<P: Ord = Bytes> {
    versions: OrdMap<P, NonZeroU64>,
}

/// `NonZeroU64::new(1)`, evaluated at compile time. Used by [`Version::event`]
/// to seed an entry for a party observing its first event.
const ONE: NonZeroU64 = NonZeroU64::new(1).expect("1 is non-zero");

/// The empty version: no party has been observed yet. Pointwise-less than
/// or equal to every other version under [`PartialOrd`].
impl<P: Ord> Default for Version<P> {
    fn default() -> Self {
        Self {
            versions: Default::default(),
        }
    }
}

impl<P: Ord + Debug> Debug for Version<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.versions.fmt(f)
    }
}

impl<P: Ord> Version<P> {
    /// Construct a version vector from any number of other version vectors.
    pub(crate) fn new<I>(i: I) -> Self
    where
        P: Clone,
        I: IntoIterator<Item = Self>,
    {
        Self {
            versions: OrdMap::unions_with(i.into_iter().map(|v| v.versions), |a, b| a.max(b)),
        }
    }

    /// Record an event for some party, incrementing its version.
    pub(crate) fn event(&mut self, party: &P)
    where
        P: Clone,
    {
        if let Some(v) = self.versions.get_mut(party) {
            *v = v.checked_add(1).expect("version counter overflow");
        } else {
            self.versions.insert(party.clone(), ONE);
        }
    }

    /// Get the version for a particular party. Absent parties report `0`.
    pub(crate) fn for_party(&self, party: &P) -> u64 {
        self.versions.get(party).map(|v| v.get()).unwrap_or(0)
    }

    /// Get a reference to the underlying version vector. The inner counter
    /// is [`NonZeroU64`] because an entry with value `0` is structurally
    /// identical to an absent entry under [`for_party`](Self::for_party).
    pub(crate) fn versions(&self) -> &OrdMap<P, NonZeroU64> {
        &self.versions
    }
}

/// Version vector partial ordering: `a <= b` iff every party's count in `a` is
/// at most the corresponding count in `b` (missing entries count as 0). If one
/// side is pointwise-less on some party and pointwise-greater on another, the
/// versions are concurrent and incomparable, so `partial_cmp` returns `None`.
impl<P: Ord> PartialOrd for Version<P> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Fold one party's comparison into the running product-order verdict.
        // An `Equal` observation leaves the verdict unchanged; a non-equal
        // observation upgrades an `Equal` verdict, reinforces a matching one,
        // or — if it opposes the verdict — witnesses that the two vectors
        // disagree on direction, making them concurrent and incomparable.
        fn refine(verdict: Ordering, step: Ordering) -> Option<Ordering> {
            match (verdict, step) {
                (_, Ordering::Equal) => Some(verdict),
                (Ordering::Equal, _) => Some(step),
                (Ordering::Less, Ordering::Less) | (Ordering::Greater, Ordering::Greater) => {
                    Some(verdict)
                }
                _ => None,
            }
        }

        // Compare every party present in `self` against its counterpart in
        // `other`, treating an absent counterpart as zero.
        let mut verdict = Ordering::Equal;
        for (party, &left) in &self.versions {
            let right = other.versions.get(party).map(|v| v.get()).unwrap_or(0);
            verdict = refine(verdict, left.get().cmp(&right))?;
        }

        // Parties present only in `other` contribute with an implicit zero on
        // the left; parties present in both were already handled above. Every
        // entry in `other.versions` has a non-zero count, so the implicit
        // left-side zero is always strictly less than the right.
        for (party, _) in &other.versions {
            if !self.versions.contains_key(party) {
                verdict = refine(verdict, Ordering::Less)?;
            }
        }

        Some(verdict)
    }
}

/// Join: take the pointwise maximum of two version vectors. The result is
/// the least upper bound under [`PartialOrd`] — equal to either operand if
/// it dominates, otherwise strictly greater than both.
impl<P: Ord + Clone> BitOrAssign for Version<P> {
    fn bitor_assign(&mut self, rhs: Self) {
        let lhs = mem::take(&mut self.versions);
        self.versions = lhs.union_with(rhs.versions, |a, b| a.max(b));
    }
}

/// Join: pointwise maximum. See [`BitOrAssign`].
impl<P: Ord + Clone> BitOr for Version<P> {
    type Output = Version<P>;

    fn bitor(mut self, rhs: Self) -> Self::Output {
        self |= rhs;
        self
    }
}

impl<P: Ord + Clone> From<(P, u64)> for Version<P> {
    fn from(value: (P, u64)) -> Self {
        let mut result = Self::default();
        // A 0 count is structurally absent: `for_party` reports 0 for any
        // unrecorded party, and the inner map's `NonZeroU64` enforces that
        // distinction.
        if let Some(count) = NonZeroU64::new(value.1) {
            result.versions.insert(value.0, count);
        }
        result
    }
}

/// Canonical, bijective borsh encoding. Delegates to the shared
/// `imbl_borsh` helpers: entries are emitted as a length-prefixed
/// run sorted by party in strictly-ascending order, so every `Version<P>`
/// value has exactly one valid serialization, and duplicates or out-of-order
/// entries on the wire are rejected on deserialization.
impl<P: Ord + BorshSerialize> BorshSerialize for Version<P> {
    fn serialize<W: borsh::io::Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        serialize_ordmap(&self.versions, writer)
    }
}

impl<P: Ord + Clone + BorshDeserialize> BorshDeserialize for Version<P> {
    fn deserialize_reader<R: borsh::io::Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let versions = deserialize_ordmap(reader)?;
        Ok(Self { versions })
    }
}

#[cfg(test)]
mod test;
