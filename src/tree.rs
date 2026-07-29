//! The content tree: a sparse Merkle radix trie that makes replica
//! difference *observable* and replica union *cheap*.
//!
//! End-user documentation lives in the [crate docs](crate); here we discuss
//! the design.
//!
//! # Shape
//!
//! Branching factor 256, fixed depth 32: a leaf's path is its 32-byte
//! content address, one byte per level, derived from the hash of its
//! `(version, value)` pair ([`Path::for_leaf`](typed::Path::for_leaf)).
//! Content addressing buys three properties at once:
//!
//! - **The set is the tree.** Where a leaf lives is fully determined by
//!   what it is, so two replicas holding the same messages hold the same
//!   tree, regardless of insertion order or which peer sent what. Union is
//!   well-defined node-by-node.
//! - **Equal hash ⟹ equal subtree.** Each node memoizes a Merkle hash of
//!   its subtree, so replicas can prune agreement wholesale — the engine of
//!   the [`mirror`] protocol's divergence-proportional cost. The Merkle
//!   hash is a 16-byte truncation, deliberately narrower than the 32-byte
//!   content address: a comparison signal tolerates truncation that an
//!   identity cannot (see [`typed::Hash`] for the asymmetry argument).
//! - **Uniform spread.** Hashed paths are uniform, so the trie is
//!   expected-balanced with no adversarial input shape; depth bounds are
//!   real bounds.
//!
//! Single-child spines are path-compressed away, and each node's hash is a
//! single preimage committing its compressed prefix and children together
//! (see [`Hash::branch`](typed::Hash::branch)). Hash agreement across
//! peers therefore rests on the tree's *canonical shape*: equal content
//! yields equal compression, by the same ≥ 2-children maximal-compression
//! invariant the node serializer relies on.
//!
//! # Memos and sharing
//!
//! Every node lives behind an `Arc` (its children a sorted radix fan of
//! further handles), so cloning a tree — every
//! [`Snapshot`](crate::Snapshot), every gossip session's working copy — is
//! O(1) and shares structure; mutation is copy-on-write along the touched
//! spine.
//! Each branch lazily memoizes three pure functions of its subtree: the
//! Merkle **hash** (mirror pruning), and the **ceiling** and **floor** of
//! its leaves' versions. The version bounds power both deletion honoring
//! (a subtree whose ceiling the counterparty's version contains holds
//! nothing it is missing — see [`traverse::unknown`]) and causal range
//! queries ([`Tree::range`]), which prune whole subtrees without
//! entering them.
//!
//! # The traversal trio
//!
//! All mutation and reconciliation is three inductive traversals over the
//! same structure ([`traverse`]): act ([`react`](Tree::react), applying
//! the reactions [`assign`](Tree::assign) stamps) applies a local batch
//! in one pass; [`join`](Tree::join) merges two in-memory trees;
//! [`mirror`] reconciles two trees over a wire. `join` and `mirror` are
//! observationally identical — both delegate deletion honoring to the same
//! filter — so every convergence property can be tested in-memory and
//! trusted on the wire.

use std::sync::Arc;

pub(crate) mod backend;
mod key;
mod traverse;
pub(crate) mod typed;

use futures::Stream;

use crate::{Version, message::Message};
use backend::{Leaf as _, Local, Node as _, Store, VersionBounds};
use typed::height::Z;

pub use key::Key;
pub use typed::hash::MERKLE_HASH_LEN;

pub mod mirror;

/// A tree's root pair, concretely typed at a backend: the node structure
/// (absent when empty) and the causal ceiling that rides *outside* it.
///
/// The ceiling outlives the nodes — it advances on effectual redactions and
/// survives a tree emptying out — which is exactly what deletion honoring
/// compares against.
pub(crate) type Root<T, S = Local> = backend::Root<S, T>;

/// A sparse Merkle radix trie with transparent path compression, whose
/// leaves store versioned [`Message<T>`]s, resident in a [`Store`] backend.
///
/// The tree has a branching factor of 256 and a depth of 32, so a leaf's
/// 32-byte path is its content-addressed hash (see
/// [`Path::for_leaf`](typed::Path::for_leaf)). The version is folded into
/// the path, so two content-identical messages inserted at distinct
/// versions occupy distinct leaves; two leaves collide only when they carry
/// the same `(version, value)` pair, which disjoint parties cannot produce.
///
/// The tree carries its backend handle: every operation that touches node
/// structure routes through the backend's [`Store`] seams, so one `Tree`
/// type serves the in-memory backend (whose seams are the synchronous
/// engines behind immediately-ready futures) and any storage-owning
/// backend alike. Cloning shares the backend handle and the root by
/// pointer, exactly as cheaply as before the backend rode along.
pub struct Tree<T: Send + Sync + 'static, S: Store<T> = Local> {
    pub(crate) backend: S,
    pub(crate) root: Root<T, S>,
}

/// A summary view (frontier and live count), independent of the backend's
/// and payload's own `Debug`: nodes are not printed.
impl<T: Send + Sync + 'static, S: Store<T>> std::fmt::Debug for Tree<T, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tree")
            .field("latest", self.latest())
            .field("len", &self.len())
            .finish_non_exhaustive()
    }
}

impl<T: Send + Sync + 'static, S: Store<T>> Clone for Tree<T, S> {
    fn clone(&self) -> Self {
        Self {
            backend: self.backend.clone(),
            root: self.root.clone(),
        }
    }
}

impl<T: Send + Sync + 'static, S: Store<T>> PartialEq for Tree<T, S> {
    fn eq(&self, other: &Self) -> bool {
        self.root == other.root
    }
}

impl<T: Send + Sync + 'static, S: Store<T>> Eq for Tree<T, S> {}

impl<T: Send + Sync + 'static, S: Store<T> + Default> Default for Tree<T, S> {
    fn default() -> Self {
        Self::new()
    }
}

/// An action to perform on the tree, locally.
#[derive(Clone, Debug)]
pub enum Action<T> {
    /// Insert some value, tagged at the current version by your own party.
    Insert(Message<T>),
    /// Forget the value corresponding to a hash.
    Forget(Key),
}

/// A lazy depth-first walk over every live message as
/// `(Key, &Version, &Arc<T>)`, in unspecified order: the borrowing test
/// oracle the owned public walks are pinned against.
///
/// An [`ExactSizeIterator`] (the live-message count is known up front) and a
/// [`DoubleEndedIterator`].
#[cfg(test)]
pub struct Iter<'a, T>(typed::Iter<'a, T>);

#[cfg(test)]
impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (Key, &'a Version, &'a Arc<T>);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(k, v, m)| (k, v, m.as_arc()))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

#[cfg(test)]
impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back().map(|(k, v, m)| (k, v, m.as_arc()))
    }
}

#[cfg(test)]
impl<'a, T> ExactSizeIterator for Iter<'a, T> {}

impl<T: Send + Sync + 'static, S: Store<T>> Tree<T, S> {
    /// Creates a new, empty tree carrying the empty [`Version`], resident
    /// in `backend`.
    ///
    /// A tree owns no party identity: advancing the version is driven by a
    /// [`Party`](before::Party) passed into [`assign`](Self::assign) by the caller (the
    /// [`Peer`](crate::Peer) that owns the party). Forking a tree is a
    /// plain [`clone`](Clone); any party split happens on the owning
    /// [`Peer`](crate::Peer).
    pub fn new_in(backend: S) -> Self {
        Tree {
            backend,
            root: Root::default(),
        }
    }

    /// [`new_in`](Self::new_in) with the backend defaulted: the shape the
    /// in-memory backend (a zero-sized handle) is always built through.
    pub fn new() -> Self
    where
        S: Default,
    {
        Self::new_in(S::default())
    }

    /// Returns the latest version for the tree.
    pub fn latest(&self) -> &Version {
        &self.root.ceiling
    }

    /// Returns the earliest version present in the tree, owned: the
    /// backend's span accessor mints its bounds per read, so no
    /// node-lifetime borrow exists to hand out.
    pub fn earliest(&self) -> Option<Version> {
        self.root
            .root
            .as_ref()
            .map(|node| node.span().meet().clone())
    }

    /// Returns `true` if the tree holds no messages.
    pub fn is_empty(&self) -> bool {
        self.root.root.is_none()
    }

    /// Returns the number of messages in the tree.
    pub fn len(&self) -> usize {
        self.root
            .root
            .as_ref()
            .map(backend::Node::len)
            .unwrap_or_default()
    }

    /// The largest canonical version encoding among every bound the
    /// tree holds — leaf versions and every branch's ceiling and floor —
    /// in bytes; zero for the empty tree.
    ///
    /// A read of a per-node aggregate maintained exactly, memoized like
    /// the bounds it covers: redacting the version that carries the
    /// maximum resizes it down.
    #[cfg(any(test, feature = "test-internals"))]
    pub(crate) fn max_version_bytes(&self) -> usize {
        self.root
            .root
            .as_ref()
            .map(backend::Node::version_bytes)
            .unwrap_or_default()
    }

    /// Returns the root hash for the tree.
    pub fn hash(&self) -> [u8; MERKLE_HASH_LEN] {
        #[cfg(test)]
        meter::record_root_hash_read();
        self.root
            .root
            .as_ref()
            .map(backend::Node::hash)
            .unwrap_or_else(typed::Hash::empty_root)
            .into()
    }

    /// Looks up a single live message by its [`Key`], returning its
    /// version and shared payload.
    pub async fn get(&self, key: &Key) -> Result<Option<(Version, Arc<T>)>, S::Error> {
        Ok(self
            .backend
            .clone()
            .get(self.root.root.clone(), typed::Path::from(*key))
            .await?
            .map(|leaf| (leaf.span().join().clone(), leaf.message().as_arc().clone())))
    }

    /// Streams the live leaves whose versions fall within `bounds`, in
    /// ascending path order, as bare leaf handles keyed by their [`Key`]s.
    ///
    /// Subtrees wholly outside the range are pruned by their resident
    /// version bounds without being fetched, so streaming a small causal
    /// delta against a large tree costs work proportional to the delta.
    pub(crate) fn range(
        &self,
        bounds: VersionBounds,
    ) -> impl Stream<Item = Result<(Key, S::Node<Z>), S::Error>> + Send + 'static {
        self.backend.clone().range(self.root.root.clone(), bounds)
    }

    /// Forces every lazily-memoized structural value — the observable hash
    /// and the version-bounds span — for the whole tree.
    ///
    /// Each accessor recurses, so one call apiece warms the entire subtree.
    ///
    /// For benchmark and test calibration only: it lets a subsequent operation
    /// be timed against its own work rather than this one-time memoization. In
    /// production these warm naturally as the tree is hashed for the wire and
    /// reconciled against peers.
    #[doc(hidden)]
    pub fn warm_caches(&self) {
        if let Some(root) = &self.root.root {
            let _ = root.hash();
            let _ = root.span();
        }
    }

    /// Stamps each action with its committed version and key against the
    /// tree's current frontier: the synchronous *prep* step of a commit.
    ///
    /// Each [`Action::Insert`] advances the local party's component of the
    /// version vector by one before the leaf's path is derived; the inserts
    /// in a batch are therefore assigned strictly-increasing versions in the
    /// order they appear, and two content-identical messages within a batch
    /// receive distinct keys. An [`Action::Forget`] ticks too, so an
    /// effectual forget carries a version strictly greater than any prior
    /// insert (the mirror protocol's deletion-honoring inference depends on
    /// that; see the body comment). A forget that targets a key derived from
    /// an earlier insert in the same batch overrides that insert (last
    /// action on a path wins, once [`react`](Self::react) applies them).
    ///
    /// A commit built this way is "morally associative": partitioning a
    /// sequence of actions across multiple commits produces the same tree
    /// as a single commit over their concatenation, except possibly for
    /// the tree's version when several actions address the same key. In
    /// that case the version is incremented once per changed key,
    /// regardless of how many actions pertain to it.
    ///
    /// Split from [`react`](Self::react) (the build step) so a committer
    /// can run the build outside the critical section that read the
    /// frontier.
    ///
    /// Reads the frontier and the party; mutates nothing. The stamped
    /// versions are only as fresh as the frontier they were read against:
    /// the committer must hold the `(party, frontier)` pair stable (by
    /// holding the commit lock — see `Peer::commit` — or by staying inside
    /// one `watch` critical section) from `assign` through `react`'s
    /// publication.
    pub(crate) fn assign<I>(
        &self,
        party: &before::Party,
        actions: I,
    ) -> Vec<(Key, Version, Option<Message<T>>)>
    where
        I: IntoIterator<Item = Action<T>>,
    {
        // Track the running version across the batch, ticking the owning party
        // once per action so that (a) content-identical messages produce
        // distinct keys even when submitted together, and (b) forgets carry a
        // version strictly greater than any prior insert at this party. The
        // strict tick on forgets is required by the mirror protocol's
        // deletion-honoring inference, which cannot distinguish "forgot it"
        // from "never had it" when versions are equal. An empty batch is a
        // complete no-op.
        // The running version, advanced in place per action; each action
        // clones the post-tick value as the committed version that keys
        // its leaf.
        let mut new_version = self.latest().clone();
        actions
            .into_iter()
            .map(|action| {
                // Advance the version. It must be unique for every action
                // applied to the tree; otherwise the mirror protocol
                // wrongly early-aborts when versions compare equal.
                new_version.tick(party);
                let version = new_version.clone();

                // Convert unversioned, unlocalized actions into reactions
                // independent of our party and current version. The key is
                // derived from the post-tick version, which is unique per
                // insert (see [`typed::Path::for_leaf`]).
                let (key, value) = match action {
                    Action::Forget(hash) => (hash, None),
                    Action::Insert(value) => {
                        let key = typed::Path::for_leaf(&version, value.bytes()).into();
                        (key, Some(value))
                    }
                };
                (key, version, value)
            })
            .collect()
    }

    /// Applies the specified *versioned* actions as a batch to the tree
    /// without incrementing its internal version vector.
    ///
    /// In the specified iterator, `Some(message)` indicates an insert, and
    /// `None` indicates that the key should be forgotten.
    ///
    /// If multiple actions refer to the same leaf of the tree, the causally
    /// latest action wins, with order of specification breaking concurrency
    /// and version ties. Each item is keyed by its version and content hash,
    /// so if each party only stamps actions against its own tree through
    /// [`assign`](Self::assign), these conflicts cannot arise.
    ///
    /// A batch is applied in a single traversal, which is more efficient
    /// than applying its actions one at a time but semantically
    /// equivalent: in theory an O(log n) speedup over one-by-one
    /// insertion, in practice about 2-3x since the log base is 256.
    ///
    /// The *build* step of a commit; [`assign`](Self::assign) is the prep
    /// step that stamps the reactions, and states the frontier-stability
    /// obligation between the two.
    ///
    /// # The changed flag
    ///
    /// Returns whether the batch changed the tree, so the caller can answer
    /// "did anything happen?" without reading the root hash — the answer the
    /// traversal's effectual-action observer already produced. The two
    /// directions carry different promises:
    ///
    /// - **`false` is exact**: the root hash is byte-identical to what it was
    ///   before the call, and the causal ceiling did not move. Nothing about
    ///   the tree changed. A watcher skipped on `false` misses nothing.
    /// - **`true` is conservative**: the tree changed *or* an action was
    ///   silently skipped as causally prior to the leaf it targeted. The skip
    ///   is unreachable when every leaf's version is bounded by the tree's
    ///   ceiling — which the commit paths and `join` both maintain, so every
    ///   honestly built tree qualifies — because each action ticks strictly
    ///   above the ceiling. Only a store poisoned by nonconforming gossip (a
    ///   leaf *above* the ceiling; session ingestion rejects the shape) can
    ///   produce `true` without a hash change, and then the cost is one
    ///   spurious watch wakeup, never a missed one.
    pub(crate) async fn react<M, I>(&mut self, reactions: I) -> Result<bool, S::Error>
    where
        M: Into<Option<Message<T>>>,
        I: IntoIterator<Item = (Key, Version, M)>,
    {
        // Convert the specified actions into the action specification
        // required by the traversal seam. Materialized: the seam takes a
        // `Vec` so the backend's tower monomorphizes over one flat item
        // type, not this call site's iterator chain.
        let actions: Vec<_> = reactions
            .into_iter()
            .map(|(key, version, message)| match message.into() {
                None => (
                    typed::Path::from(key),
                    version,
                    backend::Action::<T>::Forget,
                ),
                Some(value) => (
                    typed::Path::from(key),
                    version,
                    backend::Action::Insert(value),
                ),
            })
            .collect();

        // Apply the batch through the backend's seam, taking the root out
        // so the traversal owns it uniquely (structural ops are then plain
        // moves, never copy-on-write deep-clones). The version join is
        // deferred to the effectual-action observer so that zero-effect
        // actions (e.g. forgetting a nonexistent key) do not bump the root
        // version. The changed flag rides the same observer: no observation
        // means no leaf was inserted, replaced, or removed and no version
        // was joined, so the tree — hash and ceiling both — is exactly what
        // it was.
        let Root { ceiling, root } = &mut self.root;
        let taken = root.take();
        let mut changed = false;
        *root = self
            .backend
            .clone()
            .act(taken, actions, |v: &Version| {
                *ceiling |= v;
                changed = true;
            })
            .await?;
        Ok(changed)
    }

    /// Merges `other` into `self` by a single simultaneous recursion over
    /// both trees.
    ///
    /// This is the in-memory counterpart to [`mirror::streaming`] and is
    /// observationally identical to it: it produces the same merged tree.
    /// Deletions are honored by version dominance: a leaf one side lacks
    /// while its version is `<=` that side's version vector was deleted
    /// there and is dropped.
    ///
    /// Both trees must be resident in the same backend instance
    /// ([`Store::join`] states why); the one production caller merges a
    /// root that a mirror session materialized into this peer's own
    /// backend.
    ///
    /// # The changed flag
    ///
    /// Returns whether the merge changed this tree's *content* — exactly
    /// whether the root hash moved, decided by the traversal itself (each
    /// leaf gained is a gain the recursion sees; each leaf dropped by
    /// deletion honoring moves a node's exact leaf count) rather than by
    /// hashing. `false` means the root hash is byte-identical to what it was
    /// before the call; `true` means it differs.
    ///
    /// The flag deliberately does *not* cover the causal ceiling, which can
    /// advance without any content change (absorbing the frontier of a peer
    /// whose every message we already hold or honor as deleted): the flag
    /// answers for what observers of the *set* can see, and a ceiling-only
    /// join leaves the set untouched.
    pub(crate) async fn join(&mut self, other: Tree<T, S>) -> Result<bool, S::Error> {
        let Root {
            ceiling: their_version,
            root: their_root,
        } = other.root;

        // Take our root out so the recursion owns it uniquely (structural ops
        // are then plain moves, never copy-on-write deep-clones); the merged
        // root is written straight back below. Our version stays in place to be
        // read as the deletion filter, then joined with theirs.
        let our_root = std::mem::take(&mut self.root.root);
        let mut changed = false;
        let merged = self
            .backend
            .clone()
            .join(
                our_root,
                their_root,
                &self.root.ceiling,
                &their_version,
                &mut changed,
            )
            .await?;

        self.root.ceiling |= their_version;
        self.root.root = merged;
        Ok(changed)
    }

    /// Persist the canonical root and identity clock through the backend's
    /// [`Store::commit`] seam: the step a root-replacing committer runs
    /// after its build and before its publish.
    pub(crate) async fn persist(&self, clock: Option<before::Clock>) -> Result<(), S::Error> {
        self.backend.commit(&self.root, clock).await
    }
}

/// The in-memory oracles: borrowing walks and one-call commits, pinned at
/// the backend whose engines they exercise directly.
#[cfg(test)]
impl<T: Send + Sync + 'static> Tree<T, Local> {
    /// Applies the specified actions as one batch commit:
    /// [`assign`](Self::assign) composed with [`react`](Self::react) in
    /// place, for the tests and oracles that need a one-call commit.
    ///
    /// Returns [`react`](Self::react)'s changed flag; the two-direction
    /// contract is stated there.
    pub fn act<I>(&mut self, party: &before::Party, actions: I) -> bool
    where
        I: IntoIterator<Item = Action<T>>,
    {
        let reactions = self.assign(party, actions);
        self.react_now(reactions)
    }

    /// [`react`](Self::react) driven to completion synchronously: the
    /// in-memory backend's seams are immediate, so tests and oracles can
    /// build without an executor. Returns [`react`](Self::react)'s
    /// changed flag.
    pub fn react_now<M, I>(&mut self, reactions: I) -> bool
    where
        M: Into<Option<Message<T>>>,
        I: IntoIterator<Item = (Key, Version, M)>,
    {
        use futures::FutureExt as _;
        self.react(reactions)
            .now_or_never()
            .expect("the in-memory backend's seams are immediate")
            .unwrap_or_else(|e| match e {})
    }

    /// [`get`](Self::get) driven to completion synchronously, unwrapping
    /// the in-memory backend's uninhabited error.
    pub fn get_now(&self, key: &Key) -> Option<(Version, Arc<T>)> {
        use futures::FutureExt as _;
        self.get(key)
            .now_or_never()
            .expect("the in-memory backend's seams are immediate")
            .unwrap_or_else(|e| match e {})
    }

    /// [`join`](Self::join) driven to completion synchronously: the
    /// in-memory backend's seams are immediate, so tests and oracles can
    /// merge without an executor. Returns [`join`](Self::join)'s changed
    /// flag.
    pub fn join_now(&mut self, other: Tree<T, Local>) -> bool {
        use futures::FutureExt as _;
        self.join(other)
            .now_or_never()
            .expect("the in-memory backend's seams are immediate")
            .unwrap_or_else(|e| match e {})
    }

    /// Lazily iterates the live leaves whose versions fall within the
    /// causal `range`, borrowed: the oracle the owned range walks are
    /// pinned against.
    pub fn range_oracle<R>(
        &self,
        range: R,
    ) -> impl DoubleEndedIterator<Item = (Key, &Version, &Arc<T>)> + Send + Sync
    where
        R: std::ops::RangeBounds<Version> + Send + Sync,
    {
        typed::node::Root::range(self.root.root.as_ref(), range)
            // The shared walk yields the full `&Message<T>`; the oracle
            // hands out only the `&Arc<T>` value, a cheap projection of it.
            .map(|(k, v, m)| (k, v, m.as_arc()))
    }

    /// Lazily iterates every live leaf currently in the tree as
    /// `(Key, &Version, &Arc<T>)`, in unspecified order: the borrowing
    /// oracle the owned walks are pinned against.
    pub fn iter(&self) -> Iter<'_, T> {
        Iter(
            self.root
                .root
                .as_ref()
                .map(typed::node::Root::iter)
                .unwrap_or_else(typed::Iter::empty),
        )
    }
}

#[cfg(any(test, feature = "test-internals"))]
impl<T: Send + Sync + 'static> Tree<T, Local> {
    /// The largest canonical encoding among every *per-node* version
    /// bound the tree holds, recomputed by direct walk with no aggregate
    /// memo: the oracle [`max_version_bytes`](Self::max_version_bytes)
    /// is pinned against.
    ///
    /// Deliberately excludes the root ceiling riding outside the nodes:
    /// that value is the greeting version, one per tree, priced outside
    /// the per-node memory model. Test instrumentation; see
    /// [`testing::max_bound_bytes`](crate::testing::max_bound_bytes).
    pub(crate) fn max_bound_bytes(&self) -> usize {
        self.root
            .root
            .as_ref()
            .map(|node| node.max_bound_bytes())
            .unwrap_or_default()
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

    // clippy's `missing_const_for_thread_local` misreads `thread_local!`'s
    // fallback-TLS lowering (illumos among the gate's targets) and denies
    // initializers that already sit in `const` blocks; the allow keeps
    // `-D warnings` honest on every platform the gate runs.
    thread_local! {
        #[allow(clippy::missing_const_for_thread_local)]
        static ROOT_HASH_READS: Cell<u64> = const { Cell::new(0) };
    }

    /// How many root hashes [`Tree::hash`](super::Tree::hash) has served on
    /// this thread.
    pub(crate) fn root_hash_reads() -> u64 {
        ROOT_HASH_READS.with(Cell::get)
    }

    pub(super) fn record_root_hash_read() {
        ROOT_HASH_READS.with(|c| c.set(c.get() + 1));
    }
}

#[cfg(test)]
pub(crate) mod arb;

#[cfg(test)]
mod tests;
