use crate::{Key, Network, Tree, Version};
use std::sync::Arc;

/// A consistent snapshot of a set of rumors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot<T> {
    network: Network,
    tree: Tree<T>,
}

impl<T> Snapshot<T> {
    /// Make a new snapshot.
    pub(crate) fn new(network: Network, tree: Tree<T>) -> Self {
        Self { network, tree }
    }

    /// The latest version of any message ever tracked by this [`Known`].
    pub fn latest(&self) -> &Version {
        self.tree.latest()
    }

    /// The earliest version of any message currently present in this [`Known`], or
    /// `None` if it has never seen a message.
    pub fn earliest(&self) -> Option<&Version> {
        self.tree.earliest()
    }

    /// Determine if there are any current messages in this [`Known`].
    pub fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }

    /// The number of live messages in this [`Known`].
    pub fn len(&self) -> usize {
        self.tree.len()
    }

    /// The observable root hash of this snapshot: a 32-byte digest of its
    /// live content, independent of party identity and insertion order. Two
    /// snapshots with equal hashes hold the same live messages. Gossip
    /// converges on causal versions rather than hashes: peers with equal
    /// hashes but different versions (for example, after an insert that was
    /// then redacted) still run a reconciliation pass.
    pub fn hash(&self) -> [u8; 32] {
        self.tree.hash()
    }

    /// Look up a single live message by its [`Key`]: one `O(depth)` descent
    /// (the key *is* the leaf's content-addressed path), never a scan.
    /// `None` when no live message has that key — never inserted, or since
    /// redacted.
    pub fn get(&self, key: &Key) -> Option<(&Version, &Arc<T>)> {
        self.tree.get(key)
    }

    /// Iterate every message currently [`Known`] as `(Key, &Version, &Arc<T>)`.
    ///
    /// Order is unspecified, and in particular does *not* follow the causal
    /// order: a message may be yielded before another that causally precedes
    /// it. Sort by the yielded [`Version`]s if your application needs an
    /// ordering consistent with causality.
    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (Key, &Version, &Arc<T>)> + DoubleEndedIterator + Send + Sync
    where
        T: Send + Sync,
    {
        self.tree.iter()
    }

    /// Iterate the messages whose [`Version`]s fall within the causal
    /// `range`: a message is yielded iff its version is contained in the
    /// range's end bound and *not* contained in its start bound — a
    /// difference of causal down-sets. Per bound kind, for a message at
    /// version `v`:
    ///
    /// - start [`Unbounded`](std::ops::Bound::Unbounded): nothing excluded;
    ///   [`Excluded(s)`](std::ops::Bound::Excluded): `v <= s` excluded;
    ///   [`Included(s)`](std::ops::Bound::Included): `v < s` excluded, so a
    ///   message at exactly `s` is yielded.
    /// - end [`Unbounded`](std::ops::Bound::Unbounded): everything kept;
    ///   [`Included(e)`](std::ops::Bound::Included): `v <= e` kept;
    ///   [`Excluded(e)`](std::ops::Bound::Excluded): `v < e` kept.
    ///
    /// Because [`Version`]s are partially ordered, a start bound of either
    /// kind keeps versions *concurrent* to it — "everything since `s`"
    /// must not drop other parties' concurrent messages — while an end
    /// bound of either kind drops them: keeping demands containment.
    ///
    /// The [`causally`](crate::causally) constructors name every shape:
    /// `range(causally::since(&s))`,
    /// `range(causally::delta(&s, &e))`,
    /// `range(causally::not_before(&s).known_at(&e))`, and so on.
    /// Plain range syntax also works — `range(&v1..=&v2)`,
    /// `range(&v1..)` — as does any other
    /// [`RangeBounds<Version>`](std::ops::RangeBounds) value, such as a
    /// [`Bound`](std::ops::Bound) tuple.
    ///
    /// Pruning rides the tree's memoized version bounds, so iterating a
    /// small causal delta against a large snapshot costs work proportional
    /// to the delta, not the snapshot. Unlike [`iter`](Self::iter), not an
    /// [`ExactSizeIterator`]: how many messages fall in the range is
    /// unknown until they are visited.
    ///
    /// Order is unspecified, and in particular does *not* follow the causal
    /// order: filtering by versions does not mean yielding in version order,
    /// and a message may be yielded before another that causally precedes
    /// it. Sort by the yielded [`Version`]s if your application needs an
    /// ordering consistent with causality.
    pub fn range<R>(
        &self,
        range: R,
    ) -> impl DoubleEndedIterator<Item = (Key, &Version, &Arc<T>)> + Send + Sync
    where
        T: Send + Sync,
        R: std::ops::RangeBounds<Version> + Send + Sync,
    {
        self.tree.range(range)
    }

    /// Force this set's tree to compute its lazy structural memos (observable
    /// hash and ceiling/floor version bounds), so a subsequent operation is
    /// timed against its own work. For benchmark and test calibration only.
    #[doc(hidden)]
    pub fn warm_caches(&self) {
        self.tree.warm_caches();
    }
}
