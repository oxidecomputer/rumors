use crate::{Key, Network, Version, causally, tree::Tree};
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
    /// Makes a new snapshot.
    pub(crate) fn new(network: Network, tree: Tree<T>) -> Self {
        Self { network, tree }
    }

    /// The snapshotted tree, for crate-internal instruments.
    #[cfg(any(test, feature = "test-internals"))]
    pub(crate) fn tree(&self) -> &Tree<T> {
        &self.tree
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

    /// Looks up a single live message by its [`Key`].
    pub fn get(&self, key: &Key) -> Option<(&Version, &Arc<T>)> {
        self.tree.get(key)
    }

    /// Iterates every live message as `(Key, &Version, &Arc<T>)`.
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

    /// Iterates the messages whose [`Version`]s the causal `query` admits.
    ///
    /// The query is anything [`Into`] a [`causally::Query`]: an expression
    /// built from the [`causally`] vocabulary (`range(causally::since(&s))`,
    /// `range(causally::delta(&s, &e))`, `range(causally::after(&s) &
    /// causally::before(&e))`, ...), a [`Span`](crate::causally::Span), or a
    /// [`Version`] (the singleton query admitting exactly that version).
    ///
    /// Iterating a small causal delta against a large snapshot costs work
    /// proportional to the delta, not the snapshot.
    ///
    /// Unlike [`iter`](Self::iter), this does not produce an
    /// [`ExactSizeIterator`]: how many messages the query admits is unknown
    /// until they are visited.
    ///
    /// As with [`iter`](Self::iter), order is unspecified and does *not*
    /// follow the causal order: filtering by versions does not mean yielding
    /// in version order. Sort by the yielded [`Version`]s if your application
    /// needs an ordering consistent with causality.
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
    /// assert_eq!(snapshot.range(causally::before(&then)).count(), 1);
    /// // The two partition the live set.
    /// assert_eq!(snapshot.range(causally::all()).count(), 3);
    /// ```
    pub fn range<'q, P: causally::Polarity>(
        &'q self,
        query: impl Into<causally::Query<'q, P>>,
    ) -> impl DoubleEndedIterator<Item = (Key, &'q Version, &'q Arc<T>)> + Send + Sync
    where
        T: Send + Sync,
    {
        self.tree.range(query)
    }

    /// Forces this set's tree to compute its lazy structural memos (observable
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
