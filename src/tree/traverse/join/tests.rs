//! `Tree::join` is observationally identical to mirroring two local trees: for
//! any divergent pair it produces the same merged tree, including honoring
//! deletions by version dominance.
//!
//! The oracle is the mirror engine driven directly (not `Known::join_then`,
//! which *is* `Tree::join` now).

use proptest::prelude::*;

use crate::tree::arb::{arb_divergent_pair, arb_tree_root};
use crate::tree::mirror::alternating::{local, mirror};
use crate::tree::{Root, Tree};

/// Drive the local-local mirror directly. This is the oracle: the merge
/// engine `Tree::join` must match, invoked without going through `Known`
/// (whose `join_then` now delegates to `Tree::join`).
fn mirror_merge(a: Root<()>, b: Root<()>) -> Root<()> {
    pollster::block_on(async {
        // Local-local oracle: the network/intent greeting fields are inert
        // here (no lib-level dispatch runs), so a placeholder network and
        // `Remain` intent suffice — only the version the handshake carries
        // is consumed.
        let l = local::Exchange::start(a);
        let r = local::Exchange::start(b);
        match mirror(l, r).await {
            Ok((merged, _)) => merged,
            Err(e) => match e {},
        }
    })
}

/// Merge via `Tree::join`.
fn join_tree(a: Root<()>, b: Root<()>) -> Root<()> {
    let mut a = Tree { root: a };
    a.join(Tree { root: b });
    a.root
}

proptest! {
    /// `Tree::join` produces a byte-identical merged tree to the mirror,
    /// including honoring deletions by version dominance.
    #[test]
    fn join_matches_mirror((a, b) in arb_divergent_pair()) {
        let tree_j = join_tree(a.clone(), b.clone());
        let tree_m = mirror_merge(a, b);

        // Same merged tree (version + structure; equal trees ⟹ equal hash).
        prop_assert_eq!(&tree_j, &tree_m);
    }

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
