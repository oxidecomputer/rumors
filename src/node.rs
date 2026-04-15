use std::collections::BTreeMap;
use std::hash::Hash;
use std::sync::Arc;

use bytes::Bytes;

use crate::node::entry::{Entry, OccupiedEntry};
use crate::version::Version;

mod cached_hash;
mod entry;

use cached_hash::CachedHash;

#[cfg(test)]
mod test;

pub struct Tree<P: Hash + Eq> {
    version: Version<P>,
    root: Node<P>,
}

impl<P: Hash + Eq> Default for Tree<P> {
    fn default() -> Self {
        Self {
            version: Version::default(),
            root: Node::default(),
        }
    }
}

impl<P: Clone + Hash + Eq> Tree<P> {
    pub fn insert(&mut self, party: P, version: u64, value: Bytes) {
        // Don't bother inserting the value if we know it already was inserted,
        // due to having been strictly posterior to the current version
        if version < self.version.for_party(&party) {
            return;
        }

        let path = path_for(&value);
        self.root.insert(party.clone(), version, path, value);
        self.version |= Version::from((party, version));
    }
}

/// Compute the path used to address a value in the trie: the bytes of
/// `blake3(value)` reversed so that `path.pop()` yields the byte dispatched
/// at the next-shallower level.
fn path_for(value: &Bytes) -> Vec<u8> {
    blake3::hash(value)
        .as_bytes()
        .iter()
        .copied()
        .rev()
        .collect()
}

#[derive(Clone)]
struct Node<P> {
    /// Compressed path above this node's own branching level, stored with the
    /// deepest byte at index 0 and the shallowest byte at the last index. An
    /// empty prefix means the node is not path-compressed above its level.
    prefix: Vec<u8>,
    /// The cached hash of this node, invalidated when any change occurs in or
    /// beneath it.
    hash: CachedHash,
    /// The children of this node: either a leaf, or a branch point.
    children: Children<P>,
}

/// The children of a node.
#[derive(Clone)]
enum Children<P> {
    /// A direct leaf, at the true bottom of the tree.
    Leaf(Leaf<P>),
    /// A materialized branch point, with the invariant that there are always >=
    /// 2 branches (or else they should be path-compressed away).
    Branch(BTreeMap<u8, Arc<Node<P>>>),
}

/// A leaf at the bottom of the tree, holding the value payload.
#[derive(Clone)]
struct Leaf<P> {
    /// The party which originally inserted this leaf into the set.
    party: P,
    /// That party's local version scalar at the time of insertion.
    version: u64,
    /// The value inserted, whose hash is the path in the tree.
    value: Bytes,
}

impl<P> Node<P> {
    /// Hash the subtree rooted at this node, using the merkle-trie convention:
    /// a leaf's "branching" layer is the distinguished sentinel `[0xff; 32]`, a
    /// branch's is 256 concatenated child hashes (with `[0x00; 32]` in empty
    /// slots), and a non-empty compressed prefix wraps that hash bottom-up, one
    /// byte at a time, so that path-compressed and fully-expanded trees with
    /// the same set of leaves produce the same hash.
    fn hash(&self) -> blake3::Hash {
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

impl<P> Default for Node<P> {
    fn default() -> Self {
        Self {
            prefix: Vec::new(),
            hash: CachedHash::default(),
            children: Children::Branch(BTreeMap::new()),
        }
    }
}

impl<P: Clone> Node<P> {
    /// Insert `(party, version, value)` at the position denoted by `path`
    /// (remaining bytes, shallowest at the end). The recursion is expressed
    /// entirely against the entry-API primitives in [`entry`]; path compression
    /// is applied and maintained as a side effect of
    /// `VacantEntry::insert_leaf`.
    fn insert(&mut self, party: P, version: u64, path: Vec<u8>, value: Bytes) {
        insert_at(&mut self.root_entry(), path, party, version, value);
    }
}

/// Tree-walk algorithm: descend from `entry` by popping one byte off the
/// shallow end of `path` per recursion, invalidating cached hashes along the
/// way so that subsequent `Node::hash()` calls recompute through the modified
/// subtree.
///
/// The path's length is expected to match the depth from `entry`'s position to
/// a terminal leaf. A mismatch (path exhausted at an interior position, or path
/// remaining at a terminal leaf) indicates a caller- side invariant violation
/// and is asserted explicitly here.
///
/// Returns `true` if there already was a value at this path.
fn insert_at<P: Clone>(
    node: &mut OccupiedEntry<'_, P>,
    mut path: Vec<u8>,
    party: P,
    version: u64,
    value: Bytes,
) -> bool {
    use Entry::*;
    use OccupiedEntry::*;

    if let Some(byte) = path.pop() {
        // We still have to descend further into the tree structure, because
        // we're not at the end of the path -- we should expect the node at this
        // position to be an interior node, because a leaf here would be at the
        // wrong depth
        let Interior(interior) = node else {
            panic!("insert path still has bytes left at a terminal leaf")
        };
        let existed = match interior.child(byte) {
            Occupied(mut node) => insert_at(&mut node, path, party, version, value),
            Vacant(vacant) => {
                vacant.insert_leaf(path, party, version, value);

                // We inserted a new leaf, so we should report this:
                false
            }
        };
        if !existed {
            node.invalidate_hash();
        }
        existed
    } else {
        // We have reached the end of the path without encountering any vacant
        // nodes; this means that we should expect an existing leaf to be here,
        // whose value hash must by construction be equal to the hash of the
        // value we are trying to insert
        let Leaf(leaf) = node else {
            panic!("insert path exhausted at an interior position")
        };
        let leaf = leaf.leaf_mut();
        debug_assert_eq!(leaf.value, value, "leaf values at the same path must match");
        leaf.party = party;
        leaf.version = version;
        leaf.value = value;

        // Because we did not insert a new leaf but rather updated an existing
        // one, we should report this:
        true
    }
}
