//! A stored node viewed from one level of its compressed path, at a known
//! height ([`untyped::View`]).

use std::{fmt, marker::PhantomData};

use before::Span;

use crate::{Version, message::Message};

use super::height::{Height, S, Z};
use super::node::Node;
use super::prefix::{ErasedPrefix, Prefix};
use super::untyped;

/// A stored node viewed at height `H`, possibly from inside its compressed
/// path.
///
/// The streaming session's in-memory handles: listing one's children
/// yields views, so a compressed path is walked one level at a time without
/// building a node per level. Converting from a [`Node`] views it whole;
/// [`into_node`](Self::into_node) builds the node a view represents.
///
/// A view holds the stored node alone, the size of a [`Node`]. The number
/// of path bytes it hides follows from its height: the stored node knows
/// the height of its own top ([`untyped::Node::height`]), and the view
/// hides one byte per level between that top and `H`
/// ([`untyped::View::at_height`]).
///
/// # The height invariant
///
/// `H` lies between the stored node's body and its top. Every constructor
/// establishes it: a whole node sits at its own top, a listed child one
/// level below its parent, a reassembled node at its own top, a walked
/// leaf at its body, and a handle re-tagged at a runtime coordinate sits
/// where the walk's bookkeeping placed it, which debug builds check against
/// the node's path ([`placed`](Self::placed)).
pub struct View<H: Height> {
    /// The height at which the view sits.
    height: PhantomData<fn() -> H>,
    /// The stored node.
    node: untyped::Node,
}

/// A [`View`] with its height forgotten: the stored node alone.
///
/// Which level of the node it views is no longer known from it, so it
/// answers nothing that depends on the level by itself. The holder keeps
/// the coordinate it sits at, the prefix from the root, and passes it to
/// every such query ([`at`](Self::at)) and to the re-tag that restores the
/// view ([`View::placed`]); in debug builds, both check the coordinate
/// against the node's own path. The version bounds and the leaf count
/// describe the leaves alone, and the stored node answers them directly.
///
/// A distinct type rather than the stored node itself, whose own queries
/// answer for the whole node: its hash or child count read where a
/// peeled view's is meant would be another level's.
#[derive(Clone)]
pub struct ErasedView {
    /// The stored node.
    node: untyped::Node,
}

/// A view and an erased one each cost one stored node's handle: the height
/// is a type, and the bytes hidden are derived from it.
const _: () = assert!(
    size_of::<View<Z>>() == size_of::<untyped::Node>()
        && size_of::<ErasedView>() == size_of::<untyped::Node>()
);

/// Display the stored node.
impl fmt::Debug for ErasedView {
    /// Delegate to the stored node's debug form.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ErasedView").field(&self.node).finish()
    }
}

impl ErasedView {
    /// The untyped view at the coordinate `at`, borrowing the stored node:
    /// the view at `at`'s height ([`untyped::View::at_height`]).
    ///
    /// Debug builds first check `at` against the node's own path bytes
    /// ([`untyped::View::placed`]); release builds trust it, as
    /// [`View::placed`] does.
    ///
    /// # Panics
    ///
    /// In debug builds, if the check fails: a programmer error, as at
    /// [`View::placed`]. In every build, if `at`'s height lies outside the
    /// stored node's levels, where no view of it sits.
    pub(crate) fn at(&self, at: &ErasedPrefix) -> untyped::View<&untyped::Node> {
        debug_assert!(
            untyped::View::placed(&self.node, at.as_bytes()).is_some(),
            "a handle read at {} must sit on its own path there: {:?}",
            hex::encode(at.as_bytes()),
            self.node,
        );
        untyped::View::at_height(&self.node, at.height())
            .expect("a handle's coordinate sits between its stored node's body and top")
    }

    /// The subtree's version bounds as one causal span.
    pub fn span(&self) -> Span<'_> {
        self.node.span()
    }

    /// The number of leaves in the subtree.
    pub fn len(&self) -> usize {
        self.node.len()
    }
}

/// Share the stored node, keeping the same height.
impl<H: Height> Clone for View<H> {
    /// Clone the stored node's handle.
    fn clone(&self) -> Self {
        Self {
            height: PhantomData,
            node: self.node.clone(),
        }
    }
}

/// Display the untyped view at this height.
impl<H: Height> fmt::Debug for View<H> {
    /// Delegate to the untyped view's debug form.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.untyped().fmt(f)
    }
}

/// View a whole node at its own height.
impl<H: Height> From<Node<H>> for View<H> {
    /// Hide none of the node's path.
    fn from(node: Node<H>) -> Self {
        Self::from_view(untyped::View::from(node.into_untyped()))
    }
}

impl<H: Height> View<H> {
    /// Tag an untyped view that sits at height `H`, keeping its stored node.
    ///
    /// The typed operations derive the views they tag from typed inputs,
    /// so the height agrees by construction; debug builds check it.
    fn from_view(view: untyped::View) -> Self {
        debug_assert_eq!(
            view.height(),
            H::HEIGHT,
            "a typed operation yields views at the height its types name",
        );
        Self {
            height: PhantomData,
            node: view.into_stored(),
        }
    }

    /// Tag the erased view's stored node at the coordinate `at`, checking
    /// the coordinate against the node's own path bytes in debug builds.
    ///
    /// This is how a height-erased handle regains its height: the streaming
    /// session keeps erased handles beside the prefixes they sit at
    /// ([`Backend::assume`](crate::tree::mirror::streaming::Backend::assume)).
    /// The height decides how many path bytes the view hides, so a wrong
    /// one would select other prefix bytes, and with them other children
    /// and hashes; the check rejects every coordinate whose height lies
    /// outside the node's levels or whose bytes disagree with the path
    /// bytes the view hides ([`untyped::View::placed`] gives the argument).
    ///
    /// A coordinate travels with its handle through the session's own
    /// bookkeeping and never comes from a peer, so a coordinate that fails
    /// the check is a programmer error. Release builds trust the walk's
    /// coordinates and tag the node unchecked.
    ///
    /// # Panics
    ///
    /// In debug builds, if the check fails. A release build tags a node at
    /// any height, and a view at a height outside the node's levels panics
    /// at its first read, where no view of the node sits.
    pub(crate) fn placed(at: &Prefix<H>, erased: ErasedView) -> Self {
        let ErasedView { node } = erased;
        debug_assert!(
            untyped::View::placed(&node, at.as_bytes()).is_some(),
            "a handle re-tagged at {} must sit on its own path there: {node:?}",
            hex::encode(at.as_bytes()),
        );
        Self {
            height: PhantomData,
            node,
        }
    }

    /// Forget the height tag, keeping the stored node; [`placed`](Self::placed)
    /// restores the view from the coordinate it sits at.
    pub(crate) fn erase(self) -> ErasedView {
        ErasedView { node: self.node }
    }

    /// The untyped view at this height, borrowing the stored node.
    fn untyped(&self) -> untyped::View<&untyped::Node> {
        untyped::View::at_height(&self.node, H::HEIGHT)
            .expect("a typed view sits between its stored node's body and top")
    }

    /// The untyped view at this height, holding the stored node.
    fn into_untyped(self) -> untyped::View {
        untyped::View::at_height(self.node, H::HEIGHT)
            .expect("a typed view sits between its stored node's body and top")
    }

    /// Build the node this view represents ([`untyped::View::into_node`]).
    pub(crate) fn into_node(self) -> Node<H> {
        Node::from_untyped(self.into_untyped().into_node())
    }

    /// The subtree's version bounds as one causal span.
    pub fn span(&self) -> Span<'_> {
        self.node.span()
    }

    /// The number of leaves in the subtree.
    pub fn len(&self) -> usize {
        self.node.len()
    }

    /// The number of children this view's children have, a child at the
    /// leaves counting one ([`untyped::View::grandchild_count`]).
    pub fn grandchild_count(&self) -> usize {
        self.untyped().grandchild_count()
    }

    /// The largest encoded version bound stored anywhere in the subtree.
    pub fn version_bytes(&self) -> usize {
        self.node.version_bytes()
    }

    /// The subtree's digest under `key` ([`untyped::View::hash`]).
    #[cfg(test)]
    pub fn hash(&self, key: &super::HashKey) -> super::Hash {
        self.untyped().hash(key)
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
        self.into_untyped()
            .leaves(prefix.as_bytes())
            .map(|(key, leaf)| (Prefix::from(key), View::from_view(leaf.into_view())))
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
        self.into_untyped()
            .children()
            .unwrap_or_else(|_| unreachable!("a nonzero-height view has a child"))
            .map(|(radix, child)| (radix, View::from_view(child)))
    }

    /// Assemble a subtree from its children, reusing `original`, the view
    /// they were listed from, when they are exactly its children
    /// ([`untyped::View::rebuild`]).
    pub(crate) fn rebuild(original: Option<Self>, children: Vec<(u8, View<H>)>) -> Option<Self> {
        let children = children
            .into_iter()
            .map(|(radix, child)| (radix, child.into_untyped()))
            .collect();
        untyped::View::rebuild(original.map(View::into_untyped), children).map(View::from_view)
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
        self.node
            .as_leaf()
            .expect("a height-zero view sees a leaf's body")
    }
}
