use std::{fmt::Debug, marker::PhantomData, mem};

use imbl::OrdMap;

use crate::{Message, Version};

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
    pub fn hash(&self) -> blake3::Hash {
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

    pub fn root_hash(node: &Option<Root<P, T>>) -> blake3::Hash
    where
        P: Clone + Ord + AsRef<[u8]>,
        T: Clone,
    {
        node.as_ref()
            .map(|n| n.hash())
            .unwrap_or_else(|| [0; 32].into())
    }
}

impl<P: Clone + Ord + AsRef<[u8]>, T: Clone + Eq, H: Height> Eq for Node<P, T, H> {}

impl<P: Clone + Ord + AsRef<[u8]>, T: Clone + PartialEq, H: Height> PartialEq for Node<P, T, H> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}
