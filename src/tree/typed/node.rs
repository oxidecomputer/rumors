use std::{fmt::Debug, iter::Map, marker::PhantomData};

use before::{Dominance, Span};

use crate::{Version, causally, message::Message};

use super::hash::Hash;
use super::height::{self, Height, S, Z};
use super::untyped;
use untyped::fan::{self, Fan};

/// The typed node with a height of 32; the root of the tree.
pub type Root = Node<height::Root>;

/// The radix-indexed children of a branch one level above height `H`: a
/// typed shell over the untyped radix fan, so inserts and removals stay
/// height-correct at compile time.
pub struct Children<H: Height> {
    height: PhantomData<fn() -> H>,
    inner: Fan,
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

    /// Insert `child` at `radix`, returning any child it displaced.
    pub fn insert(&mut self, radix: u8, child: Node<H>) -> Option<Node<H>> {
        self.inner
            .insert(radix, child.into_untyped())
            .map(Node::from_untyped)
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
    type Item = (u8, Node<H>);
    type IntoIter = Map<fan::IntoIter, fn((u8, untyped::Node)) -> (u8, Node<H>)>;

    /// Transfer ownership of each child to the caller.
    fn into_iter(self) -> Self::IntoIter {
        self.inner
            .into_iter()
            .map(typed_child::<H> as fn((u8, untyped::Node)) -> (u8, Node<H>))
    }
}

/// A typed node which enforces the structural validity of the constructed tree
/// at compile-time.
///
/// The height marker is held as `PhantomData<fn() -> H>` rather than
/// `PhantomData<H>`. Function pointers are unconditionally `Send + Sync`,
/// so any auto-trait obligation on `Node` discharges without descending
/// the `S<S<S<...S<Z>...>>>` peano-style height chain: a bare
/// `PhantomData<H>` would send the trait solver walking 32 levels of
/// `S<…>: Sync` on every `Send`/`Sync` check, even though the type
/// variable `H` is purely phantom and never constructs anything that
/// could fail to be `Send`/`Sync`.
#[repr(transparent)]
pub struct Node<H: Height> {
    height: PhantomData<fn() -> H>,
    inner: untyped::Node,
}

impl<H: Height> Clone for Node<H> {
    fn clone(&self) -> Self {
        Self {
            height: self.height,
            inner: self.inner.clone(),
        }
    }
}

impl<H> Debug for Node<H>
where
    H: Height,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl<H: Height> Node<H> {
    /// Whether both handles refer to the same node allocation, without hashing.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        self.inner.ptr_eq(&other.inner)
    }

    /// Tag an untyped node at height `H`: the caller asserts the height.
    ///
    /// The streaming mirror's erasure seam
    /// ([`Backend::assume`](crate::tree::mirror::streaming::Backend::assume))
    /// re-tags nodes it erased at the same height; internal tree code tags
    /// nodes whose height its own traversal establishes.
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

    /// Get the ceiling version of this node (the greatest version contained within).
    pub fn ceiling(&self) -> &Version {
        self.inner.ceiling()
    }

    /// Get the floor version of this node (the least version contained within).
    pub fn floor(&self) -> &Version {
        self.inner.floor()
    }

    /// This subtree's version bounds as one causal span: the memoized
    /// `[floor, ceiling]` pair, borrowed (see [`untyped::Node::span`]).
    pub fn span(&self) -> Span<'_> {
        self.inner.span()
    }

    /// How much of this subtree's memoized `[floor, ceiling]` bounds
    /// `probe` dominates: the deletion-honoring classifiers' verdict,
    /// answered from the memos without descending.
    ///
    /// A branch answers through its stored bounds span — ordered by
    /// construction, so no validating comparison is paid at any
    /// classification — and a leaf's coincident bounds collapse the
    /// question to one containment check (see
    /// [`untyped::Node::dominance`]).
    pub fn dominance(&self, probe: &Version) -> Dominance {
        self.inner.dominance(probe)
    }

    /// Get the number of leaves under this node.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// The largest canonical [`Version`] encoding among every bound this
    /// subtree holds — leaf versions and every branch's ceiling and
    /// floor — in bytes.
    ///
    /// Exact under deletion, like [`len`](Self::len): every mutation
    /// rebuilds its copy-on-write spine through the branch constructors
    /// with fresh memos, so the max is recomputed lazily from what
    /// remains.
    pub fn version_bytes(&self) -> usize {
        self.inner.version_bytes()
    }

    /// The largest canonical encoding among every version bound in this
    /// subtree, recomputed by direct walk with no aggregate memo.
    ///
    /// The independent oracle [`version_bytes`](Self::version_bytes) is
    /// pinned against; see the
    /// [untyped walk](untyped::Node::max_bound_bytes).
    #[cfg(any(test, feature = "test-internals"))]
    pub fn max_bound_bytes(&self) -> usize {
        self.inner.max_bound_bytes()
    }

    /// Hash the subtree rooted at this node.
    ///
    /// Hashes are computed lazily on first read and memoized, so the first read
    /// of a freshly-built subtree costs `O(nodes)` and every read thereafter is
    /// an `O(1)` field load.
    ///
    /// The hashing convention (see [`Hash::leaf`] and [`Hash::branch`]): one
    /// preimage per node, committing its kind, its compressed prefix in path
    /// order, and, for a branch, its children as ascending `radix ‖ hash`
    /// records. Equal content yields equal hashes because equal content
    /// yields equal canonical shape; see [`Hash::branch`]'s canonicity
    /// section.
    pub fn hash(&self) -> Hash {
        self.inner.hash()
    }

    /// Walk every leaf beneath this node, in ascending path order.
    ///
    /// `prefix` supplies the preceding path bytes. The owned walk skips
    /// compressed spans and yields one stored leaf at a time, then
    /// [`untyped::Leaf::into_node`] gives it the empty prefix required at
    /// height zero. Descent state is bounded by the tree's depth.
    pub(crate) fn leaves(
        self,
        prefix: &super::Prefix<H>,
    ) -> impl Iterator<Item = (super::Prefix<Z>, Node<Z>)> + Send + use<H> {
        untyped::RangeOwned::within(Some(self.inner), prefix.as_bytes(), causally::all()).map(
            |(key, leaf)| {
                (
                    super::Prefix::from(key),
                    Node::from_untyped(leaf.into_node()),
                )
            },
        )
    }

    /// Build the height-`H` node over one sorted run of bare leaves.
    ///
    /// Every path in `run` extends `prefix`, strictly ascending, and the
    /// run is non-empty: exactly the shape one supplied scope's leaves
    /// arrive in off the wire. The bulk inverse of
    /// [`leaves`](Self::leaves); see
    /// [`untyped::Node::from_sorted_leaves`] for the cost argument.
    pub(crate) fn from_sorted_leaves(
        prefix: &super::Prefix<H>,
        run: Vec<(super::Prefix<Z>, Node<Z>)>,
    ) -> Self {
        let depth = prefix.as_bytes().len();
        debug_assert!(
            run.iter()
                .all(|(path, _)| path.as_bytes().starts_with(prefix.as_bytes())),
            "every leaf in a run falls under the run's prefix",
        );
        let mut entries: Vec<([u8; 32], Option<untyped::Node>)> = run
            .into_iter()
            .map(|(path, leaf)| {
                let path = <[u8; 32]>::try_from(path.as_bytes())
                    .expect("a leaf prefix is a full 32-byte path");
                (path, Some(leaf.into_untyped()))
            })
            .collect();
        Self::from_untyped(untyped::Node::from_sorted_leaves(depth, &mut entries))
    }
}

impl<H: Height> Node<S<H>>
where
    S<H>: Height,
{
    /// Construct a new branch node from a map of children (inverse to
    /// [`Node::into_children`]).
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
            .expect("typed leaf failed to be a leaf")
    }
}

impl Node<height::Root> {
    /// Look up the live leaf whose full 32-byte path is `path`, by a single
    /// `O(depth)` descent.
    pub fn get(&self, path: &[u8]) -> Option<(&Version, &Message)> {
        self.inner.get(path)
    }

    /// Lazily iterate every live leaf in this root subtree as
    /// `([u8; 32], &Version, &Message)`.
    ///
    /// Delegates to the height-agnostic untyped walk; because this is a
    /// height-32 root, every yielded path is a full 32-byte array.
    pub fn iter(&self) -> untyped::Iter<'_> {
        untyped::Iter::root(&self.inner)
    }

    /// Freeze a fully-owned walk over the leaves of the (possibly absent)
    /// root `node` whose versions the causal `query` admits.
    ///
    /// The lifetime-free counterpart of [`range`](Self::range), holdable
    /// across awaits (see [`untyped::RangeOwned`]).
    pub fn range_owned<P: causally::Polarity>(
        node: Option<&Self>,
        query: causally::Query<'static, P>,
    ) -> untyped::RangeOwned<P> {
        untyped::RangeOwned::root(node.map(|node| node.inner.clone()), query)
    }

    /// Lazily iterate the live leaves of the (possibly absent) root `node`
    /// whose versions the causal `query` admits.
    ///
    /// Subtrees wholly outside the query are pruned by their memoized
    /// version bounds without being entered (see [`untyped::Range`]).
    pub fn range<'a, P: causally::Polarity>(
        node: Option<&'a Self>,
        query: causally::Query<'a, P>,
    ) -> untyped::Range<'a, P> {
        untyped::Range::root(node.map(|node| &node.inner), query)
    }

    /// The observable hash of a possibly-absent root.
    pub fn root_hash(node: &Option<Root>) -> Hash {
        // An absent root is the empty tree, which hashes as a prefixless
        // branch with no children (`sha3_256(BRANCH_TAG ‖ 0 ‖ 0u16)`), not as
        // the all-zero default.
        node.as_ref()
            .map(|n| n.hash())
            .unwrap_or_else(Hash::empty_root)
    }
}

impl<H: Height> Eq for Node<H> {}

impl<H: Height> PartialEq for Node<H> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}
