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
//! oracle tree and [`version_from_rows`] closes them back into one through
//! `close_rows`, the bottom-up closer that also checks tiling; on
//! normal-form trees the two are inverse. The deep version strategies
//! ([`crate::testing::generators::leaf_list`]) draw versions as such rows.
//!
//! The flat-tree models share two loops over row listings: [`collapse`],
//! which brings any listing to its normal form, and [`refine`], which cuts
//! several listings into their common refinement. [`collapse`] keeps a
//! closing stack of its own rather than building on `close_rows`, because
//! it rewrites the listing it closes. The flat model
//! ([`crate::testing::oracles::flat`]) builds its lattice on both loops, and
//! the independent encoder ([`crate::testing::oracles::codec`]) normalizes
//! its trees with the first.

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

/// A row of a flat tree's leaf listing: a leaf's depth and the value it
/// holds, which [`collapse`] compares between siblings.
pub(crate) trait Row {
    /// The value two sibling leaves must share to collapse.
    type Value: PartialEq;
    /// The leaf's depth below the root.
    fn depth(&self) -> u64;
    /// Moves the leaf to `depth`, as a collapse raises its survivor.
    fn set_depth(&mut self, depth: u64);
    /// The value the leaf holds.
    fn value(&self) -> &Self::Value;
}

/// A version's row: its value is its absolute height.
impl Row for Leaf {
    type Value = BigUint;

    fn depth(&self) -> u64 {
        self.depth
    }

    fn set_depth(&mut self, depth: u64) {
        self.depth = depth;
    }

    fn value(&self) -> &BigUint {
        &self.height
    }
}

/// A `(depth, value)` pair, the leaf form of the independent encoder's flat
/// trees.
impl<T: PartialEq> Row for (u64, T) {
    type Value = T;

    fn depth(&self) -> u64 {
        self.0
    }

    fn set_depth(&mut self, depth: u64) {
        self.0 = depth;
    }

    fn value(&self) -> &T {
        &self.1
    }
}

/// Collapses every pair of sibling leaves of equal value into their parent,
/// bottom up, until none remains: the normal form of the same flat tree.
///
/// A stack holds the completed subtrees still waiting for a sibling, each as
/// its depth and whether it is a single leaf, the last row written. When the
/// incoming subtree and the stack's top sit at one depth they are siblings,
/// and they close into their parent; two single leaves of one value collapse
/// into one leaf there, which may collapse again with the next subtree down
/// the stack. `on_collapse` hears each collapse: the rows so far, ending with
/// the surviving left leaf, and whether the collapse cascaded from one just
/// before it.
///
/// # Panics
///
/// Panics if the rows do not tile `[0, 1)`.
pub(crate) fn collapse<R: Row>(
    rows: impl IntoIterator<Item = R>,
    mut on_collapse: impl FnMut(&[R], bool),
) -> Vec<R> {
    let mut out: Vec<R> = Vec::new();
    let mut open: Vec<(u64, bool)> = Vec::new();
    for row in rows {
        let mut depth = row.depth();
        let mut single = true;
        let mut cascade = false;
        out.push(row);
        while open.last().is_some_and(|&(top, _)| top == depth) {
            let (_, left_single) = open.pop().expect("the loop condition saw a top entry");
            let parent = depth
                .checked_sub(1)
                .expect("rows tile the unit interval: two subtrees close above the root");
            let n = out.len();
            if left_single && single && out[n - 2].value() == out[n - 1].value() {
                out.pop();
                out[n - 2].set_depth(parent);
                on_collapse(&out, cascade);
                cascade = true;
            } else {
                single = false;
                cascade = false;
            }
            depth = parent;
        }
        open.push((depth, single));
    }
    assert!(
        matches!(open.as_slice(), [(0, _)]),
        "rows tile the unit interval: some subtree is left without a sibling"
    );
    out
}

/// One cell of a common refinement: its depth, and for each input the index
/// of the input's leaf that holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RefinedCell {
    /// The cell's depth; it is `2^-depth` wide.
    pub(crate) depth: u64,
    /// For each input, in input order, the index of its leaf holding the
    /// cell.
    pub(crate) leaves: Vec<usize>,
}

/// The common refinement of leaf depth lists, each tiling `[0, 1)`, as cells
/// left to right: the coarsest partition into dyadic cells that each lies
/// inside one leaf of every input.
///
/// The traversal keeps each input's current leaf and the position where it
/// ends. At each cell boundary, the inputs' current leaves all hold the
/// boundary point, so they are nested dyadic intervals, and one of them
/// starts there: the input whose leaf ended at the boundary advanced to it.
/// The deepest of the nested leaves is therefore the next cell, whole.
/// Zero inputs refine to the single cell `[0, 1)`.
///
/// # Panics
///
/// Panics if a depth list does not tile `[0, 1)`.
pub(crate) fn refine(inputs: &[Vec<u64>]) -> Vec<RefinedCell> {
    for depths in inputs {
        assert_tiles(depths.iter().copied());
    }
    let scale = inputs.iter().flatten().copied().max().unwrap_or(0);
    let width = |depth: u64| BigUint::from(1u8) << (scale - depth);
    let one = width(0);
    let mut index = vec![0usize; inputs.len()];
    let mut end: Vec<BigUint> = inputs.iter().map(|depths| width(depths[0])).collect();
    let mut at = BigUint::ZERO;
    let mut cells = Vec::new();
    loop {
        let depth = inputs
            .iter()
            .zip(&index)
            .map(|(depths, &i)| depths[i])
            .max()
            .unwrap_or(0);
        cells.push(RefinedCell {
            depth,
            leaves: index.clone(),
        });
        at += width(depth);
        if at == one {
            return cells;
        }
        for ((depths, i), end) in inputs.iter().zip(&mut index).zip(&mut end) {
            if *end == at {
                *i += 1;
                *end += width(depths[*i]);
            }
        }
    }
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
