//! Tests for the frozen-reference and minimum-record machinery.

use num_bigint::BigUint;
use proptest::prelude::*;

use super::heights::{
    MAX_BURIED, MAX_READ_EXCESS, MERGES_TO_NEWER, MERGES_TO_OLDER, NEGATIVE_STACKED,
    NEGATIVE_UNSTACKED, RECORD_FREEZES, UNREAD_FREEZES, UNREAD_FREEZE_BITS, WIDE_CHUNKS,
};
use crate::testing::bridge::from_oracle_version;
use crate::testing::oracles::tree::Version as Tree;

/// A leaf height from one of four width bands.
///
/// The bands are small, a little past the record allowance, around one
/// machine word, and on either side of
/// [`INLINE_CHUNK_BITS`](super::heights::INLINE_CHUNK_BITS), where a buried
/// chunk stops being copied.
fn arb_height() -> impl Strategy<Value = BigUint> {
    let one = || BigUint::from(1u8);
    prop_oneof![
        (0u64..8).prop_map(BigUint::from),
        (0u64..8, 5usize..9).prop_map(move |(s, e)| (one() << e) + s),
        (0u64..8, 62usize..70).prop_map(move |(s, e)| (one() << e) + s),
        (0u64..8, 240usize..340).prop_map(move |(s, e)| (one() << e) + s),
    ]
}

/// Event trees of up to 64 nodes whose leaves draw from [`arb_height`].
fn arb_random_tree() -> impl Strategy<Value = Tree> {
    arb_height()
        .prop_map(Tree::leaf)
        .prop_recursive(8, 64, 2, |inner| {
            (inner.clone(), inner).prop_map(|(l, r)| Tree::node(0u64, l, r))
        })
}

/// Random event trees, a quarter of them followed by
/// [`negative_coefficient_tree`], which random trees almost never reach.
fn arb_tree() -> impl Strategy<Value = Tree> {
    prop_oneof![
        3 => arb_random_tree(),
        1 => (arb_random_tree(), 3usize..8, 5usize..300).prop_map(|(prefix, closes, lift)| {
            Tree::node(0u64, prefix, negative_coefficient_tree(closes, lift))
        }),
    ]
}

/// A tree whose reference above the bottom one goes negative, is stacked
/// below the top, and is popped again.
///
/// With `W = 2^lift` and `i = W + 3`, the leaves are `0, W, W + 1`, then
/// `i`, `i + 64`, `i + 65`, `closes − 1` more leaves at `i + 65`, then `W + 2`,
/// `W + 2 + 2^20`, `W + 3 + 2^20`. The step to `W + 1` freezes a reference
/// `R`, which keeps the records of `W + 1`, `i`, and `i + 64`: three leaves.
/// The step to `i + 65` freezes again, burying `R`. The record of `i` then
/// settles with `closes` closes, since `i` is the minimum of a left chain
/// of `closes` nodes, so `R`'s coefficient falls to `2 − closes`; the chain's
/// later leaves sit level with `i + 65`, so no freeze in between merges
/// leaves back into `R`'s count. The leaf
/// `W + 2` undercuts it and records at the newer top, and the step to
/// `W + 3 + 2^20` freezes once more, stacking `R`'s negative coefficient;
/// settling `W + 2`'s record releases that top and pops it.
///
/// # Panics
///
/// Panics if `closes < 3`, where the coefficient stays nonnegative, or if
/// `lift < 5`, where the step to `W + 1` does not freeze.
fn negative_coefficient_tree(closes: usize, lift: usize) -> Tree {
    assert!(closes >= 3 && lift >= 5);
    let wide = BigUint::from(1u8) << lift;
    let i = &wide + 3u8;
    let jump = BigUint::from(1u8) << 20usize;
    let mut chain = Tree::node(
        0u64,
        Tree::leaf(i.clone()),
        Tree::node(0u64, Tree::leaf(&i + 64u8), Tree::leaf(&i + 65u8)),
    );
    for _ in 1..closes {
        chain = Tree::node(0u64, chain, Tree::leaf(&i + 65u8));
    }
    let undercut = &wide + 2u8;
    let after = Tree::node(
        0u64,
        Tree::leaf(undercut.clone()),
        Tree::node(
            0u64,
            Tree::leaf(&undercut + &jump),
            Tree::leaf(&undercut + &jump + 1u8),
        ),
    );
    let outer = Tree::node(
        0u64,
        Tree::leaf(&wide + 1u8),
        Tree::node(0u64, chain, after),
    );
    Tree::node(
        0u64,
        Tree::leaf(BigUint::ZERO),
        Tree::node(0u64, Tree::leaf(wide), outer),
    )
}

/// `min_ticks` equals the tree oracle when a reference above the bottom one
/// stacks a negative coefficient below the top and later pops it.
///
/// The coefficient stack stores signed values as zigzag codes; this shape is
/// the one that stores and restores a negative value, at every close count
/// and lift.
#[test]
fn a_negative_stacked_coefficient_settles_exactly() {
    proptest!(|(closes in 3usize..24, lift in 5usize..400)| {
        let stacked = NEGATIVE_STACKED.with(|c| c.get());
        let unstacked = NEGATIVE_UNSTACKED.with(|c| c.get());
        let tree = negative_coefficient_tree(closes, lift);
        prop_assert_eq!(from_oracle_version(&tree).min_ticks(), tree.min_ticks());
        prop_assert!(NEGATIVE_STACKED.with(|c| c.get()) > stacked, "no negative coefficient was stacked");
        prop_assert!(NEGATIVE_UNSTACKED.with(|c| c.get()) > unstacked, "no negative coefficient was popped");
    });
}

/// Read the reference-path counters on this thread.
fn counters() -> [u64; 6] {
    [
        RECORD_FREEZES.with(|c| c.get()),
        MERGES_TO_NEWER.with(|c| c.get()),
        MERGES_TO_OLDER.with(|c| c.get()),
        WIDE_CHUNKS.with(|c| c.get()),
        NEGATIVE_STACKED.with(|c| c.get()),
        NEGATIVE_UNSTACKED.with(|c| c.get()),
    ]
}

/// `min_ticks` equals the tree oracle's base total on trees mixing narrow and
/// wide heights.
///
/// The run also reaches every reference path: record-trigger freezes, merges
/// charged to each neighbour, wide chunks moved whole, a negative coefficient
/// stacked below the top and popped again, and more than one reference buried
/// at once.
#[test]
fn min_ticks_matches_the_tree_oracle_across_reference_paths() {
    let before = counters();
    MAX_BURIED.with(|c| c.set(0));
    proptest!(ProptestConfig::with_cases(512), |(tree in arb_tree())| {
        let version = from_oracle_version(&tree);
        prop_assert_eq!(version.min_ticks(), tree.min_ticks());
    });
    let after = counters();
    for (name, (b, a)) in [
        "record freezes",
        "merges to newer",
        "merges to older",
        "wide chunks",
        "a stacked negative coefficient",
        "a popped negative coefficient",
    ]
    .iter()
    .zip(before.iter().zip(after.iter()))
    {
        assert!(a > b, "the generator never reached {name}");
    }
    assert!(
        MAX_BURIED.with(|c| c.get()) >= 2,
        "the generator never buried two references at once"
    );
}

/// The board's wide-step pair spine, at the step width the board samples,
/// buries a chunk in the whole-accumulator form at every second level past
/// the first pair, and `min_ticks` still equals the tree oracle there.
///
/// The board samples this family at that step so that the one width
/// threshold among the references, where copying a buried chunk gives way
/// to moving it whole, is crossed on every sampled size, and so that every
/// record offset at a wide level is stored on its own rather than packed.
#[test]
fn wide_step_pairs_bury_whole_chunks_at_every_second_level() {
    use super::heights::INLINE_CHUNK_BITS;
    use super::integers::PACKED_BITS;
    use crate::testing::bridge::to_oracle_version;
    use crate::testing::meter::board::WIDE_STEP_PAIRS_STEP_BITS;
    use crate::testing::meter::registry::Shape;

    let step_bits = WIDE_STEP_PAIRS_STEP_BITS as u64;
    assert!(
        step_bits > INLINE_CHUNK_BITS,
        "the board's {step_bits}-bit step no longer buries chunks whole"
    );
    assert!(
        step_bits > PACKED_BITS,
        "the board's {step_bits}-bit step no longer stores wide record offsets on their own"
    );
    for d in [4usize, 16, 64, 129] {
        let version = Shape::WideStepPairs
            .build2(WIDE_STEP_PAIRS_STEP_BITS, d)
            .version();
        let before = WIDE_CHUNKS.with(|c| c.get());
        let ticks = version.min_ticks();
        let buried = WIDE_CHUNKS.with(|c| c.get()) - before;
        assert_eq!(ticks, to_oracle_version(&version).min_ticks());
        assert!(
            buried as usize >= d / 2 - 1,
            "WP({step_bits}, {d}) buried {buried} whole chunks, fewer than one per pair past the first"
        );
    }
}

/// No read of the live change pays for working width more than
/// [`UNREAD_FREEZE_BITS`] beyond its leaf's change, on a spine where that
/// condition holds at every second level.
///
/// On the wide-step pair spine at a `2^320` step, each one-tick level follows
/// a 321-bit step, so the live change is far wider than the one-tick change
/// and freezes. Freezing it unread saves reading the drift that the freeze
/// moves whole; a fold that read it first would pass the excess bound here.
#[test]
fn a_live_change_far_wider_than_its_delta_freezes_unread() {
    use crate::testing::meter::registry::Shape;

    let d = 128;
    let version = Shape::WideStepPairs.build2(320, d).version();
    let unread_before = UNREAD_FREEZES.with(|c| c.get());
    MAX_READ_EXCESS.with(|c| c.set(0));
    let _ = version.min_ticks();
    let unread = UNREAD_FREEZES.with(|c| c.get()) - unread_before;
    assert!(
        unread as usize + 1 >= d / 2,
        "WP(320, {d}) froze {unread} live changes unread, fewer than one per pair past the first"
    );
    assert!(
        MAX_READ_EXCESS.with(|c| c.get()) <= UNREAD_FREEZE_BITS,
        "a read paid for {} bits of working width beyond its change",
        MAX_READ_EXCESS.with(|c| c.get())
    );
}
