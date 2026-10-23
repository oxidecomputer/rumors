//! A set of positions shaped for descending scans: a run of consecutive top
//! positions above the rest.
//!
//! [`WrittenPositions`] answers the two questions a scan down a digit buffer
//! asks: which position below this one is in the set
//! ([`last_below`](WrittenPositions::last_below)), and, once the scan stops,
//! drop everything above it ([`truncate`](WrittenPositions::truncate)). It
//! also takes the spans that writes add
//! ([`insert_span`](WrittenPositions::insert_span)) and clears a known span
//! ([`clear`](WrittenPositions::clear)). What membership means for the digits
//! is [`Digits`](super::Digits)' concern; this module promises only set
//! semantics and the costs below.
//!
//! # Representation
//!
//! The set is a *run* of consecutive positions, possibly empty, that lies
//! above every other position, and the positions below the run, held in one
//! of two forms:
//!
//! - a *prefix* `0..n`, inline, while the set has at most one gap, between
//!   the prefix and the run;
//! - a [`Bitset`], once it has a second gap.
//!
//! Writes mostly add consecutive positions upward: a limb stream from its
//! lowest digit, a carry chain from its first. Such a span lies inside the
//! run or extends it, at the cost of a comparison. A span that jumps above
//! the run moves the run below it and starts a new run where it lands. A span
//! below the run either continues or overlaps the prefix, which absorbs it;
//! or lies in the gap and reaches the run, which lowers the run's start; or
//! lies in the gap apart from both, leaving a gap on each side of it.
//!
//! Only that last case, a second gap, converts the prefix, and the
//! conversion is the only change of form. It runs one way: the prefix's
//! positions move into a new bitset one insertion each. Each of those
//! positions was added by a write, and the bitset is kept from then on, even
//! when cleared, so the move is charged to the writes that added the prefix
//! and happens at most once per set.
//!
//! A truncation that empties the run leaves it just above the positions that
//! remain. A later span above them then extends that empty run or jumps over
//! it, so it adds no gap the set did not already have. In the bitset form
//! this keeps a scan's write-back, one position below an old top, out of the
//! bitset. In the prefix form it keeps the conversion rule exact: an empty
//! run left higher up would make a write inside the gap below it a span
//! below the run, and so a conversion.
//!
//! A clear reduces the set to position zero. The run then holds zero alone,
//! as it does when zero is inserted into a new set, so that writes from zero
//! upward extend the run at the cost of a comparison. A bitset keeps
//! position zero as well. This is the one case in which a position is held
//! both by the run and by the bitset, and the bitset then holds nothing else,
//! because every other position it holds lies below the run's start. A later
//! jump moves the run into the bitset, and finds zero already there: its
//! insertions stop at the words they share with zero, where inserting zero
//! into an emptied bitset would climb every level.
//!
//! # Costs
//!
//! Counting comparisons and the [`Bitset`]'s operations, whose costs its
//! [module](bitset#costs) states:
//!
//! - adding a span within, extending, or lowering the run costs O(1); a span
//!   beside the run costs one bitset insertion per position it adds below
//!   the run, plus the one-time conversion above;
//! - [`last_below`](WrittenPositions::last_below) costs O(1) within the run or
//!   the prefix, and one bitset predecessor query otherwise;
//! - [`truncate`](WrittenPositions::truncate) costs O(1) on the run or the
//!   prefix, and one bitset predecessor query and removal for each bitset
//!   position it removes, plus one query to find the remaining top;
//! - [`clear`](WrittenPositions::clear) costs O(1) inline and one bitset span
//!   clear otherwise. That clear is amortized: besides the words covering the
//!   span, it climbs above a level only after emptying a word there, and
//!   each such climb is charged to the insertion that filled that word, as
//!   [`Bitset::clear`] explains.
//!
//! Space is four words inline, and the bitset's [storage](bitset#space) once
//! converted.

mod bitset;
#[cfg(test)]
mod tests;

use core::ops::Range;

use bitset::Bitset;

/// A set of positions, ordered for predecessor queries.
#[derive(Clone)]
pub struct WrittenPositions {
    /// Consecutive positions, possibly none, above every position in `below`
    /// other than a position zero that both hold.
    run: Range<usize>,
    /// Every position of the set below `run.start`, and, in the bitset form
    /// after a clear, position zero while the run holds it too.
    below: Below,
}

/// The positions of a [`WrittenPositions`] below its run.
#[derive(Clone)]
enum Below {
    /// Exactly the positions `0..n`, held inline.
    ///
    /// With the run above it, this holds any set with at most one gap
    /// between its two parts, which is all a value written once at a shift
    /// needs.
    Prefix(usize),
    /// Any positions, in a bitset allocated when a second gap first appears,
    /// and kept from then on.
    ///
    /// In a set whose first insertion included position zero, as every set
    /// `Digits` writes, the bitset holds zero from its creation on, which is
    /// what lets [`clear`](WrittenPositions::clear) keep zero without
    /// inserting it:
    ///
    /// - Zero stays in such a set: a truncation removes only the positions
    ///   above the one it truncates to, and a clear keeps zero.
    /// - The conversion that creates the bitset moves a prefix that holds
    ///   zero. While the set holds zero, either the prefix holds it, or the
    ///   run starts at zero; then nothing lies below the run, and a jump
    ///   folds the run into the prefix rather than converting it.
    /// - Once in the bitset, zero leaves it neither by truncation nor by a
    ///   clear.
    ///
    /// A set that gains zero only after its bitset exists can hold zero in
    /// the run alone.
    Bitset(Box<Bitset>),
}

/// Start with no positions and no allocation.
impl Default for WrittenPositions {
    /// Construct the empty set: an empty run above an empty prefix.
    fn default() -> Self {
        Self {
            run: 0..0,
            below: Below::Prefix(0),
        }
    }
}

/// List the positions in the set, in ascending order.
impl core::fmt::Debug for WrittenPositions {
    /// Format the set as its positions, visiting every retained bitset word.
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_set()
            .entries(self.below_positions().chain(self.run.clone()))
            .finish()
    }
}

/// Insert, truncate, and find positions, mostly within the top run.
impl WrittenPositions {
    /// Add `position` to the set.
    ///
    /// # Panics
    ///
    /// Panics if `position` is `usize::MAX`, which no digit buffer can index.
    pub fn insert(&mut self, position: usize) {
        self.insert_span(position, position);
    }

    /// Add every position from `first` through `last`.
    ///
    /// # Panics
    ///
    /// Panics if `last` is `usize::MAX`, which no digit buffer can index.
    #[inline]
    pub fn insert_span(&mut self, first: usize, last: usize) {
        debug_assert!(first <= last, "a span runs upward");
        if self.run.start <= first && first <= self.run.end {
            // The span lies in the run or extends it.
            self.run.end = self.run.end.max(after(last));
        } else {
            self.insert_span_beside_run(first, last);
        }
    }

    /// Add a span that starts below the run or above its end.
    ///
    /// Kept out of line so that a write within or extending the run, the
    /// common case, carries only the comparisons in
    /// [`insert_span`](Self::insert_span).
    #[inline(never)]
    fn insert_span_beside_run(&mut self, first: usize, last: usize) {
        if first > self.run.end {
            // A jump above the run: the run joins the positions below it, and
            // the span starts a new run above them.
            if !self.run.is_empty() {
                self.insert_below(self.run.start, self.run.end - 1);
            }
            self.run = first..after(last);
        } else if matches!(self.below, Below::Prefix(end) if end < first)
            && last >= self.run.start - 1
        {
            // The span lies in the gap between the prefix and the run and
            // reaches the run, so it shortens that one gap from above: the run
            // starts lower, and the set keeps its single gap.
            self.run = first..self.run.end.max(after(last));
        } else {
            // The span starts below the run. Its part below the run joins the
            // positions below, and any part reaching the run extends it.
            self.insert_below(first, last.min(self.run.start - 1));
            if last >= self.run.start {
                self.run.end = self.run.end.max(after(last));
            }
        }
    }

    /// Add every position from `first` through `last` to the positions below
    /// the run.
    ///
    /// The caller guarantees that the span lies below the run the set has
    /// once the caller returns, with a gap between them: a jump moves the old
    /// run here before starting a new one above it, and a span that reaches
    /// the run from the prefix's gap lowers the run instead of coming here.
    fn insert_below(&mut self, first: usize, last: usize) {
        debug_assert!(first <= last, "a span runs upward");
        match &mut self.below {
            Below::Prefix(end) if first <= *end => *end = (*end).max(last + 1),
            Below::Prefix(end) => {
                // The span starts past the prefix's end and, by the caller's
                // guarantee, ends before a gap below the run: a second gap,
                // which only a bitset can hold.
                let mut bitset = Box::<Bitset>::default();
                for position in (0..*end).chain(first..=last) {
                    bitset.insert(position);
                }
                self.below = Below::Bitset(bitset);
            }
            Below::Bitset(bitset) => {
                for position in first..=last {
                    bitset.insert(position);
                }
            }
        }
    }

    /// Remove every position above `highest`.
    ///
    /// Callers truncate only through positions they have visited, which pays
    /// for removing them from a bitset one at a time.
    pub fn truncate(&mut self, highest: usize) {
        // Nothing lies above `usize::MAX`.
        let Some(end) = highest.checked_add(1) else {
            return;
        };
        if end > self.run.start {
            self.run.end = self.run.end.min(end);
            return;
        }
        // The whole run lies above `highest`, and so may the top of the
        // positions below it. The emptied run then waits just above the
        // positions that remain.
        let remaining_end = match &mut self.below {
            Below::Prefix(prefix_end) => {
                *prefix_end = (*prefix_end).min(end);
                *prefix_end
            }
            Below::Bitset(bitset) => {
                // Each query starts from the position just removed.
                let mut above = self.run.start;
                loop {
                    match bitset.last_below(above) {
                        Some(position) if position >= end => {
                            bitset.remove(position);
                            above = position;
                        }
                        Some(position) => break position + 1,
                        None => break 0,
                    }
                }
            }
        };
        self.run = remaining_end..remaining_end;
    }

    /// Return whether `position` is in the set.
    #[cfg(any(test, debug_assertions))]
    pub fn contains(&self, position: usize) -> bool {
        self.run.contains(&position)
            || match &self.below {
                Below::Prefix(end) => position < *end,
                Below::Bitset(bitset) => bitset.contains(position),
            }
    }

    /// Return the largest position in the set strictly below `position`, if
    /// any.
    #[inline]
    pub fn last_below(&self, position: usize) -> Option<usize> {
        if position > self.run.start && !self.run.is_empty() {
            // The run holds something below `position`, and it lies above
            // everything below the run.
            Some(position.min(self.run.end) - 1)
        } else {
            match &self.below {
                Below::Prefix(end) => position.min(*end).checked_sub(1),
                Below::Bitset(bitset) => bitset.last_below(position),
            }
        }
    }

    /// Reduce the set to position zero, given that the set's first insertion
    /// included position zero and that every other position lies in
    /// `lowest..=highest`.
    ///
    /// Costs O(1) in the prefix form, and in the bitset form an amortized
    /// cost proportional to the span, as the [module](self#costs) explains;
    /// a bitset keeps its storage, and position zero, for reuse. The bitset
    /// clears without inserting zero first, because in such a set it already
    /// holds zero ([`Below::Bitset`]). `Digits` inserts zero first into
    /// every set it writes. A set without position zero, or with a position
    /// outside the span, fails an assertion in a debug build, as does a
    /// bitset without zero; in a release build, every later operation on the
    /// set may then return a wrong result or panic.
    pub fn clear(&mut self, lowest: usize, highest: usize) {
        debug_assert!(
            self.last_below(1) == Some(0),
            "clear's caller guarantees that the set holds position zero"
        );
        debug_assert!(
            self.last_below(usize::MAX) <= Some(highest)
                && self.last_below(lowest).is_none_or(|position| position == 0),
            "clear's caller guarantees that every position is zero or lies in `lowest..=highest`"
        );
        match &mut self.below {
            Below::Prefix(end) => *end = 0,
            Below::Bitset(bitset) => bitset.clear(lowest, highest),
        }
        // In either form the run holds position zero, so that writes from
        // zero upward extend it; a bitset holds zero as well.
        self.run = 0..1;
    }

    /// Check that the run lies above every other position, except a position
    /// zero that a bitset shares with it, and the bitset's own invariants.
    #[cfg(test)]
    pub fn assert_structure(&self, context: &dyn core::fmt::Debug) {
        assert!(
            self.run.start <= self.run.end,
            "the run {:?} is a range after {context:?}",
            self.run
        );
        match &self.below {
            Below::Prefix(end) => assert!(
                *end <= self.run.start,
                "the prefix 0..{end} reaches the run {:?} after {context:?}",
                self.run
            ),
            Below::Bitset(bitset) => {
                assert!(
                    bitset.iter().all(|position| position < self.run.start
                        || (position == 0 && self.run.contains(&0))),
                    "a bitset position other than a zero the run holds lies at or above the \
                     run {:?} after {context:?}",
                    self.run
                );
                bitset.assert_invariants(context);
            }
        }
    }

    /// Name the form holding the positions below the run: `"prefix"` or
    /// `"bitset"`.
    #[cfg(test)]
    pub fn form(&self) -> &'static str {
        match self.below {
            Below::Prefix(_) => "prefix",
            Below::Bitset(_) => "bitset",
        }
    }

    /// Count the words of bitset storage the set retains; zero while the
    /// positions below the run are a prefix.
    #[cfg(test)]
    pub fn storage_words(&self) -> usize {
        match &self.below {
            Below::Prefix(_) => 0,
            Below::Bitset(bitset) => bitset.storage_words(),
        }
    }

    /// List every position in the set, in ascending order.
    #[cfg(test)]
    pub fn positions(&self) -> Vec<usize> {
        self.below_positions().chain(self.run.clone()).collect()
    }

    /// Iterate the positions below the run, in ascending order.
    ///
    /// A position zero that a bitset shares with the run is the run's to
    /// list, so it is skipped here.
    fn below_positions(&self) -> impl Iterator<Item = usize> + '_ {
        let (prefix, bitset) = match &self.below {
            Below::Prefix(end) => (0..*end, None),
            Below::Bitset(bitset) => (0..0, Some(bitset.as_ref())),
        };
        prefix.chain(
            bitset
                .into_iter()
                .flat_map(Bitset::iter)
                .filter(|position| !self.run.contains(position)),
        )
    }
}

/// The position after `position`.
///
/// # Panics
///
/// Panics if `position` is `usize::MAX`, which no digit buffer can index.
fn after(position: usize) -> usize {
    position
        .checked_add(1)
        .expect("a written position indexes the digit buffer, so it lies below usize::MAX")
}
