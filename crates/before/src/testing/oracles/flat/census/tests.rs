//! Calibration of the census's view against the flat model.
//!
//! The census trusts two readings of a pool: [`Grid`]'s height vectors on
//! the common refinement, and [`Grid::order_view`]'s reduction of them. Each
//! property here compares a reading with the flat model's own answers on
//! deep pools, whose heights reach thousands of bits. The census's exactness
//! argument itself is checked against a brute force in `causally`'s census
//! tests.

use proptest::prelude::*;

use super::Grid;
use crate::testing::generators::leaf_list::{arb_deep_version_triple, THREE_VERSION_DIVISOR};
use crate::testing::oracles::flat::Version;
use crate::testing::property::divided_proptest;
use crate::testing::shape_rows::Leaf;

/// A deep triple as flat versions, with every pairwise join and meet the
/// flat model computes, inputs first.
fn closed_pool((a, b, c): (Version, Version, Version)) -> Vec<Version> {
    let inputs = [a, b, c];
    let mut pool = inputs.to_vec();
    for (i, x) in inputs.iter().enumerate() {
        for y in &inputs[i + 1..] {
            pool.push(x.join(y));
            pool.push(x.meet(y));
        }
    }
    pool
}

/// Draws a deep triple as flat versions.
fn arb_flat_triple() -> impl Strategy<Value = (Version, Version, Version)> {
    arb_deep_version_triple().prop_map(|(a, b, c)| {
        (
            Version::from_rows(a.leaves()),
            Version::from_rows(b.leaves()),
            Version::from_rows(c.leaves()),
        )
    })
}

divided_proptest! {
    #![divisor(THREE_VERSION_DIVISOR)]

    /// Each pool version's height vector on the common refinement, read as
    /// rows, canonicalizes back to that version.
    ///
    /// The pools are deep triples to depth 300 with their pairwise joins and
    /// meets, so the refinement mixes every input's leaves.
    #[test]
    fn grid_vectors_rebuild_every_pool_version(triple in arb_flat_triple()) {
        let pool = closed_pool(triple);
        let grid = Grid::new(&pool);
        for (i, version) in pool.iter().enumerate() {
            let rows: Vec<Leaf> = grid
                .depths()
                .iter()
                .zip(grid.heights(i))
                .map(|(&depth, height)| Leaf { depth, height: height.clone() })
                .collect();
            prop_assert_eq!(&Version::from_rows(&rows), version, "pool version {}", i);
        }
    }

    /// The order view of a pool compares, equates, joins, and meets its
    /// vectors exactly as the flat model compares, equates, joins, and meets
    /// the versions.
    ///
    /// The pool holds a deep triple and every pairwise join and meet, so for
    /// each pair of inputs the view's join and meet can be compared with the
    /// view of the model's own join and meet. Heights reach thousands of
    /// bits, which the view replaces by ranks.
    #[test]
    fn order_view_keeps_the_flat_order_and_lattice(triple in arb_flat_triple()) {
        let pool = closed_pool(triple);
        let view = Grid::new(&pool).order_view();
        for (i, x) in pool.iter().enumerate() {
            for (j, y) in pool.iter().enumerate() {
                prop_assert_eq!(view[i].le(&view[j]), x.le(y), "order of {} and {}", i, j);
                prop_assert_eq!(view[i] == view[j], x == y, "equality of {} and {}", i, j);
            }
        }
        // `closed_pool` lists each input pair's join and then its meet after
        // the inputs, pairs in lexicographic order.
        let inputs = 3;
        let mut join = inputs;
        for i in 0..inputs {
            for j in i + 1..inputs {
                prop_assert_eq!(&view[i].join(&view[j]), &view[join], "join of {} and {}", i, j);
                prop_assert_eq!(&view[i].meet(&view[j]), &view[join + 1], "meet of {} and {}", i, j);
                join += 2;
            }
        }
    }
}
