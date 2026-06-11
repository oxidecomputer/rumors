use crate::tree::{Frozen, Leaf};
use crate::{Batch, Error, Key, Known, Network, Snapshot, Version, causally};
use borsh::{BorshDeserialize, BorshSerialize};
use futures::Stream;
use std::ops::ControlFlow;
use std::sync::Arc;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::watch,
};

/// A broadcast handle for a set of rumors.
pub struct Broadcast<T> {
    pub(crate) known: Known<T>,
    /// Liveness token for the future returned by [`Known::broadcast`]: every
    /// clone of this `Broadcast` holds a clone of this receiver, and that
    /// future awaits the paired sender's
    /// [`closed`](tokio::sync::watch::Sender::closed), which resolves exactly
    /// when the last receiver (the last `Broadcast`) drops. Nothing is ever
    /// sent on this channel.
    pub(crate) alive: watch::Receiver<()>,
}

impl<T> Clone for Broadcast<T> {
    fn clone(&self) -> Self {
        Self {
            known: Known {
                network: self.known.network,
                inner: self.known.inner.clone(),
            },
            alive: self.alive.clone(),
        }
    }
}

/// A summary view (network, latest version, live-message count), independent
/// of `T: Debug`: the messages themselves are not printed.
impl<T> std::fmt::Debug for Broadcast<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.known.inner.borrow();
        f.debug_struct("Broadcast")
            .field("network", &self.known.network)
            .field("latest", inner.tree.latest())
            .field("len", &inner.tree.len())
            .finish_non_exhaustive()
    }
}

impl<T> Broadcast<T> {
    /// Send a message to all listeners.
    ///
    /// Returns a [`Batch`] that commits when dropped: a bare
    /// `broadcast.send(message);` commits at the end of the statement, and
    /// chaining further [`send`](Batch::send)s and
    /// [`redact`](Batch::redact)s accumulates them into one commit. Building
    /// holds no lock; see [`batch`](Self::batch).
    pub fn send(&self, message: T) -> Batch<'_, T>
    where
        T: BorshSerialize + Send + Sync,
    {
        self.known.send(message)
    }

    /// Redact a message for all listeners: it is contagiously purged from
    /// the [`Known`] set for all peers who gossip with us, and will be
    /// unobserved by any future peers who did not already observe it.
    ///
    /// Returns a [`Batch`] that commits when dropped, exactly as
    /// [`send`](Self::send) does.
    pub fn redact(&self, key: Key) -> Batch<'_, T>
    where
        T: Send + Sync,
    {
        self.known.redact(key)
    }

    /// Start an empty [`Batch`] of insertions and redactions, committed
    /// atomically when dropped: one tree traversal, one change notification.
    /// Building holds no lock — the rumor set is locked only inside the
    /// commit.
    pub fn batch(&self) -> Batch<'_, T>
    where
        T: Send + Sync,
    {
        self.known.batch()
    }

    /// Gossip with a remote peer to synchronize rumor sets.
    pub async fn gossip<'a, R, W>(&mut self, read: &'a mut R, write: &'a mut W) -> Result<(), Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        self.known.gossip(read, write).await
    }

    /// The identifier shared by every peer that descends from the same
    /// [`seed`](Known::seed).
    pub fn network(&self) -> Network {
        self.known.network()
    }

    /// The latest version of any message ever tracked by this [`Known`].
    pub fn latest(&self) -> Version {
        self.known.latest()
    }

    /// The earliest version of any message currently present in this [`Known`], or
    /// `None` if it has never seen a message.
    pub fn earliest(&self) -> Option<Version> {
        self.known.earliest()
    }

    /// Determine if there are any current messages in this [`Known`].
    pub fn is_empty(&self) -> bool {
        self.known.is_empty()
    }

    /// The number of live messages in this [`Known`].
    pub fn len(&self) -> usize {
        self.known.len()
    }

    /// Take a consistent snapshot of the current state.
    pub fn snapshot(&self) -> Snapshot<T> {
        self.known.snapshot()
    }

    /// The observable root hash of this set: a 32-byte digest of its live
    /// content, independent of party identity and insertion order. Two sets
    /// with equal hashes hold the same live messages. Gossip converges on
    /// causal versions rather than hashes: peers with equal hashes but
    /// different versions (for example, after an insert that was then
    /// redacted) still run a reconciliation pass.
    pub fn hash(&self) -> [u8; 32] {
        self.known.hash()
    }

    /// Look up a single live message by its [`Key`]: one `O(depth)` descent
    /// (the key *is* the leaf's content-addressed path), never a scan.
    /// Returns owned handles cloned out of the synchronized state; `None`
    /// when no live message has that key — never inserted, or since
    /// redacted.
    pub fn get(&self, key: &Key) -> Option<(Version, Arc<T>)> {
        self.known.get(key)
    }

    /// Observe every message in this rumor set, from genesis onward: the
    /// returned future fires `on_message` once for every message currently
    /// live, then follows along with every change, firing once per message
    /// learned — by local [`send`](Self::send), by gossip, from any handle.
    ///
    /// Equivalent to [`listen_from`](Self::listen_from) at [`Version::new`];
    /// see it for the delivery contract, termination, early exit, and
    /// resumption.
    pub fn listen<B, F, Fut>(
        self,
        on_message: F,
    ) -> impl Future<Output = (Version, Option<B>)> + Send
    where
        T: Send + Sync,
        B: Send,
        F: FnMut(Key, &Version, &Arc<T>) -> Fut + Send,
        Fut: Future<Output = ControlFlow<B>> + Send,
    {
        self.listen_from(Version::new(), on_message)
    }

    /// Observe every message not already causally contained in `since`.
    ///
    /// The cursor is a causal [`Version`], not a tree: each time the shared
    /// state changes, the listener fires `on_message` for precisely the live
    /// messages whose versions the cursor does not dominate, then absorbs
    /// the snapshot's ceiling once the pass completes. Every message live at
    /// some pass is observed exactly once within one uninterrupted listen; a
    /// message inserted and redacted wholly between passes is never observed
    /// (already-redacted content is never delivered); redactions themselves
    /// are honored silently. Delivery order is unspecified and does *not*
    /// follow the causal order: a message may be observed before another
    /// that causally precedes it. Order by the [`Version`] handed to the
    /// callback if causality matters.
    ///
    /// The callback steers the listener:
    /// [`Continue(())`](ControlFlow::Continue) keeps listening, and
    /// [`Break(value)`](ControlFlow::Break) stops it immediately. The future
    /// resolves either way to `(cursor, outcome)`. On a break, `(cursor,
    /// Some(value))`, where the cursor is the *last completed pass's*
    /// frontier: a [`Version`] can only encode a causally closed boundary,
    /// and because delivery order is not causal order, the messages
    /// delivered before a mid-pass break need not form one. Resuming from it
    /// is therefore *at-least-once for the interrupted pass* — its
    /// already-delivered messages are delivered again — and exactly-once
    /// everywhere else; dedup by [`Key`] across a break if re-delivery
    /// matters. With no break, `(cursor, None)` once no further change is
    /// possible — after the [`Known`] and every [`Broadcast`] have dropped —
    /// having observed the complete final state. Either cursor is a valid
    /// `since` against any replica of the same network.
    ///
    /// Consuming `self` dissolves this handle: a listener is an observer, not
    /// an actor. It does not hold the rumor set open, and it does not block
    /// the future from [`Known::broadcast`] that reunites the [`Known`].
    /// *Dropping* the future yields no resume cursor at all: break out with
    /// [`ControlFlow::Break`] instead when you intend to come back. (Folding
    /// the delivered versions yourself does **not** make a sound resume
    /// point: a fold can causally contain a message that was never
    /// delivered, which a resume would then skip forever.)
    pub fn listen_from<B, F, Fut>(
        self,
        since: Version,
        mut on_message: F,
    ) -> impl Future<Output = (Version, Option<B>)> + Send
    where
        T: Send + Sync,
        B: Send,
        F: FnMut(Key, &Version, &Arc<T>) -> Fut + Send,
        Fut: Future<Output = ControlFlow<B>> + Send,
    {
        // Subscribe while our own sender still holds the channel open, then
        // dissolve the Broadcast eagerly (when `listen_from` returns, not
        // when the future first polls): dropping our data sender means the
        // channel closes when the last *actor* — the `Known` or a
        // `Broadcast` — goes, which is exactly the listener's termination
        // signal (holding it would self-pin the listener, waiting on a
        // channel it keeps open); dropping our liveness receiver means a
        // listener does not pin `Known::broadcast`'s reunification future.
        let mut rx = self.known.inner.subscribe();
        drop(self);
        async move {
            let mut cursor = since;
            loop {
                // Snapshot under the read guard and release it immediately:
                // the guard blocks every writer, so it must never be held
                // across an await. The tree clone is a cheap copy-on-write
                // handle onto shared structure.
                let snapshot = rx.borrow_and_update().tree.clone();
                let ceiling = snapshot.latest().clone();

                // Fire for precisely the leaves the cursor does not dominate.
                // (`since`, not `not_before`: a leaf at exactly the cursor
                // was observed by the pass that absorbed it, and must not
                // re-fire.) Dominated subtrees are pruned by their memoized
                // version bounds, so a pass costs the delta, not the tree.
                //
                // A break resolves with the cursor as it stood at the pass
                // start: delivery is in key order, not causal order, so the
                // prefix delivered before a mid-pass break need not be
                // causally closed, and no `Version` can cover exactly that
                // prefix (folding the delivered versions can causally
                // contain an undelivered message, which a resume would skip
                // forever). The last completed pass is the finest sound
                // resume point. (The filter borrows a clone so the break can
                // move the cursor out from under the live iterator.)
                let pass = cursor.clone();
                for (key, version, message) in snapshot.range(causally::since(&pass)) {
                    if let ControlFlow::Break(value) = on_message(key, version, message).await {
                        return (cursor, Some(value));
                    }
                }

                // The pass observed every survivor at or under the snapshot's
                // ceiling. Absorbing the ceiling (rather than the observed
                // leaf versions) also covers redaction ticks, which left no
                // leaves behind to observe.
                cursor |= &ceiling;

                // `Err` here means every sender is gone: no further change is
                // possible, and the pass above already drained the final
                // state.
                if rx.changed().await.is_err() {
                    break (cursor, None);
                }
            }
        }
    }

    /// Observe this rumor set as lent `(Key, &Version, &Arc<T>)` items, from
    /// genesis onward.
    ///
    /// Equivalent to [`messages_from`](Self::messages_from) at
    /// [`Version::new`]; see it and [`Messages`] for the contract.
    pub fn messages(self) -> Messages<T>
    where
        T: Send + Sync,
    {
        self.messages_from(Version::new())
    }

    /// Observe every message not already causally contained in `since`, as
    /// lent items: the zero-copy sibling of [`listen_from`](Self::listen_from)
    /// and [`stream_from`](Self::stream_from). See [`Messages`].
    pub fn messages_from(self, since: Version) -> Messages<T>
    where
        T: Send + Sync,
    {
        // Subscribe-then-dissolve, exactly as `listen_from` does and for the
        // same reasons.
        let rx = self.known.inner.subscribe();
        drop(self);
        Messages {
            rx,
            cursor: since,
            pass: None,
            current: None,
        }
    }

    /// Observe this rumor set as a [`Stream`] of owned `(Key, Version,
    /// Arc<T>)` items, from genesis onward.
    ///
    /// Equivalent to [`stream_from`](Self::stream_from) at [`Version::new`];
    /// see it for the delivery contract.
    pub fn stream(self) -> impl Stream<Item = (Key, Version, Arc<T>)> + Send
    where
        T: Send + Sync,
    {
        self.stream_from(Version::new())
    }

    /// Observe every message not already causally contained in `since`, as a
    /// [`Stream`] of owned `(Key, Version, Arc<T>)` items: the
    /// [`Stream`]-shaped sibling of [`listen_from`](Self::listen_from), for
    /// consumers who would rather `select!` and combinate than write
    /// callbacks. A thin cloning adapter over [`Messages`], which lends the
    /// same items without the clones.
    ///
    /// Same delivery contract as [`listen_from`](Self::listen_from): every
    /// message live at some pass is yielded exactly once, already-redacted
    /// content is never yielded, order is unspecified (not causal), and the
    /// stream ends once the [`Known`] and every [`Broadcast`] have dropped,
    /// after yielding the complete final state. One difference inherent to
    /// the shape: dropping the stream yields no resume cursor — for
    /// resumable consumption use [`listen_from`](Self::listen_from) and
    /// break with [`ControlFlow::Break`] (folding the yielded versions is
    /// *not* a sound resume point; see there).
    pub fn stream_from(self, since: Version) -> impl Stream<Item = (Key, Version, Arc<T>)> + Send
    where
        T: Send + Sync,
    {
        futures::stream::unfold(self.messages_from(since), |mut messages| async move {
            let item = messages
                .next()
                .await
                .map(|(key, version, value)| (key, version.clone(), value.clone()));
            item.map(|item| (item, messages))
        })
    }

    /// Force this set's tree to compute its lazy structural memos (observable
    /// hash and ceiling/floor version bounds), so a subsequent operation is
    /// timed against its own work. For benchmark and test calibration only.
    #[doc(hidden)]
    pub fn warm_caches(&self) {
        self.known.warm_caches();
    }
}

/// A lending observer of one rumor set: each call to
/// [`next`](Self::next) yields the next message as `(Key, &Version,
/// &Arc<T>)`, with the borrows lent out of the iterator until the next
/// call — no per-item `Version` clone, no `Arc` traffic. There is no
/// standard lending-iterator trait to implement, so `next` is an inherent
/// method, consumed `while let Some((key, version, value)) =
/// messages.next().await { … }`.
///
/// Same delivery contract as [`Broadcast::listen_from`]: every message live
/// at some pass is yielded exactly once, already-redacted content is never
/// yielded, order is unspecified (not causal), and `next` returns [`None`]
/// once the [`Known`] and every [`Broadcast`] have dropped, after yielding
/// the complete final state. Dropping the observer yields no resume cursor —
/// for resumable consumption use [`Broadcast::listen_from`] and break with
/// [`ControlFlow::Break`].
///
/// Internally a frozen, fully-owned walk over each pass's snapshot: between
/// items it holds only the walk's unvisited frontier (bounded by tree
/// shape, not by the delta), releasing subtrees as it goes — nothing is
/// buffered.
pub struct Messages<T> {
    rx: watch::Receiver<crate::Inner<T>>,
    cursor: Version,
    pass: Option<Pass<T>>,
    /// The most recently yielded leaf, kept alive so its version and value
    /// can be lent to the caller until the next call.
    current: Option<(Key, Leaf<T>)>,
}

/// One in-progress pass: the frozen walk over its snapshot, and the
/// snapshot's ceiling to absorb into the cursor when the walk drains.
struct Pass<T> {
    walk: Frozen<T, (std::ops::Bound<Version>, std::ops::Bound<Version>)>,
    ceiling: Version,
}

impl<T> Messages<T> {
    /// Advance to the next message, lending its version and value until the
    /// following call. Awaits quietly while the set is unchanged; resolves
    /// [`None`] once no further change is possible.
    pub async fn next(&mut self) -> Option<(Key, &Version, &Arc<T>)>
    where
        T: Send + Sync,
    {
        loop {
            // Open a pass over the latest snapshot if none is in progress.
            // The watch read guard lives only long enough to freeze the
            // walk (a root handle clone) and capture the ceiling.
            if self.pass.is_none() {
                let inner = self.rx.borrow_and_update();
                self.pass = Some(Pass {
                    walk: inner.tree.freeze((
                        std::ops::Bound::Excluded(self.cursor.clone()),
                        std::ops::Bound::Unbounded,
                    )),
                    ceiling: inner.tree.latest().clone(),
                });
            }

            // Lend the next leaf out of the walk, parking it in `current`
            // so the borrows survive the return.
            let pass = self.pass.as_mut().expect("opened above");
            if let Some((key, leaf)) = pass.walk.next() {
                let (key, leaf) = self.current.insert((key, leaf));
                return Some((*key, leaf.version(), leaf.value()));
            }

            // The pass drained: absorb its ceiling as completed, then await
            // the next change; `Err` means every sender is gone and the
            // drain above already saw the final state.
            let Pass { ceiling, .. } = self.pass.take().expect("opened above");
            self.cursor |= &ceiling;
            if self.rx.changed().await.is_err() {
                return None;
            }
        }
    }
}
