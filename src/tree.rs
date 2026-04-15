use std::hash::Hash;

use bytes::Bytes;

use crate::version::Version;

mod node;
use node::{Entry, InteriorEntry, LeafEntry, Node, OccupiedEntry, VacantEntry};

pub struct Tree<P: Hash + Eq> {
    version: Version<P>,
    root: Node<P>,
}

impl<P: Hash + Eq + Clone> Default for Tree<P> {
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

        insert_at(
            &mut self.root.walk(),
            path_for(&value),
            party.clone(),
            version,
            value,
        );

        self.version |= Version::from((party, version));
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
fn insert_at<P: Hash + Eq + Clone>(
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

#[cfg(test)]
mod test;
