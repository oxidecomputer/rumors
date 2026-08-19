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
//! leaf's version ([`Path::for_leaf`](typed::Path::for_leaf)). Message
//! bytes enter no path and no digest; identity rests on the invariant the
//! protocol already requires everywhere, that no two messages ever share
//! a version. Version addressing buys three properties at once:
//!
//! - **The set is the tree.** Where a leaf lives is fully determined by
//!   the version stamped on it, so two replicas holding the same messages
//!   hold the same tree, regardless of insertion order or which peer sent
//!   what. Union is well-defined node-by-node.
//! - **Equal hash ⟹ equal subtree.** Each node memoizes a Merkle hash of
//!   its subtree — a pure function of the version set, blind to message
//!   bytes — so replicas can prune agreement wholesale: the engine of
//!   the [`mirror`] protocol's divergence-proportional cost. The Merkle
//!   hash is a 24-byte truncation, deliberately narrower than the 32-byte
//!   version address: a comparison signal tolerates truncation that an
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

pub(crate) mod traverse;
pub(crate) mod typed;
pub(crate) mod wire;

use crate::{Version, causally, message::Message, tree::typed::Node};

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
/// 32-byte path is the full-width hash of its version (see
/// [`Path::for_leaf`](typed::Path::for_leaf)). Versions are unique per
/// send — locally by tick, globally by party disjointness — so two
/// content-identical messages sent at distinct moments occupy distinct
/// leaves, and two leaves collide only when a version has been reused,
/// which conforming peers cannot do.
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

/// The empty root: the empty [`Version`] over no nodes. The state a mirror
/// exchange starts from when the local side holds nothing yet: a
/// bootstrapping peer mirrors the provider's tree into it.
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
    /// Forget the leaf at a version-derived path.
    Forget(typed::Path),
}

/// The iterator of [`Snapshot::iter`](crate::Snapshot::iter):
/// a lazy depth-first walk over every live message as
/// `(&Version, &Arc<T>)`, in unspecified order.
///
/// An [`ExactSizeIterator`] (the live-message count is known up front) and a
/// [`DoubleEndedIterator`].
pub struct Iter<'a, T>(typed::Iter<'a, T>);

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (&'a Version, &'a Arc<T>);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(v, m)| (v, m.as_arc()))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back().map(|(v, m)| (v, m.as_arc()))
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

    /// Looks up the live message stamped with `version`, by its
    /// version-derived path.
    pub fn get(&self, version: &Version) -> Option<(&Version, &Arc<T>)> {
        let path = <[u8; 32]>::from(typed::Path::for_leaf(version));
        self.root
            .root
            .as_ref()?
            .get(&path)
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
    /// `(&Version, &Arc<T>)`, in unspecified order.
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
    ) -> impl DoubleEndedIterator<Item = (&'q Version, &'q Arc<T>)> + Send + Sync
    where
        T: Send + Sync,
    {
        typed::node::Root::range(self.root.root.as_ref(), query.into())
            // The shared walk yields the full `&Message<T>`; the public
            // contract hands out only the `&Arc<T>` value, a cheap projection
            // of it.
            .map(|(v, m)| (v, m.as_arc()))
    }

    /// Applies the specified actions as a batch to the tree, advancing its
    /// internal version vector once per action.
    ///
    /// Each [`Action::Insert`] advances the local party's component of the
    /// version vector by one before the leaf's path is derived; the inserts
    /// in a batch are therefore assigned strictly-increasing versions in the
    /// order they appear, so two content-identical messages within a batch
    /// occupy distinct leaves. An [`Action::Forget`] ticks too, so an
    /// effectual forget carries a version strictly greater than any prior
    /// insert (the mirror protocol's deletion-honoring inference depends on
    /// that; see the body comment). A forget that targets the version of
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
    ///
    /// # Errors
    ///
    /// [`traverse::LeafCollision`] if an insert lands on an occupied path
    /// disagreeing on version or payload; the tree is untouched. Paths are
    /// version-derived and each insert's fresh tick strictly dominates the
    /// ceiling bounding every live leaf, so this is unreachable outside a
    /// crate bug or an off-model hash collision — callers `expect` it, and
    /// it is never user-visible ([`traverse::LeafCollision`]).
    pub fn act<I>(
        &mut self,
        party: &before::Party,
        actions: I,
    ) -> Result<bool, traverse::LeafCollision>
    where
        T: Send + Sync,
        I: IntoIterator<Item = Action<T>>,
    {
        // Track the running version across the batch, ticking the owning party
        // once per action so that (a) content-identical messages occupy
        // distinct leaves even when submitted together, and (b) forgets carry a
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
            // independent of our party and current version. The path is
            // derived from the post-tick version, which is unique per
            // insert (see [`typed::Path::for_leaf`]).
            let (path, value) = match action {
                Action::Forget(path) => (path, None),
                Action::Insert(value) => (typed::Path::for_leaf(&version), Some(value)),
            };
            (path, version, value)
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
    /// and version ties. Each item is keyed by its version-derived path, so
    /// if each party only manipulates its own tree through [`Tree::act`],
    /// these conflicts cannot arise.
    ///
    /// As with [`act`](Self::act), a batch is applied in a single traversal,
    /// which is more efficient than applying its actions one at a time but
    /// semantically equivalent.
    ///
    /// Returns whether the effectual-action observer fired at all — the
    /// changed flag [`act`](Self::act) hands out, with the contract stated
    /// there. `false` means no observation and therefore no ceiling
    /// movement either: the tree is untouched. Errors exactly as
    /// [`act`](Self::act) does, with the tree untouched on `Err`.
    fn react<M, I>(&mut self, reactions: I) -> Result<bool, traverse::LeafCollision>
    where
        T: Send + Sync,
        M: Into<Option<Message<T>>>,
        I: IntoIterator<Item = (typed::Path, Version, M)>,
    {
        // Materialize the caller's action stream before the commit section
        // begins: a panicking caller iterator (`act`'s version ticks and key
        // derivations ride the same chain) then surfaces before any traversal
        // work is spent. This is an ordering nicety, not the atomicity
        // mechanism — the commit section below defends against every unwind,
        // this one included. The traversal's root-level radix sort would
        // materialize the stream anyway; collecting up front costs one Vec
        // the radix sort immediately consumes.
        let actions: Vec<_> = reactions
            .into_iter()
            .map(|(path, version, message)| match message.into() {
                None => (path, version, traverse::Action::Forget),
                Some(value) => (path, version, traverse::Action::Insert(value)),
            })
            .collect();

        // Traverse the tree from the root, batch-applying the actions.
        // The version join is deferred to the effectual-action observer so
        // that zero-effect actions (e.g. forgetting a nonexistent key) do not
        // bump the root version. The changed flag rides the same observer:
        // no observation means no leaf was inserted, replaced, or removed
        // and no version was joined, so the tree — hash and ceiling both —
        // is exactly what it was.
        //
        // Panic atomicity: nothing of `self` mutates until the commit point
        // below, whatever the unwind's origin — a user type's destructor or
        // our own bug. Unwind sources survive inside this walk: the leaf
        // level drops causally-skipped action messages and batch-internal
        // displaced inserts mid-walk, and on the wire-apply path those
        // messages are freshly deserialized, so the drop is the last handle
        // and runs `T`'s destructor. The walk is therefore handed an O(1)
        // structural clone of our root (nodes are Arc-shared; the walk
        // copies on write where they stay shared) while the pre-image stays
        // in place, and the observer accumulates the ceiling into a local.
        // What this rules out is an unwind publishing an emptied root under
        // a live ceiling, the byte-for-byte shape of "everything was
        // redacted". `act_unwind_leaves_tree_byte_identical` pins the entry
        // unwind, `act_destructor_unwind_leaves_tree_byte_identical` pins
        // the real mid-walk destructor source, and
        // `act_mid_walk_unwind_leaves_tree_byte_identical` pins an arbitrary
        // internal unwind via the injected fuse.
        let mut changed = false;
        let mut new_ceiling = self.root.ceiling.clone();
        let new_root = traverse::act(self.root.root.clone(), actions, |v: &Version| {
            new_ceiling |= v;
            changed = true;
        })?;

        // The commit point: the walk returned without unwinding or erroring
        // (a leaf-collision error above returns before anything of `self`
        // mutates, the same atomicity as an unwind). Both fields
        // are assigned before the pre-image drops, because that drop runs
        // user code — everything the batch displaced becomes uniquely held
        // here, so its cascading `T` destructors run now, and a panicking
        // destructor must find the tree already consistent. The defense is
        // nothing subtler than statement order: replace, assign, then drop.
        let pre_image = std::mem::replace(&mut self.root.root, new_root);
        self.root.ceiling = new_ceiling;
        drop(pre_image);
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
    ///
    /// # Errors
    ///
    /// [`traverse::LeafCollision`] if the two trees hold leaves at one path
    /// that disagree on version or payload; this tree is untouched (hash,
    /// ceiling, and content all unchanged). Unreachable outside a crate bug
    /// or an off-model hash collision — callers `expect` it, and it is
    /// never user-visible ([`traverse::LeafCollision`]).
    pub fn join(&mut self, other: Tree<T>) -> Result<bool, traverse::LeafCollision>
    where
        T: Send + Sync,
    {
        let Root {
            ceiling: their_version,
            root: their_root,
        } = other.root;

        // Panic atomicity, to the same end as `react`'s commit section:
        // nothing of `self` mutates until the commit point below, whatever
        // the unwind's origin. Unwind sources survive inside this walk
        // (deletion honoring and the duplicate-leaf arm drop the incoming
        // tree's uniquely-held leaves, running `T` destructors), so the
        // pre-image retention is load-bearing: the walk is handed an O(1)
        // structural clone of our root (nodes are Arc-shared; the walk
        // copies on write where they stay shared) while the pre-image stays
        // in place, and the merged ceiling is computed into a local first,
        // because folding in place would stake unwind atomicity on the
        // fold's internal ordering, which no contract states.
        // `join_unwind_leaves_tree_byte_identical` pins the atomicity with
        // an unwind injected mid-walk, after copy-on-write work has begun;
        // `join_destructor_unwind_leaves_tree_byte_identical` pins the real
        // mid-walk destructor source.
        let our_root = self.root.root.clone();
        let mut changed = false;
        let merged = traverse::join(
            our_root,
            their_root,
            &self.root.ceiling,
            &their_version,
            &mut changed,
        )?;
        let new_ceiling = &self.root.ceiling | their_version;

        // The commit point: the walk and the ceiling fold both completed
        // without unwinding or erroring (a leaf-collision error above
        // returns before anything of `self` mutates, the same atomicity as
        // an unwind). Both fields are assigned before the pre-image drops,
        // because that drop runs user code — everything deletion honoring
        // removed from our side becomes uniquely held here, so its
        // cascading `T` destructors run now, and a panicking destructor
        // must find the tree already consistent. The defense is nothing
        // subtler than statement order: replace, assign, then drop.
        let pre_image = std::mem::replace(&mut self.root.root, merged);
        self.root.ceiling = new_ceiling;
        drop(pre_image);
        Ok(changed)
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

/// Test-only panic injection for the commit critical sections of
/// `Tree::join` and `Tree::react`.
///
/// The fuse-based unwind pins in [`crate::tree::tests`] arm this to make
/// the merge and apply walks unwind mid-commit. [`traverse::join`] and
/// [`traverse::act`] each burn one fuse step at the walk's entry and one
/// per branch-level step, so a fuse armed at `n` unwinds only after `n`
/// earlier fire points ran: deep enough to land after copy-on-write work
/// has begun. The fuse stands in for an arbitrary internal bug and proves
/// the defense total; the destructor-source pins beside the fuse pins
/// prove the one *caller*-reachable unwind source (a panicking `T`
/// destructor on a mid-walk last-handle drop) is real.
///
/// Thread-local for the same reason as [`meter`]: every commit critical
/// section runs synchronously on its caller's thread, so a test arms and
/// burns a fuse no concurrent test can perturb.
#[cfg(test)]
pub(crate) mod panic_injection {
    use std::cell::Cell;

    // clippy's `missing_const_for_thread_local` misreads `thread_local!`'s
    // fallback-TLS lowering (illumos among the gate's targets) and denies
    // initializers that already sit in `const` blocks; the allow keeps
    // `-D warnings` honest on every platform the gate runs.
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
