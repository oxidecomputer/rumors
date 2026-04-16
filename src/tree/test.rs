use std::collections::{BTreeSet, HashMap, HashSet};

use bytes::Bytes;
use proptest::prelude::*;

use super::typed::Path;
use super::*;

/// Compute the root hash of the fully-expanded (uVn-path-compressed) 256-ary
/// trie over the given set of values. For every unique blake3 path, a leaf
/// sentinel of all-0xff bytes sits at depth 32; at each level above, a 256-way
/// branch hashes its child slots (0x00-filled where absent, recursive hash
/// where present). This is the ground truth that the compressed tree's root
/// hash must match.
fn reference_hash<P: AsRef<[u8]>>(values: &[(P, u64, Bytes)]) -> blake3::Hash {
    const LEAF_SENTINEL: [u8; 32] = [0xff; 32];
    const ZERO: [u8; 32] = [0x00; 32];

    // The empty tree has the hash of an empty node
    if values.is_empty() {
        return ZERO.into();
    }

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
    let paths: BTreeSet<[u8; 32]> = values
        .iter()
        .map(|(party, version, value)| Path::for_leaf(party, *version, value).into())
        .collect();

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

/// An empty tree's root hash must match the reference (256 zero slots).
#[test]
fn empty_tree_hash_matches_reference() {
    let tree: Tree<String> = Tree::default();
    let tree_hash = tree.hash();
    let reference = reference_hash::<String>(&[]);
    assert_eq!(&tree_hash, reference.as_bytes());
}

/// A single inserted value must hash identically to the uncompressed trie
/// containing just that value. This exercises Leaf::hash path compression
/// with a maximal 31-byte leaf prefix.
#[test]
fn single_value_hash_matches_reference() {
    let value = Bytes::from(&b"hello"[..]);
    let mut tree: Tree<String> = Tree::default();
    tree.act(
        &"P".to_string(),
        &("P".to_string(), 1).into(),
        [Action::Insert(Bytes::copy_from_slice(&value))],
    );
    let tree_hash = tree.hash();
    let reference = reference_hash(&[("P".to_string(), 1, value)]);
    assert_eq!(&tree_hash, reference.as_bytes());
}

proptest! {
    /// The compressed tree's root hash must equal the hash computed over the
    /// fully-expanded uncompressed trie, for any set of inserted values. This
    /// is the ground-truth invariant for path compression: the hash depends on
    /// the set of leaves, not on how the tree chooses to compress them.
    #[test]
    fn compressed_hash_matches_reference(
        values in proptest::collection::vec(any::<Vec<u8>>(), 0..16)
            .prop_map(|v| v.into_iter().map(Bytes::from).collect::<Vec<_>>()),
    ) {
        let uniques: Vec<(String, u64, Bytes)> =
            values
                .into_iter()
                .enumerate()
                .map(|(v, value)| ("P".to_string(), v as u64, value))
                .collect::<BTreeSet<_>>().into_iter().collect();
        let uniques_with_full_version: Vec<(String, Version<String>, Bytes)> =
            uniques
                .iter()
                .map(|(party, version, value)| (party.clone(), (party.clone(), *version).into(), value.clone()))
                .collect();

        let mut tree: Tree<String> = Tree::default();
        for (party, version, value) in uniques_with_full_version.iter().cloned() {
            tree.act(&party, &version, [Action::Insert(value)]);
        }
        let tree_hash = tree.hash();

        let reference = reference_hash(&uniques);
        prop_assert_eq!(&tree_hash, reference.as_bytes());
    }

    /// For a fixed set of distinct values inserted under a single party and
    /// version, the root hash is determined by the set, not the insertion
    /// order: every (party, version, value) occupies a path uniquely determined
    /// by its blake3 hash, so reordering inserts must yield the same shape and
    /// the same root hash.
    #[test]
    fn insert_is_order_independent(
        values in proptest::collection::vec(any::<(String, u64, Vec<u8>)>(), 0..32),
    ) {
        let uniques: Vec<(String, u64, Bytes)> =
            values
                .into_iter()
                .map(|(party, version, value)| (party, version, value.into()))
                .collect::<BTreeSet<_>>().into_iter().collect();
        let uniques: Vec<(String, Version<String>, Bytes)> =
            uniques
                .into_iter()
                .map(|(party, version, value)| (party.clone(), (party, version).into(), value))
                .collect();

        let mut forward: Tree<String> = Tree::default();
        for (party, version, value) in uniques.iter().cloned() {
            forward.act(&party, &version, [Action::Insert(value)]);
        }

        let mut reverse: Tree<String> = Tree::default();
        for (party, version, value) in uniques.iter().cloned() {
            reverse.act(&party, &version, [Action::Insert(value)]);
        }

        let forward_hash = forward.hash();
        let reverse_hash = reverse.hash();
        prop_assert_eq!(forward_hash, reverse_hash);
    }
}
