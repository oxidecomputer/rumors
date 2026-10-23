//! A set of `usize` positions as a 64-ary hierarchy of bit words, answering
//! predecessor queries in time logarithmic in the distance they skip.
//!
//! Level 0 holds one bit per position, packed into `u64` words. Level `k + 1`
//! holds one bit per word of level `k`, its *summary bit*. The hierarchy keeps
//! two invariants, which every operation preserves:
//!
//! - a summary bit is set exactly when the word it stands for is nonzero;
//! - either there are no levels, or level `k + 1` has one word per 64 words
//!   of level `k`, rounded up, and the last level has exactly one word.
//!
//! A word that changes between empty and nonempty is the only change a level
//! above needs to hear about, so operations climb only that far.
//!
//! # Costs
//!
//! Counted in word operations, for a hierarchy whose levels cover `W`
//! positions, so that it has O(1 + log₆₄ `W`) levels:
//!
//! - [`insert`](Bitset::insert) and [`remove`](Bitset::remove) cost one word
//!   per level they climb, at most every level. An insertion beyond the
//!   levels first grows them, at O(1) plus amortized O(1) per word it adds:
//!   growth fills each new word with zeros, from level 0 upward, and stops at
//!   the first level already long enough, since each level above it holds
//!   one word per 64 words of the level below, rounded up, and so is long
//!   enough too.
//! - [`last_below`](Bitset::last_below) climbs only while neither the
//!   query's word below it nor the word before that holds a position, then
//!   descends through the highest set bit of each word. The word before
//!   holds every position within one word's span below, so a query whose
//!   answer lies `d` positions down stops climbing by level log₆₄ `d`, and
//!   costs O(1 + log₆₄ `d`).
//! - [`clear`](Bitset::clear) costs amortized O(1 + `s` / 64) for a span of
//!   `s` positions, not the levels' width: see its documentation.
//!
//! # Space
//!
//! The levels cover every position up to the highest ever inserted, and keep
//! that storage when positions are removed or cleared: one bit per covered
//! position, plus summaries totaling about a sixty-third of that. An empty
//! bitset allocates nothing until its first insertion.
//!
//! # Alternatives
//!
//! The `roaring` crate's bitmaps hold sparse positions more compactly: an
//! array container stores 2 bytes per position and an empty 65,536-position
//! chunk stores nothing, where this hierarchy spends one bit on every
//! position it covers, written or not. Its predecessor query, though, scans
//! a dense container backward one word at a time, up to 1,024 words,
//! whatever the distance skipped, and its bitmaps hold only `u32` positions,
//! so `usize` positions need its two-level treemap. This module trades
//! space on sparse wide sets for a predecessor query bounded by the distance
//! it skips, which is the cost a descending scan is charged.

#[cfg(test)]
mod tests;

/// The number of positions or summary bits one word holds.
const WORD_BITS: usize = u64::BITS as usize;

/// A set of positions as a 64-ary hierarchy of bit words.
#[derive(Clone, Default)]
pub struct Bitset {
    /// `levels[0]` holds one bit per position; each later level holds the
    /// summary bits of the level before it.
    levels: Vec<Vec<u64>>,
    /// The levels [`cover`](Self::cover) has examined and left at their
    /// length, counted so that tests can check where growth stops.
    #[cfg(test)]
    unchanged_levels_examined: usize,
    /// The levels [`clear`](Self::clear) has examined, counted so that tests
    /// can check where its climb stops.
    #[cfg(test)]
    cleared_levels_examined: usize,
    /// The levels [`insert`](Self::insert) has climbed, counted so that
    /// tests can check where an insertion stops.
    #[cfg(test)]
    inserted_levels_climbed: usize,
}

/// Maintain positions and their summary bits, climbing only where a word
/// changes between empty and nonempty.
impl Bitset {
    /// Add `position`, growing the levels to cover it.
    pub fn insert(&mut self, position: usize) {
        self.cover(position);
        let mut index = position;
        for words in &mut self.levels {
            #[cfg(test)]
            {
                self.inserted_levels_climbed += 1;
            }
            let word = &mut words[index / WORD_BITS];
            let was_empty = *word == 0;
            *word |= bit(index);
            // A word that already held a bit is already summarized above.
            if !was_empty {
                return;
            }
            index /= WORD_BITS;
        }
    }

    /// Remove `position`; removing an absent position does nothing.
    pub fn remove(&mut self, position: usize) {
        let mut index = position;
        for words in &mut self.levels {
            let Some(word) = words.get_mut(index / WORD_BITS) else {
                return;
            };
            if *word & bit(index) == 0 {
                return;
            }
            *word &= !bit(index);
            // A word that still holds a bit keeps its summary bit.
            if *word != 0 {
                return;
            }
            index /= WORD_BITS;
        }
    }

    /// Return whether `position` is in the set.
    #[cfg(any(test, debug_assertions))]
    pub fn contains(&self, position: usize) -> bool {
        self.levels
            .first()
            .and_then(|words| words.get(position / WORD_BITS))
            .is_some_and(|word| word & bit(position) != 0)
    }

    /// Return the largest position in the set strictly below `position`, if
    /// any.
    #[inline(never)]
    pub fn last_below(&self, position: usize) -> Option<usize> {
        // Climb: `word` and `below` select the bits strictly below the query
        // in its word at the current level. A word beyond a level's storage
        // is empty, but every stored word lies below it, so we search the
        // last stored word whole instead.
        let mut word = position / WORD_BITS;
        let mut below = bit(position) - 1;
        let mut found = None;
        for (level, words) in self.levels.iter().enumerate() {
            let (word_index, mask) = match words.get(word) {
                Some(_) => (word, below),
                None => (words.len() - 1, u64::MAX),
            };
            let candidates = words[word_index] & mask;
            if candidates != 0 {
                found = Some((level, word_index * WORD_BITS + highest_bit(candidates)));
                break;
            }
            // Nothing lies below the query in this word. The word before it
            // holds every position within one word's span below, so a query
            // whose answer lies `d` positions down stops climbing by level
            // log₆₄ `d`. The last level has one word, so the climb ends there.
            let previous = word_index.checked_sub(1)?;
            if words[previous] != 0 {
                found = Some((level, previous * WORD_BITS + highest_bit(words[previous])));
                break;
            }
            // Both words are empty, so the answer lies in an earlier word,
            // whose summary bit sits below `previous`'s one level up.
            word = previous / WORD_BITS;
            below = bit(previous) - 1;
        }
        // Descend: the highest set bit of each summarized word leads to the
        // highest position below it.
        let (level, mut index) = found?;
        for words in self.levels[..level].iter().rev() {
            let word = words[index];
            debug_assert_ne!(word, 0, "a summary bit is set only for a nonzero word");
            index = index * WORD_BITS + highest_bit(word);
        }
        Some(index)
    }

    /// Reduce the set to position zero, given that it holds position zero
    /// and that every other position lies in `lowest..=highest`, keeping the
    /// storage.
    ///
    /// A set without position zero fails the assertion in a debug build. In
    /// a release build the clear may then break the summary invariant: over
    /// a span starting in word zero it sets position zero at level 0,
    /// possibly without the summary bits above it, and over a span starting
    /// above word zero it may set a summary bit above an empty word zero. A
    /// position other than zero outside the span is left set while summary
    /// words above it may be cleared, which breaks the invariant too. Every
    /// later operation may then return a wrong
    /// result or panic. The written set clears its bitset only when the
    /// bitset holds zero, as [`Below`](super::Below) documents.
    ///
    /// Position zero stays, so word zero stays nonempty at every level, and
    /// an insertion after the clear stops at the first word it shares with
    /// position zero, as it would before the clear, instead of climbing
    /// every level of an emptied hierarchy.
    ///
    /// The clear works upward from level 0. Every position other than zero
    /// lies in the span, so at each level, every nonempty word other than
    /// word zero lies among the words that summarize the span, and word zero
    /// holds the bit on position zero's path and otherwise only bits standing
    /// for the span. The clear sets word zero to that one bit, if word zero
    /// lies in the span, and empties the span's other words. Word zero stays
    /// nonempty, so a level at which no other word empties changes no
    /// summary bit above it, and the clear stops there.
    ///
    /// The cost is O(1 + (`highest` − `lowest`) / 64) word operations at
    /// level 0, and at each level above, O(1) plus the span's words there.
    /// The clear reaches a level only after emptying a word other than word
    /// zero on the level below. That word turned nonempty through an
    /// insertion that climbed to the level reached, and the clear leaves it
    /// empty, so each level's O(1) is charged once to that climb.
    pub fn clear(&mut self, lowest: usize, highest: usize) {
        debug_assert_eq!(
            self.last_below(1),
            Some(0),
            "clear's caller guarantees that the set holds position zero"
        );
        let mut first_word = lowest / WORD_BITS;
        let mut last_word = highest / WORD_BITS;
        for words in &mut self.levels {
            #[cfg(test)]
            {
                self.cleared_levels_examined += 1;
            }
            if first_word == 0 {
                // Every bit of word zero but the one on position zero's path
                // stands for a position, or a word, in the span.
                words[0] = bit(0);
            }
            let end = (last_word + 1).min(words.len());
            let others = words.get_mut(first_word.max(1)..end).unwrap_or_default();
            if others.iter().all(|&word| word == 0) {
                return;
            }
            others.fill(0);
            first_word /= WORD_BITS;
            last_word /= WORD_BITS;
        }
    }

    /// Iterate the positions in ascending order, reading level 0 alone.
    ///
    /// Visits every retained level-0 word, so it costs time proportional to
    /// the retained storage rather than to the positions held.
    pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        let words = self.levels.first().map_or(&[][..], Vec::as_slice);
        words
            .iter()
            .enumerate()
            .flat_map(|(word_index, &word)| word_positions(word_index, word))
    }

    /// Count the words of storage the levels retain, by capacity.
    #[cfg(test)]
    pub fn storage_words(&self) -> usize {
        self.levels.iter().map(Vec::capacity).sum()
    }

    /// Grow the levels so that `position` is addressable, from level 0
    /// upward, stopping at the first level already long enough or at a new
    /// one-word top.
    ///
    /// A level created on top summarizes the level below it, so summary bits
    /// stay exact across growth.
    fn cover(&mut self, position: usize) {
        let mut needed = position / WORD_BITS + 1;
        // Checking level 0 before the loop keeps the common insertion, one
        // within the levels, off the loop's path; the loop's first iteration
        // repeats the check only for a growing call.
        if self
            .levels
            .first()
            .is_some_and(|words| words.len() >= needed)
        {
            #[cfg(test)]
            {
                self.unchanged_levels_examined += 1;
            }
            return;
        }
        for level in 0.. {
            if let Some(words) = self.levels.get_mut(level) {
                // Each level above this one has `div_ceil(64)` of the words
                // of the level below it, and `needed` shrinks the same way,
                // so once one level is long enough, every level above it is.
                if words.len() >= needed {
                    #[cfg(test)]
                    {
                        self.unchanged_levels_examined += 1;
                    }
                    return;
                }
                words.resize(needed, 0);
            } else {
                let mut words = vec![0; needed];
                if let Some(below) = level.checked_sub(1).map(|below| &self.levels[below]) {
                    for (index, &word) in below.iter().enumerate() {
                        if word != 0 {
                            words[index / WORD_BITS] |= bit(index);
                        }
                    }
                }
                // The levels vector grows by exactly one level at a time,
                // rather than reserving room for several it may never need.
                self.levels.reserve_exact(1);
                self.levels.push(words);
            }
            if needed == 1 {
                break;
            }
            needed = needed.div_ceil(WORD_BITS);
        }
    }

    /// Check both invariants of the module: each summary bit is set exactly
    /// when its word is nonzero, and the levels have the documented shape.
    #[cfg(test)]
    pub fn assert_invariants(&self, context: &dyn core::fmt::Debug) {
        for (words, summaries) in self.levels.iter().zip(self.levels.iter().skip(1)) {
            assert_eq!(
                summaries.len(),
                words.len().div_ceil(WORD_BITS),
                "a summary level has one bit per word below it after {context:?}"
            );
            for (index, &word) in words.iter().enumerate() {
                assert_eq!(
                    summaries[index / WORD_BITS] & bit(index) != 0,
                    word != 0,
                    "the summary bit of word {index} disagrees with the word after {context:?}"
                );
            }
        }
        if let Some(top) = self.levels.last() {
            assert_eq!(top.len(), 1, "the last level is one word after {context:?}");
        }
    }
}

/// The positions of the set bits of `word`, the word at index `word_index`,
/// in ascending order.
fn word_positions(word_index: usize, word: u64) -> impl Iterator<Item = usize> {
    let mut remaining = word;
    core::iter::from_fn(move || {
        (remaining != 0).then(|| {
            let offset = remaining.trailing_zeros() as usize;
            remaining &= remaining - 1;
            word_index * WORD_BITS + offset
        })
    })
}

/// The bit that stands for `index` within its word.
fn bit(index: usize) -> u64 {
    1 << (index % WORD_BITS)
}

/// The offset of the highest set bit of a nonzero word.
fn highest_bit(word: u64) -> usize {
    debug_assert_ne!(word, 0);
    (WORD_BITS - 1) - word.leading_zeros() as usize
}
