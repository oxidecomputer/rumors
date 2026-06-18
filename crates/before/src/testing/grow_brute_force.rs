//! Brute-force grow-optimality reference.
//!
//! The paper's event condition (§3, §5.3.4) requires `event` register a
//! *minimal* inflation: `e < e'` and `e'` dominates no more than needed. `grow`
//! delivers this by a dynamic-programming search that, at every branch node,
//! greedily descends the cheaper child. Both the recursive oracle and the
//! packed impl realize that *same* DP — so the op-trace and arbitrary-tree
//! differentials (impl == oracle) can only confirm the two agree, never that
//! the shared DP is actually optimal. That is this module's job, and it is
//! independent of the DP: it enumerates the *entire* feasible single-region
//! inflation space by brute force (descending BOTH children at every node, with
//! no pruning), computes each candidate's true `(expansions, depth)` cost from
//! first principles, and takes the global minimum. If `grow`'s greedy local
//! choice ever disagrees with the global brute-force minimum, the DP is wrong.
//!
//! A "single-region inflation" of `(id, e)` is exactly what the paper's `grow`
//! may produce: pick one owned leaf-region of `e` (a region the id holds with a
//! `1`), then either increment its integer (a free inflation, cost `0`
//! expansions) or, where the id is a node but the event is a leaf, expand that
//! leaf into `(n, 0, 0)` (one expansion) and descend. The cost is `(expansions,
//! depth)`, lexicographic; ties favor the *right* (root-ward) child. This
//! mirrors the paper's recursion structurally, but where `grow` keeps only the
//! cheaper child at each node, [`all_inflations`] keeps *every* feasible child,
//! so its global minimum is computed over the full search space rather than the
//! pruned one.

use crate::oracle;

/// The inflation cost the paper assigns: `(expansions, depth)`, lexicographic.
/// Matches the oracle's `Cost` and the impl's `grow::Cost`.
pub(crate) type GrowCost = (u32, u32);

/// Every feasible single-region inflation of `(id, e)`, each paired with its true
/// `(expansions, depth)` cost.
///
/// The full search space `grow` optimizes over, enumerated
/// without pruning. Empty iff the id owns nothing here (an empty region can never be
/// inflated). Trees are raw (un-normalized), exactly as the paper's `grow` builds them;
/// callers normalize before comparing to `event`'s output. Recursive over a bounded test
/// tree (the impl's own traversals are iterative).
pub(crate) fn all_inflations(
    id: &oracle::Party,
    e: &oracle::Version,
) -> Vec<(oracle::Version, GrowCost)> {
    use oracle::Party as P;
    use oracle::Version as V;
    match (id, e) {
        // id full over a leaf: the one free inflation — increment in place.
        (P::Leaf(true), V::Leaf(n)) => vec![(V::Leaf(n + 1u32), (0, 0))],
        // id full over a node: descend either child; the id stays full (`1`) under it.
        (P::Leaf(true), V::Node(n, el, er)) => {
            let mut out = Vec::new();
            for (el2, c) in all_inflations(&P::Leaf(true), el) {
                out.push((
                    V::Node(n.clone(), Box::new(el2), er.clone()),
                    (c.0, c.1 + 1),
                ));
            }
            for (er2, c) in all_inflations(&P::Leaf(true), er) {
                out.push((
                    V::Node(n.clone(), el.clone(), Box::new(er2)),
                    (c.0, c.1 + 1),
                ));
            }
            out
        }
        // empty id: nothing owned here, so no inflation is feasible.
        (P::Leaf(false), _) => Vec::new(),
        // id node over a leaf: expand the leaf into `(n, 0, 0)` (one expansion), descend.
        (P::Node(..), V::Leaf(n)) => {
            let expanded = V::Node(
                n.clone(),
                Box::new(V::Leaf(0u32.into())),
                Box::new(V::Leaf(0u32.into())),
            );
            all_inflations(id, &expanded)
                .into_iter()
                .map(|(e2, c)| (e2, (c.0 + 1, c.1)))
                .collect()
        }
        // id node over an event node: descend either child under the matching id child.
        (P::Node(il, ir), V::Node(n, el, er)) => {
            let mut out = Vec::new();
            for (el2, c) in all_inflations(il, el) {
                out.push((
                    V::Node(n.clone(), Box::new(el2), er.clone()),
                    (c.0, c.1 + 1),
                ));
            }
            for (er2, c) in all_inflations(ir, er) {
                out.push((
                    V::Node(n.clone(), el.clone(), Box::new(er2)),
                    (c.0, c.1 + 1),
                ));
            }
            out
        }
    }
}

/// The globally minimal inflation cost over the full search space, or `None` if
/// the id owns nothing. Independent of `grow`'s DP: a flat minimum over
/// [`all_inflations`].
pub(crate) fn min_inflation_cost(id: &oracle::Party, e: &oracle::Version) -> Option<GrowCost> {
    all_inflations(id, e).into_iter().map(|(_, c)| c).min()
}

/// The single inflation the paper's `grow` must choose: globally cost-minimal,
/// with the root-ward (right-favoring) tie-break applied *locally* at each
/// branch node.
///
/// Returns the raw (un-normalized) tree and its cost, or `None` if
/// the id owns nothing.
///
/// Independent of `grow`'s greedy DP in the way that matters: each child's
/// weight is its *full-enumeration* minimum cost ([`min_inflation_cost`]), not
/// a value carried up a pruned recursion. So if `grow`'s local pruning ever
/// diverges from the global optimum, `grow`'s output will differ from this. The
/// right-favoring rule is the paper's: descend left iff the left child's
/// minimum is strictly cheaper than the right's (`cl < cr`), else descend
/// right.
pub(crate) fn best_inflation(
    id: &oracle::Party,
    e: &oracle::Version,
) -> Option<(oracle::Version, GrowCost)> {
    use oracle::Party as P;
    use oracle::Version as V;
    match (id, e) {
        (P::Leaf(false), _) => None,
        (P::Leaf(true), V::Leaf(n)) => Some((V::Leaf(n + 1u32), (0, 0))),
        (P::Node(..), V::Leaf(n)) => {
            let expanded = V::Node(
                n.clone(),
                Box::new(V::Leaf(0u32.into())),
                Box::new(V::Leaf(0u32.into())),
            );
            best_inflation(id, &expanded).map(|(e2, c)| (e2, (c.0 + 1, c.1)))
        }
        // Both node cases share the right-favoring child selection; only the id
        // children differ (`(1, 1)` for a full id over a node, `(il, ir)` for
        // an id node).
        (P::Leaf(true) | P::Node(..), V::Node(n, el, er)) => {
            let (idl, idr): (&P, &P) = match id {
                P::Node(il, ir) => (il, ir),
                _ => (&P::Leaf(true), &P::Leaf(true)),
            };
            let cl = min_inflation_cost(idl, el);
            let cr = min_inflation_cost(idr, er);
            // Descend left only when it is strictly cheaper and feasible; the
            // root-ward tie-break (and any infeasible left) sends us right.
            let go_left = match (cl, cr) {
                (Some(cl), Some(cr)) => cl < cr,
                (Some(_), None) => true,
                (None, _) => false,
            };
            if go_left {
                let (el2, c) = best_inflation(idl, el)?;
                Some((
                    V::Node(n.clone(), Box::new(el2), er.clone()),
                    (c.0, c.1 + 1),
                ))
            } else {
                let (er2, c) = best_inflation(idr, er)?;
                Some((
                    V::Node(n.clone(), el.clone(), Box::new(er2)),
                    (c.0, c.1 + 1),
                ))
            }
        }
    }
}
