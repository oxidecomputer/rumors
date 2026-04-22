use std::mem;
use std::sync::Arc;

use imbl::OrdMap;

use crate::{Message, Version};

#[derive(Clone, Debug)]
pub struct Node<P: Ord + AsRef<[u8]>, T> {
    inner: Arc<NodeInner<P, T>>,
}

#[derive(Clone, Debug)]
struct NodeInner<P: Ord + AsRef<[u8]>, T> {
    /// Compressed path above this node's own branching level, stored with the
    /// deepest byte at index 0 and the shallowest byte at the last index. An
    /// empty prefix means the node is not path-compressed above its level.
    ///
    /// Each entry pairs a path byte with the hash of the virtual node produced
    /// by wrapping the children-level hash through `prefix[0..=i]` — computed
    /// once at construction. `prefix.last().1`, when present, is therefore the
    /// node's observable hash.
    prefix: Vec<(u8, blake3::Hash)>,
    /// Hash of this node's children (the leaf sentinel or the branch-level
    /// hash), independent of any compressed prefix above it. Computed once at
    /// construction.
    children_hash: blake3::Hash,
    /// The maximal version of any child of this node.
    version: Version<P>,
    /// The children of this node: either a leaf, or a branch point.
    children: Children<P, T>,
}

/// The children of a node.
#[derive(Clone, Debug)]
enum Children<P: Ord + AsRef<[u8]>, T> {
    /// A direct leaf, at the true bottom of the tree.
    Leaf(Message<T>),
    /// A materialized branch point, with the invariant that there are always >=
    /// 2 branches (or else they should be path-compressed away).
    Branch(OrdMap<u8, Node<P, T>>),
}

/// Sentinel hash used as a leaf's "hash" so leaves are distinguishable from
/// any branch (which always hashes a 256-slot buffer). Empty branch slots
/// hash as `[0x00; 32]`, the natural zero-init of the staging buffer.
const LEAF_SENTINEL: [u8; 32] = [0xff; 32];

impl<P: Ord + Clone + AsRef<[u8]>, T: Clone> Node<P, T> {
    /// Construct a new branch node from a list of children with distinct
    /// indices (inverse to [`Node::into_children`]).
    pub fn branch(children: OrdMap<u8, Node<P, T>>) -> Option<Self> {
        match children.len() {
            0 => None,
            1 => {
                let Some((index, node)) = children.into_iter().next() else {
                    unreachable!("a map with 1 element cannot fail to iterate");
                };
                Some(node.beneath(index))
            }
            _ => {
                let mut buf = [0u8; 256 * 32];
                for (&i, child) in children.iter() {
                    buf[i as usize * 32..][..32].copy_from_slice(child.hash().as_bytes());
                }
                let children_hash = blake3::hash(&buf);
                Some(Node {
                    inner: Arc::new(NodeInner {
                        prefix: Vec::new(),
                        children_hash,
                        version: Version::new(children.values().map(|n| n.version().clone())),
                        children: Children::Branch(children),
                    }),
                })
            }
        }
    }

    /// Convert a node into a map from child index to child node (inverse to
    /// [`Node::branch`]).
    ///
    /// If `self` is a leaf node, returns `Err(self)`.
    pub fn into_children(mut self) -> Result<OrdMap<u8, Node<P, T>>, Node<P, T>> {
        if !self.inner.prefix.is_empty() {
            // Path-compressed: pop the top byte and rewrap self under it. The
            // popped entry's precomputed hash is dropped; every shorter
            // prefix-level hash and the children-level hash are still valid
            // because the children and the surviving byte sequence are
            // unchanged.
            let inner = Arc::make_mut(&mut self.inner);
            let (index, _hash) = inner.prefix.pop().expect("non-empty prefix");
            Ok(OrdMap::from_iter([(index, self)]))
        } else {
            match &self.inner.children {
                Children::Leaf(_) => Err(self),
                Children::Branch(_) => {
                    // Extract the children map; self is dropped, so leaving its
                    // precomputed hash referencing the now-vacated branch is
                    // harmless.
                    let inner = Arc::make_mut(&mut self.inner);
                    let Children::Branch(branch) = &mut inner.children else {
                        unreachable!("just matched Branch")
                    };
                    Ok(mem::take(branch))
                }
            }
        }
    }

    /// Construct a new leaf node.
    pub fn leaf(version: Version<P>, value: Message<T>) -> Self {
        Node {
            inner: Arc::new(NodeInner {
                prefix: Vec::new(),
                children_hash: LEAF_SENTINEL.into(),
                version,
                children: Children::Leaf(value),
            }),
        }
    }

    /// Get a reference to the leaf at this node, if it is a leaf.
    pub fn as_leaf(&self) -> Option<&Message<T>> {
        match &self.inner.children {
            Children::Leaf(leaf) => Some(leaf),
            _ => None,
        }
    }

    /// Hash the subtree rooted at this node.
    ///
    /// The hash is precomputed at construction and stored, so this is an O(1)
    /// field read. The hashing convention: a leaf's "hash" is the distinguished
    /// sentinel `[0xff; 32]`, a branch's is the hash of 256 concatenated child
    /// hashes (with `[0x00; 32]` in empty slots). Hashing does not depend on
    /// path compression: a one-child branch and a node path-compressed by one
    /// byte produce identical hashes.
    pub fn hash(&self) -> blake3::Hash {
        self.inner
            .prefix
            .last()
            .map(|(_, h)| *h)
            .unwrap_or(self.inner.children_hash)
    }

    /// Get the version of this node (the maximal version of all children).
    pub fn version(&self) -> &Version<P> {
        &self.inner.version
    }

    /// Place a node beneath the given child index, increasing its height by
    /// one. Eagerly computes the new top-of-prefix hash by wrapping the old
    /// observable hash through one virtual-branch level.
    fn beneath(mut self, index: u8) -> Node<P, T> {
        let mut buf = [0u8; 256 * 32];
        buf[index as usize * 32..][..32].copy_from_slice(self.hash().as_bytes());
        let new_top = blake3::hash(&buf);
        let inner = Arc::make_mut(&mut self.inner);
        inner.prefix.push((index, new_top));
        self
    }

    /// Return `true` if no node in the tree violates path compression: every
    /// branch must have at least two children. The empty tree is represented by
    /// the absence of a root, so empty and one-child branches are never valid
    /// anywhere in the tree.
    #[cfg(test)]
    fn is_max_compressed(&self) -> bool {
        match &self.inner.children {
            Children::Leaf(_) => true,
            Children::Branch(branch) => {
                branch.len() >= 2 && branch.values().all(Self::is_max_compressed)
            }
        }
    }
}

impl<P: Ord + Clone + AsRef<[u8]>, T: Clone> Eq for Node<P, T> {}

impl<P: Ord + Clone + AsRef<[u8]>, T: Clone> PartialEq for Node<P, T> {
    fn eq(&self, other: &Self) -> bool {
        self.hash() == other.hash()
    }
}

#[cfg(test)]
mod test;
