use std::mem;
use std::sync::Arc;

use imbl::OrdMap;

use crate::cached::Cached;
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
    /// Each entry pairs a path byte with the cached hash of the virtual node
    /// produced by wrapping the children-level hash through `prefix[0..=i]`.
    /// These intermediate caches let `into_children` strip the topmost byte
    /// without invalidating any of the shorter levels: only the popped
    /// entry's cache is discarded. Pushing onto the prefix via `beneath`
    /// likewise leaves the existing entries' caches valid, since the bytes
    /// below the new top are unchanged.
    prefix: Vec<(u8, Cached<blake3::Hash>)>,
    /// The cached hash of this node's children (the leaf sentinel or the
    /// branch-level hash), independent of any compressed prefix above it.
    /// Invalidated only when the children themselves change.
    children_hash: Cached<blake3::Hash>,
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
            _ => Some(Node {
                inner: Arc::new(NodeInner {
                    prefix: Vec::new(),
                    children_hash: Cached::new(),
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
    pub fn into_children(mut self) -> Result<OrdMap<u8, Node<P, T>>, Node<P, T>> {
        if !self.inner.prefix.is_empty() {
            // Path-compressed: pop the top byte and rewrap self under it.
            // The popped entry's intermediate hash cache goes with it; the
            // children-level hash and every shorter prefix-level hash stay
            // valid because the children and the surviving byte sequence
            // are unchanged.
            let inner = Arc::make_mut(&mut self.inner);
            let (index, _hash) = inner.prefix.pop().expect("non-empty prefix");
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
    pub fn leaf(version: Version<P>, value: Message<T>) -> Self {
        Node {
            inner: Arc::new(NodeInner {
                prefix: Vec::new(),
                children_hash: Cached::new(),
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
    /// Hashes are lazily computed and cached until the tree structure changes
    /// to invalidate them.
    ///
    /// The hashing convention: a leaf's "hash" is the distinguished sentinel
    /// `[0xff; 32]`, a branch's is the hash of 256 concatenated child hashes
    /// (with `[0x00; 32]` in empty slots). Hashing should not depend on path
    /// compression; we ensure this by hashing "virtual" nodes as we traverse up
    /// a compressed path.
    pub fn hash(&self) -> blake3::Hash {
        // Walk the compressed prefix top-down: prefix[last] is the shallowest
        // byte (this node's own observable level), prefix[0] is the deepest.
        // Each entry's `Cached` slot holds the hash at that level — i.e. the
        // result of wrapping the children hash through prefix[0..=i]. We start
        // at the top and recurse downward only on a cache miss, so a hot
        // top-level cache returns immediately without touching any lower entry
        // or computing the children hash. On a miss at level i we recurse to
        // level i-1, wrap the result by prefix[i].0, and populate prefix[i].1
        // along the way out. The base case (i == -1, represented by `len == 0`)
        // returns the children-level hash.
        self.hash_at(self.inner.prefix.len())
    }

    /// Hash of this node observed `len` levels above its children — i.e. the
    /// children hash wrapped by `prefix[0..len]`. `len == 0` is the children
    /// level itself; `len == prefix.len()` is the node's own observable level.
    fn hash_at(&self, len: usize) -> blake3::Hash {
        let Some(i) = len.checked_sub(1) else {
            return self.children_hash();
        };
        let (byte, cached) = &self.inner.prefix[i];
        let byte = *byte;
        cached.clone_or_compute(|| {
            // Each virtual branch is 256 × 32 = 8192 bytes: all zero-slots
            // except for the one occupied child. We build the full buffer
            // contiguously to avoid 256 separate tiny update calls per byte.
            let lower = self.hash_at(i);
            let mut buf = [0u8; 256 * 32];
            buf[byte as usize * 32..][..32].copy_from_slice(lower.as_bytes());
            blake3::hash(&buf)
        })
    }

    /// Hash of this node's children, before any compressed-prefix wrapping.
    /// Independent of the prefix, so push/pop on the prefix never invalidates
    /// it.
    fn children_hash(&self) -> blake3::Hash {
        self.inner
            .children_hash
            .clone_or_compute(|| match &self.inner.children {
                Children::Leaf(_) => [0xff; 32].into(),
                Children::Branch(branch) => {
                    let mut buf = [0u8; 256 * 32];
                    for (&i, child) in branch.iter() {
                        buf[i as usize * 32..][..32].copy_from_slice(child.hash().as_bytes());
                    }
                    blake3::hash(&buf)
                }
            })
    }

    /// Get the version of this node (the maximal version of all children).
    pub fn version(&self) -> &Version<P> {
        &self.inner.version
    }

    /// Place a node beneath the given child index, increasing its height by one.
    /// The new top byte gets a fresh empty cache slot; every byte below it
    /// keeps its existing intermediate-hash cache (the byte sequence at those
    /// levels is unchanged), as does the children-level hash.
    fn beneath(mut self, index: u8) -> Node<P, T> {
        let inner = Arc::make_mut(&mut self.inner);
        inner.prefix.push((index, Cached::new()));
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
