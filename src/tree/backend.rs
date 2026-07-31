//! The materiality boundary: what a session's node *is* and what it costs.
//!
//! A [`Backend`] decides what a tree node physically is — a value carried
//! in the node's handle, or a reference into storage the backend owns —
//! and the streaming protocol is generic over that decision: an
//! implementation may hold its tree entirely in memory or in a storage
//! engine of its own, and the session schedule is identical either way.
//! [`Local`] is the reference implementation: its nodes are handles into
//! the crate's own in-memory tree, resident regardless of the session, so
//! custody costs it nothing new.
//!
//! Materiality has a price only the backend can state:
//! [`Backend::node_bytes`] declares what one node handle keeps resident,
//! and the session window prices every in-flight reference through it.
//! The obligation is sharp because its failure mode is the odd one out:
//! everywhere else in the budget derivation a mis-estimate costs latency,
//! while an underpriced node breaches the *memory* envelope instead. The
//! backend conformance suite (`crate::conformance::backend`, compiled as
//! this crate's own test gate; see [`crate::conformance`]) is how an
//! implementation proves its account.

use std::pin::{Pin, pin};

use futures::{Stream, StreamExt as _, stream};

use crate::{
    Version, causally,
    message::Message,
    tree::{
        mirror::streaming::convert::Convert,
        typed::{
            Hash, Prefix,
            height::{self, Height, S, Z},
        },
    },
};

// The specific backends:
mod local;
pub use local::Local;
#[cfg(test)]
pub(super) use local::with_schedule as with_local_schedule;

mod store;
pub use store::Store;
pub(crate) use store::ranged;
// The seam vocabulary `Store` speaks — the per-path action of the
// batch-apply, and the owned causal bounds of the range walk — re-exported
// where implementors and their conformance checks import the trait.
pub use super::traverse::act::Action;
pub use super::traverse::store::walk::VersionBounds;

/// A backend value is a cheap cloneable *handle* to its storage.
pub trait Backend<T: Send + Sync + 'static>: Clone + Send + Sync + 'static
where
    Self::Node<Z>: Leaf<T>,
{
    /// The type of nodes carrying messages of type `T`, indexed by height `H`.
    type Node<H: Height>: Node<T, Height = H, Backend = Self> + Clone + Send + Sync + 'static;

    /// The type of errors returned by this backend.
    ///
    /// The bound is what lets a backend failure surface through the public
    /// error enums (`thiserror` sources) unchanged.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Bytes one node value with `children` child entries keeps resident
    /// beyond the replica's own storage, its version bounds (ceiling and
    /// floor together) encoding within `version_bound` bytes.
    ///
    /// This prices the session window's in-flight references and the
    /// decode fan's buffered leaves
    /// ([`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget)).
    /// [`Local`]'s nodes are handles into a tree that is resident
    /// regardless, so its price is one pointer at every argument; a
    /// backend without an always-resident tree must charge everything its
    /// `Node` values own — at minimum the hash, the child table, and the
    /// two endpoints of the bounds span [`Node::span`] borrows from the
    /// node.
    ///
    /// A leaf is priced at `children = 0`: what its handle keeps resident
    /// after [`Leaf::leaf`] has had its chance to persist the payload.
    /// Payload bytes still in flight inside one wire message are priced
    /// by [`target_message_size`](crate::Peer::target_message_size), not
    /// here.
    ///
    /// The result must be an **upper bound**, monotone in both arguments
    /// — the derivation evaluates it at per-depth quantiles, and
    /// monotonicity is what keeps a quantile evaluation an upper bound
    /// (debug-asserted when a session derives its window, and swept over
    /// a fan and version-bound grid by the crate's backend conformance
    /// suite). Everywhere else in the budget derivation, mis-estimation
    /// costs latency; an underpriced node is the one input that breaches
    /// the *memory* envelope instead.
    fn node_bytes(children: usize, version_bound: usize) -> usize;

    /// Assemble one parent node at `prefix` from one radix-keyed child group.
    ///
    /// The group is the parent's entire child set, in strictly increasing radix
    /// order. A `None` entry is an explicit child *deletion*: the child does
    /// not join the parent. A backend materializing a session's incoming
    /// tree may drop whatever it stores beneath that radix; a backend whose
    /// nodes are shared copy-on-write across live tree versions must not —
    /// the deleted child's old parents remain reachable, and reclamation
    /// belongs to the backend's own garbage collection, keyed on the last
    /// reference, never to this call. A `None` return means no child survived and should propagate
    /// as a `None` entry one level up, cascading deletion to parents whose
    /// entire child set was deleted. The group may also be empty outright — a
    /// scope that resolved to nothing at all, such as the pruned-to-nothing
    /// reply to a request — and resolves to `None` the same way. Given at least
    /// one real child, construction should always yield a parent.
    fn parent<H>(
        self,
        prefix: Prefix<S<H>>,
        children: Vec<(u8, Option<Self::Node<H>>)>,
    ) -> impl Future<Output = Result<Option<Self::Node<S<H>>>, Self::Error>> + Send
    where
        H: Height,
        S<H>: Height;

    /// Explode one parent node at `prefix` into its children, one height down.
    ///
    /// The children are produced in strictly increasing prefix order, each
    /// keyed by the parent's prefix extended with the child's radix.
    fn children<H>(
        self,
        prefix: Prefix<S<H>>,
        parent: Self::Node<S<H>>,
    ) -> impl NodeStream<Self, T, H>
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
    ) -> impl NodeStream<Self, T, Z> {
        H::explode(
            self,
            Box::pin(stream::once(async move { Ok((prefix, node)) })),
        )
    }

    /// Assemble a strictly ascending leaf stream into height-`H` nodes, one
    /// node per maximal run of leaves sharing a height-`H` prefix, in run
    /// order.
    ///
    /// The inverse of [`leaves`](Self::leaves), and the same kind of seam:
    /// by default the leaves fold up through [`parent`](Self::parent) one
    /// level at a time, but a backend whose nodes are directly
    /// constructible may override it with a bulk builder.
    fn assemble<'a, H: Convert>(
        self,
        leaves: BoxNodeStream<'a, Self, T, Z>,
    ) -> impl NodeStream<Self, T, H> + 'a {
        H::assemble(self, leaves)
    }
}

/// The inspection operations of a backend's individual node type.
pub trait Node<T: Send + Sync + 'static> {
    /// The backend to which this node belongs.
    type Backend: Backend<T, Node<Z>: Leaf<T>, Node<Self::Height> = Self>;

    /// The height of the node above the leaf level.
    type Height: Height;

    /// The node's version bounds as one causal span: the floor (the
    /// minimum version of any node under this one) as the span's meet,
    /// the ceiling (the maximum) as its join.
    ///
    /// The trait's whole version-bounds obligation lives here. The
    /// deletion-honoring filter classifies subtrees by asking the span
    /// [`dominance_of`](causally::Span::dominance_of) directly, without
    /// descending: [`After`](causally::Dominance::After) iff the
    /// ceiling is within the probe's causal past (everything under the
    /// node is already known there),
    /// [`Before`](causally::Dominance::Before) iff the probe dominates
    /// not even the floor (the whole subtree is unknown), and
    /// [`Between`](causally::Dominance::Between) otherwise (mixed, so
    /// the filter descends). Single-endpoint consumers — bound pricing,
    /// containment checks, leaf-version reads — take
    /// [`meet`](causally::Span::meet) or [`join`](causally::Span::join)
    /// off the same span.
    ///
    /// The span's ordering — `floor <= ceiling`, which every honest
    /// meet/join pair over one leaf set satisfies — is the
    /// implementor's obligation, priced at *construction* rather than
    /// per read: the in-memory backend stores each branch's memoized
    /// bounds as one [`causally::Span`], ordered by construction, and
    /// answers by reborrowing it
    /// ([`causally::Span::reborrow`]). A backend reading bounds back
    /// from its own storage validates the pair once at node load: bounds
    /// stored as the span's canonical bytes load through
    /// [`causally::Span::decode`] — one pass that parses both endpoints
    /// and proves them ordered, rejecting corrupt bytes and crossed
    /// pairs alike as the load-time storage corruption they are — and
    /// bounds held as two already-decoded versions validate through
    /// [`causally::Span::new`], surfacing
    /// [`Crossed`](causally::Crossed) the same way. Either way the
    /// backend answers thereafter through the trusted door
    /// ([`causally::Span::new_unchecked`]) or its own stored span's reborrow.
    /// A violated ordering is not a detected fault: every verdict read
    /// off the span becomes unspecified, which here means silently
    /// wrong reconciliation — news withheld or re-sent — so the check
    /// belongs at the load seam, where it is paid once.
    fn span(&self) -> causally::Span<'_>;

    /// The merkle hash of this node.
    ///
    /// Must answer without I/O: either stored in the node record or
    /// computable from state the handle keeps resident. Traversals prune
    /// agreement by comparing hashes *before* fetching children, so a
    /// hash behind a fetch would defeat the pruning it exists to serve.
    fn hash(&self) -> Hash;

    /// The number of live leaves under this node, exact.
    ///
    /// A leaf answers one; a parent holds the sum over its children,
    /// fixed when [`Backend::parent`] assembles it — the same
    /// recompute-on-reassembly discipline as
    /// [`version_bytes`](Self::version_bytes), and cheap for a persistent
    /// backend to keep as a stored field. The root's value is the exact
    /// set size the session greeting carries.
    fn len(&self) -> usize;

    /// The largest canonical encoding among every version bound under
    /// this node — its leaf versions and every interior node's ceiling
    /// and floor, its own included — in bytes, exact.
    ///
    /// A leaf answers its own version's encoded length; a parent answers
    /// the **max over its children's values and its own two bounds'
    /// encodings**. That recurrence is the whole maintenance story: every
    /// mutation rebuilds its spine through [`Backend::parent`], so the
    /// max is recomputed from what remains and redacting the version
    /// that carries it resizes the aggregate *down* with no separate
    /// invalidation. Interior bounds must be covered: a ceiling joins
    /// every leaf version below it, and a join of many small concurrent
    /// stamps can encode several times larger than any one of them.
    ///
    /// The root's value is the version-size bound the session greeting
    /// carries, which the memory budget prices nodes with — an inflated
    /// value costs latency, a deflated one breaches the memory envelope.
    fn version_bytes(&self) -> usize;
}

/// What crosses between backends at the conversion boundary, and the one node
/// shape every backend must represent faithfully.
pub trait Leaf<T: Send + Sync + 'static>: Node<T> {
    /// The message stored at this leaf node.
    fn message(&self) -> &Message<T>;

    /// The leaf's version.
    ///
    /// Every read surface yields versions through this accessor, cloning
    /// the borrow it hands out — an `O(1)` copy of a refcounted handle,
    /// never a deep copy of the ITC event tree, because a [`Version`]
    /// clone shares its packed encoding. The same value is both endpoints
    /// of the leaf's coincident [`Node::span`].
    fn version(&self) -> &Version;

    /// Construct a leaf node from one decoded wire record, taking custody
    /// of its payload.
    ///
    /// This is the backend's one opportunity to persist the message
    /// before the leaf enters the decode fan: an eagerly persisting
    /// backend writes the payload here and returns a thin handle, and a
    /// backend that batches writes stages it in its own write-behind
    /// buffer — memory the backend owns and must price through
    /// [`Backend::node_bytes`] at `children = 0`, the shape the session
    /// budget charges for every buffered leaf. [`Local`] keeps the
    /// payload in the handle and completes immediately: its tree is
    /// resident regardless, so custody costs it nothing new.
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
        message: Message<T>,
    ) -> impl Future<Output = Result<Self, <Self::Backend as Backend<T>>::Error>> + Send
    where
        Self: Sized;
}

/// Collect one node's children, addressed by radix.
///
/// The materialization every level-wise traversal shares: one
/// [`Backend::children`] stream, drained into the radix-keyed fan a caller
/// rebuilds through [`Backend::parent`].
pub async fn children_of<B, T, H>(
    backend: &B,
    prefix: Prefix<S<H>>,
    node: B::Node<S<H>>,
) -> Result<Vec<(u8, B::Node<H>)>, B::Error>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
{
    let mut children = pin!(backend.clone().children(prefix, node));
    let mut fan = Vec::new();
    while let Some(item) = children.next().await {
        let (prefix, child) = item?;
        let (_, radix) = prefix.pop();
        fan.push((radix, child));
    }
    Ok(fan)
}

/// Type synonym for a fallible [`Stream`] of prefix-keyed nodes represented by
/// a given backend.
pub trait NodeStream<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static, H: Height>:
    Stream<Item = Result<(Prefix<H>, B::Node<H>), B::Error>> + Send
{
}
impl<N, B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static, H: Height> NodeStream<B, T, H>
    for N
where
    N: Stream<Item = Result<(Prefix<H>, B::Node<H>), B::Error>> + Send,
{
}

/// A [`NodeStream`] erased to one level of type depth.
pub(crate) type BoxNodeStream<'a, B, T, H> = Pin<Box<dyn NodeStream<B, T, H> + 'a>>;

/// A boxed, fully-owned walk over selected live leaves, keyed by their
/// [`Key`](crate::tree::Key)s: what holders of [`Store::range`]'s stream
/// keep across polls and awaits.
pub(crate) type LeafWalk<T, S> = futures::stream::BoxStream<
    'static,
    Result<(crate::tree::Key, <S as Backend<T>>::Node<Z>), <S as Backend<T>>::Error>,
>;

/// A backend's whole tree at rest: the node structure (absent when empty)
/// and the causal ceiling that rides *outside* it.
///
/// The ceiling outlives the nodes — it advances on effectual redactions and
/// survives a tree emptying out — which is exactly what deletion honoring
/// compares against. What a replica holds between mutations, and what a
/// mirror session consumes and produces.
#[derive(Debug)]
pub struct Root<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static> {
    /// The maximum version this tree has incorporated.
    pub ceiling: Version,
    /// The root node, or nothing when the tree is empty.
    pub root: Option<B::Node<height::Root>>,
}

/// The empty root: the empty [`Version`] over no nodes. Lets callers
/// `mem::take` a root out of a `&mut` borrow (e.g. to move it into a mirror
/// exchange and write the merged result back) without an interim clone.
impl<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static> Default for Root<B, T> {
    fn default() -> Self {
        Root {
            ceiling: Version::new(),
            root: None,
        }
    }
}

impl<B: Store<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static> PartialEq for Root<B, T> {
    fn eq(&self, other: &Self) -> bool {
        self.ceiling == other.ceiling
            && match (&self.root, &other.root) {
                (None, None) => true,
                (Some(ours), Some(theirs)) => {
                    crate::tree::traverse::store::join::equivalent::<B, T, _>(ours, theirs)
                }
                _ => false,
            }
    }
}

impl<B: Store<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static> Eq for Root<B, T> {}

// Manual because the derive would demand `T: Clone`; nodes are cloneable
// handles regardless of the message type they carry.
impl<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static> Clone for Root<B, T> {
    fn clone(&self) -> Self {
        Root {
            ceiling: self.ceiling.clone(),
            root: self.root.clone(),
        }
    }
}

impl<B: Backend<T, Node<Z>: Leaf<T>>, T: Send + Sync + 'static> Root<B, T> {
    /// The tree's live message count: the root node's [`len`](Node::len)
    /// aggregate, or zero when empty. What the session greeting carries
    /// as the exact set size.
    pub(crate) fn len(&self) -> u64 {
        self.root
            .as_ref()
            .map(|node| node.len() as u64)
            .unwrap_or_default()
    }

    /// The largest canonical version-bound encoding in the tree, in
    /// bytes: what the session greeting carries as the version-size
    /// bound.
    ///
    /// The root node's [`version_bytes`](Node::version_bytes) aggregate
    /// — leaf versions and every interior ceiling and floor — or zero
    /// when empty. The first read materializes it: every branch's
    /// bounds memo is forced tree-wide, `O(#branches)` bound
    /// folds, once per tree lineage — the memos are shared through the
    /// node handles across snapshots, and a mutation invalidates only
    /// its own spine. A fully converged pair pays this once, at
    /// greeting time.
    pub(crate) fn max_version_bytes(&self) -> u64 {
        self.root
            .as_ref()
            .map(|node| node.version_bytes() as u64)
            .unwrap_or_default()
    }
}
