//! Join's survivor rule and algebraic laws, including concurrent redactions.
//!
//! A flat leaf-by-leaf model checks deletion semantics without subtree bounds,
//! hashes, or either reconciliation implementation. It grounds `Tree::join` as
//! the oracle used by the wire tests.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use proptest::prelude::*;

use crate::Version;
use crate::message::Message;
use crate::tree::arb::{
    arb_divergent_pair, arb_divergent_roots, arb_tree_root, arb_wide_divergent_pair, nth_party,
    roots_at_depth,
};
use crate::tree::{Action, Root, Tree};

/// Merge two roots through the public tree operation.
fn join_tree(a: Root, b: Root) -> Root {
    let mut a = Tree::<()>::from_root(a);
    a.join(Tree::from_root(b));
    a.root
}

/// Read the concrete version set, independently of the root's hash and memos.
fn leaves(root: &Root) -> BTreeSet<Vec<u8>> {
    let tree = Tree::<()>::from_root(root.clone());
    let leaves: BTreeSet<_> = tree.iter().map(|(v, _)| v.as_bytes().to_vec()).collect();
    assert_eq!(
        leaves.len(),
        tree.len(),
        "each version names exactly one leaf"
    );
    leaves
}

/// The flat set rule: keep a leaf unless some replica has seen it and lacks it.
///
/// Every candidate comes from an input. Presence on all sides preserves it;
/// absence only means deletion if that side's causal history contains the
/// version. A concurrent or later version is still new to that side.
fn survivors<const N: usize>(roots: &[Root; N]) -> BTreeSet<Vec<u8>> {
    let live = roots.each_ref().map(leaves);
    let trees = roots
        .each_ref()
        .map(|root| Tree::<()>::from_root(root.clone()));
    let candidates: BTreeMap<_, _> = trees
        .iter()
        .flat_map(|tree| tree.iter().map(|(v, _)| (v.as_bytes().to_vec(), v.clone())))
        .collect();
    candidates
        .into_iter()
        .filter(|(key, version)| {
            roots.iter().zip(&live).all(|(root, live)| {
                live.contains(key)
                    || !matches!(
                        version.partial_cmp(&root.ceiling),
                        Some(Ordering::Less | Ordering::Equal)
                    )
            })
        })
        .map(|(key, _)| key)
        .collect()
}

/// Both merge directions must produce the independently calculated set and history.
fn check_survivors(roots: [Root; 2]) -> Result<(), TestCaseError> {
    let expected = survivors(&roots);
    let ceiling = &roots[0].ceiling | &roots[1].ceiling;
    let [a, b] = roots;
    for result in [join_tree(a.clone(), b.clone()), join_tree(b, a)] {
        prop_assert_eq!(&leaves(&result), &expected);
        prop_assert_eq!(&result.ceiling, &ceiling);
    }
    Ok(())
}

/// Joining a tree with its own causal past preserves clone-derived fans.
///
/// Redacting each leaf in turn creates a root fan that shares the remaining
/// child nodes with the saved past. Join must pair those children by radix;
/// losing alignment after a shared run would delete an unrelated leaf.
#[test]
fn joining_own_causal_past_preserves_clone_derived_fans() {
    let mut past = Tree::<()>::new();
    past.act(
        &nth_party(0),
        (0..25).map(|_| Action::Insert(Message::new(()))),
    );
    let paths: Vec<_> = past
        .iter()
        .map(|(version, _)| crate::tree::typed::Path::for_leaf(version))
        .collect();

    for path in paths {
        let mut current = Tree::<()>::from_root(past.root.clone());
        current.act(&nth_party(1), [Action::Forget(path)]);
        let expected = current.root.clone();

        current.join(Tree::from_root(past.root.clone()));
        assert_eq!(current.root, expected, "joining the past at {path:?}");
    }
}

proptest! {
    /// Merging a tree with itself preserves its content and causal history.
    #[test]
    fn join_idempotent((a, _b) in arb_divergent_pair()) {
        prop_assert_eq!(join_tree(a.clone(), a.clone()), a);
    }

    /// Merge direction does not affect the resulting tree or its history.
    #[test]
    fn join_commutative((a, b) in arb_divergent_pair()) {
        prop_assert_eq!(join_tree(a.clone(), b.clone()), join_tree(b, a));
    }

    /// Joining disjoint histories is associative, including empty live sets.
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

    /// Join keeps shared and unseen leaves, drops redacted ones, and unions history.
    #[test]
    fn join_matches_survivor_rule(
        (a, b) in prop_oneof![arb_divergent_pair(), arb_wide_divergent_pair()],
    ) {
        check_survivors([a, b])?;
    }

    /// The survivor rule holds at every branching depth, including the leaf parent.
    #[test]
    fn join_matches_survivor_rule_at_every_depth((a, b) in arb_divergent_pair()) {
        // Apply the same causal history at each depth. This makes coverage of
        // the leaf-level filter independent of improbable hash collisions.
        for depth in 0..32 {
            check_survivors(roots_at_depth([a.clone(), b.clone()], depth))?;
        }
    }

    /// All orders and groupings honor redactions, concurrent inserts, and shared leaves.
    #[test]
    fn join_associative_with_redactions(
        roots in arb_divergent_roots::<3>(0..6, 0..5),
        depth in proptest::option::of(0usize..32),
    ) {
        let roots = match depth {
            Some(depth) => roots_at_depth(roots, depth),
            None => roots,
        };
        let expected = survivors(&roots);
        let ceiling = roots.iter().fold(Version::new(), |sum, root| sum | &root.ceiling);
        for [a, b, c] in [[0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]] {
            let [a, b, c] = [a, b, c].map(|index| roots[index].clone());
            let left = join_tree(join_tree(a.clone(), b.clone()), c.clone());
            let right = join_tree(a, join_tree(b, c));
            // Equal but wrong results could satisfy associativity. Check each
            // result against the original inputs' flat survivor rule as well.
            prop_assert_eq!(&leaves(&left), &expected);
            prop_assert_eq!(&leaves(&right), &expected);
            prop_assert_eq!(&left.ceiling, &ceiling);
            prop_assert_eq!(&right.ceiling, &ceiling);
            prop_assert_eq!(left, right);
        }
    }
}
