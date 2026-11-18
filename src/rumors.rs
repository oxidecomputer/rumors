mod causal;
mod changes;
mod channel;
mod unordered;

pub use causal::CausalMessages;
pub use changes::{Changes, TryTick};
pub use unordered::{TryNext, UnorderedMessages};

use crate::bookmark::{Bookmark, NoBookmark};
use crate::message::EncodeError;
use crate::{Batch, Committed, Error, Gossiped, Network, Peer, Settings, Snapshot, Version};
use futures::StreamExt;
use futures::stream::FusedStream;
use std::borrow::Borrow;
use std::sync::{Arc, Mutex, PoisonError};
use tokio::{
    io::{AsyncBufRead, AsyncWrite},
    sync::watch,
};

/// A handle for [`send`](Rumors::send)ing and [`redact`](Rumors::redact)ing
/// messages, and [`gossip`](Rumors::gossip)ing the results with peers.
///
/// Unlike [`Peer`], [`Rumors`] is [`Clone`]: any number of tasks may interact
/// with clones concurrently. All synchronization is internal and automatic:
/// anything one [`Rumors`] learns, all clones of it do.
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

/// Share the replica, configuration, and storage.
impl<T: Send + Sync + 'static, B: Bookmark> Clone for Rumors<T, B> {
    /// Create another handle referring to the same [`Peer`].
    fn clone(&self) -> Self {
        Self {
            peer: self.peer.share(),
            reunion: self.reunion.clone(),
        }
    }
}

/// A summary, independent of `T: Debug`.
impl<T: Send + Sync + 'static, B: Bookmark> std::fmt::Debug for Rumors<T, B> {
    /// Debug the replica without printing payloads.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.peer.fmt_summary("Rumors", f)
    }
}

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

    /// Send a new message, returning the sent message's [`Version`].
    ///
    /// The returned [`Version`] uniquely identifies this message throughout the
    /// gossip network and can be passed directly to [`redact`](Self::redact) in
    /// the future.
    ///
    /// To send many messages all at once, use [`send_all`](Self::send_all); to
    /// mix sends and redactions in one commit, use [`batch`](Self::batch).
    ///
    /// # Errors
    ///
    /// Before accepting the message, [`send`](Rumors::send) checks that every
    /// receiver would decode exactly the message you sent, returning an
    /// [`EncodeError`] if a check fails:
    ///
    /// - [`Serialize`](EncodeError::Serialize) if the message's
    ///   [`Serialize`](serde::Serialize) implementation fails;
    ///
    /// - [`Deserialize`](EncodeError::Deserialize) if its
    ///   [`Deserialize`](serde::Deserialize) implementation rejects that
    ///   serialization;
    /// - [`Roundtrip`](EncodeError::Roundtrip) if the result of serializing and
    ///   deserializing is something not equal to the original `message`;
    ///
    /// - [`TooDeep`](EncodeError::TooDeep) if serializing the message requires
    ///   deeper recursion than [`Peer::message_depth_limit`].
    ///
    /// A rejected message is neither stored nor sent.
    pub fn send(&self, message: T) -> Result<Version, EncodeError> {
        self.peer.send(message)
    }

    /// Redact an existing message by supplying its unique [`Version`],
    /// returning `true` if the message previously existed.
    ///
    /// This removes the message, if any exists, which is stamped with
    /// `version`. The removal propagates automatically during gossip.
    ///
    /// To redact many versions all at once, use
    /// [`redact_all`](Self::redact_all); to mix redactions and sends in one
    /// commit, use [`batch`](Self::batch).
    pub fn redact(&self, version: &Version) -> bool {
        self.peer.redact(version)
    }

    /// Send everything in `messages`, in one all-or-nothing commit.
    ///
    /// Returns the last message's [`Version`], or `None` for an empty iterator.
    pub fn send_all<I>(&self, messages: I) -> Result<Option<Version>, EncodeError>
    where
        I: IntoIterator<Item = T>,
    {
        self.peer.send_all(messages)
    }

    /// Redact everything in `versions`, in one commit.
    ///
    /// Returns how many messages it removed.
    pub fn redact_all<I>(&self, versions: I) -> usize
    where
        I: IntoIterator,
        I::Item: Borrow<Version>,
    {
        self.peer.redact_all(versions)
    }

    /// Apply several changes to the set of messages, atomically.
    ///
    /// Runs `f` with a [`Batch`] handle for queueing [`send`](Batch::send)s and
    /// [`redact`](Batch::redact)s, and commits everything only if `f` returns
    /// `Ok`. Any other exit (i.e. `Err` or panic) does not commit anything.
    ///
    /// Returns [`Committed`], which annotates the closure's original value with
    /// information about how many messages the batch sent and redacted, and the
    /// [`Peer`]'s [`Version`] right after the commit.
    ///
    /// The closure is synchronous and the type system prevents the handle from
    /// leaving it, so async cancellation cannot observe a half-built batch. The
    /// closure may use the same `Rumors` handle safely, though perhaps
    /// confusingly, since such nested operations commit *inside-out*,
    /// separately.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::{Peer, error::EncodeError};
    ///
    /// let rumors = Peer::<String>::seed().into_rumors();
    /// rumors.batch(|batch| {
    ///     batch.send("a".to_string())?;
    ///     batch.send("b".to_string())?;
    ///     Ok::<(), EncodeError>(())
    /// })?;
    /// // Both landed, in a single commit.
    /// assert_eq!(rumors.snapshot().len(), 2);
    /// # Ok::<(), EncodeError>(())
    /// ```
    ///
    /// The scope handle cannot be moved out of the closure...
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
    /// ...and cannot be returned from it either:
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

    /// Take a consistent point-in-time snapshot of the set of messages.
    pub fn snapshot(&self) -> Snapshot<T> {
        self.peer.snapshot()
    }

    /// Iterate through a stream of every message in this [`Rumors`], however it
    /// arrived, in arbitrary (*non-causal*) order.
    ///
    /// See [`UnorderedMessages`] for details.
    pub fn unordered_messages(&self) -> UnorderedMessages<T> {
        self.peer.unordered_messages()
    }

    /// Iterate through a stream of every message not causally contained in
    /// `since`, then everything learned afterwards, in arbitrary (*non-causal*)
    /// order.
    ///
    /// See [`UnorderedMessages`] for details.
    pub fn unordered_messages_since(&self, since: Version) -> UnorderedMessages<T> {
        self.peer.unordered_messages_since(since)
    }

    /// Iterate through a stream of every message in this [`Rumors`], however it
    /// arrived, in *causal order*.
    ///
    /// See [`CausalMessages`] for details.
    pub fn causal_messages(&self) -> CausalMessages<T> {
        self.peer.causal_messages()
    }

    /// Iterate through a stream of every message not causally contained in
    /// `since`, then everything learned afterwards, in *causal order*.
    ///
    /// See [`CausalMessages`] for details.
    pub fn causal_messages_since(&self, since: Version) -> CausalMessages<T> {
        self.peer.causal_messages_since(since)
    }

    /// Iterate through a stream of notifications *that* this [`Rumors`]
    /// changed, without observing what changed.
    ///
    /// The result is a coalescing stream that yields `()` immediately on first
    /// poll, then again every time the set of messages changes.
    ///
    /// See [`Changes`] for details.
    pub fn changes(&self) -> Changes<T> {
        self.peer.changes()
    }

    /// Return the globally unique identifier of the gossip network to which
    /// this replica belongs.
    pub fn network(&self) -> Network {
        self.peer.network()
    }

    /// Return the local settings used to configure the underlying
    /// [`Peer`].
    pub fn settings(&self) -> Settings {
        self.peer.settings()
    }

    /// Alias this set's live party for invariant assertions in tests; see
    /// [`Peer::dangerously_alias_party`] for what the caller must uphold.
    #[cfg(any(test, feature = "test-internals"))]
    #[doc(hidden)]
    pub fn dangerously_alias_party(&self) -> before::Party {
        self.peer.dangerously_alias_party()
    }
}

/// Gossip, and recover exclusive ownership of the peer.
impl<T: Send + Sync + 'static, B: Bookmark> Rumors<T, B> {
    /// Gossip over one connection until it ends.
    ///
    /// To observe details about each completed gossip session, use
    /// [`gossip_stream`](Rumors::gossip_stream) and iterate over its returned
    /// stream of [`Gossiped`] status reports.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[expect(dead_code)]
    /// # async fn example(
    /// #     rumors: &rumors::Rumors<String>,
    /// #     connection: impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Unpin,
    /// # ) -> Result<(), rumors::Error> {
    /// let (read, write) = tokio::io::split(connection);
    /// let read = tokio::io::BufReader::new(read);
    ///
    /// rumors.gossip(read, write).await
    /// # }
    /// ```
    pub async fn gossip<R, W>(&self, read: R, write: W) -> Result<(), Error<B>>
    where
        R: AsyncBufRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        let mut sessions = self.gossip_stream(read, write);
        while let Some(session) = sessions.next().await {
            session?;
        }
        Ok(())
    }

    /// Gossip over one connection until it ends, yielding a report after each
    /// gossip session.
    ///
    /// Each successful [`Gossiped`] reports the converged [`Version`], and
    /// various statistics about the session. **You must keep polling the
    /// resultant stream to drive the connection.**
    ///
    /// To force one immediate gossip session regardless of the configured
    /// policies on the [`Peer`], use [`gossip_once`](Self::gossip_once). To
    /// drive the connection without inspecting each session, use
    /// [`gossip`](Self::gossip).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[expect(dead_code)]
    /// # async fn example(
    /// #     rumors: &rumors::Rumors<String>,
    /// #     connection: impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Unpin,
    /// # ) -> Result<(), rumors::Error> {
    /// use futures::StreamExt;
    ///
    /// let (read, write) = tokio::io::split(connection);
    /// let read = tokio::io::BufReader::new(read);
    ///
    /// let mut sessions = rumors.gossip_stream(read, write);
    /// while let Some(session) = sessions.next().await {
    ///     let gossiped = session?;
    ///     // Inspect `gossiped` or continue driving the connection.
    ///     # let _ = gossiped;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "the driver does nothing until the returned stream is polled"]
    pub fn gossip_stream<'a, R, W>(
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

    /// Run a single gossip session with a remote peer.
    ///
    /// The configured [`Peer::gossip_timeout`] applies to this exchange. The
    /// initiation policy does not: this call always starts a gossip session,
    /// regardless of [`Peer::gossip_when`](crate::Peer::gossip_when) and
    /// [`Peer::gossip_until`](crate::Peer::gossip_until).
    pub async fn gossip_once<R, W>(&self, read: R, write: W) -> Result<(Gossiped, R, W), Error<B>>
    where
        R: AsyncBufRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        self.peer.gossip_once(read, write).await
    }

    /// Give up this [`Rumors`] handle and reclaim the [`Peer`]: resolves when
    /// no [`Rumors`] for this set remain, handing the `Peer` to exactly one
    /// caller of this function.
    ///
    /// Cancelling a pending [`try_into_peer`](Self::try_into_peer) abandons its
    /// claim. If every handle goes away with no
    /// [`try_into_peer`](Self::try_into_peer) pending, the `Peer` is gone for
    /// good, and streams of messages will all end.
    pub async fn try_into_peer(self) -> Option<Peer<T, B>> {
        self.try_into_peer_inner().await
    }
}
