use std::collections::BTreeSet;

use proptest::collection::vec;
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

use crate::tree::arb::{arb_leaf_paths, arb_radixes, arb_version};
use crate::{Version, message::Message};

use super::{
    Body, HashKey, Node, PATH_LEN, SortedLeaves, View,
    fan::{Fan, heap_slots},
};

mod reassembly;

/// Upper bound on the depth of trees generated in property tests.
///
/// Each test samples a depth in `0..=MAX_TEST_DEPTH` so that proptest shrinks
/// tree height as well as structure when a counterexample is found. Kept
/// modest to keep generated structures small and shrinking fast.
pub(super) const MAX_TEST_DEPTH: usize = 4;

/// Maximum children per branch in generated trees. Capped at the alphabet
/// size so that every legal branching factor is reachable (subject to the
/// leaf budget).
const MAX_BRANCHING: usize = 256;

/// Upper bound on the number of leaves in any generated tree, used as a
/// branching budget.
///
/// Each branch divides its budget randomly across its
/// children — every child gets at least 1, and the parts sum to the parent's
/// budget — so a branch that fans out as wide as its budget forces every
/// child down to a single leaf, and any deeper branch beneath such a child is
/// forced to be single-child and path-compress into a chain. The actual
/// branching factor at any node is capped at `min(MAX_BRANCHING, budget)`,
/// so to exercise very wide branches the budget must be at least that wide.
pub(super) const TREE_LEAF_BUDGET: usize = 16;

/// The hash key these tests digest nodes under: the fixture network's, the
/// one key every node built here caches its digest under.
pub(super) fn key() -> HashKey {
    HashKey::fixture()
}

/// Generate an arbitrary tree of uniform depth `depth` with at most `budget`
/// leaves, constructed only via the public smart constructors `Node::leaf` and
/// `Node::branch`.
///
/// At depth 0 the strategy produces a bare leaf; at depth N > 0
/// it produces a branch with 1..=min(MAX_BRANCHING, budget) children at
/// distinct indices, the parent's budget divided randomly among them (each
/// child gets at least 1 and the shares sum to the parent's budget). This
/// guarantees all leaves sit at a common depth, and no more than `budget`
/// leaves are generated. `budget` must be at least 1.
pub(super) fn arb_tree(depth: usize, budget: usize) -> BoxedStrategy<Node> {
    if depth == 0 {
        // The leaf payload is not examined at this abstraction layer, so we
        // stuff in a fixed empty value rather than generating one; only the
        // version is varied.
        arb_version()
            .prop_map(|version| Node::leaf(version, Message::new(())))
            .boxed()
    } else {
        // A branch fans out to between 1 and `min(MAX_BRANCHING, budget)`
        // children at distinct byte indices. Capping the count at `budget`
        // leaves at least one unit of budget for every child.
        let max_n = MAX_BRANCHING.min(budget);
        arb_radixes(1..=max_n)
            .prop_flat_map(move |indices| {
                let n = indices.len();
                // Give every child a baseline of 1, then scatter the
                // remaining `budget - n` leaves across children at random:
                // each token bumps one child's share by 1. The shares always
                // sum to exactly `budget`, so no layer can exceed it, and the
                // randomness diversifies the shapes of deeper subtrees.
                let extra = budget - n;
                (Just(indices), vec(0..n, extra))
            })
            .prop_flat_map(move |(indices, tokens)| {
                let mut per_child = vec![1usize; indices.len()];
                for child in tokens {
                    per_child[child] += 1;
                }
                let subtrees: Vec<_> = per_child
                    .into_iter()
                    .map(|child_budget| arb_tree(depth - 1, child_budget))
                    .collect();
                (Just(indices), subtrees)
            })
            .prop_map(|(indices, subtrees)| {
                let children: Fan = indices.into_iter().zip(subtrees).collect();
                Node::branch(children).expect("branch input has >= 1 child")
            })
            .boxed()
    }
}

/// Collect each leaf's path, version, and payload through `into_children`.
/// Paths run from shallowest to deepest; decomposing a compressed prefix
/// preserves the leaf's version and payload.
fn enumerate_leaves(node: Node, path: Vec<u8>) -> Vec<(Vec<u8>, Version, Message)> {
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
            let version = leaf_node.ceiling().clone();
            let leaf = leaf_node
                .as_leaf()
                .expect("into_children returned Err only for leaves")
                .clone();
            vec![(path, version, leaf)]
        }
    }
}

/// Rebuild a tree through its constructors, applying `f` to each payload.
/// Leaf versions and paths stay the same; branch bounds are memoized lazily
/// from the rebuilt children. Cloning each payload reconstructs the tree.
fn rebuild_with<F>(node: Node, f: &F) -> Node
where
    F: Fn(&Message) -> Message,
{
    match node.into_children() {
        Err(leaf_node) => {
            let leaf = leaf_node
                .as_leaf()
                .expect("into_children returned Err only for leaves");
            Node::leaf(leaf_node.ceiling().clone(), f(leaf))
        }
        Ok(children) => {
            let rebuilt: Fan = children
                .into_iter()
                .map(|(k, v)| (k, rebuild_with(v, f)))
                .collect();
            Node::branch(rebuilt).expect("non-empty")
        }
    }
}

/// Empty branches have no node; the constructor returns `None`.
#[test]
fn empty_branch_is_none() {
    let empty: Fan = Fan::new();
    assert!(Node::branch(empty).is_none());
}

/// A bulk leaf run must contain at least one leaf.
#[test]
#[should_panic(expected = "a leaf run is non-empty")]
fn bulk_build_rejects_an_empty_run() {
    Node::from_sorted_leaves(0, &mut []);
}

/// A bulk build cannot consume a leaf whose slot has already been taken.
#[test]
#[should_panic(expected = "each leaf node is consumed exactly once")]
fn bulk_build_rejects_a_consumed_leaf() {
    Node::from_sorted_leaves(0, &mut [([0; PATH_LEN], None)]);
}

/// A prefix-free branch still cannot occupy a bulk builder's leaf slot.
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "a leaf run supplies bare leaf nodes")]
fn bulk_build_rejects_a_branch_as_a_leaf() {
    let children = [0, 1]
        .into_iter()
        .map(|radix| (radix, Node::leaf(Version::new(), Message::new(()))))
        .collect();
    let branch = Node::branch(children).expect("two children form a branch");
    Node::from_sorted_leaves(0, &mut [([0; PATH_LEN], Some(branch))]);
}

proptest! {
    /// Duplicate and descending paths are rejected before recursive grouping,
    /// at any starting depth. A later panic must not mask a missing order check.
    #[cfg(debug_assertions)]
    #[test]
    fn bulk_build_rejects_nonascending_paths(
        mut low in any::<[u8; PATH_LEN]>(),
        depth in 0..PATH_LEN,
        radix in 0u8..u8::MAX,
    ) {
        low[depth] = radix;
        let mut high = low;
        high[depth] += 1;
        for paths in [[high, low], [low, low]] {
            let result = std::panic::catch_unwind(|| {
                let mut entries = paths.map(|path| {
                    (path, Some(Node::leaf(Version::new(), Message::new(()))))
                });
                Node::from_sorted_leaves(depth, &mut entries);
            });
            let panic = result.expect_err("unordered paths must be rejected");
            prop_assert_eq!(
                panic.downcast_ref::<&str>().copied(),
                Some("a leaf run is strictly ascending by path"),
            );
        }
    }

    /// A bulk leaf must have no prefix: the supplied path provides its suffix.
    #[cfg(debug_assertions)]
    #[test]
    fn bulk_build_rejects_a_prefixed_leaf(
        path in any::<[u8; PATH_LEN]>(),
        prefix_len in 1..=PATH_LEN,
        depth in 0..=PATH_LEN,
    ) {
        let result = std::panic::catch_unwind(|| {
            let mut leaf = Node::leaf(Version::new(), Message::new(()));
            for &radix in &path[..prefix_len] {
                leaf = leaf.beneath(radix);
            }
            Node::from_sorted_leaves(depth, &mut [(path, Some(leaf))]);
        });
        let panic = result.expect_err("a prefixed leaf must be rejected");
        prop_assert_eq!(
            panic.downcast_ref::<&str>().copied(),
            Some("a leaf run supplies bare leaf nodes"),
        );
    }

    /// Any tree built from the public smart constructors satisfies the
    /// path-compression invariant: every branch has at least two children.
    #[test]
    fn arbitrary_tree_is_max_compressed(
        tree in (0..=MAX_TEST_DEPTH).prop_flat_map(|depth| arb_tree(depth, TREE_LEAF_BUDGET)),
    ) {
        prop_assert!(tree.is_max_compressed());
    }

    /// Every fan a tree stores holds at most [`heap_slots`] of its length,
    /// whichever path stored it.
    ///
    /// The paths are arbitrary trees assembled through [`Node::branch`],
    /// canonical trees bulk-built from sorted leaves, and the copy-on-write
    /// copy of a shared root that raising it a level makes.
    #[test]
    fn stored_fans_meet_their_slot_bound(
        tree in (0..=MAX_TEST_DEPTH).prop_flat_map(|depth| arb_tree(depth, TREE_LEAF_BUDGET)),
        paths in arb_leaf_paths(256),
    ) {
        fans_fit(&tree)?;
        // `tree` keeps its root shared, so raising another handle copies the
        // root's state, its fan included, rather than mutating it in place.
        let raised = tree.clone().beneath(0);
        prop_assert!(!raised.ptr_eq(&tree), "raising a shared root copies it");
        fans_fit(&raised)?;
        let paths: Vec<[u8; 32]> = paths.into_iter().collect();
        fans_fit(&canonical_at(0, &paths))?;
    }

    /// [`Node::branch`] stores its fan within [`heap_slots`] of its length,
    /// with the same children in the same order, whatever slack the fan
    /// carried in.
    ///
    /// The slack is room collection reserved for repeated radixes and room
    /// left behind by children removed after the fan grew. The fan collects
    /// every radix in `kept` and `removed` in ascending order followed by the
    /// `repeats`, so any repeat sends collection through its deduplicating
    /// path; the radixes only in `removed` then come out again.
    #[test]
    fn branch_fits_the_fan_it_stores(
        kept in arb_radixes(2..=256),
        removed in arb_radixes(0..=256),
        repeats in vec(any::<prop::sample::Index>(), 0..=256),
    ) {
        let pushed: Vec<u8> = kept.union(&removed).copied().collect();
        let repeated = repeats.iter().map(|index| *index.get(&pushed));
        let mut fan: Fan = pushed
            .iter()
            .copied()
            .chain(repeated)
            .map(|radix| (radix, Node::leaf(Version::new(), Message::new(()))))
            .collect();
        for &radix in removed.difference(&kept) {
            fan.remove(radix);
        }

        let node = Node::branch(fan).expect("two or more kept children make a branch");
        let stored = node.stored_fan().expect("a branch stores its fan");
        prop_assert!(stored.iter().map(|(radix, _)| radix).eq(kept.iter().copied()));
        prop_assert!(stored.heap_capacity() <= heap_slots(kept.len()));
    }

    /// Decomposing and rebuilding a tree preserves its root hash and ceiling,
    /// including when singleton branches are compressed back into prefixes.
    #[test]
    fn decompose_and_rebuild_preserves_hash_and_ceiling(
        tree in (0..=MAX_TEST_DEPTH).prop_flat_map(|d| arb_tree(d, TREE_LEAF_BUDGET)),
    ) {
        let hash_before = tree.hash(&key());
        let ceiling_before = tree.ceiling().clone();
        let rebuilt = rebuild_with(tree, &|b| b.clone());
        prop_assert_eq!(rebuilt.hash(&key()), hash_before);
        prop_assert_eq!(rebuilt.ceiling(), &ceiling_before);
    }

    /// Enumerating a generated tree's leaves via the public API yields
    /// exactly as many leaves as the tree holds, every leaf sits at path
    /// length equal to the generated depth, and no two leaves share a
    /// path.
    ///
    /// This pins down three independent claims in one place: that
    /// `into_children` unpacks exactly one prefix byte per step, that all
    /// leaves live at a common depth (the `arb_tree` contract), and that
    /// branch indices are distinct so leaf paths are unique.
    #[test]
    fn leaf_enumeration_has_expected_shape(
        (depth, tree) in (0..=MAX_TEST_DEPTH)
            .prop_flat_map(|d| (Just(d), arb_tree(d, TREE_LEAF_BUDGET))),
    ) {
        let leaves = enumerate_leaves(tree, Vec::new());
        prop_assert!(!leaves.is_empty());
        for (path, _, _) in &leaves {
            prop_assert_eq!(path.len(), depth);
        }
        let distinct: BTreeSet<Vec<u8>> =
            leaves.iter().map(|(p, _, _)| p.clone()).collect();
        prop_assert_eq!(distinct.len(), leaves.len());
    }

    /// The root ceiling is the join of all leaf versions and dominates each one.
    #[test]
    fn ceiling_is_join_of_leaf_versions(
        tree in (0..=MAX_TEST_DEPTH).prop_flat_map(|d| arb_tree(d, TREE_LEAF_BUDGET)),
    ) {
        let root_ceiling = tree.ceiling().clone();
        let leaves = enumerate_leaves(tree, Vec::new());

        for (_, v, _) in &leaves {
            prop_assert!(v <= root_ceiling);
        }

        let joined = leaves
            .iter()
            .map(|(_, v, _)| v.clone())
            .fold(Version::new(), |acc, v| acc | v);
        prop_assert_eq!(joined, root_ceiling);
    }

    /// The root floor is the meet of all leaf versions and is dominated by each.
    #[test]
    fn floor_is_meet_of_leaf_versions(
        tree in (0..=MAX_TEST_DEPTH).prop_flat_map(|d| arb_tree(d, TREE_LEAF_BUDGET)),
    ) {
        let root_floor = tree.floor().clone();
        let leaves = enumerate_leaves(tree, Vec::new());

        for (_, v, _) in &leaves {
            prop_assert!(root_floor <= *v);
        }

        let mut versions = leaves.iter().map(|(_, v, _)| v);
        let mut met = versions
            .next()
            .expect("a generated tree holds at least one leaf")
            .clone();
        for v in versions {
            met &= v;
        }
        prop_assert_eq!(met, root_floor);
    }

    /// Every node's bounds equal the meet and join of its descendant versions.
    /// Checking each subtree prevents a sibling's contribution from masking a
    /// wrong interior memo at the root. Separate folds check the combined fold.
    #[test]
    fn bounds_are_the_leaf_fold_at_every_node(
        tree in (0..=MAX_TEST_DEPTH).prop_flat_map(|d| arb_tree(d, TREE_LEAF_BUDGET)),
    ) {
        /// Check this subtree's bounds and return its leaf versions to the parent.
        fn check(node: &Node) -> Result<Vec<Version>, TestCaseError> {
            let floor = node.floor().clone();
            let ceiling = node.ceiling().clone();
            let leaves = match node.clone().into_children() {
                Ok(children) => {
                    let mut all = Vec::new();
                    for (_, child) in children {
                        all.extend(check(&child)?);
                    }
                    all
                }
                Err(leaf) => vec![leaf.ceiling().clone()],
            };
            let (first, rest) = leaves.split_first().expect("every subtree holds a leaf");
            prop_assert_eq!(
                &first.meet_all(rest),
                &floor,
                "a node's floor must be its own leaves' meet"
            );
            prop_assert_eq!(
                &first.join_all(rest),
                &ceiling,
                "a node's ceiling must be its own leaves' join"
            );
            prop_assert!(
                floor <= ceiling,
                "the memoized bounds are an ordered interval at every node"
            );
            Ok(leaves)
        }
        check(&tree)?;
    }

    /// Wrapping a child in N nested singleton branches accumulates an
    /// N-byte compressed prefix above it, committed in the child's own
    /// single preimage.
    ///
    /// The wraps must never materialize one-child branch nodes — the
    /// prefix length grows by exactly N — and the observable hash must
    /// match the independent literal-preimage reference, so a wrong prefix
    /// rule or stale memoization surfaces as a mismatch.
    #[test]
    fn nested_singleton_wraps_extend_the_committed_prefix(
        indices in vec(any::<u8>(), 2..=8),
        child in (0..=MAX_TEST_DEPTH).prop_flat_map(|d| arb_tree(d, TREE_LEAF_BUDGET)),
    ) {
        let child_prefix_len = child.compressed_prefix_len();

        let mut wrapped = child;
        for &index in &indices {
            wrapped = Node::branch(Fan::unit(index, wrapped))
                .expect("one-child branch is non-empty");
        }

        prop_assert_eq!(
            wrapped.compressed_prefix_len(),
            child_prefix_len + indices.len(),
        );
        prop_assert_eq!(wrapped.hash(&key()), reference_hash(wrapped.clone()));
    }

    /// Popping the topmost compressed-prefix byte (via `into_children`)
    /// must produce a node whose hash matches a freshly-built node with
    /// the same children and the shortened prefix.
    ///
    /// Pop shortens the prefix
    /// and resets the lazy hash memo, so the recomputed value must match the
    /// from-scratch reference; a missing memo reset would surface here.
    #[test]
    fn pop_top_byte_matches_freshly_built_shorter_prefix(
        indices in arb_radixes(2..=8),
        child in (0..=MAX_TEST_DEPTH).prop_flat_map(|d| arb_tree(d, TREE_LEAF_BUDGET)),
    ) {
        let indices: Vec<u8> = indices.into_iter().collect();

        // Build the wrapped node by nesting singleton branches.
        let mut wrapped = child.clone();
        for &index in &indices {
            wrapped = Node::branch(Fan::unit(index, wrapped))
                .expect("one-child branch is non-empty");
        }

        // Pop the topmost byte. The returned fan has exactly one entry
        // because `wrapped` was a singleton-branch chain; the entry's
        // key is the popped byte and its value is the same node with a
        // one-shorter prefix.
        let mut popped_children = wrapped.into_children().expect("non-empty");
        prop_assert_eq!(popped_children.len(), 1);
        let (popped_byte, popped) = popped_children
            .iter()
            .next()
            .map(|(k, v)| (k, v.clone()))
            .expect("singleton");
        popped_children.remove(popped_byte);
        prop_assert_eq!(popped_byte, *indices.last().expect("non-empty indices"));

        // Build a reference node with the same children but the shortened
        // prefix from scratch.
        let mut reference = child;
        for &index in &indices[..indices.len() - 1] {
            reference = Node::branch(Fan::unit(index, reference))
                .expect("one-child branch is non-empty");
        }

        prop_assert_eq!(popped.hash(&key()), reference.hash(&key()));
    }

    /// A one-child branch at index `i` never materializes: `Node::branch`
    /// collapses it into the child's compressed prefix, and the index byte
    /// joins the prefix the child's single preimage commits.
    ///
    /// The observable hash must match the independent literal-preimage
    /// reference, and must differ from the child's own hash (the committed
    /// prefix grew), so path compression stays observation-invisible while
    /// the level itself stays observable.
    #[test]
    fn singleton_branch_commits_the_index_in_the_prefix(
        index in any::<u8>(),
        child in (0..=MAX_TEST_DEPTH).prop_flat_map(|d| arb_tree(d, TREE_LEAF_BUDGET)),
    ) {
        let child_hash = child.hash(&key());
        let wrapped = Node::branch(Fan::unit(index, child))
            .expect("one-child branch is non-empty");

        prop_assert_ne!(wrapped.hash(&key()), child_hash);
        prop_assert_eq!(wrapped.hash(&key()), reference_hash(wrapped.clone()));
    }

    /// `beneath` must invalidate the memoized hash.
    ///
    /// We force the child's hash
    /// to be computed and cached *first*, then wrap it; the wrapped node's
    /// observable hash must reflect the new top level (the extended
    /// committed prefix), not the stale cached child hash. Without the memo
    /// reset in `beneath`, the wrapped node would report the child's
    /// pre-wrap hash.
    #[test]
    fn beneath_invalidates_memoized_hash(
        index in any::<u8>(),
        child in (0..=MAX_TEST_DEPTH).prop_flat_map(|d| arb_tree(d, TREE_LEAF_BUDGET)),
    ) {
        let child_hash = child.hash(&key()); // populate the child's lazy memo
        let wrapped = child.beneath(index);

        prop_assert_ne!(wrapped.hash(&key()), child_hash);
        prop_assert_eq!(wrapped.hash(&key()), reference_hash(wrapped.clone()));
    }

    /// `into_children` popping a prefix byte must invalidate the memoized hash.
    ///
    /// We force the wrapped node's hash to be cached at its top level first,
    /// then pop; the popped child's observable hash must drop back to the
    /// child's own hash, not retain the stale top-level value.
    #[test]
    fn pop_invalidates_memoized_hash(
        index in any::<u8>(),
        child in (0..=MAX_TEST_DEPTH).prop_flat_map(|d| arb_tree(d, TREE_LEAF_BUDGET)),
    ) {
        let child_hash = child.hash(&key());
        let wrapped = child.beneath(index);
        let top_hash = wrapped.hash(&key()); // populate the wrapped node's lazy memo
        prop_assert_ne!(top_hash, child_hash);

        let popped = wrapped.into_children().expect("non-empty");
        let (_, popped_child) = popped.iter().next().expect("singleton");

        prop_assert_eq!(popped_child.hash(&key()), child_hash);
    }

    /// Node hashes match the independent encoding of their kind, prefix,
    /// and ordered child records.
    #[test]
    fn hash_matches_independent_reference(
        tree in (0..=MAX_TEST_DEPTH).prop_flat_map(|d| arb_tree(d, TREE_LEAF_BUDGET)),
    ) {
        prop_assert_eq!(tree.hash(&key()), reference_hash(tree.clone()));
    }

    /// Node diagnostics display compressed prefixes in path order at every length.
    #[test]
    fn debug_prefix_is_in_path_order(
        prefix in vec(any::<u8>(), 0..=super::PATH_LEN),
    ) {
        let mut node = Node::leaf(Version::new(), Message::new(()));
        for &radix in prefix.iter().rev() {
            node = node.beneath(radix);
        }
        let expected = format!("Node {{ prefix: {:?}", hex::encode(prefix));
        let actual = format!("{node:?}");
        prop_assert!(actual.starts_with(&expected));
    }

    /// Every virtual level of every compressed spine hashes exactly as a
    /// canonically constructed tree over the same content at that depth.
    ///
    /// For a random full-depth tree, descend one virtual level at a time
    /// via `into_children`; at every position — materialized or mid-spine —
    /// the exploded node's hash must equal `from_sorted_leaves` rebuilt
    /// from scratch over the leaves beneath it at that depth. This is the
    /// property mixed-shape peer comparison rests on: under the
    /// single-preimage rule it rests on canonical shape, not on the hash
    /// construction itself, so it is pinned here rather than left untested.
    #[test]
    fn every_virtual_level_hashes_canonically(paths in arb_leaf_paths(16)) {
        let paths: Vec<[u8; 32]> = paths.into_iter().collect();
        let tree = canonical_at(0, &paths);
        check_virtual_levels(tree, 0, &paths)?;
    }

    /// A full-length leaf prefix can grow and shrink through every length
    /// without changing retained nodes or reusing a hash from another depth.
    #[test]
    fn full_leaf_prefix_preserves_shared_nodes(path in any::<[u8; 32]>()) {
        let bare = Node::leaf(Version::new(), Message::new(()));
        let mut node = bare.clone();
        for &radix in path.iter().rev() {
            // Cache before each change: adding a level must invalidate it.
            node.hash(&key());
            node = node.beneath(radix);
        }
        prop_assert_eq!(node.hash(&key()), key().leaf(PATH_LEN, &path));
        prop_assert!(bare.get(&[]).is_some());

        for depth in 0..path.len() {
            // Keeping this handle forces prefix removal to copy node state.
            let retained = node.clone();
            let hash = retained.hash(&key());
            let children = node.into_children().expect("a prefix has a child");
            prop_assert_eq!(children.len(), 1);
            let (radix, child) = children.into_iter().next().expect("one child");
            prop_assert_eq!(radix, path[depth]);
            let below = &path[depth + 1..];
            prop_assert_eq!(child.hash(&key()), key().leaf(below.len(), below));
            prop_assert_eq!(retained.hash(&key()), hash);
            prop_assert!(retained.get(&path[depth..]).is_some());
            node = child;
        }
        prop_assert!(node.into_children().is_err());
    }

    /// The owned walk visits each path once, including radix 255 at nested
    /// levels, then remains exhausted after unwinding the whole descent.
    #[test]
    fn owned_walk_finishes_levels_at_the_largest_radix(
        common in any::<[u8; 32]>(),
        depth in 0usize..32,
        mut radixes in vec(any::<u8>(), 0..=16).prop_map(BTreeSet::from_iter),
    ) {
        radixes.extend([0, u8::MAX]);
        let mut paths = BTreeSet::new();
        for radix in radixes {
            let mut path = common;
            path[depth] = radix;
            if depth + 1 < path.len() {
                // Exhaust a child at 255 while its parent may still have
                // siblings to visit. Both levels must advance correctly.
                for child in [0, u8::MAX] {
                    path[depth + 1] = child;
                    paths.insert(path);
                }
            } else {
                paths.insert(path);
            }
        }
        let expected: Vec<_> = paths.into_iter().collect();
        let mut walk = super::RangeOwned::root(
            Some(canonical_at(0, &expected)),
            crate::causally::all(),
        );
        // Bound collection so a repeated final leaf fails without looping.
        let actual: Vec<_> = walk.by_ref().take(expected.len() + 1)
            .map(|(path, _)| path).collect();
        prop_assert_eq!(actual, expected);
        prop_assert!(walk.next().is_none());
        prop_assert!(walk.next().is_none());
    }
}

/// Independent reference for the node-hash convention, computed with literal
/// tag bytes over the public decomposition API.
fn reference_hash(node: Node) -> super::Hash {
    reference_hash_and_height(node).0
}

/// The reference digest of `node` under the fixture key, with the height of
/// its top, derived from the decomposition alone.
///
/// The node is unwound one virtual level at a time via `into_children`: a
/// singleton child map is a compressed-prefix byte (a materialized branch
/// always has >= 2 children, by the path-compression invariant), so its
/// index accumulates onto the reference prefix in path order until the
/// underlying leaf or true branch point is reached. The height is counted
/// rather than read from the node: a leaf's top lies one level above it per
/// prefix byte, and a branch's one level above its children's plus one per
/// prefix byte. The preimage is then assembled by hand —
/// `LEAF_TAG ‖ height ‖ len ‖ prefix` for a leaf,
/// `BRANCH_TAG ‖ height ‖ len ‖ prefix ‖ count(u16 BE) ‖ (radix ‖ hash)*`
/// for a branch — with every child hash computed by the same reference
/// recursively, never by [`Node::hash`], and keyed by [`HashKey::of`].
fn reference_hash_and_height(mut node: Node) -> (super::Hash, usize) {
    const LEAF_TAG: u8 = 0;
    const BRANCH_TAG: u8 = 1;
    let byte = |n: usize| u8::try_from(n).expect("a height or span fits one byte");
    let mut prefix: Vec<u8> = Vec::new();
    loop {
        node = match node.into_children() {
            Err(_leaf) => {
                let height = prefix.len();
                let mut buf = vec![LEAF_TAG, byte(height), byte(prefix.len())];
                buf.extend_from_slice(&prefix);
                return (key().of(&buf), height);
            }
            Ok(children) if children.len() == 1 => {
                let (index, child) = children.into_iter().next().expect("len checked");
                prefix.push(index);
                child
            }
            Ok(children) => {
                let count = u16::try_from(children.len()).expect("fan-out is at most 256");
                let mut records = Vec::new();
                let mut child_heights = BTreeSet::new();
                for (radix, child) in children {
                    let (hash, height) = reference_hash_and_height(child);
                    child_heights.insert(height);
                    records.push(radix);
                    records.extend_from_slice(hash.as_bytes());
                }
                let [child_height] = child_heights.into_iter().collect::<Vec<_>>()[..] else {
                    panic!("every child of a branch sits at one height");
                };
                let height = child_height + 1 + prefix.len();
                let mut buf = vec![BRANCH_TAG, byte(height), byte(prefix.len())];
                buf.extend_from_slice(&prefix);
                buf.extend_from_slice(&count.to_be_bytes());
                buf.extend_from_slice(&records);
                return (key().of(&buf), height);
            }
        };
    }
}

/// The canonical tree over `paths` observed from `depth`, built from
/// scratch by the maximally-compressing bulk constructor.
///
/// Leaf versions are all genesis: the hash convention never commits a
/// version, so varying them adds nothing to the hash properties checked
/// against this reference.
fn canonical_at(depth: usize, paths: &[[u8; 32]]) -> Node {
    let mut entries: Vec<([u8; 32], Option<Node>)> = paths
        .iter()
        .map(|path| (*path, Some(Node::leaf(Version::new(), Message::new(())))))
        .collect();
    Node::from_sorted_leaves(depth, &mut entries)
}

/// Check that every fan stored in `node`'s subtree meets [`heap_slots`] of
/// its length. The recursion is bounded by the key depth.
fn fans_fit(node: &Node) -> Result<(), TestCaseError> {
    if let Some(fan) = node.stored_fan() {
        prop_assert!(
            fan.heap_capacity() <= heap_slots(fan.len()),
            "a fan of {} children holds {} heap slots",
            fan.len(),
            fan.heap_capacity(),
        );
        for child in fan.values() {
            fans_fit(child)?;
        }
    }
    Ok(())
}

/// Children the shortened fan grows to by pushes.
const GROWN: u8 = 200;

/// Children the shortened fan keeps after its removals.
const KEPT: u8 = 3;

/// A fan grown to [`GROWN`] children and cut back to [`KEPT`] still holds
/// the allocation it grew, and [`Node::branch`] releases that excess.
///
/// Before the branch stores it, the fan holds far more than [`heap_slots`]
/// of its length; the stored fan holds at most that.
#[test]
fn branch_releases_the_allocation_a_shortened_fan_kept() {
    let mut fan = Fan::new();
    for radix in 0..GROWN {
        fan.push(radix, Node::leaf(Version::new(), Message::new(())));
    }
    for radix in KEPT..GROWN {
        fan.remove(radix);
    }
    assert!(
        fan.heap_capacity() > heap_slots(fan.len()),
        "removal keeps the {} slots the fan grew",
        fan.heap_capacity(),
    );
    let node = Node::branch(fan).expect("a fan of several children makes a branch");
    let stored = node.stored_fan().expect("a branch stores its fan");
    assert_eq!(stored.len(), usize::from(KEPT));
    assert!(stored.heap_capacity() <= heap_slots(stored.len()));
}

/// Recursively explode `node` (observed from `depth`, holding exactly the
/// leaves at `paths`) one virtual level at a time, checking at every
/// position that its hash matches the canonical from-scratch construction.
fn check_virtual_levels(node: Node, depth: usize, paths: &[[u8; 32]]) -> Result<(), TestCaseError> {
    prop_assert_eq!(node.hash(&key()), canonical_at(depth, paths).hash(&key()));
    match node.into_children() {
        // A bare leaf: the walk consumed the entire 32-byte path.
        Err(_leaf) => prop_assert_eq!(depth, 32),
        Ok(children) => {
            for (radix, child) in children {
                let beneath: Vec<[u8; 32]> = paths
                    .iter()
                    .copied()
                    .filter(|path| path[depth] == radix)
                    .collect();
                check_virtual_levels(child, depth + 1, &beneath)?;
            }
        }
    }
    Ok(())
}

/// The memoized node hash commits the compressed prefix in path order —
/// shallowest byte first, the *reverse* of the shallowest-last in-memory
/// storage — pinned against a hand-built byte-literal preimage.
///
/// `beneath(0xAA)` then `beneath(0xBB)` stores `[0xAA, 0xBB]` (deepest
/// first), but the path from the root reads `0xBB` then `0xAA`; a
/// storage-order preimage would commit the reversed spine and fail here.
#[test]
fn node_hash_preimage_is_in_path_order() {
    const LEAF_TAG: u8 = 0;
    let leaf = Node::leaf(Version::new(), Message::new(()));
    let wrapped = leaf.beneath(0xAA).beneath(0xBB);
    // Height 2: the wrapped leaf's top lies two levels above it.
    assert_eq!(
        wrapped.hash(&key()),
        key().of(&[LEAF_TAG, 2, 2, 0xBB, 0xAA])
    );
}

/// A hand-built two-leaf tree pins the preimages end to end.
///
/// Each leaf commits its height and its length-tagged 29-byte suffix, both
/// 29, and the root branch commits the root's height, 32, its 2-byte shared
/// prefix in path order, the big-endian `u16` child count, and both
/// ascending `radix ‖ hash` records.
#[test]
fn small_tree_hash_matches_byte_literal_preimage() {
    const LEAF_TAG: u8 = 0;
    const BRANCH_TAG: u8 = 1;

    let mut low = [0u8; 32];
    low[..3].copy_from_slice(&[1, 2, 3]);
    let mut high = [0u8; 32];
    high[..4].copy_from_slice(&[1, 2, 7, 9]);
    let tree = canonical_at(0, &[low, high]);

    let leaf_hash = |suffix: &[u8]| {
        let len = u8::try_from(suffix.len()).expect("short suffix");
        let mut buf = vec![LEAF_TAG, len, len];
        buf.extend_from_slice(suffix);
        key().of(&buf)
    };
    // Root: height 32, prefix [1, 2] (path order), two children at radixes
    // 3 and 7, each a leaf whose suffix is the rest of its path.
    let mut preimage = vec![BRANCH_TAG, 32, 2, 1, 2, 0x00, 0x02];
    preimage.push(3);
    preimage.extend_from_slice(leaf_hash(&low[3..]).as_bytes());
    preimage.push(7);
    preimage.extend_from_slice(leaf_hash(&high[3..]).as_bytes());

    assert_eq!(tree.hash(&key()), key().of(&preimage));
}

/// Keep node layout growth explicit: every leaf pays for the largest
/// `Body` variant as well as the inline prefix and cached hash.
///
/// The inline prefix enlarges the node body but avoids a separate allocation
/// for compressed paths.
#[test]
#[cfg(target_pointer_width = "64")]
fn node_inner_stays_within_budget() {
    assert!(std::mem::size_of::<Fan>() <= 40);
    assert!(std::mem::size_of::<super::Body>() <= 160);
    assert!(std::mem::size_of::<super::NodeInner>() <= 224);
}

/// Leaves at `paths`, sorted and deduplicated, observed from `depth`: every
/// path takes `base`'s first `depth` bytes. Each leaf carries its own
/// version, so the trees compared below cannot swap two leaves unseen.
pub(super) fn leaves_below(
    base: [u8; PATH_LEN],
    depth: usize,
    paths: BTreeSet<[u8; PATH_LEN]>,
) -> Vec<([u8; PATH_LEN], Node)> {
    let party = crate::tree::arb::nth_party(0);
    let mut version = Version::new();
    let paths: BTreeSet<_> = paths
        .into_iter()
        .map(|mut path| {
            path[..depth].copy_from_slice(&base[..depth]);
            path
        })
        .collect();
    paths
        .into_iter()
        .map(|path| {
            version.tick(&party);
            (path, Node::leaf(version.clone(), Message::new(())))
        })
        .collect()
}

/// Require `actual` and `expected` to be one tree.
///
/// That is equal compressed prefixes, hashes, and leaf counts everywhere,
/// the same leaf at every leaf position, and the same radices at every
/// branch. The recursion is bounded by the key depth.
fn same_tree(actual: &Node, expected: &Node) -> Result<(), TestCaseError> {
    prop_assert_eq!(
        actual.inner.prefix.as_slice(),
        expected.inner.prefix.as_slice()
    );
    prop_assert_eq!(actual.hash(&key()), expected.hash(&key()));
    prop_assert_eq!(actual.len(), expected.len());
    match (&actual.inner.body, &expected.inner.body) {
        (
            Body::Leaf { version: ours, .. },
            Body::Leaf {
                version: theirs, ..
            },
        ) => {
            prop_assert_eq!(ours, theirs);
        }
        (
            Body::Branch { children: ours, .. },
            Body::Branch {
                children: theirs, ..
            },
        ) => {
            prop_assert_eq!(ours.len(), theirs.len());
            for ((our_radix, ours), (their_radix, theirs)) in ours.iter().zip(theirs.iter()) {
                prop_assert_eq!(our_radix, their_radix);
                same_tree(ours, theirs)?;
            }
        }
        _ => prop_assert!(
            false,
            "both trees hold a leaf or a branch at the same position"
        ),
    }
    Ok(())
}

proptest! {
    /// Building a subtree one leaf at a time over its open right spine
    /// yields exactly the tree the bulk constructor builds from the whole
    /// run, at every starting depth, with every fan fitted.
    ///
    /// The paths come from a narrow alphabet, whose long shared prefixes
    /// exercise compression and deep spines, and from the full byte range,
    /// whose wide fans exercise branch closing at the top.
    #[test]
    fn spine_build_matches_the_bulk_build(
        base in any::<[u8; PATH_LEN]>(),
        depth in 0..PATH_LEN,
        paths in arb_leaf_paths(300),
    ) {
        let leaves = leaves_below(base, depth, paths);
        let mut entries: Vec<_> = leaves
            .iter()
            .map(|(path, leaf)| (*path, Some(leaf.clone())))
            .collect();
        let expected = Node::from_sorted_leaves(depth, &mut entries);
        let mut spine = SortedLeaves::new(depth);
        for (path, leaf) in leaves {
            spine.push(path, leaf);
        }
        let actual = spine.finish().expect("the run holds at least one leaf");
        same_tree(&actual, &expected)?;
        fans_fit(&actual)?;
    }
}

/// A spine build that receives no leaf finishes with no subtree.
#[test]
fn spine_build_of_nothing_is_none() {
    assert!(SortedLeaves::new(0).finish().is_none());
}

/// A spine build rejects a leaf that does not ascend from the previous
/// one: a duplicate path here.
#[test]
#[should_panic(expected = "a leaf run is strictly ascending by path")]
fn spine_build_rejects_a_repeated_path() {
    let mut spine = SortedLeaves::new(0);
    spine.push([1; PATH_LEN], Node::leaf(Version::new(), Message::new(())));
    spine.push([1; PATH_LEN], Node::leaf(Version::new(), Message::new(())));
}

proptest! {
    /// Every node of an arbitrary tree, and every peeled view of its
    /// compressed paths, stores the grandchild count a direct count of its
    /// children's children finds.
    #[test]
    fn grandchildren_match_a_direct_count(
        tree in (0..=MAX_TEST_DEPTH).prop_flat_map(|depth| arb_tree(depth, TREE_LEAF_BUDGET)),
    ) {
        prop_assert!(tree.miscounted_grandchildren().is_none(), "{:?}", tree.miscounted_grandchildren());
    }

    /// Every node of an arbitrary tree, and every peeled view of its
    /// compressed paths, reports as its child count the number of children
    /// its listing yields, none for a leaf.
    ///
    /// The traversal keeps an explicit stack, so it never recurses on the
    /// tree's shape.
    #[test]
    fn child_counts_match_a_direct_listing(
        tree in (0..=MAX_TEST_DEPTH).prop_flat_map(|depth| arb_tree(depth, TREE_LEAF_BUDGET)),
    ) {
        let mut stack = vec![View::from(tree)];
        while let Some(view) = stack.pop() {
            let count = view.child_count();
            let listed = match view.clone().children() {
                Ok(children) => {
                    let children: Vec<_> = children.map(|(_, child)| child).collect();
                    let listed = children.len();
                    stack.extend(children);
                    listed
                }
                Err(_) => 0,
            };
            prop_assert_eq!(count, listed, "{:?}", view);
        }
    }

    /// Both bulk builders, over the runs a reply decoder assembles, build
    /// trees whose every node stores the grandchild count a direct count
    /// finds, at every starting depth.
    #[test]
    fn built_runs_count_grandchildren_exactly(
        base in any::<[u8; PATH_LEN]>(),
        depth in 0..PATH_LEN,
        paths in arb_leaf_paths(300),
    ) {
        let leaves = leaves_below(base, depth, paths);
        let mut entries: Vec<_> = leaves
            .iter()
            .map(|(path, leaf)| (*path, Some(leaf.clone())))
            .collect();
        let bulk = Node::from_sorted_leaves(depth, &mut entries);
        let mut spine = SortedLeaves::new(depth);
        for (path, leaf) in leaves {
            spine.push(path, leaf);
        }
        let spine = spine.finish().expect("the run holds at least one leaf");
        for built in [bulk, spine] {
            prop_assert!(built.miscounted_grandchildren().is_none(), "{:?}", built.miscounted_grandchildren());
        }
    }
}

/// A full fan of branches, each over a full fan of leaves, stores the
/// largest grandchild count a branch can have, 65,536, which its `u16`
/// holds exactly as 65,535.
#[test]
fn a_full_fan_of_full_fans_counts_every_grandchild() {
    let paths: Vec<[u8; PATH_LEN]> = (0..=u8::MAX)
        .flat_map(|top| {
            (0..=u8::MAX).map(move |next| {
                let mut path = [0; PATH_LEN];
                path[0] = top;
                path[1] = next;
                path
            })
        })
        .collect();
    let root = canonical_at(0, &paths);
    assert_eq!(super::View::from(root.clone()).grandchild_count(), 65_536);
    assert!(root.miscounted_grandchildren().is_none());
}
