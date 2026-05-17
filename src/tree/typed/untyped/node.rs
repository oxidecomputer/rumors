use std::fmt::Debug;
use std::mem;
use std::sync::Arc;

use borsh::BorshSerialize;
use imbl::OrdMap;

use crate::{message::Message, version::Version, tree::typed::Hash};

pub struct Node<P: Ord + AsRef<[u8]>, T> {
    inner: Arc<NodeInner<P, T>>,
}

impl<P: Ord + AsRef<[u8]>, T> Clone for Node<P, T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

struct NodeInner<P: Ord + AsRef<[u8]>, T> {
    /// Compressed path above this node's own branching level, stored with the
    /// deepest byte at index 0 and the shallowest byte at the last index. An
    /// empty prefix means the node is not path-compressed above its level.
    ///
    /// Each entry pairs a path byte with the precomputed hash for the virtual
    /// node sitting at that level.
    prefix: Vec<(u8, Hash)>,
    /// Hash of this node's children (the leaf sentinel or the branch-level
    /// hash), independent of any compressed prefix above it. Computed once at
    /// construction.
    children_hash: Hash,
    /// The maximal version of any child of this node.
    version: Version<P>,
    /// The children of this node: either a leaf, or a branch point.
    children: Children<P, T>,
}

impl<P: Clone + Ord + AsRef<[u8]>, T> Clone for NodeInner<P, T> {
    fn clone(&self) -> Self {
        Self {
            prefix: self.prefix.clone(),
            children_hash: self.children_hash.clone(),
            version: self.version.clone(),
            children: self.children.clone(),
        }
    }
}

impl<P: Debug + Ord + AsRef<[u8]>, T: Debug> Debug for Node<P, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("Node");
        s.field(
            "prefix",
            &hex::encode(
                self.inner
                    .prefix
                    .iter()
                    .map(|(b, _)| *b)
                    .collect::<Vec<_>>()
                    .as_slice(),
            ),
        );

        s.field("version", &self.inner.version)
            .field("children", &self.inner.children)
            .finish()
    }
}

/// The children of a node.
#[derive(Debug)]
enum Children<P: Ord + AsRef<[u8]>, T> {
    /// A direct leaf, at the true bottom of the tree.
    Leaf(Message<T>),
    /// A materialized branch point, with the invariant that there are always >=
    /// 2 branches (or else they should be path-compressed away).
    Branch(OrdMap<u8, Node<P, T>>),
}

impl<P: Clone + Ord + AsRef<[u8]>, T> Clone for Children<P, T> {
    fn clone(&self) -> Self {
        match self {
            Self::Leaf(l) => Self::Leaf(l.clone()),
            Self::Branch(b) => Self::Branch(b.clone()),
        }
    }
}

/// Sentinel hash used as a leaf's "hash" so leaves are distinguishable from
/// any branch (which always hashes a 256-slot buffer). Empty branch slots
/// hash as `[0x00; 32]`, the natural zero-init of the staging buffer.
const LEAF_SENTINEL: [u8; 32] = [0xff; 32];

impl<P: Ord + Clone + AsRef<[u8]>, T> Node<P, T> {
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
                let children_hash = Hash::of(&buf);
                let version = Version::new(children.values().map(|n| n.version().clone()));
                Some(Node {
                    inner: Arc::new(NodeInner {
                        prefix: Vec::new(),
                        children_hash,
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
    pub fn into_children(mut self) -> Result<OrdMap<u8, Node<P, T>>, Node<P, T>> {
        if !self.inner.prefix.is_empty() {
            // Path-compressed: pop the top byte and rewrap self under it. The
            // popped entry's precomputed hash and cumulative forgotten are
            // dropped; every shorter prefix-level's stored hash and
            // cumulative forgotten remain valid because they were computed
            // independently of the popped level, and the children and the
            // surviving byte sequence are unchanged.
            let inner = Arc::make_mut(&mut self.inner);
            let (index, _hash) = inner.prefix.pop().expect("non-empty prefix");
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
    pub fn leaf(version: Version<P>, value: Message<T>) -> Self {
        Node {
            inner: Arc::new(NodeInner {
                prefix: Vec::new(),
                children_hash: Hash(LEAF_SENTINEL),
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
    pub fn hash(&self) -> Hash {
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
    ///    - [`Children::Leaf`]: `version: Version<P>`, then `message: Message<T>`;
    ///    - [`Children::Branch`]: `count_minus_two: u8`, then for each
    ///      child (in canonical `OrdMap` key order): `radix: u8`,
    ///      `serialize_to(child)`.
    ///
    /// Leaf-vs-branch is **not** tagged on the wire: at the receiver, the
    /// typed height and the running `prefix_len` together name the body's
    /// shape. Multi-child branches always carry at least two children, by
    /// the path-compression invariant.
    pub fn serialize_to<W: borsh::io::Write>(&self, writer: &mut W) -> borsh::io::Result<()>
    where
        P: BorshSerialize,
    {
        let prefix_len = u8::try_from(self.inner.prefix.len()).map_err(|_| {
            borsh::io::Error::new(
                borsh::io::ErrorKind::InvalidData,
                "node prefix length does not fit in a u8",
            )
        })?;
        prefix_len.serialize(writer)?;
        // Wire order is shallowest-first; the in-memory `prefix` stores the
        // shallowest byte at the last index, so iterate in reverse.
        for (byte, _hash) in self.inner.prefix.iter().rev() {
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
    /// one. Eagerly computes the new top-of-prefix hash by wrapping the old
    /// observable hash through one virtual-branch level.
    pub fn beneath(mut self, index: u8) -> Node<P, T> {
        let mut buf = [0u8; 256 * 32];
        buf[index as usize * 32..][..32].copy_from_slice(self.hash().as_bytes());
        let new_top_hash = Hash::of(&buf);
        let inner = Arc::make_mut(&mut self.inner);
        inner.prefix.push((index, new_top_hash));
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

impl<P: Ord + Clone + AsRef<[u8]>, T> Eq for Node<P, T> {}

impl<P: Ord + Clone + AsRef<[u8]>, T> PartialEq for Node<P, T> {
    fn eq(&self, other: &Self) -> bool {
        self.hash() == other.hash()
    }
}

#[cfg(test)]
mod test;
