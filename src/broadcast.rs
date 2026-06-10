use crate::{Error, Key, Known, Message, Network, Snapshot, Version, causally, tree::Action};
use borsh::{BorshDeserialize, BorshSerialize};
use std::future::ready;
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

impl<T> Broadcast<T> {
    /// Send messages to all listeners.
    pub fn send<'a, I>(&'a mut self, messages: I)
    where
        T: BorshSerialize + Send + Sync + 'a,
        I: IntoIterator<Item = T> + Send,
        I::IntoIter: Send,
    {
        self.known.inner.send_if_modified(|inner| {
            let party = inner
                .party
                .as_ref()
                .expect("party must be present for send");
            let hash_before = inner.tree.hash();
            pollster::block_on(inner.tree.act(
                |batch| {
                    batch.tick(party);
                },
                messages.into_iter().map(Message::from).map(Action::Insert),
                |_, _, _| ready(()),
            ));
            inner.tree.hash() != hash_before
        });
    }

    /// Redact the given keys for all listeners.
    ///
    /// The corresponding messages will be contagiously purged from the
    /// [`Known`] set for all peers who gossip with us, and will be unobserved
    /// by any future peers who did not already observe the messages.
    pub fn redact<I: IntoIterator<Item = Key>>(&mut self, redacted: I)
    where
        T: Send + Sync,
    {
        self.known.inner.send_if_modified(|inner| {
            let party = inner
                .party
                .as_ref()
                .expect("party must be present for redact");
            let hash_before = inner.tree.hash();
            pollster::block_on(inner.tree.act(
                |batch| {
                    batch.tick(party);
                },
                redacted.into_iter().map(Action::Forget),
                |_, _, _| ready(()),
            ));
            inner.tree.hash() != hash_before
        });
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

    /// Observe every message in this rumor set, from genesis onward: the
    /// returned future fires `on_message` once for every message currently
    /// live, then follows along with every change, firing once per message
    /// learned — by local [`send`](Self::send), by gossip, from any handle.
    ///
    /// Equivalent to [`listen_from`](Self::listen_from) at [`Version::new`];
    /// see it for the delivery contract, termination, and resumption.
    pub fn listen<F, Fut>(self, on_message: F) -> impl Future<Output = Version> + Send
    where
        T: Send + Sync,
        F: FnMut(Key, &Version, &Arc<T>) -> Fut + Send,
        Fut: Future<Output = ()> + Send,
    {
        self.listen_from(Version::new(), on_message)
    }

    /// Observe every message not already causally contained in `since`.
    ///
    /// The cursor is a causal [`Version`], not a tree: each time the shared
    /// state changes, the listener fires `on_message` for precisely the live
    /// messages whose versions the cursor does not dominate, then absorbs the
    /// snapshot's ceiling. Every message live at some pass is observed
    /// exactly once; a message inserted and redacted wholly between passes is
    /// never observed (already-redacted content is never delivered);
    /// redactions themselves are honored silently. Delivery order is
    /// unspecified and does *not* follow the causal order: a message may be
    /// observed before another that causally precedes it. Order by the
    /// [`Version`] handed to the callback if causality matters.
    ///
    /// Consuming `self` dissolves this handle: a listener is an observer, not
    /// an actor. It does not hold the rumor set open, and it does not block
    /// the future from [`Known::broadcast`] that reunites the [`Known`].
    ///
    /// The future resolves once no further change is possible — after the
    /// [`Known`] and every [`Broadcast`] have dropped — having observed the
    /// complete final state, and yields the version up to which it processed:
    /// a valid `since` for a later `listen_from` against any replica of the
    /// same network. To resume after *cancelling* a listener (dropping the
    /// future loses its internal cursor), fold the observed versions in the
    /// callback (`resume |= version`) and pass the fold as `since`; every
    /// previously-fired message is contained in the fold, so nothing
    /// re-fires.
    pub fn listen_from<F, Fut>(
        self,
        since: Version,
        mut on_message: F,
    ) -> impl Future<Output = Version> + Send
    where
        T: Send + Sync,
        F: FnMut(Key, &Version, &Arc<T>) -> Fut + Send,
        Fut: Future<Output = ()> + Send,
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
                for (key, version, message) in snapshot.range(causally::since(&cursor)) {
                    on_message(key, version, message).await;
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
                    break cursor;
                }
            }
        }
    }

    /// Force this set's tree to compute its lazy structural memos (observable
    /// hash and ceiling/floor version bounds), so a subsequent operation is
    /// timed against its own work. For benchmark and test calibration only.
    #[doc(hidden)]
    pub fn warm_caches(&self) {
        self.known.warm_caches();
    }
}
