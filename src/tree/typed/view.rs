//! A stored node viewed from one level of its compressed path, at a known
//! height ([`untyped::View`]).

use std::{fmt, marker::PhantomData};

use before::Span;

use crate::{Version, message::Message};

use super::height::{Height, S, Z};
use super::node::Node;
use super::prefix::Prefix;
use super::untyped;

/// A stored node viewed at height `H`, possibly from inside its compressed
/// path.
///
/// The streaming session's in-memory handles: listing one's children
/// yields views, so a compressed path is walked one level at a time without
/// building a node per level. Converting from a [`Node`] views it whole;
/// [`into_node`](Self::into_node) builds the node a view represents.
pub struct View<H: Height> {
    /// The height at which the view sits.
    height: PhantomData<fn() -> H>,
    /// The untyped view.
    inner: untyped::View,
}

/// Share the stored node, keeping the same height and peel.
impl<H: Height> Clone for View<H> {
    /// Clone the view's handle.
    fn clone(&self) -> Self {
        Self::from_untyped(self.inner.clone())
    }
}

/// Display the untyped view.
impl<H: Height> fmt::Debug for View<H> {
    /// Delegate to the untyped view's debug form.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

/// View a whole node at its own height.
impl<H: Height> From<Node<H>> for View<H> {
    /// Hide none of the node's path.
    fn from(node: Node<H>) -> Self {
        Self::from_untyped(untyped::View::from(node.into_untyped()))
    }
}

impl<H: Height> View<H> {
    /// Tag an untyped view at height `H`: the caller asserts the height.
    pub(crate) fn from_untyped(inner: untyped::View) -> Self {
        Self {
            height: PhantomData,
            inner,
        }
    }

    /// Forget the height tag; the inverse of
    /// [`from_untyped`](Self::from_untyped).
    pub(crate) fn into_untyped(self) -> untyped::View {
        self.inner
    }

    /// Build the node this view represents ([`untyped::View::into_node`]).
    pub(crate) fn into_node(self) -> Node<H> {
        Node::from_untyped(self.inner.into_node())
    }

    /// The subtree's version bounds as one causal span.
    pub fn span(&self) -> Span<'_> {
        self.inner.span()
    }

    /// The number of leaves in the subtree.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// The number of children this view's children have, a child at the
    /// leaves counting one ([`untyped::View::grandchild_count`]).
    pub fn grandchild_count(&self) -> usize {
        self.inner.grandchild_count()
    }

    /// The largest encoded version bound stored anywhere in the subtree.
    pub fn version_bytes(&self) -> usize {
        self.inner.version_bytes()
    }

    /// The subtree's hash ([`untyped::View::hash`]).
    #[cfg(test)]
    pub fn hash(&self) -> super::Hash {
        self.inner.hash()
    }

    /// Walk every leaf beneath this view at `prefix`, in ascending path
    /// order, each as a view at height zero.
    ///
    /// The walk starts at the stored node and skips compressed spans
    /// ([`untyped::View::leaves`]); descent state is bounded by the tree's
    /// depth. Each leaf is the stored leaf with its whole path hidden
    /// ([`untyped::Leaf::into_view`]), so the walk allocates nothing per
    /// leaf: one shared handle each.
    pub(crate) fn leaves(
        self,
        prefix: &Prefix<H>,
    ) -> impl Iterator<Item = (Prefix<Z>, View<Z>)> + Send + use<H> {
        self.inner
            .leaves(prefix.as_bytes())
            .map(|(key, leaf)| (Prefix::from(key), View::from_untyped(leaf.into_view())))
    }
}

/// List and reassemble views one level above height `H`.
impl<H: Height> View<S<H>>
where
    S<H>: Height,
{
    /// List the immediate children in radix order, allocating nothing
    /// ([`untyped::View::children`]).
    pub(crate) fn children(self) -> impl Iterator<Item = (u8, View<H>)> + Send + use<H> {
        self.inner
            .children()
            .unwrap_or_else(|_| unreachable!("a nonzero-height view has a child"))
            .map(|(radix, child)| (radix, View::from_untyped(child)))
    }

    /// Assemble a subtree from its children, reusing `original`, the view
    /// they were listed from, when they are exactly its children
    /// ([`untyped::View::rebuild`]).
    pub(crate) fn rebuild(original: Option<Self>, children: Vec<(u8, View<H>)>) -> Option<Self> {
        let children = children
            .into_iter()
            .map(|(radix, child)| (radix, child.into_untyped()))
            .collect();
        untyped::View::rebuild(original.map(View::into_untyped), children).map(View::from_untyped)
    }
}

/// Construct and read height-zero leaves.
impl View<Z> {
    /// Construct a leaf, viewed whole.
    pub fn leaf(version: Version, message: Message) -> Self {
        Self::from(Node::leaf(version, message))
    }

    /// The message at this leaf.
    pub fn message(&self) -> &Message {
        self.inner
            .as_leaf()
            .expect("a height-zero view sees a leaf's body")
    }
}
