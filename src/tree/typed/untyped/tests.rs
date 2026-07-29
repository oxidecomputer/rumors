use std::collections::BTreeSet;

use proptest::collection::{btree_set, vec};
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

use crate::tree::arb::arb_version;
use crate::{Version, message::Message};

use super::{Node, fan::Fan};

/// Upper bound on the depth of trees generated in property tests.
///
/// Each test samples a depth in `0..=MAX_TEST_DEPTH` so that proptest shrinks
/// tree height as well as structure when a counterexample is found. Kept
/// modest to keep generated structures small and shrinking fast.
const MAX_TEST_DEPTH: usize = 4;

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
const TREE_LEAF_BUDGET: usize = 16;

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
fn arb_tree(depth: usize, budget: usize) -> BoxedStrategy<Node<()>> {
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
        btree_set(any::<u8>(), 1..=max_n)
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
                let children: Fan<()> = indices.into_iter().zip(subtrees).collect();
                Node::branch(children).expect("branch input has >= 1 child")
            })
            .boxed()
    }
}

/// Walk a tree via the public `into_children` API and collect every
/// (path, version, leaf) triple.
///
/// Paths list the child indices from
/// shallowest to deepest, matching the order in which `into_children`
/// yields them. The version is the leaf's own version as recorded by
/// `Node::leaf`, and is preserved across path compression because
/// `into_children` never mutates `version` — only `prefix`.
fn enumerate_leaves(node: Node<()>, path: Vec<u8>) -> Vec<(Vec<u8>, Version, Message<()>)> {
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

/// Recursively traverse a tree via the public smart constructors, mapping
/// each leaf's bytes through `f` and rebuilding the tree bottom-up.
///
/// With
/// `f = |b| b.clone()` this is an identity functor that decomposes and
/// rebuilds; with a constant `f` it swaps every leaf's payload. The
/// branching structure and every node's `version` are preserved exactly:
/// leaves pass their original version back into `Node::leaf`, and branch
/// versions are recomputed by `Node::branch` from the same per-child
/// versions we started with.
fn rebuild_with<F>(node: Node<()>, f: &F) -> Node<()>
where
    F: Fn(&Message<()>) -> Message<()>,
{
    let version = node.ceiling().clone();
    match node.into_children() {
        Err(leaf_node) => {
            let leaf = leaf_node
                .as_leaf()
                .expect("into_children returned Err only for leaves");
            Node::leaf(version, f(leaf))
        }
        Ok(children) => {
            let rebuilt: Fan<()> = children
                .into_iter()
                .map(|(k, v)| (k, rebuild_with(v, f)))
                .collect();
            Node::branch(rebuilt).expect("non-empty")
        }
    }
}

/// A branch with zero children is not a legal node: the smart constructor
/// must reject it rather than materialize an empty `Branch`.
///
/// This is the
/// "no empty nodes anywhere" half of the path-compression invariant; the
/// one-child case is handled by `beneath`-collapse instead.
#[test]
fn empty_branch_is_none() {
    let empty: Fan<()> = Fan::new();
    assert!(Node::branch(empty).is_none());
}

proptest! {
    /// Any tree built from the public smart constructors satisfies the
    /// path-compression invariant: every branch has at least two children.
    #[test]
    fn arbitrary_tree_is_max_compressed(
        tree in (0..=MAX_TEST_DEPTH).prop_flat_map(|depth| arb_tree(depth, TREE_LEAF_BUDGET)),
    ) {
        prop_assert!(tree.is_max_compressed());
    }

    /// Decomposing a tree into its leaves via `into_children` and rebuilding
    /// bottom-up with `Node::leaf` + `Node::branch` must produce a tree
    /// with the same root hash and the same root version as the original.
    ///
    /// This is the strongest statement that hash and version are pure
    /// functions of the public structural API: any node we can take apart,
    /// we can put back together, and the observable invariants are the
    /// same. Path-compressed single-child branches round-trip through
    /// `branch`→`beneath`, so this also exercises the compression path.
    #[test]
    fn decompose_and_rebuild_preserves_hash_and_version(
        tree in (0..=MAX_TEST_DEPTH).prop_flat_map(|d| arb_tree(d, TREE_LEAF_BUDGET)),
    ) {
        let hash_before = tree.hash();
        let version_before = tree.ceiling().clone();
        let rebuilt = rebuild_with(tree, &|b| b.clone());
        prop_assert_eq!(rebuilt.hash(), hash_before);
        prop_assert_eq!(rebuilt.ceiling(), &version_before);
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

    /// Every node's ceiling is the join of its descendant leaves' versions.
    ///
    /// At the root this means: (a) every leaf's version is ≤ the root
    /// ceiling, and (b) the root ceiling is exactly the join of all leaf
    /// versions, with no larger component, so the root never over-reports
    /// causality. A branch's ceiling is computed lazily from its children's
    /// (see `Node::ceiling`) and `beneath` leaves it alone, so the invariant
    /// must hold at every layer of the construction.
    #[test]
    fn version_is_join_of_leaf_versions(
        tree in (0..=MAX_TEST_DEPTH).prop_flat_map(|d| arb_tree(d, TREE_LEAF_BUDGET)),
    ) {
        let root_version = tree.ceiling().clone();
        let leaves = enumerate_leaves(tree, Vec::new());

        for (_, v, _) in &leaves {
            prop_assert!(v <= root_version);
        }

        let joined = leaves
            .iter()
            .map(|(_, v, _)| v.clone())
            .fold(Version::new(), |acc, v| acc | v);
        prop_assert_eq!(joined, root_version);
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
        prop_assert_eq!(wrapped.hash(), reference_hash(wrapped.clone()));
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
        indices in btree_set(any::<u8>(), 2..=8),
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

        prop_assert_eq!(popped.hash(), reference.hash());
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
        let child_hash = child.hash();
        let wrapped = Node::branch(Fan::unit(index, child))
            .expect("one-child branch is non-empty");

        prop_assert_ne!(wrapped.hash(), child_hash);
        prop_assert_eq!(wrapped.hash(), reference_hash(wrapped.clone()));
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
        let child_hash = child.hash(); // populate the child's lazy memo
        let wrapped = child.beneath(index);

        prop_assert_ne!(wrapped.hash(), child_hash);
        prop_assert_eq!(wrapped.hash(), reference_hash(wrapped.clone()));
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
        let child_hash = child.hash();
        let wrapped = child.beneath(index);
        let top_hash = wrapped.hash(); // populate the wrapped node's lazy memo
        prop_assert_ne!(top_hash, child_hash);

        let popped = wrapped.into_children().expect("non-empty");
        let (_, popped_child) = popped.iter().next().expect("singleton");

        prop_assert_eq!(popped_child.hash(), child_hash);
    }

    /// Every generated tree's hash equals the independent literal-preimage
    /// reference.
    ///
    /// One preimage per node — kind tag, length-tagged path-order prefix,
    /// and, for a branch, a big-endian `u16` child count followed by
    /// ascending 17-byte `radix ‖ hash` records.
    #[test]
    fn hash_matches_independent_reference(
        tree in (0..=MAX_TEST_DEPTH).prop_flat_map(|d| arb_tree(d, TREE_LEAF_BUDGET)),
    ) {
        prop_assert_eq!(tree.hash(), reference_hash(tree.clone()));
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
    fn every_virtual_level_hashes_canonically(paths in full_depth_paths()) {
        let paths: Vec<[u8; 32]> = paths.into_iter().collect();
        let tree = canonical_at(0, &paths);
        check_virtual_levels(tree, 0, &paths)?;
    }
}

/// Independent reference for the node-hash convention, computed with literal
/// tag bytes over the public decomposition API.
///
/// The node is unwound one virtual level at a time via `into_children`: a
/// singleton child map is a compressed-prefix byte (a materialized branch
/// always has >= 2 children, by the path-compression invariant), so its
/// index accumulates onto the reference prefix in path order until the
/// underlying leaf or true branch point is reached. The preimage is then
/// assembled by hand — `LEAF_TAG ‖ len ‖ prefix` for a leaf,
/// `BRANCH_TAG ‖ len ‖ prefix ‖ count(u16 BE) ‖ (radix ‖ hash)*` for a
/// branch — with every child hash computed by the same reference
/// recursively, never by [`Node::hash`].
fn reference_hash(mut node: Node<()>) -> super::Hash {
    const LEAF_TAG: u8 = 0;
    const BRANCH_TAG: u8 = 1;
    let mut prefix: Vec<u8> = Vec::new();
    loop {
        node = match node.into_children() {
            Err(_leaf) => {
                let mut buf = vec![LEAF_TAG, u8::try_from(prefix.len()).expect("short prefix")];
                buf.extend_from_slice(&prefix);
                return super::Hash::of(&buf);
            }
            Ok(children) if children.len() == 1 => {
                let (index, child) = children.into_iter().next().expect("len checked");
                prefix.push(index);
                child
            }
            Ok(children) => {
                let mut buf = vec![
                    BRANCH_TAG,
                    u8::try_from(prefix.len()).expect("short prefix"),
                ];
                buf.extend_from_slice(&prefix);
                let count = u16::try_from(children.len()).expect("fan-out is at most 256");
                buf.extend_from_slice(&count.to_be_bytes());
                for (radix, child) in children {
                    buf.push(radix);
                    buf.extend_from_slice(reference_hash(child).as_bytes());
                }
                return super::Hash::of(&buf);
            }
        };
    }
}

/// Full 32-byte leaf-path sets for the virtual-level walk.
///
/// Paths over a tiny alphabet share long prefixes, forcing deep compressed
/// spines and branch points at many depths; paths over the full alphabet
/// mostly diverge at the top, forcing wide fans. Both shapes matter.
fn full_depth_paths() -> impl Strategy<Value = BTreeSet<[u8; 32]>> {
    prop_oneof![
        btree_set(proptest::array::uniform32(0u8..3), 1..=16),
        btree_set(proptest::array::uniform32(any::<u8>()), 1..=16),
    ]
}

/// The canonical tree over `paths` observed from `depth`, built from
/// scratch by the maximally-compressing bulk constructor.
///
/// Leaf versions are all genesis: the hash convention never commits a
/// version, so varying them adds nothing to the hash properties checked
/// against this reference.
fn canonical_at(depth: usize, paths: &[[u8; 32]]) -> Node<()> {
    let mut entries: Vec<([u8; 32], Option<Node<()>>)> = paths
        .iter()
        .map(|path| (*path, Some(Node::leaf(Version::new(), Message::new(())))))
        .collect();
    Node::from_sorted_leaves(depth, &mut entries)
}

/// Recursively explode `node` (observed from `depth`, holding exactly the
/// leaves at `paths`) one virtual level at a time, checking at every
/// position that its hash matches the canonical from-scratch construction.
fn check_virtual_levels(
    node: Node<()>,
    depth: usize,
    paths: &[[u8; 32]],
) -> Result<(), TestCaseError> {
    prop_assert_eq!(node.hash(), canonical_at(depth, paths).hash());
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
    assert_eq!(wrapped.hash(), super::Hash::of(&[LEAF_TAG, 2, 0xBB, 0xAA]),);
}

/// A hand-built two-leaf tree pins the preimages end to end.
///
/// Each leaf commits its length-tagged 29-byte suffix, and the root branch
/// commits its 2-byte shared prefix in path order, the big-endian `u16`
/// child count, and both ascending `radix ‖ hash` records.
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
        let mut buf = vec![LEAF_TAG, u8::try_from(suffix.len()).expect("short suffix")];
        buf.extend_from_slice(suffix);
        super::Hash::of(&buf)
    };
    // Root: prefix [1, 2] (path order), two children at radixes 3 and 7,
    // each a leaf whose suffix is the rest of its path.
    let mut preimage = vec![BRANCH_TAG, 2, 1, 2, 0x00, 0x02];
    preimage.push(3);
    preimage.extend_from_slice(leaf_hash(&low[3..]).as_bytes());
    preimage.push(7);
    preimage.extend_from_slice(leaf_hash(&high[3..]).as_bytes());

    assert_eq!(tree.hash(), super::Hash::of(&preimage));
}

/// Growing the per-node allocation price must be a deliberate, reviewed
/// decision, never a silent regression.
///
/// Every node allocation pays `NodeInner`'s full size: the `Children`
/// enum takes its largest variant, so leaves (the most numerous nodes)
/// carry the branch variant's width, fan included.
///
/// Measured on 64-bit: `Fan<()>` = 40 (8 capacity + 2 inline 16-byte
/// entries), `Children<()>` = 136 (three memo cells, the leaf count, and
/// the fan), `NodeInner<()>` = 184 (prefix `Vec` + hash memo + children).
#[test]
#[cfg(target_pointer_width = "64")]
fn node_inner_stays_within_budget() {
    assert!(std::mem::size_of::<Fan<()>>() <= 40);
    assert!(std::mem::size_of::<super::Children<()>>() <= 136);
    assert!(std::mem::size_of::<super::NodeInner<()>>() <= 184);
}
