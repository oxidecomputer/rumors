//! Compare the written-position set with an ordered-set oracle.
//!
//! The set's subtle work lies where its top run meets the positions below it
//! (extending the run, moving it down on a jump, shortening or dropping it on
//! a truncation, parking it above what remains), and where the inline prefix
//! gives way to a bitset. Positions are drawn near the bitset's word and level
//! boundaries, in spans that extend the top run and fill whole words, and far
//! enough apart to need several levels, so that the set's results also cross
//! the bitset's own boundaries; the bitset's own model test covers those
//! boundaries directly.

use std::collections::BTreeSet;

use proptest::prelude::*;
use proptest::sample::Index;

use super::WrittenPositions;

/// Positions at and beside the boundaries where a word, a level-1 word, and a
/// level-2 word end, so that inserts climb and queries cross each level.
const BOUNDARIES: [usize; 4] = [0, 64, 64 * 64, 64 * 64 * 64];

/// The bound on isolated positions, high enough to need four levels.
const FAR: usize = 1 << 19;

/// The longest span of consecutive positions one operation inserts: enough to
/// fill two whole words and part of a third.
const MAX_SPAN: usize = 160;

/// One operation applied to the hierarchy and the oracle alike.
#[derive(Clone, Debug)]
enum Op {
    /// Insert one position.
    Insert(usize),
    /// Insert every position from the first through `len - 1` above it, as
    /// one carry chain does.
    InsertSpan(usize, usize),
    /// Remove every position above this one.
    Truncate(usize),
    /// Remove every position above one the oracle holds, chosen by index;
    /// nothing when the oracle is empty.
    TruncateToHeld(Index),
    /// Remove the largest position alone, as a descent of one step does.
    TruncateBelowLargest,
    /// Clear from the oracle's least position other than zero, less the first
    /// slack, through its largest position, plus the second, as `Digits`
    /// clears from its floor through its top.
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

/// One operation, with inserts weighted above truncations so that words fill.
fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        5 => position().prop_map(Op::Insert),
        2 => (position(), 1..=MAX_SPAN).prop_map(|(first, len)| Op::InsertSpan(first, len)),
        1 => position().prop_map(Op::Truncate),
        1 => any::<Index>().prop_map(Op::TruncateToHeld),
        2 => Just(Op::TruncateBelowLargest),
        1 => (0..=130_usize, 0..=130_usize).prop_map(|(below, above)| Op::Clear(below, above)),
    ]
}

/// Apply a truncation to `highest` to both sets.
fn truncate(
    highest: usize,
    set: &mut WrittenPositions,
    oracle: &mut BTreeSet<usize>,
    touched: &mut BTreeSet<usize>,
) {
    set.truncate(highest);
    oracle.retain(|&position| position <= highest);
    touched.insert(highest);
}

/// Apply `op` to both sets, recording every position it names in `touched`.
fn apply(
    op: &Op,
    set: &mut WrittenPositions,
    oracle: &mut BTreeSet<usize>,
    touched: &mut BTreeSet<usize>,
) {
    match *op {
        Op::Insert(position) => {
            set.insert(position);
            oracle.insert(position);
            touched.insert(position);
        }
        Op::InsertSpan(first, len) => {
            let last = first + len - 1;
            set.insert_span(first, last);
            oracle.extend(first..=last);
            touched.extend([first, last]);
        }
        Op::Truncate(highest) => truncate(highest, set, oracle, touched),
        Op::TruncateToHeld(index) => {
            if !oracle.is_empty() {
                let highest = *oracle
                    .iter()
                    .nth(index.index(oracle.len()))
                    .expect("an index below the length selects an element");
                truncate(highest, set, oracle, touched);
            }
        }
        Op::TruncateBelowLargest => {
            if let Some(&largest) = oracle.last() {
                truncate(largest.saturating_sub(1), set, oracle, touched);
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

/// Panic unless the hierarchy and the oracle agree on every query.
fn assert_agrees(
    set: &WrittenPositions,
    oracle: &BTreeSet<usize>,
    touched: &BTreeSet<usize>,
    trace: &[Op],
) {
    set.assert_structure(&trace);
    assert_eq!(
        set.positions(),
        oracle.iter().copied().collect::<Vec<_>>(),
        "the run, the prefix, and the level-0 bits hold exactly the oracle's positions after \
         {trace:?}"
    );
    for probe in probes(oracle, touched) {
        assert_eq!(
            set.contains(probe),
            oracle.contains(&probe),
            "contains({probe}) after {trace:?}"
        );
        assert_eq!(
            set.last_below(probe),
            oracle.range(..probe).next_back().copied(),
            "last_below({probe}) after {trace:?}"
        );
    }
}

proptest! {
    /// Every sequence of inserts, truncations, and clears leaves the set
    /// agreeing with an ordered-set oracle on every query.
    ///
    /// After every operation the set holds exactly the oracle's positions,
    /// its run lies above every other position, any bitset keeps its own
    /// invariants, and `contains` and `last_below` agree with the oracle at
    /// every probe: each position an operation named, each held position,
    /// their neighbors, the bitset's word and level boundaries, and positions
    /// above everything held.
    #[test]
    fn written_positions_match_an_ordered_set(
        ops in proptest::collection::vec(op(), 1..48),
    ) {
        let mut set = WrittenPositions::default();
        let mut oracle = BTreeSet::new();
        let mut touched = BTreeSet::new();
        assert_agrees(&set, &oracle, &touched, &[]);
        for (step, op) in ops.iter().enumerate() {
            apply(op, &mut set, &mut oracle, &mut touched);
            assert_agrees(&set, &oracle, &touched, &ops[..=step]);
        }
    }
}

/// One operation after a value has been written once at a shift.
struct AfterOneGap {
    /// What the operation does, for failure messages.
    name: &'static str,
    /// The span it writes, given the prefix length and the run's position.
    span: fn(usize, usize) -> (usize, usize),
    /// The form expected to hold the positions below the run afterward.
    form: &'static str,
}

/// Each way an operation can meet a set holding one gap.
const AFTER_ONE_GAP: [AfterOneGap; 6] = [
    // A carry or stream extending the run keeps one gap.
    AfterOneGap {
        name: "extend the run",
        span: |_, run| (run + 1, run + 2),
        form: "prefix",
    },
    // A write continuing the prefix narrows the gap without splitting it.
    AfterOneGap {
        name: "continue the prefix",
        span: |prefix, _| (prefix, prefix),
        form: "prefix",
    },
    // A write overlapping the prefix and stopping in the gap still continues it.
    AfterOneGap {
        name: "overlap the prefix",
        span: |prefix, _| (prefix - 1, prefix),
        form: "prefix",
    },
    // A write in the gap that reaches the run narrows the gap from above.
    AfterOneGap {
        name: "shorten the gap from above",
        span: |_, run| (run - 1, run - 1),
        form: "prefix",
    },
    // A write in the gap apart from both the prefix and the run opens a
    // second gap.
    AfterOneGap {
        name: "split the gap",
        span: |prefix, _| (prefix + 1, prefix + 1),
        form: "bitset",
    },
    // A second jump leaves a gap below each run.
    AfterOneGap {
        name: "jump again",
        span: |_, run| (run + 3, run + 3),
        form: "bitset",
    },
];

/// A set written once at a shift holds its positions below the run as a
/// prefix, and moves them into a bitset exactly when an operation opens a
/// second gap.
///
/// Every prefix length from one to three and every jump of three to five
/// positions above it, so that the gap has room for a position apart from
/// both sides, is followed by each operation of [`AFTER_ONE_GAP`]: extending
/// the run, continuing the prefix, or shortening the gap from above keeps the
/// prefix, while splitting the gap or jumping again converts it to a bitset.
/// The positions match the oracle throughout.
#[test]
fn a_second_gap_and_only_a_second_gap_converts_the_prefix() {
    for prefix in 1..=3 {
        for run in prefix + 3..=prefix + 5 {
            for AfterOneGap { name, span, form } in AFTER_ONE_GAP {
                let mut set = WrittenPositions::default();
                let mut oracle = BTreeSet::new();
                set.insert_span(0, prefix - 1);
                set.insert(run);
                oracle.extend((0..prefix).chain([run]));
                assert_eq!(
                    set.form(),
                    "prefix",
                    "a prefix of {prefix} below a run at {run} is one gap, held as a prefix"
                );
                let (first, last) = span(prefix, run);
                set.insert_span(first, last);
                oracle.extend(first..=last);
                let context = format!("{name} with prefix {prefix} and run at {run}");
                assert_eq!(set.form(), form, "form after: {context}");
                set.assert_structure(&context);
                assert_eq!(
                    set.positions(),
                    oracle.iter().copied().collect::<Vec<_>>(),
                    "positions after: {context}"
                );
            }
        }
    }
}

/// Converting a prefix of any length moves every one of its positions into
/// the bitset.
///
/// Every prefix length `n` in {1, 63, 64, 65, 130}, below a run far above it,
/// is followed by a write at each `p` from `n` through `n + 70`: `p = n`
/// continues the prefix, and every other `p` opens a second gap and converts
/// it. The lengths straddle the bitset's word boundary at 64 and span two
/// words, and the positions must match the oracle after every write.
#[test]
fn converting_a_prefix_keeps_every_position() {
    const RUN: usize = 1_000;
    for prefix in [1, 63, 64, 65, 130] {
        for p in prefix..=prefix + 70 {
            let mut set = WrittenPositions::default();
            let mut oracle = BTreeSet::new();
            set.insert_span(0, prefix - 1);
            set.insert(RUN);
            oracle.extend((0..prefix).chain([RUN]));
            set.insert(p);
            oracle.insert(p);
            let expected = if p == prefix { "prefix" } else { "bitset" };
            let context = format!("prefix {prefix}, then a write at {p}");
            assert_eq!(set.form(), expected, "form after: {context}");
            set.assert_structure(&context);
            assert_eq!(
                set.positions(),
                oracle.iter().copied().collect::<Vec<_>>(),
                "positions after: {context}"
            );
        }
    }
}
