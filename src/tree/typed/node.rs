use std::{
    fmt::{self, Debug},
    iter::Map,
    marker::PhantomData,
};

use before::Span;

use crate::{Version, causally, message::Message};

use super::hash::{Hash, HashKey, PATH_LEN};
use super::height::{self, Height, S, Z};
use super::prefix::Prefix;
use super::untyped;
use untyped::fan::{self, Fan};

/// The typed node with a height of 32; the root of the tree.
pub type Root = Node<height::Root>;

/// The radix-indexed children of a branch one level above height `H`: a typed
/// shell over the untyped radix fan, so inserts and removals stay
/// height-correct at compile time.
pub struct Children<H: Height> {
    /// The height shared by every child.
    height: PhantomData<fn() -> H>,
    /// Child handles sorted by radix.
    inner: Fan,
}

/// Builds one bulk subtree at height `H` from its leaves in ascending path
/// order, holding only its open right spine ([`untyped::SortedLeaves`]).
pub(crate) struct SortedRun<H: Height> {
    /// The subtree's prefix, which every leaf's path extends.
    prefix: Prefix<H>,
    /// The spine built so far.
    leaves: untyped::SortedLeaves,
}

impl<H: Height> SortedRun<H> {
    /// Begin the subtree at `prefix`.
    pub(crate) fn new(prefix: Prefix<H>) -> Self {
        Self {
            prefix,
            leaves: untyped::SortedLeaves::new(prefix.as_bytes().len()),
        }
    }

    /// The subtree's prefix.
    pub(crate) fn prefix(&self) -> Prefix<H> {
        self.prefix
    }

    /// Append the next leaf, at `path`.
    ///
    /// # Panics
    ///
    /// Panics if `path` does not ascend strictly from the previous leaf's, and
    /// in debug builds if it falls outside the subtree's prefix. Session
    /// decoding establishes both.
    pub(crate) fn push(&mut self, path: Prefix<Z>, leaf: Node<Z>) {
        debug_assert!(
            path.as_bytes().starts_with(self.prefix.as_bytes()),
            "every leaf in a run falls under the run's prefix",
        );
        self.leaves.push(path.into(), leaf.into_untyped());
    }

    /// Finish the maximally compressed subtree, with its prefix.
    ///
    /// # Panics
    ///
    /// Panics if no leaf was pushed: a run holds at least one leaf.
    pub(crate) fn finish(self) -> (Prefix<H>, Node<H>) {
        let node = self.leaves.finish().expect("a leaf run is non-empty");
        (self.prefix, Node::from_untyped(node))
    }
}

/// Construct an empty child list.
impl<H: Height> Default for Children<H> {
    /// Start with no occupied radices.
    fn default() -> Self {
        Self {
            height: PhantomData,
            inner: Fan::new(),
        }
    }
}

/// Update children while preserving their common height.
impl<H: Height> Children<H> {
    /// Wrap a fan whose children have height `H`.
    fn from_fan(inner: Fan) -> Self {
        Self {
            height: PhantomData,
            inner,
        }
    }

    /// Remove the height marker when passing children to untyped storage.
    fn into_fan(self) -> Fan {
        self.inner
    }

    /// An empty list with room for `capacity` children, at most 256.
    pub fn with_capacity(capacity: usize) -> Self {
        Self::from_fan(Fan::with_capacity(capacity))
    }

    /// Append a child whose radix is greater than every existing radix.
    pub fn push(&mut self, radix: u8, child: Node<H>) {
        self.inner.push(radix, child.into_untyped());
    }

    /// The number of children present (0..=256).
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Remove and return the child at `radix`, if any.
    pub fn remove(&mut self, radix: u8) -> Option<Node<H>> {
        self.inner.remove(radix).map(Node::from_untyped)
    }
}

/// Collect children, preserving the fan's sorted, unique radix invariant.
impl<H: Height> FromIterator<(u8, Node<H>)> for Children<H> {
    /// Collect ascending input without sorting; otherwise sort and deduplicate.
    fn from_iter<I: IntoIterator<Item = (u8, Node<H>)>>(iter: I) -> Self {
        Self::from_fan(
            iter.into_iter()
                .map(|(radix, child)| (radix, child.into_untyped()))
                .collect(),
        )
    }
}

/// Restore the height marker to a child from the untyped iterator.
fn typed_child<H: Height>((radix, inner): (u8, untyped::Node)) -> (u8, Node<H>) {
    (radix, Node::from_untyped(inner))
}

/// Consume children in ascending radix order without cloning their handles.
impl<H: Height> IntoIterator for Children<H> {
    /// A radix and the child at that position.
    type Item = (u8, Node<H>);
    /// The fan's owned iterator with typed child handles restored.
    type IntoIter = Map<fan::IntoIter, fn((u8, untyped::Node)) -> (u8, Node<H>)>;

    /// Transfer ownership of each child to the caller.
    fn into_iter(self) -> Self::IntoIter {
        self.inner
            .into_iter()
            .map(typed_child::<H> as fn((u8, untyped::Node)) -> (u8, Node<H>))
    }
}

/// A storage node viewed at height `H`.
///
/// Constructors and traversals preserve this height. The function marker avoids
/// recursive auto-trait bounds on `H`; see [`S`].
#[repr(transparent)]
pub struct Node<H: Height> {
    /// The height at which this storage node is being viewed.
    height: PhantomData<fn() -> H>,
    /// Shared storage, including any compressed path below this height.
    inner: untyped::Node,
}

/// Share storage while retaining the same height.
impl<H: Height> Clone for Node<H> {
    /// Clone the node handle and its phantom height marker.
    fn clone(&self) -> Self {
        Self {
            height: self.height,
            inner: self.inner.clone(),
        }
    }
}

/// Display the underlying node's content.
impl<H> Debug for Node<H>
where
    H: Height,
{
    /// Delegate to the storage node's debug view.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

/// Read and construct nodes at a known height.
impl<H: Height> Node<H> {
    /// Whether both handles refer to the same node allocation, without hashing.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        self.inner.ptr_eq(&other.inner)
    }

    /// Tag an untyped node at height `H`: the caller asserts the height, the
    /// node's own top ([`untyped::Node::height`]).
    ///
    /// Internal tree code tags nodes whose height its own traversal
    /// establishes. The streaming mirror's erasure seam re-tags views instead,
    /// and checks them in debug builds ([`View::placed`](super::View::placed)).
    pub(crate) fn from_untyped(inner: untyped::Node) -> Self {
        Self {
            height: PhantomData,
            inner,
        }
    }

    /// Forget this node's height tag; the inverse of
    /// [`from_untyped`](Self::from_untyped).
    pub(crate) fn into_untyped(self) -> untyped::Node {
        self.inner
    }

    /// The join of every leaf version in this subtree.
    pub fn ceiling(&self) -> &Version {
        self.inner.ceiling()
    }

    /// The meet of every leaf version in this subtree.
    pub fn floor(&self) -> &Version {
        self.inner.floor()
    }

    /// This subtree's version bounds as one causal span: the memoized
    /// `[floor, ceiling]` pair, borrowed (see [`untyped::Node::span`]).
    pub fn span(&self) -> Span<'_> {
        self.inner.span()
    }

    /// The number of leaves in this subtree.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// The largest encoded version bound stored anywhere in this subtree.
    pub fn version_bytes(&self) -> usize {
        self.inner.version_bytes()
    }

    /// The largest canonical encoding among every version bound in this
    /// subtree, recomputed by direct walk with no aggregate memo.
    ///
    /// The independent oracle [`version_bytes`](Self::version_bytes) is pinned
    /// against; see the [untyped walk](untyped::Node::max_bound_bytes).
    #[cfg(test)]
    pub fn max_bound_bytes(&self) -> usize {
        self.inner.max_bound_bytes()
    }

    /// The memoized digest of this subtree under `key`, the hash key of the
    /// network whose tree holds it; see [`untyped::Node::hash`].
    pub fn hash(&self, key: &HashKey) -> Hash {
        self.inner.hash(key)
    }

    /// Whether `self` and `other`, at this height in trees of the network whose
    /// hash key is `key`, hold the same version set; see
    /// [`untyped::Node::same`].
    pub fn same(&self, other: &Self, key: &HashKey) -> bool {
        self.inner.same(&other.inner, key)
    }
}

/// Assemble branches or descend one level toward their children.
impl<H: Height> Node<S<H>>
where
    S<H>: Height,
{
    /// Assemble a subtree from its children.
    ///
    /// Empty input returns `None`. A single child is folded into its compressed
    /// prefix; only two or more children produce a materialized branch.
    pub fn branch(children: Children<H>) -> Option<Self> {
        Some(Node {
            height: PhantomData,
            inner: untyped::Node::branch(children.into_fan())?,
        })
    }

    /// Convert a node into a map from child index to child node (inverse to
    /// [`Node::branch`]).
    pub fn into_children(self) -> Children<H> {
        let children = match self.inner.into_children() {
            Ok(children) => children,
            Err(_) => unreachable!("typed nonzero-height node cannot be an uncompressed leaf"),
        };

        Children::from_fan(children)
    }

    /// Extend a test node's compressed prefix through slot `index`.
    ///
    /// Deep-tree fixtures use this to place a node one level farther down
    /// without allocating a single-child branch.
    #[cfg(test)]
    pub fn beneath(child: Node<H>, index: u8) -> Self {
        Node {
            height: PhantomData,
            inner: child.inner.beneath(index),
        }
    }
}

/// Construct and read height-zero leaves.
impl Node<Z> {
    /// Construct a new leaf node from a versioned message.
    pub fn leaf(version: Version, message: Message) -> Self {
        Self {
            height: PhantomData,
            inner: untyped::Node::leaf(version, message),
        }
    }

    /// Get a reference to the message at this leaf node.
    pub fn message(&self) -> &Message {
        self.inner
            .as_leaf()
            .expect("height-zero constructors admit only bare leaves")
    }
}

/// Look up leaves and start walks from the root.
impl Node<height::Root> {
    /// Look up a full leaf address in one descent, bounded by tree depth.
    pub fn get(&self, path: &[u8; PATH_LEN]) -> Option<(&Version, &Message)> {
        self.inner.get(path)
    }

    /// Borrow every leaf's version and message in ascending path order.
    pub fn iter(&self) -> untyped::Iter<'_> {
        untyped::Iter::root(&self.inner)
    }

    /// Start an owned walk over leaves whose versions satisfy `query`.
    ///
    /// The lifetime-free counterpart of [`range`](Self::range), holdable across
    /// awaits (see [`untyped::RangeOwned`]).
    pub fn range_owned<P: causally::Polarity>(
        node: Option<&Self>,
        query: causally::Query<'static, P>,
    ) -> untyped::RangeOwned<P> {
        untyped::RangeOwned::root(node.map(|node| node.inner.clone()), query)
    }

    /// Lazily iterate the live leaves of the (possibly absent) root `node`
    /// whose versions the causal `query` admits.
    ///
    /// Subtrees wholly outside the query are pruned by their memoized version
    /// bounds without being entered (see [`untyped::Range`]).
    pub fn range<'a, P: causally::Polarity>(
        node: Option<&'a Self>,
        query: causally::Query<'a, P>,
    ) -> untyped::Range<'a, P> {
        untyped::Range::root(node.map(|node| &node.inner), query)
    }

    /// The observable digest of a possibly-absent root under `key`.
    #[cfg(any(test, feature = "test-internals"))]
    pub fn root_hash(node: Option<&Root>, key: &HashKey) -> Hash {
        // An absent root is the empty tree, which hashes as a prefixless
        // branch with no children at the root's height
        // ([`HashKey::empty_root`]), not as the all-zero default.
        node.map_or_else(|| key.empty_root(), |node| node.hash(key))
    }
}
