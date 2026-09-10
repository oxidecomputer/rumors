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
/// #     serve.gossip(&mut far).await.unwrap();
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
/// #     counterparty.gossip(&mut far).await.unwrap();
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
    pub(crate) network: Network,
    /// The reconciliation window choice selected by
    /// [`sync_memory_budget`](Self::sync_memory_budget), resolved per
    /// session against the greeting's exchanged set sizes.
    pub(crate) window: WindowConfig,
    /// The supply-run byte budget selected by
    /// [`target_message_size`](Self::target_message_size).
    pub(crate) run_budget: RunBudget,
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

impl<T, B: Bookmark> Peer<T, B> {
    /// Leave the gossip network after synchronizing with a remote member.
    ///
    /// Retiring helps keep message versions compact as peers come and go.
    /// The remote member runs ordinary [`gossip`](Rumors::gossip); it needs
    /// no special call to accept the retirement.
    ///
    /// [`Retire`] reports whether this peer left, can retry, or was consumed
    /// with an uncertain outcome. See the [lifecycle example](Peer) and the
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

impl<T, B: BookmarkError> Peer<T, B> {
    /// The globally unique identifier for this network of gossiping [`Peer`]s.
    pub fn network(&self) -> Network {
        self.network
    }

    /// Bound the memory a synchronization may spend on pipelining.
    ///
    /// Reconciliation pipelines disputed subtrees to pay wire latency per
    /// tree level rather than per disputed subtree. Pipelining is
    /// what costs memory — kilobytes per disputed subtree in flight,
    /// priced by the storage backend's own cost function — and
    /// `budget_bytes` is its worst-case envelope, not an allocation: a
    /// session holds only what it actually disputes, typically
    /// kilobytes. The budget also pre-charges the decode fans' flat
    /// residency (one fan of backend-priced leaves plus an in-hand
    /// record per reply stream — ~0.2 MB under the in-memory backend, a
    /// term of the corpus-fixed charge `F` in the accuracy band below).
    /// This setting does not govern encoded wire messages in hand: the
    /// wire schedule bounds those, at most one run per stream per
    /// direction, so up to
    /// [`STREAM_COUNT`](crate::link::STREAM_COUNT) ×
    /// [`target_message_size`](Self::target_message_size) — ~28 MB per
    /// direction at the defaults, plus a lone over-target record's
    /// overhang.
    ///
    /// A budget can add latency, never break a session. A divergence
    /// wider than the derived capacities drains in capacity-sized
    /// waves, at the worst-case factor the trade-off table below
    /// prices; any budget, including zero, leaves every session
    /// deadlock-free, with at least one disputed subtree in flight per internal tree level.
    /// The budget is per session: concurrent gossip on separate links
    /// carries one envelope each; for a global application memory cap, you must limit
    /// the concurrency of your gossip sessions. The default,
    /// [`DEFAULT_SYNC_MEMORY_BUDGET`], is 512 MiB.
    ///
    /// Each session divides the budget into fixed per-level channel
    /// capacities from what the two replicas exchange at session start:
    /// exact set sizes and version-size bounds, so every input to the
    /// worst case is on the table before the descent begins. Under
    /// uniform version hashing, dispute populations thin geometrically
    /// with depth and scale with the *product* of the two set sizes, so
    /// the budget buys width only where disputes can exist. The setting
    /// is not wire-visible: peers with different budgets interoperate.
    ///
    /// The choice follows the peer through
    /// [`into_rumors`](Self::into_rumors), cloning and reunion,
    /// bookmarking, and retirement.
    ///
    /// # What this does not bound
    ///
    /// - **Encoded wire messages in hand**: the run buffers stated
    ///   above, priced by
    ///   [`target_message_size`](Self::target_message_size), up to
    ///   [`STREAM_COUNT`](crate::link::STREAM_COUNT) ×
    ///   `target_message_size` per direction.
    /// - **The replica itself.** The live set's resident bytes are the
    ///   application's to provision; the budget prices only what a
    ///   session holds in flight.
    /// - **Observers.** [`CausalMessages`] stages an internal backlog
    ///   with bursts up to the size of the set (its docs state the
    ///   cost); no observer's memory is charged here.
    /// - **Other sessions.** The budget is per session, so a peer
    ///   gossiping over `K` links at once can hold up to
    ///   `K × (budget + 2 × STREAM_COUNT × target_message_size)`
    ///   across them in the worst case
    ///   ([`STREAM_COUNT`](crate::link::STREAM_COUNT) counting each
    ///   direction's streams once).
    ///
    /// # Choosing a budget
    ///
    /// The intuition: the budget buys parallelism on the wire. A
    /// session keeps a window of disputed subtrees in flight at once,
    /// each holding a few kilobytes of memory while it waits for its
    /// reply. A window wide enough to keep the link's whole
    /// bandwidth-delay product occupied runs at wire speed; a narrower
    /// window makes the session stop and wait for replies in waves,
    /// spending extra round trips instead of extra memory.
    ///
    /// Sizing starts from two numbers. Your link contributes one:
    /// `BDP = bandwidth × RTT`, the bytes in flight on a full pipe;
    /// measure it. Worked figures below use the specification BDP of
    /// 12.5 MB, where 1 Gbps × 100 ms and 100 Gbps × 1 ms coincide;
    /// substitute your own measurement. Your corpus contributes the
    /// other: `m`, the mean encoded record size (the CBOR-encoded
    /// payload of a disputed message's leaf record). Two constants
    /// then convert between bytes and disputes, both derived and
    /// pinned: each in-flight dispute (one disputed subtree, the unit
    /// the table below counts as a disputed scope) charges the budget
    /// a 5431 B envelope (recomputed exactly by test), and each disputed
    /// message costs 43 B of wire overhead on top of its record
    /// (calibrated by deterministic byte counts,
    /// `tests/dispute_wire.rs`).
    ///
    /// For mental arithmetic, one closed form estimates the whole
    /// trade. A session's worst-case slowdown, relative to a session
    /// limited only by wire time, is about
    ///
    /// > `slowdown ≈ max(1, BDP × 5431 / (budget × (43 + m)))`
    ///
    /// Read it as a ratio of two message counts: how many disputed
    /// messages the wire holds, `BDP / (43 + m)`, against how many the
    /// budget keeps in flight, `budget / 5431`. Slowdown 1 is
    /// wire-time-optimal: bandwidth-bound stays bandwidth-bound.
    ///
    /// The estimate has a stated accuracy band. It overstates the
    /// window by roughly `F / budget`, where `F` is the corpus-fixed
    /// component of the real charge, so the slowdown it returns runs
    /// ~2–3× low at ~10 MB budgets, ~1.4× low at ~26 MB, and within a
    /// few percent past ~300 MB. It also prices no population ceiling,
    /// so where windows reach corpus scale, the exact solve's numbers
    /// (the table below, and the pinned crossover) replace it.
    ///
    /// The ballpark answers, at the specification BDP:
    ///
    /// - **Is the default enough?** For any corpus whose mean encoded
    ///   record size is at least 52 B, yes: the default imposes no
    ///   window-induced serialization at all, because the in-flight
    ///   disputes' own transfer time covers the round trip. That
    ///   52 B crossover comes from the exact solve, evaluated
    ///   self-consistently (each record size at its own BDP-scale
    ///   corpus: the specification BDP in `m`-sized records, per side)
    ///   and pinned by `default_crossover_matches_the_solve`;
    ///   the closed form's safe-side estimate is ~84 B.
    /// - **What budget removes the wait entirely?** About
    ///   `BDP × 5431 / (43 + m)` bytes. The design record (`m = 172`)
    ///   needs ~316 MB, where the solve agrees with the form to three
    ///   digits (this is the design point the envelope is pinned at).
    ///   A minimal `u64`-record corpus (9 B encoded) needs ~1.3 GB by
    ///   the form, ~0.8 GB by the solve: population caps thin the deep
    ///   charge at BDP-scale corpora, so the estimate is conservative
    ///   there.
    /// - **What does a smaller budget cost?** Smooth latency, never
    ///   memory, and only on the interleaved dispute walk (bulk supply
    ///   runs stream outside the window). `u64` records at the default
    ///   run at ~2.6× wire time for a BDP-scale corpus, and the factor
    ///   grows slowly with set size as the derived window narrows:
    ///   ~11.5× at 10⁷ messages, ~21.4× at 10¹⁰ (all derived from the
    ///   solve). `tests/window_operator.rs` holds the wave model
    ///   against measured sessions on a bandwidth-limited link.
    ///
    /// The table below is the full sizing reference: worst-case
    /// wire-time slowdown by budget and mean encoded record size `m`,
    /// with cells clamped at the 1.0× optimum. Each row's window `K`
    /// (second column, in disputed scopes) is derived by the same
    /// solve sessions run at handshake time, evaluated at the design
    /// session of 62500-message corpora a side; larger corpora derive
    /// narrower windows. Each cell then applies the measured wave form
    /// `slowdown = max(1, BDP_messages / K)`, with
    /// `BDP_messages = BDP / (43 + m)` evaluated at the specification
    /// BDP of 12.5 MB (the wave form is measured:
    /// `tests/window_knee.rs`, `tests/window_operator.rs`). One
    /// caution when reading it: in rows whose window reaches the
    /// design session's population ceiling of 62500 scopes (every
    /// stage granted its full population envelope), the cells for
    /// records smaller than the design record are upper envelopes at
    /// the stated corpus, not predictions for yours; a corpus at such
    /// a column's own BDP scale derives its own, wider window.
    ///
    #[doc = include_str!("tree/mirror/streaming/window/tradeoff.md")]
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
