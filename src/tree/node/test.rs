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
