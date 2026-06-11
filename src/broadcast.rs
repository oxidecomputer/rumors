use crate::tree::{Frozen, Leaf};
use crate::{Batch, Error, Key, Known, Network, Snapshot, Version};
use borsh::{BorshDeserialize, BorshSerialize};
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
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

    /// Observe every message in this rumor set, from genesis onward. See
    /// [`Messages`] for the contract; equivalent to
    /// [`messages_from`](Self::messages_from) at [`Version::new`].
    pub fn messages(&self) -> Messages<T>
    where
        T: Send + Sync,
    {
        self.known.messages()
    }

    /// Observe every message not already causally contained in `since`. See
    /// [`Messages`] for the contract.
    pub fn messages_from(&self, since: Version) -> Messages<T>
    where
        T: Send + Sync,
    {
        self.known.messages_from(since)
    }

    /// Force this set's tree to compute its lazy structural memos (observable
    /// hash and ceiling/floor version bounds), so a subsequent operation is
    /// timed against its own work. For benchmark and test calibration only.
    #[doc(hidden)]
    pub fn warm_caches(&self) {
        self.known.warm_caches();
    }
}

/// An observer of one rumor set: every message not causally contained in
/// the starting cursor, then every message learned afterwards — by local
/// [`send`](Broadcast::send), by gossip, from any handle — and `None` once
/// the [`Known`] and every [`Broadcast`] have dropped and no further change
/// is possible, after yielding the complete final state.
///
/// Two faces over one engine:
///
/// - [`borrow_next`](Self::borrow_next) lends each message as `(Key,
///   &Version, &Arc<T>)`, the borrows living until the next call — no
///   per-item `Version` clone, no `Arc` traffic. There is no standard
///   lending-iterator trait, so it is an inherent method, consumed
///   `while let Some((key, version, value)) = messages.borrow_next().await`.
/// - The [`Stream`] impl (for `T: 'static`) yields owned `(Key, Version,
///   Arc<T>)` items for `select!`-and-combinate consumers; on the sync
///   mirror the same face is an [`Iterator`].
///
/// The delivery contract: every message live at some pass is yielded
/// exactly once; a message inserted and redacted wholly between passes is
/// never yielded (already-redacted content is never delivered); redactions
/// themselves are honored silently. Order is unspecified and does *not*
/// follow the causal order — a message may be yielded before another that
/// causally precedes it; order by the yielded [`Version`]s if causality
/// matters.
///
/// Pausing, cancelling, and resuming are ordinary control flow: between
/// items the observer holds only a constant-size descent spine (one entry
/// per materialized branch level, at most the tree's depth — nothing
/// buffered, nothing growing with the delta or the tree), so hold it as
/// long as you like and ask again later; drop it to cancel. To resume in a
/// *later process*, or on another replica of the same network, persist
/// [`cursor`](Self::cursor) and start a new observer from it.
///
/// An observer is not an actor: it holds no send handle, does not keep the
/// rumor set open, and does not block the future from [`Known::broadcast`]
/// that reunites the [`Known`].
pub struct Messages<T> {
    /// The watch channel, or the in-flight wait for it to change. The wait
    /// future owns the receiver and hands it back: the `Stream` face cannot
    /// hold a borrowing `changed()` future across polls (recreating one per
    /// poll would drop its waker registration and lose the wakeup), so the
    /// wait is materialized; `borrow_next` enters it only to finish what a
    /// `Stream` poll started.
    channel: Option<Channel<T>>,
    cursor: Version,
    pass: Option<Pass<T>>,
    /// The most recently yielded leaf, kept alive so its version and value
    /// can be lent to the caller until the next call.
    current: Option<(Key, Leaf<T>)>,
}

/// A wait for the channel to change, owning the receiver; resolves to
/// whether the channel closed, and the receiver itself.
type WaitForChange<T> =
    Pin<Box<dyn Future<Output = (bool, watch::Receiver<crate::Inner<T>>)> + Send>>;

enum Channel<T> {
    /// The channel is in hand.
    Ready(watch::Receiver<crate::Inner<T>>),
    /// A wait for change is in flight.
    Waiting(WaitForChange<T>),
}

/// One in-progress pass: the frozen walk over its snapshot, and the
/// snapshot's ceiling to absorb into the cursor when the walk drains.
struct Pass<T> {
    walk: Frozen<T, (std::ops::Bound<Version>, std::ops::Bound<Version>)>,
    ceiling: Version,
}

impl<T> Messages<T> {
    pub(crate) fn subscribe(inner: &watch::Sender<crate::Inner<T>>, since: Version) -> Self {
        Self {
            channel: Some(Channel::Ready(inner.subscribe())),
            cursor: since,
            pass: None,
            current: None,
        }
    }

    /// Open a pass over the latest snapshot if none is in progress. The
    /// watch read guard lives only long enough to freeze the walk (a root
    /// handle clone) and capture the ceiling.
    fn open_pass(
        pass: &mut Option<Pass<T>>,
        rx: &mut watch::Receiver<crate::Inner<T>>,
        cursor: &Version,
    ) where
        T: Send + Sync,
    {
        if pass.is_none() {
            let inner = rx.borrow_and_update();
            *pass = Some(Pass {
                walk: inner.tree.freeze((
                    std::ops::Bound::Excluded(cursor.clone()),
                    std::ops::Bound::Unbounded,
                )),
                ceiling: inner.tree.latest().clone(),
            });
        }
    }

    /// Advance to the next message, lending its version and value until the
    /// following call. Awaits quietly while the set is unchanged; resolves
    /// [`None`] once no further change is possible.
    pub async fn borrow_next(&mut self) -> Option<(Key, &Version, &Arc<T>)>
    where
        T: Send + Sync,
    {
        loop {
            match self.channel.as_mut().expect("channel state present") {
                // Finish a wait the `Stream` face left in flight.
                Channel::Waiting(wait) => {
                    let (closed, rx) = wait.as_mut().await;
                    self.channel = Some(Channel::Ready(rx));
                    if closed {
                        return None;
                    }
                }
                Channel::Ready(rx) => {
                    Self::open_pass(&mut self.pass, rx, &self.cursor);

                    // Lend the next leaf out of the walk, parking it in
                    // `current` so the borrows survive the return.
                    let pass = self.pass.as_mut().expect("opened above");
                    if let Some((key, leaf)) = pass.walk.next() {
                        let (key, leaf) = self.current.insert((key, leaf));
                        return Some((*key, leaf.version(), leaf.value()));
                    }

                    // The pass drained: absorb its ceiling as completed,
                    // then await the next change; `Err` means every sender
                    // is gone and the drain above already saw the final
                    // state.
                    let Pass { ceiling, .. } = self.pass.take().expect("opened above");
                    self.cursor |= &ceiling;
                    if rx.changed().await.is_err() {
                        return None;
                    }
                }
            }
        }
    }

    /// The sound resume point: the causal frontier of the last *completed*
    /// pass, suitable for persisting across processes or handing to another
    /// replica of the same network — a later
    /// [`messages_from(cursor)`](Broadcast::messages_from) re-observes
    /// nothing from completed passes and everything not yet delivered.
    /// Messages already delivered from the *in-progress* pass are delivered
    /// again (a [`Version`] can only encode a causally closed boundary, and
    /// delivery order is not causal order, so a partial pass's prefix need
    /// not be one); dedup by [`Key`] across such a resume if re-delivery
    /// matters. For the same reason, folding the yielded versions yourself
    /// is *not* a sound resume point: the fold can causally contain a
    /// message that was never delivered, which a resume would then skip
    /// forever.
    ///
    /// After the observer ends (`None`), this is the complete final
    /// frontier. To merely pause in-process, just hold the observer: its
    /// idle state is constant-size, and the cursor stays inside it.
    pub fn cursor(&self) -> &Version {
        &self.cursor
    }
}

/// The owned-item face: `(Key, Version, Arc<T>)` per item, cloned out of
/// the same engine [`borrow_next`](Messages::borrow_next) lends from.
/// `T: 'static` because the quiet-period wait is materialized as an owned
/// future (see the `channel` field).
impl<T: Send + Sync + 'static> Stream for Messages<T> {
    type Item = (Key, Version, Arc<T>);

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            match this.channel.as_mut().expect("channel state present") {
                Channel::Waiting(wait) => match wait.as_mut().poll(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready((closed, rx)) => {
                        this.channel = Some(Channel::Ready(rx));
                        if closed {
                            return Poll::Ready(None);
                        }
                    }
                },
                Channel::Ready(rx) => {
                    Self::open_pass(&mut this.pass, rx, &this.cursor);

                    let pass = this.pass.as_mut().expect("opened above");
                    if let Some((key, leaf)) = pass.walk.next() {
                        return Poll::Ready(Some((
                            key,
                            leaf.version().clone(),
                            leaf.value().clone(),
                        )));
                    }

                    // The pass drained: absorb its ceiling, then enter the
                    // owned wait (the receiver rides inside the future and
                    // comes back with the result).
                    let Pass { ceiling, .. } = this.pass.take().expect("opened above");
                    this.cursor |= &ceiling;
                    let Some(Channel::Ready(mut rx)) = this.channel.take() else {
                        unreachable!("matched Ready above");
                    };
                    this.channel = Some(Channel::Waiting(Box::pin(async move {
                        let closed = rx.changed().await.is_err();
                        (closed, rx)
                    })));
                }
            }
        }
    }
}
