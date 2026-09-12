//! The local rumor set: [`Peer`] and its synchronized state, plus the local
//! API for sending, redacting, and observing messages. The wire-session
//! drivers (bootstrap, gossip, retire) live in [`gossip`].

use std::sync::{Arc, PoisonError, RwLock};

use before::Party;
use rand::{RngCore, rngs::OsRng};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{Mutex, watch};

use crate::bookmark::{BookmarkError, Bookmarked, NoBookmark};
use crate::link::{Acceptor, Connector, Link};
pub use crate::message::{DEFAULT_PAYLOAD_DEPTH_LIMIT, PayloadDepthLimit};
use crate::message::{EncodeError, PayloadCodec};
use crate::observe::{Attachment, Observer};
use crate::tree::Tree;
pub use crate::tree::mirror::streaming::remote::DEFAULT_TARGET_MESSAGE_SIZE;
use crate::tree::mirror::streaming::remote::RunBudget;
pub use crate::tree::mirror::streaming::window::DEFAULT_SYNC_MEMORY_BUDGET;
use crate::tree::mirror::streaming::window::WindowConfig;
use crate::{
    Batch, Bookmark, CausalMessages, Network, Rumors, Snapshot, UnorderedMessages, Version,
};

use serde::Serialize;
use serde::de::DeserializeOwned;
mod bootstrap;
mod gossip;
mod policy;

pub use bootstrap::{Bootstrap, Joined};
pub use gossip::{Gossip, Gossiped, Led, Retire, Unbookmarked};

/// The start and end of a [`Rumors`]'s lifecycle.
///
/// Create a network with [`seed`](Self::seed), or join an established peer's
/// network with [`bootstrap`](Self::bootstrap). Independently seeded networks
/// cannot gossip with one another.
///
/// Convert the peer into [`Rumors`] to use and share the replica. A `Peer`
/// cannot be cloned and exists only while no `Rumors` handles to that replica
/// remain, so [`retire`](Self::retire) cannot interrupt another handle's use.
///
/// # Example
///
/// The lifecycle of a [`Peer`] usually looks something like this:
///
/// ```
/// use rumors::{Joined, Peer, Retire};
///
/// # tokio::runtime::Builder::new_current_thread()
/// #     .build()
/// #     .unwrap()
/// #     .block_on(async {
/// // The counterparty this example talks to: the universe's seed, serving
/// // the bootstrap and later accepting the retirement, over in-memory links.
/// let counterparty = Peer::<String>::seed().into_rumors();
/// let (mut near, mut far) = rumors::link::memory();
/// # let serve = counterparty.clone();
/// # tokio::spawn(async move {
/// #     serve.gossip_once(&mut far).await.unwrap();
/// # });
/// // Join through an established peer. The new peer receives its full set.
/// let Joined::Joined { peer } = Peer::<String>::bootstrap().join(&mut near).await else {
///     panic!("the established counterparty must serve the bootstrap");
/// };
///
/// // A `Peer` is `!Clone`; trade it for `Rumors` handles to send and gossip.
/// let rumors = peer.into_rumors();
/// let other = rumors.clone();
/// // ... send, redact, and gossip concurrently through the clones ...
///
/// // Once every other handle is gone, the unique `Peer` can be reclaimed.
/// drop(other);
/// let Some(peer) = rumors.try_into_peer().await else {
///     unreachable!("all other handles were dropped already");
/// };
///
/// // Leave the network through any gossiping peer. It need not be the
/// // one we joined through.
/// let (mut near, mut far) = rumors::link::memory();
/// # tokio::spawn(async move {
/// #     counterparty.gossip_once(&mut far).await.unwrap();
/// # });
/// let retry = match peer.retire(&mut near).await {
///     // Retirement completed; nothing more to do.
///     Retire::Retired => None,
///     // Both sides were retiring; retry against a different peer.
///     Retire::Declined { peer } => Some(peer),
///     // Retirement did not proceed. Retry using a fresh link.
///     Retire::Recovered { peer, error: _ } => Some(peer),
///     // The outcome is uncertain, and this peer cannot be used again.
///     Retire::Uncertain { error } => return Err(error),
/// };
/// assert!(retry.is_none(), "the example's retirement succeeds");
/// # Ok(())
/// # })?;
/// # Ok::<(), rumors::Error>(())
/// ```
///
/// # Bootstrapping without consensus
///
/// If your application admits a distinguished "first peer" (for example, via
/// leader election or another consensus mechanism), have that peer call
/// [`Peer::seed`].
///
/// Absent any true consensus mechanism, another reasonable approach to
/// bootstrapping a [`Network`] is for *every* [`Peer`] to initially call
/// [`Peer::seed`] and attempt to [`gossip`](crate::Rumors::gossip) with all
/// others. Initially, peers report
/// [`Mismatch::Network`](crate::error::Mismatch::Network);
/// whenever a peer observes one, it can use a deterministic metric to decide
/// whether it or its peer should dominate.
///
/// A reasonable such metric ships inside the error itself: compare its
/// `local_min_events` against its `remote_min_events`. The greater minimal
/// event count wins, with ties broken by comparing the two [`Network`] ids
/// (their ordering is total).
/// Each side declared its count in the session's handshake, so both apply
/// the rule from the one error alone, with nothing further to fetch or
/// race, and agree without coordination: the winner stays in its network,
/// and the loser drops its replica and attempts to
/// re-[`bootstrap`](Peer::bootstrap) into the winning [`Network`].
///
/// If peers are reasonably well-connected as the network gets started, this
/// quickly reaches a stable steady state, disrupted only if a group of new
/// peers joins exclusively with one another and spends a long time
/// partitioned before reuniting with the rest of the network.
pub struct Peer<T, B: BookmarkError = NoBookmark> {
    /// The network this replica belongs to, established at seed or join.
    pub(crate) network: Network,
    /// The reconciliation window choice selected by
    /// [`sync_memory_budget`](Self::sync_memory_budget), resolved per
    /// session against the greeting's exchanged set sizes.
    pub(crate) window: WindowConfig,
    /// The supply-run byte budget selected by
    /// [`target_message_size`](Self::target_message_size).
    pub(crate) run_budget: RunBudget,
    /// Initiation and deadline factories inherited by every gossip handle.
    pub(crate) gossip_policy: policy::Policy<T>,
    /// Shared replica state and change notifications for handles and drivers.
    pub(crate) inner: watch::Sender<Inner<T>>,
    /// The identity bookmark: persistence handle and its in-memory record,
    /// behind an async mutex and shared with every [`Rumors`] clone.
    ///
    /// Separate from `inner` because persisting is `async` and the record is
    /// `!Clone`; see [`Bookmarked`].
    pub(crate) bookmark: Arc<Mutex<Bookmarked<B>>>,
    /// The payload codec built at construction: the typed ingress every
    /// gossip session's supplied leaf records decode through.
    ///
    /// Carries the [`payload_depth_limit`](Self::payload_depth_limit)
    /// beside the codec's fn pointers (see [`PayloadCodec`]).
    pub(crate) codec: PayloadCodec,
    /// The wire-observation handler selected by [`observe`](Self::observe).
    pub(crate) observe: Attachment,
}

/// The replica's identity and content, shared through a watch channel.
///
/// Retirement owns the consumed `Peer`, excluding other writable handles.
pub(crate) struct Inner<T> {
    /// Identity used to stamp local events.
    pub(crate) party: Party,
    /// The published content and causal ceiling.
    pub(crate) tree: Tree<T>,
    /// Serializes local tree preparation and the fallback gossip join.
    ///
    /// Party changes and optimistic swaps take it shared; local preparation and
    /// the fallback join take it exclusively. Acquire it before any watch guard.
    /// Snapshot readers do not take it.
    /// The gate contains no data to repair after a panic, so poison is ignored.
    commit_gate: Arc<RwLock<()>>,
}

/// Try the session result, then one rebase, before excluding competing writers.
///
/// More optimistic attempts can avoid excluding writers, but each failed rebase
/// wastes a join. This bounds speculative work without tuning a public contract.
const OPTIMISTIC_ATTEMPTS: usize = 2;

impl<T> Inner<T> {
    /// Construct a replica whose commits share one writer gate.
    pub(crate) fn new(party: Party, tree: Tree<T>) -> Self {
        tree.warm_memos();
        Self {
            party,
            tree,
            commit_gate: Arc::new(RwLock::new(())),
        }
    }

    /// Prepare and publish a local tree change without blocking snapshot reads.
    ///
    /// The writer gate keeps the tree and party together until publication.
    /// Borrow the party while applying the update, then release the watch guard
    /// before warming the new tree. Readers continue to see the previous tree.
    /// Retain incoming payloads outside `update`: a skipped insert's destructor
    /// must run after both guards are released, including on unwind.
    pub(crate) fn commit(
        sender: &watch::Sender<Self>,
        update: impl FnOnce(&Party, &mut Tree<T>) -> bool,
    ) {
        // Declaring the candidate before the guards makes its destructor run
        // after them on unwind too. Until the swap, the sender retains our old
        // payloads; afterwards the candidate retains the displaced tree.
        let mut candidate;
        {
            let gate = sender.borrow().commit_gate.clone();
            let _hold = gate.write().unwrap_or_else(PoisonError::into_inner);
            let changed = {
                let inner = sender.borrow();
                candidate = inner.tree.clone();
                update(&inner.party, &mut candidate)
            };
            candidate.warm_memos();
            sender.send_if_modified(|inner| {
                std::mem::swap(&mut inner.tree, &mut candidate);
                changed
            });
        }
        drop(candidate);
    }

    /// Change party custody at the published frontier without notifying readers.
    ///
    /// A local commit must publish before a fork can inherit its party and
    /// history. The shared writer gate orders these changes with preparation.
    /// Callers needing a bookmark guard must acquire it before this method.
    fn update_party(sender: &watch::Sender<Self>, update: impl FnOnce(&mut Party, &Tree<T>)) {
        let gate = sender.borrow().commit_gate.clone();
        let _hold = gate.read().unwrap_or_else(PoisonError::into_inner);
        sender.send_if_modified(|inner| {
            update(&mut inner.party, &inner.tree);
            false
        });
    }

    /// Clone the published tree without carrying its read guard into the caller.
    fn snapshot(sender: &watch::Sender<Self>) -> Tree<T> {
        // An assignment may drop the caller's old tree. End the borrow here so
        // that a destructor triggered by that assignment runs outside this lock.
        sender.borrow().tree.clone()
    }

    /// Publish reconciled content without walking the tree under the watch lock.
    ///
    /// The session result already includes `prior`. If that snapshot is still
    /// current, swap in the result directly. Otherwise join it with a fresh
    /// snapshot outside the lock and retry. After repeated conflicts, exclude
    /// other writers for one join and swap; readers remain free to take snapshots.
    ///
    /// Keep every input and displaced root alive until both locks are released,
    /// including on unwind: this guards against a pathological case where their
    /// payload destructors may access this replica, which would otherwise
    /// deadlock. `update` runs once, under the watch lock, to publish an
    /// identity change alongside the tree. Only a tree change notifies observers.
    fn publish<E>(
        sender: &watch::Sender<Self>,
        prior: &Tree<T>,
        reconciled: &Tree<T>,
        mut update: impl FnMut(&mut Self) -> Result<(), E>,
    ) -> Result<(), E>
    where
        T: Send + Sync,
    {
        let gate = sender.borrow().commit_gate.clone();
        // Reconciliation already joined against `prior`. If it is still current,
        // publishing needs only a swap, with no second join of the same inputs.
        let mut expected = prior.clone();
        let mut candidate = reconciled.clone();
        for attempt in 0..OPTIMISTIC_ATTEMPTS {
            if attempt > 0 {
                // A writer moved the root. Replace the failed candidate with a
                // join against the latest snapshot, releasing the old attempt's
                // roots here while neither lock is held.
                expected = Self::snapshot(sender);
                candidate = reconciled.clone();
                candidate.join(expected.clone());
            }
            candidate.warm_memos();
            #[cfg(test)]
            tests::before_swap();
            let result = {
                // Join work is already done. Hold the shared gate only while
                // checking and publishing, so local writers can keep progressing.
                let _hold = gate.read().unwrap_or_else(PoisonError::into_inner);
                Self::try_swap(sender, &expected, &mut candidate, &mut update)
            };
            if let Some(result) = result {
                return result;
            }
        }

        // Further retries could keep losing to busy writers. Excluding them
        // makes one final join sufficient, while leaving snapshot reads free.
        Self::publish_exclusive(sender, reconciled, &gate, &mut update)
    }

    /// Join and publish while excluding other writers, without blocking snapshots.
    fn publish_exclusive<E>(
        sender: &watch::Sender<Self>,
        reconciled: &Tree<T>,
        gate: &RwLock<()>,
        update: &mut impl FnMut(&mut Self) -> Result<(), E>,
    ) -> Result<(), E>
    where
        T: Send + Sync,
    {
        // Declare retained roots before the guard so unwinding also releases
        // the gate before any payload can be destroyed.
        let snapshot;
        let mut candidate = reconciled.clone();
        {
            let _hold = gate.write().unwrap_or_else(PoisonError::into_inner);
            // Acquire the gate before reading: another writer may have committed
            // since the last failed attempt. Now this snapshot stays current
            // through the join, without holding the watch lock during the walk.
            snapshot = Self::snapshot(sender);
            candidate.join(snapshot.clone());
            candidate.warm_memos();
            Self::try_swap(sender, &snapshot, &mut candidate, update)
                .expect("exclusive commit cannot lose its snapshot")
        }
    }

    /// Swap only if the published root still matches `expected`.
    ///
    /// The caller holds the writer gate and has warmed `candidate`.
    /// A conflict leaves `candidate` and `update` untouched; success moves the
    /// displaced tree into `candidate` for destruction after the gate is released.
    fn try_swap<E>(
        sender: &watch::Sender<Self>,
        expected: &Tree<T>,
        candidate: &mut Tree<T>,
        update: &mut impl FnMut(&mut Self) -> Result<(), E>,
    ) -> Option<Result<(), E>> {
        // Equality includes the causal ceiling: even an empty tree can convey
        // new redactions. It may compute hashes, so keep it outside the watch lock.
        let changed = candidate != expected;
        let mut result = None;
        sender.send_if_modified(|inner| {
            if !inner.tree.root_is(expected) {
                // The candidate may lack a concurrent change. Leave both it and
                // the identity callback untouched until we have rebased it.
                return false;
            }
            match update(inner) {
                Ok(()) => {
                    // Return the displaced root through `candidate`; dropping it
                    // here could run a payload destructor under both locks.
                    std::mem::swap(&mut inner.tree, candidate);
                    result = Some(Ok(()));
                    changed
                }
                Err(error) => {
                    result = Some(Err(error));
                    false
                }
            }
        });
        result
    }
}

#[cfg(test)]
mod tests;

/// A summary view (network, latest version, live-message count), independent
/// of `T: Debug`: the messages themselves are not printed.
impl<T, B: BookmarkError> std::fmt::Debug for Peer<T, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner.borrow();
        f.debug_struct("Peer")
            .field("network", &self.network)
            .field("latest", inner.tree.latest())
            .field("len", &inner.tree.len())
            .finish_non_exhaustive()
    }
}

impl<T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static> Peer<T, NoBookmark> {
    /// Create a new gossip network containing only this peer.
    ///
    /// Call once per network; other participants join through
    /// [`bootstrap`](Self::bootstrap). Independently seeded peers cannot gossip
    /// with each other. The payload type must follow the
    /// [payload contract](crate#choosing-a-payload-type).
    pub fn seed() -> Self {
        Self::seed_rng(&mut OsRng)
    }

    /// Like [`seed`](Self::seed), but draws the universe's [`Network`]
    /// identifier from a caller-supplied RNG instead of [`OsRng`].
    #[doc(hidden)]
    pub fn seed_rng<R: RngCore + ?Sized>(rng: &mut R) -> Self {
        Self {
            network: Network::from_rng(rng),
            window: WindowConfig::default(),
            run_budget: RunBudget::default(),
            gossip_policy: policy::Policy::default(),
            inner: watch::Sender::new(Inner::new(Party::seed(), Tree::new())),
            bookmark: Arc::new(Mutex::new(Bookmarked::new(NoBookmark))),
            codec: PayloadCodec::new::<T>(PayloadDepthLimit::default()),
            observe: Attachment::default(),
        }
    }
}

impl<T> Peer<T> {
    /// Configure a join to an existing gossip network.
    ///
    /// Call [`Bootstrap::join`] with a link to an established member. The
    /// returned peer retains the builder's settings. Failed sessions return
    /// the builder for retry. See [`Joined`] for the possible outcomes.
    pub fn bootstrap() -> Bootstrap<T> {
        Bootstrap::new()
    }

    /// Attach restart bookkeeping that limits version growth after crashes.
    ///
    /// Reuse this bookmark when restarting the peer. It records internal
    /// protocol state, not messages; recover content by joining and gossiping.
    /// [`Bookmark`] explains the storage and ownership requirements.
    ///
    /// Attachment reads and updates storage before returning. A pristine seed
    /// with no prior activity defers this until its first gossip session.
    /// Select [`Bootstrap::bookmark`] to perform attachment as part of joining.
    ///
    /// # Errors
    ///
    /// A storage or decoding failure returns the peer unchanged and without a
    /// bookmark in [`Unbookmarked`]. Repair or replace the storage, then retry.
    pub async fn bookmark<B: Bookmark>(
        self,
        bookmark: B,
    ) -> Result<Peer<T, B>, Unbookmarked<T, B>> {
        self.bookmark_inner(bookmark).await
    }
}

/// Retire an exclusively held replica.
impl<T, B: Bookmark> Peer<T, B> {
    /// Leave the gossip network after synchronizing with a remote member.
    ///
    /// Retiring helps keep message versions compact as peers come and go.
    /// The remote member runs ordinary [`gossip`](Rumors::gossip); it needs
    /// no special call to accept the retirement.
    ///
    /// [`Retire`] reports whether this peer left, can retry, or was consumed
    /// with an uncertain outcome, including after [`session_deadline`](Self::session_deadline)
    /// expires. See the [lifecycle example](Peer) and the
    /// [session contract](crate::link::Link#what-a-session-promises).
    pub async fn retire<CR, CW, C, A>(self, link: &mut Link<CR, CW, C, A>) -> Retire<T, B>
    where
        T: Send + Sync + 'static,
        CR: AsyncRead + Unpin + Send,
        CW: AsyncWrite + Unpin + Send,
        C: Connector,
        A: Acceptor,
    {
        self.retire_inner(link).await
    }
}

/// Inspect the network and configure replication behavior.
impl<T, B: BookmarkError> Peer<T, B> {
    /// The globally unique identifier for this network of gossiping [`Peer`]s.
    pub fn network(&self) -> Network {
        self.network
    }

    /// Select how each connection initiates gossip. By default it follows changes.
    ///
    /// The factory receives a fresh [`Changes`](crate::Changes) subscription for
    /// each [`Rumors::gossip`](crate::Rumors::gossip) driver. It can adapt that
    /// stream, combine it with periodic heartbeat requests, or ignore it and
    /// provide its own policy. An always-pending stream only serves remote
    /// initiations. An endless, always-ready stream can busy-loop.
    ///
    /// Each item converts to [`Gossip`]; `()` means initiate if
    /// this connection has new local state. The driver suppresses echoes of its
    /// own completed sessions. Remote initiations are always served, regardless
    /// of the local policy. Ending the policy stream ends the driver cleanly
    /// after any active session.
    ///
    /// The factory is shared by all handles and may be called concurrently for
    /// different links. Each returned stream belongs to one driver.
    ///
    /// # Graceful shutdown
    ///
    /// Capture a shared shutdown signal in the factory to end the policy streams
    /// for every clone's gossip drivers. Each driver observes the signal between
    /// sessions; keep polling it until it ends so an active exchange can finish.
    /// Use [`session_deadline`](Self::session_deadline) to bound a stalled exchange.
    ///
    /// ```
    /// # pollster::block_on(async {
    /// use futures::{channel::oneshot, FutureExt, StreamExt};
    /// use rumors::Peer;
    ///
    /// let (stop, stopped) = oneshot::channel::<()>();
    /// let stopped = stopped.map(|_| ()).shared();
    /// let rumors = Peer::<String>::seed()
    ///     .gossip_when(move |changes| changes.take_until(stopped.clone()))
    ///     .into_rumors();
    ///
    /// let (mut link, _remote) = rumors::link::memory();
    /// let handle = rumors.clone();
    /// let drive = async move {
    ///     let mut sessions = handle.gossip(&mut link);
    ///     while let Some(session) = sessions.next().await {
    ///         session?;
    ///     }
    ///     Ok::<_, rumors::Error>(())
    /// }; // Finishing this future drops its `handle`.
    ///
    /// // Request shutdown, then keep driving gossip while reclaiming the peer.
    /// stop.send(()).unwrap();
    /// let (finished, peer) = futures::join!(drive, rumors.try_into_peer());
    /// finished.unwrap();
    /// let peer = peer.expect("this is the only caller reclaiming the peer");
    /// // The peer can now be reconfigured or retired.
    /// # });
    /// ```
    ///
    /// [`Rumors::try_into_peer`] waits for every other handle to be dropped,
    /// including handles held by tasks doing work other than gossip.
    ///
    /// Shutdown remains in effect for new drivers using this policy; replace the
    /// policy before restarting gossip. Shutdown does not force a final
    /// synchronization of pending local changes, retire the peer, or affect
    /// explicit [`Rumors::gossip_once`] calls.
    pub fn gossip_when<F, S>(mut self, when: F) -> Self
    where
        F: Fn(crate::Changes<T>) -> S + Send + Sync + 'static,
        S: futures::Stream + Send + 'static,
        S::Item: Into<crate::Gossip>,
    {
        self.gossip_policy.set_when(when);
        self
    }

    /// Supply a fresh deadline for each wire session. Disabled by default.
    ///
    /// The deadline covers gossip, bootstrap, and retirement through final
    /// confirmation. It starts when an explicit call or the local policy
    /// initiates a session, or when the first remote bytes arrive. Idle waits
    /// are untimed, and traffic does not reset the deadline.
    ///
    /// Expiry reports [`Error::DeadlineExceeded`](crate::Error::DeadlineExceeded)
    /// and poisons the link. A completed local commit is not undone.
    /// [`Retire`] preserves a usable peer when recovery is safe; otherwise it
    /// reports an uncertain retirement. A gossip driver ends after the error.
    ///
    /// Because Rumors is runtime-independent, the application owns the clock
    /// used for the deadline: for a one-second deadline with Tokio, pass `||
    /// tokio::time::sleep(std::time::Duration::from_secs(1))`. Configure
    /// [`Bootstrap::session_deadline`] before joining; the returned peer
    /// inherits it. Bookmark attachment after the join's wire session is a
    /// separate local storage operation and is not covered by this deadline.
    pub fn session_deadline<D, F>(mut self, deadline: D) -> Self
    where
        D: Fn() -> F + Send + Sync + 'static,
        F: Future<Output = ()> + Send + 'static,
    {
        self.gossip_policy.set_deadline(deadline);
        self
    }

    /// Set the per-session memory budget for reconciliation pipelining.
    ///
    /// Rumors compares several subtrees while waiting for earlier replies.
    /// A larger budget allows more comparisons in flight, which can reduce
    /// waiting on the network. A smaller budget limits buffering but may add
    /// waits. Both peers may choose different budgets.
    ///
    /// The default is [`DEFAULT_SYNC_MEMORY_BUDGET`] (512 MiB). The budget
    /// applies separately to each synchronization; it is not reserved up
    /// front. All [`Rumors`] clones inherit this setting, and it is preserved
    /// when returning to a `Peer`, attaching a bookmark, or retiring.
    ///
    /// # Memory accounting
    ///
    /// This is a sizing target, not a hard memory limit. Buffer sizes are
    /// estimated from the two sets; unusually clustered hashes can require
    /// more memory than estimated. Even a zero budget retains the minimum
    /// buffers needed for progress over a conforming [`Link`]; their memory
    /// cost may exceed the target.
    ///
    /// Allow additional memory for the replica, observer backlogs such as
    /// [`CausalMessages`], and wire and transport buffers. Wire buffering is
    /// controlled separately by [`target_message_size`](Self::target_message_size).
    /// Limit concurrent sessions to control their combined memory use.
    ///
    /// See [choosing a synchronization budget](crate::sizing) for tuning
    /// guidance and example trade-offs.
    #[must_use]
    pub fn sync_memory_budget(mut self, budget_bytes: usize) -> Self {
        self.window = WindowConfig::Budget(budget_bytes);
        self
    }

    /// Pin every future session's pipeline window at the one-slot floor.
    ///
    /// Test-only: capacity one is the configuration the deadlock-freedom
    /// argument certifies, so test suites opt in explicitly to keep the
    /// capacity-one orderings exercised; the default derives capacities
    /// from [`sync_memory_budget`](Self::sync_memory_budget)'s default
    /// regardless of how the crate is built. Follows the peer exactly as
    /// `sync_memory_budget` does.
    #[cfg(any(test, feature = "test-internals"))]
    #[doc(hidden)]
    #[must_use]
    pub fn sync_window_floor(mut self) -> Self {
        self.window = WindowConfig::FLOOR;
        self
    }

    /// Attach a wire-observation handler to this peer's future sessions.
    ///
    /// For every session the peer enters — gossip, bootstrap serving,
    /// and retirement alike — the handler is asked for a per-session
    /// observer, which sees each directed stream's protocol messages
    /// as raw CBOR items. The full contract (the three handler levels,
    /// the ordering and back-pressure rules, what exactly is observed)
    /// is the [`observe`](crate::observe) module's.
    ///
    /// Observation never changes the wire: an observed session's bytes
    /// are identical to an unobserved one's. The choice follows the
    /// peer through [`into_rumors`](Self::into_rumors), cloning and
    /// reunion, bookmarking, and retirement; every [`Rumors`] clone
    /// shares the one handler. To observe a joining peer's own
    /// bootstrap session, attach on the builder instead
    /// ([`Bootstrap::observe`]).
    #[must_use]
    pub fn observe(mut self, observer: Arc<dyn Observer>) -> Self {
        self.observe.attach(observer);
        self
    }

    /// Bound the encoded size of the batched messages this peer sends.
    ///
    /// When the default protocol supplies a subtree the counterparty lacks,
    /// its leaves ship as *runs*: one wire message carrying a delimited
    /// sequence of leaf records. Batching is chunked by bytes: a run
    /// flushes once appending the next leaf would push the message's full
    /// encoded size, framing included, past `bytes`. Every run carries
    /// at least one leaf, so a message whose single leaf alone outgrows the
    /// target ships anyway and exceeds it. Runs never span reconciliation
    /// units: batching stops at each supplied subtree's last leaf.
    ///
    /// # Memory
    ///
    /// The target is the unit of wire-message buffering on both sides:
    /// the encoder accumulates at most one run per stream before writing
    /// it, and the receiver buffers one run's encoded bytes per message,
    /// handing each decoded leaf to the storage backend as it is read
    /// (the constructed leaves it holds in flight are charged against
    /// [`sync_memory_budget`](Self::sync_memory_budget), not this
    /// setting). Each session therefore runs at
    /// the **minimum** of the two ends' targets: the greeting carries
    /// each side's setting, and each side's *encoder* batches within
    /// that minimum. Your setting thus bounds the frames you build and,
    /// through the minimum, the frames a conforming peer sends you:
    /// the more memory-constrained peer sets the pace. Peers with
    /// different settings interoperate.
    ///
    /// The default, [`DEFAULT_TARGET_MESSAGE_SIZE`], is the byte size of
    /// the wire's maximally disputed reply (the decode side's documented
    /// per-reply memory unit), so default batching never raises the wire's
    /// established memory ceiling. Any value is safe: zero degrades to one
    /// leaf per message, and values above the wire's run byte cap
    /// (`u32::MAX` less the frame envelope) saturate to it, so a run built
    /// within the target always fits the cap.
    ///
    /// The choice follows the peer through
    /// [`into_rumors`](Self::into_rumors), cloning and reunion,
    /// bookmarking, and retirement.
    #[must_use]
    pub fn target_message_size(mut self, bytes: usize) -> Self {
        self.run_budget = RunBudget::from_bytes(bytes);
        self
    }

    /// Bound the nesting depth of the message payloads this peer sends
    /// and accepts.
    ///
    /// A payload value is accepted only if decoding its CBOR encoding as
    /// the peer's payload type recurses at most `limit` steps. What
    /// consumes one step is the decode engine's own accounting for that
    /// type — arrays, maps, and tags each do, and so can type-driven
    /// wrappers such as an enum's variant scope — so the bound is
    /// engine-defined, not a structural count of the bytes. The default,
    /// [`DEFAULT_PAYLOAD_DEPTH_LIMIT`], is 256 steps: exactly the bound
    /// the decoder applies by default, so a fleet at the default sees no
    /// acceptance change on existing content.
    ///
    /// Three points enforce the one bound:
    ///
    /// - **Send** ([`Rumors::send`](crate::Rumors::send),
    ///   [`Batch::send`](crate::Batch::send)): admission runs the exact
    ///   decode every receiver's wire ingress runs — same payload type,
    ///   same limit, same engine — so an over-deep value is rejected at
    ///   its author, at the moment of choice, with a typed
    ///   [`EncodeError`].
    /// - **Handshake**: the greeting carries each side's configured
    ///   limit, and a session proceeds only if the two are exactly equal;
    ///   a mismatch in either direction aborts both sides with
    ///   [`Mismatch::PayloadDepth`](crate::error::Mismatch::PayloadDepth)
    ///   before anything else — the converged-session short-circuit
    ///   included — so a mixed configuration is caught at every pairing.
    /// - **Wire ingress**: every payload decode runs under this same
    ///   limit, so over-deep *content* supplied by a nonconforming
    ///   implementation fails its session with a typed decode error.
    ///   The bound governs the decode's recursion, not the bytes' shape:
    ///   deep byte patterns the engine consumes without recursing (a tag
    ///   chain in a scalar position, say) decode fine and are harmless.
    ///
    /// Together those establish the invariant this setting exists for:
    /// between conforming peers, no session can fail on payload depth at
    /// all — true by construction within a decode-engine version, because
    /// admission and ingress are one computation, not two accountings
    /// held in agreement. Over-deep values are rejected at their author,
    /// and mismatched fleets are rejected at the handshake. (A fleet
    /// mixing builds whose CBOR engine versions account recursion
    /// differently could still diverge; upgrading the engine is a
    /// fleet-coordination event in the same register as changing this
    /// limit.) The limit is a property of the *shared set* — every
    /// replica must be able to hold and forward all content — which is
    /// why the handshake demands equality rather than negotiating: a
    /// peer whose session bound dropped below its own configured limit
    /// could already hold messages deeper than the negotiated bound,
    /// which it would then not be allowed to gossip. Changing the limit
    /// is therefore a fleet-coordinated configuration event, like
    /// changing the selected [`Protocol`](crate::Protocol), never a
    /// per-peer tuning parameter.
    ///
    /// The choice follows the peer through
    /// [`into_rumors`](Self::into_rumors), cloning and reunion,
    /// bookmarking, and retirement.
    #[must_use]
    pub fn payload_depth_limit(mut self, limit: PayloadDepthLimit) -> Self {
        self.codec = self.codec.with_limit(limit);
        self
    }

    /// Convert the [`Peer`] into a [`Rumors`] so it can [`send`](Rumors::send),
    /// [`redact`](Rumors::redact), and [`gossip`](Rumors::gossip).
    ///
    /// Unlike [`Peer`], [`Rumors`] is [`Clone`], so that gossip may proceed
    /// concurrently. Once a single [`Rumors`] handle remains,
    /// [`try_into_peer`](Rumors::try_into_peer) converts it back into a
    /// [`Peer`].
    pub fn into_rumors(self) -> Rumors<T, B> {
        Rumors::new(self)
    }

    pub(crate) fn send(&self, message: T) -> Result<(), EncodeError>
    where
        T: Send + Sync + 'static,
    {
        let mut batch = Batch::new(&self.inner, self.codec);
        batch.send(message)?;
        batch.commit();
        Ok(())
    }

    pub(crate) fn redact(&self, version: &Version)
    where
        T: Send + Sync,
    {
        let mut batch = Batch::new(&self.inner, self.codec);
        batch.redact(version);
        batch.commit();
    }

    pub(crate) fn batch<R, E, F>(&self, f: F) -> Result<R, E>
    where
        T: Send + Sync,
        F: for<'s> FnOnce(&'s mut Batch<'_, T>) -> Result<R, E>,
    {
        let mut batch = Batch::new(&self.inner, self.codec);
        let result = f(&mut batch)?;
        batch.commit();
        Ok(result)
    }

    pub(crate) fn send_all<I>(&self, messages: I) -> Result<(), EncodeError>
    where
        T: Send + Sync + 'static,
        I: IntoIterator<Item = T>,
    {
        let mut batch = Batch::new(&self.inner, self.codec);
        batch.send_all(messages)?;
        batch.commit();
        Ok(())
    }

    pub(crate) fn redact_all<'v, I>(&self, versions: I)
    where
        T: Send + Sync,
        I: IntoIterator<Item = &'v Version>,
    {
        let mut batch = Batch::new(&self.inner, self.codec);
        batch.redact_all(versions);
        batch.commit();
    }

    pub(crate) fn snapshot(&self) -> Snapshot<T> {
        Snapshot::new(self.network, self.inner.borrow().tree.clone())
    }

    pub(crate) fn unordered_messages(&self) -> UnorderedMessages<T>
    where
        T: Send + Sync,
    {
        self.messages_since(Version::new())
    }

    pub(crate) fn messages_since(&self, since: Version) -> UnorderedMessages<T>
    where
        T: Send + Sync,
    {
        UnorderedMessages::subscribe(&self.inner, since)
    }

    pub(crate) fn causal_messages(&self) -> CausalMessages<T>
    where
        T: Send + Sync,
    {
        self.causal_messages_since(Version::new())
    }

    pub(crate) fn causal_messages_since(&self, since: Version) -> CausalMessages<T>
    where
        T: Send + Sync,
    {
        CausalMessages::subscribe(&self.inner, since)
    }

    /// Alias the live identity for invariant assertions in tests.
    ///
    /// Compare it or use it in an accounting fold; never tick with it. An alias
    /// shares the original identity's space and cannot act as another peer.
    #[cfg(any(test, feature = "test-internals"))]
    #[doc(hidden)]
    pub fn dangerously_alias_party(&self) -> Party {
        self.inner.borrow().party.dangerously_alias()
    }
}
