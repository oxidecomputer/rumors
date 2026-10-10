//! Tests for the frozen-reference and minimum-record machinery.

use num_bigint::BigUint;
use proptest::prelude::*;

use super::heights::{MAX_BURIED, MERGES_TO_NEWER, MERGES_TO_OLDER, RECORD_FREEZES, WIDE_CHUNKS};
use crate::testing::bridge::from_oracle_version;
use crate::testing::oracles::tree::Version as Tree;

/// A leaf height from one of four width bands: small, a little past the
/// record allowance, around one machine word, and far past it.
fn arb_height() -> impl Strategy<Value = BigUint> {
    let one = || BigUint::from(1u8);
    prop_oneof![
        (0u64..8).prop_map(BigUint::from),
        (0u64..8, 5usize..9).prop_map(move |(s, e)| (one() << e) + s),
        (0u64..8, 62usize..70).prop_map(move |(s, e)| (one() << e) + s),
        (0u64..8, 120usize..200).prop_map(move |(s, e)| (one() << e) + s),
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
