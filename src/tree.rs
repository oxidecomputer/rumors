//! The content tree: a sparse Merkle radix trie that makes replica
//! difference *observable* and replica union *cheap*.
//!
//! End-user documentation lives in the [crate docs](crate); here we discuss
//! the design.
//!
//! # Shape
//!
//! Branching factor 256, fixed depth 32: a leaf's path is its 32-byte
//! version address, one byte per level — the full-width hash of the
//! leaf's version, keyed by the network's secret
//! ([`Path::for_leaf`](typed::Path::for_leaf)). Message
//! bytes enter no path and no digest; identity rests on the invariant the
//! protocol already requires everywhere, that no two messages ever share
//! a version. Version addressing buys three properties at once:
//!
//! - **The set is the tree.** Where a leaf lives is fully determined by
//!   the version stamped on it, so two replicas holding the same messages
//!   hold the same tree, regardless of insertion order or which peer sent
//!   what. Union is well-defined node-by-node.
//! - **Equal hash ⟹ equal subtree.** Each node memoizes a Merkle hash of
//!   its subtree — a pure function of the version set under the network's
//!   hash key, blind to message bytes — so replicas can prune agreement
//!   wholesale: the engine of the [`mirror`] protocol's
//!   divergence-proportional cost. The Merkle hash is a 16-byte truncation,
//!   deliberately narrower than the 32-byte version address: a comparison
//!   signal tolerates truncation that an identity cannot (see
//!   [`typed::Hash`] for the asymmetry argument).
//! - **Uniform spread.** Hashed paths are uniform, so the trie is
//!   expected-balanced; and keyed by a secret no one outside the network
//!   holds, no one can choose versions that crowd one subtree or collide
//!   in a digest. Depth bounds are real bounds.
//!
//! Single-child spines are path-compressed away, and each node's hash is a
//! single keyed preimage committing its height, compressed prefix and
//! children together (see [`HashKey::branch`](typed::HashKey::branch)).
//! Hash agreement across
//! peers therefore rests on the tree's *canonical shape*: equal content
//! yields equal compression, by the same ≥ 2-children maximal-compression
//! invariant enforced by the node constructors.
//!
//! # Memos and sharing
//!
//! Every node lives behind an `Arc` (its children a sorted radix fan of further
//! handles), so cloning a tree is O(1) and shares structure; mutation is
//! copy-on-write along the touched spine.
//!
//! Nodes memoize their Merkle hashes, version bounds, and maximum encoded
//! version sizes. Publication forces these values before readers can reach a
//! tree; traversal candidates may still be cold. Unchanged subtrees share their
//! memos. The bounds let deletion filtering and causal range queries skip whole
//! subtrees, and encoded-version sizes let a session budget its decode buffers.
//!
//! # The traversal trio
//!
//! All mutation and reconciliation is three inductive traversals over the
//! same structure ([`traverse`]): [`act`](Tree::act) applies a local batch
//! in one pass; [`join`](Tree::join) merges two in-memory trees;
//! [`mirror`] reconciles two trees over a wire. `join` and `mirror` must produce
//! the same live set and causal history. Their deletion filters are separate
//! implementations: tests check `join` against a flat, leaf-by-leaf survivor
//! rule, then compare wire reconciliation with `join`.

use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

pub(crate) mod traverse;
pub(crate) mod typed;

use crate::{Version, causally, message::Message, network::NetworkKeys, tree::typed::Node};

#[cfg(any(test, feature = "test-internals"))]
pub(crate) use typed::hash::MERKLE_HASH_LEN;

pub mod mirror;

/// The fully-owned, lifetime-free leaf walk and the leaf handle it yields;
/// the engine beneath [`Rumors::unordered_messages`](crate::Rumors::unordered_messages) and the
/// streams built over it.
pub use typed::{Leaf, RangeOwned};

/// A sparse Merkle radix trie with transparent path compression, whose
/// leaves store versioned [`Message`]s.
///
/// The tree has a branching factor of 256 and a depth of 32, so a leaf's
/// 32-byte path is the full-width hash of its version, keyed by the secret of
/// the network the tree belongs to (see
/// [`Path::for_leaf`](typed::Path::for_leaf)). Versions are unique per
/// send — locally by tick, globally by party disjointness — so two
/// content-identical messages sent at distinct moments occupy distinct
/// leaves, and two leaves collide only when a version has been reused,
/// which conforming peers cannot do.
pub struct Tree<T> {
    /// The live nodes and the causal history used to honor redactions.
    pub(crate) root: Root,
    /// The keys of the network this tree belongs to, whose path key places
    /// every leaf in it.
    ///
    /// Two trees' paths agree only under one secret, so only trees with
    /// equal keys may be joined. Shared, so every clone of a tree, and the
    /// peer it belongs to, hold one copy of the keys.
    keys: Arc<NetworkKeys>,
    /// The payload type this tree's typed faces read leaves at.
    ///
    /// Storage is erased ([`Message`] holds `dyn Any`); the facade's `T`
    /// names the type its faces downcast to (as `fn() -> T`, so
    /// auto-traits never descend into `T`).
    payload: PhantomData<fn() -> T>,
}

/// The live nodes and the latest version observed by the tree.
///
/// The ceiling survives redactions, even when no nodes remain. Joins use it
/// to distinguish messages not yet seen from messages already redacted.
///
/// A root carries no keys, so it compares through the tree that holds it
/// ([`same`](Self::same)).
#[derive(Clone, Debug)]
pub struct Root {
    /// Causal history, including redactions; excluded from the node hash.
    ceiling: Version,
    /// Live messages, absent when the tree is empty.
    node: Option<typed::node::Root>,
}

/// The empty root: the empty [`Version`] over no nodes. The state a mirror
/// exchange starts from when the local side holds nothing yet: a
/// bootstrapping peer mirrors the provider's tree into it.
impl Default for Root {
    fn default() -> Self {
        Root {
            ceiling: Version::new(),
            node: None,
        }
    }
}

impl Root {
    /// Return the number of live messages represented by this root.
    pub(crate) fn len(&self) -> usize {
        self.node.as_ref().map(typed::Node::len).unwrap_or_default()
    }

    /// Whether `self` and `other`, roots of trees of the network whose hash
    /// key is `key`, hold the same causal history and the same live
    /// version set ([`typed::Node::same`]).
    pub(crate) fn same(&self, other: &Self, key: &typed::HashKey) -> bool {
        self.ceiling == other.ceiling
            && match (&self.node, &other.node) {
                (None, None) => true,
                (Some(ours), Some(theirs)) => ours.same(theirs, key),
                _ => false,
            }
    }
}

/// Compare two roots of trees under the fixture keys, as [`same`](Root::same)
/// does under their hash key.
///
/// Test-only: every test that compares bare roots builds them under
/// [`NetworkKeys::fixture`], the keys streaming test sessions run under. A
/// root of another network must be compared through its tree, since its
/// nodes cache their digests under that network's key.
#[cfg(test)]
impl PartialEq for Root {
    fn eq(&self, other: &Self) -> bool {
        self.same(other, NetworkKeys::fixture().hash())
    }
}

/// Root equality under the fixture keys is an equivalence relation.
#[cfg(test)]
impl Eq for Root {}

/// Clone a tree by sharing its immutable nodes.
impl<T> Clone for Tree<T> {
    /// Clones the root while preserving the payload facade.
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
            keys: Arc::clone(&self.keys),
            payload: PhantomData,
        }
    }
}

impl<T> Tree<T> {
    /// Whether this shares a snapshot's node allocation and causal ceiling.
    ///
    /// A sufficient check for a conditional swap that never hashes under the
    /// replica lock. Equal trees with distinct allocations may fail this check
    /// and require a retry; full content-and-ceiling equality would also be safe.
    pub(crate) fn root_is(&self, snapshot: &Self) -> bool {
        let same_node = match (&self.root.node, &snapshot.root.node) {
            (None, None) => true,
            (Some(ours), Some(theirs)) => ours.ptr_eq(theirs),
            _ => false,
        };
        same_node && self.root.ceiling == snapshot.root.ceiling
    }
}

/// Compare the network, the replicated version set, and the causal frontier.
impl<T> PartialEq for Tree<T> {
    /// Returns whether both trees represent the same replica state of one
    /// network.
    fn eq(&self, other: &Self) -> bool {
        self.keys == other.keys && self.root.same(&other.root, self.hash_key())
    }
}

/// Mark tree equality as an equivalence relation.
impl<T> Eq for Tree<T> {}

/// Format a tree without inspecting its payloads.
impl<T> fmt::Debug for Tree<T> {
    /// Formats the stored tree without requiring a debuggable payload type.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tree").field("root", &self.root).finish()
    }
}

/// An empty tree under the fixture secret, for trees tests build outside any
/// network.
#[cfg(any(test, feature = "test-internals"))]
impl<T> Default for Tree<T> {
    fn default() -> Self {
        Self::new(NetworkKeys::fixture())
    }
}

/// What one [`Tree::act`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Acted {
    /// `false` only if neither content nor history changed; `true` is
    /// conservative, as [`Tree::act`] explains.
    pub changed: bool,
    /// How many leaves the tree held before the batch and holds no longer:
    /// the messages the batch's forgets removed.
    pub removed: usize,
}

/// An action to perform on the tree, locally.
#[derive(Clone, Debug)]
pub enum Action {
    /// Insert a message, stamped with the next tick of the local party.
    Insert(Message),
    /// Forget the live leaf stamped with this version, if the tree holds one.
    Forget(Version),
}

/// The iterator of [`Snapshot::iter`](crate::Snapshot::iter): a lazy
/// depth-first walk over every live message as `(&Version, Arc<T>)`, in
/// unspecified order.
///
/// The version is borrowed from the tree; the payload is an owned handle
/// into the shared storage.
///
/// An [`ExactSizeIterator`] (the live-message count is known up front) and a
/// [`DoubleEndedIterator`]. Once exhausted, it remains exhausted.
pub struct Iter<'a, T>(typed::Iter<'a>, PhantomData<fn() -> T>);

/// Format an iterator without inspecting its remaining payloads.
impl<T> fmt::Debug for Iter<'_, T> {
    /// Formats the number of messages left without inspecting payloads.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Iter")
            .field("remaining", &self.0.len())
            .finish()
    }
}

/// Visit live messages from the front of the tree walk.
impl<'a, T: Send + Sync + 'static> Iterator for Iter<'a, T> {
    type Item = (&'a Version, Arc<T>);

    /// Yields the next live message from the front of the walk.
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(v, m)| (v, m.arc::<T>()))
    }

    /// Returns the exact number of messages left to visit.
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

/// Visit live messages from the back of the same tree walk.
impl<'a, T: Send + Sync + 'static> DoubleEndedIterator for Iter<'a, T> {
    /// Yields the next live message from the back of the walk.
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back().map(|(v, m)| (v, m.arc::<T>()))
    }
}

/// Report the exact number of live messages left to visit.
impl<'a, T: Send + Sync + 'static> ExactSizeIterator for Iter<'a, T> {}

/// Promise that an exhausted tree walk remains exhausted.
impl<'a, T: Send + Sync + 'static> std::iter::FusedIterator for Iter<'a, T> {}

impl<T> Tree<T> {
    /// Creates a new, empty tree of the network with `keys`, carrying the
    /// empty [`Version`].
    ///
    /// A tree owns no party identity: advancing the version is driven by a
    /// [`Party`](before::Party) passed into [`act`](Self::act) by the caller (the
    /// [`Peer`](crate::Peer) that owns the party). Forking a tree is a
    /// plain [`clone`](Clone); any party split happens on the owning
    /// [`Peer`](crate::Peer).
    pub fn new(keys: impl Into<Arc<NetworkKeys>>) -> Self {
        Self::from_root(keys, Root::default())
    }

    /// Wrap an already-built root pair as a typed tree facade of the network
    /// with `keys`: the caller asserts that the root's leaves sit at paths
    /// under those keys, and the payload type they were constructed with.
    pub(crate) fn from_root(keys: impl Into<Arc<NetworkKeys>>, root: Root) -> Self {
        Tree {
            root,
            keys: keys.into(),
            payload: PhantomData,
        }
    }

    /// The keys of the network this tree belongs to.
    pub(crate) fn keys(&self) -> &Arc<NetworkKeys> {
        &self.keys
    }

    /// The keyed hash that places this tree's leaves.
    pub(crate) fn path_key(&self) -> &typed::PathKey {
        self.keys.path()
    }

    /// The keyed hash that digests this tree's nodes: the one key their
    /// memos are filled under.
    pub(crate) fn hash_key(&self) -> &typed::HashKey {
        self.keys.hash()
    }

    /// Returns the tree's causal history, including redacted messages.
    pub fn latest(&self) -> &Version {
        &self.root.ceiling
    }

    /// Returns the meet of the live message versions, or `None` when empty.
    pub fn earliest(&self) -> Option<&Version> {
        self.root.node.as_ref().map(Node::floor)
    }

    /// Returns `true` if the tree holds no messages.
    pub fn is_empty(&self) -> bool {
        self.root.node.is_none()
    }

    /// Returns the number of messages in the tree.
    pub fn len(&self) -> usize {
        self.root.len()
    }

    /// The largest canonical version encoding among every bound the
    /// tree holds — leaf versions and every branch's ceiling and floor —
    /// in bytes; zero for the empty tree.
    ///
    /// A read of a per-node aggregate maintained exactly, memoized like
    /// the bounds it covers: redacting the version that carries the
    /// maximum resizes it down.
    #[cfg(test)]
    pub(crate) fn max_version_bytes(&self) -> usize {
        self.root
            .node
            .as_ref()
            .map(Node::version_bytes)
            .unwrap_or_default()
    }

    /// The largest canonical encoding among every *per-node* version
    /// bound the tree holds, recomputed by direct walk with no aggregate
    /// memo: the oracle [`max_version_bytes`](Self::max_version_bytes)
    /// is pinned against.
    ///
    /// Deliberately excludes the root ceiling riding outside the nodes:
    /// that value is the greeting version, one per tree, priced outside
    /// the per-node memory model. Test instrumentation; see
    /// [`testing::max_bound_bytes`](crate::testing::max_bound_bytes).
    #[cfg(test)]
    pub(crate) fn max_bound_bytes(&self) -> usize {
        self.root
            .node
            .as_ref()
            .map(|node| node.max_bound_bytes())
            .unwrap_or_default()
    }

    /// The root digest of the live version set under the tree's hash key,
    /// excluding the root's causal ceiling.
    ///
    /// Node digests are memoized, so reading a published tree's digest is
    /// cheap.
    #[cfg(any(test, feature = "test-internals"))]
    pub(crate) fn hash(&self) -> [u8; MERKLE_HASH_LEN] {
        #[cfg(test)]
        meter::record_root_hash_read();
        Node::root_hash(self.root.node.as_ref(), self.hash_key()).into()
    }

    /// Finds the live message whose path and stored version match `version`.
    fn message(&self, version: &Version) -> Option<&Message> {
        let path = <[u8; 32]>::from(typed::Path::for_leaf(self.path_key(), version));
        let (stored, message) = self.root.node.as_ref()?.get(&path)?;
        (stored == version).then_some(message)
    }

    /// Returns whether a live leaf carries `version`, without reading its
    /// payload.
    pub(crate) fn contains(&self, version: &Version) -> bool {
        self.message(version).is_some()
    }

    /// Looks up the live message stamped with `version`, by its
    /// version-derived path.
    ///
    /// The stored version is not echoed back: a leaf's path derives from
    /// its version, so the hit's version is the queried one.
    pub fn get(&self, version: &Version) -> Option<Arc<T>>
    where
        T: Send + Sync + 'static,
    {
        self.message(version).map(Message::arc::<T>)
    }

    /// Lazily borrows every live leaf's version without accessing its payload.
    pub(crate) fn versions(
        &self,
    ) -> impl DoubleEndedIterator<Item = &Version> + ExactSizeIterator + Send + Sync {
        self.untyped_iter().map(|(version, _)| version)
    }

    /// Builds the untyped leaf walk shared by the public read methods.
    fn untyped_iter(&self) -> typed::Iter<'_> {
        self.root
            .node
            .as_ref()
            .map(typed::node::Root::iter)
            .unwrap_or_else(typed::Iter::empty)
    }

    /// Compute hashes, version bounds, and encoded-version size before publication.
    ///
    /// Accessors recurse only through cold nodes. Unchanged subtrees retain
    /// their memos, so an incremental commit pays for its new spine. Warming
    /// version sizes also computes the bounds they depend on.
    pub(crate) fn warm_memos(&self) {
        if let Some(root) = &self.root.node {
            root.hash(self.hash_key());
            root.version_bytes();
        }
    }

    /// Lazily iterates every live leaf currently in the tree as
    /// `(&Version, Arc<T>)`, in unspecified order.
    pub fn iter(&self) -> Iter<'_, T>
    where
        T: Send + Sync + 'static,
    {
        Iter(self.untyped_iter(), PhantomData)
    }

    /// Creates an owned walk over the live leaves whose versions the causal
    /// `query` admits.
    ///
    /// The lifetime-free counterpart of [`range`](Self::range), holdable
    /// across awaits and in long-lived state, pinning only its unvisited
    /// frontier. The query's bounds are settled owned
    /// ([`Query::into_owned`](causally::Query::into_owned)), so the walk
    /// carries no lifetime.
    pub fn range_owned<'q, P: causally::Polarity>(
        &self,
        query: impl Into<causally::Query<'q, P>>,
    ) -> RangeOwned<P> {
        typed::node::Root::range_owned(self.root.node.as_ref(), query.into().into_owned())
    }

    /// Lazily iterates the live leaves whose versions the causal `query`
    /// admits.
    ///
    /// Subtrees wholly outside the query are pruned by their memoized
    /// version bounds without being entered, so iterating a small delta
    /// against a large tree costs work proportional to the delta (plus the
    /// pruning frontier), not the tree.
    ///
    /// Unlike [`iter`](Self::iter), not an [`ExactSizeIterator`]: how many
    /// leaves pass is unknown until they are visited.
    pub fn range<'q, P: causally::Polarity>(
        &'q self,
        query: impl Into<causally::Query<'q, P>>,
    ) -> impl DoubleEndedIterator<Item = (&'q Version, Arc<T>)> + Send + Sync
    where
        T: Send + Sync + 'static,
    {
        typed::node::Root::range(self.root.node.as_ref(), query.into())
            // The shared walk yields the full `&Message`; the public
            // contract hands out the owned payload handle, one reference
            // bump on the shared allocation.
            .map(|(v, m)| (v, m.arc::<T>()))
    }

    /// Apply a batch of local insertions and redactions.
    ///
    /// Each action ticks `party` from the running version, initially the tree's
    /// ceiling. Inserts use that version for both their address and their
    /// stored timestamp. Redactions tick too: a deletion must be later than the
    /// message it removes for other replicas to recognize it.
    ///
    /// A single sorted traversal applies the batch. Empty batches, missing-key
    /// redactions, and an insertion cancelled within the same batch leave the
    /// affected nodes and their memos intact. Such keys add no history; the
    /// ceiling joins the last applied version of each remaining affected key.
    /// Splitting a batch can therefore change its resulting versions.
    ///
    /// The result's `changed` is `false` only if neither content nor history
    /// changed. `true` normally means content changed, but is conservative for
    /// malformed stored state: actions older than a resident leaf are skipped
    /// and can still report `true`, without advancing the ceiling. Local ticks
    /// cannot encounter this case when the tree's ceiling bounds all its
    /// leaves. Its `removed` counts the leaves the tree held before the batch
    /// and holds no longer; a forget of a key that holds no leaf, or a second
    /// forget of one key, adds nothing.
    pub fn act<I>(&mut self, party: &before::Party, actions: I) -> Acted
    where
        I: IntoIterator<Item = Action>,
    {
        // Tick in specification order, before sorting by path. This gives
        // every insert a unique address and each forget a later timestamp.
        // A clone of the shared keys, so that borrowing the path key does
        // not hold `self` across the traversal that mutates it.
        let keys = Arc::clone(&self.keys);
        let key = keys.path();
        let mut new_version = self.latest().clone();
        self.react(actions.into_iter().map(|action| {
            new_version.tick(party);
            let version = new_version.clone();

            let (path, edit) = match action {
                Action::Forget(forgotten) => (
                    typed::Path::for_leaf(key, &forgotten),
                    traverse::Action::Forget,
                ),
                Action::Insert(value) => (
                    typed::Path::for_leaf(key, &version),
                    traverse::Action::Insert(value),
                ),
            };
            (path, version, edit)
        }))
    }

    /// Apply `act`'s versioned actions without ticking again.
    ///
    /// Each insert uses its version-derived path; a forget removes that path.
    /// Actions at one path must be causally ascending in specification order,
    /// as `act` guarantees. This is not a general conflict resolver for
    /// concurrent or reordered actions.
    ///
    /// Returns `act`'s result. Panics from traversal or discarded action
    /// payloads leave the tree untouched.
    fn react<I>(&mut self, reactions: I) -> Acted
    where
        I: IntoIterator<Item = (typed::Path, Version, traverse::Action)>,
    {
        // Finish the caller's iterator before starting the traversal. The owned
        // batch also keeps its payloads alive until the walk completes.
        let actions = reactions.into_iter().collect();

        // Work on a shared root and a separate ceiling so any unwind leaves
        // `self` intact. Only keys with an effect contribute history;
        // forgetting a missing key must not advance the ceiling.
        let mut acted = Acted {
            changed: false,
            removed: 0,
        };
        let mut new_ceiling = self.root.ceiling.clone();
        let new_root = traverse::act(
            self.root.node.clone(),
            actions,
            &mut |v: &Version, removed: bool| {
                new_ceiling |= v;
                acted.changed = true;
                acted.removed += usize::from(removed);
            },
        );

        // Publish both fields before releasing displaced payloads. Their
        // destructors may panic, so they must find a consistent tree.
        let pre_image = std::mem::replace(&mut self.root.node, new_root);
        self.root.ceiling = new_ceiling;
        drop(pre_image);
        acted
    }

    /// Merge `other` into this tree, honoring deletions by causal version.
    ///
    /// This is the in-memory oracle for wire reconciliation. A leaf absent
    /// from a side whose ceiling includes its version was deleted there.
    ///
    /// Returns whether the live set changed, as detected by the traversal.
    /// The causal ceiling can advance even when the result is `false`.
    /// Panics during the walk or ceiling merge leave this tree untouched.
    ///
    /// # Panics
    ///
    /// Panics if the trees belong to different networks: their leaves sit at
    /// paths under different keys, so no merge of them means anything. Every
    /// caller joins trees of one replica's network.
    pub fn join(&mut self, other: Tree<T>) -> bool {
        assert!(
            self.keys == other.keys,
            "only trees of one network can be joined"
        );
        let Root {
            ceiling: their_version,
            node: their_root,
        } = other.root;

        // Retain our root while building the candidate: filtering their tree
        // can run payload destructors, and a panic must leave us unchanged.
        // Compute the new ceiling separately for the same reason.
        let our_root = self.root.node.clone();
        let mut changed = false;
        let merged = traverse::join(
            our_root,
            their_root,
            &self.root.ceiling,
            &their_version,
            self.keys.hash(),
            &mut changed,
        );
        let new_ceiling = &self.root.ceiling | their_version;

        // Publish both fields before releasing our old root. Payloads removed
        // by this join may become uniquely owned here; if their destructors
        // panic, they must find the new tree and ceiling consistent.
        let pre_image = std::mem::replace(&mut self.root.node, merged);
        self.root.ceiling = new_ceiling;
        drop(pre_image);
        changed
    }
}

/// Test-only meter for root-hash reads through [`Tree::hash`].
///
/// A read may be answered from the node memos, but a *fresh* tree spine (the
/// copy-on-write path every commit rebuilds) has no memo, so a read inside a
/// commit's critical section re-hashes that spine while the watch lock is
/// held. The pinned tests over this counter (`root_hash_read_meter_is_live`
/// and the commit-path pins beside it in `crate::tests`) enforce how many
/// such reads each commit path performs.
///
/// Thread-local, because every commit critical section runs synchronously on
/// its caller's thread: a test brackets the operation on its own thread and
/// reads a count no concurrent test can perturb.
#[cfg(test)]
pub(crate) mod meter {
    use std::cell::Cell;

    // Clippy's `missing_const_for_thread_local` can reject const-block
    // initializers on targets that lower `thread_local!` through fallback TLS.
    // The allow keeps `-D warnings` clean on those targets.
    thread_local! {
        #[allow(clippy::missing_const_for_thread_local)]
        static ROOT_HASH_READS: Cell<u64> = const { Cell::new(0) };
    }

    /// How many root hashes [`Tree::hash`](super::Tree::hash) has served on
    /// this thread.
    pub(crate) fn root_hash_reads() -> u64 {
        ROOT_HASH_READS.with(Cell::get)
    }

    /// Record one root-hash read on this thread.
    pub(super) fn record_root_hash_read() {
        ROOT_HASH_READS.with(|c| c.set(c.get() + 1));
    }
}

/// Test-only panic injection for the commit critical sections of
/// `Tree::join` and `Tree::react`.
///
/// The fuse-based unwind pins in [`crate::tree::tests`] arm this to make
/// the merge and apply walks unwind mid-commit. [`traverse::join`] and
/// [`traverse::act`] each burn one fuse step at the walk's entry and one
/// per branch-level step, so a fuse armed at `n` unwinds only after `n`
/// earlier fire points ran: deep enough to land after copy-on-write work
/// has begun. The fuse models an internal panic; separate tests use a
/// panicking payload destructor to exercise the caller-reachable unwind path.
///
/// Thread-local for the same reason as [`meter`]: every commit critical
/// section runs synchronously on its caller's thread, so a test arms and
/// burns a fuse no concurrent test can perturb.
#[cfg(test)]
pub(crate) mod panic_injection {
    use std::cell::Cell;

    // Clippy's `missing_const_for_thread_local` can reject const-block
    // initializers on targets that lower `thread_local!` through fallback TLS.
    // The allow keeps `-D warnings` clean on those targets.
    thread_local! {
        #[allow(clippy::missing_const_for_thread_local)]
        static FUSE: Cell<Option<u64>> = const { Cell::new(None) };
    }

    /// Disarms the fuse when dropped; returned by [`arm`] so a pin that
    /// fails before the fuse burns down cannot leak an armed fuse into the
    /// next test on the same thread.
    pub(crate) struct Armed;

    impl Drop for Armed {
        fn drop(&mut self) {
            FUSE.with(|c| c.set(None));
        }
    }

    /// Arms the fuse: after `steps` further [`fire_if_armed`] calls on this
    /// thread, the next one panics (`0` panics on the very next call).
    #[must_use = "dropping the guard disarms the fuse"]
    pub(crate) fn arm(steps: u64) -> Armed {
        FUSE.with(|c| c.set(Some(steps)));
        Armed
    }

    /// Burns one fuse step, panicking when the fuse reaches zero (and
    /// disarming first, so the unwind is one-shot).
    pub(crate) fn fire_if_armed() {
        FUSE.with(|c| match c.get() {
            None => {}
            Some(0) => {
                c.set(None);
                panic!("injected: panic inside the commit critical section");
            }
            Some(n) => c.set(Some(n - 1)),
        });
    }
}

#[cfg(test)]
pub(crate) mod arb;

#[cfg(test)]
mod tests;
