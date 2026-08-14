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
//! queries ([`Tree::range`]), which prune whole subtrees without entering
//! them.
//!
//! # The traversal trio
//!
//! All mutation and reconciliation is three inductive traversals over the
//! same structure ([`traverse`]): [`act`](Tree::act) applies a local batch
//! in one pass; [`join`](Tree::join) merges two in-memory trees;
//! [`mirror`] reconciles two trees over a wire. `join` and `mirror` are
//! observationally identical — both delegate deletion honoring to the same
//! filter — so every convergence property can be tested in-memory and
//! trusted on the wire.

use std::sync::Arc;

mod key;
mod traverse;
pub(crate) mod typed;

use crate::{Version, causally, message::Message, tree::typed::Node};

pub use key::Key;
pub use typed::hash::MERKLE_HASH_LEN;

pub mod mirror;

/// The fully-owned, lifetime-free leaf walk and the leaf handle it yields;
/// the engine beneath [`Rumors::unordered_messages`](crate::Rumors::unordered_messages) and the
/// streams built over it.
pub use typed::{Leaf, RangeOwned};

/// A sparse Merkle radix trie with transparent path compression, whose
/// leaves store versioned [`Message<T>`]s.
///
/// The tree has a branching factor of 256 and a depth of 32, so a leaf's
/// 32-byte path is its content-addressed hash (see
/// [`Path::for_leaf`](typed::Path::for_leaf)). The version is folded into
/// the path, so two content-identical messages inserted at distinct
/// versions occupy distinct leaves; two leaves collide only when they carry
/// the same `(version, value)` pair, which disjoint parties cannot produce.
#[derive(Debug, Eq)]
pub struct Tree<T> {
    pub(crate) root: Root<T>,
}

/// A tree's root pair: the node structure (absent when empty) and the
/// causal ceiling that rides *outside* it.
///
/// The ceiling outlives the nodes — it advances on effectual redactions and
/// survives a tree emptying out — which is exactly what deletion honoring
/// compares against.
#[derive(Debug, Eq)]
pub struct Root<T> {
    ceiling: Version,
    root: Option<typed::node::Root<T>>,
}

impl<T> From<Root<T>> for Option<typed::node::Root<T>> {
    fn from(value: Root<T>) -> Self {
        value.root
    }
}

impl<T> Clone for Root<T> {
    fn clone(&self) -> Self {
        Self {
            ceiling: self.ceiling.clone(),
            root: self.root.clone(),
        }
    }
}

/// The empty root: the empty [`Version`] over no nodes. Lets callers
/// `mem::take` a root out of a `&mut` borrow (e.g. to move it into a mirror
/// exchange and write the merged result back) without an interim clone.
impl<T> Default for Root<T> {
    fn default() -> Self {
        Root {
            ceiling: Version::new(),
            root: None,
        }
    }
}

impl<T> PartialEq for Root<T> {
    fn eq(&self, other: &Self) -> bool {
        self.ceiling == other.ceiling && self.root == other.root
    }
}

impl<T> Clone for Tree<T> {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
        }
    }
}

impl<T> PartialEq for Tree<T> {
    fn eq(&self, other: &Self) -> bool {
        self.root == other.root
    }
}

impl<T> Default for Tree<T> {
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

/// The iterator of [`Snapshot::iter`](crate::Snapshot::iter):
/// a lazy depth-first walk over every live message as
/// `(Key, &Version, &Arc<T>)`, in unspecified order.
///
/// An [`ExactSizeIterator`] (the live-message count is known up front) and a
/// [`DoubleEndedIterator`].
pub struct Iter<'a, T>(typed::Iter<'a, T>);

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (Key, &'a Version, &'a Arc<T>);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(k, v, m)| (k, v, m.as_arc()))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back().map(|(k, v, m)| (k, v, m.as_arc()))
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {}

impl<T> Tree<T> {
    /// Creates a new, empty tree carrying the empty [`Version`].
    ///
    /// A tree owns no party identity: advancing the version is driven by a
    /// [`Party`](before::Party) passed into [`act`](Self::act) by the caller (the
    /// [`Peer`](crate::Peer) that owns the party). Forking a tree is a
    /// plain [`clone`](Clone); any party split happens on the owning
    /// [`Peer`](crate::Peer).
    pub fn new() -> Self {
        Tree {
            root: Root {
                ceiling: Version::new(),
                root: None,
            },
        }
    }

    /// Returns the latest version for the tree.
    pub fn latest(&self) -> &Version {
        &self.root.ceiling
    }

    /// Returns the earliest version present in the tree.
    pub fn earliest(&self) -> Option<&Version> {
        self.root.root.as_ref().map(Node::floor)
    }

    /// Returns `true` if the tree holds no messages.
    pub fn is_empty(&self) -> bool {
        self.root.root.is_none()
    }

    /// Returns the number of messages in the tree.
    pub fn len(&self) -> usize {
        self.root.root.as_ref().map(Node::len).unwrap_or_default()
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
    #[cfg(any(test, feature = "test-internals"))]
    pub(crate) fn max_bound_bytes(&self) -> usize {
        self.root
            .root
            .as_ref()
            .map(|node| node.max_bound_bytes())
            .unwrap_or_default()
    }

    /// Returns the root hash for the tree.
    pub fn hash(&self) -> [u8; MERKLE_HASH_LEN] {
        #[cfg(test)]
        meter::record_root_hash_read();
        Node::root_hash(&self.root.clone().into()).into()
    }

    /// Looks up a single live message by its [`Key`].
    pub fn get(&self, key: &Key) -> Option<(&Version, &Arc<T>)> {
        self.root
            .root
            .as_ref()?
            .get(&key.0)
            .map(|(version, message)| (version, message.as_arc()))
    }

    /// Forces every lazily-memoized structural value — the observable hash
    /// and the ceiling/floor version bounds — for the whole tree.
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
            let _ = root.ceiling();
            let _ = root.floor();
        }
    }

    /// Lazily iterates every live leaf currently in the tree as
    /// `(Key, &Version, &Arc<T>)`, in unspecified order.
    pub fn iter(&self) -> Iter<'_, T>
    where
        T: Send + Sync,
    {
        Iter(
            self.root
                .root
                .as_ref()
                .map(typed::node::Root::iter)
                .unwrap_or_else(typed::Iter::empty),
        )
    }

    /// Freezes a fully-owned walk over the live leaves whose versions the
    /// causal `query` admits.
    ///
    /// The lifetime-free counterpart of [`range`](Self::range), holdable
    /// across awaits and in long-lived state, pinning only its unvisited
    /// frontier. The query's bounds are settled owned
    /// ([`Query::into_owned`](causally::Query::into_owned)), so the walk
    /// carries no lifetime.
    pub fn range_owned<'q, P: causally::Polarity>(
        &self,
        query: impl Into<causally::Query<'q, P>>,
    ) -> RangeOwned<T, P> {
        typed::node::Root::range_owned(self.root.root.as_ref(), query.into().into_owned())
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
    ) -> impl DoubleEndedIterator<Item = (Key, &'q Version, &'q Arc<T>)> + Send + Sync
    where
        T: Send + Sync,
    {
        typed::node::Root::range(self.root.root.as_ref(), query.into())
            // The shared walk yields the full `&Message<T>`; the public
            // contract hands out only the `&Arc<T>` value, a cheap projection
            // of it.
            .map(|(k, v, m)| (k, v, m.as_arc()))
    }

    /// Applies the specified actions as a batch to the tree, advancing its
    /// internal version vector once per action.
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
    /// action on a path wins).
    ///
    /// A batch is applied to the tree in a single traversal, which is more
    /// efficient than applying its actions one at a time: in theory an
    /// O(log n) speedup over one-by-one insertion, in practice about 2-3x
    /// since the log base is 256.
    ///
    /// This function is "morally associative": partitioning a sequence of
    /// actions across multiple `act` calls produces the same tree as a
    /// single `act` over their concatenation, except possibly for the tree's
    /// version when several actions address the same key. In that case the
    /// version is incremented once per changed key, regardless of how many
    /// actions pertain to it.
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
    ///   ceiling — which `act` and `join` both maintain, so every honestly
    ///   built tree qualifies — because each action ticks strictly above the
    ///   ceiling. Only a store poisoned by nonconforming gossip (a leaf
    ///   *above* the ceiling; session ingestion rejects the shape) can
    ///   produce `true` without a hash change, and then the cost is one
    ///   spurious watch wakeup, never a missed one.
    pub fn act<I>(&mut self, party: &before::Party, actions: I) -> bool
    where
        T: Send + Sync,
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
        // its leaf. The reactions flow into `react` lazily; the whole
        // chain materializes only once, at the traversal's radix sort.
        let mut new_version = self.latest().clone();
        self.react(actions.into_iter().map(|action| {
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
        }))
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
    /// so if each party only manipulates its own tree through
    /// [`Tree::act`], these conflicts cannot arise.
    ///
    /// As with [`act`](Self::act), a batch is applied in a single traversal,
    /// which is more efficient than applying its actions one at a time but
    /// semantically equivalent.
    ///
    /// Returns whether the effectual-action observer fired at all — the
    /// changed flag [`act`](Self::act) hands out, with the contract stated
    /// there. `false` means no observation and therefore no ceiling
    /// movement either: the tree is untouched.
    fn react<M, I>(&mut self, reactions: I) -> bool
    where
        T: Send + Sync,
        M: Into<Option<Message<T>>>,
        I: IntoIterator<Item = (Key, Version, M)>,
    {
        // Convert the specified actions, lazily, into the action specification
        // required by the inductive traversal of the tree
        let actions = reactions
            .into_iter()
            .map(|(key, version, message)| match message.into() {
                None => (typed::Path::from(key), version, traverse::Action::Forget),
                Some(value) => (
                    typed::Path::from(key),
                    version,
                    traverse::Action::Insert(value),
                ),
            });

        // Traverse the tree from the root, batch-applying the actions.
        // The version join is deferred to the effectual-action observer so
        // that zero-effect actions (e.g. forgetting a nonexistent key) do not
        // bump the root version. The changed flag rides the same observer:
        // no observation means no leaf was inserted, replaced, or removed
        // and no version was joined, so the tree — hash and ceiling both —
        // is exactly what it was.
        //
        // Panic atomicity: the traversal is fallible (it drains the
        // caller-supplied iterator inside its radix sort), so nothing of
        // `self` mutates until it returns. The pre-image is retained by an
        // O(1) structural clone of the root (nodes are Arc-shared; the
        // traversal copies on write where they stay shared), the observer
        // accumulates the ceiling in a local, and root and ceiling are
        // assigned together at the commit point below — an unwind out of
        // the traversal leaves the tree byte-identical, never an emptied
        // root under a live ceiling (the shape of "everything was
        // redacted"; the unwind pins in `tests` hold this).
        let mut changed = false;
        let mut new_ceiling = self.root.ceiling.clone();
        let new_root = traverse::act(self.root.root.clone(), actions, |v: &Version| {
            new_ceiling |= v;
            changed = true;
        });

        // The commit point: the traversal returned without unwinding.
        self.root.root = new_root;
        self.root.ceiling = new_ceiling;
        changed
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
    pub fn join(&mut self, other: Tree<T>) -> bool
    where
        T: Send + Sync,
    {
        let Root {
            ceiling: their_version,
            root: their_root,
        } = other.root;

        // Panic atomicity: the merge walk is fallible, so nothing of `self`
        // mutates until it returns. The recursion is handed an O(1)
        // structural clone of our root (nodes are Arc-shared; the walk
        // copies on write where they stay shared) while the pre-image stays
        // in place, our ceiling is read in place as the deletion filter,
        // and root and ceiling are assigned together at the commit point
        // below — an unwind out of the walk leaves the tree byte-identical,
        // never an emptied root under a live ceiling (the shape of
        // "everything was redacted"; the unwind pins in `tests` hold this).
        let our_root = self.root.root.clone();
        let mut changed = false;
        let merged = traverse::join(
            our_root,
            their_root,
            &self.root.ceiling,
            &their_version,
            &mut changed,
        );

        // The commit point: the walk returned without unwinding.
        self.root.ceiling |= their_version;
        self.root.root = merged;
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

/// Test-only panic injection for the commit critical sections.
///
/// The panic-atomicity pins in [`crate::tree::tests`] arm this to make a
/// fallible traversal unwind mid-commit; [`traverse::join`] fires it at the
/// start of the merge walk. The injection is one-shot: firing disarms the
/// flag first, so the pin's post-unwind assertions run without re-tripping.
///
/// Thread-local for the same reason as [`meter`]: every commit critical
/// section runs synchronously on its caller's thread, so a test arms and
/// observes a flag no concurrent test can perturb.
#[cfg(test)]
pub(crate) mod panic_injection {
    use std::cell::Cell;

    // clippy's `missing_const_for_thread_local` misreads `thread_local!`'s
    // fallback-TLS lowering (illumos among the gate's targets) and denies
    // initializers that already sit in `const` blocks; the allow keeps
    // `-D warnings` honest on every platform the gate runs.
    thread_local! {
        #[allow(clippy::missing_const_for_thread_local)]
        static ARMED: Cell<bool> = const { Cell::new(false) };
    }

    /// Arms the injection: the next [`fire_if_armed`] on this thread panics.
    pub(crate) fn arm() {
        ARMED.with(|c| c.set(true));
    }

    /// Panics iff armed, disarming first so the unwind is one-shot.
    pub(crate) fn fire_if_armed() {
        if ARMED.with(Cell::get) {
            ARMED.with(|c| c.set(false));
            panic!("injected: panic inside the commit critical section");
        }
    }
}

#[cfg(test)]
pub(crate) mod arb;

#[cfg(test)]
mod tests;
