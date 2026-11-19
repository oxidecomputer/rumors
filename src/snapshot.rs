use std::{fmt, sync::Arc};

#[cfg(any(test, feature = "test-internals"))]
use crate::tree::MERKLE_HASH_LEN;
use crate::{Network, Version, causally, tree::Tree};

pub use crate::tree::Iter;

/// An atomic point-in-time view of the content in a set of
/// [`Peer`](crate::Peer).
///
/// Taking one via [`Peer::snapshot`](crate::Peer::snapshot) is cheap: it
/// shares structure rather than copying, and can be kept alive independently of
/// its underlying [`Peer`](crate::Peer).
pub struct Snapshot<T: Send + Sync + 'static> {
    /// The immutable live set and causal frontier, with its network's keys.
    tree: Tree<T>,
}

impl<T: Send + Sync + 'static> Snapshot<T> {
    /// Makes a new snapshot.
    pub(crate) fn new(tree: Tree<T>) -> Self {
        Self { tree }
    }

    /// The snapshotted tree, for crate-internal instruments.
    #[cfg(any(test, feature = "test-internals"))]
    pub(crate) fn tree(&self) -> &Tree<T> {
        &self.tree
    }

    /// Return the gossip network to which this snapshot belongs.
    pub fn network(&self) -> Network {
        self.tree.keys().network()
    }

    /// The latest causal version of everything this set has seen.
    ///
    /// Two replicas with the same [`Network`] and equal `latest` have seen the
    /// same history of sent messages and redactions.
    pub fn latest(&self) -> &Version {
        self.tree.latest()
    }

    /// The floor of the messages' versions in this snapshot: every message's
    /// version contains it.
    ///
    /// Returns `None` when `self.is_empty()`.
    pub fn earliest(&self) -> Option<&Version> {
        self.tree.earliest()
    }

    /// Whether no message remains in the set of messages in this snapshot.
    pub fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }

    /// The number of messages in this snapshot.
    pub fn len(&self) -> usize {
        self.tree.len()
    }

    /// Returns the digest of the Merkle root over the live messages.
    ///
    /// This hash is equal for any two replicas of one network that hold equal
    /// live sets. Hash equality *does not* imply causal equality:
    /// [`Snapshot::latest`] may differ while [`Snapshot::hash`] is the same, if
    /// one snapshot has sent and then redacted messages.
    #[cfg(any(test, feature = "test-internals"))]
    pub fn hash(&self) -> [u8; MERKLE_HASH_LEN] {
        self.tree.hash()
    }

    /// Returns whether `version` identifies a live message in the snapshot.
    pub fn contains(&self, version: &Version) -> bool {
        self.tree.contains(version)
    }

    /// Looks up the message stamped with `version`.
    ///
    /// Returns `None` when no message carries this version.
    pub fn get(&self, version: &Version) -> Option<Arc<T>> {
        self.tree.get(version)
    }

    /// Iterates the versions of every message without accessing the messages
    /// themselves.
    ///
    /// Order is unspecified, and in particular does *not* follow causal order:
    /// a message may be yielded before another that causally precedes it. Sort
    /// the result by [`Version::rank`] (i.e. using
    /// [`sort_by_cached_key`](slice::sort_by_cached_key)) if you want an order
    /// that places causes before their effects.
    pub fn versions(
        &self,
    ) -> impl DoubleEndedIterator<Item = &Version> + ExactSizeIterator + Send + Sync {
        self.tree.versions()
    }

    /// Iterates every message as `(&Version, Arc<T>)`.
    ///
    /// Order is unspecified, and in particular does *not* follow causal order:
    /// a message may be yielded before another that causally precedes it. Sort
    /// the result by [`Version::rank`] (i.e. using
    /// [`sort_by_cached_key`](slice::sort_by_cached_key)) if you want an order
    /// that places causes before their effects.
    ///
    /// ```
    /// let peer = rumors::Peer::<String>::seed();
    /// peer.send("first".into())?;
    /// peer.send("second".into())?;
    /// let snapshot = peer.snapshot();
    /// let mut messages: Vec<_> = snapshot.iter().collect();
    /// messages.sort_by_cached_key(|(version, _)| version.rank());
    /// # Ok::<(), rumors::error::EncodeError>(())
    /// ```
    pub fn iter(&self) -> Iter<'_, T> {
        self.tree.iter()
    }

    /// Iterates the messages whose [`Version`]s the causal `query` specifies.
    ///
    /// The query can be anything [`Into`] a [`causally::Query`], i.e. an
    /// expression built from the [`causally`] vocabulary, such as
    /// `causally::since(&s)`, `causally::delta(&s, &e)`, `causally::after(&s) &
    /// causally::before(&e)`, etc., or a [`Span`](before::Span), or a
    /// [`Version`] itself.
    ///
    /// Iterating a small causal query against a large snapshot costs work
    /// proportional to the delta, not the snapshot.
    ///
    /// Unlike [`iter`](Self::iter), this method does *not* produce an
    /// [`ExactSizeIterator`], because the number of messages admitted by the
    /// query is unknown until they are visited.
    ///
    /// As with [`iter`](Self::iter), order is unspecified and does *not* follow
    /// causal order. This method filters by versions, but does not yield them
    /// in version-order. Sort the result by [`Version::rank`] (i.e. using
    /// [`sort_by_cached_key`](slice::sort_by_cached_key)) if you want an order
    /// that places causes before their effects.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::{Peer, causally};
    ///
    /// let peer = Peer::<String>::seed();
    /// peer.send("first".to_string())?;
    /// let then = peer.snapshot().latest().clone();
    /// peer.send("second".to_string())?;
    /// peer.send("third".to_string())?;
    ///
    /// let snapshot = peer.snapshot();
    /// // Everything not already contained in `then`.
    /// assert_eq!(snapshot.range(causally::since(&then)).count(), 2);
    /// // Everything `then` already contained.
    /// assert_eq!(snapshot.range(causally::before(&then)).count(), 1);
    /// // The two partition the set of messages.
    /// assert_eq!(snapshot.range(causally::all()).count(), 2 + 1);
    /// # Ok::<(), rumors::error::EncodeError>(())
    /// ```
    pub fn range<'q, P: causally::Polarity>(
        &'q self,
        query: impl Into<causally::Query<'q, P>>,
    ) -> impl DoubleEndedIterator<Item = (&'q Version, Arc<T>)> + Send + Sync {
        self.tree.range(query)
    }
}

/// Clone snapshots by sharing immutable storage.
impl<T: Send + Sync + 'static> Clone for Snapshot<T> {
    /// Clones the snapshot.
    fn clone(&self) -> Self {
        Self {
            tree: self.tree.clone(),
        }
    }
}

/// Debug snapshots without inspecting their messages.
impl<T: Send + Sync + 'static> fmt::Debug for Snapshot<T> {
    ///  Format the captured state without walking or printing its messages.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Snapshot")
            .field("network", &self.network())
            .field("latest", self.tree.latest())
            .field("len", &self.tree.len())
            .finish_non_exhaustive()
    }
}

/// Compare snapshots.
impl<T: Send + Sync + 'static> PartialEq for Snapshot<T> {
    /// Compares network, live messages, and latest version.
    fn eq(&self, other: &Self) -> bool {
        self.tree == other.tree
    }
}

/// Snapshot equality is an equivalence relation.
impl<T: Send + Sync + 'static> Eq for Snapshot<T> {}

/// Iterate over a borrowed snapshot's messages.
impl<'a, T: Send + Sync + 'static> IntoIterator for &'a Snapshot<T> {
    type Item = (&'a Version, Arc<T>);
    type IntoIter = Iter<'a, T>;

    /// Returns the same iterator as [`Snapshot::iter`], because it internally
    /// clones.
    fn into_iter(self) -> Self::IntoIter {
        self.tree.iter()
    }
}
