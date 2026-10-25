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
//! - [`clear`](Bitset::clear) costs O(1) plus one word operation per word it
//!   empties and per position it removes, never the levels' width: see its
//!   documentation.
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

/// The most levels a hierarchy can have: enough for every `usize` position.
///
/// Each level divides the positions by 64, so covering positions below
/// `2^usize::BITS` takes `usize::BITS / 6` levels, rounded up.
pub const MAX_LEVELS: usize = usize::BITS.div_ceil(WORD_BITS.ilog2()) as usize;

/// A set of positions as a 64-ary hierarchy of bit words.
#[derive(Clone, Default)]
pub struct Bitset {
    /// `levels[0]` holds one bit per position; each later level holds the
    /// summary bits of the level before it.
    ///
    /// The levels are a boxed slice rather than a `Vec`, so that the count
    /// below fits in the word a `Vec`'s capacity would take, and a bitset
    /// stays three words. Only [`cover`](Self::cover) changes their number,
    /// by one level at a time, at most [`MAX_LEVELS`] times.
    levels: Box<[Vec<u64>]>,
    /// The number of positions the set holds.
    len: usize,
    /// The levels [`cover`](Self::cover) has examined and left at their
    /// length, counted so that tests can check where growth stops.
    #[cfg(test)]
    unchanged_levels_examined: usize,
    /// The levels [`clear`](Self::clear) has examined, counted so that tests
    /// can check where its climb stops.
    #[cfg(test)]
    cleared_levels_examined: usize,
    /// The words [`clear`](Self::clear) has emptied below the levels it
    /// examined, counted so that tests can check that it reads no empty word.
    #[cfg(test)]
    cleared_words_emptied: usize,
    /// The levels [`insert`](Self::insert) has climbed, counted so that
    /// tests can check where an insertion stops.
    #[cfg(test)]
    inserted_levels_climbed: usize,
    /// The calls of [`insert`](Self::insert), counted so that tests can
    /// separate an insertion's one level from the words it fills.
    #[cfg(test)]
    insertions: usize,
}

// A bitset outside tests is its levels and its count, three words, so the
// count adds nothing to the boxed bitset a written set allocates.
#[cfg(not(test))]
const _: () = assert!(core::mem::size_of::<Bitset>() == 3 * core::mem::size_of::<usize>());

/// A snapshot of a bitset's test-only work counters.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClimbCounts {
    /// Calls of [`Bitset::insert`].
    pub insertions: usize,
    /// Levels those insertions climbed, each counting the level it stopped
    /// at, so the climbs exceed the insertions by the words they filled.
    pub inserted_levels_climbed: usize,
    /// Levels [`Bitset::clear`] examined.
    pub cleared_levels_examined: usize,
    /// Levels the bitset holds now.
    pub levels: usize,
}

/// Maintain positions and their summary bits, climbing only where a word
/// changes between empty and nonempty.
impl Bitset {
    /// Add `position`, growing the levels to cover it.
    pub fn insert(&mut self, position: usize) {
        #[cfg(test)]
        {
            self.insertions += 1;
        }
        self.cover(position);
        let mut index = position;
        for (level, words) in self.levels.iter_mut().enumerate() {
            #[cfg(test)]
            {
                self.inserted_levels_climbed += 1;
            }
            let word = &mut words[index / WORD_BITS];
            let held = *word;
            *word |= bit(index);
            // At level 0 a bit is a position, so the word read here also
            // tells whether the set gained one.
            if level == 0 && held & bit(index) == 0 {
                self.len += 1;
            }
            // A word that already held a bit is already summarized above.
            if held != 0 {
                return;
            }
            index /= WORD_BITS;
        }
    }

    /// Remove `position`; removing an absent position does nothing.
    pub fn remove(&mut self, position: usize) {
        let mut index = position;
        for (level, words) in self.levels.iter_mut().enumerate() {
            let Some(word) = words.get_mut(index / WORD_BITS) else {
                return;
            };
            if *word & bit(index) == 0 {
                return;
            }
            *word &= !bit(index);
            // At level 0 a bit is a position, and it was held.
            if level == 0 {
                self.len -= 1;
            }
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

    /// Reduce the set to position zero, which it must hold, keeping the
    /// storage, and pass every other position it held to `removed`.
    ///
    /// A set without position zero fails the assertion in a debug build; in a
    /// release build the clear then leaves the summary invariant broken, and
    /// every later operation may return a wrong result or panic. The written
    /// set clears its bitset only when the bitset holds zero, as
    /// [`Below`](super::Below) documents.
    ///
    /// Position zero stays, so word zero stays nonempty at every level, and
    /// an insertion after the clear stops at the first word it shares with
    /// position zero, as it would before the clear, instead of climbing
    /// every level of an emptied hierarchy.
    ///
    /// The clear works upward from level 0, one level per step. At level
    /// `k`, word zero holds the bit on position zero's path, and other bits
    /// that stand for words of level `k - 1` or, at level 0, for positions 1
    /// to 63. The step empties each of those words and everything below
    /// them, and leaves word zero that one bit. After the step at level `k`,
    /// the set holds no position other than zero below `64^(k + 1)`, so the
    /// clear climbs to the next level only while it holds one at or above.
    /// That position lies under word zero of some higher level, so the climb
    /// ends by the last level, whose one word covers every position.
    ///
    /// # Cost
    ///
    /// The clear reads word zero once at each level it examines, reads each
    /// word below them that it empties once, and passes each removed position
    /// once; it enters no empty word. It climbs past level `k` only while a
    /// position at or above `64^(k + 1)` remains, which lies in a word other
    /// than word zero at level `k`, and the clear empties that word before it
    /// returns. So every word operation is charged to a word the clear empties
    /// or to a position it removes, and the insertion that filled that word,
    /// or inserted that position, paid for it. A clear therefore costs O(1)
    /// plus amortized O(1) per position it removes, however wide the levels.
    ///
    /// The charge rests on two facts about how a word other than word zero
    /// turns nonempty. Growth never does it: it appends zero words, and a new
    /// top level summarizes a level whose only nonempty word is word zero,
    /// because that level was the one-word top before the growth began, or
    /// was itself created by it, so only the new level's word zero gains a
    /// bit. And a clone copies words filled in the original's history; the
    /// clone's copy, one word operation per word, pays for them.
    pub fn clear(&mut self, mut removed: impl FnMut(usize)) {
        debug_assert!(
            self.levels
                .first()
                .is_some_and(|words| words[0] & bit(0) != 0),
            "clear's caller guarantees that the set holds position zero"
        );
        let mut level = 0;
        loop {
            #[cfg(test)]
            {
                self.cleared_levels_examined += 1;
            }
            let others = core::mem::replace(&mut self.levels[level][0], bit(0)) & !bit(0);
            for offset in word_positions(0, others) {
                if level == 0 {
                    removed(offset);
                    self.len -= 1;
                } else {
                    let emptied = self.empty_subtree(level - 1, offset, &mut removed);
                    self.len -= emptied;
                }
            }
            if self.len == 1 {
                return;
            }
            level += 1;
        }
    }

    /// Empty word `index` of `level` and every word below it, passing each
    /// position they held to `removed`, and return how many there were.
    ///
    /// The walk is depth-first and keeps one frame per level: a word's index
    /// and the bits of it not yet descended into. It empties each word as it
    /// enters it, and enters only words whose summary bit was set, so it
    /// reads each nonempty word below once and no empty word. Word `index`
    /// must not lie on position zero's path, which the caller keeps.
    fn empty_subtree(
        &mut self,
        level: usize,
        index: usize,
        removed: &mut impl FnMut(usize),
    ) -> usize {
        let mut count = 0;
        let mut frames = [(0_usize, 0_u64); MAX_LEVELS];
        frames[0] = (index, self.take_word(level, index));
        let mut depth = 0;
        loop {
            let word_level = level - depth;
            let (word_index, bits) = &mut frames[depth];
            if *bits == 0 {
                if depth == 0 {
                    return count;
                }
                depth -= 1;
                continue;
            }
            let child = *word_index * WORD_BITS + bits.trailing_zeros() as usize;
            *bits &= *bits - 1;
            if word_level == 0 {
                removed(child);
                count += 1;
            } else {
                depth += 1;
                frames[depth] = (child, self.take_word(word_level - 1, child));
            }
        }
    }

    /// Empty word `index` of `level` and return what it held.
    fn take_word(&mut self, level: usize, index: usize) -> u64 {
        #[cfg(test)]
        {
            self.cleared_words_emptied += 1;
        }
        core::mem::take(&mut self.levels[level][index])
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

    /// Read the test-only work counters.
    #[cfg(test)]
    pub fn climb_counts(&self) -> ClimbCounts {
        ClimbCounts {
            insertions: self.insertions,
            inserted_levels_climbed: self.inserted_levels_climbed,
            cleared_levels_examined: self.cleared_levels_examined,
            levels: self.levels.len(),
        }
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
                // The levels grow by exactly one level at a time, rather than
                // reserving room for several they may never need, so the
                // boxed slice reuses the allocation `reserve_exact` makes.
                let mut levels = core::mem::take(&mut self.levels).into_vec();
                levels.reserve_exact(1);
                levels.push(words);
                self.levels = levels.into_boxed_slice();
            }
            if needed == 1 {
                break;
            }
            needed = needed.div_ceil(WORD_BITS);
        }
    }

    /// Check both invariants of the module, that each summary bit is set
    /// exactly when its word is nonzero and that the levels have the
    /// documented shape, and that the count of held positions is exact.
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
        assert!(
            self.levels.len() <= MAX_LEVELS,
            "{} levels exceed the most any position needs after {context:?}",
            self.levels.len()
        );
        assert_eq!(
            self.len,
            self.iter().count(),
            "the count of held positions disagrees with level 0 after {context:?}"
        );
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
