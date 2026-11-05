//! A stored node as one level of its own compressed path sees it.
//!
//! A node whose compressed path is `p` bytes long stands for `p` levels of
//! the tree above its body, one per path byte: listing its children yields
//! the same body one level down, under one byte less of path. A [`View`]
//! represents such a level without building a node for it, as a shared
//! handle to the stored node beside the number of top path bytes it hides.
//! Listing a view's children therefore allocates nothing, whoever else
//! holds the node, and the stored tree never holds a view: only a view
//! reassembly keeps becomes a node ([`View::into_node`]).

use std::fmt;
use std::sync::{Arc, OnceLock};

use before::Span;
use seq_macro::seq;

use super::{
    Body, Node, PATH_LEN, RangeOwned, Shape,
    memo_census::{self, Work},
    seeded_bounds,
};
use crate::{causally, message::Message, tree::typed::Hash};

#[cfg(test)]
mod tests;

seq!(N in 0..=32 {
    /// How many top bytes of a stored node's compressed path a [`View`]
    /// hides: none, through all [`PATH_LEN`] bytes of a whole path.
    ///
    /// A fieldless enum rather than a `u8`, so the byte it occupies has
    /// spare values: an enum with a view in one variant keeps its
    /// discriminant there, and stays the size of the view. The streaming
    /// walk keeps its handles in such enums.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    #[repr(u8)]
    enum Peel {
        #(
            #[doc = concat!("Hides ", N, " path bytes.")]
            P~N,
        )*
    }

    impl Peel {
        /// Every peel, in order of the bytes it hides.
        const ALL: [Peel; PATH_LEN + 1] = [#(Peel::P~N,)*];
    }
});

impl Peel {
    /// The number of path bytes hidden.
    fn bytes(self) -> usize {
        self as usize
    }

    /// The peel hiding one byte more, or `None` past a whole path.
    fn deeper(self) -> Option<Peel> {
        Peel::ALL.get(self.bytes() + 1).copied()
    }
}

/// A shared stored node seen from inside its compressed path: the node with
/// its top `peeled` path bytes hidden.
///
/// It is the subtree a copy of the node with those bytes popped would hold,
/// and answers every query as that copy would, from the shared node: the
/// visible path decides the children listed, their count and radices, and
/// the hash; the body decides everything else.
#[derive(Clone)]
pub struct View {
    /// The stored node.
    node: Node,
    /// How many of its top path bytes are hidden: at most its path's length.
    peeled: Peel,
}

// The peel's spare values hold the discriminants of enums around a view:
// two levels of `Option` cost no space, where a `u8` peel leaves room for
// only the one the handle's non-null pointer provides.
const _: () = assert!(size_of::<Option<Option<View>>>() == size_of::<View>());

/// View a whole stored node, hiding none of its path.
impl From<Node> for View {
    /// Wrap the handle at peel zero.
    fn from(node: Node) -> Self {
        View {
            node,
            peeled: Peel::P0,
        }
    }
}

/// Display the stored node and how much of its path is hidden.
impl fmt::Debug for View {
    /// Format the peel, then the stored node.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("View")
            .field("peeled", &self.peeled.bytes())
            .field("node", &self.node)
            .finish()
    }
}

impl View {
    /// The visible part of the stored node's path over its body.
    fn shape(&self) -> Shape<'_> {
        let prefix = &self.node.inner.prefix;
        Shape {
            prefix: &prefix[..prefix.len() - self.peeled.bytes()],
            body: &self.node.inner.body,
        }
    }

    /// The stored node, whose body, and so whose bounds, leaf count and
    /// encoded version size, the view shares.
    pub(super) fn stored(&self) -> &Node {
        &self.node
    }

    /// Whether `self` and `other` are the same view of the same stored
    /// node: the same allocation, hiding the same number of bytes.
    #[cfg(test)]
    pub fn same(&self, other: &View) -> bool {
        self.peeled == other.peeled && self.node.ptr_eq(&other.node)
    }

    /// The subtree's version bounds ([`Node::span`]): the body's.
    pub fn span(&self) -> Span<'_> {
        self.node.span()
    }

    /// The number of leaves in the subtree.
    pub fn len(&self) -> usize {
        self.node.len()
    }

    /// The largest encoded version bound in the subtree
    /// ([`Node::version_bytes`]).
    pub fn version_bytes(&self) -> usize {
        self.node.version_bytes()
    }

    /// The leaf's message, if the body is a leaf, whatever path is visible
    /// above it.
    pub fn as_leaf(&self) -> Option<&Message> {
        self.node.as_leaf()
    }

    /// The subtree's hash, as observed from the top of the visible path.
    ///
    /// A whole node reads its memo. A peeled view computes the hash on each
    /// call, one preimage over the visible path and the body's children's
    /// memoized hashes: the stored memo commits the whole path, and a view
    /// keeps no memo of its own.
    pub fn hash(&self) -> Hash {
        match self.peeled {
            Peel::P0 => self.node.hash(),
            _ => self.shape().hash(),
        }
    }

    /// The number of children [`children`](Self::children) lists: a bare
    /// branch's fan, one for a visible path, none for a bare leaf.
    pub fn child_count(&self) -> usize {
        self.shape().child_count()
    }

    /// The radices [`children`](Self::children) lists, ascending, read in
    /// place.
    pub fn child_radices(&self) -> impl Iterator<Item = u8> + '_ {
        self.shape().child_radices()
    }

    /// The number of children the listed children have: over the
    /// children [`children`](Self::children) lists, each one's own child
    /// count, a child at the leaves counting one. A bare leaf, which has no
    /// children, has none.
    ///
    /// Exact, and O(1): a view with two or more visible path bytes lists
    /// one child that itself lists one, so it has one; a view with one
    /// visible byte over a branch has that branch's fan as its
    /// grandchildren, and over a leaf has the leaf alone; a bare branch
    /// stores its count.
    pub fn grandchild_count(&self) -> usize {
        self.shape().grandchild_count()
    }

    /// List the immediate children, or return the view if it is a bare
    /// leaf.
    ///
    /// A visible path yields one child, this node with one more byte
    /// hidden; a bare branch yields its stored children, whole. Either way
    /// listing clones handles and allocates nothing.
    pub fn children(self) -> Result<ViewChildren, View> {
        if let Some(&radix) = self.shape().prefix.last() {
            let peeled = self
                .peeled
                .deeper()
                .expect("a visible path byte lies within the path, so hiding it stays in range");
            return Ok(ViewChildren::Peeled(Some((
                radix,
                View {
                    node: self.node,
                    peeled,
                },
            ))));
        }
        match &self.node.inner.body {
            Body::Leaf { .. } => Err(self),
            Body::Branch { .. } => Ok(ViewChildren::Stored {
                node: self.node,
                next: Some(0),
            }),
        }
    }

    /// Build the node this view represents: the stored node itself when
    /// nothing is hidden, and otherwise a node with the visible path over
    /// the same body.
    ///
    /// A peeled view copies the stored node unless this handle is its only
    /// one, in which case the node is shortened in place: one node
    /// allocation, and for a branch its fan's heap slots, sharing the
    /// body's leaf or child handles. The copy keeps the body's memos, and
    /// computes its hash on first read.
    pub fn into_node(self) -> Node {
        let View { mut node, peeled } = self;
        if peeled != Peel::P0 {
            let inner = Arc::make_mut(&mut node.inner);
            let visible = inner.prefix.len() - peeled.bytes();
            inner.prefix.truncate(visible);
            inner.hash = OnceLock::new();
        }
        node
    }
}

/// Reassemble views, reusing the view they were listed from.
impl View {
    /// Assemble children into a subtree as [`Node::branch`] does, reusing
    /// `original`, the view the children were listed from before any of
    /// them changed.
    ///
    /// When the children are exactly those `original` lists, at the same
    /// radices, this returns `original` itself, with its memos, so
    /// reassembling an unchanged scope allocates nothing and computes no
    /// memo. A child counts as unchanged as the same view of the same
    /// stored node, hiding the same bytes, whether stored in a fan or
    /// peeled from a compressed path ([`Listed::is`]). Otherwise this builds the node `branch` builds
    /// from the nodes the children represent. When that is a bare branch,
    /// it seeds the new node's bounds from `original`'s wherever that is
    /// exact ([`seeded_bounds`]), before it builds any child's node, since
    /// seeding tells unchanged children by their views.
    ///
    /// Building a peeled child's node copies the stored node when it is
    /// shared ([`View::into_node`]). A changed child is built fresh, and so
    /// unshared; an unchanged one is copied only where it joins a new bare
    /// branch beside children at other radices, the one place reassembly
    /// keeps a peeled level of the stored tree.
    pub fn rebuild(original: Option<View>, children: Vec<(u8, View)>) -> Option<View> {
        let Some(original) = original else {
            return View::branch(children);
        };
        // An empty group deletes the scope, whatever stood there.
        if children.is_empty() {
            return None;
        }
        if original.lists_exactly(&children) {
            memo_census::record(Work::Reused);
            return Some(original);
        }
        if children.len() < 2 {
            // One child: `branch` extends its path, which keeps its memos.
            return View::branch(children);
        }
        let bounds = seeded_bounds(&original, &children);
        let fan = children
            .into_iter()
            .map(|(radix, child)| (radix, child.into_node()))
            .collect();
        Some(View::from(Node::bare_branch(fan, bounds)))
    }

    /// The subtree [`Node::branch`] builds from the nodes `children`
    /// represent.
    fn branch(children: Vec<(u8, View)>) -> Option<View> {
        let fan = children
            .into_iter()
            .map(|(radix, child)| (radix, child.into_node()))
            .collect();
        Node::branch(fan).map(View::from)
    }

    /// Whether `children` are exactly the children
    /// [`children`](Self::children) lists for this view: the same radices,
    /// each child the one listed there ([`Listed::is`]).
    fn lists_exactly(&self, children: &[(u8, View)]) -> bool {
        let mut listed = self.listed();
        children.iter().all(|(radix, child)| {
            listed
                .next()
                .is_some_and(|(at, own)| at == *radix && own.is(child))
        }) && listed.next().is_none()
    }

    /// The children [`children`](Self::children) lists for this view, read
    /// in place, in radix order: a visible path's one peeled view, a bare
    /// branch's fan, and none for a bare leaf.
    pub(super) fn listed(&self) -> impl Iterator<Item = (u8, Listed<'_>)> {
        let shape = self.shape();
        let peeled = shape.prefix.last().map(|&top| (top, Listed::Peeled(self)));
        let fan = match (peeled, shape.body) {
            (None, Body::Branch { children, .. }) => Some(
                children
                    .iter()
                    .map(|(radix, child)| (radix, Listed::Stored(child))),
            ),
            _ => None,
        };
        peeled.into_iter().chain(fan.into_iter().flatten())
    }

    /// Walk every leaf beneath this view, in ascending path order, as the
    /// stored tree holds it.
    ///
    /// `path` holds the bytes above the view, shallowest first; its length
    /// plus the view's height is the whole key. The walk starts at the
    /// stored node, above the hidden bytes, which end `path`, so it yields
    /// the same keys without building the view's node.
    pub(crate) fn leaves(self, path: &[u8]) -> RangeOwned<causally::Neutral> {
        let above = path.len() - self.peeled.bytes();
        RangeOwned::within(Some(self.node), &path[..above], causally::all())
    }
}

/// One child a view lists, as [`View::children`] would yield it, read in
/// place.
#[derive(Clone, Copy)]
pub(super) enum Listed<'a> {
    /// A bare branch's child, stored in its fan.
    Stored(&'a Node),
    /// The one child of a view with a visible path: the view it is listed
    /// from, with one more byte hidden.
    Peeled(&'a View),
}

impl<'a> Listed<'a> {
    /// The stored node whose memos answer for this child's bounds: the
    /// child itself, or for a peeled one, the stored node it shares its
    /// body with.
    pub(super) fn memos(self) -> &'a Node {
        match self {
            Listed::Stored(node) => node,
            Listed::Peeled(view) => &view.node,
        }
    }

    /// Whether `candidate` is this child, as listing yields it: a whole
    /// view of the stored child, or the parent's stored node with one more
    /// byte hidden.
    pub(super) fn is(self, candidate: &View) -> bool {
        match self {
            Listed::Stored(node) => candidate.peeled == Peel::P0 && node.ptr_eq(&candidate.node),
            Listed::Peeled(parent) => {
                parent.peeled.deeper() == Some(candidate.peeled)
                    && parent.node.ptr_eq(&candidate.node)
            }
        }
    }
}

/// An owned, ascending walk over one view's immediate children
/// ([`View::children`]).
pub enum ViewChildren {
    /// The one child of a view with a visible path, not yet yielded.
    Peeled(Option<(u8, View)>),
    /// A bare branch's stored children, from the next radix on.
    Stored {
        /// The branch, which keeps its fan alive.
        node: Node,
        /// The radix to resume from, or `None` once the fan is done.
        next: Option<u8>,
    },
}

/// Yield each child as a view, cloning one handle per child.
impl Iterator for ViewChildren {
    /// A radix and the child there.
    type Item = (u8, View);

    /// Yield the next child, cloning its handle.
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Peeled(child) => child.take(),
            Self::Stored { node, next } => {
                let start = (*next)?;
                let Body::Branch { children, .. } = &node.inner.body else {
                    unreachable!("a stored listing retains only a bare branch")
                };
                let Some((radix, child)) = children.successor(start) else {
                    *next = None;
                    return None;
                };
                *next = radix.checked_add(1);
                Some((radix, View::from(child.clone())))
            }
        }
    }
}
