mod causal;
mod changes;
mod channel;
mod unordered;

pub use causal::CausalMessages;
pub use changes::{Changes, TryTick};
pub use unordered::{TryNext, UnorderedMessages};

use crate::bookmark::{Bookmark, NoBookmark};
use crate::message::EncodeError;
use crate::{
    Batch, Committed, Error, Gossiped, Network, Peer, Snapshot, SynchronizationSettings, Version,
};
use futures::stream::FusedStream;
use std::borrow::Borrow;
use std::sync::{Arc, Mutex, PoisonError};
use tokio::{
    io::{AsyncBufRead, AsyncWrite},
    sync::watch,
};

/// A handle for [`send`](Rumors::send)ing and [`redact`](Rumors::redact)ing
/// messages, and [`gossip`](Rumors::gossip)ing the result with peers.
///
/// Unlike [`Peer`], [`Rumors`] is [`Clone`]: any number of tasks may interact
/// with the set concurrently. Synchronization is internal: anything one clone
/// learns, all do.
///
/// Call [`snapshot`](Self::snapshot) to read a consistent view of the set.
pub struct Rumors<T: Send + Sync + 'static, B: Bookmark = NoBookmark> {
    /// A peer view sharing this replica's state and configuration.
    ///
    /// Declared before `reunion`, so that dropping a handle drops its view
    /// before its `alive` sender, as [`Reunion`] requires.
    peer: Peer<T, B>,
    /// Tracks live handles and holds the view that reunion hands back.
    reunion: Reunion<T, B>,
}

/// Tracks when a [`Rumors`] generation may be reunited into its [`Peer`], and
/// holds the view that reunion hands back.
///
/// Each handle owns one `alive` sender. Closing that channel therefore marks
/// the point when every handle has either been dropped or begun
/// [`try_into_peer`](Rumors::try_into_peer), and the [`Peer`] may be reclaimed.
///
/// Every handle gives up its view before its `alive` sender: a reuniter parks
/// its view or drops it, and a dropped handle drops its fields in declaration
/// order. So once the channel closes, the parked view is the generation's only
/// view, and the reclaimed `Peer` owns its bookmark lock alone. That is what
/// lets reunion release a lock held by a leaked session future
/// ([`BookmarkHandle::release_abandoned_lock`](crate::bookmark::BookmarkHandle::release_abandoned_lock)).
struct Reunion<T: Send + Sync + 'static, B: Bookmark> {
    /// One sender per live handle; channel closure signals quiescence.
    alive: watch::Sender<()>,
    /// Where the first reuniter parks the view the reclaimed `Peer` is made
    /// from.
    parked: Parked<T, B>,
}

/// Count another handle and share the parking place, without requiring `Clone`
/// payloads or storage.
impl<T: Send + Sync + 'static, B: Bookmark> Clone for Reunion<T, B> {
    /// Add a sender for the new handle and share the parking place.
    fn clone(&self) -> Self {
        Self {
            alive: self.alive.clone(),
            parked: self.parked.clone(),
        }
    }
}

/// The view the reclaimed [`Peer`] is made from: the first reuniter of a
/// generation parks its view here, and later reuniters drop theirs.
///
/// This is also the exactly-once claim: among reuniters that observe quiescence
/// concurrently, the one whose [`claim`](Self::claim) finds the view is handed
/// the `Peer`; the rest resolve `None`. A cancelled reuniter leaves its parked
/// view for the others.
struct Parked<T: Send + Sync + 'static, B: Bookmark>(Arc<Mutex<Option<Peer<T, B>>>>);

/// Share the parking place, without requiring `Clone` payloads or storage.
impl<T: Send + Sync + 'static, B: Bookmark> Clone for Parked<T, B> {
    /// Share the same parking place.
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

/// Park and claim the view that reunion hands back.
impl<T: Send + Sync + 'static, B: Bookmark> Parked<T, B> {
    /// Create an empty parking place for a new generation.
    fn new() -> Self {
        Self(Arc::new(Mutex::new(None)))
    }

    /// Park `peer` if no view is parked yet; otherwise return it, for the
    /// caller to drop outside the lock, which only ever guards moving a view in
    /// or out.
    fn park(&self, peer: Peer<T, B>) -> Option<Peer<T, B>> {
        let mut parked = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if parked.is_some() {
            return Some(peer);
        }
        *parked = Some(peer);
        None
    }

    /// Take the parked view, if no other reuniter has taken it.
    fn claim(&self) -> Option<Peer<T, B>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).take()
    }
}

/// Share the replica, configuration, and storage while retaining a handle
/// claim.
impl<T: Send + Sync + 'static, B: Bookmark> Clone for Rumors<T, B> {
    /// Create another handle to the same peer.
    fn clone(&self) -> Self {
        Self {
            peer: self.peer.share(),
            reunion: self.reunion.clone(),
        }
    }
}

/// A bounded replica summary, independent of `T: Debug`.
impl<T: Send + Sync + 'static, B: Bookmark> std::fmt::Debug for Rumors<T, B> {
    /// Summarize the replica without printing payloads.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.peer.fmt_summary("Rumors", f)
    }
}

/// Manage the shared replica and its handle lifetime.
impl<T: Send + Sync + 'static, B: Bookmark> Rumors<T, B> {
    /// Assemble the first handle of a fresh [`Rumors`] generation around
    /// `peer`. Every other handle is a [`Clone`] of this one, so the channel's
    /// sender count faithfully counts handles.
    pub(crate) fn new(peer: Peer<T, B>) -> Self {
        Self {
            peer,
            reunion: Reunion {
                alive: watch::Sender::new(()),
                parked: Parked::new(),
            },
        }
    }

    /// Return the local settings used to configure synchronization sessions.
    pub fn synchronization_settings(&self) -> SynchronizationSettings {
        self.peer.synchronization_settings()
    }

    /// Await quiescence and restore the unique [`Peer`] handle.
    async fn try_into_peer_inner(self) -> Option<Peer<T, B>> {
        let Self {
            peer,
            reunion: Reunion { alive, parked },
        } = self;
        let mut closed = alive.subscribe();
        // Give up this view before this handle's `alive` sender.
        drop(parked.park(peer));
        drop(alive);

        // Nothing is sent on this channel. Closure means every handle is
        // gone, and no new sender can be cloned after that point.
        while closed.changed().await.is_ok() {}

        // Concurrent reuniters observe closure together. Only one `take`
        // finds the parked view, so exactly one restores the Peer/Rumors XOR.
        let mut peer = parked.claim()?;
        // No session future of this generation can run or be dropped again
        // now: every future that borrowed a handle has ended, since the
        // handle could not otherwise have been consumed or dropped, and a
        // future owning a handle would have kept the channel open. So a held
        // bookmark lock, or an outstanding fork reservation, belongs to a
        // leaked future that can never run again, and we release both.
        peer.bookmark = peer.bookmark.release_abandoned_lock();
        peer.release_abandoned_reservations();
        Some(peer)
    }

    /// Send a message, committing it immediately.
    ///
    /// The message is serialized and admitted here, at the call: admission runs
    /// the exact decode every receiver's wire ingress runs, so a payload a
    /// receiver would reject or misread is the typed [`EncodeError`] instead
    /// (its variants name the causes), and nothing commits. To send many
    /// messages in one commit, use [`send_all`](Self::send_all); to mix sends
    /// and redactions in one commit, use [`batch`](Self::batch).
    ///
    /// Returns the message's [`Version`]. This uniquely identifies this send
    /// throughout the network and can be passed directly to
    /// [`redact`](Self::redact). Even byte-identical re-sends receive distinct
    /// versions and remain distinct messages.
    ///
    /// # Observe-then-send is domination
    ///
    /// Every message this replica observed before a commit is in the causal
    /// past of that commit's sends, which is the supersession contract
    /// last-write-wins patterns lean on. The boundary: sends from different
    /// threads or different batches carry **no** guaranteed causal relationship
    /// to one another unless the application synchronizes them itself.
    ///
    /// # Panics
    ///
    /// If `message` fails to serialize: a violation of the payload contract
    /// ([choosing a payload type](crate#choosing-a-payload-type)).
    pub fn send(&self, message: T) -> Result<Version, EncodeError> {
        self.peer.send(message)
    }

    /// Redact a message: remove the live message stamped with `version` from
    /// the set, here and, through gossip, everywhere, committing immediately.
    ///
    /// Returns whether it removed a message: `true` if this replica held the
    /// message live, `false` if it never received it or had already redacted
    /// it. A redaction that returns `false` is a no-op, here and everywhere
    /// else: it advances no history. So a message this replica never received
    /// is live if it arrives later, and redacting it then removes it. A message
    /// this replica already redacted stays redacted: gossip honors that earlier
    /// redaction, so the message never becomes live here again.
    ///
    /// Redaction is infallible: no payload is created, so no depth admission
    /// applies. To redact many versions in one commit, use
    /// [`redact_all`](Self::redact_all); to mix redactions and sends in one
    /// commit, use [`batch`](Self::batch).
    ///
    /// ```
    /// use rumors::{EncodeError, Peer};
    ///
    /// let rumors = Peer::<u64>::seed().into_rumors();
    /// let version = rumors.send(7)?;
    /// assert!(rumors.redact(&version), "a held message is removed");
    /// assert!(!rumors.redact(&version), "a second redaction removes nothing");
    /// # Ok::<(), EncodeError>(())
    /// ```
    ///
    /// # Deletion is honored
    ///
    /// Once a redaction commits anywhere, no gossip schedule re-establishes the
    /// redacted message from replicas that still hold it. Nothing crosses the
    /// wire to represent a deletion; reconciliation infers deletions from the
    /// causal frontiers the two sides exchange. A message the counterparty's
    /// version shows it must already have seen, yet it no longer holds, was
    /// deleted there, so the holder drops its own copy instead of transmitting
    /// it. And because every send creates a fresh version, re-sending
    /// byte-identical content after a redaction is a *new* message: no
    /// resurrection, no suppression. For the same reason, two identical sends
    /// are two messages, and redacting one never touches the other.
    ///
    /// Use the [`Version`] returned by [`send`](Self::send), or one reported by
    /// an observer or [`Snapshot`]. Batched sends do not return versions
    /// because one call may insert several messages.
    pub fn redact(&self, version: &Version) -> bool {
        self.peer.redact(version)
    }

    /// Send every message `messages` yields, as one all-or-nothing commit.
    ///
    /// Equivalent to a [`batch`](Self::batch) whose whole body is one
    /// [`Batch::send_all`]: observers and concurrent gossip sessions see all of
    /// them land at once, in at most one observer wakeup, and every message is
    /// in the causal past of the same commit. Admission runs per message,
    /// exactly as [`send`](Self::send) states; the first message a receiver
    /// would reject or misread is the returned [`EncodeError`], and then
    /// **nothing** commits, not even the messages admitted before it. An empty
    /// iterator commits nothing and wakes no observer.
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
    /// If any message fails to serialize: a violation of the payload contract
    /// ([choosing a payload type](crate#choosing-a-payload-type)), exactly as
    /// [`send`](Self::send) treats it.
    pub fn send_all<I>(&self, messages: I) -> Result<(), EncodeError>
    where
        I: IntoIterator<Item = T>,
    {
        self.peer.send_all(messages)
    }

    /// Redact every version `versions` yields, as one commit.
    ///
    /// Equivalent to a [`batch`](Self::batch) whose whole body is one
    /// [`Batch::redact_all`]: observers and concurrent gossip sessions see
    /// every redaction land at once, in at most one observer wakeup. Like
    /// [`redact`](Self::redact) it is infallible, and versions not currently
    /// held are skipped. An empty iterator commits nothing and wakes no
    /// observer.
    ///
    /// Returns how many messages it removed: the distinct versions this replica
    /// held live. Each version it did not hold, or named twice, adds nothing,
    /// exactly as [`redact`](Self::redact) returning `false` does.
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
    /// assert_eq!(rumors.redact_all(evens), 5, "every even payload was held");
    /// assert_eq!(rumors.snapshot().len(), 5);
    /// # Ok::<(), EncodeError>(())
    /// ```
    pub fn redact_all<I>(&self, versions: I) -> usize
    where
        I: IntoIterator,
        I::Item: Borrow<Version>,
    {
        self.peer.redact_all(versions)
    }

    /// Apply several changes in one all-or-nothing commit.
    ///
    /// Runs `f` with a [`Batch`] scope handle for queueing
    /// [`send`](Batch::send)s and [`redact`](Batch::redact)s, and commits
    /// everything queued **iff `f` returns `Ok`**: observers and concurrent
    /// gossip sessions see it all land as one commit, one tree traversal, and
    /// at most one observer wakeup. Any other exit — a returned `Err` (how a
    /// caller abandons a batch) or a panic — commits nothing.
    ///
    /// On commit, returns [`Committed`]: the closure's value, and how many
    /// messages the batch's redactions removed, counted as
    /// [`redact_all`](Self::redact_all) counts them.
    ///
    /// The closure is synchronous and the scope handle cannot leave it (the
    /// examples below show both escape routes failing to compile), so async
    /// cancellation cannot observe a half-built batch. The closure may use the
    /// same `Rumors` handle — a [`send`](Self::send), or a nested `batch` — and
    /// such nested operations commit first, as their own separate commits.
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
    pub fn batch<R, E, F>(&self, f: F) -> Result<Committed<R>, E>
    where
        F: for<'s> FnOnce(&'s mut Batch<'_, T>) -> Result<R, E>,
    {
        self.peer.batch(f)
    }

    /// Return the gossip network this replica belongs to.
    pub fn network(&self) -> Network {
        self.peer.network()
    }

    /// Take a consistent point-in-time view of the live set: cheap
    /// (structure-sharing, no copy), atomic, and isolated from every later
    /// change. See [`Snapshot`] for what it can answer.
    pub fn snapshot(&self) -> Snapshot<T> {
        self.peer.snapshot()
    }

    /// Observe every message in this [`Rumors`], however it arrived, in
    /// arbitrary (*non-causal*) order.
    ///
    /// See [`UnorderedMessages`] for details.
    pub fn unordered_messages(&self) -> UnorderedMessages<T> {
        self.peer.unordered_messages()
    }

    /// Observe every message not causally contained in `since`, then everything
    /// learned afterwards, in arbitrary (*non-causal*) order.
    pub fn unordered_messages_since(&self, since: Version) -> UnorderedMessages<T> {
        self.peer.unordered_messages_since(since)
    }

    /// Observe every message in this [`Rumors`], however it arrived, in *causal
    /// order*.
    ///
    /// See [`CausalMessages`] for details.
    pub fn causal_messages(&self) -> CausalMessages<T> {
        self.peer.causal_messages()
    }

    /// Observe every message not causally contained in `since`, then everything
    /// learned afterwards, in *causal order*.
    ///
    /// See [`CausalMessages`] for details.
    pub fn causal_messages_since(&self, since: Version) -> CausalMessages<T> {
        self.peer.causal_messages_since(since)
    }

    /// Observe *that* this [`Rumors`] changes, without observing what changed.
    ///
    /// The result is a coalescing stream that yields `()` immediately on first
    /// poll and then once per observed advance of the set's causal frontier.
    ///
    /// See [`Changes`] for details.
    pub fn changes(&self) -> Changes<T> {
        self.peer.changes()
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
impl<T: Send + Sync + 'static, B: Bookmark> Rumors<T, B> {
    /// Give up this handle and reclaim the [`Peer`]: resolves when no
    /// [`Rumors`] for this set remains, handing the `Peer` to exactly one
    /// caller.
    ///
    /// Cancelling a pending [`try_into_peer`](Self::try_into_peer) abandons its
    /// claim: the handle was already consumed, so dropping the future is no
    /// different from having dropped the `Rumors`. If every handle goes away
    /// with no [`try_into_peer`](Self::try_into_peer) pending, the `Peer` is
    /// gone for good: observers drain the final state and stop.
    ///
    /// # Leaked sessions
    ///
    /// Leaking a session future (with [`mem::forget`](std::mem::forget), say)
    /// instead of dropping it skips its cleanup. A session leaked while it
    /// waits on this peer's [`Bookmark`] storage keeps that storage locked, and
    /// every later session through this generation's handles waits forever.
    /// Reclaiming the `Peer` releases that lock: the returned peer, and the
    /// handles made from it, run sessions normally, as long as the storage
    /// itself still serves them. Reclaiming cannot release a lock inside the
    /// storage: a session leaked while a `FileBookmark` load or store waits
    /// behind an earlier one, whose session was cancelled, leaves every later
    /// operation on that bookmark waiting forever. A session leaked while
    /// serving a joining peer also pauses this replica's reclamation of old
    /// identity space, and reclaiming the `Peer` resumes it.
    pub async fn try_into_peer(self) -> Option<Peer<T, B>> {
        self.try_into_peer_inner().await
    }

    /// Run one reconciliation session with one remote peer over one connection.
    ///
    /// `read` and `write` are the two halves of one reliable, ordered duplex
    /// byte stream to the peer; see the [transport
    /// requirements](crate#transport). The session takes them by value and
    /// returns them on success, so pass `&mut read, &mut write` to keep a
    /// connection you hold elsewhere.
    ///
    /// `Ok` carries the session's [`Gossiped`]: the converged version and the
    /// session's [`SessionStats`](crate::SessionStats). Its `led` is always
    /// [`Led::Local`](crate::Led::Local): calling `gossip_once` is this side's
    /// initiation, and a remote initiation already in flight merges into the
    /// same session, exactly as racing [`gossip`](Self::gossip) triggers do.
    ///
    /// On `Ok`, both replicas hold every message either one held when the
    /// session began **and neither had deleted**, and the peer has confirmed
    /// that it completed and committed the session too. The returned halves
    /// rest exactly at the session boundary, ready to host this pair's next
    /// session.
    ///
    /// The configured [`Peer::session_deadline`] applies to this exchange. The
    /// initiation policy does not: this call always starts a session.
    ///
    /// On failure or cancellation, the connection's position is unknown:
    /// discard it and reconnect. A later session on it gets no typed warning.
    /// No partial reconciliation is published, but a completed local commit is
    /// not undone: a failure in
    /// [`Phase::Completion`](crate::error::Phase::Completion) leaves the peer's
    /// commit unconfirmed. Accepting a retirement can also commit content
    /// before a bookmark write fails; [`Error::Bookmark`] explains how to
    /// recover. See [what a session promises](crate#what-a-session-promises).
    ///
    /// An established peer automatically serves bootstrappers and accepts
    /// retirements through this same call.
    ///
    /// Sessions may run concurrently on separate connections, through the same
    /// or different handles. Each publishes its reconciled content atomically.
    /// Ownership of the halves, or the `&mut` borrow of them, prevents
    /// overlapping sessions on one connection. A bookmarked peer also
    /// serializes storage access; see [`Bookmark`].
    pub async fn gossip_once<R, W>(&self, read: R, write: W) -> Result<(Gossiped, R, W), Error<B>>
    where
        R: AsyncBufRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        self.peer.gossip_once(read, write).await
    }

    /// Keep one connection synchronized, yielding an outcome after each gossip
    /// session.
    ///
    /// `read` and `write` are the two halves of one connection, as for
    /// [`gossip_once`](Self::gossip_once). The driver owns them for its
    /// lifetime; to reuse the connection after the policy ends, pass
    /// `&mut read, &mut write`.
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
    /// - An error, including deadline expiry, yields one final `Err` and ends
    ///   the stream. Discard the connection and reconnect.
    /// - The local policy ending stops the driver after any active session. The
    ///   connection rests at a session boundary and can serve another driver. A
    ///   shared signal can end the policies across all handles; see
    ///   [`Peer::gossip_when`]'s graceful-shutdown example. Keep polling each
    ///   driver until it ends.
    /// - A remote hang-up between sessions ends the stream cleanly. Reconnect
    ///   to reach that peer again.
    ///
    /// However it ends, the stream stays ended: it is a [`FusedStream`]. Once a
    /// poll has returned `None`, [`is_terminated`](FusedStream::is_terminated)
    /// returns `true` and every later poll returns `None`. After the final
    /// `Err`, the next poll is the one that returns `None`; until then
    /// `is_terminated` returns `false`.
    ///
    /// # Deadlines and cancellation
    ///
    /// A configured deadline starts when the local policy initiates a session
    /// or the first remote bytes arrive. It covers the entire exchange through
    /// confirmation of completion, without being reset by traffic. When both
    /// the exchange and its deadline are ready, the exchange's result takes
    /// precedence.
    ///
    /// Dropping a `next()` future preserves the active session and its deadline
    /// inside the stream. Dropping the stream cancels any active session and
    /// leaves the connection at an unknown position: discard it. Dropping it
    /// between completed sessions is safe. To stop cleanly regardless of
    /// timing, end the policy stream and drain the driver.
    ///
    /// Failure or cancellation publishes no partial reconciliation, but does
    /// not undo a local commit already made. In particular, expiry while
    /// awaiting completion can leave the remote's commit unconfirmed. Accepting
    /// a retirement can also commit content before a bookmark write fails;
    /// [`Error::Bookmark`] explains recovery. See [what a session
    /// promises](crate#what-a-session-promises).
    ///
    /// Separate connections can run concurrently through the same or different
    /// handles. Each publishes its reconciled content atomically. A driver owns
    /// its connection, which prevents concurrent sessions on it; [`Bookmark`]
    /// storage access is also serialized.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # async fn example(rumors: &rumors::Rumors<String>, connection: tokio::net::TcpStream)
    /// # -> Result<(), rumors::Error> {
    /// use futures::StreamExt;
    ///
    /// let (read, write) = connection.into_split();
    /// let read = tokio::io::BufReader::new(read);
    /// let mut sessions = rumors.gossip(read, write);
    /// while let Some(session) = sessions.next().await {
    ///     let completed = session?;
    ///     // Inspect `completed` or continue driving the connection.
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "the driver does nothing until the returned stream is polled"]
    pub fn gossip<'a, R, W>(
        &'a self,
        read: R,
        write: W,
    ) -> impl FusedStream<Item = Result<Gossiped, Error<B>>> + Unpin + 'a
    where
        R: AsyncBufRead + Unpin + Send + 'a,
        W: AsyncWrite + Unpin + Send + 'a,
    {
        self.peer.gossip_driver(read, write)
    }
}
