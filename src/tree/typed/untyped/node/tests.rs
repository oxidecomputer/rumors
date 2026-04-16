use std::hash::Hash;
use std::sync::Arc;

use bytes::Bytes;
use imbl::OrdMap;
use proptest::collection::{btree_set, vec};
use proptest::prelude::*;

use super::{Children, Node};

/// Upper bound on the depth of trees generated in property tests. Each test
/// samples a depth in `0..=MAX_TEST_DEPTH` so that proptest shrinks tree
/// height as well as structure when a counterexample is found. Kept modest
/// because every byte of path compression wraps the subtree hash through
/// another 8 KB of hash input, so deep trees get expensive fast.
const MAX_TEST_DEPTH: usize = 4;

/// Maximum children per branch in generated trees. Capped at the alphabet
/// size so that every legal branching factor is reachable (subject to the
/// leaf budget).
const MAX_BRANCHING: usize = 256;

/// Upper bound on the number of leaves in any generated tree, used as a
/// branching budget. The budget is split across the children of each branch
/// (roughly `budget / n` per child), so branches that try to fan out wide
/// quickly run the budget down to 1 — at which point further branches are
/// forced to be single-child and path-compress into a chain. The actual
/// branching factor at any node is capped at `min(MAX_BRANCHING, budget)`,
/// so to exercise very wide branches the budget must be at least that wide.
const TREE_LEAF_BUDGET: usize = 16;

/// Generate an arbitrary tree of uniform depth `depth` with at most `budget`
/// leaves, constructed only via the public smart constructors `Node::leaf`
/// and `Node::branch`. At depth 0 the strategy produces a bare leaf; at
/// depth N > 0 it produces a branch of 1..=min(MAX_BRANCHING, budget)
/// children at distinct indices, each recursively budgeted. This guarantees
/// all leaves sit at a common depth, which is the precondition for
/// `Node::unions`. `budget` must be at least 1.
fn arb_tree<P>(depth: usize, budget: usize) -> BoxedStrategy<Node<P>>
where
    P: Arbitrary + Hash + Eq + Clone + AsRef<[u8]> + 'static,
{
    if depth == 0 {
        // Bytes payload is not examined at this abstraction layer, so we
        // stuff in a fixed empty value rather than generating one.
        (any::<P>(), any::<u64>())
            .prop_map(|(party, version)| Node::leaf((party, version).into(), Bytes::new()))
            .boxed()
    } else {
        let max_n = MAX_BRANCHING.min(budget);
        btree_set(any::<u8>(), 1..=max_n)
            .prop_flat_map(move |indices| {
                let n = indices.len();
                // Split the budget across children; with `n <= budget`, the
                // per-child budget is always at least 1.
                let per_child_budget = budget / n;
                let subtrees = vec(arb_tree::<P>(depth - 1, per_child_budget), n);
                (Just(indices), subtrees)
            })
            .prop_map(|(indices, subtrees)| {
                let pairs: OrdMap<u8, Node<P>> = indices.into_iter().zip(subtrees).collect();
                Node::branch(pairs).expect("branch input has >= 1 child")
            })
            .boxed()
    }
}

/// Recursively clear the cached hash at every node in the tree. After this
/// runs, `hash()` must recompute from scratch; comparing the pre-clear and
/// post-clear results is how we catch hash-invalidation bugs. Uses private
/// field access (only available to test code in this child module).
fn clear_hash_cache<P: Hash + Eq + Clone + AsRef<[u8]>>(node: &mut Node<P>) {
    let inner = Arc::make_mut(&mut node.inner);
    inner.hash.invalidate();
    if let Children::Branch(branch) = &mut inner.children {
        // Collect keys first so the iteration doesn't alias `branch.children`
        // while we recurse through `get_mut`.
        let keys: Vec<u8> = branch.keys().copied().collect();
        for k in keys {
            let child = branch.get_mut(&k).expect("key from keys()");
            clear_hash_cache(child);
        }
    }
}

/// Walk a tree via the public `into_children` API and collect every
/// (path, leaf) pair. Paths list the child indices from shallowest to
/// deepest, matching the order in which `into_children` yields them.
fn enumerate_leaves<P: Hash + Eq + Clone + AsRef<[u8]>>(
    node: Node<P>,
    path: Vec<u8>,
) -> Vec<(Vec<u8>, Bytes)> {
    match node.into_children() {
        Ok(children) => children
            .into_iter()
            .flat_map(|(idx, child)| {
                let mut child_path = path.clone();
                child_path.push(idx);
                enumerate_leaves(child, child_path)
            })
            .collect(),
        Err(leaf_node) => {
            let leaf = leaf_node
                .as_leaf()
                .expect("into_children returned Err only for leaves");
            vec![(path, leaf.clone())]
        }
    }
}

proptest! {
    /// Any tree built from the public smart constructors satisfies the
    /// path-compression invariant: every branch has at least two children.
    #[test]
    fn arbitrary_tree_is_max_compressed(
        tree in (0..=MAX_TEST_DEPTH).prop_flat_map(|depth| arb_tree::<String>(depth, TREE_LEAF_BUDGET)),
    ) {
        prop_assert!(tree.is_max_compressed());
    }
}
