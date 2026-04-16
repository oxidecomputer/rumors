use std::{hash::Hash, marker::PhantomData, mem};

use imbl::OrdMap;

use bytes::Bytes;

use super::height::{self, Height, S, Z};
use super::untyped;

pub use untyped::Leaf;

/// The typed node with a height of 32; the root of the tree.
pub type Root<P> = Node<P, height::Root>;

/// The type of children of a given height.
pub type Children<P, H> = OrdMap<u8, Node<P, H>>;

/// A typed node which enforces the structural validity of the constructed tree
/// at compile-time.
#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct Node<P: Clone + Eq + Hash + AsRef<[u8]>, H: Height> {
    height: PhantomData<H>,
    inner: untyped::Node<P>,
}

impl<P: Clone + Eq + Hash + AsRef<[u8]>, H: Height> Node<P, H> {
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
        self.inner.hash()
    }
}

impl<P: Clone + Eq + Hash + AsRef<[u8]>, H: Height> Node<P, S<H>>
where
    S<H>: Height,
{
    /// Construct a new branch node from a map of children (inverse to
    /// [`Node::into_children`]).
    pub fn branch(children: Children<P, H>) -> Option<Self> {
        // Transmute the map of children from typed nodes with the correct
        // height into untyped nodes.
        //
        // SAFETY: TypedNode is #[repr(transparent)] and `OrdMap` treats values
        // in the map parametrically (i.e. no use of `TypeId`, etc.)
        let children = unsafe { mem::transmute(children) };

        Some(Node {
            height: PhantomData,
            inner: untyped::Node::branch(children)?,
        })
    }

    /// Convert a node into a map from child index to child node (inverse to
    /// [`Node::branch`]).
    pub fn into_children(self) -> Children<P, H> {
        // Transmute the map of children into typed nodes with the correct
        // height, to recursively enforce type-safe height.
        //
        // SAFETY: TypedNode is #[repr(transparent)] and `OrdMap` treats values
        // in the map parametrically (i.e. no use of `TypeId`, etc.)
        unsafe { mem::transmute(self.inner.into_children()) }
    }
}

impl<P: Clone + Eq + Hash + AsRef<[u8]>> Node<P, Z> {
    /// Construct a new leaf node.
    pub fn leaf(party: P, version: u64, value: Bytes) -> Self {
        Self {
            height: PhantomData,
            inner: untyped::Node::leaf(party, version, value),
        }
    }

    /// Get a reference to the leaf at this node.
    pub fn as_leaf(&self) -> &Leaf<P> {
        self.inner
            .as_leaf()
            .expect("typed leaf failed to be a leaf")
    }
}

impl<P: Clone + Eq + Hash + AsRef<[u8]>, H: Height> Eq for Node<P, H> {}

impl<P: Clone + Eq + Hash + AsRef<[u8]>, H: Height> PartialEq for Node<P, H> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}
