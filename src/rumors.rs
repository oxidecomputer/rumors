mod causal;
mod changes;
mod unordered;

pub use causal::CausalMessages;
pub use changes::{Changes, TryTick};
pub use unordered::{TryNext, UnorderedMessages};

use crate::bookmark::{Bookmark, BookmarkError, NoBookmark};
use crate::mode::{Async, Blocking, Mode};
use crate::{Batch, Error, Gossiped, Key, Network, Peer, Snapshot, Version};
use borsh::{BorshDeserialize, BorshSerialize};
use futures::Stream;
use futures::io::AllowStdIo;
use std::io::{Read, Write};
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::watch,
};
use tokio_util::compat::{FuturesAsyncReadCompatExt, FuturesAsyncWriteCompatExt};

/// A handle for [`send`](Rumors::send)ing and [`redact`](Rumors::redact)ing
/// messages, and [`gossip`](Rumors::gossip)ing the result with peers.
///
/// Unlike [`Peer`], [`Rumors`] is [`Clone`], which means that any number of
/// tasks may concurrently interact with the set of rumors, arbitrarily.
/// Synchronization is internal: anything one clone learns, all do.
pub struct Rumors<T, B: BookmarkError = NoBookmark, M: Mode = Async> {
    peer: Peer<T, B, M>,
    /// This handle's claim to existence; see [`Extant`].
    extant: Extant,
}

/// One handle's share of a [`Rumors`] generation's existence. The `token`
/// [`Arc`]'s strong count *is* the number of extant handles (a pending
/// [`try_into_peer`](Rumors::try_into_peer) has already shed its share), so the
/// count reaching zero is the moment the generation has quiesced and the
/// [`Peer`] may be reclaimed.
#[derive(Clone)]
struct Extant {
    /// The extancy token. An `Option` only so [`Drop`] can shed it *before*
    /// waking waiters on `drops`: a reuniter woken by that send must already
    /// observe the decremented strong count. Always `Some` outside `Drop`.
    token: Option<Arc<()>>,
    /// The exactly-once claim on the reclaimed [`Peer`]: among reuniters
    /// that observe quiescence concurrently, the one that wins this flag is
    /// handed the `Peer`; the rest resolve `None`.
    claimed: Arc<AtomicBool>,
    /// Wakes pending reuniters after each handle's token drops. Nothing
    /// meaningful is ever sent; only the version bump matters.
    drops: watch::Sender<()>,
}

impl Drop for Extant {
    fn drop(&mut self) {
        // Shed the token first, then wake: see the field docs above.
        self.token = None;
        self.drops.send_replace(());
    }
}

impl<T, B: BookmarkError, M: Mode> Clone for Rumors<T, B, M> {
    fn clone(&self) -> Self {
        Self {
            peer: Peer {
                network: self.peer.network,
                inner: self.peer.inner.clone(),
                bookmark: Arc::clone(&self.peer.bookmark),
                marker: PhantomData,
            },
            extant: self.extant.clone(),
        }
    }
}

/// A summary view (network, latest version, live-message count), independent
/// of `T: Debug`: the messages themselves are not printed.
impl<T, B: BookmarkError, M: Mode> std::fmt::Debug for Rumors<T, B, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.peer.inner.borrow();
        f.debug_struct("Rumors")
            .field("network", &self.peer.network)
            .field("latest", inner.tree.latest())
            .field("len", &inner.tree.len())
            .finish_non_exhaustive()
    }
}

impl<T, B: BookmarkError, M: Mode> Rumors<T, B, M> {
    /// Assemble the first handle of a fresh broadcast generation around `peer`,
    /// the only constructor: every other handle is a [`Clone`] of this one, so
    /// the token count faithfully counts handles.
    pub(crate) fn new(peer: Peer<T, B, M>) -> Self {
        Self {
            peer,
            extant: Extant {
                token: Some(Arc::new(())),
                claimed: Arc::new(AtomicBool::new(false)),
                drops: watch::Sender::new(()),
            },
        }
    }

    /// The mode-agnostic body behind the async and blocking
    /// [`try_into_peer`](Rumors::try_into_peer); see those for the public
    /// contract.
    pub(crate) async fn try_into_peer_inner(self) -> Option<Peer<T, B, M>> {
        let Self { peer, extant } = self;
        let token = Arc::downgrade(extant.token.as_ref().expect("Some outside Drop"));
        let claimed = Arc::clone(&extant.claimed);
        // Subscribe before shedding our token, so no later drop's wake can be
        // missed; our own shed below wakes us once, harmlessly.
        let mut drops = extant.drops.subscribe();
        drop(extant);
        loop {
            // Monotone once zero: minting a token takes a live `Rumors` to
            // clone, and every reuniter has already shed its own.
            if token.strong_count() == 0 {
                // Exactly one reuniter wins the claim; the Peer/Rumors
                // XOR is restored the instant this swap succeeds.
                return claimed
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                    .then_some(peer);
            }
            // `Err` here means every sender — every `Extant` — is gone, so
            // the count re-check above terminates the loop.
            let _ = drops.changed().await;
        }
    }

    /// Send a message.
    ///
    /// Returns a [`Batch`] that commits when dropped: a bare
    /// `rumors.send(message);` commits at the end of the statement, and
    /// chaining further [`send`](Batch::send)s and [`redact`](Batch::redact)s
    /// accumulates them into one commit.
    ///
    /// # Panics
    ///
    /// If `message` fails to serialize (see [`Batch::send`]).
    pub fn send(&self, message: T) -> Batch<'_, T>
    where
        T: BorshSerialize + Send + Sync,
    {
        self.peer.send(message)
    }

    /// Redact a message: remove the live message named by `key` from the set,
    /// here and — through gossip — everywhere. Redacting a key not currently
    /// held is a no-op.
    ///
    /// Returns a [`Batch`] that commits when dropped: a bare
    /// `rumors.redact(key);` commits at the end of the statement, and chaining
    /// further [`send`](Batch::send)s and [`redact`](Batch::redact)s
    /// accumulates them into one commit.
    pub fn redact(&self, key: Key) -> Batch<'_, T>
    where
        T: Send + Sync,
    {
        self.peer.redact(key)
    }

    /// Start an empty [`Batch`], for committing several changes as one
    /// atomic unit: observers and concurrent gossip sessions see either
    /// none of the batch or all of it, never a prefix.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::Peer;
    ///
    /// let rumors = Peer::<String>::seed().into_rumors();
    /// rumors
    ///     .batch()
    ///     .send("a".to_string())
    ///     .send("b".to_string());
    /// // The batch committed, atomically, when the statement ended.
    /// assert_eq!(rumors.snapshot().len(), 2);
    /// ```
    pub fn batch(&self) -> Batch<'_, T>
    where
        T: Send + Sync,
    {
        self.peer.batch()
    }

    /// The identifier shared by every peer that descends from the same
    /// [`seed`](Peer::seed).
    pub fn network(&self) -> Network {
        self.peer.network()
    }

    /// Take a consistent point-in-time view of the live set: cheap
    /// (structure-sharing, no copy), atomic, and isolated from every later
    /// change. See [`Snapshot`] for what it can answer.
    pub fn snapshot(&self) -> Snapshot<T> {
        self.peer.snapshot()
    }

    /// Monitor every message sent to in this [`Rumors`], in arbitrary
    /// (*non-causal*) order.
    ///
    /// See [`UnorderedMessages`] for details.
    pub fn unordered_messages(&self) -> UnorderedMessages<T, M>
    where
        T: Send + Sync,
    {
        self.peer.unordered_messages()
    }

    /// Monitor every message sent to this [`Rumors`] which is not already
    /// causally contained in `since`, then everything learned afterwards, in
    /// arbitrary (*non-causal*) order.
    pub fn unordered_messages_since(&self, since: Version) -> UnorderedMessages<T, M>
    where
        T: Send + Sync,
    {
        self.peer.messages_since(since)
    }

    /// Monitor every message sent to this [`Rumors`], in *causal order*.
    ///
    /// See [`CausalMessages`] for details.
    pub fn causal_messages(&self) -> CausalMessages<T, M>
    where
        T: Send + Sync,
    {
        self.peer.causal_messages()
    }

    /// Monitor every message sent to this [`Rumors`] which is not already
    /// causally contained in `since`, in *causal order*.
    ///
    /// See [`CausalMessages`] for details.
    pub fn causal_messages_since(&self, since: Version) -> CausalMessages<T, M>
    where
        T: Send + Sync,
    {
        self.peer.causal_messages_since(since)
    }

    /// Observe *that* this [`Rumors`] changes, without observing what changed.
    ///
    /// The result is a coalescing stream that yields `()` immediately on first
    /// poll and then once per observed advance of the set's causal frontier.
    ///
    /// See [`Changes`] for details.
    pub fn changes(&self) -> Changes<T, M> {
        Changes::subscribe(&self.peer.inner)
    }

    /// Force this set's tree to compute its lazy structural memos (observable
    /// hash and ceiling/floor version bounds), so a subsequent operation is
    /// timed against its own work. For benchmark and test calibration only.
    #[doc(hidden)]
    pub fn warm_caches(&self) {
        self.peer.warm_caches();
    }

    /// Alias this set's live party for invariant assertions in tests; see
    /// [`Peer::dangerously_alias_party`] for the contract.
    #[cfg(any(test, feature = "test-internals"))]
    #[doc(hidden)]
    pub fn dangerously_alias_party(&self) -> Option<before::Party> {
        self.peer.dangerously_alias_party()
    }
}

impl<T, B: Bookmark> Rumors<T, B, Async> {
    /// Give up this handle and reclaim the [`Peer`] once no more other handles
    /// exist: resolves when no [`Rumors`] for this set remains, handing the
    /// `Peer` to exactly one caller.
    ///
    /// Cancelling a pending [`try_into_peer`](Self::try_into_peer) abandons its
    /// claim: the handle was already consumed, so dropping the future is no
    /// different from having dropped the `Rumors`. If every handle goes away
    /// with no [`try_into_peer`](Self::try_into_peer) pending, the `Peer` is
    /// gone for good: observers drain the final state and stop.
    pub async fn try_into_peer(self) -> Option<Peer<T, B, Async>> {
        self.try_into_peer_inner().await
    }

    /// Run one reconciliation session with one remote peer over the given
    /// transport.
    ///
    /// On `Ok`, both replicas hold every message either one held when the
    /// session began (the full contract, including failure and cancellation
    /// semantics, is in the [crate docs](crate#what-a-session-promises)).
    ///
    /// Gossip sessions may run concurrently on different clones of the same
    /// [`Rumors`]; each commits atomically when it completes.
    ///
    /// On `Ok`, the transport rests exactly at the session boundary, ready to
    /// host this pair's next session. On `Err`, the replica is unchanged, but
    /// the transport is mid-frame garbage: discard the connection rather than
    /// starting another session on it.
    pub async fn gossip<'a, R, W>(&self, read: &'a mut R, write: &'a mut W) -> Result<(), Error<B>>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        self.peer.gossip(read, write).await
    }

    /// Drive a long-lived connection: run one gossip session per `when` tick
    /// (if there's been local change since the last gossip), and serve every
    /// session the remote initiates, until `when` ends or the connection fails.
    ///
    /// `when` defines the local initiation policy: providing
    /// [`self.changes()`](Self::changes) implements push-on-change; an interval
    /// stream gossips regularly; adding debounce/jitter/rate-limit adapters can
    /// set cadence; an always-pending stream only ever serves in response to
    /// remote initiation.
    ///
    /// Do not provide an always-ready stream (e.g.
    /// [`stream::repeat`](futures::stream::repeat)), because this would
    /// busy-loop: `when` should go quiet between reasons to gossip.
    ///
    /// The returned stream from this method *must be polled* for gossip to
    /// continue. It yields one [`Gossiped`] per completed gossip session. It
    /// terminates in one of three ways:
    ///
    /// - the connection fails: one final `Err` (replica unchanged, the
    ///   transport is now mid-frame garbage: discard the transport);
    /// - `when` ends, cleanly, after finishing any session in flight;
    /// - the remote hangs up at a session boundary, cleanly.
    ///
    /// # Suppression
    ///
    /// A tick from the `when` stream initiates gossip only if the local
    /// [`Rumors`] has advanced past this connection's last
    /// [`converged`](Gossiped::converged) version. Providing
    /// [`changes`](Self::changes) as `when` therefore never echoes a session
    /// back after its own gossip. However, a local tick from the `when` stream
    /// never *pulls* from the other side: each side pushes its own news, so
    /// probing a silent connection for liveness must be the transport's job
    /// (e.g. TCP keepalives), not the `when`-stream's.
    ///
    /// # Cancellation and connection reuse
    ///
    /// Futures derived from polling the result-stream are cancel-safe: all
    /// driver state lives in the stream itself. Dropping the result stream,
    /// however, is *not* cancellation-safe.
    ///
    /// # Examples
    ///
    /// Two replicas keep one connection converged, each end driving with
    /// its own change signal:
    ///
    /// ```
    /// use futures::StreamExt;
    /// use rumors::Peer;
    ///
    /// # tokio::runtime::Builder::new_current_thread()
    /// #     .build()
    /// #     .unwrap()
    /// #     .block_on(async {
    /// let alice = Peer::<String>::seed().into_rumors();
    /// # let (near, far) = tokio::io::duplex(64 * 1024);
    /// # let serve = alice.clone();
    /// # let server = tokio::spawn(async move {
    /// #     let (mut read, mut write) = tokio::io::split(far);
    /// #     serve.gossip(&mut read, &mut write).await.unwrap();
    /// # });
    /// # let (mut read, mut write) = tokio::io::split(near);
    /// let bob = Peer::<String>::bootstrap(&mut read, &mut write)
    ///     .await?
    ///     .expect("alice is established")
    ///     .into_rumors();
    /// # server.await.unwrap();
    ///
    /// // A long-lived connection between them, one driver per end.
    /// let (alice_side, bob_side) = tokio::io::duplex(64 * 1024);
    /// let (mut a_read, mut a_write) = tokio::io::split(alice_side);
    /// let (mut b_read, mut b_write) = tokio::io::split(bob_side);
    ///
    /// alice.send("psst".to_string());
    ///
    /// let mut alice_drive = alice.gossip_when(alice.changes(), &mut a_read, &mut a_write);
    /// let mut bob_drive = bob.gossip_when(bob.changes(), &mut b_read, &mut b_write);
    ///
    /// // Alice's change signal initiates; Bob's driver serves. One session
    /// // converges the pair, and each driver reports it.
    /// let (pushed, served) = tokio::join!(alice_drive.next(), bob_drive.next());
    /// pushed.expect("driver running")?;
    /// served.expect("driver running")?;
    /// assert_eq!(bob.snapshot().len(), 1);
    /// # Ok::<(), rumors::Error>(())
    /// # })?;
    /// # Ok::<(), rumors::Error>(())
    /// ```
    pub fn gossip_when<'a, R, W, S>(
        &'a self,
        when: S,
        read: &'a mut R,
        write: &'a mut W,
    ) -> impl Stream<Item = Result<Gossiped, Error<B>>> + Unpin + 'a
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
        S: Stream<Item = ()> + 'a,
    {
        self.peer.gossip_when(when, read, write)
    }
}

impl<T, B: crate::sync::Bookmark + Send + Sync> Rumors<T, B, Blocking> {
    /// Blocking [`try_into_peer`](Rumors::try_into_peer).
    pub fn try_into_peer(self) -> Option<Peer<T, B, Blocking>> {
        pollster::block_on(self.try_into_peer_inner())
    }

    /// Blocking [`gossip`](Rumors::gossip) over [`std::io`].
    pub fn gossip<R, W>(&mut self, read: &mut R, write: &mut W) -> Result<(), Error<B>>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync,
        R: Read + Send,
        W: Write + Send,
    {
        let mut read = AllowStdIo::new(read).compat();
        let mut write = AllowStdIo::new(write).compat_write();
        pollster::block_on(self.peer.gossip(&mut read, &mut write))
    }
}
