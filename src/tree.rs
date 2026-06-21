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
//! Single-child spines are path-compressed away, and the branch hash rule
//! is compression-invariant by construction (a one-child level hashes the
//! same whether materialized or compressed; see
//! [`Hash::branch`](typed::Hash::branch)).
//!
//! # Memos and sharing
//!
//! Nodes are persistent (`imbl::OrdMap` children behind `Arc`), so cloning
//! a tree — every [`Snapshot`](crate::Snapshot), every gossip session's
//! working copy — is O(1) and shares structure; mutation is copy-on-write.
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
mod stream;
mod traverse;
mod typed;

use crate::{Version, message::Message, tree::typed::Node};

pub use key::Key;
pub use typed::hash::MERKLE_HASH_LEN;

pub use traverse::mirror;

/// The fully-owned, lifetime-free leaf walk and the leaf handle it yields;
/// the engine beneath [`Rumors::unordered_messages`](crate::Rumors::unordered_messages) and the
/// streams built over it.
pub use typed::{RangeOwned, Leaf};

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
///
/// A thin shell over the internal leaf walk that projects each leaf's
/// payload down to its `&Arc<T>` value.
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
    /// Create a new, empty tree carrying the empty [`Version`].
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

    /// Get the latest version for the tree.
    pub fn latest(&self) -> &Version {
        &self.root.ceiling
    }

    /// Get the earliest version present in the tree.
    pub fn earliest(&self) -> Option<&Version> {
        self.root.root.as_ref().map(Node::floor)
    }

    /// Determine if this root is empty.
    pub fn is_empty(&self) -> bool {
        self.root.root.is_none()
    }

    /// Get the number of messages in the tree.
    pub fn len(&self) -> usize {
        self.root.root.as_ref().map(Node::len).unwrap_or_default()
    }

    /// Get the root hash for the tree.
    pub fn hash(&self) -> [u8; MERKLE_HASH_LEN] {
        Node::root_hash(&self.root.clone().into()).into()
    }

    /// Look up a single live message by its [`Key`].
    pub fn get(&self, key: &Key) -> Option<(&Version, &Arc<T>)> {
        self.root
            .root
            .as_ref()?
            .get(&key.0)
            .map(|(version, message)| (version, message.as_arc()))
    }

    /// Force every lazily-memoized structural value — the observable hash and
    /// the ceiling/floor version bounds — for the whole tree. Each accessor
    /// recurses, so one call apiece warms the entire subtree.
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

    /// Lazily iterate every live leaf currently in the tree as
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

    /// Freeze a fully-owned walk over the live leaves whose versions fall
    /// within the causal `range`.
    ///
    /// The lifetime-free counterpart of [`range`](Self::range), holdable
    /// across awaits and in long-lived state, pinning only its unvisited
    /// frontier.
    pub fn range_owned<R>(&self, range: R) -> RangeOwned<T, R>
    where
        R: std::ops::RangeBounds<Version>,
    {
        typed::node::Root::range_owned(self.root.root.as_ref(), range)
    }

    /// Lazily iterate the live leaves whose versions fall within the causal
    /// `range`.
    ///
    /// A leaf is yielded iff its version is contained in the
    /// range's end bound and *not* contained in its start bound — a
    /// difference of causal down-sets (see
    /// [`untyped::Range`](typed::untyped::Range) for the
    /// per-bound semantics). Subtrees wholly outside the range are pruned by
    /// their memoized version bounds without being entered, so iterating a
    /// small delta against a large tree costs work proportional to the delta
    /// (plus the pruning frontier), not the tree.
    ///
    /// Unlike [`iter`](Self::iter), not an [`ExactSizeIterator`]: how many
    /// leaves pass is unknown until they are visited.
    pub fn range<R>(
        &self,
        range: R,
    ) -> impl DoubleEndedIterator<Item = (Key, &Version, &Arc<T>)> + Send + Sync
    where
        T: Send + Sync,
        R: std::ops::RangeBounds<Version> + Send + Sync,
    {
        typed::node::Root::range(self.root.root.as_ref(), range)
            // The shared walk yields the full `&Message<T>`; the public
            // contract hands out only the `&Arc<T>` value, a cheap projection
            // of it.
            .map(|(k, v, m)| (k, v, m.as_arc()))
    }

    /// Apply the specified actions as a batch to the tree, advancing its
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
    pub fn act<I>(&mut self, party: &before::Party, actions: I)
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
        let mut new_version = self.latest().clone();

        // Hold one version `Batch` open across the whole run: each `tick`
        // advances the materialized working form in place, and `snapshot` reads
        // the per-action committed version that keys the leaf. This pays the
        // unpack cost once for the batch rather than once per action — a bare
        // `Version::tick` opens and drops its own batch (an unpack and a repack)
        // on every call. The reactions flow into `react` lazily; the whole
        // chain materializes only once, at the traversal's radix sort.
        let mut batch = new_version.batch();
        self.react(actions.into_iter().map(|action| {
            // Advance the version. It must be unique for every action
            // applied to the tree; otherwise the mirror protocol
            // wrongly early-aborts when versions compare equal.
            batch.tick(party);
            let version = batch.snapshot();

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
        }));
    }

    /// Apply the specified *versioned* actions as a batch to the tree without
    /// incrementing its internal version vector.
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
    fn react<M, I>(&mut self, reactions: I)
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
        // bump the root version.
        let root_version = &mut self.root.ceiling;
        self.root.root = traverse::act(self.root.root.take(), actions, |v: &Version| {
            *root_version |= v;
        });
    }

    /// Merge `other` into `self` by a single simultaneous recursion over both
    /// trees.
    ///
    /// This is the in-memory counterpart to mirroring two local trees (see
    /// [`traverse::mirror`]) and is observationally identical to it: it
    /// produces the same merged tree. Deletions are honored by version
    /// dominance: a leaf one side lacks while its version is `<=` that side's
    /// version vector was deleted there and is dropped.
    pub fn join(&mut self, other: Tree<T>)
    where
        T: Send + Sync,
    {
        let Root {
            ceiling: their_version,
            root: their_root,
        } = other.root;

        // Take our root out so the recursion owns it uniquely (structural ops
        // are then plain moves, never `Arc::make_mut` deep-clones); the merged
        // root is written straight back below. Our version stays in place to be
        // read as the deletion filter, then joined with theirs.
        let our_root = std::mem::take(&mut self.root.root);
        let merged = traverse::join(our_root, their_root, &self.root.ceiling, &their_version);

        self.root.ceiling |= their_version;
        self.root.root = merged;
    }
}

#[cfg(test)]
mod arb;

#[cfg(test)]
mod tests;
