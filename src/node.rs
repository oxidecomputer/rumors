use std::cmp::Ordering;
use std::collections::{BTreeMap, btree_map::Entry};
use std::mem;
use std::ops::RangeInclusive;
use std::sync::Arc;

use bytes::Bytes;

use crate::version::Version;

/// A node in the tree, maximum branching factor 256.
#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub(crate) struct Node<P: Ord> {
    /// Compressed path down to this node from its parent, stored in reverse
    /// descent order so that `pop` yields the next byte of descent.
    pub(crate) path: Vec<u8>,
    /// Version of this node (join of all live descendants' versions).
    pub(crate) version: Version<P>,
    /// Ranged tombstone versions for deleted children of this node: must be
    /// non-overlapping and sorted. A tombstone remains even when a concurrent
    /// or newer insert re-occupies a byte in its range, so peers that missed
    /// the deletion can still learn about it via gossip.
    pub(crate) deleted: Vec<(RangeInclusive<u8>, Version<P>)>,
    /// Children of this node: either the byte payload if a leaf, or 2+ child
    /// nodes (if there would have been exactly 1 child node, it should have
    /// been compressed into this node).
    pub(crate) children: Children<P>,
}

/// Children of a node; either a leaf or branches.
#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub(crate) enum Children<P: Ord> {
    Leaf(Bytes),
    Branch(BTreeMap<u8, Arc<Node<P>>>),
}

impl<P: Ord> Default for Children<P> {
    fn default() -> Self {
        Children::Branch(Default::default())
    }
}

impl<P: Ord> Default for Node<P> {
    fn default() -> Self {
        Self {
            path: Default::default(),
            version: Default::default(),
            deleted: Default::default(),
            children: Default::default(),
        }
    }
}

impl<P: Ord> Node<P> {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Insert `value` at `path` under version `version`, merging `version`
    /// into the version of every node touched on the way down to the new
    /// leaf. Returns `true` if the insert took effect and `false` if it was
    /// dropped by a tombstone that causally dominates `version` (i.e., the
    /// deletion observed this insertion).
    ///
    /// `path` is supplied in *reverse* byte order: `path.pop()` yields the
    /// next byte of descent. This lets the caller splice the path top-down
    /// without a double reversal.
    pub(crate) fn insert(&mut self, version: Version<P>, mut path: Vec<u8>, value: Bytes) -> bool
    where
        P: Clone,
    {
        // Empty-node shortcut: a node with no path, no tombstones, and an
        // empty branch map has no identity of its own, so rather than spawn a
        // singleton child (which would violate the "branches have 2+ live
        // children" invariant), we absorb the insert into this node. This is
        // the shape of a freshly-constructed root into which the first value
        // is being inserted.
        if self.path.is_empty()
            && self.deleted.is_empty()
            && matches!(&self.children, Children::Branch(m) if m.is_empty())
        {
            self.path = path;
            self.version = version;
            self.children = Children::Leaf(value);
            return true;
        }

        // Walk `self.path` in descent order (reverse iteration of reversed
        // storage), popping one byte of `path` per step. A mismatch means the
        // incoming path diverges from the compressed prefix stored on this
        // node, and we must split.
        for (i, &expected) in self.path.iter().rev().enumerate() {
            let actual = path
                .pop()
                .expect("insert path shorter than self.path violates fixed-length invariant");
            if expected != actual {
                return self.split(i, expected, actual, path, version, value);
            }
        }

        // `self.path` matched in full; `path` now holds the bytes strictly
        // below this node.
        if path.is_empty() {
            let Children::Leaf(bytes) = &mut self.children else {
                unreachable!(
                    "empty remaining path at a branch node violates fixed-length path invariant"
                );
            };
            // Fixed-length paths imply same-path leaves carry the same value.
            debug_assert_eq!(
                bytes.as_ref(),
                value.as_ref(),
                "leaf value mismatch at identical path",
            );
            *bytes = value;
            self.version |= version;
            return true;
        }

        let b = path.pop().expect("path is non-empty");
        let Children::Branch(map) = &mut self.children else {
            unreachable!("reached a leaf with path remaining violates fixed-length path invariant");
        };

        // Tombstones at this node cover child-slots of this node. If a
        // tombstone's version strictly dominates the incoming version, the
        // deletion observed this insert: drop it. Concurrent, equal, and
        // newer inserts survive; the tombstone is left intact so gossip peers
        // that missed the delete can still learn about it.
        for (range, tver) in &self.deleted {
            if range.contains(&b) && matches!(version.partial_cmp(tver), Some(Ordering::Less)) {
                return false;
            }
        }

        let applied = match map.entry(b) {
            Entry::Vacant(e) => {
                let child = Node {
                    path,
                    version: version.clone(),
                    deleted: Vec::new(),
                    children: Children::Leaf(value),
                };
                e.insert(Arc::new(child));
                true
            }
            Entry::Occupied(mut e) => {
                let child = Arc::make_mut(e.get_mut());
                child.insert(version.clone(), path, value)
            }
        };

        if applied {
            self.version |= version;
        }
        applied
    }

    /// Split `self` at descent-order index `i` within its compressed path:
    /// `self` becomes the new intermediate branch carrying the common
    /// prefix, the previous contents of `self` become one child under edge
    /// byte `old_next`, and a fresh leaf is placed under edge byte
    /// `new_next`.
    ///
    /// With paths stored in reverse descent order, the common prefix is the
    /// top `i` elements of `self.path` (what `pop` would yield first), the
    /// separator byte is the next `pop`, and the remainder is the old
    /// child's suffix already in the right storage form.
    fn split(
        &mut self,
        i: usize,
        old_next: u8,
        new_next: u8,
        new_leaf_path: Vec<u8>,
        version: Version<P>,
        value: Bytes,
    ) -> bool
    where
        P: Clone,
    {
        let mut take = mem::take(&mut self.path);
        self.path = take.split_off(take.len() - i);
        let sep = take.pop().expect("separator byte present");
        debug_assert_eq!(sep, old_next);
        let old_suffix = take;

        let old_children = mem::take(&mut self.children);
        let old_deleted = mem::take(&mut self.deleted);
        let old_version = mem::take(&mut self.version);

        let old_child = Node {
            path: old_suffix,
            version: old_version.clone(),
            deleted: old_deleted,
            children: old_children,
        };

        let new_leaf = Node {
            path: new_leaf_path,
            version: version.clone(),
            deleted: Vec::new(),
            children: Children::Leaf(value),
        };

        let mut map = BTreeMap::new();
        map.insert(old_next, Arc::new(old_child));
        map.insert(new_next, Arc::new(new_leaf));

        self.version = old_version | version;
        self.children = Children::Branch(map);
        // self.deleted is already empty from the `mem::take` above.
        true
    }
}

#[cfg(test)]
mod test;
