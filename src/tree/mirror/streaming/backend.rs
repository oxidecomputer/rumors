//! Node storage for streaming reconciliation.
//!
//! A [`Backend`] defines the node handles used by a session and the
//! operations over those handles. This boundary is crate-internal; [`Local`]
//! is its production implementation and stores handles into the crate's
//! in-memory tree.
//!
//! The backend conformance suite checks the behavioral contracts against
//! each implementation used by the crate.

use std::pin::Pin;

use before::Span;
use futures::{Stream, stream};

use crate::{
    Version,
    counter::widen,
    message::Message,
    tree::{
        mirror::streaming::{Radices, convert::Convert},
        typed::{
            Hash, Prefix,
            height::{self, Height, S, Z},
        },
    },
};

/// The in-memory production backend.
mod local;
pub use local::Local;
#[cfg(test)]
pub(super) use local::with_schedule as with_local_schedule;

/// Node storage used by a streaming session; values are cheap cloneable handles.
///
/// # Independence from other session work
///
/// Every operation completes without waiting on any other work of the
/// session. The decoders that absorb supplied leaves and the walk share the
/// backend, and the decoders must never wait on the walk: an operation that
/// held a lock, transaction, or pooled resource while a walk task was
/// blocked on a queue could stop a decoder, and with it the delivery the
/// walk is waiting for.
pub trait Backend: Clone + Send + Sync + 'static
where
    Self::Node<Z>: Leaf,
{
    /// The type of nodes, indexed by height `H`.
    type Node<H: Height>: Node<Height = H, Backend = Self> + Clone + Send + 'static;

    /// One runtime representation shared by every height's
    /// [`Node<H>`](Self::Node).
    ///
    /// The height parameter on [`Node`](Self::Node) is a compile-time tag
    /// over runtime data that already knows its place in the tree, so a
    /// backend can forget the tag ([`erase`](Self::erase)) and restore it
    /// ([`assume`](Self::assume)) without changing the value. The
    /// session's internal plumbing — its channels, pumps, and walk workers
    /// — carries this one type where a height-typed payload would
    /// instantiate the whole machinery once per height.
    ///
    /// A backend whose nodes share one representation across heights (as
    /// [`Local`]'s do) uses that representation directly; otherwise it uses a
    /// sum of its per-height representations.
    type Erased: ErasedNode + Clone + Send + 'static;

    /// The type of errors returned by this backend.
    type Error: Send + 'static;

    /// Forget a node's height tag.
    fn erase<H: Height>(node: Self::Node<H>) -> Self::Erased;

    /// Re-tag an erased node at height `H`.
    ///
    /// `H` must be the height the node was erased at:
    /// `assume::<H>(erase::<H>(node)) == node` is the whole contract, and
    /// a cross-height re-tag is a programmer error whose behavior is
    /// unspecified. The pairing discipline lives inside the session's
    /// erased plumbing, witnessed by the prefixes that travel alongside
    /// every node: an erased prefix's byte length is its height, and
    /// re-tagging one debug-asserts that length. Peer input cannot reach
    /// a mispairing — every wire node decodes through height-typed
    /// readers before it is ever erased.
    fn assume<H: Height>(erased: Self::Erased) -> Self::Node<H>;

    /// Assemble one parent node at `prefix` from one radix-keyed child group.
    ///
    /// The group is the parent's entire child set, in strictly increasing radix
    /// order. A `None` entry is an explicit child *deletion*: the child does
    /// not join the parent, and the backend may drop whatever it stores beneath
    /// that radix. A `None` return means no child survived and should propagate
    /// as a `None` entry one level up, cascading deletion to parents whose
    /// entire child set was deleted. The group may also be empty outright — a
    /// scope that resolved to nothing at all, such as the pruned-to-nothing
    /// reply to a request — and resolves to `None` the same way. Given at least
    /// one `Some` child, construction **must** yield a parent: under the default
    /// [`assemble`](Self::assemble), a `None` here becomes a missing assembled
    /// node, which the reply decoder treats as a backend contract violation
    /// and enforces by panic.
    ///
    /// `original` is the node that stood at `prefix` before the session
    /// explored it: the node whose [`children`](Self::children) the session
    /// listed there, or `None` when nothing was listed, as when this side
    /// held nothing at `prefix` or the group comes from a leaf stream. The
    /// group alone decides the result; `original` only lets a backend skip
    /// work. When the group's surviving children are exactly the children
    /// `original` lists, at the same radices, the backend may return
    /// `original` itself, with everything it stores or has computed for it.
    /// Otherwise it may derive the new node's aggregates from `original`'s
    /// and those of the children that differ, wherever that derivation is
    /// exact.
    fn parent<H>(
        self,
        prefix: Prefix<S<H>>,
        original: Option<Self::Node<S<H>>>,
        children: Vec<(u8, Option<Self::Node<H>>)>,
    ) -> impl Future<Output = Result<Option<Self::Node<S<H>>>, Self::Error>> + Send
    where
        H: Height,
        S<H>: Height;

    /// Explode one parent node at `prefix` into its children, one height down.
    ///
    /// The children are produced in strictly increasing prefix order, each
    /// keyed by the parent's prefix extended with the child's radix.
    ///
    /// Listing allocates nothing beyond the handles it yields: the session
    /// charges each listing's handles to the place that holds them, and
    /// charges nothing for the listing itself. A backend whose node handles
    /// reach storage it loads keeps that storage within the handles; the
    /// in-memory backend yields views that share the stored node. The
    /// backend conformance suite meters every listing against this.
    fn children<H>(
        self,
        prefix: Prefix<S<H>>,
        parent: Self::Node<S<H>>,
    ) -> impl NodeStream<Self, H>
    where
        H: Height,
        S<H>: Height;

    /// Get the leaves of a node directly.
    ///
    /// By default, this is implemented as a streaming recursive traversal of
    /// the node's children, but some backends may be able to obtain this more
    /// efficiently. An override must preserve what the default guarantees —
    /// the wire encoder enforces each by panic:
    ///
    /// - every yielded prefix extends the requested `prefix` (containment);
    /// - prefixes are yielded in strictly ascending path order;
    /// - a node yields at least one leaf (every node contains one).
    fn leaves<H: Convert>(
        self,
        prefix: Prefix<H>,
        node: Self::Node<H>,
    ) -> impl NodeStream<Self, Z> {
        H::explode(
            self,
            Box::pin(stream::once(async move { Ok((prefix, node)) })),
        )
    }

    /// Assemble a strictly ascending leaf stream into height-`H` nodes, one
    /// node per maximal run of leaves sharing a height-`H` prefix, in run
    /// order.
    ///
    /// This is the inverse of [`leaves`](Self::leaves). By default, leaves fold
    /// through [`parent`](Self::parent) one level at a time, each height
    /// holding one open group of at most a fan of children, so the default
    /// holds transient memory bounded by the heights and the fan, whatever
    /// a run's size. A backend with directly constructible nodes may build
    /// in fewer steps; an override should keep that bound, since a run can
    /// be a whole supplied subtree.
    ///
    /// An override must preserve what the default guarantees — the reply
    /// decoder enforces each by panic:
    ///
    /// - exactly one node per maximal run, in run order (never merged,
    ///   split, or skipped);
    /// - each node at its own run's height-`H` prefix.
    fn assemble<'a, H: Convert>(
        self,
        leaves: BoxNodeStream<'a, Self, Z>,
    ) -> impl NodeStream<Self, H> + 'a {
        H::assemble(self, leaves)
    }
}

/// The inspection operations of a backend's individual node type.
pub trait Node {
    /// The backend to which this node belongs.
    type Backend: Backend<Node<Z>: Leaf, Node<Self::Height> = Self>;

    /// The height of the node above the leaf level.
    type Height: Height;

    /// The meet and join of every live version in this subtree.
    ///
    /// Deletion filtering compares a probe with this span. If the probe
    /// dominates the ceiling, all messages are known; if it does not dominate
    /// even the floor, all are unknown. Mixed spans require descent.
    ///
    /// The bounds must be ordered. Construct them with the node, or validate
    /// them once when loading storage: [`Span::decode`] checks encoded bounds
    /// and [`Span::new`] checks decoded ones. Then borrow the validated span
    /// for each read. Invalid bounds can make reconciliation discard or resend
    /// messages, so this obligation cannot be deferred to callers.
    fn span(&self) -> Span<'_>;

    /// The exact number of live leaves in the subtree.
    ///
    /// A leaf has one; a parent has the sum of its children's counts. The
    /// root's count supplies the session greeting's set size.
    fn len(&self) -> usize;

    /// The exact number of children this node's children have, as
    /// [`Backend::children`] lists them: over its children, each one's own
    /// child count, a child at the leaves counting one. A leaf has none.
    ///
    /// It bounds the reply this side gives to a question about the node: a
    /// session sends this count beside each child it lists, and its peer
    /// sizes the memory that reply can need from it before asking, so an
    /// undercount fails the session when the reply arrives. Like
    /// [`len`](Self::len), maintain it wherever a node is built; it depends
    /// only on the node's children, and on whether each child is itself a
    /// leaf or has one child or many.
    fn grandchild_count(&self) -> usize;

    /// The largest encoded version bound in this subtree, in bytes.
    ///
    /// Include leaf versions and every branch's floor and ceiling. A leaf
    /// returns its version's size; a parent takes the maximum over its own
    /// bounds and its children's aggregates. Recompute this when rebuilding a
    /// node, including after redactions.
    ///
    /// Interior bounds matter: joining small concurrent versions can produce
    /// a larger encoding. The root supplies this aggregate to the session's
    /// greeting, and the peer fails the session at any supplied version
    /// that encodes larger, so an undercount fails sessions with conforming
    /// peers.
    fn version_bytes(&self) -> usize;
}

/// Inspect a node without knowing its height.
///
/// The session compares hashes and classifies version spans before choosing
/// whether to descend. Operations that need the height, such as reading
/// children or a leaf's payload, require [`Backend::assume`] first.
pub trait ErasedNode {
    /// The node's version bounds as one causal span ([`Node::span`]).
    fn span(&self) -> Span<'_>;

    /// The subtree digest used to compare nodes at the same trie position.
    fn hash(&self) -> Hash;

    /// The number of live leaves under this node, exact ([`Node::len`]).
    fn len(&self) -> usize;

    /// The number of children [`Backend::children`] lists for this node,
    /// exact: at most a fan, and none for a leaf.
    ///
    /// A session reads this before it lists the children of a node it
    /// asks about, to price the question's listing at exactly that many
    /// entries, and allocates the listing at that capacity. A backend that
    /// maintains [`grandchild_count`](Self::grandchild_count) already computes it
    /// for every child it sums over. A count above a fan, or one the
    /// listing does not bear out, fails the session
    /// ([`CountError`](super::message::CountError)).
    fn child_count(&self) -> usize;

    /// The number of children this node's children have, exact
    /// ([`Node::grandchild_count`]).
    fn grandchild_count(&self) -> usize;

    /// The radices of the children [`Backend::children`] lists for this
    /// node, exact: [`child_count`](Self::child_count) of them, and none
    /// for a leaf.
    ///
    /// A session reads this before it explodes a node it answers about,
    /// and prices the answer at the children either side holds: these
    /// radices joined with those the peer listed. A backend can answer it
    /// from what it stores for the node without listing the children; the
    /// in-memory backend reads its fan's radices, or a compressed node's
    /// one. A radix missing from the answer leaves the answer's resolution
    /// and plan costing more than its grant.
    fn child_radices(&self) -> Radices;
}

/// What crosses between backends at the conversion boundary, and the one node
/// shape every backend must represent faithfully.
pub trait Leaf: Node {
    /// The message stored at this leaf node.
    fn message(&self) -> &Message;

    /// Construct a leaf node from one decoded wire record, taking custody
    /// of its payload.
    ///
    /// This is the backend's one opportunity to persist the message
    /// before the leaf enters the decode fan: an eagerly persisting
    /// backend writes the payload here and returns a thin handle, and a
    /// backend that batches writes stages it in its own write-behind
    /// buffer, memory the backend owns and bounds itself. [`Local`] keeps
    /// the payload in
    /// its handle and completes immediately because its tree is already
    /// resident.
    ///
    /// # Errors
    ///
    /// A failed construction is a backend error and ends the session the
    /// same way a failed [`Backend::parent`] does; the record it carried
    /// is re-supplied by a later session.
    ///
    /// # Cancel safety
    ///
    /// A session dropped mid-decode drops this future with it. The
    /// backend must tolerate the drop at any await point: a persisted
    /// payload whose handle never surfaced must be either idempotently
    /// re-persistable or garbage the backend can reclaim — the record
    /// arrives again in a later session while the divergence persists,
    /// and a meanwhile-redacted message correctly never re-arrives
    /// (the reclaimable-garbage case).
    fn leaf(
        version: Version,
        message: Message,
    ) -> impl Future<Output = Result<Self, <Self::Backend as Backend>::Error>> + Send
    where
        Self: Sized;
}

/// Type synonym for a fallible [`Stream`] of prefix-keyed nodes represented by
/// a given backend.
pub trait NodeStream<B: Backend<Node<Z>: Leaf>, H: Height>:
    Stream<Item = Result<(Prefix<H>, B::Node<H>), B::Error>> + Send
{
}
impl<N, B: Backend<Node<Z>: Leaf>, H: Height> NodeStream<B, H> for N where
    N: Stream<Item = Result<(Prefix<H>, B::Node<H>), B::Error>> + Send
{
}

/// A [`NodeStream`] erased to one level of type depth.
pub(crate) type BoxNodeStream<'a, B, H> = Pin<Box<dyn NodeStream<B, H> + 'a>>;

/// A backend's whole tree at rest: what a mirror session consumes and produces.
///
/// This is the backend-generic form of [`tree::Root`](crate::tree::Root); the
/// `Local` backend converts between the two with [`From`].
#[derive(Debug, Clone)]
pub struct Root<B: Backend<Node<Z>: Leaf>> {
    /// The maximum version this tree has incorporated.
    pub ceiling: Version,
    /// The root node, or nothing when the tree is empty.
    pub root: Option<B::Node<height::Root>>,
}

/// Read the aggregate metadata exchanged in a session greeting.
impl<B: Backend<Node<Z>: Leaf>> Root<B> {
    /// The tree's live message count: the root node's [`len`](Node::len)
    /// aggregate, or zero when empty. What the session greeting carries
    /// as the exact set size.
    pub(crate) fn len(&self) -> u64 {
        self.root
            .as_ref()
            .map(|node| widen(node.len()))
            .unwrap_or_default()
    }

    /// The largest canonical version-bound encoding in the tree, in
    /// bytes: what the session greeting carries as the version-size
    /// bound.
    ///
    /// This is the root node's [`version_bytes`](Node::version_bytes)
    /// aggregate, or zero for an empty tree.
    pub(crate) fn max_version_bytes(&self) -> u64 {
        self.root
            .as_ref()
            .map(|node| widen(node.version_bytes()))
            .unwrap_or_default()
    }
}
