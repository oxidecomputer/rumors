use std::fmt::Debug;
use std::mem;
use std::sync::{Arc, OnceLock};

use borsh::BorshSerialize;
use imbl::OrdMap;

use crate::{message::Message, tree::typed::Hash, version::Version};

mod iter;
pub use iter::{Iter, Range};

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
    /// [`Node::hash`]), and the cheap commitment makes that
    /// recomputation negligible.
    prefix: Vec<u8>,
    /// The node's observable hash (the hash of the subtree as seen from the top
    /// of its compressed prefix), computed lazily on first read and memoized.
    /// Unlike the ceiling/floor memos, this lives on `NodeInner` rather than
    /// inside [`Children::Branch`] so a path-compressed leaf memoizes its hash
    /// too: a deep single-leaf spine costs the wrap only once. The memo is a
    /// pure function of the subtree, so it is safe to share across the
    /// structurally-shared (copy-on-write) clones a forked tree produces. It
    /// folds in the compressed prefix, so any mutation of `prefix` *or*
    /// `children` invalidates it and must reset this cell.
    hash: OnceLock<Hash>,
    /// The children of this node: either a leaf, or a branch point.
    children: Children<T>,
}

impl<T> Clone for NodeInner<T> {
    fn clone(&self) -> Self {
        Self {
            prefix: self.prefix.clone(),
            hash: self.hash.clone(),
            children: self.children.clone(),
        }
    }
}

impl<T: Debug> Debug for Node<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("prefix", &hex::encode(&self.inner.prefix))
            .field("children", &self.inner.children)
            .finish()
    }
}

/// The children of a node.
#[derive(Debug)]
enum Children<T> {
    /// A direct leaf, at the true bottom of the tree.
    Leaf {
        /// The version of this leaf.
        version: Version,
        /// The payload of this leaf.
        message: Message<T>,
    },
    /// A materialized branch point, with the invariant that there are always >=
    /// 2 branches (or else they should be path-compressed away).
    Branch {
        /// The maximal version of any child of this node, computed lazily on
        /// first read and memoized.
        ///
        /// This must be reset whenever the branch's children change, but not
        /// when its prefix does.
        ceiling: OnceLock<Version>,
        /// The minimal version of any child of this node, computed lazily on
        /// first read and memoized.
        ///
        /// This must be reset whenever the branch's children change, but not
        /// when its prefix does.
        floor: OnceLock<Version>,
        /// The number of total leaves under this branch.
        leaves: usize,
        /// The children of this branch.
        children: OrdMap<u8, Node<T>>,
    },
}

impl<T> Clone for Children<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Leaf { version, message } => Self::Leaf {
                version: version.clone(),
                message: message.clone(),
            },
            // The lazy memos are pure functions of the (shared) subtree, so
            // cloning the `OnceLock`s carries any already-computed value over
            // to the copy-on-write clone rather than discarding it.
            Self::Branch {
                ceiling,
                floor,
                leaves,
                children,
            } => Self::Branch {
                ceiling: ceiling.clone(),
                floor: floor.clone(),
                leaves: *leaves,
                children: children.clone(),
            },
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
            _ => Some(Node {
                inner: Arc::new(NodeInner {
                    prefix: Vec::new(),
                    hash: OnceLock::new(),
                    children: Children::Branch {
                        ceiling: OnceLock::new(),
                        floor: OnceLock::new(),
                        leaves: children.values().map(Node::len).sum(),
                        children,
                    },
                }),
            }),
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
                Children::Leaf { .. } => Err(self),
                Children::Branch { .. } => {
                    // Extract the children map; self is dropped, so leaving
                    // its precomputed metadata referencing the now-vacated
                    // branch is harmless.
                    let inner = Arc::make_mut(&mut self.inner);
                    let Children::Branch {
                        children: branch, ..
                    } = &mut inner.children
                    else {
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
                children: Children::Leaf {
                    message: value,
                    version,
                },
            }),
        }
    }

    /// Get a reference to the leaf at this node, if it is a leaf.
    pub fn as_leaf(&self) -> Option<&Message<T>> {
        match &self.inner.children {
            Children::Leaf { message, .. } => Some(message),
            _ => None,
        }
    }

    /// Look up the leaf at `path` beneath this node: a single root-to-leaf
    /// descent costing `O(depth)`, never a scan. `None` when no live leaf
    /// sits at that path.
    pub fn get(&self, mut path: &[u8]) -> Option<(&Version, &Message<T>)> {
        let mut node = self;
        loop {
            // Consume the compressed prefix, shallowest byte first (it is
            // stored shallowest-last); any divergence means the path exits
            // the tree inside the compressed span.
            for &byte in node.inner.prefix.iter().rev() {
                match path.split_first() {
                    Some((&next, rest)) if next == byte => path = rest,
                    _ => return None,
                }
            }
            match &node.inner.children {
                // A full 32-byte path lands exactly at a leaf; a leftover
                // tail means the path was deeper than the tree.
                Children::Leaf { version, message } => {
                    return path.is_empty().then_some((version, message));
                }
                Children::Branch { children, .. } => {
                    let (radix, rest) = path.split_first()?;
                    node = children.get(radix)?;
                    path = rest;
                }
            }
        }
    }

    /// Get the number of leaves under a node.
    pub fn len(&self) -> usize {
        match self.inner.children {
            Children::Leaf { .. } => 1,
            Children::Branch { leaves, .. } => leaves,
        }
    }

    /// Whether two nodes share the same backing allocation: a sufficient
    /// (not necessary) test for structural equality that touches no hash.
    /// Forked trees share their unchanged subtrees by `Arc`, so an in-memory
    /// merge can short-circuit those in `O(1)`, even with cold memos, before
    /// falling back to the content hash for subtrees that diverged in memory
    /// but hold equal content.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
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
        *self.inner.hash.get_or_init(|| {
            // Start from the node's base hash at its own level: the `Hash::leaf()`
            // constant for a leaf, or the branch commitment over the children's
            // hashes for a branch.
            let mut hash = match &self.inner.children {
                Children::Leaf { .. } => Hash::leaf(),
                Children::Branch { children, .. } => {
                    Hash::branch(children.iter().map(|(radix, child)| (*radix, child.hash())))
                }
            };
            // Wrap that base up through the compressed prefix one byte at a
            // time, deepest byte first. `prefix[0]` is the deepest level
            // (closest to the children), so folding front-to-back wraps from the
            // bottom up to the observable top. A single-child wrap and a
            // materialized one-child branch share this rule, so the result is
            // independent of how the path is compressed.
            for &byte in &self.inner.prefix {
                hash = Hash::branch([(byte, hash)]);
            }
            hash
        })
    }

    /// Get the ceiling version of this node (the maximal version of all
    /// children).
    ///
    /// Like [`hash`](Self::hash), the ceiling is computed lazily on first call
    /// and memoized: a leaf's is set at construction, and a branch's is the
    /// join of its children's ceilings, computed once on demand. The memo is a
    /// pure function of the subtree, so it is safe to share across the
    /// structurally-shared clones a forked tree produces.
    pub fn ceiling(&self) -> &Version {
        match &self.inner.children {
            Children::Leaf { version, .. } => version,
            Children::Branch {
                ceiling, children, ..
            } => ceiling.get_or_init(|| {
                // The join (least upper bound) of the children's ceilings,
                // accumulated from the empty version (the lattice bottom, the
                // join identity). Path compression doesn't change which leaves
                // the subtree contains, so the prefix plays no part. Drive the
                // joins through a single `Batch` so the working form is
                // materialized once and repacked once, rather than once per
                // child, and join by reference so no child's version is cloned.
                let mut version = Version::new();
                {
                    let mut batch = version.batch();
                    for child in children.values() {
                        batch |= child.ceiling();
                    }
                }
                version
            }),
        }
    }

    /// Get the floor version of this node (the minimal version of all
    /// children).
    ///
    /// Like [`hash`](Self::hash), the floor is computed lazily on first call
    /// and memoized: a leaf's is set at construction, and a branch's is the
    /// meet of its children's floors, computed once on demand. The memo is a
    /// pure function of the subtree, so it is safe to share across the
    /// structurally-shared clones a forked tree produces.
    pub fn floor(&self) -> &Version {
        match &self.inner.children {
            Children::Leaf { version, .. } => version,
            Children::Branch {
                floor, children, ..
            } => floor.get_or_init(|| {
                // The meet (greatest lower bound) of the children's floors.
                // Unlike the join, the meet has no identity element (there is
                // no top version), so seed with the first child's floor and
                // meet the rest in. A branch always has >= 2 children by the
                // path-compression invariant, so `next()` cannot be empty.
                // Drive the meets through a single `Batch` so the working form
                // is materialized once and repacked once, and meet by reference
                // so no child's version is cloned.
                let mut children = children.values();
                let mut version = children
                    .next()
                    .expect("a branch always has >= 2 children")
                    .floor()
                    .clone();
                {
                    let mut batch = version.batch();
                    for child in children {
                        batch &= child.floor();
                    }
                }
                version
            }),
        }
    }

    /// Whether this node's content is a single leaf (regardless of any
    /// path-compressed prefix above it). A leaf carries exactly one version,
    /// so its [`floor`](Self::floor) and [`ceiling`](Self::ceiling) coincide —
    /// which lets callers decide "keep or drop this whole subtree" from the
    /// version check alone, without exploding the compressed prefix.
    pub fn is_leaf(&self) -> bool {
        matches!(self.inner.children, Children::Leaf { .. })
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
    pub fn serialize_to<W: borsh::io::Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
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
            Children::Leaf { message, version } => {
                version.serialize(writer)?;
                message.serialize(writer)?;
            }
            Children::Branch { children, .. } => {
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
            Children::Leaf { .. } => true,
            Children::Branch { children, .. } => {
                children.len() >= 2 && children.values().all(Self::is_max_compressed)
            }
        }
    }
}

impl<T> Eq for Node<T> {}

impl<T> PartialEq for Node<T> {
    fn eq(&self, other: &Self) -> bool {
        // Shared backing settles equality with no hashing (and even cold): the
        // common case for forked/cloned trees and the subtrees they share. Only
        // distinct allocations fall back to the content hash.
        self.ptr_eq(other) || self.hash() == other.hash()
    }
}

#[cfg(test)]
mod test;
