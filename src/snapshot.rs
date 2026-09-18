use std::{fmt, sync::Arc};

#[cfg(any(test, feature = "test-internals"))]
use crate::tree::MERKLE_HASH_LEN;
use crate::{Network, Version, causally, tree::Tree};

/// An iterator over the live messages in a [`Snapshot`].
///
/// It yields each message's version and an owned handle to its payload. The
/// order is unspecified; the iterator is exact-size and double-ended.
pub use crate::tree::Iter;

/// A consistent point-in-time view of a set of rumors.
///
/// Consistent means atomic: the snapshot holds exactly the live set as of one
/// moment. Taking one ([`Rumors::snapshot`](crate::Rumors::snapshot)) is cheap:
/// it shares structure with the live set rather than copying it, and later
/// changes never show through. Hold it as long as you like; it keeps its
/// messages alive, not the [`Peer`](crate::Peer).
pub struct Snapshot<T: Send + Sync + 'static> {
    /// The gossip network whose state was captured.
    network: Network,
    /// The immutable live set and causal frontier.
    tree: Tree<T>,
}

impl<T: Send + Sync + 'static> Snapshot<T> {
    /// Makes a new snapshot.
    pub(crate) fn new(network: Network, tree: Tree<T>) -> Self {
        Self { network, tree }
    }

    /// The snapshotted tree, for crate-internal instruments.
    #[cfg(any(test, feature = "test-internals"))]
    pub(crate) fn tree(&self) -> &Tree<T> {
        &self.tree
    }

    /// Return the gossip network this snapshot belongs to.
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

    /// Returns the Merkle root over the live messages.
    ///
    /// Test support for checking the tree's internal comparison signal. Use
    /// snapshot equality or the read methods in application code.
    #[cfg(any(test, feature = "test-internals"))]
    pub fn hash(&self) -> [u8; MERKLE_HASH_LEN] {
        self.tree.hash()
    }

    /// Returns whether `version` identifies a live message.
    pub fn contains(&self, version: &Version) -> bool {
        self.tree.contains(version)
    }

    /// Looks up the live message stamped with `version` (for example, a
    /// version an observer yielded earlier). Returns `None` when no live
    /// message carries it — never sent here, or since redacted.
    ///
    /// The yielded handle is an owned reference bump into the shared
    /// storage: cheap to take, and it keeps the message alive on its own.
    pub fn get(&self, version: &Version) -> Option<Arc<T>> {
        self.tree.get(version)
    }

    /// Iterates the versions of every live message without accessing its
    /// payload.
    ///
    /// The order is unspecified. The iterator is exact-size and double-ended.
    pub fn versions(
        &self,
    ) -> impl DoubleEndedIterator<Item = &Version> + ExactSizeIterator + Send + Sync {
        self.tree.versions()
    }

    /// Iterates every live message as `(&Version, Arc<T>)` — the version
    /// borrowed from the snapshot, the payload an owned handle into the
    /// shared storage.
    ///
    /// Order is unspecified, and in particular does *not* follow the causal
    /// order: a message may be yielded before another that causally precedes
    /// it. Sort by [`Version::ranked`] for a deterministic total order that
    /// places causes before their effects.
    ///
    /// ```
    /// let rumors = rumors::Peer::<String>::seed().into_rumors();
    /// rumors.send("first".into())?;
    /// rumors.send("second".into())?;
    /// let snapshot = rumors.snapshot();
    /// let mut messages: Vec<_> = snapshot.iter().collect();
    /// messages.sort_by(|a, b| a.0.ranked().cmp(&b.0.ranked()));
    /// # Ok::<(), rumors::EncodeError>(())
    /// ```
    pub fn iter(&self) -> Iter<'_, T> {
        self.tree.iter()
    }

    /// Iterates the messages whose [`Version`]s the causal `query` admits.
    ///
    /// The query is anything [`Into`] a [`causally::Query`]: an expression
    /// built from the [`causally`] vocabulary (`range(causally::since(&s))`,
    /// `range(causally::delta(&s, &e))`, `range(causally::after(&s) &
    /// causally::before(&e))`, ...), a [`Span`](before::Span), or a
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
    /// in version order. Sort by [`Version::ranked`] for a deterministic total
    /// order that places causes before their effects.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::{Peer, causally};
    ///
    /// let rumors = Peer::<String>::seed().into_rumors();
    /// rumors.send("first".to_string())?;
    /// let then = rumors.snapshot().latest().clone();
    /// rumors.send("second".to_string())?;
    /// rumors.send("third".to_string())?;
    ///
    /// let snapshot = rumors.snapshot();
    /// // Everything not already contained in `then`: the two later sends.
    /// assert_eq!(snapshot.range(causally::since(&then)).count(), 2);
    /// // Everything `then` already contained: just the first.
    /// assert_eq!(snapshot.range(causally::before(&then)).count(), 1);
    /// // The two partition the live set.
    /// assert_eq!(snapshot.range(causally::all()).count(), 3);
    /// # Ok::<(), rumors::EncodeError>(())
    /// ```
    pub fn range<'q, P: causally::Polarity>(
        &'q self,
        query: impl Into<causally::Query<'q, P>>,
    ) -> impl DoubleEndedIterator<Item = (&'q Version, Arc<T>)> + Send + Sync {
        self.tree.range(query)
    }
}

/// Clone snapshots by sharing their immutable storage.
impl<T: Send + Sync + 'static> Clone for Snapshot<T> {
    /// Clones the snapshot by sharing its immutable tree.
    fn clone(&self) -> Self {
        Self {
            network: self.network,
            tree: self.tree.clone(),
        }
    }
}

/// Format snapshots without inspecting their payloads.
impl<T: Send + Sync + 'static> fmt::Debug for Snapshot<T> {
    /// Summarizes the captured state without walking or printing its messages.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Snapshot")
            .field("network", &self.network)
            .field("latest", self.tree.latest())
            .field("len", &self.tree.len())
            .finish_non_exhaustive()
    }
}

/// Compare the complete captured replica state.
impl<T: Send + Sync + 'static> PartialEq for Snapshot<T> {
    /// Compares the network, live messages, and causal frontier.
    fn eq(&self, other: &Self) -> bool {
        self.network == other.network && self.tree == other.tree
    }
}

/// Mark snapshot equality as an equivalence relation.
impl<T: Send + Sync + 'static> Eq for Snapshot<T> {}

/// Iterate over a borrowed snapshot's live messages.
impl<'a, T: Send + Sync + 'static> IntoIterator for &'a Snapshot<T> {
    type Item = (&'a Version, Arc<T>);
    type IntoIter = Iter<'a, T>;

    /// Returns the same iterator as [`Snapshot::iter`].
    fn into_iter(self) -> Self::IntoIter {
        self.tree.iter()
    }
}
