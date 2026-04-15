use std::{hash::Hash, marker::PhantomData, mem};

use imbl::OrdMap;

use bytes::Bytes;

use super::height::{self, Height, S, Z};
use super::untyped;

pub use untyped::Leaf;

/// The typed node with a height of 32; the root of the tree.
pub type Root<P> = Node<P, height::Root>;

/// A typed node which enforces the structural validity of the constructed tree
/// at compile-time.
#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct Node<P: Clone + Eq + Hash, H: Height> {
    height: PhantomData<H>,
    inner: untyped::Node<P>,
}

impl<P: Clone + Eq + Hash, H: Height> Node<P, H> {
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

    /// Merge an iterator of nodes of the same height, taking the union of their
    /// leaves. Returns `None` if the iterator is empty.
    ///
    /// If a leaf appears simultaneously in multiple nodes, the last one wins.
    pub fn unions<I>(nodes: I) -> Option<Self>
    where
        I: IntoIterator<Item = Self>,
    {
        Some(Self {
            height: PhantomData,
            inner: untyped::Node::unions(nodes.into_iter().map(|t| t.inner))?,
        })
    }
}

impl<P: Clone + Eq + Hash, H: Height> Node<P, S<H>>
where
    S<H>: Height,
{
    /// Construct a new branch node from a list of children (inverse to
    /// [`Node::into_children`]).
    pub fn branch<I>(i: I) -> Option<Self>
    where
        I: IntoIterator<Item = (u8, Node<P, H>)>,
    {
        Some(Node {
            height: PhantomData,
            inner: untyped::Node::branch(i.into_iter().map(|(i, t)| (i, t.inner)))?,
        })
    }

    /// Convert a node into a map from child index to child node (inverse to
    /// [`Node::branch`]).
    pub fn into_children(self) -> OrdMap<u8, Node<P, H>> {
        // Transmute the map of children into typed nodes with the correct
        // height, to recursively enforce type-safe height.
        //
        // SAFETY: TypedNode is #[repr(transparent)] and `OrdMap` treats values
        // in the map parametrically (i.e. no use of `TypeId`, etc.)
        unsafe { mem::transmute(self.inner.into_children()) }
    }
}

impl<P: Clone + Eq + Hash> Node<P, Z> {
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
