mod causal;
mod changes;
mod unordered;

pub use causal::CausalMessages;
pub use changes::{Changes, TryTick};
pub use unordered::{TryNext, UnorderedMessages};

use crate::bookmark::{Bookmark, BookmarkError, NoBookmark};
use crate::link::{Acceptor, Connector, Link};
use crate::message::EncodeError;
use crate::{Batch, Error, Gossiped, Network, Peer, Snapshot, Version};
use futures::Stream;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::watch,
};

/// A handle for [`send`](Rumors::send)ing and [`redact`](Rumors::redact)ing
/// messages, and [`gossip`](Rumors::gossip)ing the result with peers.
///
/// Unlike [`Peer`], [`Rumors`] is [`Clone`]: any number of tasks may
/// interact with the set concurrently. Synchronization is internal:
/// anything one clone learns, all do.
pub struct Rumors<T, B: BookmarkError = NoBookmark> {
    peer: Peer<T, B>,
    /// This handle's claim to existence; see [`Extant`].
    extant: Extant,
}

/// One handle's share of a [`Rumors`] generation's existence.
///
/// The `token` [`Arc`]'s strong count *is* the number of extant handles (a
/// pending [`try_into_peer`](Rumors::try_into_peer) has already shed its
/// share), so the count reaching zero is the moment the generation has quiesced
/// and the [`Peer`] may be reclaimed.
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

/// Share the replica, configuration, and storage while retaining a handle claim.
impl<T, B: BookmarkError> Clone for Rumors<T, B> {
    /// Create another handle to the same peer.
    fn clone(&self) -> Self {
        Self {
            peer: Peer {
                network: self.peer.network,
                window: self.peer.window,
                run_budget: self.peer.run_budget,
                gossip_policy: self.peer.gossip_policy.clone(),
                inner: self.peer.inner.clone(),
                bookmark: Arc::clone(&self.peer.bookmark),
                codec: self.peer.codec,
                observe: self.peer.observe.clone(),
            },
            extant: self.extant.clone(),
        }
    }
}

/// A summary view (network, latest version, live-message count), independent
/// of `T: Debug`: the messages themselves are not printed.
impl<T, B: BookmarkError> std::fmt::Debug for Rumors<T, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.peer.inner.borrow();
        f.debug_struct("Rumors")
            .field("network", &self.peer.network)
            .field("latest", inner.tree.latest())
            .field("len", &inner.tree.len())
            .finish_non_exhaustive()
    }
}

impl<T, B: BookmarkError> Rumors<T, B> {
    /// Assemble the first handle of a fresh broadcast generation around `peer`,
    /// the only constructor: every other handle is a [`Clone`] of this one, so
    /// the token count faithfully counts handles.
    pub(crate) fn new(peer: Peer<T, B>) -> Self {
        Self {
            peer,
            extant: Extant {
                token: Some(Arc::new(())),
                claimed: Arc::new(AtomicBool::new(false)),
                drops: watch::Sender::new(()),
            },
        }
    }

    /// Await quiescence and restore the unique [`Peer`] handle.
    async fn try_into_peer_inner(self) -> Option<Peer<T, B>> {
        let Self { peer, extant } = self;
        let token = Arc::downgrade(extant.token.as_ref().expect("Some outside Drop"));
        let claimed = Arc::clone(&extant.claimed);
        // Subscribe before shedding our token, so no later drop's wake can be
        // missed; our own shed below wakes us once, harmlessly.
        let mut drops = extant.drops.subscribe();
        drop(extant);
        loop {
            // Monotone once zero: creating a token takes a live `Rumors` to
            // clone, and every reuniter has already shed its own.
            if token.strong_count() == 0 {
                // Exactly one reuniter wins the claim; the Peer/Rumors
                // XOR is restored the instant this swap succeeds.
                return claimed
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                    .then_some(peer);
            }
            // `Err` here means every sender (every `Extant`) is gone, so
            // the count re-check above terminates the loop.
            let _ = drops.changed().await;
        }
    }

    /// Send a message, committing it immediately.
    ///
    /// The message is serialized and admitted here, at the call:
    /// admission runs the exact decode every receiver's wire ingress
    /// runs, so a payload a receiver would reject or misread is the
    /// typed [`EncodeError`] instead (its variants name the causes),
    /// and nothing commits. To send many messages in one commit, use
    /// [`send_all`](Self::send_all); to mix sends and redactions in one
    /// commit, use [`batch`](Self::batch).
    ///
    /// `send` does not return the message's [`Version`]. Versions come back
    /// through observation: the observers and [`Snapshot`] attach every
    /// message to the version its send created, unique across the universe's
    /// whole history, so even byte-identical re-sends are distinct messages
    /// under distinct versions. [`redact`](Self::redact) states the intended
    /// observe-then-redact pattern and why the write path returns no
    /// version.
    ///
    /// # Observe-then-send is domination
    ///
    /// Every message this replica observed before a commit is in the
    /// causal past of that commit's sends, which is the supersession
    /// contract last-write-wins patterns lean on. The boundary: sends from
    /// different threads or different batches carry **no** guaranteed
    /// causal relationship to one another unless the application
    /// synchronizes them itself.
    ///
    /// # Panics
    ///
    /// If `message` fails to serialize: a violation of the payload
    /// contract ([choosing a payload
    /// type](crate#choosing-a-payload-type)).
    pub fn send(&self, message: T) -> Result<(), EncodeError>
    where
        T: Send + Sync + 'static,
    {
        self.peer.send(message)
    }

    /// Redact a message: remove the live message stamped with `version`
    /// from the set, here and, through gossip, everywhere, committing
    /// immediately.
    ///
    /// Redacting a version not currently held is a no-op, and redaction
    /// is infallible: no payload is created, so no depth admission
    /// applies. To redact many versions in one commit, use
    /// [`redact_all`](Self::redact_all); to mix redactions and sends in
    /// one commit, use [`batch`](Self::batch).
    ///
    /// # Deletion is honored
    ///
    /// Once a redaction commits anywhere, no gossip schedule re-establishes
    /// the redacted message from replicas that still hold it. Nothing
    /// crosses the wire to represent a deletion; reconciliation infers
    /// deletions from the causal frontiers the two sides exchange. A
    /// message the counterparty's version shows it must already have seen,
    /// yet it no longer holds, was deleted there, so the holder drops its
    /// own copy instead of transmitting it. And because every send creates a
    /// fresh version, re-sending byte-identical content after a redaction
    /// is a *new* message: no resurrection, no suppression. For the same
    /// reason, two identical sends are two messages, and redacting one
    /// never touches the other.
    ///
    /// # Where the version comes from
    ///
    /// [`send`](Self::send) does not return a [`Version`], deliberately,
    /// for two reasons. The intended shape of an application is a state
    /// machine driven from observed messages: the observers and
    /// [`Snapshot`] attach every message to its version, so the read path,
    /// not the write path, is where a version-holding workflow like
    /// send-then-redact lives. Observe your own message back out, keep its
    /// version, redact it later. And batching breaks the correspondence
    /// anyway: a batch inserts all its messages at once, so sends are not
    /// 1:1 with insertions and a message's version is not knowable until
    /// insertion.
    pub fn redact(&self, version: &Version)
    where
        T: Send + Sync,
    {
        self.peer.redact(version)
    }

    /// Send every message `messages` yields, as one all-or-nothing commit.
    ///
    /// Equivalent to a [`batch`](Self::batch) whose whole body is one
    /// [`Batch::send_all`]: observers and concurrent gossip sessions see
    /// all of them land at once, in at most one observer wakeup, and every
    /// message is in the causal past of the same commit. Admission runs per
    /// message, exactly as [`send`](Self::send) states; the first message
    /// a receiver would reject or misread is the returned [`EncodeError`],
    /// and then **nothing** commits, not even the messages admitted before
    /// it. An empty iterator commits nothing and wakes no observer.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::{EncodeError, Peer};
    ///
    /// let rumors = Peer::<u64>::seed().into_rumors();
    /// rumors.send_all(0..10)?;
    /// // All ten landed, in one commit.
    /// assert_eq!(rumors.snapshot().len(), 10);
    /// # Ok::<(), EncodeError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// If any message fails to serialize: a violation of the payload
    /// contract ([choosing a payload
    /// type](crate#choosing-a-payload-type)), exactly as
    /// [`send`](Self::send) treats it.
    pub fn send_all<I>(&self, messages: I) -> Result<(), EncodeError>
    where
        T: Send + Sync + 'static,
        I: IntoIterator<Item = T>,
    {
        self.peer.send_all(messages)
    }

    /// Redact every version `versions` yields, as one commit.
    ///
    /// Equivalent to a [`batch`](Self::batch) whose whole body is one
    /// [`Batch::redact_all`]: observers and
    /// concurrent gossip sessions see every redaction land at once, in at
    /// most one observer wakeup. Like [`redact`](Self::redact) it is
    /// infallible, and versions not currently held are skipped. An empty
    /// iterator commits nothing and wakes no observer.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::{EncodeError, Peer, Version};
    ///
    /// let rumors = Peer::<u64>::seed().into_rumors();
    /// rumors.send_all(0..10)?;
    /// // Observe the versions back out, then redact the even payloads.
    /// let evens: Vec<Version> = rumors
    ///     .snapshot()
    ///     .iter()
    ///     .filter(|(_, message)| **message % 2 == 0)
    ///     .map(|(version, _)| version.clone())
    ///     .collect();
    /// rumors.redact_all(&evens);
    /// assert_eq!(rumors.snapshot().len(), 5);
    /// # Ok::<(), EncodeError>(())
    /// ```
    pub fn redact_all<'v, I>(&self, versions: I)
    where
        T: Send + Sync,
        I: IntoIterator<Item = &'v Version>,
    {
        self.peer.redact_all(versions)
    }

    /// Apply several changes in one all-or-nothing commit.
    ///
    /// Runs `f` with a [`Batch`] scope handle for queueing
    /// [`send`](Batch::send)s and [`redact`](Batch::redact)s, and
    /// commits everything queued **iff `f` returns `Ok`**: observers
    /// and concurrent gossip sessions see it all land as one commit,
    /// one tree traversal, and at most one observer wakeup. Any other
    /// exit — a returned `Err` (how a caller abandons a batch) or a
    /// panic — commits nothing.
    ///
    /// The closure is synchronous and the scope handle cannot leave it
    /// (the examples below show both escape routes failing to compile),
    /// so async cancellation cannot observe a half-built batch. The
    /// closure may use the same `Rumors` handle — a
    /// [`send`](Self::send), or a nested `batch` — and such nested
    /// operations commit first, as their own separate commits.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::{EncodeError, Peer};
    ///
    /// let rumors = Peer::<String>::seed().into_rumors();
    /// rumors.batch(|batch| {
    ///     batch.send("a".to_string())?;
    ///     batch.send("b".to_string())?;
    ///     Ok::<(), EncodeError>(())
    /// })?;
    /// // Both landed, in one commit.
    /// assert_eq!(rumors.snapshot().len(), 2);
    /// # Ok::<(), EncodeError>(())
    /// ```
    ///
    /// The scope handle cannot be stashed outside the closure:
    ///
    /// ```compile_fail
    /// let rumors = rumors::Peer::<String>::seed().into_rumors();
    /// let mut stash = None;
    /// let _ = rumors.batch::<_, (), _>(|batch| {
    ///     stash = Some(batch);
    ///     Ok(())
    /// });
    /// ```
    ///
    /// ...and cannot be returned out of it:
    ///
    /// ```compile_fail
    /// let rumors = rumors::Peer::<String>::seed().into_rumors();
    /// let escaped = rumors.batch::<_, (), _>(|batch| Ok(batch));
    /// ```
    pub fn batch<R, E, F>(&self, f: F) -> Result<R, E>
    where
        T: Send + Sync,
        F: for<'s> FnOnce(&'s mut Batch<'_, T>) -> Result<R, E>,
    {
        self.peer.batch(f)
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

    /// Monitor every message sent to this [`Rumors`], in arbitrary
    /// (*non-causal*) order.
    ///
    /// See [`UnorderedMessages`] for details.
    pub fn unordered_messages(&self) -> UnorderedMessages<T>
    where
        T: Send + Sync,
    {
        self.peer.unordered_messages()
    }

    /// Monitor every message sent to this [`Rumors`] which is not already
    /// causally contained in `since`, then everything learned afterwards, in
    /// arbitrary (*non-causal*) order.
    pub fn unordered_messages_since(&self, since: Version) -> UnorderedMessages<T>
    where
        T: Send + Sync,
    {
        self.peer.messages_since(since)
    }

    /// Monitor every message sent to this [`Rumors`], in *causal order*.
    ///
    /// See [`CausalMessages`] for details.
    pub fn causal_messages(&self) -> CausalMessages<T>
    where
        T: Send + Sync,
    {
        self.peer.causal_messages()
    }

    /// Monitor every message sent to this [`Rumors`] which is not already
    /// causally contained in `since`, in *causal order*.
    ///
    /// See [`CausalMessages`] for details.
    pub fn causal_messages_since(&self, since: Version) -> CausalMessages<T>
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
    pub fn changes(&self) -> Changes<T> {
        Changes::subscribe(&self.peer.inner)
    }

    /// Alias this set's live party for invariant assertions in tests; see
    /// [`Peer::dangerously_alias_party`] for what the caller must uphold.
    #[cfg(any(test, feature = "test-internals"))]
    #[doc(hidden)]
    pub fn dangerously_alias_party(&self) -> before::Party {
        self.peer.dangerously_alias_party()
    }
}

/// Drive replication and recover exclusive ownership of the peer.
impl<T, B: Bookmark> Rumors<T, B> {
    /// Give up this handle and reclaim the [`Peer`]: resolves when no
    /// [`Rumors`] for this set remains, handing the `Peer` to exactly one
    /// caller.
    ///
    /// Cancelling a pending [`try_into_peer`](Self::try_into_peer) abandons its
    /// claim: the handle was already consumed, so dropping the future is no
    /// different from having dropped the `Rumors`. If every handle goes away
    /// with no [`try_into_peer`](Self::try_into_peer) pending, the `Peer` is
    /// gone for good: observers drain the final state and stop.
    pub async fn try_into_peer(self) -> Option<Peer<T, B>> {
        self.try_into_peer_inner().await
    }

    /// Run one reconciliation session with one remote peer over the given
    /// [`Link`].
    ///
    /// `Ok` carries the session's [`Gossiped`]: the converged version and
    /// the session's [`SessionStats`](crate::SessionStats). Its `led` is
    /// always [`Led::Local`](crate::Led::Local): calling `gossip_once` is this
    /// side's initiation, and a remote initiation already in flight merges
    /// into the same session, exactly as racing
    /// [`gossip`](Self::gossip) triggers do.
    ///
    /// On `Ok`, both replicas hold every message either one held when the
    /// session began **and neither had deleted**, and the peer has
    /// confirmed that it completed and committed the session too. The
    /// link rests exactly at the session boundary, ready to host this
    /// pair's next session.
    ///
    /// The configured [`Peer::session_deadline`] applies to this exchange.
    /// The initiation policy does not: this call always starts a session.
    ///
    /// On failure or cancellation, discard the poisoned link and reconnect.
    /// No partial reconciliation is published, but a completed local commit
    /// is not undone: a failure in [`Phase::Completion`](crate::error::Phase::Completion)
    /// leaves the peer's commit unconfirmed. Accepting a retirement can also
    /// commit content before a bookmark write fails; [`Error::Bookmark`]
    /// explains how to recover. See the [session contract](crate::link::Link#what-a-session-promises).
    ///
    /// An established peer automatically serves bootstrappers and accepts
    /// retirements through this same call.
    ///
    /// Sessions may run concurrently on separate links, through the same or
    /// different handles. Each publishes its reconciled content atomically.
    /// The `&mut Link` borrow prevents overlapping sessions on one link.
    /// A bookmarked peer also serializes storage access; see [`Bookmark`].
    pub async fn gossip_once<CR, CW, C, A>(
        &self,
        link: &mut Link<CR, CW, C, A>,
    ) -> Result<Gossiped, Error<B>>
    where
        T: Send + Sync + 'static,
        CR: AsyncRead + Unpin + Send,
        CW: AsyncWrite + Unpin + Send,
        C: Connector,
        A: Acceptor,
    {
        self.peer.gossip_once(link).await
    }

    /// Keep one link synchronized, yielding an outcome after each gossip session.
    ///
    /// The driver serves remote initiations and follows the local policy
    /// selected by [`Peer::gossip_when`]. The default policy starts
    /// immediately, then pushes local changes. Configure
    /// [`Peer::session_deadline`] to bound active sessions without timing idle
    /// waits.
    ///
    /// Each successful [`Gossiped`] reports the converged version, initiation
    /// direction, and statistics. You can discard these if you don't care about
    /// them, but **keep polling the resultant stream to drive the connection.**
    ///
    /// To force one exchange regardless of the initiation policy, use
    /// [`gossip_once`](Self::gossip_once).
    ///
    /// # Ending the driver
    ///
    /// - An error, including deadline expiry, yields one final `Err` and ends the
    ///   stream. Discard the poisoned link and reconnect.
    /// - The local policy ending stops the driver after any active session.
    ///   The link remains usable by another driver. A shared signal can end the
    ///   policies across all handles; see [`Peer::gossip_when`]'s graceful-shutdown
    ///   example. Keep polling each driver until it ends.
    /// - A remote hang-up between sessions ends the stream cleanly. Reconnect
    ///   to reach that peer again.
    ///
    /// # Deadlines and cancellation
    ///
    /// A configured deadline starts when the local policy initiates a session or
    /// the first remote bytes arrive. It covers the entire exchange through confirmation of
    /// completion, without being reset by traffic. When both the exchange and its
    /// deadline are ready, the exchange's result takes precedence.
    ///
    /// Dropping a `next()` future preserves the active session and its deadline
    /// inside the stream. Dropping the stream cancels any active session and
    /// poisons the link. Dropping it between completed sessions is safe. To stop
    /// cleanly regardless of timing, end the policy stream and drain the driver.
    ///
    /// Failure or cancellation publishes no partial reconciliation, but does not
    /// undo a local commit already made. In particular, expiry while awaiting
    /// completion can leave the remote's commit unconfirmed. Accepting a retirement
    /// can also commit content before a bookmark write fails; [`Error::Bookmark`]
    /// explains recovery. See the [session contract](crate::link::Link#what-a-session-promises).
    ///
    /// Separate links can run concurrently through the same or different handles.
    /// Each publishes its reconciled content atomically. The mutable link borrow
    /// prevents concurrent sessions on one link; [`Bookmark`] storage access is
    /// also serialized.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # async fn example(rumors: &rumors::Rumors<String>, link: &mut rumors::link::MemoryLink)
    /// # -> Result<(), rumors::Error> {
    /// use futures::StreamExt;
    ///
    /// let mut sessions = rumors.gossip(link);
    /// while let Some(session) = sessions.next().await {
    ///     let completed = session?;
    ///     // Inspect `completed` or continue driving the connection.
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "the driver does nothing until the returned stream is polled"]
    pub fn gossip<'a, CR, CW, C, A>(
        &'a self,
        link: &'a mut Link<CR, CW, C, A>,
    ) -> impl Stream<Item = Result<Gossiped, Error<B>>> + Unpin + 'a
    where
        T: Send + Sync + 'static,
        CR: AsyncRead + Unpin + Send,
        CW: AsyncWrite + Unpin + Send,
        C: Connector,
        A: Acceptor,
    {
        self.peer.gossip_driver(link)
    }
}
