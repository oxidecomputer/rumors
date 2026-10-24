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
//!
//! A version's rows are [`Leaf`]s. [`oracle_plateaus`] reads them off an
//! oracle tree and [`version_from_rows`] closes them back into one, through
//! the one bottom-up closer every row listing here shares; on normal-form
//! trees the two are inverse. The deep version strategies
//! ([`crate::testing::generators::leaf_list`]) draw versions as such rows.

use std::sync::Arc;

use crate::shape::{Cell, Plateau, Region, Rise};
use crate::testing::oracles::tree;
use num_bigint::BigUint;

/// One leaf of a version's row listing: its depth and its absolute height.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Leaf {
    /// The leaf's depth below the root; its width is `2^-depth`.
    pub(crate) depth: u64,
    /// The sum of the bases on the leaf's root-to-leaf path.
    pub(crate) height: BigUint,
}

/// Closes a row listing's leaves, given in stream order with their depths,
/// into one tree, iteratively.
///
/// A stack holds the subtrees still waiting for a sibling. While the incoming
/// subtree and the top of the stack sit at one depth, they are siblings: they
/// close through `node` into one subtree a level up. Leaves that tile `[0, 1)`
/// exactly close into one root at depth 0.
///
/// # Panics
///
/// Panics if the depths do not tile `[0, 1)` exactly.
fn close_rows<T>(leaves: impl IntoIterator<Item = (u64, T)>, node: impl Fn(T, T) -> T) -> T {
    let mut open: Vec<(u64, T)> = Vec::new();
    for (mut depth, mut subtree) in leaves {
        while open.last().is_some_and(|(top, _)| *top == depth) {
            let (_, left) = open.pop().expect("the loop condition saw a top entry");
            subtree = node(left, subtree);
            depth = depth
                .checked_sub(1)
                .expect("rows tile the unit interval: two subtrees close above the root");
        }
        open.push((depth, subtree));
    }
    match (open.pop(), open.is_empty()) {
        (Some((0, root)), true) => root,
        _ => panic!("rows tile the unit interval: some subtree is left without a sibling"),
    }
}

/// Apply one rise to a running height, asserting the vocabulary's
/// invariants in passing: magnitudes are nonzero and the height never
/// goes negative.
pub(crate) fn apply_rise(height: &mut BigUint, rise: &Option<Rise>) {
    match rise {
        Some(Rise::Up(count)) => {
            assert_ne!(count.0, BigUint::ZERO, "rise magnitudes are nonzero");
            *height += &count.0;
        }
        Some(Rise::Down(count)) => {
            assert_ne!(count.0, BigUint::ZERO, "rise magnitudes are nonzero");
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
    close_rows(depths.into_iter().map(|depth| (depth, ())), |(), ()| ());
}

/// Fold a plateau stream into absolute [`Leaf`] rows, asserting
/// canonicality and tiling in passing.
pub(crate) fn fold_heights(plateaus: impl IntoIterator<Item = Plateau>) -> Vec<Leaf> {
    let mut height = BigUint::ZERO;
    let rows: Vec<Leaf> = plateaus
        .into_iter()
        .map(|plateau| {
            apply_rise(&mut height, &plateau.rise);
            Leaf {
                depth: plateau.depth,
                height: height.clone(),
            }
        })
        .collect();
    assert_tiles(rows.iter().map(|leaf| leaf.depth));
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
) -> Vec<(u64, Vec<BigUint>)> {
    let mut heights = vec![BigUint::ZERO; N];
    let rows: Vec<(u64, Vec<BigUint>)> = cells
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
) -> Vec<(u64, BigUint, bool)> {
    let mut height = BigUint::ZERO;
    let rows: Vec<(u64, BigUint, bool)> = overlay
        .into_iter()
        .map(|(plateau, owned)| {
            apply_rise(&mut height, &plateau.rise);
            (plateau.depth, height.clone(), owned)
        })
        .collect();
    assert_tiles(rows.iter().map(|&(depth, ..)| depth));
    rows
}

/// The plateaus of an oracle event tree: a [`Leaf`] row per preorder leaf.
///
/// Inverse to [`version_from_rows`] on normal-form trees. Recurses once per
/// tree level, as the oracle does, inside the envelope the tree's harness
/// bounds.
pub(crate) fn oracle_plateaus(tree: &tree::Version) -> Vec<Leaf> {
    fn walk(tree: &tree::Version, offset: &BigUint, depth: u64, out: &mut Vec<Leaf>) {
        match tree {
            tree::Version::Leaf(n) => out.push(Leaf {
                depth,
                height: offset + n,
            }),
            tree::Version::Node(n, l, r) => {
                let offset = offset + n;
                walk(l, &offset, depth + 1, out);
                walk(r, &offset, depth + 1, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(tree, &BigUint::ZERO, 0, &mut out);
    out
}

/// The regions of an oracle id tree: ownership and depth per preorder
/// leaf.
///
/// Recurses once per tree level, as the oracle does, inside the envelope the
/// tree's harness bounds.
pub(crate) fn oracle_regions(tree: &tree::Party) -> Vec<(bool, u64)> {
    fn walk(tree: &tree::Party, depth: u64, out: &mut Vec<(bool, u64)>) {
        match tree {
            tree::Party::Leaf(owned) => out.push((*owned, depth)),
            tree::Party::Node(l, r) => {
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
///
/// Recurses as deep as the deepest input, an oracle tree inside the envelope
/// its harness bounds.
pub(crate) fn oracle_cells(inputs: Vec<(BigUint, tree::Version)>) -> Vec<(u64, Vec<BigUint>)> {
    fn walk(inputs: Vec<(BigUint, tree::Version)>, depth: u64, out: &mut Vec<(u64, Vec<BigUint>)>) {
        if inputs
            .iter()
            .all(|(_, tree)| matches!(tree, tree::Version::Leaf(_)))
        {
            let heights = inputs
                .iter()
                .map(|(offset, tree)| match tree {
                    tree::Version::Leaf(n) => offset + n,
                    tree::Version::Node(..) => unreachable!("all inputs are leaves"),
                })
                .collect();
            out.push((depth, heights));
            return;
        }
        let side = |left: bool| {
            inputs
                .iter()
                .map(|(offset, tree)| match tree {
                    tree::Version::Leaf(_) => (offset.clone(), tree.clone()),
                    tree::Version::Node(n, l, r) => {
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
/// `(0, 0)` unrepresentable). Recurses once per level of `tree`, an oracle
/// tree inside the envelope its harness bounds.
pub(crate) fn party_as_steps(tree: &tree::Party) -> tree::Version {
    match tree {
        tree::Party::Leaf(owned) => tree::Version::Leaf(BigUint::from(u8::from(*owned))),
        tree::Party::Node(l, r) => tree::Version::Node(
            BigUint::ZERO,
            Arc::new(party_as_steps(l)),
            Arc::new(party_as_steps(r)),
        ),
    }
}

/// Rebuild the oracle event tree a [`Leaf`] row listing describes, through
/// the normalizing constructor — fragments a refinement introduced collapse
/// back, so the result is the normal form of the rows' step function.
///
/// Inverse to [`oracle_plateaus`] on normal-form trees. Iterative, so rows of
/// any depth close.
///
/// # Panics
///
/// Panics if the rows do not tile the unit interval.
pub(crate) fn version_from_rows(rows: &[Leaf]) -> tree::Version {
    close_rows(
        rows.iter()
            .map(|leaf| (leaf.depth, tree::Version::Leaf(leaf.height.clone()))),
        |left, right| tree::Version::node(BigUint::ZERO, left, right),
    )
}

/// Rebuild the oracle id tree an `(owned, depth)` row listing describes,
/// through the normalizing constructor, as [`version_from_rows`].
///
/// # Panics
///
/// Panics if the rows do not tile the unit interval.
pub(crate) fn party_from_rows(rows: &[(bool, u64)]) -> tree::Party {
    close_rows(
        rows.iter()
            .map(|&(owned, depth)| (depth, tree::Party::Leaf(owned))),
        tree::Party::node,
    )
}
