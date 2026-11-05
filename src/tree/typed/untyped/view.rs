//! A stored node as one level of its own compressed path sees it.
//!
//! A node whose compressed path is `p` bytes long stands for `p` levels of
//! the tree above its body, one per path byte: listing its children yields
//! the same body one level down, under one byte less of path. A [`View`]
//! represents such a level without building a node for it, as a handle to
//! the stored node beside the number of top path bytes it hides. Listing a
//! view's children therefore allocates nothing, whoever else holds the node,
//! and the stored tree never holds a view: only a view reassembly keeps
//! becomes a node ([`View::into_node`]).
//!
//! A view is never stored either. The handles the streaming session keeps
//! are stored nodes alone, each tagged with the height it sits at, or paired
//! with the coordinate it sits at; a stored node knows the height of its own
//! top ([`Node::height`]), so the number of bytes hidden follows from the
//! height ([`View::at_height`]), and a coordinate's bytes can be checked
//! against the path bytes hidden ([`View::placed`]).

use std::borrow::Borrow;
use std::fmt;
use std::sync::{Arc, OnceLock};

use seq_macro::seq;

use super::{
    Body, Node, PATH_LEN, RangeOwned, Shape,
    memo_census::{self, Work},
    seeded_bounds,
};
use crate::{causally, tree::typed::Hash};

#[cfg(test)]
mod tests;

seq!(N in 0..=32 {
    /// How many top bytes of a stored node's compressed path a [`View`]
    /// hides: none, through all [`PATH_LEN`] bytes of a whole path.
    ///
    /// A fieldless enum rather than an integer, so a peel beyond a whole
    /// path cannot be constructed.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    #[repr(u8)]
    pub(super) enum Peel {
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
///
/// `N` is how the view holds the node: a handle of its own (the default),
/// which listing and reassembly consume, or a borrowed one, which answers
/// queries about a handle stored elsewhere without cloning it.
#[derive(Clone)]
pub struct View<N: Borrow<Node> = Node> {
    /// The stored node.
    node: N,
    /// How many of its top path bytes are hidden: at most its path's length.
    peeled: Peel,
}

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
impl<N: Borrow<Node>> fmt::Debug for View<N> {
    /// Format the peel, then the stored node.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("View")
            .field("peeled", &self.peeled.bytes())
            .field("node", self.node.borrow())
            .finish()
    }
}

/// Place a stored node at a height or a coordinate, and read the view.
impl<N: Borrow<Node>> View<N> {
    /// View `node` from the level at `height`: hiding as many top path
    /// bytes as its top lies above that height.
    ///
    /// Returns `None` unless `height` is one of the node's levels: at or
    /// below its top ([`Node::height`]), and at or above its body, below
    /// which it has no path to hide.
    ///
    /// The height alone decides the view, so it trusts the caller for the
    /// coordinate: a handle tagged with a height its traversal established
    /// uses this; one placed by a coordinate the program computed at runtime
    /// uses [`placed`](Self::placed), which checks it.
    pub fn at_height(node: N, height: usize) -> Option<Self> {
        let stored = node.borrow();
        let hidden = stored.height().checked_sub(height)?;
        if hidden > stored.inner.prefix.len() {
            return None;
        }
        let peeled = Peel::ALL[hidden];
        Some(View { node, peeled })
    }

    /// View `node` at the coordinate `at`, the path bytes from the root to
    /// the level it sits at, shallowest first, checking the coordinate
    /// against the node's own path.
    ///
    /// The coordinate's length decides the height, `PATH_LEN - at.len()`,
    /// and so the view ([`at_height`](Self::at_height)). Returns `None`
    /// unless that height is one of the node's levels and the bytes the
    /// view hides are exactly the coordinate's last bytes.
    ///
    /// # What the check proves
    ///
    /// Call the node's top height `T` and the coordinate's height `H`. The
    /// view hides the top `T − H` path bytes, which name the levels from
    /// `T` down to `H`: in a key beneath the node, the bytes at positions
    /// `PATH_LEN − T` up to `PATH_LEN − H`. The coordinate names the same
    /// positions with its last `T − H` bytes. Those are the only positions
    /// both name, and the check requires them equal, so a view this
    /// returns yields, for every key beneath it, the coordinate followed by
    /// the key's bytes below `H`, which is a key the stored node holds
    /// under its own path. Selecting other prefix bytes, and so other
    /// children or hashes, takes a coordinate that disagrees with the node
    /// at one of those positions, or a height outside the node's levels,
    /// and either returns `None`.
    ///
    /// The bytes above the node's top, positions before `PATH_LEN − T`,
    /// the node does not store, so no check can judge them; the view they
    /// select is the same whatever they are.
    pub fn placed(node: N, at: &[u8]) -> Option<Self> {
        let height = PATH_LEN.checked_sub(at.len())?;
        let view = Self::at_height(node, height)?;
        let hidden = view.peeled.bytes();
        let prefix = &view.node.borrow().inner.prefix;
        // The path is stored deepest byte first, so its top `hidden` bytes,
        // read in path order, run backwards from its end.
        let agrees = at[at.len() - hidden..]
            .iter()
            .eq(prefix[prefix.len() - hidden..].iter().rev());
        agrees.then_some(view)
    }

    /// The height of the level the view sits at: the stored node's top,
    /// less the bytes hidden.
    pub fn height(&self) -> usize {
        self.node.borrow().height() - self.peeled.bytes()
    }

    /// The visible part of the stored node's path over its body.
    fn shape(&self) -> Shape<'_> {
        let prefix = &self.node.borrow().inner.prefix;
        Shape {
            prefix: &prefix[..prefix.len() - self.peeled.bytes()],
            body: &self.node.borrow().inner.body,
        }
    }

    /// The stored node, whose body, and so whose bounds, leaf count,
    /// encoded version size and message, the view shares: those describe
    /// the leaves alone, so the stored node answers them for every view of
    /// it.
    pub(super) fn stored(&self) -> &Node {
        self.node.borrow()
    }

    /// Release the stored node, forgetting how much of its path the view
    /// hides: [`at_height`](Self::at_height) recovers the view from the
    /// height it sits at ([`height`](Self::height)).
    pub fn into_stored(self) -> N {
        self.node
    }

    /// Whether `self` and `other` are the same view of the same stored
    /// node: the same allocation, hiding the same number of bytes.
    #[cfg(test)]
    pub fn same<M: Borrow<Node>>(&self, other: &View<M>) -> bool {
        self.peeled == other.peeled && self.stored().ptr_eq(other.stored())
    }

    /// The subtree's hash, as observed from the top of the visible path.
    ///
    /// A whole node reads its memo. A peeled view computes the hash on each
    /// call, one preimage over the visible path and the body's children's
    /// memoized hashes: the stored memo commits the whole path, and a view
    /// keeps no memo of its own.
    pub fn hash(&self) -> Hash {
        match self.peeled {
            Peel::P0 => self.stored().hash(),
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

    /// The children [`children`](View::children) lists for this view, read
    /// in place, in radix order: a visible path's one peeled view, a bare
    /// branch's fan, and none for a bare leaf.
    pub(super) fn listed(&self) -> impl Iterator<Item = (u8, Listed<'_>)> {
        let shape = self.shape();
        let peeled = shape.prefix.last().map(|&top| {
            (
                top,
                Listed::Peeled {
                    node: self.stored(),
                    peeled: self.peeled,
                },
            )
        });
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
}

/// List, build, and walk a view that holds its own handle.
impl View {
    /// View a stored node from below its whole compressed path: its body
    /// alone, at the height of the body itself.
    pub(super) fn body(node: Node) -> View {
        let peeled = Peel::ALL[node.inner.prefix.len()];
        View { node, peeled }
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
    /// The one child of a view with a visible path: the stored node of the
    /// view it is listed from, which hides `peeled` bytes, with one more
    /// byte hidden.
    Peeled {
        /// The parent view's stored node.
        node: &'a Node,
        /// The bytes the parent view hides.
        peeled: Peel,
    },
}

impl<'a> Listed<'a> {
    /// The stored node whose memos answer for this child's bounds: the
    /// child itself, or for a peeled one, the stored node it shares its
    /// body with.
    pub(super) fn memos(self) -> &'a Node {
        match self {
            Listed::Stored(node) | Listed::Peeled { node, .. } => node,
        }
    }

    /// Whether `candidate` is this child, as listing yields it: a whole
    /// view of the stored child, or the parent's stored node with one more
    /// byte hidden.
    pub(super) fn is(self, candidate: &View) -> bool {
        match self {
            Listed::Stored(node) => candidate.peeled == Peel::P0 && node.ptr_eq(&candidate.node),
            Listed::Peeled { node, peeled } => {
                peeled.deeper() == Some(candidate.peeled) && node.ptr_eq(&candidate.node)
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
