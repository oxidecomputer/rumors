//! Tests for the frozen-reference and minimum-record machinery.

use num_bigint::BigUint;
use proptest::prelude::*;

use super::heights::{MAX_BURIED, MERGES_TO_NEWER, MERGES_TO_OLDER, RECORD_FREEZES, WIDE_CHUNKS};
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
fn arb_tree() -> impl Strategy<Value = Tree> {
    arb_height()
        .prop_map(Tree::leaf)
        .prop_recursive(8, 64, 2, |inner| {
            (inner.clone(), inner).prop_map(|(l, r)| Tree::node(0u64, l, r))
        })
}

/// Read the reference-path counters on this thread.
fn counters() -> [u64; 4] {
    [
        RECORD_FREEZES.with(|c| c.get()),
        MERGES_TO_NEWER.with(|c| c.get()),
        MERGES_TO_OLDER.with(|c| c.get()),
        WIDE_CHUNKS.with(|c| c.get()),
    ]
}

/// `min_ticks` equals the tree oracle's base total on trees mixing narrow and
/// wide heights.
///
/// The run also reaches every reference path: record-trigger freezes, merges
/// charged to each neighbour, wide chunks moved whole, and more than one
/// reference buried at once.
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
