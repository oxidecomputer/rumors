//! Public-door witnesses over combined shapes, plus the unit pins on the
//! walks' edges.
//!
//! The witnesses hold the lattice to pointwise extrema and the causal
//! order to pointwise comparison over combined item streams; the unit
//! pins cover arity 0 and 1, the started-input guard, fusing, and the
//! trivial values.
//!
//! The oracle differentials for the walks themselves are descriptors in
//! the pointwise differential table
//! ([`diff_ops`](crate::testing::diff_ops)), driven over the shared
//! populations; the row folds they share
//! ([`shape_rows`](crate::testing::shape_rows)) assert the item streams'
//! own canonicality and tiling in passing, here as there.

use proptest::prelude::*;

use crate::testing::bridge::from_oracle_version;
use crate::testing::generators;
use crate::testing::shape_rows::fold_cells;
use crate::{Clock, Version};

use super::{combine, Cell, Plateau, Rise};

proptest! {
    /// The join is the pointwise maximum and the meet the pointwise
    /// minimum, in every cell of the pair combined with its own join
    /// and meet.
    ///
    /// The public-door witness of the join/meet kernel, through nothing
    /// but item streams.
    #[test]
    fn join_and_meet_are_pointwise_extrema(
        a in generators::arb_oracle_version(),
        b in generators::arb_oracle_version(),
    ) {
        let (va, vb) = (from_oracle_version(&a), from_oracle_version(&b));
        let (join, meet) = (va.join(&vb), va.meet(&vb));
        for (_, heights) in fold_cells(combine([va.shape(), vb.shape(), join.shape(), meet.shape()])) {
            let [ha, hb, hj, hm] = heights.try_into().expect("four inputs, four heights");
            prop_assert_eq!(&hj, (&ha).max(&hb));
            prop_assert_eq!(&hm, (&ha).min(&hb));
        }
    }

    /// The causal order agrees with the pointwise order over combined
    /// shapes: `a <= b` iff no cell's height crosses, in each direction —
    /// the public-door witness of the comparison kernel.
    #[test]
    fn comparison_agrees_with_pointwise_order(
        a in generators::arb_oracle_version(),
        b in generators::arb_oracle_version(),
    ) {
        let (va, vb) = (from_oracle_version(&a), from_oracle_version(&b));
        let rows = fold_cells(combine([va.shape(), vb.shape()]));
        let le = rows.iter().all(|(_, h)| h[0] <= h[1]);
        let ge = rows.iter().all(|(_, h)| h[0] >= h[1]);
        let want = match (le, ge) {
            (true, true) => Some(core::cmp::Ordering::Equal),
            (true, false) => Some(core::cmp::Ordering::Less),
            (false, true) => Some(core::cmp::Ordering::Greater),
            (false, false) => None,
        };
        prop_assert_eq!(va.partial_cmp(&vb), want);
    }
}

/// The zero-input combine is the trivial refinement: one whole-interval
/// cell with no entries, then a fused end.
#[test]
fn empty_combine_is_the_trivial_cell() {
    let mut cells = combine::<0>([]);
    assert_eq!(
        cells.next(),
        Some(Cell {
            depth: 0,
            rises: []
        })
    );
    assert_eq!(cells.next(), None);
    assert_eq!(cells.next(), None);
}

/// A single-input combine yields the input's own plateaus, one cell
/// each: the refinement of one tiling is that tiling.
#[test]
fn single_input_combine_is_the_shape() {
    let version: Version = "(1, 1, (0, 0, 2))".parse().unwrap();
    let cells: Vec<(u64, [Option<Rise>; 1])> = combine([version.shape()])
        .map(|cell| (cell.depth, cell.rises))
        .collect();
    let plateaus: Vec<(u64, [Option<Rise>; 1])> =
        version.shape().map(|p| (p.depth, [p.rise])).collect();
    assert_eq!(cells, plateaus);
}

/// `combine` rejects an input that has already yielded an item: a
/// mid-walk shape has lost the interval alignment the refinement is
/// defined by.
#[test]
#[should_panic(expected = "un-iterated")]
fn combine_rejects_started_shapes() {
    let version = Version::new();
    let mut shape = version.shape();
    let _ = shape.next();
    let _ = combine([shape]);
}

/// Every shape iterator is fused: after the final item, `next` stays
/// `None`.
#[test]
fn shape_walks_fuse() {
    let version: Version = "(1, 1, (0, 0, 2))".parse().unwrap();
    let clock = Clock::from_parts("(1, 0)".parse().unwrap(), version.clone());

    let mut plateaus = version.shape();
    assert_eq!(plateaus.by_ref().count(), 3);
    assert_eq!(plateaus.next(), None);
    assert_eq!(plateaus.next(), None);

    let mut regions = clock.party().shape();
    assert_eq!(regions.by_ref().count(), 2);
    assert_eq!(regions.next(), None);
    assert_eq!(regions.next(), None);

    let mut overlay = clock.shape();
    assert_eq!(overlay.by_ref().count(), 3);
    assert_eq!(overlay.next(), None);
    assert_eq!(overlay.next(), None);

    let mut cells = combine([version.shape(), version.shape()]);
    assert_eq!(cells.by_ref().count(), 3);
    assert_eq!(cells.next(), None);
    assert_eq!(cells.next(), None);
}

/// The empty version and the seed clock walk as single whole-interval
/// items: no rise (height 0), full ownership on the seed.
#[test]
fn trivial_values_walk_whole() {
    assert_eq!(
        Version::new().shape().collect::<Vec<_>>(),
        vec![Plateau {
            rise: None,
            depth: 0
        }]
    );
    assert_eq!(
        Clock::seed().shape().collect::<Vec<_>>(),
        vec![(
            Plateau {
                rise: None,
                depth: 0
            },
            true
        )]
    );
}
