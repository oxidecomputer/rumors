use std::{hash::Hash, marker::PhantomData, mem};

use imbl::OrdMap;

use crate::{Message, Version};

use super::height::{self, Height, S, Z};
use super::untyped;

/// The typed node with a height of 32; the root of the tree.
pub type Root<P, T> = Node<P, T, height::Root>;

/// The type of children of a given height.
pub type Children<P, T, H> = OrdMap<u8, Node<P, T, H>>;

/// A typed node which enforces the structural validity of the constructed tree
/// at compile-time.
#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct Node<P: Clone + Eq + Hash + AsRef<[u8]>, T, H: Height> {
    height: PhantomData<H>,
    inner: untyped::Node<P, T>,
}

impl<P: Clone + Eq + Hash + AsRef<[u8]>, T: Clone, H: Height> Node<P, T, H> {
    /// Get the version of this node.
    pub fn version(&self) -> &Version<P> {
        self.inner.version()
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
        self.inner.hash()
    }
}

impl<P: Clone + Eq + Hash + AsRef<[u8]>, T: Clone, H: Height> Node<P, T, S<H>>
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
        let children = unsafe { mem::transmute(children) };

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

impl<P: Clone + Eq + Hash + AsRef<[u8]>, T: Clone> Node<P, T, Z> {
    /// Construct a new leaf node.
    pub fn leaf(version: Version<P>, value: Message<T>) -> Self {
        Self {
            height: PhantomData,
            inner: untyped::Node::leaf(version, value),
        }
    }

    /// Get a reference to the leaf at this node.
    pub fn value(&self) -> &Message<T> {
        self.inner
            .as_leaf()
            .expect("typed leaf failed to be a leaf")
    }
}

impl<P: Clone + Eq + Hash + AsRef<[u8]>, T: Clone + Eq, H: Height> Eq for Node<P, T, H> {}

impl<P: Clone + Eq + Hash + AsRef<[u8]>, T: Clone + PartialEq, H: Height> PartialEq
    for Node<P, T, H>
{
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}
