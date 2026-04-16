use std::collections::{BTreeSet, HashMap};

use bytes::Bytes;
use proptest::prelude::*;

use super::*;

/// An action drawn from a small pool of fixed values. Both inserts and
/// deletes target the same pool so actions frequently collide at shared
/// leaves, exposing any order-dependence in the root hash.
fn any_action() -> impl Strategy<Value = Action<u64>> {
    const POOL: usize = 8;
    prop_oneof![
        3 => (0..POOL, any::<u64>(), any::<u64>()).prop_map(|(i, party, version)| {
            Action::Insert {
                party,
                version,
                value: Bytes::copy_from_slice(&[i as u8; 4]),
            }
        }),
        1 => (0..POOL).prop_map(|i| Action::Delete {
            hash: *blake3::hash(&[i as u8; 4]).as_bytes(),
        }),
    ]
}

/// Compute the root hash of the fully-expanded (uVn-path-compressed) 256-ary
/// trie over the given set of values. For every unique blake3 path, a leaf
/// sentinel of all-0xff bytes sits at depth 32; at each level above, a 256-way
/// branch hashes its child slots (0x00-filled where absent, recursive hash
/// where present). This is the ground truth that the compressed tree's root
/// hash must match.
fn reference_hash(values: &[Bytes]) -> blake3::Hash {
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

/// An empty tree's root hash must match the reference (256 zero slots).
#[test]
fn empty_tree_hash_matches_reference() {
    let tree: Tree<u64> = Tree::default();
    let tree_hash = tree.hash();
    let reference = reference_hash(&[]);
    assert_eq!(&tree_hash, reference.as_bytes());
}

/// A single inserted value must hash identically to the uncompressed trie
/// containing just that value. This exercises Leaf::hash path compression
/// with a maximal 31-byte leaf prefix.
#[test]
fn single_value_hash_matches_reference() {
    let value = Bytes::from(&b"hello"[..]);
    let mut tree: Tree<u64> = Tree::default();
    tree.act([Action::Insert {
        party: 0,
        version: 1,
        value: Bytes::copy_from_slice(&value),
    }]);
    let tree_hash = tree.hash();
    let reference = reference_hash(&[value]);
    assert_eq!(&tree_hash, reference.as_bytes());
}

proptest! {
    /// The compressed tree's root hash must equal the hash computed over the
    /// fully-expanded uncompressed trie, for any set of inserted values. This
    /// is the ground-truth invariant for path compression: the hash depends on
    /// the set of leaves, not on how the tree chooses to compress them.
    #[test]
    fn compressed_hash_matches_reference(
        values in proptest::collection::vec(any::<Vec<u8>>(), 0..16),
    ) {
        let uniques: Vec<Bytes> =
            values.into_iter().collect::<BTreeSet<_>>().into_iter().map(Bytes::from).collect();

        let mut tree: Tree<u64> = Tree::default();
        for (i, v) in uniques.iter().enumerate() {
            tree.act([Action::Insert { party: 0, version: (i as u64) + 1, value: Bytes::copy_from_slice(v)}]);
        }
        let tree_hash = tree.hash();
        let reference = reference_hash(&uniques);
        prop_assert_eq!(&tree_hash, reference.as_bytes());
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
            forward.act([Action::Insert{ party: 0, version: (i as u64) + 1, value: Bytes::copy_from_slice(v)}]);
        }

        let mut reverse: Tree<u64> = Tree::default();
        for (i, v) in uniques.iter().rev().enumerate() {
            reverse.act([Action::Insert{ party: 0, version: (i as u64) + 1, value: Bytes::copy_from_slice(v)}]);
        }

        let forward_hash = forward.hash();
        let reverse_hash = reverse.hash();
        prop_assert_eq!(forward_hash, reverse_hash);
    }


    /// `Tree::act` is associative over concatenation: for any list of actions
    /// and any partition of that list into consecutive chunks, applying the
    /// full list as one batch must produce the same root hash as sequentially
    /// applying each chunk.
    #[test]
    fn act_is_associative(
        actions in proptest::collection::vec(any_action(), 0..32),
        cuts in proptest::collection::vec(any::<usize>(), 0..8),
    ) {
        let n = actions.len();
        let mut splits: Vec<usize> = cuts
            .into_iter()
            .map(|c| if n == 0 { 0 } else { c % (n + 1) })
            .collect();
        splits.push(0);
        splits.push(n);
        splits.sort();
        splits.dedup();

        let mut batched: Tree<u64> = Tree::default();
        batched.act(actions.iter().cloned());

        let mut chunked: Tree<u64> = Tree::default();
        for w in splits.windows(2) {
            chunked.act(actions[w[0]..w[1]].iter().cloned());
        }

        prop_assert_eq!(batched.hash(), chunked.hash());
    }

    /// The tree's occupied leaf set after a sequence of actions matches a
    /// HashMap oracle applying "last writer wins" semantics.
    ///
    /// The oracle processes actions sequentially: inserts upsert by content
    /// hash, deletes remove by hash. We then build a reference tree from the
    /// oracle's surviving entries and compare root hashes.
    ///
    /// Because the root hash depends only on which paths are occupied (leaf
    /// hashes are a fixed sentinel), this verifies structural correctness:
    /// inserts create leaves at the right paths, deletes remove them, and
    /// batch semantics reduce to the final surviving set.
    #[test]
    fn act_matches_oracle(
        actions in proptest::collection::vec(any_action(), 0..32),
    ) {
        let mut tree: Tree<u64> = Tree::default();
        tree.act(actions.iter().cloned());

        let mut oracle: HashMap<[u8; 32], (u64, u64, Bytes)> = HashMap::new();
        for action in &actions {
            match action {
                Action::Insert { party, version, value } => {
                    oracle.insert(
                        *blake3::hash(value).as_bytes(),
                        (*party, *version, value.clone()),
                    );
                }
                Action::Delete { hash } => {
                    oracle.remove(hash);
                }
            }
        }

        let mut reference: Tree<u64> = Tree::default();
        reference.act(oracle.into_values().map(|(party, version, value)| {
            Action::Insert { party, version, value }
        }));

        prop_assert_eq!(tree.hash(), reference.hash());
    }
}
