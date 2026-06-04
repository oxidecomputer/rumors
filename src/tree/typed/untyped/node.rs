use std::fmt::Debug;
use std::mem;
use std::sync::{Arc, OnceLock};

use borsh::BorshSerialize;
use imbl::OrdMap;

use crate::{message::Message, tree::typed::Hash, version::Version};

pub struct Node<T> {
    inner: Arc<NodeInner<T>>,
}

impl<T> Clone for Node<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

struct NodeInner<T> {
    /// Compressed path above this node's own branching level, stored with the
    /// deepest byte at index 0 and the shallowest byte at the last index. An
    /// empty prefix means the node is not path-compressed above its level.
    ///
    /// Only the path bytes are stored: every level's hash is recoverable by
    /// wrapping the children's hash up through these bytes (see
    /// [`NodeInner::compute_hash`]), and the cheap commitment makes that
    /// recomputation negligible.
    prefix: Vec<u8>,
    /// The node's observable hash (the hash of the subtree as seen from the top
    /// of its compressed prefix), computed lazily on first read and memoized.
    /// The memo is a pure function of the subtree, so it is safe to share
    /// across the structurally-shared (copy-on-write) clones a forked tree
    /// produces. Any mutation of `prefix` or `children` invalidates it and must
    /// reset this cell.
    hash: OnceLock<Hash>,
    /// The maximal version of any child of this node.
    version: Version,
    /// The children of this node: either a leaf, or a branch point.
    children: Children<T>,
}

impl<T> NodeInner<T> {
    /// Compute the observable hash from scratch: hash the children (a leaf
    /// constant or the branch commitment over their hashes), then wrap that up
    /// through the compressed prefix one byte at a time, deepest byte first.
    /// A single-child wrap and a materialized one-child branch share the same
    /// rule, so the result is independent of how the path is compressed.
    fn compute_hash(&self) -> Hash {
        let mut hash = match &self.children {
            Children::Leaf(_) => Hash::leaf(),
            Children::Branch(branch) => {
                Hash::branch(branch.iter().map(|(radix, child)| (*radix, child.hash())))
            }
        };
        // `prefix[0]` is the deepest level (closest to the children), so
        // folding front-to-back wraps from the bottom up to the observable top.
        for &byte in &self.prefix {
            hash = Hash::branch([(byte, hash)]);
        }
        hash
    }
}

impl<T> Clone for NodeInner<T> {
    fn clone(&self) -> Self {
        Self {
            prefix: self.prefix.clone(),
            hash: self.hash.clone(),
            version: self.version.clone(),
            children: self.children.clone(),
        }
    }
}

impl<T: Debug> Debug for Node<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("prefix", &hex::encode(&self.inner.prefix))
            .field("version", &self.inner.version)
            .field("children", &self.inner.children)
            .finish()
    }
}

/// The children of a node.
#[derive(Debug)]
enum Children<T> {
    /// A direct leaf, at the true bottom of the tree.
    Leaf(Message<T>),
    /// A materialized branch point, with the invariant that there are always >=
    /// 2 branches (or else they should be path-compressed away).
    Branch(OrdMap<u8, Node<T>>),
}

impl<T> Clone for Children<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Leaf(l) => Self::Leaf(l.clone()),
            Self::Branch(b) => Self::Branch(b.clone()),
        }
    }
}

impl<T> Node<T> {
    /// Construct a new branch node from a list of children with distinct
    /// indices (inverse to [`Node::into_children`]).
    pub fn branch(children: OrdMap<u8, Node<T>>) -> Option<Self> {
        match children.len() {
            0 => None,
            1 => {
                let Some((index, node)) = children.into_iter().next() else {
                    unreachable!("a map with 1 element cannot fail to iterate");
                };
                Some(node.beneath(index))
            }
            _ => {
                // A branch's version is the join (least upper bound) of its
                // children's versions, accumulated from the empty version (the
                // lattice bottom). Drive the joins through a single `Batch` so
                // the working form is materialized once and repacked once,
                // rather than once per child, and join by reference so no
                // child's version is cloned. The hash is left lazy: it is
                // computed on first observation, not at construction.
                let mut version = Version::new();
                {
                    let mut batch = version.batch();
                    for child in children.values() {
                        batch |= child.version();
                    }
                }
                Some(Node {
                    inner: Arc::new(NodeInner {
                        prefix: Vec::new(),
                        hash: OnceLock::new(),
                        version,
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
    pub fn into_children(mut self) -> Result<OrdMap<u8, Node<T>>, Node<T>> {
        if !self.inner.prefix.is_empty() {
            // Path-compressed: pop the top (shallowest) byte and rewrap self
            // under it. Popping shortens the prefix, so the observable hash
            // moves down one virtual level; the memoized hash is now stale and
            // must be cleared so the next read recomputes from the shortened
            // prefix.
            let inner = Arc::make_mut(&mut self.inner);
            let index = inner.prefix.pop().expect("non-empty prefix");
            inner.hash = OnceLock::new();
            Ok(OrdMap::from_iter([(index, self)]))
        } else {
            match &self.inner.children {
                Children::Leaf(_) => Err(self),
                Children::Branch(_) => {
                    // Extract the children map; self is dropped, so leaving
                    // its precomputed metadata referencing the now-vacated
                    // branch is harmless.
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
    pub fn leaf(version: Version, value: Message<T>) -> Self {
        Node {
            inner: Arc::new(NodeInner {
                prefix: Vec::new(),
                hash: OnceLock::new(),
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
    /// The hash is computed lazily on first call and memoized, so the first
    /// read of a freshly-built subtree is `O(nodes)` and every read thereafter
    /// is an `O(1)` field load. The convention (see [`Hash::branch`] and
    /// [`Hash::leaf`]): a leaf hashes to `blake3(LEAF_TAG)`; a branch to
    /// `blake3(BRANCH_TAG ‖ r₀ ‖ h₀ ‖ …)` over its children in ascending radix
    /// order. Hashing does not depend on path compression: a one-child branch
    /// and a node path-compressed by one byte produce identical hashes.
    pub fn hash(&self) -> Hash {
        *self.inner.hash.get_or_init(|| self.inner.compute_hash())
    }

    /// Get the version of this node (the maximal version of all children).
    pub fn version(&self) -> &Version {
        &self.inner.version
    }

    /// Number of path-compressed prefix bytes carried on this node — i.e.,
    /// the count of virtual-branch levels collapsed above the node's actual
    /// content. Zero for a leaf or a non-compressed branch.
    #[cfg(test)]
    pub fn compressed_prefix_len(&self) -> usize {
        self.inner.prefix.len()
    }

    /// Borsh-serialize the node in its in-memory layout. This is the
    /// canonical encoder: the typed `BorshSerialize` impl is a thin
    /// delegate over it, and on the decode side the same shape is
    /// reconstructed via the chain-reader trick that synthesizes per-level
    /// `prefix_len` bytes (see the module docs on
    /// [`crate::tree::traverse::mirror`] for the full wire-format spec).
    ///
    /// The encoded shape, in order, is:
    ///
    /// 1. `prefix_len: u8` — the path-compressed prefix's byte count;
    /// 2. `prefix_len` head bytes, shallowest first (decoders peel from the
    ///    outermost compressed level inward);
    /// 3. the body, dispatched on `children`:
    ///    - [`Children::Leaf`]: `version: Version`, then `message: Message<T>`;
    ///    - [`Children::Branch`]: `count_minus_two: u8`, then for each
    ///      child (in canonical `OrdMap` key order): `radix: u8`,
    ///      `serialize_to(child)`.
    ///
    /// Leaf-vs-branch is **not** tagged on the wire: at the receiver, the
    /// typed height and the running `prefix_len` together name the body's
    /// shape. Multi-child branches always carry at least two children, by
    /// the path-compression invariant.
    pub fn serialize_to<W: borsh::io::Write>(&self, writer: &mut W) -> borsh::io::Result<()>
where {
        let prefix_len = u8::try_from(self.inner.prefix.len()).map_err(|_| {
            borsh::io::Error::new(
                borsh::io::ErrorKind::InvalidData,
                "node prefix length does not fit in a u8",
            )
        })?;
        prefix_len.serialize(writer)?;
        // Wire order is shallowest-first; the in-memory `prefix` stores the
        // shallowest byte at the last index, so iterate in reverse.
        for byte in self.inner.prefix.iter().rev() {
            byte.serialize(writer)?;
        }
        match &self.inner.children {
            Children::Leaf(msg) => {
                self.inner.version.serialize(writer)?;
                msg.serialize(writer)?;
            }
            Children::Branch(children) => {
                debug_assert!(
                    (2..=256).contains(&children.len()),
                    "multi-child branch must have 2..=256 children",
                );
                let count_minus_two = u8::try_from(children.len() - 2).map_err(|_| {
                    borsh::io::Error::new(
                        borsh::io::ErrorKind::InvalidData,
                        "branch children count does not fit in count_minus_two: u8",
                    )
                })?;
                count_minus_two.serialize(writer)?;
                for (radix, child) in children {
                    radix.serialize(writer)?;
                    child.serialize_to(writer)?;
                }
            }
        }
        Ok(())
    }

    /// Place a node beneath the given child index, increasing its height by
    /// one. Pushing onto the prefix raises the observable hash by one virtual
    /// level, so the memoized hash is invalidated and recomputed lazily on the
    /// next read.
    pub fn beneath(mut self, index: u8) -> Node<T> {
        let inner = Arc::make_mut(&mut self.inner);
        inner.prefix.push(index);
        inner.hash = OnceLock::new();
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
            Children::Branch(children) => {
                children.len() >= 2 && children.values().all(Self::is_max_compressed)
            }
        }
    }
}

impl<T> Eq for Node<T> {}

impl<T> PartialEq for Node<T> {
    fn eq(&self, other: &Self) -> bool {
        self.hash() == other.hash()
    }
}

/// A lazy depth-first iterator over every live leaf in a subtree, yielding each
/// leaf's reconstructed 32-byte path [`Key`], its [`Version`], and a borrowed
/// handle to its message [`Arc`].
///
/// The iterator is lazy: a single `next()` descends only far enough to reach
/// the next leaf, so the first item is produced after walking one root-to-leaf
/// spine rather than the whole tree. Each pending node on the stack carries the
/// path bytes accumulated to reach it (above its own compressed prefix); since
/// the tree's depth is fixed at 32, those buffers never exceed 32 bytes.
///
/// Iteration order is unspecified — matching the `on_message` callback contract
/// in [`Known`](crate::Known), which makes no ordering promise.
///
/// `Iter` is `Send + Sync` whenever `T: Send + Sync`: it holds only `&Node<T>`
/// references and `Vec<u8>` path buffers.
pub struct Iter<'a, T> {
    /// Pending `(node, path-to-reach-it)` frames, LIFO. Empty once exhausted.
    stack: Vec<(&'a Node<T>, Vec<u8>)>,
}

impl<'a, T> Iter<'a, T> {
    /// Iterate the subtree rooted at `node` (a height-32 root, so every leaf's
    /// path is a full 32-byte [`Key`]).
    pub(crate) fn root(node: &'a Node<T>) -> Self {
        Self {
            stack: vec![(node, Vec::with_capacity(32))],
        }
    }

    /// The empty iterator, for a tree with no root.
    pub(crate) fn empty() -> Self {
        Self { stack: Vec::new() }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (crate::tree::key::Key, &'a Version, &'a Arc<T>);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((node, mut path)) = self.stack.pop() {
            // The compressed prefix sits above this node's level and is stored
            // shallowest-last, so replay it shallowest-first to extend the path.
            for &byte in node.inner.prefix.iter().rev() {
                path.push(byte);
            }
            match &node.inner.children {
                Children::Leaf(message) => {
                    let path = <[u8; 32]>::try_from(path)
                        .expect("a leaf sits at depth 32, so its path is 32 bytes");
                    return Some((
                        crate::tree::key::Key(path),
                        &node.inner.version,
                        message.as_arc(),
                    ));
                }
                Children::Branch(children) => {
                    // Push each child with its own extended path; the owned
                    // buffer per frame is what keeps the descent lazy without a
                    // separate pop phase.
                    for (radix, child) in children.iter() {
                        let mut child_path = path.clone();
                        child_path.push(*radix);
                        self.stack.push((child, child_path));
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod test;
