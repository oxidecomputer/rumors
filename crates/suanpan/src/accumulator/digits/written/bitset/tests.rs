//! Compare the hierarchical bitset with an ordered-set oracle.
//!
//! The bitset's subtle work lies in its summary levels: climbing while a word
//! changes between empty and nonempty, growing a new level on top, clearing a
//! span at every level, and descending from a summary bit to a position.
//! Positions are therefore drawn near word and level boundaries, in runs that
//! fill whole words, and far enough apart to need several levels, rather than
//! uniformly, which would almost never exercise a climb.

use std::collections::BTreeSet;

use proptest::prelude::*;
use proptest::sample::Index;

use super::Bitset;

/// Positions at and beside the boundaries where a word, a level-1 word, and a
/// level-2 word end, so that operations climb and queries cross each level.
const BOUNDARIES: [usize; 4] = [0, 64, 64 * 64, 64 * 64 * 64];

/// The bound on isolated positions, high enough to need four levels.
const FAR: usize = 1 << 19;

/// The longest run of consecutive positions one operation inserts: enough to
/// fill two whole words and part of a third.
const MAX_RUN: usize = 160;

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
    /// Clear from the oracle's least position other than zero, less the first
    /// slack, through its largest position, plus the second, which satisfies
    /// `clear`'s precondition.
    Clear(usize, usize),
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
        1 => (0..=130_usize, 0..=130_usize).prop_map(|(below, above)| Op::Clear(below, above)),
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
        Op::Clear(below, above) => {
            let highest = oracle.last().copied().unwrap_or(0) + above;
            let lowest = oracle
                .range(1..)
                .next()
                .map_or(highest, |&least| least.saturating_sub(below));
            set.clear(lowest, highest);
            oracle.clear();
            touched.extend([lowest, highest]);
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
