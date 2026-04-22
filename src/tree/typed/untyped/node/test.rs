use std::collections::BTreeSet;
use std::sync::Arc;

use imbl::OrdMap;
use proptest::collection::{btree_set, vec};
use proptest::prelude::*;

use crate::{Message, Version};

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
fn arb_tree<P>(depth: usize, budget: usize) -> BoxedStrategy<Node<P, ()>>
where
    P: Arbitrary + Ord + Clone + AsRef<[u8]> + 'static,
{
    if depth == 0 {
        // Bytes payload is not examined at this abstraction layer, so we
        // stuff in a fixed empty value rather than generating one.
        (any::<P>(), any::<u64>())
            .prop_map(|(party, version)| Node::leaf((party, version).into(), Message::new(())))
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
                let pairs: OrdMap<u8, Node<P, ()>> = indices.into_iter().zip(subtrees).collect();
                Node::branch(pairs).expect("branch input has >= 1 child")
            })
            .boxed()
    }
}

/// Recursively clear every cached hash in the tree: the children-level hash
/// at each node, and every intermediate prefix-level hash along compressed
/// paths. After this runs, `hash()` must recompute from scratch; comparing
/// the pre-clear and post-clear results is how we catch hash-invalidation
/// bugs (including stale intermediate caches along a prefix). Uses private
/// field access (only available to test code in this child module).
fn clear_hash_cache<P: Ord + Clone + AsRef<[u8]>>(node: &mut Node<P, ()>) {
    let inner = Arc::make_mut(&mut node.inner);
    inner.children_hash.invalidate();
    for (_, cached) in &mut inner.prefix {
        cached.invalidate();
    }
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
/// (path, version, leaf) triple. Paths list the child indices from
/// shallowest to deepest, matching the order in which `into_children`
/// yields them. The version is the leaf's own version as recorded by
/// `Node::leaf`, and is preserved across path compression because
/// `into_children` never mutates `version` — only `prefix` and `hash`.
fn enumerate_leaves<P: Ord + Clone + AsRef<[u8]>>(
    node: Node<P, ()>,
    path: Vec<u8>,
) -> Vec<(Vec<u8>, Version<P>, Message<()>)> {
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
            let version = leaf_node.version().clone();
            let leaf = leaf_node
                .as_leaf()
                .expect("into_children returned Err only for leaves")
                .clone();
            vec![(path, version, leaf)]
        }
    }
}

/// Recursively traverse a tree via the public smart constructors, mapping
/// each leaf's bytes through `f` and rebuilding the tree bottom-up. With
/// `f = |b| b.clone()` this is an identity functor that decomposes and
/// rebuilds; with a constant `f` it swaps every leaf's payload. The
/// branching structure and every node's `version` are preserved exactly:
/// leaves pass their original version back into `Node::leaf`, and branch
/// versions are recomputed by `Node::branch` from the same per-child
/// versions we started with.
fn rebuild_with<P, F>(node: Node<P, ()>, f: &F) -> Node<P, ()>
where
    P: Ord + Clone + AsRef<[u8]>,
    F: Fn(&Message<()>) -> Message<()>,
{
    let version = node.version().clone();
    match node.into_children() {
        Err(leaf_node) => {
            let leaf = leaf_node
                .as_leaf()
                .expect("into_children returned Err only for leaves");
            Node::leaf(version, f(leaf))
        }
        Ok(children) => {
            let rebuilt: OrdMap<u8, Node<P, ()>> = children
                .into_iter()
                .map(|(k, v)| (k, rebuild_with(v, f)))
                .collect();
            Node::branch(rebuilt).expect("non-empty")
        }
    }
}

/// A branch with zero children is not a legal node: the smart constructor
/// must reject it rather than materialize an empty `Branch`. This is the
/// "no empty nodes anywhere" half of the path-compression invariant; the
/// one-child case is handled by `beneath`-collapse instead.
#[test]
fn empty_branch_is_none() {
    let empty: OrdMap<u8, Node<String, ()>> = OrdMap::new();
    assert!(Node::branch(empty).is_none());
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

    /// The cached hash must agree with a fresh recomputation from scratch.
    /// After computing `hash()` to populate every node's cache, recursively
    /// invalidating that cache, and recomputing, both values must match.
    /// This witnesses that the cached blake3 is a pure function of the
    /// node's structure — no stale entry can survive invalidation and no
    /// compute path disagrees with the cache it populates.
    #[test]
    fn hash_unchanged_by_cache_clear(
        tree in (0..=MAX_TEST_DEPTH).prop_flat_map(|d| arb_tree::<String>(d, TREE_LEAF_BUDGET)),
    ) {
        let cached = tree.hash();
        let mut tree = tree;
        clear_hash_cache(&mut tree);
        let fresh = tree.hash();
        prop_assert_eq!(cached, fresh);
    }

    /// Decomposing a tree into its leaves via `into_children` and rebuilding
    /// bottom-up with `Node::leaf` + `Node::branch` must produce a tree
    /// with the same root hash and the same root version as the original.
    /// This is the strongest statement that hash and version are pure
    /// functions of the public structural API: any node we can take apart,
    /// we can put back together, and the observable invariants are the
    /// same. Path-compressed single-child branches round-trip through
    /// `branch`→`beneath`, so this also exercises the compression path.
    #[test]
    fn decompose_and_rebuild_preserves_hash_and_version(
        tree in (0..=MAX_TEST_DEPTH).prop_flat_map(|d| arb_tree::<String>(d, TREE_LEAF_BUDGET)),
    ) {
        let hash_before = tree.hash();
        let version_before = tree.version().clone();
        let rebuilt = rebuild_with(tree, &|b| b.clone());
        prop_assert_eq!(rebuilt.hash(), hash_before);
        prop_assert_eq!(rebuilt.version(), &version_before);
    }

    /// Enumerating a generated tree's leaves via the public API yields
    /// exactly as many leaves as the tree holds, every leaf sits at path
    /// length equal to the generated depth, and no two leaves share a
    /// path. This pins down three independent claims in one place: that
    /// `into_children` unpacks exactly one prefix byte per step, that all
    /// leaves live at a common depth (the `arb_tree` contract), and that
    /// branch indices are distinct so leaf paths are unique.
    #[test]
    fn leaf_enumeration_has_expected_shape(
        (depth, tree) in (0..=MAX_TEST_DEPTH)
            .prop_flat_map(|d| (Just(d), arb_tree::<String>(d, TREE_LEAF_BUDGET))),
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

    /// Every node's `version` is the pointwise-max join of its descendant
    /// leaves' versions. At the root this means: (a) every leaf's version
    /// is ≤ the root version, and (b) the root version is exactly the
    /// join of all leaf versions — no component is larger, so the root
    /// never over-reports causality. `Node::branch` computes this via
    /// `Version::new(children.versions)` and `beneath` leaves it alone,
    /// so the invariant has to hold at every layer of the construction.
    #[test]
    fn version_is_join_of_leaf_versions(
        tree in (0..=MAX_TEST_DEPTH).prop_flat_map(|d| arb_tree::<String>(d, TREE_LEAF_BUDGET)),
    ) {
        let root_version = tree.version().clone();
        let leaves = enumerate_leaves(tree, Vec::new());

        for (_, v, _) in &leaves {
            prop_assert!(v <= &root_version);
        }

        let joined = Version::new(leaves.iter().map(|(_, v, _)| v.clone()));
        prop_assert_eq!(joined, root_version);
    }

    /// A single top-level `into_children` → `Node::branch` round-trip
    /// preserves the tree's hash and version. Strictly subsumed by the
    /// recursive decompose/rebuild property above, but localizes failures
    /// to the top-level merge step: if this fails while the full rebuild
    /// passes, the bug is in how `Node::branch` reconstructs a branch
    /// from a children map, not in recursion.
    #[test]
    fn branch_into_children_round_trips(
        tree in (1..=MAX_TEST_DEPTH).prop_flat_map(|d| arb_tree::<String>(d, TREE_LEAF_BUDGET)),
    ) {
        let hash_before = tree.hash();
        let version_before = tree.version().clone();
        let children = tree
            .into_children()
            .expect("depth >= 1 so the tree is not a bare leaf");
        let rebuilt = Node::branch(children).expect("children map is non-empty");
        prop_assert_eq!(rebuilt.hash(), hash_before);
        prop_assert_eq!(rebuilt.version(), &version_before);
    }

    /// Wrapping a child in N nested singleton branches accumulates an
    /// N-byte compressed prefix above it. The observable hash must equal
    /// the result of N successive virtual-branch wraps of the child's
    /// hash, and every traversal of that prefix — first-call (cold cache,
    /// populates intermediates), repeat-call (hot cache hits at every
    /// level), and post-clear (re-populates intermediates from scratch) —
    /// must return the exact same value. Targets the intermediate
    /// prefix-cache populate-and-reuse path that doesn't fire when prefix
    /// length is at most 1.
    #[test]
    fn nested_singleton_wraps_match_repeated_virtual_branch_hash(
        indices in vec(any::<u8>(), 2..=8),
        child in (0..=MAX_TEST_DEPTH).prop_flat_map(|d| arb_tree::<String>(d, TREE_LEAF_BUDGET)),
    ) {
        let mut expected = child.hash();
        for &index in &indices {
            let mut buf = [0u8; 256 * 32];
            buf[index as usize * 32..][..32].copy_from_slice(expected.as_bytes());
            expected = blake3::hash(&buf);
        }

        let mut wrapped = child;
        for &index in &indices {
            wrapped = Node::branch(OrdMap::from_iter([(index, wrapped)]))
                .expect("one-child branch is non-empty");
        }

        // Cold-cache traversal: every prefix entry's intermediate cache
        // is empty, so each level computes-and-populates.
        prop_assert_eq!(wrapped.hash(), expected);
        // Hot-cache traversal: every prefix entry's intermediate cache
        // is populated, so each level should be a clean read.
        prop_assert_eq!(wrapped.hash(), expected);
        // Post-clear traversal: reset every cache (children-level and
        // every intermediate) and recompute from scratch. If any cache
        // were storing the wrong intermediate value or were re-used
        // without proper invalidation, this would diverge.
        let mut cleared = wrapped.clone();
        clear_hash_cache(&mut cleared);
        prop_assert_eq!(cleared.hash(), expected);
    }

    /// Popping the topmost compressed-prefix byte (via `into_children`)
    /// must produce a node whose hash matches a freshly-built node with
    /// the same children and the shortened prefix. With intermediate
    /// hash caching, the surviving prefix entries' caches must remain
    /// valid across the pop: their byte sequence and the children
    /// underneath are unchanged. If any surviving intermediate were
    /// stale, the recomputed top hash would diverge from the reference.
    #[test]
    fn pop_top_byte_preserves_intermediate_cache_validity(
        indices in btree_set(any::<u8>(), 2..=8),
        child in (0..=MAX_TEST_DEPTH).prop_flat_map(|d| arb_tree::<String>(d, TREE_LEAF_BUDGET)),
    ) {
        let indices: Vec<u8> = indices.into_iter().collect();

        // Build the wrapped node and force every intermediate cache to
        // populate by computing the hash once.
        let mut wrapped = child.clone();
        for &index in &indices {
            wrapped = Node::branch(OrdMap::from_iter([(index, wrapped)]))
                .expect("one-child branch is non-empty");
        }
        let _ = wrapped.hash();

        // Pop the topmost byte. The returned map has exactly one entry
        // because `wrapped` was a singleton-branch chain; the entry's
        // key is the popped byte and its value is the same node with a
        // one-shorter prefix and intermediate caches still populated.
        let mut popped_children = wrapped.into_children().expect("non-empty");
        prop_assert_eq!(popped_children.len(), 1);
        let (popped_byte, popped) = popped_children
            .iter()
            .next()
            .map(|(k, v)| (*k, v.clone()))
            .expect("singleton");
        popped_children.remove(&popped_byte);
        prop_assert_eq!(popped_byte, *indices.last().expect("non-empty indices"));

        // Build a reference node with the same children but the shortened
        // prefix from scratch (no shared cache state).
        let mut reference = child;
        for &index in &indices[..indices.len() - 1] {
            reference = Node::branch(OrdMap::from_iter([(index, reference)]))
                .expect("one-child branch is non-empty");
        }

        prop_assert_eq!(popped.hash(), reference.hash());
    }

    /// A one-child branch at index `i` hashes as a "virtual" 256-slot
    /// branch with slot `i` holding the child's hash and every other
    /// slot holding `[0x00; 32]`. `Node::branch` collapses the one-child
    /// case into `beneath`, which path-compresses by pushing a byte onto
    /// the child's prefix rather than materializing a branch node. The
    /// hash-computation loop must unwind that prefix byte-by-byte to
    /// produce the same result a materialized single-child branch would.
    #[test]
    fn singleton_branch_matches_virtual_branch_hash(
        index in any::<u8>(),
        child in (0..=MAX_TEST_DEPTH).prop_flat_map(|d| arb_tree::<String>(d, TREE_LEAF_BUDGET)),
    ) {
        let child_hash = child.hash();
        let wrapped = Node::branch(OrdMap::from_iter([(index, child)]))
            .expect("one-child branch is non-empty");

        let mut buf = [0u8; 256 * 32];
        buf[index as usize * 32..][..32].copy_from_slice(child_hash.as_bytes());
        let expected = blake3::hash(&buf);

        prop_assert_eq!(wrapped.hash(), expected);
    }
}
