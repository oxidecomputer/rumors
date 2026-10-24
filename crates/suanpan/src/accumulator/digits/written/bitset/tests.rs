//! Compare the hierarchical bitset with an ordered-set oracle.
//!
//! The bitset's subtle work lies in its summary levels: climbing while a word
//! changes between empty and nonempty, growing a new level on top, clearing
//! level by level while leaving position zero, and descending from a summary
//! bit to a position.
//! Positions are therefore drawn near word and level boundaries, in runs that
//! fill whole words, and far enough apart to need several levels, rather than
//! uniformly, which would almost never exercise a climb.

use std::collections::BTreeSet;

use proptest::prelude::*;
use proptest::sample::Index;

use super::{Bitset, MAX_LEVELS, WORD_BITS};

/// Positions at and beside the boundaries where a word, a level-1 word, and a
/// level-2 word end, so that operations climb and queries cross each level.
const BOUNDARIES: [usize; 4] = [0, 64, 64 * 64, 64 * 64 * 64];

/// The bound on isolated positions, high enough to need four levels.
const FAR: usize = 1 << 19;

/// The longest run of consecutive positions one operation inserts: enough to
/// fill two whole words and part of a third.
const MAX_RUN: usize = 160;

/// Level-0 lengths, in words, at and beside each length where the levels
/// change shape: a second level appears past 1 word, a third past 64, and a
/// fourth past 4,096, and level 1 gains a word past 128 and past 4,160.
const LEVEL_0_LENGTHS: &[usize] = &[
    1, 2, 63, 64, 65, 66, 128, 129, 4095, 4096, 4097, 4098, 4160, 4161,
];

/// One operation applied to the bitset and the oracle alike.
#[derive(Clone, Debug)]
enum Op {
    /// Insert one position.
    Insert(usize),
    /// Insert every position from the first through `len - 1` above it.
    InsertRun(usize, usize),
    /// Remove one position, held or not.
    Remove(usize),
    /// Remove a position the oracle holds, chosen by index; nothing when the
    /// oracle is empty.
    RemoveHeld(Index),
    /// Insert position zero, which `clear` requires, then clear, which must
    /// pass exactly the oracle's other positions and leave position zero.
    ///
    /// The clear must examine level 0 and one more level per level at which
    /// the largest position lies outside word zero, and empty exactly the
    /// nonempty words off position zero's path.
    Clear,
}

/// A position near a word or level boundary, in the first few words, or far up.
fn position() -> impl Strategy<Value = usize> {
    prop_oneof![
        4 => 0..256_usize,
        4 => (proptest::sample::select(&BOUNDARIES[..]), -3..=3_isize)
            .prop_map(|(boundary, offset)| boundary.saturating_add_signed(offset)),
        1 => 0..FAR,
    ]
}

/// One operation, with inserts weighted above removals so that words fill.
fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        5 => position().prop_map(Op::Insert),
        2 => (position(), 1..=MAX_RUN).prop_map(|(first, len)| Op::InsertRun(first, len)),
        2 => position().prop_map(Op::Remove),
        2 => any::<Index>().prop_map(Op::RemoveHeld),
        1 => Just(Op::Clear),
    ]
}

/// Apply `op` to both sets, recording every position it names in `touched`.
fn apply(op: &Op, set: &mut Bitset, oracle: &mut BTreeSet<usize>, touched: &mut BTreeSet<usize>) {
    match *op {
        Op::Insert(position) => {
            set.insert(position);
            oracle.insert(position);
            touched.insert(position);
        }
        Op::InsertRun(first, len) => {
            for position in first..first + len {
                set.insert(position);
            }
            oracle.extend(first..first + len);
            touched.extend([first, first + len - 1]);
        }
        Op::Remove(position) => {
            set.remove(position);
            oracle.remove(&position);
            touched.insert(position);
        }
        Op::RemoveHeld(index) => {
            if !oracle.is_empty() {
                let position = *oracle
                    .iter()
                    .nth(index.index(oracle.len()))
                    .expect("an index below the length selects an element");
                set.remove(position);
                oracle.remove(&position);
                touched.insert(position);
            }
        }
        Op::Clear => {
            // `clear` requires position zero, which a removal may have taken.
            set.insert(0);
            let highest = oracle.last().copied().unwrap_or(0);
            let off_path_words = nonempty_words(set) - set.levels.len();
            set.cleared_levels_examined = 0;
            set.cleared_words_emptied = 0;
            let mut removed = Vec::new();
            set.clear(|position| removed.push(position));
            removed.sort_unstable();
            assert_eq!(
                removed,
                oracle.range(1..).copied().collect::<Vec<_>>(),
                "a clear passes each position but zero once"
            );
            assert_eq!(
                set.cleared_levels_examined,
                1 + levels_off_word_zero(highest),
                "a clear climbs only while a position remains above word zero"
            );
            assert_eq!(
                set.cleared_words_emptied, off_path_words,
                "a clear empties exactly the nonempty words off position zero's path"
            );
            touched.extend(oracle.iter().copied());
            *oracle = BTreeSet::from([0]);
        }
    }
}

/// Every position whose membership or predecessor could disagree: each named
/// or held position and its neighbors, each boundary and its neighbors, and
/// positions above everything held, up to `usize::MAX`.
fn probes(oracle: &BTreeSet<usize>, touched: &BTreeSet<usize>) -> BTreeSet<usize> {
    let above = oracle.last().map_or(0, |&last| last + 1);
    let beyond = [above, above + 64, usize::MAX];
    touched
        .iter()
        .chain(oracle)
        .chain(&BOUNDARIES)
        .chain(&beyond)
        .flat_map(|&position| {
            [
                position.saturating_sub(1),
                position,
                position.saturating_add(1),
            ]
        })
        .collect()
}

proptest! {
    /// Every sequence of insertions, removals, and clears leaves the bitset
    /// agreeing with an ordered-set oracle on every query, with both of its
    /// invariants intact.
    ///
    /// After every operation, level 0 holds exactly the oracle's positions,
    /// every summary bit is set exactly when its word is nonzero, the levels
    /// have their documented shape, and `contains` and `last_below` agree
    /// with the oracle at every probe: each position an operation named, each
    /// held position, their neighbors, the word and level boundaries, and
    /// positions above everything held.
    #[test]
    fn bitset_matches_an_ordered_set(ops in proptest::collection::vec(op(), 1..48)) {
        let mut set = Bitset::default();
        let mut oracle = BTreeSet::new();
        let mut touched = BTreeSet::new();
        for (step, op) in ops.iter().enumerate() {
            apply(op, &mut set, &mut oracle, &mut touched);
            let trace = &ops[..=step];
            set.assert_invariants(&trace);
            prop_assert_eq!(
                set.iter().collect::<Vec<_>>(),
                oracle.iter().copied().collect::<Vec<_>>(),
                "level 0 holds exactly the oracle's positions after {:?}",
                trace
            );
            for probe in probes(&oracle, &touched) {
                prop_assert_eq!(
                    set.contains(probe),
                    oracle.contains(&probe),
                    "contains({}) after {:?}",
                    probe,
                    trace
                );
                prop_assert_eq!(
                    set.last_below(probe),
                    oracle.range(..probe).next_back().copied(),
                    "last_below({}) after {:?}",
                    probe,
                    trace
                );
            }
        }
    }
}

/// Growing the levels stops at the first level already long enough, so an
/// insertion examines at most one level it leaves at its length.
///
/// Each level holds one word per 64 words of the level below, rounded up, so
/// every level above the first long-enough one is long enough too, and
/// examining it would cost a word operation that changes nothing. From no
/// levels and from every length in [`LEVEL_0_LENGTHS`], an insertion into the
/// last word of every length in it examines exactly one unchanged level when
/// some existing level keeps its length, and none when every level grows.
#[test]
fn growth_stops_at_the_first_level_long_enough() {
    let starts = core::iter::once(None).chain(LEVEL_0_LENGTHS.iter().copied().map(Some));
    for start in starts {
        for &target in LEVEL_0_LENGTHS {
            let mut set = Bitset::default();
            if let Some(start) = start {
                set.insert(start * WORD_BITS - 1);
            }
            let before: Vec<usize> = set.levels.iter().map(Vec::len).collect();
            set.unchanged_levels_examined = 0;
            set.insert(target * WORD_BITS - 1);
            let examined = set.unchanged_levels_examined;
            let after: Vec<usize> = set.levels.iter().map(Vec::len).collect();
            let kept = before.iter().zip(&after).any(|(old, new)| old == new);
            assert_eq!(
                examined,
                usize::from(kept),
                "unchanged levels examined by an insertion into word {} that takes the \
                 level lengths from {before:?} to {after:?}",
                target - 1
            );
        }
    }
}

/// [`MAX_LEVELS`] is the number of levels that growth gives a hierarchy
/// covering the largest position a set can hold.
///
/// Growth gives level 0 one word per 64 positions, and each level above one
/// word per 64 words below, rounded up, until a level of one word. Following
/// that recurrence from position `usize::MAX - 1`, the highest a digit buffer
/// could index, must count exactly the levels the constant states.
#[test]
fn max_levels_covers_every_position() {
    let mut words = (usize::MAX - 1) / WORD_BITS + 1;
    let mut levels = 1;
    while words > 1 {
        words = words.div_ceil(WORD_BITS);
        levels += 1;
    }
    assert_eq!(levels, MAX_LEVELS);
}

/// The position whose insertion gives the hierarchy four levels before each
/// clear in [`a_clear_leaves_zero_and_climbs_only_past_emptied_words`].
const TALL: usize = 64 * 64 * 64;

/// Positions at and beside the word, level-1, and level-2 boundaries, from
/// which [`a_clear_leaves_zero_and_climbs_only_past_emptied_words`] builds
/// the sets it clears and draws the insertions that follow.
const CLEAR_POINTS: [usize; 7] = [0, 1, 63, 64, 4095, 4096, TALL];

/// Count the nonempty words of `set`, at every level.
fn nonempty_words(set: &Bitset) -> usize {
    set.levels
        .iter()
        .map(|words| words.iter().filter(|&&word| word != 0).count())
        .sum()
}

/// Count the levels at which `position` lies in a word other than word zero:
/// level `k` exactly when `position` reaches `64^(k + 1)`.
fn levels_off_word_zero(position: usize) -> usize {
    core::iter::successors(Some(WORD_BITS), |span| span.checked_mul(WORD_BITS))
        .take_while(|&span| span <= position)
        .count()
}

/// A clear leaves position zero alone, climbs only past levels at which it
/// empties a word other than word zero, and empties exactly the nonempty
/// words off position zero's path.
///
/// After it, a later insertion and every query agree with the oracle.
///
/// A hierarchy of four levels is built by inserting position zero and
/// [`TALL`], and clearing. Then, for every subset of [`CLEAR_POINTS`], the
/// subset is inserted and cleared. The clear must pass exactly the subset's
/// points other than zero and leave exactly position zero, with both
/// invariants and the storage kept, having examined level 0 and one level
/// more for each level at which the largest point lies outside word zero.
/// An empty subset, or zero alone, therefore stops at level 0. Below the
/// levels it examines, it must empty exactly the words that were nonempty
/// and not on position zero's path, so it reads no empty word. Each point,
/// and the position after it, is then inserted into a copy. Because
/// word zero stays nonempty at every level, the insertion must climb one
/// level plus one for each level at which its position lies outside word
/// zero, where an insertion into an emptied hierarchy would climb every
/// level. Then `contains` and `last_below` must agree with the oracle at
/// every probe.
#[test]
fn a_clear_leaves_zero_and_climbs_only_past_emptied_words() {
    let mut tall = Bitset::default();
    tall.insert(0);
    tall.insert(TALL);
    tall.clear(|_| {});
    assert_eq!(
        tall.levels.len(),
        4,
        "a hierarchy covering {TALL} has four levels"
    );
    for mask in 0..1_u32 << CLEAR_POINTS.len() {
        let subset: Vec<usize> = (0..CLEAR_POINTS.len())
            .filter(|&index| mask & (1 << index) != 0)
            .map(|index| CLEAR_POINTS[index])
            .collect();
        let highest = subset.last().copied().unwrap_or(0);
        let mut set = tall.clone();
        for &point in &subset {
            set.insert(point);
        }
        let storage = set.storage_words();
        let off_path_words = nonempty_words(&set) - set.levels.len();
        set.cleared_levels_examined = 0;
        set.cleared_words_emptied = 0;
        let mut removed = Vec::new();
        set.clear(|position| removed.push(position));
        let context = format!("clearing {subset:?}");
        set.assert_invariants(&context);
        removed.sort_unstable();
        assert_eq!(
            removed,
            subset
                .iter()
                .copied()
                .filter(|&point| point != 0)
                .collect::<Vec<_>>(),
            "positions passed by {context}"
        );
        assert_eq!(
            set.cleared_words_emptied, off_path_words,
            "words emptied below the examined levels by {context}"
        );
        assert_eq!(
            set.iter().collect::<Vec<_>>(),
            [0],
            "position zero alone remains after {context}"
        );
        assert_eq!(
            set.storage_words(),
            storage,
            "the storage is kept after {context}"
        );
        assert_eq!(
            set.cleared_levels_examined,
            1 + levels_off_word_zero(highest),
            "levels examined by {context}"
        );
        for inserted in CLEAR_POINTS.iter().flat_map(|&point| [point, point + 1]) {
            let mut after = set.clone();
            after.inserted_levels_climbed = 0;
            after.insert(inserted);
            let oracle = BTreeSet::from([0, inserted]);
            let touched = BTreeSet::from([highest, inserted]);
            let context = format!("{context}, then inserting {inserted}");
            after.assert_invariants(&context);
            assert_eq!(
                after.inserted_levels_climbed,
                1 + levels_off_word_zero(inserted),
                "levels climbed by {context}"
            );
            assert_eq!(
                after.iter().collect::<Vec<_>>(),
                oracle.iter().copied().collect::<Vec<_>>(),
                "level 0 holds exactly the oracle's positions after {context}"
            );
            for probe in probes(&oracle, &touched) {
                assert_eq!(
                    after.contains(probe),
                    oracle.contains(&probe),
                    "contains({probe}) after {context}"
                );
                assert_eq!(
                    after.last_below(probe),
                    oracle.range(..probe).next_back().copied(),
                    "last_below({probe}) after {context}"
                );
            }
        }
    }
}
