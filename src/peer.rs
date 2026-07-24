//! The local rumor set: [`Peer`] and its synchronized state, plus the local
//! API for sending, redacting, and observing messages. The wire-session
//! drivers (bootstrap, gossip, retire) live in [`gossip`].

use std::sync::Arc;

use before::Party;
use borsh::{BorshDeserialize, BorshSerialize};
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
    Batch, Bookmark, CausalMessages, Error, Key, Network, Protocol, Rumors, Snapshot,
    UnorderedMessages, Version,
};

mod gossip;

pub use gossip::{Gossiped, Led, PROTOCOL_MAGIC, Retire, Unbookmarked};

/// The start and end of the lifecycle of a [`Rumors`].
///
/// A [`Peer`] is the unique `!Clone` anchor for the identity of a participant
/// in the gossip protocol. Peer identity in [`rumors`](crate) is *not*
/// self-sovereign; you might say that in order to have a sense of `self`, you
/// must derive it from your community of [`Peer`]s. Exactly *one* [`Peer`]
/// should call [`Peer::seed`] to establish the unique [`Network`]; peers whose
/// identity descends ultimately from different initial calls to [`Peer::seed`]
/// will never be able to [`gossip`](Rumors::gossip) with one another.
///
/// You can only get your hands on a [`Peer`] when there are no existing
/// [`Rumors`] handles outstanding, which ensures that it is statically
/// impossible to [`retire`](Peer::retire) a [`Peer`] out from under another
/// extant handle to the same identity.
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
/// # // The counterparty this example talks to: the universe's seed, serving
/// # // the bootstrap and later absorbing the retirement, over in-memory links.
/// # let counterparty = Peer::<String>::seed().into_rumors();
/// # let (mut near, mut far) = rumors::link::memory();
/// # let serve = counterparty.clone();
/// # tokio::spawn(async move {
/// #     serve.gossip(&mut far).await.unwrap();
/// # });
/// # async fn bootstrap_from_another_peer() -> Result<Peer<String>, rumors::Error> {
/// #     unreachable!("the example's counterparty is the established seed")
/// # }
/// // Join an existing universe through any connected peer. (The universe's
/// // very first peer is created with `Peer::seed()` instead.)
/// let peer = match Peer::<String>::bootstrap(&mut near).await? {
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
/// # let (mut near, mut far) = rumors::link::memory();
/// # tokio::spawn(async move {
/// #     counterparty.gossip(&mut far).await.unwrap();
/// # });
/// //
/// // Each outcome tells us whether our identity survived the attempt:
/// // `Declined` and `Recovered` hand the peer back to retry elsewhere,
/// // while `Retired` and `Uncertain` consume it.
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
/// others. At first, this will lead to many [`Error::NetworkMismatch`]es;
/// whenever a peer observes one, it can use a deterministic metric to decide
/// whether it or its peer should dominate.
///
/// A reasonable such metric ships inside the error itself: compare
/// [`Error::NetworkMismatch`]'s `local_min_events` against its
/// `remote_min_events`, so that whichever universe records the greater
/// minimal event count wins, with total comparison on [`Network`] breaking
/// ties. Both fields ride the one error — each side declared its count in
/// the session's handshake — so both sides apply the rule from the error
/// alone, with nothing further to fetch or race. Based on this comparison,
/// both come to uncoordinated consensus on which will persist in its
/// [`Peer`] identity (the greater), and which will attempt to
/// re-[`bootstrap`](Peer::bootstrap) into the dominating [`Network`] (the
/// lesser).
///
/// If peers are reasonably well-connected to one another as the network gets
/// started, this will quickly lead to a stable and steady state which can only
/// be disrupted if a group of new peers join only with one another and spend a
/// long time partitioned from the rest of the network before reuniting with it.
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
    /// Bootstrap a brand-new rumor set from a remote peer.
    ///
    /// `Ok(None)` means the counterparty was itself still bootstrapping, so
    /// neither side had anything to share and no identity moved. It is a
    /// clean session boundary: the link remains usable. Connect to another
    /// peer and try again.
    ///
    /// On `Ok(Some(peer))` the provider has confirmed committing its side of
    /// the donation. The confirmation exchange leaves one irreducible
    /// residue (the two-generals problem): if the session fails at the very
    /// end with [`Error::Epilogue`], the provider may have committed while
    /// our side reports an error, and the forked identity is lost. Losing a
    /// fork is safe — no invariant depends on it arriving — but not free:
    /// its id-region is identity space gone for good, unless coordination
    /// outside this library reclaims it. What `Err` and cancellation leave
    /// behind is stated in [what a session
    /// promises](crate::link::Link#what-a-session-promises).
    ///
    /// The peer arrives unbookmarked: its identity has been forked away to us
    /// but not yet persisted, so a crash before it is recorded strands it. To
    /// make the received identity durable, attach a [`Bookmark`] with
    /// [`bookmark`](Peer::bookmark) immediately.
    pub async fn bootstrap<CR, CW, C, A>(
        link: &mut Link<CR, CW, C, A>,
    ) -> Result<Option<Self>, Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'static,
        CR: AsyncRead + Unpin + Send,
        CW: AsyncWrite + Unpin + Send,
        C: Connector,
        A: Acceptor,
    {
        Self::bootstrap_with_protocol(Protocol::V2, link).await
    }

    /// Bootstrap using an explicitly selected reconciliation protocol.
    ///
    /// Use this companion to [`protocol`](Peer::protocol) when joining through
    /// a provider which selected a non-default dialect such as `Protocol::V1`
    /// (behind the `protocol-v1` cargo feature). The returned peer retains
    /// `protocol` for all later sessions.
    pub async fn bootstrap_with_protocol<CR, CW, C, A>(
        protocol: Protocol,
        link: &mut Link<CR, CW, C, A>,
    ) -> Result<Option<Self>, Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'static,
        CR: AsyncRead + Unpin + Send,
        CW: AsyncWrite + Unpin + Send,
        C: Connector,
        A: Acceptor,
    {
        Self::bootstrap_inner(protocol, link).await
    }

    /// Attach `bookmark` to this [`Peer`], persisting its identity before
    /// returning.
    ///
    /// This peer's own identity is [`load`](crate::Bookmark::load)ed into the
    /// record and [`store`](crate::Bookmark::store)d back *eagerly*, here, so a
    /// freshly received fork cannot strand on a crash before the first gossip.
    /// Reclaiming *other* stranded identities — which grows the live party — is
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
    /// live party is exactly as it was: a failed attach cannot leave a reclaimed
    /// region live in this peer yet stranded on disk.
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
    /// See the [type-level lifecycle example](Peer) for how the four
    /// [`Retire`] outcomes are handled; in brief, a session reconciles content
    /// exactly as [`gossip`](crate::Rumors::gossip) would and then the peer
    /// absorbs our identity, with the outcome reporting what survived. What
    /// `Ok`, `Err`, and cancellation promise is stated in [what a session
    /// promises](crate::link::Link#what-a-session-promises).
    pub async fn retire<CR, CW, C, A>(self, link: &mut Link<CR, CW, C, A>) -> Retire<T, B>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'static,
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
    /// Reconciliation pipelines disputed subtrees so wire latency is paid
    /// per tree level rather than per disputed subtree. Pipelining is
    /// what costs memory — kilobytes per disputed subtree in flight,
    /// priced by the storage backend's own cost function — and
    /// `budget_bytes` is its worst-case envelope, not an allocation: a
    /// session holds only what it actually disputes, typically
    /// kilobytes. The budget also pre-charges the decode fans' flat
    /// residency (one fan of backend-priced leaves plus an in-hand
    /// record per reply stream — `fans` in the forms below, ~0.2 MB
    /// under the in-memory backend). Encoded wire messages in hand are
    /// the one term this setting does not govern: derived from the
    /// stream schedule, at most one run per stream per direction, so up
    /// to [`STREAM_COUNT`](crate::link::STREAM_COUNT) ×
    /// [`target_message_size`](Self::target_message_size) — ~19 MB per
    /// direction at the defaults, plus a lone over-target record's
    /// overhang.
    ///
    /// Each session divides the budget into fixed per-level channel
    /// capacities from what the two replicas exchange at session start:
    /// exact set sizes and version-size bounds, so every input to the
    /// worst case is on the table before the descent begins. Under
    /// uniform content hashing, dispute populations thin geometrically
    /// with depth and scale with the *product* of the two set sizes, so
    /// the budget buys width only where disputes can exist. The setting
    /// is not wire-visible: peers with different budgets interoperate.
    ///
    /// A budget can add latency, never break a session. A divergence
    /// wider than the derived capacities drains in capacity-sized
    /// waves, at the worst-case factor the closed form below prices;
    /// any budget, including zero, leaves every session deadlock-free
    /// at one disputed subtree in flight per level. The budget is per
    /// session: concurrent gossip on separate links carries one
    /// envelope each. The default, [`DEFAULT_SYNC_MEMORY_BUDGET`], is a
    /// stated policy choice — 512 MiB, chosen round rather than derived
    /// — and what it buys is read off the same form.
    ///
    /// # Choosing a budget
    ///
    /// One closed form answers every question here, and you arrive
    /// holding two of its three quantities. `BDP = bandwidth × RTT` is
    /// your link's bandwidth-delay product in bytes — the one number
    /// your link contributes; measure it. `m` is your corpus's mean
    /// encoded record size, the borsh-encoded payload of a disputed
    /// message's leaf record. `budget` is this setting. The two
    /// constants are derived and pinned: the 4865 B per-scope session
    /// envelope by exact recomputation, the 28 B per-message wire
    /// overhead by deterministic byte-count calibration
    /// (`tests/dispute_wire.rs`). Every figure below is derived from
    /// those inputs and re-derives if any of them move; worked examples
    /// are at the specification BDP of 12.5 MB, where 1 Gbps × 100 ms
    /// and 100 Gbps × 1 ms coincide — substitute your own measurement.
    ///
    /// ## What minimal record size runs at minimal latency, given my BDP and budget?
    ///
    /// > `m* = BDP × 4865 / budget − 28`
    ///
    /// At the spec BDP and the default 512 MiB, `m* ≈ 85.3 B`: the
    /// default imposes **no window-induced serialization for any corpus
    /// whose mean encoded record size is at least 86 B** — above the
    /// crossover, the in-flight disputes' own transfer time covers the
    /// round trip.
    ///
    /// ## What budget ensures minimal latency, given my BDP and record size?
    ///
    /// > `budget* = BDP × 4865 / (28 + m)`
    ///
    /// At the spec BDP: a minimal 8-byte-record corpus (`m = 8`) needs
    /// ~1.7 GB; the design record (`m = 172`) needs ~304 MB.
    ///
    /// ## What slowdown will I incur, given BDP, record size, and budget?
    ///
    /// > `slowdown = max(1, BDP × 4865 / (budget × (28 + m)))`
    ///
    /// relative to wire-time-optimal. At the spec BDP, `u64` records at
    /// the default budget run at worst ~3.1×. The table below is this
    /// question rendered at the spec BDP.
    ///
    /// Slowdown 1 is wire-time-optimal: bandwidth-bound stays
    /// bandwidth-bound. The factor prices the interleaved dispute walk
    /// only — supply runs stream outside the window — and degradation
    /// below the crossover is smooth `1/(28 + m)` latency, never
    /// memory. The form is an envelope for the operating regime:
    /// budgets past a few MB (below that, the flat decode-fan
    /// pre-charge above and the near-root band's full-fan scope prices
    /// both bite, so the form understates the factor) and sets up to
    /// roughly the BDP in messages (the per-scope envelope grows slowly
    /// with set size past it). `tests/window_operator.rs` holds the
    /// underlying wave model against measured sessions on a
    /// bandwidth-limited link, and `tests/dispute_wire.rs` pins the
    /// per-message wire law the record size `m` enters through.
    ///
    #[doc = include_str!("tree/mirror/streaming/window/tradeoff.md")]
    ///
    /// Like [`protocol`](Self::protocol), the choice follows the peer
    /// through [`into_rumors`](Self::into_rumors), cloning and reunion,
    /// bookmarking, and retirement. `Protocol::V1` sessions (behind the
    /// `protocol-v1` cargo feature) ignore it: the alternating protocol
    /// batches whole levels instead of pipelining.
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
    /// sequence of leaf records. Batching is chunked by bytes — a run
    /// flushes once appending the next leaf would push the message's full
    /// encoded size, framing included, past `bytes` — and every run carries
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
    /// the **minimum** of the two ends' targets — the greeting carries
    /// each side's setting, so yours bounds both the frames you build
    /// and the frames built for you, and the more memory-constrained
    /// peer sets the pace. Peers with different settings interoperate.
    ///
    /// The default, [`DEFAULT_TARGET_MESSAGE_SIZE`], is the byte size of
    /// the wire's maximally disputed reply — the decode side's documented
    /// per-reply memory unit — so default batching never raises the wire's
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
    /// concurrently. Once only one remaining [`Rumors`] exists again, it can be
    /// converted back into a [`Peer`] using
    /// [`try_into_peer`](Rumors::try_into_peer).
    pub fn into_rumors(self) -> Rumors<T, B> {
        Rumors::new(self)
    }

    pub(crate) fn send(&self, message: T) -> Batch<'_, T>
    where
        T: BorshSerialize + Send + Sync,
    {
        let mut batch = self.batch();
        batch.send(message);
        batch
    }

    pub(crate) fn redact(&self, key: Key) -> Batch<'_, T>
    where
        T: Send + Sync,
    {
        let mut batch = self.batch();
        batch.redact(key);
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
    /// [`is_disjoint`](Party::is_disjoint) — never use it as an identity.
    ///
    /// The alias shares the live party's id-region without forking it, so
    /// treating it as a participant violates the linearity everything else
    /// rests on. `None` only while a retirement has the party in flight.
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
