use std::cmp::Ordering;
use std::mem;
use std::ops::{BitOr, BitOrAssign};

use imbl::OrdMap;

/// A sparse copy-on-write version vector amongst parties of type `P`.
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct Version<P: Ord> {
    versions: OrdMap<P, u64>,
}

impl<P: Ord> Default for Version<P> {
    fn default() -> Self {
        Self {
            versions: Default::default(),
        }
    }
}

impl<P: Ord> Version<P> {
    /// Construct a version vector from any number of other version vectors.
    pub fn new<I>(i: I) -> Self
    where
        P: Clone,
        I: IntoIterator<Item = Self>,
    {
        Self {
            versions: OrdMap::unions_with(i.into_iter().map(|v| v.versions), u64::max),
        }
    }

    /// Record an event for some party, incrementing its version.
    pub fn event(&mut self, party: P)
    where
        P: Clone,
    {
        *self.versions.entry(party).or_default() += 1;
    }
}

/// Version vector partial ordering: `a <= b` iff every party's count in `a` is
/// at most the corresponding count in `b` (missing entries count as 0). If one
/// side is pointwise-less on some party and pointwise-greater on another, the
/// versions are concurrent and incomparable, so `partial_cmp` returns `None`.
impl<P: Ord> PartialOrd for Version<P> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let mut result = Ordering::Equal;
        let mut left = self.versions.iter().peekable();
        let mut right = other.versions.iter().peekable();
        loop {
            let step = match (left.peek(), right.peek()) {
                (None, None) => return Some(result),
                (Some(&(_, &lv)), None) => {
                    left.next();
                    lv.cmp(&0)
                }
                (None, Some(&(_, &rv))) => {
                    right.next();
                    0u64.cmp(&rv)
                }
                (Some(&(lp, &lv)), Some(&(rp, &rv))) => match lp.cmp(rp) {
                    Ordering::Less => {
                        left.next();
                        lv.cmp(&0)
                    }
                    Ordering::Greater => {
                        right.next();
                        0u64.cmp(&rv)
                    }
                    Ordering::Equal => {
                        left.next();
                        right.next();
                        lv.cmp(&rv)
                    }
                },
            };
            match (result, step) {
                (_, Ordering::Equal) => {}
                (Ordering::Equal, s) => result = s,
                (Ordering::Less, Ordering::Less) | (Ordering::Greater, Ordering::Greater) => {}
                _ => return None,
            }
        }
    }
}

impl<P: Ord + Clone> BitOrAssign for Version<P> {
    fn bitor_assign(&mut self, rhs: Self) {
        let lhs = mem::take(&mut self.versions);
        self.versions = lhs.union_with(rhs.versions, u64::max);
    }
}

impl<P: Ord + Clone> BitOr for Version<P> {
    type Output = Version<P>;

    fn bitor(mut self, rhs: Self) -> Self::Output {
        self |= rhs;
        self
    }
}

#[cfg(test)]
mod test;
