//! The local rumor set: [`Peer`] and its synchronized state, plus the local
//! API for sending, redacting, and observing messages. The wire-session
//! drivers (bootstrap, gossip, retire) live in [`gossip`].

use std::sync::Arc;

use before::Party;
use rand::{RngCore, rngs::OsRng};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{Mutex, watch};

use crate::bookmark::{BookmarkError, Bookmarked, NoBookmark};
use crate::link::{Acceptor, Connector, Link};
use crate::tree::Tree;
pub use crate::tree::mirror::streaming::remote::DEFAULT_TARGET_MESSAGE_SIZE;
use crate::tree::mirror::streaming::remote::RunBudget;
pub use crate::tree::mirror::streaming::window::DEFAULT_SYNC_MEMORY_BUDGET;
use crate::tree::mirror::streaming::window::WindowConfig;
use crate::{
    Batch, Bookmark, CausalMessages, Network, Protocol, Rumors, Snapshot, UnorderedMessages,
    Version,
};

use serde::Serialize;
use serde::de::DeserializeOwned;
mod bootstrap;
mod gossip;

pub use bootstrap::{BookmarkedBootstrap, Bootstrap, Joined};
pub use gossip::{Gossiped, Led, PROTOCOL_MAGIC, Retire, Unbookmarked};

/// The start and end of a [`Rumors`]'s lifecycle.
///
/// A [`Peer`] is the unique `!Clone` anchor for a participant's identity in
/// the gossip protocol. Peer identity in [`rumors`](crate) is *not*
/// self-sovereign: it descends from the community of [`Peer`]s. Exactly *one*
/// [`Peer`] should call [`Peer::seed`] to establish the unique [`Network`];
/// peers whose identities descend from different calls to [`Peer::seed`] can
/// never [`gossip`](Rumors::gossip) with one another.
///
/// A [`Peer`] can exist only while no [`Rumors`] handles to the same identity
/// are outstanding, so it is statically impossible to
/// [`retire`](Peer::retire) one out from under another handle.
///
/// # Example
///
/// The lifecycle of a [`Peer`] usually looks something like this:
///
/// ```
/// use rumors::{Peer, Retire};
///
/// # tokio::runtime::Builder::new_current_thread()
/// #     .build()
/// #     .unwrap()
/// #     .block_on(async {
/// // The counterparty this example talks to: the universe's seed, serving
/// // the bootstrap and later absorbing the retirement, over in-memory links.
/// let counterparty = Peer::<String>::seed().into_rumors();
/// let (mut near, mut far) = rumors::link::memory();
/// # let serve = counterparty.clone();
/// # tokio::spawn(async move {
/// #     serve.gossip(&mut far).await.unwrap();
/// # });
/// // A real deployment would dial a different provider here; this example's
/// // counterparty is established, so the retry path is never taken.
/// async fn bootstrap_from_another_peer() -> Result<Peer<String>, rumors::Error> {
///     unreachable!("the example's counterparty is the established seed")
/// }
///
/// // Join an existing universe through any connected peer. (The universe's
/// // very first peer is created with `Peer::seed()` instead.)
/// let peer = match Peer::<String>::bootstrap().join(&mut near).await? {
///     Some(peer) => peer,
///     // The counterparty was *itself* bootstrapping: neither side holds
///     // a universe to share yet, and nothing was exchanged. Connect to a
///     // different, more established peer and try again.
///     None => bootstrap_from_another_peer().await?,
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
/// // Leave the universe, donating our identity to any gossiping peer (it
/// // does not need to be the one we bootstrapped from).
/// let (mut near, mut far) = rumors::link::memory();
/// # tokio::spawn(async move {
/// #     counterparty.gossip(&mut far).await.unwrap();
/// # });
/// let retry = match peer.retire(&mut near).await {
///     // The peer absorbed our identity; nothing more to do.
///     Retire::Retired => None,
///     // The peer was itself retiring, so it could not absorb us;
///     // retry against a different peer.
///     Retire::Declined { peer } => Some(peer),
///     // The session failed before we sent our identity to the peer;
///     // retry here or elsewhere.
///     Retire::Recovered { peer, error: _ } => Some(peer),
///     // The session failed after we sent our identity: the peer may
///     // hold it, so we cannot safely retry.
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
/// others. At first, this will lead to many [`Error::NetworkMismatch`](crate::Error::NetworkMismatch)es;
/// whenever a peer observes one, it can use a deterministic metric to decide
/// whether it or its peer should dominate.
///
/// A reasonable such metric ships inside the error itself: compare its
/// `local_min_events` against its `remote_min_events`. The greater minimal
/// event count wins, with ties broken by comparing the two [`Network`] ids
/// (their ordering is total).
/// Each side declared its count in the session's handshake, so both apply
/// the rule from the one error alone, with nothing further to fetch or
/// race, and agree without coordination: the greater persists in its
/// [`Peer`] identity, and the lesser attempts to
/// re-[`bootstrap`](Peer::bootstrap) into the dominating [`Network`].
///
/// If peers are reasonably well-connected as the network gets started, this
/// quickly reaches a stable steady state, disrupted only if a group of new
/// peers joins exclusively with one another and spends a long time
/// partitioned before reuniting with the rest of the network.
pub struct Peer<T, B: BookmarkError = NoBookmark> {
    pub(crate) network: Network,
    pub(crate) protocol: Protocol,
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
}

/// The replica's shared mutable state, behind the `watch` channel every
/// handle and observer subscribes to: the identity (absent only while a
/// retirement has it in flight) and the content tree.
///
/// Mutations happen inside `send_if_modified` critical sections so observers
/// wake exactly once per committed change.
pub(crate) struct Inner<T> {
    pub(crate) party: Option<Party>,
    pub(crate) tree: Tree<T>,
}

/// A summary view (network, latest version, live-message count), independent
/// of `T: Debug`: the messages themselves are not printed.
impl<T, B: BookmarkError> std::fmt::Debug for Peer<T, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner.borrow();
        f.debug_struct("Peer")
            .field("network", &self.network)
            .field("protocol", &self.protocol)
            .field("latest", inner.tree.latest())
            .field("len", &inner.tree.len())
            .finish_non_exhaustive()
    }
}

impl<T> Peer<T, NoBookmark> {
    /// Create the distinguished seed rumor set: the single root from which
    /// every other participant must [`bootstrap`](Peer::bootstrap).
    ///
    /// Call this exactly once per universe of cooperating peers.
    pub fn seed() -> Self {
        Self::seed_rng(&mut OsRng)
    }

    /// Like [`seed`](Self::seed), but draws the universe's [`Network`]
    /// identifier from a caller-supplied RNG instead of [`OsRng`].
    #[doc(hidden)]
    pub fn seed_rng<R: RngCore + ?Sized>(rng: &mut R) -> Self {
        Self {
            network: Network::from_rng(rng),
            protocol: Protocol::default(),
            window: WindowConfig::default(),
            run_budget: RunBudget::default(),
            inner: watch::Sender::new(Inner {
                party: Some(Party::seed()),
                tree: Tree::new(),
            }),
            bookmark: Arc::new(Mutex::new(Bookmarked::new(NoBookmark))),
        }
    }
}

impl<T> Peer<T> {
    /// Begin joining an existing universe: the [`Bootstrap`] configuration
    /// for one session against an established provider.
    ///
    /// [`Bootstrap::join`] runs the session and returns the brand-new peer;
    /// its docs state the session contract (the mutual-bootstrap bail, what
    /// a failure at the very end can cost, the unbookmarked arrival). The
    /// builder's settings ([`Bootstrap::protocol`],
    /// [`Bootstrap::sync_memory_budget`],
    /// [`Bootstrap::target_message_size`]) are the peer-to-be's own,
    /// selected before it exists so the bootstrap session and every
    /// session after it run configured. The zero-configuration join is
    /// `Peer::bootstrap().join(&mut link)`.
    pub fn bootstrap() -> Bootstrap<T> {
        Bootstrap::new()
    }

    /// Attach `bookmark` to this [`Peer`], persisting its identity before
    /// returning.
    ///
    /// A joining peer can skip this step: selecting the bookmark on the
    /// builder ([`Bootstrap::bookmark`]) hands back the peer already
    /// attached, with no window in which a crash could strand the received
    /// identity unrecorded.
    ///
    /// This peer's own identity is [`load`](crate::Bookmark::load)ed into the
    /// record and [`store`](crate::Bookmark::store)d back *eagerly*, here, so a
    /// freshly received fork cannot strand on a crash before the first gossip.
    /// Reclaiming *other* stranded identities (which grows the live party) is
    /// left to the first gossip, behind that path's persist gate, never done at
    /// attach.
    ///
    /// A pristine [`seed`](Peer::seed), with nothing sent and no identity yet
    /// donated or absorbed, has nothing worth persisting, so this touches
    /// storage only once the peer *knows* something: any content, or any
    /// identity beyond the undivided seed.
    ///
    /// # Errors
    ///
    /// If the bookmark cannot be read or written, nothing reaches storage and
    /// the peer is handed back **untouched**, still unbookmarked, inside
    /// [`Unbookmarked`], to drop or retry. Because the attach never reclaims, the
    /// live party is exactly as it was: a failed attach cannot leave reclaimed
    /// identity live in this peer yet stranded on disk.
    pub async fn bookmark<B: Bookmark>(
        self,
        bookmark: B,
    ) -> Result<Peer<T, B>, Unbookmarked<T, B>> {
        self.bookmark_inner(bookmark).await
    }
}

impl<T, B: Bookmark> Peer<T, B> {
    /// Retire this rumor set into a remote peer, handing it our identity so
    /// that it can be recycled by the network.
    ///
    /// See the [type-level lifecycle example](Peer) for how to handle the
    /// four [`Retire`] outcomes; in brief, a session reconciles content
    /// exactly as [`gossip`](crate::Rumors::gossip) would, then the peer
    /// absorbs our identity, and the outcome reports what survived. What
    /// `Ok`, `Err`, and cancellation promise is stated in [what a session
    /// promises](crate::link::Link#what-a-session-promises).
    pub async fn retire<CR, CW, C, A>(self, link: &mut Link<CR, CW, C, A>) -> Retire<T, B>
    where
        T: DeserializeOwned + Serialize + Send + Sync + 'static,
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

    /// Select the reconciliation protocol used by this peer's future sessions.
    ///
    /// The choice follows the peer through [`into_rumors`](Self::into_rumors),
    /// cloning and reunion, bookmarking, and retirement. Both endpoints of a
    /// connection must select the same protocol. New peers default to
    /// [`Protocol::V2`].
    #[must_use]
    pub fn protocol(mut self, protocol: Protocol) -> Self {
        self.protocol = protocol;
        self
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
    /// Like [`protocol`](Self::protocol), the choice follows the peer
    /// through [`into_rumors`](Self::into_rumors), cloning and reunion,
    /// bookmarking, and retirement. `Protocol::V1` sessions (behind the
    /// `protocol-v1` cargo feature) ignore it: the alternating protocol
    /// batches whole levels instead of pipelining.
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
    /// message costs 35 B of wire overhead on top of its record
    /// (calibrated by deterministic byte counts,
    /// `tests/dispute_wire.rs`).
    ///
    /// For mental arithmetic, one closed form estimates the whole
    /// trade. A session's worst-case slowdown, relative to a session
    /// limited only by wire time, is about
    ///
    /// > `slowdown ≈ max(1, BDP × 5431 / (budget × (35 + m)))`
    ///
    /// Read it as a ratio of two message counts: how many disputed
    /// messages the wire holds, `BDP / (35 + m)`, against how many the
    /// budget keeps in flight, `budget / 5431`. Slowdown 1 is
    /// wire-time-optimal: bandwidth-bound stays bandwidth-bound.
    ///
    /// The estimate has a stated accuracy band. It overstates the
    /// window by roughly `F / budget`, where `F` is the corpus-fixed
    /// component of the real charge, so the slowdown it returns runs
    /// ~2.3× low at a 10 MB budget, ~1.6× low at 16 MiB, and within a
    /// few percent past ~300 MB. It also prices no population ceiling,
    /// so where windows reach corpus scale, the exact solve's numbers
    /// (the table below, and the pinned crossover) replace it.
    /// Measured: sessions whose serialized one-way trips are counted
    /// exactly on a virtual clock, at 10–31 MB budgets on the design
    /// corpus, ran 1.3–1.65× the form's figure
    /// (`tests/tradeoff_probe.rs`).
    ///
    /// The ballpark answers, at the specification BDP:
    ///
    /// - **Is the default enough?** For any corpus whose mean encoded
    ///   record size is at least 60 B, yes: the default imposes no
    ///   window-induced serialization at all, because the in-flight
    ///   disputes' own transfer time covers the round trip. That
    ///   60 B crossover comes from the exact solve, evaluated
    ///   self-consistently (each record size at its own BDP-scale
    ///   corpus: the specification BDP in `m`-sized records, per side)
    ///   and pinned by `default_crossover_matches_the_solve`;
    ///   the closed form's safe-side estimate is ~91 B.
    /// - **What budget removes the wait entirely?** About
    ///   `BDP × 5431 / (35 + m)` bytes. The design record (`m = 172`)
    ///   needs ~330 MB, where the solve agrees with the form to three
    ///   digits (this is the design point the envelope is pinned at).
    ///   A minimal `u64`-record corpus (9 B encoded) needs ~1.5 GB by
    ///   the form, ~1.1 GB by the solve: population caps thin the deep
    ///   charge at BDP-scale corpora, so the estimate is conservative
    ///   there.
    /// - **What does a smaller budget cost?** Smooth latency, never
    ///   memory, and only on the interleaved dispute walk (bulk supply
    ///   runs stream outside the window). `u64` records at the default
    ///   run at ~4.3× wire time for a BDP-scale corpus, and the factor
    ///   grows slowly with set size as the derived window narrows:
    ///   ~13.6× at 10⁷ messages, ~25.3× at 10¹⁰ (all derived from the
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
    /// `BDP_messages = BDP / (35 + m)` evaluated at the specification
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
    /// leaf per message, and values above the wire's framing ceiling
    /// (`u32::MAX` less the frame envelope) saturate to it, so a run built
    /// within the target always fits its length header.
    ///
    /// Like [`protocol`](Self::protocol), the choice follows the peer
    /// through [`into_rumors`](Self::into_rumors), cloning and reunion,
    /// bookmarking, and retirement. `Protocol::V1` sessions (behind the
    /// `protocol-v1` cargo feature) ignore it: the alternating protocol's
    /// wire format is frozen.
    #[must_use]
    pub fn target_message_size(mut self, bytes: usize) -> Self {
        self.run_budget = RunBudget::from_bytes(bytes);
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

    pub(crate) fn send(&self, message: T) -> Batch<'_, T>
    where
        T: Serialize + Send + Sync,
    {
        let mut batch = self.batch();
        batch.send(message);
        batch
    }

    pub(crate) fn redact(&self, version: &Version) -> Batch<'_, T>
    where
        T: Send + Sync,
    {
        let mut batch = self.batch();
        batch.redact(version);
        batch
    }

    pub(crate) fn batch(&self) -> Batch<'_, T>
    where
        T: Send + Sync,
    {
        Batch::new(&self.inner)
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

    /// Force this set's tree to compute its lazy structural memos (observable
    /// hash and ceiling/floor version bounds), so a subsequent operation is
    /// timed against its own work. For benchmark and test calibration only.
    #[doc(hidden)]
    pub fn warm_caches(&self) {
        self.inner.borrow().tree.warm_caches();
    }

    /// Alias this set's live party for invariant assertions in tests:
    /// compare it, [`join`](Party::join) it into an accounting fold, or test
    /// [`is_disjoint`](Party::is_disjoint); never use it as an identity.
    ///
    /// The alias shares the live party's identity space without forking it,
    /// so treating it as a participant violates the linearity everything
    /// else rests on. `None` only while a retirement has the party in
    /// flight.
    #[cfg(any(test, feature = "test-internals"))]
    #[doc(hidden)]
    pub fn dangerously_alias_party(&self) -> Option<Party> {
        self.inner
            .borrow()
            .party
            .as_ref()
            .map(Party::dangerously_alias)
    }
}
