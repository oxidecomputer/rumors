//! `Tree::join`'s algebraic laws and its deletion honoring by version
//! dominance.
//!
//! Join is the in-memory oracle the wire reconciliation is differentially
//! tested against (the streaming suites' join-oracle properties), so these
//! laws — with the route-equivalence property in the tree's own suite —
//! are what ground that oracle.

use proptest::prelude::*;

use crate::tree::arb::{arb_divergent_pair, arb_tree_root};
use crate::tree::{Root, Tree};

/// Merges via `Tree::join`.
fn join_tree(a: Root, b: Root) -> Root {
    let mut a = Tree::<()>::from_root(a);
    a.join(Tree::from_root(b));
    a.root
}

proptest! {
    /// Merging a tree with itself is a content no-op.
    #[test]
    fn join_idempotent((a, _b) in arb_divergent_pair()) {
        let tree_j = join_tree(a.clone(), a.clone());
        prop_assert_eq!(tree_j, a);
    }

    /// The merged tree is independent of merge direction.
    #[test]
    fn join_commutative((a, b) in arb_divergent_pair()) {
        prop_assert_eq!(join_tree(a.clone(), b.clone()), join_tree(b, a));
    }

    /// The merge is associative over three mutually-disjoint trees.
    ///
    /// (Uses `arb_tree_root` on three distinct party indices so the three are
    /// pairwise disjoint; `arb_divergent_pair` bakes in parties 0/1/2 and so
    /// cannot be composed three-way. Associativity in the presence of redactions
    /// is covered transitively: `join` matches the mirror, which proves it under
    /// its own redacting generators.)
    #[test]
    fn join_associative(
        a in arb_tree_root(0, 0..6),
        b in arb_tree_root(1, 0..6),
        c in arb_tree_root(2, 0..6),
    ) {
        let left = join_tree(join_tree(a.clone(), b.clone()), c.clone());
        let right = join_tree(a, join_tree(b, c));
        prop_assert_eq!(left, right);
    }
}
