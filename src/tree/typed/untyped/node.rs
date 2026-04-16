use std::hash::Hash;
use std::mem;
use std::sync::Arc;

use bytes::Bytes;
use imbl::OrdMap;

mod cached;
use cached::Cached;

use crate::Version;

#[derive(Clone, Debug)]
pub struct Node<P: Hash + Eq + AsRef<[u8]>> {
    inner: Arc<NodeInner<P>>,
}

#[derive(Clone, Debug)]
struct NodeInner<P: Hash + Eq + AsRef<[u8]>> {
    /// Compressed path above this node's own branching level, stored with the
    /// deepest byte at index 0 and the shallowest byte at the last index. An
    /// empty prefix means the node is not path-compressed above its level.
    prefix: Vec<u8>,
    /// The cached hash of this node, invalidated when any change occurs in or
    /// beneath it.
    hash: Cached<blake3::Hash>,
    /// The maximal version of any child of this node.
    version: Version<P>,
    /// The children of this node: either a leaf, or a branch point.
    children: Children<P>,
}

/// The children of a node.
#[derive(Clone, Debug)]
enum Children<P: Hash + Eq + AsRef<[u8]>> {
    /// A direct leaf, at the true bottom of the tree.
    Leaf(Bytes),
    /// A materialized branch point, with the invariant that there are always >=
    /// 2 branches (or else they should be path-compressed away).
    Branch(OrdMap<u8, Node<P>>),
}

impl<P: Hash + Eq + Clone + AsRef<[u8]>> Node<P> {
    /// Construct a new branch node from a list of children with distinct
    /// indices (inverse to [`Node::into_children`]).
    pub fn branch(children: OrdMap<u8, Node<P>>) -> Option<Self> {
        match children.len() {
            0 => None,
            1 => {
                let Some((index, node)) = children.into_iter().next() else {
                    unreachable!("a map with 1 element cannot fail to iterate");
                };
                Some(node.beneath(index))
            }
            _ => Some(Node {
                inner: Arc::new(NodeInner {
                    prefix: Vec::new(),
                    hash: Cached::new(),
                    version: Version::new(children.values().map(|n| n.version().clone())),
                    children: Children::Branch(children),
                }),
            }),
        }
    }

    /// Convert a node into a map from child index to child node (inverse to
    /// [`Node::branch`]).
    ///
    /// If `self` is a leaf node, returns `Err(self)`.
    pub fn into_children(mut self) -> Result<OrdMap<u8, Node<P>>, Node<P>> {
        if !self.inner.prefix.is_empty() {
            // Path-compressed: pop the top byte and rewrap self under
            // it. The caller observes self with a shortened prefix,
            // so the cached hash must be invalidated.
            let inner = Arc::make_mut(&mut self.inner);
            inner.hash.invalidate();
            let index = inner.prefix.pop().expect("non-empty prefix");
            Ok(OrdMap::from_iter([(index, self)]))
        } else {
            match &self.inner.children {
                Children::Leaf(_) => Err(self),
                Children::Branch(_) => {
                    // Extract the children map; self is dropped without
                    // the caller observing its mutated state, so hash
                    // invalidation would be wasted work.
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
    pub fn leaf(version: Version<P>, value: Bytes) -> Self {
        Node {
            inner: Arc::new(NodeInner {
                prefix: Vec::new(),
                hash: Cached::new(),
                version,
                children: Children::Leaf(value),
            }),
        }
    }

    /// Get a reference to the leaf at this node, if it is a leaf.
    pub fn as_leaf(&self) -> Option<&Bytes> {
        match &self.inner.children {
            Children::Leaf(leaf) => Some(leaf),
            _ => None,
        }
    }

    /// Hash the subtree rooted at this node.
    ///
    /// Hashes are lazily computed and cached until the tree structure changes
    /// to invalidate them.
    ///
    /// The hashing convention: a leaf's "hash" is the distinguished sentinel
    /// `[0xff; 32]`, a branch's is the hash of 256 concatenated child hashes
    /// (with `[0x00; 32]` in empty slots). Hashing should not depend on path
    /// compression; we ensure this by hashing "virtual" nodes as we traverse up
    /// a compressed path.
    pub fn hash(&self) -> blake3::Hash {
        self.inner.hash.get(|| {
            let mut hash: blake3::Hash = match &self.inner.children {
                Children::Leaf(_) => [0xff; 32].into(),
                Children::Branch(branch) => {
                    let mut buf = [0u8; 256 * 32];
                    for (&i, child) in branch.iter() {
                        buf[i as usize * 32..][..32].copy_from_slice(child.hash().as_bytes());
                    }
                    blake3::hash(&buf)
                }
            };
            // Wrap with the compressed prefix bottom-up: prefix[0] is the
            // deepest byte, applied first; prefix[last] is the shallowest byte,
            // applied last (producing the hash at this node's own level).
            //
            // Each virtual branch is 256 × 32 = 8192 bytes: all zero-slots
            // except for the one occupied child. We build the full buffer
            // contiguously to avoid 256 separate tiny update calls per byte.
            for &byte in &self.inner.prefix {
                let mut buf = [0u8; 256 * 32];
                buf[byte as usize * 32..][..32].copy_from_slice(hash.as_bytes());
                hash = blake3::hash(&buf);
            }
            hash
        })
    }

    /// Get the version of this node (the maximal version of all children).
    pub fn version(&self) -> &Version<P> {
        &self.inner.version
    }

    /// Place a node beneath the given child index, increasing its height by one.
    fn beneath(mut self, index: u8) -> Node<P> {
        let inner = Arc::make_mut(&mut self.inner);
        inner.hash.invalidate();
        inner.prefix.push(index);
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

impl<P: Hash + Eq + Clone + AsRef<[u8]>> Eq for Node<P> {}

impl<P: Hash + Eq + Clone + AsRef<[u8]>> PartialEq for Node<P> {
    fn eq(&self, other: &Self) -> bool {
        self.hash() == other.hash()
    }
}

#[cfg(test)]
mod tests;
