use std::collections::BTreeMap;
use std::hash::Hash;
use std::sync::Arc;

use bytes::Bytes;

use crate::Version;

mod cached;
mod entry;

use cached::Cached;
pub use entry::{Entry, InteriorEntry, LeafEntry, OccupiedEntry, VacantEntry};

#[derive(Clone)]
pub struct Node<P: Hash + Eq> {
    /// Compressed path above this node's own branching level, stored with the
    /// deepest byte at index 0 and the shallowest byte at the last index. An
    /// empty prefix means the node is not path-compressed above its level.
    prefix: Vec<u8>,
    /// The cached hash of this node, invalidated when any change occurs in or
    /// beneath it.
    hash: Cached<blake3::Hash>,
    /// The maximum of all versions beneath this node, cached to allow efficient causal merging of trees.
    version: Cached<Version<P>>,
    /// The children of this node: either a leaf, or a branch point.
    children: Children<P>,
}

/// The children of a node.
#[derive(Clone)]
enum Children<P: Hash + Eq> {
    /// A direct leaf, at the true bottom of the tree.
    Leaf(Leaf<P>),
    /// A materialized branch point, with the invariant that there are always >=
    /// 2 branches (or else they should be path-compressed away).
    Branch(BTreeMap<u8, Arc<Node<P>>>),
}

/// A leaf at the bottom of the tree, holding the value payload.
#[derive(Clone)]
pub struct Leaf<P: Hash + Eq> {
    /// The party which originally inserted this leaf into the set.
    pub party: P,
    /// That party's local version scalar at the time of insertion.
    pub version: u64,
    /// The value inserted, whose hash is the path in the tree.
    pub value: Bytes,
}

impl<P: Hash + Eq> Node<P> {
    /// Hash the subtree rooted at this node, using the merkle-trie convention:
    /// a leaf's "branching" layer is the distinguished sentinel `[0xff; 32]`, a
    /// branch's is 256 concatenated child hashes (with `[0x00; 32]` in empty
    /// slots), and a non-empty compressed prefix wraps that hash bottom-up, one
    /// byte at a time, so that path-compressed and fully-expanded trees with
    /// the same set of leaves produce the same hash.
    pub fn hash(&self) -> blake3::Hash {
        self.hash.get(|| {
            let mut hash: blake3::Hash = match &self.children {
                Children::Leaf(_) => [0xff; 32].into(),
                Children::Branch(map) => {
                    let mut hasher = blake3::Hasher::new();
                    for i in u8::MIN..=u8::MAX {
                        match map.get(&i) {
                            None => hasher.update(&[0x00; 32]),
                            Some(child) => hasher.update(child.hash().as_bytes()),
                        };
                    }
                    hasher.finalize()
                }
            };
            // Wrap with the compressed prefix bottom-up: prefix[0] is the
            // deepest byte, applied first; prefix[last] is the shallowest byte,
            // applied last (producing the hash at this node's own level).
            for &byte in &self.prefix {
                let mut hasher = blake3::Hasher::new();
                for i in u8::MIN..=u8::MAX {
                    match byte == i {
                        false => hasher.update(&[0x00; 32]),
                        true => hasher.update(hash.as_bytes()),
                    };
                }
                hash = hasher.finalize();
            }
            hash
        })
    }
}

impl<P: Hash + Eq + Clone> Default for Node<P> {
    fn default() -> Self {
        Self {
            prefix: Default::default(),
            hash: Default::default(),
            version: Default::default(),
            children: Children::Branch(BTreeMap::new()),
        }
    }
}

/// Return `true` if no node in the tree violates path compression: branches
/// must have at least two children (except an empty branch at the root,
/// which is the empty-tree representation), and there are no one-child
/// branches anywhere.
fn is_max_compressed<P: Hash + Eq>(root: &Node<P>) -> bool {
    fn check<P: Hash + Eq>(node: &Node<P>, is_root: bool) -> bool {
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
