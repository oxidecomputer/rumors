use std::cmp::Ordering;
use std::hash::Hash;
use std::mem;
use std::ops::{BitOr, BitOrAssign};

use imbl::HashMap;

/// A sparse copy-on-write version vector amongst parties of type `P`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Version<P: Hash + Eq> {
    versions: HashMap<P, u64>,
}

impl<P: Hash + Eq> Default for Version<P> {
    fn default() -> Self {
        Self {
            versions: Default::default(),
        }
    }
}

impl<P: Hash + Eq> Version<P> {
    /// Construct a version vector from any number of other version vectors.
    pub fn new<I>(i: I) -> Self
    where
        P: Clone,
        I: IntoIterator<Item = Self>,
    {
        Self {
            versions: HashMap::unions_with(i.into_iter().map(|v| v.versions), u64::max),
        }
    }

    /// Record an event for some party, incrementing its version.
    pub fn event(&mut self, party: P)
    where
        P: Clone,
    {
        *self.versions.entry(party).or_default() += 1;
    }

    /// Get the version for a particular party.
    pub fn for_party(&self, party: &P) -> u64 {
        *self.versions.get(&party).unwrap_or(&0)
    }
}

/// Version vector partial ordering: `a <= b` iff every party's count in `a` is
/// at most the corresponding count in `b` (missing entries count as 0). If one
/// side is pointwise-less on some party and pointwise-greater on another, the
/// versions are concurrent and incomparable, so `partial_cmp` returns `None`.
impl<P: Hash + Eq> PartialOrd for Version<P> {
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
            let right = other.versions.get(party).copied().unwrap_or(0);
            verdict = refine(verdict, left.cmp(&right))?;
        }

        // Parties present only in `other` contribute with an implicit zero on
        // the left; parties present in both were already handled above.
        for (party, &right) in &other.versions {
            if !self.versions.contains_key(party) {
                verdict = refine(verdict, 0u64.cmp(&right))?;
            }
        }

        Some(verdict)
    }
}

impl<P: Hash + Eq + Clone> BitOrAssign for Version<P> {
    fn bitor_assign(&mut self, rhs: Self) {
        let lhs = mem::take(&mut self.versions);
        self.versions = lhs.union_with(rhs.versions, u64::max);
    }
}

impl<P: Hash + Eq + Clone> BitOr for Version<P> {
    type Output = Version<P>;

    fn bitor(mut self, rhs: Self) -> Self::Output {
        self |= rhs;
        self
    }
}

impl<P: Clone + Hash + Eq> From<(P, u64)> for Version<P> {
    fn from(value: (P, u64)) -> Self {
        let mut result = Self::default();
        result.versions.insert(value.0, value.1);
        result
    }
}

#[cfg(test)]
mod test;
