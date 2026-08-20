mod causal;
mod changes;
mod unordered;

pub use causal::CausalMessages;
pub use changes::{Changes, TryTick};
pub use unordered::{TryNext, UnorderedMessages};

use crate::bookmark::{Bookmark, BookmarkError, NoBookmark};
use crate::link::{Acceptor, Connector, Link};
use crate::message::PayloadDepthError;
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

impl<T, B: BookmarkError> Clone for Rumors<T, B> {
    fn clone(&self) -> Self {
        Self {
            peer: Peer {
                network: self.peer.network,
                protocol: self.peer.protocol,
                window: self.peer.window,
                run_budget: self.peer.run_budget,
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
            .field("protocol", &self.peer.protocol)
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
    /// The message is serialized and admitted here, at the call: a
    /// payload whose encoding nests deeper than the peer's
    /// [`payload_depth_limit`](crate::Peer::payload_depth_limit) is the
    /// typed error, and nothing commits. To apply several changes in one
    /// commit, use [`batch`](Self::batch).
    ///
    /// `send` does not return the message's [`Version`]. Versions come back
    /// through observation: the observers and [`Snapshot`] attach every
    /// message to the version its send minted, unique across the universe's
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
    /// If `message` fails to serialize (see [`Batch::send`]).
    pub fn send(&self, message: T) -> Result<(), PayloadDepthError>
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
    /// applies. To bundle redactions and sends into one commit, use
    /// [`batch`](Self::batch).
    ///
    /// # Deletion is honored
    ///
    /// Once a redaction commits anywhere, no gossip schedule re-establishes
    /// the redacted message from replicas that still hold it. Nothing
    /// crosses the wire to represent a deletion; reconciliation infers
    /// deletions from the causal frontiers the two sides exchange. A
    /// message the counterparty's version shows it must already have seen,
    /// yet it no longer holds, was deleted there, so the holder drops its
    /// own copy instead of transmitting it. And because every send mints a
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

    /// Apply several changes in one all-or-nothing commit.
    ///
    /// Runs `f` with a [`Batch`] scope handle for queueing
    /// [`send`](Batch::send)s and [`redact`](Batch::redact)s, then
    /// commits everything queued **iff `f` returns `Ok`**: observers and
    /// concurrent gossip sessions see all of it land at once, as one
    /// commit, one tree traversal, and at most one observer wakeup. Any
    /// other exit commits nothing, earlier-queued actions included: a
    /// returned `Err` — your own, or a `?`-propagated
    /// [`PayloadDepthError`] from a depth-rejected send — and a panic's
    /// unwind alike cancel the batch whole. Returning `Err` is the
    /// deliberate abort affordance: a batch is all-or-nothing.
    ///
    /// The closure is synchronous, so batch state cannot be held across
    /// an `.await` point (a synchronous closure body cannot await), and
    /// the scope handle cannot leave the closure (the examples below pin
    /// both escape routes as compile errors). Async cancellation
    /// therefore cannot observe a half-built batch: a cancellation lands
    /// between polls, and the whole closure runs inside one poll.
    ///
    /// The closure may itself use the same `Rumors` handle — a
    /// [`send`](Self::send), or a nested `batch` — because building a
    /// batch holds no lock and the outer commit runs only after the
    /// closure returns: nested operations commit first, the outer batch
    /// after, as separate commits (inner-before-outer).
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::{PayloadDepthError, Peer};
    ///
    /// let rumors = Peer::<String>::seed().into_rumors();
    /// rumors.batch(|batch| {
    ///     batch.send("a".to_string())?;
    ///     batch.send("b".to_string())?;
    ///     Ok::<(), PayloadDepthError>(())
    /// })?;
    /// // Both landed, in one commit.
    /// assert_eq!(rumors.snapshot().len(), 2);
    /// # Ok::<(), PayloadDepthError>(())
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

    /// Force this set's tree to compute its lazy structural memos (observable
    /// hash and ceiling/floor version bounds), so a subsequent operation is
    /// timed against its own work. For benchmark and test calibration only.
    #[doc(hidden)]
    pub fn warm_caches(&self) {
        self.peer.warm_caches();
    }

    /// Alias this set's live party for invariant assertions in tests; see
    /// [`Peer::dangerously_alias_party`] for what the caller must uphold.
    #[cfg(any(test, feature = "test-internals"))]
    #[doc(hidden)]
    pub fn dangerously_alias_party(&self) -> Option<before::Party> {
        self.peer.dangerously_alias_party()
    }
}

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
    /// always [`Led::Local`](crate::Led::Local): calling `gossip` is this
    /// side's initiation, and a remote initiation already in flight merges
    /// into the same session, exactly as racing
    /// [`gossip_when`](Self::gossip_when) triggers do.
    ///
    /// On `Ok`, both replicas hold every message either one held when the
    /// session began **and neither had deleted**, and, under
    /// [`Protocol::V2`](crate::Protocol::V2), the peer has confirmed that
    /// it completed and committed the session too (the frozen
    /// [`Protocol::V1`](crate::Protocol::V1) oracle wire has no
    /// confirmation exchange, so a V1 session's `Ok` certifies only the
    /// local commit). The link rests exactly at the session boundary,
    /// ready to host this pair's next session.
    ///
    /// On `Err`, the replica is unchanged and the link is poisoned:
    /// discard it and reconnect. This is enforced, not advisory, since
    /// every subsequent session on the link fails fast with
    /// [`Error::LinkPoisoned`] rather than misreading its mid-frame
    /// control stream. Cancellation counts as `Err` ([what a session
    /// promises](crate::link::Link#what-a-session-promises)).
    /// "Unchanged" has three qualified exceptions:
    ///
    /// - On [`Error::Epilogue`], every local effect of the session is
    ///   already committed; only the confirmation of the *peer's*
    ///   completion was lost ([`Error::Epilogue`] explains why that gap
    ///   cannot be closed).
    /// - A failure while donating a bootstrap fork costs that fork's
    ///   identity space (deliberately: the newcomer may hold it),
    ///   narrowing this replica's identity without touching its content.
    /// - An [`Error::Bookmark`] raised after absorbing a retiring peer
    ///   leaves the session fully committed (reconciled content *and* the
    ///   absorbed identity) with only its durable record unwritten (the
    ///   error's docs carry the crash-safety consequence).
    ///
    /// Independently of these, an `Err` never rolls back identity the
    /// session reclaimed from the bookmark: it stays live in memory, and
    /// the next successful persist records it
    /// ([`Error::Bookmark`] carries the
    /// mechanism).
    ///
    /// Gossip sessions may run concurrently through any handles (the
    /// same clone or different ones), each over its own link; each commits
    /// atomically when it completes. Sessions on one link are serialized,
    /// which the `&mut` borrow enforces; a bookmarked peer's sessions also
    /// queue at the bookmark lock before any wire traffic
    /// ([`Bookmark`]).
    pub async fn gossip<CR, CW, C, A>(
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
        self.peer.gossip(link).await
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
    /// The returned stream *must be polled* for gossip to continue. It
    /// yields one [`Gossiped`] per completed gossip session. It terminates
    /// in one of three ways:
    ///
    /// - the connection fails: one final `Err`, with the replica unchanged,
    ///   subject to the same qualified exceptions as [`gossip`](Self::gossip)
    ///   (the post-commit [`Error::Epilogue`] and retiree-absorption
    ///   [`Error::Bookmark`] cases, and a donated fork lost in flight), and
    ///   the link is poisoned on every error path, so any later session on
    ///   it fails fast with [`Error::LinkPoisoned`]: discard the link;
    /// - `when` ends, cleanly, after finishing any session in flight;
    /// - the remote hangs up at a session boundary, cleanly.
    ///
    /// Either clean termination leaves the link at a session boundary, but
    /// they differ in what the link is still good for. When `when` ends,
    /// the connection is intact: hand the link to another driver or
    /// session. When the remote hangs up, the peer is gone: a new driver on
    /// the same link only observes the goodbye again, and a one-shot
    /// session fails against the closed transport. Each driven session
    /// promises exactly what a one-shot [`gossip`](Self::gossip) does
    /// ([what a session promises](crate::link::Link#what-a-session-promises)).
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
    /// # Cancellation
    ///
    /// Futures derived from polling the result-stream are cancel-safe: all
    /// driver state lives in the stream itself. Dropping the result stream,
    /// however, is *not* cancellation-safe: a session in flight is
    /// cancelled with it, poisoning the link exactly as dropping a
    /// [`gossip`](Self::gossip) future would. To stop cleanly, end the
    /// `when` stream and poll the driver to completion; what the link
    /// remains good for after each clean termination is stated above.
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
    /// let (mut near, mut far) = rumors::link::memory();
    /// # let serve = alice.clone();
    /// # let server = tokio::spawn(async move {
    /// #     serve.gossip(&mut far).await.unwrap();
    /// # });
    /// let bob = Peer::<String>::bootstrap().join(&mut near)
    ///     .await?
    ///     .expect("alice is established")
    ///     .into_rumors();
    /// # server.await.unwrap();
    ///
    /// // A long-lived link between them, one driver per end.
    /// let (mut alice_side, mut bob_side) = rumors::link::memory();
    ///
    /// alice.send("psst".to_string()).expect("flat payload");
    ///
    /// let mut alice_drive = alice.gossip_when(alice.changes(), &mut alice_side);
    /// let mut bob_drive = bob.gossip_when(bob.changes(), &mut bob_side);
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
    #[must_use = "the driver does nothing until the returned stream is polled"]
    pub fn gossip_when<'a, CR, CW, C, A, S>(
        &'a self,
        when: S,
        link: &'a mut Link<CR, CW, C, A>,
    ) -> impl Stream<Item = Result<Gossiped, Error<B>>> + Unpin + 'a
    where
        T: Send + Sync + 'static,
        CR: AsyncRead + Unpin + Send,
        CW: AsyncWrite + Unpin + Send,
        C: Connector,
        A: Acceptor,
        S: Stream<Item = ()> + 'a,
    {
        self.peer.gossip_when(when, link)
    }
}
