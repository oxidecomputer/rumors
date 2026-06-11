use std::sync::Arc;

mod key;
mod traverse;
mod typed;

use crate::{message::Message, tree::typed::Node, version::Version};

pub use key::Key;

pub use traverse::mirror;

/// The fully-owned, lifetime-free leaf walk and the leaf handle it yields;
/// the engine beneath [`Broadcast::messages`](crate::Broadcast) and the
/// streams built over it.
pub use typed::{Frozen, Leaf};

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
/// `(Key, &Version, &Arc<T>)`, in unspecified order. An
/// [`ExactSizeIterator`] (the live-message count is known up front) and a
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
    /// [`Party`] passed into [`act`](Self::act) by the caller (the
    /// [`Known`](crate::Known) that owns the party). Forking a tree is a
    /// plain [`clone`](Clone); any party split happens on the owning
    /// [`Known`].
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
    #[allow(unused)]
    pub fn hash(&self) -> [u8; 32] {
        Node::root_hash(&self.root.clone().into()).into()
    }

    /// Look up a single live message by its [`Key`]: one `O(depth)` descent
    /// (the key *is* the leaf's path), never a scan. `None` when no live
    /// message has that key — never inserted, or since redacted.
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
    /// within the causal `range`: the lifetime-free counterpart of
    /// [`range`](Self::range), holdable across awaits and in long-lived
    /// state, pinning only its unvisited frontier.
    pub fn freeze<R>(&self, range: R) -> Frozen<T, R>
    where
        R: std::ops::RangeBounds<Version>,
    {
        typed::node::Root::freeze(self.root.root.as_ref(), range)
    }

    /// Lazily iterate the live leaves whose versions fall within the causal
    /// `range`: a leaf is yielded iff its version is contained in the
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

    /// Get all the values stored at a list of hash paths in the tree.
    ///
    /// A live tree holds at most one leaf per path, so the result has one entry
    /// per requested path that is present, in unspecified order. This filters
    /// the lazy leaf walk rather than descending each path: it is a test-only
    /// helper (no production caller), so simplicity wins over the targeted
    /// descent a hot path would want.
    #[cfg(test)]
    fn get_all<I>(&self, paths: I) -> Vec<(Key, Version, Arc<T>)>
    where
        T: Send + Sync,
        I: IntoIterator<Item = Key>,
    {
        let wanted: std::collections::HashSet<Key> = paths.into_iter().collect();
        self.iter()
            .filter(|(key, _, _)| wanted.contains(key))
            .map(|(key, version, message)| (key, version.clone(), message.clone()))
            .collect()
    }

    /// Get all the values in this tree which are unknown relative to the given
    /// version vector.
    #[cfg(test)]
    pub fn unknown(&self, version: &Version) -> Vec<(Key, Version, Message<T>)>
    where
        T: Send + Sync,
    {
        traverse::unknown(self.root.clone().into(), version)
            .iter()
            .flat_map(|node| node.leaves(typed::Prefix::new()))
            .map(|(k, v, m)| (k, v.clone(), m.clone()))
            .collect()
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
    /// incrementing its internal version vector. In the specified iterator,
    /// `Some(message)` indicates an insert, and `None` indicates that the key
    /// should be forgotten.
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
    pub fn react<M, I>(&mut self, reactions: I)
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
mod test;
