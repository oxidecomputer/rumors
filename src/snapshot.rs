use std::convert::Infallible;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::Stream;

use crate::tree::{RangeOwned, Tree};
use crate::{Key, Network, StorageError, Version};

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

/// The stream of [`Snapshot::iter`] and [`Snapshot::range`]: every selected
/// live message as an owned `(Key, Version, Arc<T>)`, in unspecified order.
///
/// Fully owned and lifetime-free: hold it across awaits or in long-lived
/// state; it pins only its unvisited frontier (plus each yielded message's
/// shared payload). With the in-memory backend every item is immediately
/// ready and the error is uninhabited.
pub struct Messages<T> {
    walk: RangeOwned<T, (std::ops::Bound<Version>, std::ops::Bound<Version>)>,
}

impl<T: Send + Sync> Stream for Messages<T> {
    type Item = Result<(Key, Version, Arc<T>), StorageError<Infallible>>;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(
            self.get_mut()
                .walk
                .next()
                .map(|(key, leaf)| Ok((key, leaf.version().clone(), leaf.value().clone()))),
        )
    }
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
    ///
    /// Exact and O(1); the count [`iter`](Self::iter) will yield.
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

    /// Looks up a single live message by its [`Key`], returning its version
    /// and shared payload.
    pub async fn get(
        &self,
        key: &Key,
    ) -> Result<Option<(Version, Arc<T>)>, StorageError<Infallible>> {
        Ok(self
            .tree
            .get(key)
            .map(|(version, value)| (version.clone(), value.clone())))
    }

    /// Streams every live message as owned `(Key, Version, Arc<T>)`.
    ///
    /// Order is unspecified, and in particular does *not* follow the causal
    /// order: a message may be yielded before another that causally precedes
    /// it. Sort by the yielded [`Version`]s if your application needs an
    /// ordering consistent with causality. The stream yields exactly
    /// [`len`](Self::len) messages.
    pub fn iter(&self) -> Messages<T>
    where
        T: Send + Sync,
    {
        self.range(..)
    }

    /// Streams the messages whose [`Version`]s fall within the causal `range`.
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
    /// `range(causally::delta(&s, &e)?)`,
    /// `range(causally::not_before(&s).known_at(&e)?)`, and so on (pairing a
    /// start with an end validates that the start lies within the end
    /// bound). Plain range
    /// syntax like `&v1..=&v2`, `&v1..` also works, as does any other
    /// [`RangeBounds<Version>`](std::ops::RangeBounds) value, such as a
    /// tuple of [`Bound`](std::ops::Bound)s.
    ///
    /// Streaming a small causal delta against a large snapshot costs work
    /// proportional to the delta, not the snapshot.
    ///
    /// As with [`iter`](Self::iter), order is unspecified and does *not*
    /// follow the causal order: filtering by versions does not mean yielding
    /// in version order. Sort by the yielded [`Version`]s if your application
    /// needs an ordering consistent with causality.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures::TryStreamExt;
    /// use rumors::{Peer, causally};
    ///
    /// # tokio::runtime::Builder::new_current_thread()
    /// #     .build()
    /// #     .unwrap()
    /// #     .block_on(async {
    /// let rumors = Peer::<String>::seed().into_rumors();
    /// rumors.send("first".to_string()).await?;
    /// let then = rumors.snapshot().latest().clone();
    /// rumors.send("second".to_string()).await?;
    /// rumors.send("third".to_string()).await?;
    ///
    /// let snapshot = rumors.snapshot();
    /// // Everything not already contained in `then`: the two later sends.
    /// let since: Vec<_> = snapshot.range(causally::since(&then)).try_collect().await?;
    /// assert_eq!(since.len(), 2);
    /// // Everything `then` already contained: just the first.
    /// let known: Vec<_> = snapshot.range(causally::known_at(&then)).try_collect().await?;
    /// assert_eq!(known.len(), 1);
    /// // The two compose into the same partition of the live set.
    /// let all: Vec<_> = snapshot.range(causally::all()).try_collect().await?;
    /// assert_eq!(all.len(), 3);
    /// # Ok::<(), rumors::StorageError<std::convert::Infallible>>(())
    /// # })?;
    /// # Ok::<(), rumors::StorageError<std::convert::Infallible>>(())
    /// ```
    pub fn range<R>(&self, range: R) -> Messages<T>
    where
        T: Send + Sync,
        R: std::ops::RangeBounds<Version>,
    {
        use std::ops::Bound;
        let cloned = |bound: Bound<&Version>| match bound {
            Bound::Included(v) => Bound::Included(v.clone()),
            Bound::Excluded(v) => Bound::Excluded(v.clone()),
            Bound::Unbounded => Bound::Unbounded,
        };
        Messages {
            walk: self
                .tree
                .range_owned((cloned(range.start_bound()), cloned(range.end_bound()))),
        }
    }

    /// Forces this set's tree to compute its lazy structural memos (observable
    /// hash and ceiling/floor version bounds), so a subsequent operation is
    /// timed against its own work. For benchmark and test calibration only.
    #[doc(hidden)]
    pub fn warm_caches(&self) {
        self.tree.warm_caches();
    }
}
