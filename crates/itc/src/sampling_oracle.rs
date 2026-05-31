//! A second, semantically independent reference: the paper's function-space semantics
//! (§4), sampled on a dyadic grid.
//!
//! Every other differential reference in this crate — the recursive [`oracle`] and the
//! packed impl — represents an ITC stamp as a *tree* and realizes each operation as the
//! *same* tree recursion (the impl is a hand-optimized transcription of the oracle's
//! recursion). So "impl `==` oracle" can only ever confirm the two trees agree; it is, by
//! construction, blind to a bug the two share. This module is built on a representation that
//! shares *no* code and *no* structure with the tree recursion: it evaluates the paper's
//! semantic functions `JeK` / `JiK` (§4) at a finite grid of dyadic rationals in `[0, 1)`,
//! then defines the operations *pointwise* over those samples, exactly as the paper's
//! framework does (§4).
//!
//! Representation:
//! - The **event** component `e` becomes its step function `JeK` sampled at the `2^d` grid
//!   points `k / 2^d` for `k ∈ 0..2^d`. The value at a point is the sum of the base values
//!   on the root-to-leaf path to the leaf interval containing it (§4: "a base value, common
//!   for the whole interval, plus a relative value from the subtree"). Sample values are
//!   arbitrary-precision [`Base`], so a path sum that would overflow a `u64` is represented
//!   exactly — there is no truncation point.
//! - The **id** component `i` becomes its characteristic function `JiK ∈ {0, 1}` sampled on
//!   the same grid: the set of owned dyadic sub-intervals (§4).
//!
//! Operations, defined pointwise over the samples (§4):
//! - `leq` (event `≤`): pointwise `≤` over all samples.
//! - `join` (event `⊔`): pointwise `max` over all samples.
//! - id `contains` / `disjoint`: pointwise `≥` / "no common owned point" over the id
//!   samples (the disjointness invariant `i₁ · i₂ = 0`).
//!
//! ## Why a fixed grid suffices (grid resolution vs. tree depth)
//!
//! `JeK` is a *step* function, constant on each leaf interval; a normal-form tree of depth
//! `d` partitions `[0, 1)` into intervals of width `≥ 2^-d`. Sampling at the `2^d` left
//! endpoints `k / 2^d` therefore lands **at least one** grid point strictly inside every
//! leaf interval, and the function is constant there — so the sampled vector is a *faithful
//! and complete* description of `JeK` for any tree of depth `≤ d`. Pointwise `≤` / `max` /
//! equality over the samples are then exactly the function-space relations, with no aliasing
//! and no loss. The differential test picks `d = max(depth(a), depth(b))` (capped at
//! [`MAX_GRID_DEPTH`]), so the grid always resolves the trees under test.
//!
//! ## ⏱ Decoupling grid density from tree depth
//!
//! The grid has `2^d` points: it grows *exponentially* in the tree depth `d`, while the
//! tree corpus grows at its own (much slower for the gate variant) rate. These two axes are
//! deliberately kept separate. The grid depth is always `min(actual_tree_depth,
//! MAX_GRID_DEPTH)` — driven by the *depth* of the trees in hand, never by a corpus size
//! knob (which, used as a grid exponent, would be explosive). The gate-resident tests hold
//! `d` *small*: the arbitrary-normal-form generators cap depth at 4 and the op-trace tops
//! out at 7, both well under [`MAX_GRID_DEPTH`], so the cap never bites and every sample
//! vector is an exact description. The `#[ignore]`d dense variant
//! ([`tests::dense_deep_arbitrary`]) pushes far deeper random trees through the same checks
//! at a higher cap.

#[cfg(test)]
mod tests;

use crate::codec::Base;
use crate::oracle;

/// Upper bound on the dyadic-grid exponent `d`. The grid has `2^d` points, so this is a hard
/// ceiling on per-comparison cost. It is set high enough that, for the trees the tests
/// actually build, the cap **never bites** — so the grid always equals the true tree depth
/// and the sampling is a *complete, faithful* description (no aliasing). The arbitrary
/// generators cap tree depth at 4 (`test_support::ARB_DEPTH`, +1 for a `tick`), and the
/// seed-derived op-trace (≤30 ops over ≤8 parties) was measured to top out at depth 7; `10`
/// clears both with headroom while `2^10 = 1024` points keep each op microsecond-cheap. The
/// `grid_cap_is_never_reached` test pins that this headroom holds, so a spurious
/// (aliasing-induced) disagreement is impossible for the gate inputs.
pub(crate) const MAX_GRID_DEPTH: u32 = 10;

/// The structural depth of an event tree: 0 for a leaf, else 1 + max child depth. Iterative
/// (explicit stack), matching the crate's no-recursion-on-depth discipline even in test
/// scaffolding.
pub(crate) fn ev_depth(e: &oracle::Version) -> u32 {
    use oracle::Version as V;
    let mut max = 0u32;
    let mut stack = vec![(e, 0u32)];
    while let Some((node, d)) = stack.pop() {
        max = max.max(d);
        if let V::Node(_, l, r) = node {
            stack.push((l, d + 1));
            stack.push((r, d + 1));
        }
    }
    max
}

/// The structural depth of an id tree: 0 for a leaf, else 1 + max child depth. Iterative.
pub(crate) fn id_depth(i: &oracle::Party) -> u32 {
    use oracle::Party as P;
    let mut max = 0u32;
    let mut stack = vec![(i, 0u32)];
    while let Some((node, d)) = stack.pop() {
        max = max.max(d);
        if let P::Node(l, r) = node {
            stack.push((l, d + 1));
            stack.push((r, d + 1));
        }
    }
    max
}

/// Sample `JeK` (the event step function) at the `2^d` grid points `k / 2^d`, `k ∈ 0..2^d`.
///
/// `samples[k]` is the value of the function on the dyadic sub-interval
/// `[k / 2^d, (k+1) / 2^d)`: the sum of the base values on the path from the root to the
/// leaf interval that contains that sub-interval (§4, event tree semantics). Values are
/// arbitrary-precision [`Base`], so large path sums are represented exactly.
///
/// Walked iteratively: a worklist of `(node, base_offset, lo, hi)` where `[lo, hi)` is the
/// half-open grid-index range the node covers. A leaf fills its whole range with the
/// accumulated offset; a node splits its range in half and recurses into the children with
/// the offset advanced by the node base. For a tree of depth `≤ d`, every leaf range is
/// non-empty, so every sample is written exactly once.
pub(crate) fn sample_event(e: &oracle::Version, d: u32) -> Vec<Base> {
    use oracle::Version as V;
    let n = 1usize << d;
    let mut samples = vec![Base::ZERO; n];
    // (node, accumulated base offset, lo grid index, hi grid index) over `[lo, hi)`.
    let mut stack: Vec<(&V, Base, usize, usize)> = vec![(e, Base::ZERO, 0, n)];
    while let Some((node, off, lo, hi)) = stack.pop() {
        match node {
            V::Leaf(b) => {
                let v = &off + b;
                for slot in &mut samples[lo..hi] {
                    *slot = v.clone();
                }
            }
            V::Node(b, l, r) => {
                let mid = (lo + hi) / 2;
                let child_off = &off + b;
                stack.push((l, child_off.clone(), lo, mid));
                stack.push((r, child_off, mid, hi));
            }
        }
    }
    samples
}

/// Sample `JiK` (the id characteristic function) at the same `2^d` grid: `owned[k]` is
/// `true` iff the dyadic sub-interval `[k / 2^d, (k+1) / 2^d)` is owned (maps to `1`). Walked
/// iteratively, identically to [`sample_event`] minus the integer offset.
pub(crate) fn sample_id(i: &oracle::Party, d: u32) -> Vec<bool> {
    use oracle::Party as P;
    let n = 1usize << d;
    let mut owned = vec![false; n];
    let mut stack: Vec<(&P, usize, usize)> = vec![(i, 0, n)];
    while let Some((node, lo, hi)) = stack.pop() {
        match node {
            P::Leaf(b) => {
                if *b {
                    owned[lo..hi].fill(true);
                }
            }
            P::Node(l, r) => {
                let mid = (lo + hi) / 2;
                stack.push((l, lo, mid));
                stack.push((r, mid, hi));
            }
        }
    }
    owned
}

/// The grid exponent that resolves both event trees, capped at [`MAX_GRID_DEPTH`].
pub(crate) fn ev_grid_depth(a: &oracle::Version, b: &oracle::Version) -> u32 {
    ev_depth(a).max(ev_depth(b)).min(MAX_GRID_DEPTH)
}

/// The grid exponent that resolves both id trees, capped at [`MAX_GRID_DEPTH`].
pub(crate) fn id_grid_depth(a: &oracle::Party, b: &oracle::Party) -> u32 {
    id_depth(a).max(id_depth(b)).min(MAX_GRID_DEPTH)
}

/// Event `≤` in the function space: pointwise `≤` over the samples (§4).
pub(crate) fn ev_leq(a: &[Base], b: &[Base]) -> bool {
    debug_assert_eq!(a.len(), b.len(), "samples taken at the same grid depth");
    a.iter().zip(b).all(|(x, y)| x <= y)
}

/// Event join `⊔` in the function space: pointwise `max` over the samples (§4).
pub(crate) fn ev_join(a: &[Base], b: &[Base]) -> Vec<Base> {
    debug_assert_eq!(a.len(), b.len(), "samples taken at the same grid depth");
    a.iter().zip(b).map(|(x, y)| x.max(y).clone()).collect()
}

/// Id containment in the function space: `a ⊇ b` iff every point `b` owns, `a` also owns
/// (pointwise `≥` over the characteristic functions). The oracle's `Party::partial_cmp`
/// reads an *ancestor* (a larger owned region) as `Less`, so `a ⊇ b ⇔ a ≤ b` there.
pub(crate) fn id_contains(a: &[bool], b: &[bool]) -> bool {
    debug_assert_eq!(a.len(), b.len(), "samples taken at the same grid depth");
    a.iter().zip(b).all(|(x, y)| *x || !*y)
}

/// Id disjointness in the function space: `i₁ · i₂ = 0` (§4) — no grid point is
/// owned by both.
pub(crate) fn id_disjoint(a: &[bool], b: &[bool]) -> bool {
    debug_assert_eq!(a.len(), b.len(), "samples taken at the same grid depth");
    !a.iter().zip(b).any(|(x, y)| *x && *y)
}
