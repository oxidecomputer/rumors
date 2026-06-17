use crate::{Key, Network, Version, tree::Tree};
use std::sync::Arc;

/// The iterator of [`Snapshot::iter`], re-exported from the tree internals:
/// every live message as `(Key, &Version, &Arc<T>)`, unspecified order,
/// exact-size and double-ended.
pub use crate::tree::Iter;

/// A consistent point-in-time view of a set of rumors.
///
/// Consistent means atomic: the snapshot holds exactly the live set as of one
/// moment. Taking one ([`Rumors::snapshot`](crate::Rumors::snapshot)) is cheap:
/// it shares structure with the live set rather than copying it, and later
/// changes never show through. Hold it as long as you like; it keeps its
/// messages alive, not the [`Peer`](crate::Peer).
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

    /// The identifier shared by every peer that descends from the same
    /// [`seed`](crate::Peer::seed) as the snapshotted set.
    pub fn network(&self) -> Network {
        self.network
    }

    /// The causal frontier of everything this set has ever done.
    ///
    /// This is the join of the [`Version`] of every send *and every redaction*
    /// it has tracked, not merely the latest live message. Two replicas with
    /// the same [`Network`] and equal `latest` have seen the same history.
    pub fn latest(&self) -> &Version {
        self.tree.latest()
    }

    /// The floor of the *live* messages' versions: every live message's
    /// version contains it.
    ///
    /// Returns `None` when `self.is_empty()` (unlike [`latest`](Self::latest),
    /// which is advanced by all operations and always returns a [`Version`]).
    pub fn earliest(&self) -> Option<&Version> {
        self.tree.earliest()
    }

    /// Whether no live message remains: none ever sent, or every one since
    /// redacted.
    pub fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }

    /// The number of live messages in this snapshot.
    pub fn len(&self) -> usize {
        self.tree.len()
    }

    /// The observable root hash of this snapshot.
    ///
    /// Two snapshots with equal hashes represent the exact same set of messages
    /// and point in causal time.
    pub fn hash(&self) -> [u8; crate::MERKLE_HASH_LEN] {
        self.tree.hash()
    }

    /// Look up a single live message by its [`Key`].
    pub fn get(&self, key: &Key) -> Option<(&Version, &Arc<T>)> {
        self.tree.get(key)
    }

    /// Iterate every live message as `(Key, &Version, &Arc<T>)`.
    ///
    /// Order is unspecified, and in particular does *not* follow the causal
    /// order: a message may be yielded before another that causally precedes
    /// it. Sort by the yielded [`Version`]s if your application needs an
    /// ordering consistent with causality.
    pub fn iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = (Key, &Version, &Arc<T>)> + ExactSizeIterator + Send + Sync
    where
        T: Send + Sync,
    {
        self.tree.iter()
    }

    /// Iterate the messages whose [`Version`]s fall within the causal `range`.
    ///
    /// A message is yielded if and only if its version is contained in the
    /// range's end bound and *not* contained in its start bound. Per bound
    /// kind, for a message at version `v`:
    ///
    /// - start [`Unbounded`](std::ops::Bound::Unbounded): nothing excluded;
    ///   [`Excluded(s)`](std::ops::Bound::Excluded): `v <= s` excluded;
    ///   [`Included(s)`](std::ops::Bound::Included): `v < s` excluded, so a
    ///   message at exactly `s` is yielded.
    /// - end [`Unbounded`](std::ops::Bound::Unbounded): everything kept;
    ///   [`Included(e)`](std::ops::Bound::Included): `v <= e` kept;
    ///   [`Excluded(e)`](std::ops::Bound::Excluded): `v < e` kept.
    ///
    /// Because [`Version`]s are partially ordered, a start bound of either kind
    /// keeps versions *concurrent* to it, while an end bound of either kind
    /// drops them.
    ///
    /// The [`causally`](crate::causally) constructors are an idiomatic way to
    /// specify causal ranges: `range(causally::since(&s))`,
    /// `range(causally::delta(&s, &e))`,
    /// `range(causally::not_before(&s).known_at(&e))`, and so on. Plain range
    /// syntax like `&v1..=&v2`, `&v1..` also works, as does any other
    /// [`RangeBounds<Version>`](std::ops::RangeBounds) value, such as a
    /// tuple of [`Bound`](std::ops::Bound)s.
    ///
    /// Iterating a small causal delta against a large snapshot costs work
    /// proportional to the delta, not the snapshot.
    ///
    /// Unlike [`iter`](Self::iter), this does not produce an
    /// [`ExactSizeIterator`]: how many messages fall in the range is unknown
    /// until they are visited.
    ///
    /// Order of iteration is unspecified, and in particular does *not* follow
    /// the causal order: filtering by versions does not mean yielding in
    /// version order, and a message may be yielded before another that causally
    /// precedes it. Sort by the yielded [`Version`]s if your application needs
    /// an ordering consistent with causality.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::{Peer, causally};
    ///
    /// let rumors = Peer::<String>::seed().into_rumors();
    /// rumors.send("first".to_string());
    /// let then = rumors.snapshot().latest().clone();
    /// rumors.send("second".to_string());
    /// rumors.send("third".to_string());
    ///
    /// let snapshot = rumors.snapshot();
    /// // Everything not already contained in `then`: the two later sends.
    /// assert_eq!(snapshot.range(causally::since(&then)).count(), 2);
    /// // Everything `then` already contained: just the first.
    /// assert_eq!(snapshot.range(causally::known_at(&then)).count(), 1);
    /// // The two compose into the same partition of the live set.
    /// assert_eq!(snapshot.range(causally::all()).count(), 3);
    /// ```
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

impl<'a, T: Send + Sync> IntoIterator for &'a Snapshot<T> {
    type Item = (Key, &'a Version, &'a Arc<T>);
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.tree.iter()
    }
}
