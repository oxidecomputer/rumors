//! Shared row vocabulary for the shape-walk differentials: folds of the
//! public item streams into absolute rows, the recursive oracle's own
//! enumerations of the same rows, and exact tree reconstruction from
//! rows.
//!
//! The folds are deliberately assertive: every fold checks the item
//! stream's own invariants in passing — nonzero rise magnitudes, a
//! never-negative running height, and widths that tile the unit interval
//! exactly — so every descriptor and test that folds a walk also holds
//! those invariants on its whole population. The reconstruction helpers
//! close the loop for the function-space legs: rows rebuild into normal
//! oracle trees (the normalizing constructors collapse any fragments a
//! refinement introduced), which the existing function-space comparisons
//! then scan.

use std::sync::Arc;

use crate::codec::Base;
use crate::oracle;
use crate::shape::{Cell, Plateau, Region, Rise};

/// Apply one rise to a running height, asserting the vocabulary's
/// invariants in passing: magnitudes are nonzero and the height never
/// goes negative.
pub(crate) fn apply_rise(height: &mut Base, rise: &Option<Rise>) {
    match rise {
        Some(Rise::Up(count)) => {
            assert_ne!(count.0, Base::ZERO, "rise magnitudes are nonzero");
            *height += &count.0;
        }
        Some(Rise::Down(count)) => {
            assert_ne!(count.0, Base::ZERO, "rise magnitudes are nonzero");
            assert!(count.0 <= *height, "the running height never goes negative");
            *height -= &count.0;
        }
        None => {}
    }
}

/// Assert a depth sequence tiles the unit interval exactly: read as a
/// preorder leaf listing, adjacent equal depths merge bottom-up and the
/// whole sequence must close to the single depth-0 interval.
pub(crate) fn assert_tiles(depths: impl IntoIterator<Item = u64>) {
    let mut stack: Vec<u64> = Vec::new();
    for depth in depths {
        stack.push(depth);
        while stack.len() >= 2 && stack[stack.len() - 1] == stack[stack.len() - 2] {
            let merged = stack.pop().expect("two entries are on the stack");
            assert!(merged > 0, "two whole intervals cannot both be present");
            *stack.last_mut().expect("one entry remains") = merged - 1;
        }
    }
    assert_eq!(stack, vec![0], "widths sum to exactly 1");
}

/// Fold a plateau stream into absolute `(height, depth)` rows, asserting
/// canonicality and tiling in passing.
pub(crate) fn fold_heights(plateaus: impl IntoIterator<Item = Plateau>) -> Vec<(Base, u64)> {
    let mut height = Base::ZERO;
    let rows: Vec<(Base, u64)> = plateaus
        .into_iter()
        .map(|plateau| {
            apply_rise(&mut height, &plateau.rise);
            (height.clone(), plateau.depth)
        })
        .collect();
    assert_tiles(rows.iter().map(|&(_, depth)| depth));
    rows
}

/// Fold a region stream into `(owned, depth)` rows, asserting tiling in
/// passing.
pub(crate) fn fold_regions(regions: impl IntoIterator<Item = Region>) -> Vec<(bool, u64)> {
    let rows: Vec<(bool, u64)> = regions
        .into_iter()
        .map(|Region { owned, depth }| (owned, depth))
        .collect();
    assert_tiles(rows.iter().map(|&(_, depth)| depth));
    rows
}

/// Fold a cell stream into `(depth, height per input)` rows, asserting
/// canonicality and tiling in passing.
pub(crate) fn fold_cells<const N: usize>(
    cells: impl IntoIterator<Item = Cell<N>>,
) -> Vec<(u64, Vec<Base>)> {
    let mut heights = vec![Base::ZERO; N];
    let rows: Vec<(u64, Vec<Base>)> = cells
        .into_iter()
        .map(|cell| {
            for (height, rise) in heights.iter_mut().zip(&cell.rises) {
                apply_rise(height, rise);
            }
            (cell.depth, heights.clone())
        })
        .collect();
    assert_tiles(rows.iter().map(|&(depth, _)| depth));
    rows
}

/// Fold a clock overlay stream into `(depth, height, owned)` rows,
/// asserting canonicality and tiling in passing.
pub(crate) fn fold_overlay(
    overlay: impl IntoIterator<Item = (Plateau, bool)>,
) -> Vec<(u64, Base, bool)> {
    let mut height = Base::ZERO;
    let rows: Vec<(u64, Base, bool)> = overlay
        .into_iter()
        .map(|(plateau, owned)| {
            apply_rise(&mut height, &plateau.rise);
            (plateau.depth, height.clone(), owned)
        })
        .collect();
    assert_tiles(rows.iter().map(|&(depth, ..)| depth));
    rows
}

/// The plateaus of an oracle event tree: absolute height and depth per
/// preorder leaf. Recursive, as the oracle is.
pub(crate) fn oracle_plateaus(tree: &oracle::Version) -> Vec<(Base, u64)> {
    fn walk(tree: &oracle::Version, offset: &Base, depth: u64, out: &mut Vec<(Base, u64)>) {
        match tree {
            oracle::Version::Leaf(n) => out.push((offset + n, depth)),
            oracle::Version::Node(n, l, r) => {
                let offset = offset + n;
                walk(l, &offset, depth + 1, out);
                walk(r, &offset, depth + 1, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(tree, &Base::ZERO, 0, &mut out);
    out
}

/// The regions of an oracle id tree: ownership and depth per preorder
/// leaf. Recursive, as the oracle is.
pub(crate) fn oracle_regions(tree: &oracle::Party) -> Vec<(bool, u64)> {
    fn walk(tree: &oracle::Party, depth: u64, out: &mut Vec<(bool, u64)>) {
        match tree {
            oracle::Party::Leaf(owned) => out.push((*owned, depth)),
            oracle::Party::Node(l, r) => {
                walk(l, depth + 1, out);
                walk(r, depth + 1, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(tree, 0, &mut out);
    out
}

/// The coarsest common refinement of oracle event trees, directly by
/// recursion: split while any input is a node, emit a cell when all are
/// leaves. `(depth, absolute height per input)` rows, left to right.
pub(crate) fn oracle_cells(inputs: Vec<(Base, oracle::Version)>) -> Vec<(u64, Vec<Base>)> {
    fn walk(inputs: Vec<(Base, oracle::Version)>, depth: u64, out: &mut Vec<(u64, Vec<Base>)>) {
        if inputs
            .iter()
            .all(|(_, tree)| matches!(tree, oracle::Version::Leaf(_)))
        {
            let heights = inputs
                .iter()
                .map(|(offset, tree)| match tree {
                    oracle::Version::Leaf(n) => offset + n,
                    oracle::Version::Node(..) => unreachable!("all inputs are leaves"),
                })
                .collect();
            out.push((depth, heights));
            return;
        }
        let side = |left: bool| {
            inputs
                .iter()
                .map(|(offset, tree)| match tree {
                    oracle::Version::Leaf(_) => (offset.clone(), tree.clone()),
                    oracle::Version::Node(n, l, r) => {
                        (offset + n, if left { (**l).clone() } else { (**r).clone() })
                    }
                })
                .collect::<Vec<_>>()
        };
        walk(side(true), depth + 1, out);
        walk(side(false), depth + 1, out);
    }
    let mut out = Vec::new();
    walk(inputs, 0, &mut out);
    out
}

/// An oracle id tree read as a 0/1-valued oracle event tree, so
/// [`oracle_cells`] can drive the clock overlay's expected rows.
///
/// Structure-preserving on canonical ids, and the result is normal form:
/// every id node has an empty leaf below it (an all-full node would
/// bottom out in the forbidden `(1, 1)`), so every mapped node's minimum
/// is zero, and equal sibling leaves cannot occur (`(1, 1)` forbidden,
/// `(0, 0)` unrepresentable).
pub(crate) fn party_as_steps(tree: &oracle::Party) -> oracle::Version {
    match tree {
        oracle::Party::Leaf(owned) => oracle::Version::Leaf(Base::from(u8::from(*owned))),
        oracle::Party::Node(l, r) => oracle::Version::Node(
            Base::ZERO,
            Arc::new(party_as_steps(l)),
            Arc::new(party_as_steps(r)),
        ),
    }
}

/// Rebuild the oracle event tree a `(height, depth)` row listing
/// describes, through the normalizing constructor — fragments a
/// refinement introduced collapse back, so the result is the normal form
/// of the rows' step function.
///
/// # Panics
///
/// Panics if the rows do not tile the unit interval.
pub(crate) fn version_from_rows(rows: &[(Base, u64)]) -> oracle::Version {
    let mut stack: Vec<(oracle::Version, u64)> = Vec::new();
    for (height, depth) in rows {
        stack.push((oracle::Version::Leaf(height.clone()), *depth));
        while stack.len() >= 2 && stack[stack.len() - 1].1 == stack[stack.len() - 2].1 {
            let (right, depth) = stack.pop().expect("two entries are on the stack");
            let (left, _) = stack.pop().expect("one entry remains");
            assert!(depth > 0, "two whole intervals cannot both be present");
            stack.push((oracle::Version::node(Base::ZERO, left, right), depth - 1));
        }
    }
    let [(tree, 0)] =
        <[(oracle::Version, u64); 1]>::try_from(stack).expect("rows tile the unit interval")
    else {
        panic!("rows tile the unit interval");
    };
    tree
}

/// Rebuild the oracle id tree an `(owned, depth)` row listing describes,
/// through the normalizing constructor, as [`version_from_rows`].
///
/// # Panics
///
/// Panics if the rows do not tile the unit interval.
pub(crate) fn party_from_rows(rows: &[(bool, u64)]) -> oracle::Party {
    let mut stack: Vec<(oracle::Party, u64)> = Vec::new();
    for (owned, depth) in rows {
        stack.push((oracle::Party::Leaf(*owned), *depth));
        while stack.len() >= 2 && stack[stack.len() - 1].1 == stack[stack.len() - 2].1 {
            let (right, depth) = stack.pop().expect("two entries are on the stack");
            let (left, _) = stack.pop().expect("one entry remains");
            assert!(depth > 0, "two whole intervals cannot both be present");
            stack.push((oracle::Party::node(left, right), depth - 1));
        }
    }
    let [(tree, 0)] =
        <[(oracle::Party, u64); 1]>::try_from(stack).expect("rows tile the unit interval")
    else {
        panic!("rows tile the unit interval");
    };
    tree
}
