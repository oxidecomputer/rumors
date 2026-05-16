use std::{fmt::Debug, marker::PhantomData, mem};

use borsh::{BorshDeserialize, BorshSerialize};
use imbl::OrdMap;

use crate::{Message, Version};

use super::hash::Hash;
use super::height::{self, Height, S, Z};
use super::levels::{Top, levels};
use super::untyped;

/// The typed node with a height of 32; the root of the tree.
pub type Root<P, T> = Node<P, T, height::Root>;

/// The type of children of a given height.
pub type Children<P, T, H> = OrdMap<u8, Node<P, T, H>>;

/// A typed node which enforces the structural validity of the constructed tree
/// at compile-time.
#[derive(Clone)]
#[repr(transparent)]
pub struct Node<P: Ord + AsRef<[u8]>, T, H: Height> {
    height: PhantomData<H>,
    inner: untyped::Node<P, T>,
}

impl<P, T, H> Debug for Node<P, T, H>
where
    P: Debug + Ord + AsRef<[u8]>,
    T: Debug,
    H: Height,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl<P: Clone + Ord + AsRef<[u8]>, T: Clone, H: Height> Node<P, T, H> {
    /// Get the version of this node.
    pub fn version(&self) -> &Version<P> {
        self.inner.version()
    }

    /// Number of path-compressed prefix bytes on this node — i.e., the
    /// count of singleton virtual-branch levels collapsed above the node's
    /// actual content. Zero for a leaf or a non-compressed branch.
    #[cfg(test)]
    pub fn compressed_prefix_len(&self) -> usize {
        self.inner.compressed_prefix_len()
    }

    /// Hash the subtree rooted at this node.
    ///
    /// Hashes are computed eagerly at node construction and stored, so this
    /// is an O(1) field read.
    ///
    /// The hashing convention: a leaf's "hash" is the distinguished sentinel
    /// `[0xff; 32]`, a branch's is the hash of 256 concatenated child hashes
    /// (with `[0x00; 32]` in empty slots). Hashing does not depend on path
    /// compression: a one-child branch and a node path-compressed by one byte
    /// produce identical hashes.
    pub fn hash(&self) -> Hash {
        self.inner.hash()
    }
}

impl<P: Clone + Ord + AsRef<[u8]>, T: Clone, H: Height> Node<P, T, S<H>>
where
    S<H>: Height,
{
    /// Construct a new branch node from a map of children (inverse to
    /// [`Node::into_children`]).
    pub fn branch(children: Children<P, T, H>) -> Option<Self> {
        // Transmute the map of children from typed nodes with the correct
        // height into untyped nodes.
        //
        // SAFETY: TypedNode is #[repr(transparent)] and `OrdMap` treats values
        // in the map parametrically (i.e. no use of `TypeId`, etc.)
        let children = unsafe {
            mem::transmute::<OrdMap<u8, Node<P, T, H>>, OrdMap<u8, untyped::Node<P, T>>>(children)
        };

        Some(Node {
            height: PhantomData,
            inner: untyped::Node::branch(children)?,
        })
    }

    /// Convert a node into a map from child index to child node (inverse to
    /// [`Node::branch`]).
    pub fn into_children(self) -> Children<P, T, H> {
        // Transmute the map of children into typed nodes with the correct
        // height, to recursively enforce type-safe height.
        //
        // SAFETY: TypedNode is #[repr(transparent)] and `OrdMap` treats values
        // in the map parametrically (i.e. no use of `TypeId`, etc.)
        unsafe { mem::transmute(self.inner.into_children()) }
    }

    /// Wrap `child` (at height `H`) beneath slot `index` of a virtual branch
    /// at height `S<H>`. The result is the typed counterpart of
    /// `untyped::Node::beneath`: it path-compresses a single-child wrap into
    /// the underlying node's prefix without materializing the intervening
    /// branch level.
    pub fn beneath(child: Node<P, T, H>, index: u8) -> Self {
        Node {
            height: PhantomData,
            inner: child.inner.beneath(index),
        }
    }
}

impl<P: Clone + Ord + AsRef<[u8]>, T: Clone> Node<P, T, Z> {
    /// Construct a new leaf node from a versioned message.
    pub fn leaf(version: Version<P>, message: Message<T>) -> Self {
        Self {
            height: PhantomData,
            inner: untyped::Node::leaf(version, message),
        }
    }

    /// Get a reference to the message at this leaf node.
    pub fn message(&self) -> &Message<T> {
        self.inner
            .as_leaf()
            .expect("typed leaf failed to be a leaf")
    }
}

impl<P, T> Node<P, T, height::Root>
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
{
    pub fn levels(node: Option<Root<P, T>>) -> Top<P, T> {
        levels(node)
    }

    pub fn root_hash(node: &Option<Root<P, T>>) -> Hash
    where
        P: Clone + Ord + AsRef<[u8]>,
        T: Clone,
    {
        node.as_ref().map(|n| n.hash()).unwrap_or_default()
    }
}

impl<P: Clone + Ord + AsRef<[u8]>, T: Clone + Eq, H: Height> Eq for Node<P, T, H> {}

impl<P: Clone + Ord + AsRef<[u8]>, T: Clone + PartialEq, H: Height> PartialEq for Node<P, T, H> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

// Borsh wire format. Serialization is height-uniform: every typed
// `Node<P, T, H>` delegates to [`untyped::Node::serialize_to`], which
// emits the in-memory representation directly (prefix length, head bytes,
// then either a leaf body or a `count_minus_two` + children list). No
// leaf-vs-branch tag is needed on the wire — at the receiver, the typed
// height together with the running `prefix_len` names the body's shape.
//
// Deserialization at typed height `H` reads `prefix_len`, then either
// decodes the body directly (when `prefix_len == 0`) or peels one head
// byte and recurses at the next-finer typed height — synthesizing the
// `prefix_len - 1` byte for the inner reader via
// [`borsh::io::Read::chain`]. The recursion bottoms out at the typed
// level matching the structural level of the underlying body: a multi-
// child branch at `S<_>` heights, or a leaf at `Z`.
//
// Multi-child branches always carry at least two children (the path-
// compression invariant); singletons appear on the wire only as
// `prefix_len > 0` and reconstruct through [`Node::beneath`].

impl<P, T, H> BorshSerialize for Node<P, T, H>
where
    P: Clone + Ord + AsRef<[u8]> + BorshSerialize,
    T: Clone,
    H: Height,
{
    fn serialize<W: borsh::io::Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        self.inner.serialize_to(writer)
    }
}

impl<P, T> BorshDeserialize for Node<P, T, Z>
where
    P: Clone + Ord + AsRef<[u8]> + BorshDeserialize,
    T: Clone + BorshDeserialize,
{
    fn deserialize_reader<R: borsh::io::Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let prefix_len = u8::deserialize_reader(reader)?;
        if prefix_len != 0 {
            return Err(borsh::io::Error::new(
                borsh::io::ErrorKind::InvalidData,
                "leaf height cannot carry a prefix",
            ));
        }
        let version = Version::<P>::deserialize_reader(reader)?;
        let message = Message::<T>::deserialize_reader(reader)?;
        Ok(Node::leaf(version, message))
    }
}

impl<P, T, H> BorshDeserialize for Node<P, T, S<H>>
where
    P: Clone + Ord + AsRef<[u8]> + BorshDeserialize,
    T: Clone + BorshDeserialize,
    H: Height,
    S<H>: Height,
    Node<P, T, H>: BorshDeserialize,
{
    fn deserialize_reader<R: borsh::io::Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let prefix_len = u8::deserialize_reader(reader)?;
        if (prefix_len as usize) > <S<H>>::HEIGHT {
            return Err(borsh::io::Error::new(
                borsh::io::ErrorKind::InvalidData,
                "prefix length exceeds typed height",
            ));
        }
        if prefix_len == 0 {
            let count_minus_two = u8::deserialize_reader(reader)?;
            let count = (count_minus_two as usize) + 2;
            if count > 256 {
                return Err(borsh::io::Error::new(
                    borsh::io::ErrorKind::InvalidData,
                    "branch children count exceeds 256",
                ));
            }
            let mut children: OrdMap<u8, Node<P, T, H>> = OrdMap::new();
            let mut prev: Option<u8> = None;
            for _ in 0..count {
                let radix = u8::deserialize_reader(reader)?;
                if let Some(p) = prev
                    && radix <= p
                {
                    return Err(borsh::io::Error::new(
                        borsh::io::ErrorKind::InvalidData,
                        "branch radices not strictly ascending",
                    ));
                }
                prev = Some(radix);
                let child = Node::<P, T, H>::deserialize_reader(reader)?;
                children.insert(radix, child);
            }
            Node::branch(children).ok_or_else(|| {
                borsh::io::Error::new(
                    borsh::io::ErrorKind::InvalidData,
                    "branch could not be reconstructed",
                )
            })
        } else {
            let head = u8::deserialize_reader(reader)?;
            // Prepend `prefix_len - 1` to the rest of the stream so the
            // inner typed level reads it as if it were on the wire,
            // synthesizing the singleton-chain recursion without a helper
            // trait.
            let synthesized = [prefix_len - 1];
            let mut chained = borsh::io::Read::chain(synthesized.as_slice(), &mut *reader);
            let inner = Node::<P, T, H>::deserialize_reader(&mut chained)?;
            Ok(Node::beneath(inner, head))
        }
    }
}
