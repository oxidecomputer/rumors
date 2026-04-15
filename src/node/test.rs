use std::collections::{BTreeSet, HashMap};

use proptest::prelude::*;

use super::*;

/// Compute the root hash of the fully-expanded (un-path-compressed) 256-ary
/// trie over the given set of values. For every unique blake3 path, a leaf
/// sentinel of all-0xff bytes sits at depth 32; at each level above, a 256-way
/// branch hashes its child slots (0x00-filled where absent, recursive hash
/// where present). This is the ground truth that the compressed tree's root
/// hash must match.
fn reference_hash(values: &[Vec<u8>]) -> blake3::Hash {
    const LEAF_SENTINEL: [u8; 32] = [0xff; 32];
    const ZERO: [u8; 32] = [0x00; 32];

    let hash_branch = |children: &HashMap<u8, blake3::Hash>| -> blake3::Hash {
        let mut hasher = blake3::Hasher::new();
        for i in u8::MIN..=u8::MAX {
            match children.get(&i) {
                Some(h) => hasher.update(h.as_bytes()),
                None => hasher.update(&ZERO),
            };
        }
        hasher.finalize()
    };

    // Level 32 (the value level): every distinct path maps to the sentinel.
    let paths: BTreeSet<[u8; 32]> = values.iter().map(|v| *blake3::hash(v).as_bytes()).collect();

    if paths.is_empty() {
        return hash_branch(&HashMap::new());
    }

    let mut current: HashMap<Vec<u8>, blake3::Hash> = paths
        .into_iter()
        .map(|p| (p.to_vec(), LEAF_SENTINEL.into()))
        .collect();

    // Fold upward one level at a time: group entries by the prefix they share
    // at the next-shallower depth, then hash each group as a 256-way branch.
    for level in (0..32).rev() {
        let mut groups: HashMap<Vec<u8>, HashMap<u8, blake3::Hash>> = HashMap::new();
        for (prefix, hash) in current {
            let new_prefix = prefix[..level].to_vec();
            let byte = prefix[level];
            groups.entry(new_prefix).or_default().insert(byte, hash);
        }
        current = groups
            .into_iter()
            .map(|(prefix, children)| (prefix, hash_branch(&children)))
            .collect();
    }

    *current
        .get(&Vec::<u8>::new())
        .expect("exactly one root entry")
}

/// A value inserted into an empty tree produces a leaf that hashes without
/// panicking. This exercises the Vacant-branch case, which previously panicked
/// on the first insert.
#[test]
fn single_insert_does_not_panic() {
    let mut tree: Tree<u64> = Tree::default();
    tree.insert(0, 1, Bytes::from_static(b"hello"));
    let _ = tree.root.hash();
}

/// Reinserting the same (party, value) at an older version must be a no-op:
/// the stored leaf keeps its newer version, and the tree shape is unchanged.
#[test]
fn older_reinsert_is_skipped() {
    let mut tree: Tree<u64> = Tree::default();
    tree.insert(0, 5, Bytes::from_static(b"hello"));
    let before = tree.root.hash();
    tree.insert(0, 3, Bytes::from_static(b"hello"));
    let after = tree.root.hash();
    assert_eq!(before.as_bytes(), after.as_bytes());
}

/// An empty tree's root hash must match the reference (256 zero slots).
#[test]
fn empty_tree_hash_matches_reference() {
    let tree: Tree<u64> = Tree::default();
    let tree_hash = tree.root.hash();
    let reference = reference_hash(&[]);
    assert_eq!(tree_hash.as_bytes(), reference.as_bytes());
}

/// A single inserted value must hash identically to the uncompressed trie
/// containing just that value. This exercises Leaf::hash path compression
/// with a maximal 31-byte leaf prefix.
#[test]
fn single_value_hash_matches_reference() {
    let value = b"hello".to_vec();
    let mut tree: Tree<u64> = Tree::default();
    tree.insert(0, 1, Bytes::copy_from_slice(&value));
    let tree_hash = tree.root.hash();
    let reference = reference_hash(&[value]);
    assert_eq!(tree_hash.as_bytes(), reference.as_bytes());
}

proptest! {
    /// The compressed tree's root hash must equal the hash computed over the
    /// fully-expanded uncompressed trie, for any set of inserted values. This
    /// is the ground-truth invariant for path compression: the hash depends
    /// on the set of leaves, not on how the tree chooses to compress them.
    #[test]
    fn compressed_hash_matches_reference(
        values in proptest::collection::vec(any::<Vec<u8>>(), 0..16),
    ) {
        let uniques: Vec<Vec<u8>> =
            values.into_iter().collect::<BTreeSet<_>>().into_iter().collect();

        let mut tree: Tree<u64> = Tree::default();
        for (i, v) in uniques.iter().enumerate() {
            tree.insert(0, (i as u64) + 1, Bytes::copy_from_slice(v));
        }
        let tree_hash = tree.root.hash();
        let reference = reference_hash(&uniques);
        prop_assert_eq!(tree_hash.as_bytes(), reference.as_bytes());
    }

    /// Inserting any sequence of values under a single party with strictly
    /// increasing versions must not panic, and the resulting root hash must
    /// be computable. A regression guard against the empty-branch descent bug.
    #[test]
    fn insert_sequence_does_not_panic(
        values in proptest::collection::vec(any::<Vec<u8>>(), 0..32),
    ) {
        let mut tree: Tree<u64> = Tree::default();
        for (i, v) in values.iter().enumerate() {
            tree.insert(0, (i as u64) + 1, Bytes::copy_from_slice(v));
        }
        let _ = tree.root.hash();
    }

    /// For a fixed set of distinct values inserted under a single party, the
    /// root hash is determined by the set, not the insertion order: every
    /// value occupies a path uniquely determined by its blake3 hash, so
    /// reordering inserts must yield the same shape and the same root hash.
    #[test]
    fn insert_is_order_independent(
        values in proptest::collection::vec(any::<Vec<u8>>(), 0..32),
    ) {
        let uniques: Vec<Vec<u8>> =
            values.into_iter().collect::<BTreeSet<_>>().into_iter().collect();

        let mut forward: Tree<u64> = Tree::default();
        for (i, v) in uniques.iter().enumerate() {
            forward.insert(0, (i as u64) + 1, Bytes::copy_from_slice(v));
        }

        let mut reverse: Tree<u64> = Tree::default();
        for (i, v) in uniques.iter().rev().enumerate() {
            reverse.insert(0, (i as u64) + 1, Bytes::copy_from_slice(v));
        }

        let forward_hash = forward.root.hash();
        let reverse_hash = reverse.root.hash();
        prop_assert_eq!(forward_hash.as_bytes(), reverse_hash.as_bytes());
    }
}

/// Derive a deterministic `path_len`-byte path from arbitrary input bytes by
/// taking a prefix of `blake3(raw)`. Distinct inputs almost always map to
/// distinct paths, but collisions just mean re-insertion at the same address
/// (which is fine because the stored value also derives from the path).
fn derive_path(raw: &[u8], path_len: usize) -> Vec<u8> {
    blake3::hash(raw).as_bytes()[..path_len].to_vec()
}

/// Return `true` if no node in the tree violates path compression: branches
/// must have at least two children (except an empty branch at the root,
/// which is the empty-tree representation), and there are no one-child
/// branches anywhere.
fn is_max_compressed<P>(root: &Node<P>) -> bool {
    fn check<P>(node: &Node<P>, is_root: bool) -> bool {
        match &node.children {
            Children::Leaf(_) => true,
            Children::Branch(map) => {
                if map.len() == 1 {
                    return false;
                }
                if !is_root && map.is_empty() {
                    return false;
                }
                map.values().all(|arc| check(arc, false))
            }
        }
    }
    check(root, true)
}
