//! Known zero ranges between stored digits.
//!
//! A shifted update can write one digit millions of positions above the
//! current value. The positions between are known to be zero, but storing
//! them in the digit buffer is necessary so the high position remains directly
//! addressable. A later descending scan must not visit that entire gap.
//!
//! `ZeroRanges` stores each such gap as `(lo, hi)`, meaning that every position
//! strictly between the endpoints is zero. The ranges have nonempty interiors,
//! never overlap, and at completed operation boundaries never extend above the
//! highest nonzero digit. A scan removes a range when it uses it to jump to
//! `lo`; an update removes the part of every range it writes through while
//! preserving untouched pieces.
//!
//! This is enough to keep scans proportional to prior writes rather than to
//! the largest shift. Any position visited individually was previously
//! written individually. Any untouched interval is crossed once by removing
//! its range. Reaching it again requires another update either to write
//! through it or to create a new range.
//!
//! The ordered map adds logarithmic bookkeeping per lookup or edit. Its space
//! is linear in the digit buffer: disjoint ranges with nonempty interiors
//! require at least two positions each.

use std::collections::BTreeMap;

/// Disjoint open intervals whose interior digits are all zero.
#[derive(Clone, Default)]
pub struct ZeroRanges {
    /// Lower endpoint to upper endpoint, ordered for descending scans.
    ranges: Ranges,
}

/// Inline the overwhelmingly common single gap; allocate a tree only when
/// independent gaps coexist.
#[derive(Clone, Default)]
enum Ranges {
    /// No known gap.
    #[default]
    Empty,
    /// One open interval `(lo, hi)`.
    One(usize, usize),
    /// Multiple intervals ordered by their lower endpoint.
    Many(BTreeMap<usize, usize>),
}

/// Display known-zero ranges as their ordered endpoint map.
impl core::fmt::Debug for ZeroRanges {
    /// Preserve the map representation in the accumulator's debug output.
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut map = formatter.debug_map();
        match &self.ranges {
            Ranges::Empty => {}
            Ranges::One(lo, hi) => {
                map.entry(lo, hi);
            }
            Ranges::Many(ranges) => {
                for (lo, hi) in ranges {
                    map.entry(lo, hi);
                }
            }
        }
        map.finish()
    }
}

/// Record skipped ranges, trim writes from them, and consume them during scans.
impl ZeroRanges {
    /// Rebuild every interior zero range after a full-buffer rewrite.
    pub fn rebuild(&mut self, digits: &[i64]) {
        self.clear();
        let mut previous_nonzero = 0usize;
        for (index, &digit) in digits.iter().enumerate() {
            if digit == 0 {
                continue;
            }
            if index > previous_nonzero.saturating_add(1) {
                self.insert(previous_nonzero, index);
            }
            previous_nonzero = index;
        }
    }

    /// Record a write's untouched gap above the previous high digit.
    ///
    /// A genuine upward jump starts above every existing range, so its new
    /// range cannot overlap. Sign compaction can call this while old ranges
    /// still extend higher, but it writes at `previous_highest` and therefore
    /// creates no range here; the completed write trims them afterward.
    pub fn record_gap(&mut self, previous_highest: usize, landing: usize) {
        if landing > previous_highest.saturating_add(1) {
            debug_assert!(self.last().is_none_or(|(_, hi)| hi <= previous_highest));
            self.insert(previous_highest, landing);
        }
    }

    /// Remove the written interval `[from, to]` from every known-zero range.
    ///
    /// Walk from the highest lower endpoint below `to`. Disjointness lets
    /// the walk stop at the first run ending at or below `from`. Retaining
    /// a lower remnant also makes the next probe stop there.
    pub fn remove_written(&mut self, from: usize, to: usize) {
        debug_assert!(from <= to);
        if let Ranges::One(lo, hi) = self.ranges {
            if lo >= to || hi <= from {
                return;
            }
            self.ranges = Ranges::Empty;
            if from > lo + 1 {
                self.insert(lo, from);
            }
            if hi > to + 1 {
                self.insert(to, hi);
            }
            return;
        }
        if let Ranges::Many(ranges) = &mut self.ranges {
            while let Some((&lo, &hi)) = ranges.range(..to).next_back() {
                if hi <= from {
                    break;
                }
                ranges.remove(&lo);
                if from > lo + 1 {
                    ranges.insert(lo, from);
                }
                if hi > to + 1 {
                    ranges.insert(to, hi);
                }
            }
        }
        self.compact_storage();
    }

    /// Remove and return a known-zero range immediately below `above`.
    ///
    /// The returned index is its lower endpoint; every digit strictly
    /// between that index and `above` is zero.
    pub fn take_below(&mut self, above: usize) -> Option<usize> {
        match &mut self.ranges {
            Ranges::Empty => None,
            Ranges::One(lo, hi) if *lo < above && *hi >= above => {
                let lo = *lo;
                debug_assert!(lo.saturating_add(1) < *hi);
                self.ranges = Ranges::Empty;
                Some(lo)
            }
            Ranges::One(_, _) => None,
            Ranges::Many(ranges) => {
                let (&lo, &hi) = ranges.range(..above).next_back()?;
                if hi < above {
                    return None;
                }
                debug_assert!(lo.saturating_add(1) < hi);
                ranges.remove(&lo);
                self.compact_storage();
                Some(lo)
            }
        }
    }

    /// Discard all ranges after reset or collapse to zero.
    pub fn clear(&mut self) {
        self.ranges = Ranges::Empty;
    }

    /// Insert one range, promoting to ordered storage only for a second range.
    fn insert(&mut self, lo: usize, hi: usize) {
        match &mut self.ranges {
            Ranges::Empty => self.ranges = Ranges::One(lo, hi),
            Ranges::One(old_lo, old_hi) if *old_lo == lo => *old_hi = hi,
            Ranges::One(old_lo, old_hi) => {
                let mut ranges = BTreeMap::new();
                ranges.insert(*old_lo, *old_hi);
                ranges.insert(lo, hi);
                self.ranges = Ranges::Many(ranges);
            }
            Ranges::Many(ranges) => {
                ranges.insert(lo, hi);
            }
        }
    }

    /// Highest range, if any.
    fn last(&self) -> Option<(usize, usize)> {
        match &self.ranges {
            Ranges::Empty => None,
            Ranges::One(lo, hi) => Some((*lo, *hi)),
            Ranges::Many(ranges) => ranges.last_key_value().map(|(&lo, &hi)| (lo, hi)),
        }
    }

    /// Return from tree storage when at most one range remains.
    fn compact_storage(&mut self) {
        let replacement = match &self.ranges {
            Ranges::Many(ranges) if ranges.is_empty() => Some(Ranges::Empty),
            Ranges::Many(ranges) if ranges.len() == 1 => {
                let (&lo, &hi) = ranges.first_key_value().expect("the map has one range");
                Some(Ranges::One(lo, hi))
            }
            _ => None,
        };
        if let Some(replacement) = replacement {
            self.ranges = replacement;
        }
    }

    /// Check the range invariants exercised by the operation-sequence tests.
    #[cfg(test)]
    pub fn assert_invariants(&self, digits: &[i64], highest_nonzero: usize, schedule: &[u8]) {
        let mut previous_end = 0;
        let check = |lo: usize, hi: usize, previous_end: &mut usize| {
            assert!(
                lo + 1 < hi,
                "empty zero range ({lo}, {hi}) after {schedule:?}"
            );
            assert!(
                hi <= highest_nonzero,
                "zero range ({lo}, {hi}) above highest digit {highest_nonzero} after {schedule:?}"
            );
            assert!(
                lo >= *previous_end,
                "overlapping zero range ({lo}, {hi}) after {schedule:?}"
            );
            assert!(
                digits[lo + 1..hi].iter().all(|&digit| digit == 0),
                "zero range ({lo}, {hi}) covers a nonzero digit after {schedule:?}"
            );
            *previous_end = hi;
        };
        match &self.ranges {
            Ranges::Empty => {}
            Ranges::One(lo, hi) => check(*lo, *hi, &mut previous_end),
            Ranges::Many(ranges) => {
                for (&lo, &hi) in ranges {
                    check(lo, hi, &mut previous_end);
                }
            }
        }
    }

    /// Check that inactive digit storage has no known-zero ranges.
    #[cfg(test)]
    pub fn assert_empty(&self, schedule: &[u8]) {
        assert!(
            matches!(self.ranges, Ranges::Empty),
            "inactive digit storage has zero ranges after {schedule:?}"
        );
    }
}
