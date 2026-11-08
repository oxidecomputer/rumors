//! The local rumor set: [`Peer`] and its synchronized state, plus the local API
//! for sending, redacting, and observing messages. The wire-session drivers
//! (bootstrap, gossip, retire) live in [`gossip`].

use std::{
    borrow::Borrow,
    ops::{Bound, RangeBounds},
    sync::{Arc, PoisonError, RwLock},
};

use before::Party;
use rand::CryptoRng;
use rand::rand_core::UnwrapErr;
use rand::rngs::SysRng;
use tokio::io::{AsyncBufRead, AsyncWrite};
use tokio::sync::watch;

use crate::bookmark::{BookmarkHandle, NoBookmark, Reservation, Reservations};
use crate::counter::widen;
use crate::default;
use crate::message::{EncodeError, PayloadCodec, PayloadDepthLimit};
use crate::network::{NetworkKeys, NetworkSecret};
use crate::observe::{Attachment, Observer};
pub use crate::tree::mirror::streaming::remote::MAX_RUN_BUDGET_BYTES;
use crate::tree::mirror::streaming::remote::{RunBudget, WriteBatch};
use crate::tree::mirror::streaming::{
    Local,
    materialized::{self, SessionBudget},
};
use crate::tree::{self, Tree};
use crate::{
    Batch, Bookmark, CausalMessages, Changes, Committed, Network, Rumors, Snapshot,
    UnorderedMessages, Version,
};

use serde::{Serialize, de::DeserializeOwned};

/// The smallest memory budget a synchronization session runs on at the default
/// message-size target ([`default::TARGET_MESSAGE_SIZE`]) or a larger one:
/// about 36.4 MiB on 64-bit targets.
///
/// A smaller budget, zero included, is raised to at most this, whatever the
/// target, so a session counts at most the larger of its budget and this
/// minimum. It covers the room every session keeps for the work in flight at
/// each level of its comparison, up to a bound for each kind of work at each
/// level, of which the first piece is always admitted, which is how a session
/// always makes progress; room beside it for the largest single piece of work;
/// the fixed state every session holds whatever its budget; and the room for
/// messages on their way through the connection, which grows with the target
/// ([`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget) lists each).
/// That room never raises the budget: a session clamps a larger target to the
/// largest one whose room fits the larger of its budget and this minimum
/// ([`Peer::target_message_size`](crate::Peer::target_message_size)). A smaller
/// target sets less room aside, and lowers the minimum by about twice the
/// target's decrease in each direction. At or above this minimum, no measured
/// session has run more than one round trip slower at a larger budget than at a
/// smaller one; that is observed behavior, not a guarantee.
pub const MIN_SYNC_MEMORY_BUDGET: usize =
    materialized::Allowances::of::<Local>(default::TARGET_MESSAGE_SIZE).minimum();

/// The largest message of supplied records a session's memory budget covers at
/// the default message-size target ([`default::TARGET_MESSAGE_SIZE`]): about
/// 512 KiB on 64-bit targets.
///
/// A message is counted as
/// [`Peer::target_message_size`](crate::Peer::target_message_size) counts it,
/// by its whole encoded size, framing included. A session builds every message
/// within the target, except one that carries a single record larger than the
/// target, and a conforming peer does the same, so only such a record can make
/// a message larger than this. While it crosses, a larger message is held
/// beyond the budget by its excess over this size, and a session holds at most
/// one such message in each direction at once. At another target the bound is
/// about twice the target the session runs at, which its budget can clamp
/// ([`Peer::target_message_size`](crate::Peer::target_message_size)).
pub const MAX_BUDGETED_MESSAGE_SIZE: usize =
    crate::tree::mirror::streaming::remote::budgeted_message_bytes(default::TARGET_MESSAGE_SIZE);

/// A peer's memory budget setting, as stored until a session runs on it.
///
/// The setters store what the caller gives, and everything that needs the
/// budgets a session runs on computes them from this and the peer's
/// message-size target together ([`session`](Self::session)), so the order in
/// which a caller sets the two never changes the result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MemoryBudget {
    /// A budget given to a public setter: sessions clamp the peer's target to
    /// what it affords, and raise it to the minimum for the clamped target.
    Requested(usize),
    /// A ledger budget the test path sets
    /// ([`ledger_budget`](crate::testing::ledger_budget)): sessions run on the
    /// memory budget that leaves their ledgers exactly this, at whatever
    /// target, below the minimum included.
    #[cfg(any(test, feature = "test-internals"))]
    Ledger(usize),
    /// The serial configuration the test path sets
    /// ([`serial_budget`](crate::testing::serial_budget)): one unit of each
    /// kind and depth at a time.
    ///
    /// Sessions run their ledgers at a budget of zero with every kind and
    /// depth's bound zero, on the memory budget a ledger budget of zero gives.
    #[cfg(any(test, feature = "test-internals"))]
    Serial,
}

impl MemoryBudget {
    /// The message-size target and the budgets a session runs on when its
    /// peer's own target is `target`.
    ///
    /// A requested budget clamps the target to the largest one, at most
    /// `target`, whose minimum is within the larger of the budget and
    /// [`MIN_SYNC_MEMORY_BUDGET`]
    /// ([`affordable_target`](materialized::affordable_target)). Every target
    /// at or below the default is within that, so only a larger target is ever
    /// clamped, and never below the default. The budget is then raised to the
    /// minimum for the clamped target, and its ledger never admits less than
    /// the largest grant
    /// ([`Allowances::requested`](materialized::Allowances::requested)), so a
    /// session counts at most the larger of the budget and
    /// [`MIN_SYNC_MEMORY_BUDGET`], whatever the target.
    ///
    /// A ledger budget keeps `target`, and sits beside what every session at
    /// `target` sets aside, the floor allowance and the session allowance
    /// ([`Allowances::ledger`](materialized::Allowances::ledger)).
    pub(crate) fn session(self, target: RunBudget) -> SessionLimits {
        match self {
            Self::Requested(bytes) => {
                let target = RunBudget::from_bytes(materialized::affordable_target::<Local>(
                    bytes.max(MIN_SYNC_MEMORY_BUDGET),
                    target.bytes(),
                ));
                let allowances = materialized::Allowances::of::<Local>(target.bytes());
                SessionLimits {
                    target,
                    budget: allowances.requested(bytes),
                }
            }
            #[cfg(any(test, feature = "test-internals"))]
            Self::Ledger(bytes) => {
                let allowances = materialized::Allowances::of::<Local>(target.bytes());
                SessionLimits {
                    target,
                    budget: allowances.ledger(bytes),
                }
            }
            #[cfg(any(test, feature = "test-internals"))]
            Self::Serial => {
                let allowances = materialized::Allowances::of::<Local>(target.bytes());
                SessionLimits {
                    target,
                    budget: allowances.ledger(0),
                }
            }
        }
    }
}

/// The message-size target a session runs at and the budgets it runs on,
/// computed together from its peer's settings ([`MemoryBudget::session`]).
///
/// The budgets set the wire pools aside at this target, and a session sizes its
/// pools at the target its greeting advertises, so the two travel as one value.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SessionLimits {
    /// The target the session advertises and sizes its wire pools at: the
    /// peer's own, clamped to what its memory budget affords.
    pub(crate) target: RunBudget,
    /// The most the session holds in all, and what its ledger admits, at
    /// [`target`](Self::target).
    pub(crate) budget: SessionBudget,
}

/// How a peer's sessions size their memory and their writes, as its setters
/// stored them.
///
/// It carries no payload type, so it crosses whole into the sessions'
/// transport-erased bodies, which compile once for every payload type. The
/// memory budget travels beside the target it clamps, so a session derives its
/// limits from the pair its peer stored ([`limits`](Self::limits)).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SessionSizing {
    /// The memory budget selected by [`Peer::sync_memory_budget`] or
    /// [`Bootstrap::sync_memory_budget`], as given.
    pub(crate) sync_memory_budget: MemoryBudget,
    /// The supply-run byte budget selected by [`Peer::target_message_size`] or
    /// [`Bootstrap::target_message_size`], as given.
    pub(crate) run_budget: RunBudget,
    /// The write batch range selected by [`Peer::write_batch_range`] or
    /// [`Bootstrap::write_batch_range`].
    pub(crate) write_batch: WriteBatch,
}

impl SessionSizing {
    /// The message-size target a session advertises and the budgets it runs on:
    /// the memory budget's, at the stored target ([`MemoryBudget::session`]).
    pub(crate) fn limits(self) -> SessionLimits {
        self.sync_memory_budget.session(self.run_budget)
    }

    /// Start a session's local participant over `root`, listing and comparing
    /// under `keys`, at the limits the stored settings give
    /// ([`limits`](Self::limits)).
    ///
    /// The participant advertises the clamped target in its greeting, the pipe
    /// sizes its wire pools at the target the greeting advertises, and the
    /// ledger's budget is what the memory budget leaves beside those pools at
    /// that same target, so all three come from one [`SessionLimits`] here. The
    /// serial configuration replaces the ledger budget with one unit of each
    /// kind and depth at a time.
    pub(crate) fn start(
        self,
        root: tree::Root,
        keys: Option<Arc<NetworkKeys>>,
    ) -> materialized::Handshaking<Local, materialized::Start> {
        let limits = self.limits();
        let walk = materialized::Handshaking::start(
            Local,
            root.into(),
            keys,
            widen(limits.target.bytes()),
        );
        #[cfg(any(test, feature = "test-internals"))]
        if self.sync_memory_budget == MemoryBudget::Serial {
            return walk.serial();
        }
        walk.budget(limits.budget.ledger)
    }
}

/// The settings a peer's sessions run under, which a [`Bootstrap`] builder
/// selects and the peer it joins inherits whole.
///
/// The payload nesting limit is not among them: a peer holds it inside its
/// payload codec, which only a peer's construction can build, while a builder
/// holds the limit alone.
pub(crate) struct SessionSettings<T: Send + Sync + 'static> {
    /// How the sessions size their memory and their writes.
    pub(crate) sizing: SessionSizing,
    /// Initiation and deadline factories inherited by every gossip handle.
    pub(crate) gossip_policy: policy::Policy<T>,
    /// The wire observers added through [`Peer::observe`] or
    /// [`Bootstrap::observe`].
    pub(crate) observe: Attachment,
}

/// The library's defaults: no observer, push-on-change with no deadline, and
/// the default sizing.
impl<T: Send + Sync + 'static> Default for SessionSettings<T> {
    /// Use every setting's default.
    fn default() -> Self {
        Self {
            sizing: SessionSizing::default(),
            gossip_policy: policy::Policy::default(),
            observe: Attachment::default(),
        }
    }
}

/// Share the settings without imposing `Clone` on the payload.
impl<T: Send + Sync + 'static> Clone for SessionSettings<T> {
    /// Copy the sizing and share the policy's factories and the observers.
    fn clone(&self) -> Self {
        Self {
            sizing: self.sizing,
            gossip_policy: self.gossip_policy.clone(),
            observe: self.observe.clone(),
        }
    }
}

/// The library's default budget, raised like any requested budget.
impl Default for MemoryBudget {
    /// Request [`default::SYNC_MEMORY_BUDGET`].
    fn default() -> Self {
        Self::Requested(default::SYNC_MEMORY_BUDGET)
    }
}

mod bookmark;
mod bootstrap;
mod connection;
mod gossip;
mod policy;

pub use bookmark::Unbookmarked;
pub use bootstrap::{Bootstrap, Joined};
pub use connection::Connection;
#[cfg(any(test, feature = "test-internals"))]
pub(crate) use gossip::COMPLETION_MARKER;
pub use gossip::{Gossip, Gossiped, Led, Retire};

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
/// use rumors::{Connection, Joined, Peer, Retire};
///
/// # tokio::runtime::Builder::new_current_thread()
/// #     .build()
/// #     .unwrap()
/// #     .block_on(async {
/// // The counterparty this example talks to: the universe's seed, serving
/// // the bootstrap and later accepting the retirement, over in-memory
/// // connections. Each session takes the two halves of one connection.
/// let counterparty = Peer::<String>::seed().into_rumors();
/// let (near, far) = tokio::io::duplex(64 * 1024);
/// let (near_read, near_write) = tokio::io::split(near);
/// let near_read = tokio::io::BufReader::new(near_read);
/// # let serve = counterparty.clone();
/// # tokio::spawn(async move {
/// #     let (far_read, far_write) = tokio::io::split(far);
/// #     let far_read = tokio::io::BufReader::new(far_read);
/// #     serve.gossip_once(far_read, far_write).await.unwrap();
/// # });
/// // Join through an established peer. The new peer receives its full set.
/// let Joined::Joined { peer, .. } = Peer::<String>::bootstrap()
///     .join(near_read, near_write)
///     .await
/// else {
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
/// let (near, far) = tokio::io::duplex(64 * 1024);
/// let (near_read, near_write) = tokio::io::split(near);
/// let near_read = tokio::io::BufReader::new(near_read);
/// # tokio::spawn(async move {
/// #     let (far_read, far_write) = tokio::io::split(far);
/// #     let far_read = tokio::io::BufReader::new(far_read);
/// #     counterparty.gossip_once(far_read, far_write).await.unwrap();
/// # });
/// let retry = match peer.retire(near_read, near_write).await {
///     // This peer left the network; nothing more to do. A lost connection
///     // carries the session's error, worth logging.
///     Retire::Retired { connection } => {
///         if let Connection::Lost { error } = connection {
///             eprintln!("retired, but the session then failed: {error}");
///         }
///         None
///     }
///     // Both sides were retiring; retry against a different peer.
///     Retire::Declined { peer, .. } => Some(peer),
///     // Retirement did not proceed. Retry using a fresh connection.
///     Retire::Recovered { peer, error: _ } => Some(peer),
/// };
/// assert!(retry.is_none(), "the example's retirement succeeds");
/// # });
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
/// [`Mismatch::Network`](crate::error::Mismatch::Network); whenever a peer
/// observes one, it can use a deterministic metric to decide whether it or its
/// peer should dominate.
///
/// A reasonable such metric ships inside the error itself: compare its
/// `local_min_events` against its `remote_min_events`. The greater minimal
/// event count wins, with ties broken by comparing the two [`Network`]
/// identifiers (their ordering is total). Each side declared its count in the
/// session's handshake, so both apply the rule from the one error alone, with
/// nothing further to fetch or race, and agree without coordination: the winner
/// stays in its network, and the loser drops its replica and attempts to
/// re-[`bootstrap`](Peer::bootstrap) into the winning [`Network`].
///
/// If peers are reasonably well-connected as the network gets started, this
/// quickly reaches a stable steady state, disrupted only if a group of new
/// peers joins exclusively with one another and spends a long time partitioned
/// before reuniting with the rest of the network.
pub struct Peer<T: Send + Sync + 'static, B: Bookmark = NoBookmark> {
    /// The keys of the network this replica belongs to, derived from the secret
    /// drawn at seed or received at join. Its public identifier is
    /// [`network`](Self::network).
    ///
    /// It is the same allocation as the keys of every tree `inner` publishes,
    /// whose path key places that tree's leaves.
    pub(crate) keys: Arc<NetworkKeys>,
    /// The settings this peer's sessions run under, shared with every gossip
    /// handle.
    pub(crate) settings: SessionSettings<T>,
    /// Shared replica state and change notifications for handles and drivers.
    pub(crate) inner: watch::Sender<Inner<T>>,
    /// The identity bookmark: persistence handle and its in-memory record,
    /// behind an async mutex and shared with every [`Rumors`] clone, beside the
    /// size limit selected by
    /// [`bookmark_size_limit`](Self::bookmark_size_limit).
    ///
    /// Separate from `inner` because persisting is `async` and the record is
    /// `!Clone`; see [`BookmarkHandle`].
    pub(crate) bookmark: BookmarkHandle<B>,
    /// The payload codec built at construction: the typed ingress every gossip
    /// session's supplied leaf records decode through.
    ///
    /// Carries the [`payload_depth_limit`](Self::payload_depth_limit) beside
    /// the codec's fn pointers (see [`PayloadCodec`]).
    pub(crate) codec: PayloadCodec,
}

/// The local settings that govern a peer's synchronization sessions.
///
/// Read these from [`Peer::synchronization_settings`] or
/// [`Rumors::synchronization_settings`](crate::Rumors::synchronization_settings).
/// A session admits its work against this peer's memory budget and uses the
/// smaller of the two peers' message-size targets. The values here are this
/// peer's inputs to those decisions.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SynchronizationSettings {
    /// The memory budget each session admits its work against
    /// ([`Peer::sync_memory_budget`]).
    ///
    /// It is the budget set, raised to the minimum for
    /// [`target_message_size`](Self::target_message_size), so never more than
    /// the larger of the budget set and [`MIN_SYNC_MEMORY_BUDGET`].
    pub sync_memory_budget: usize,
    /// The wire-message target this peer advertises
    /// ([`Peer::target_message_size`]): the target set, saturated at
    /// [`MAX_RUN_BUDGET_BYTES`] and clamped to what the memory budget affords.
    ///
    /// Passing it back to [`Peer::target_message_size`] selects the same
    /// target.
    pub target_message_size: usize,
    /// The write batch range this peer's sessions batch their writes within
    /// ([`Peer::write_batch_range`]), as they apply it.
    ///
    /// It runs from the segment size, included, to the largest batch, included
    /// and saturated at the write buffer's capacity, or unbounded. Passing it
    /// back to [`Peer::write_batch_range`] selects the same batching.
    pub write_batch_range: (Bound<usize>, Bound<usize>),
    /// The payload nesting limit this peer requires its counterpart to match.
    pub payload_depth_limit: u64,
}

/// The replica's identity and content, shared through a watch channel.
///
/// Retirement owns the consumed `Peer`, excluding other writable handles.
pub(crate) struct Inner<T: Send + Sync + 'static> {
    /// Identity used to stamp local events.
    pub(crate) party: Party,
    /// Forks reserved for donations to bootstrapping counterparties, not yet
    /// sent.
    ///
    /// Bookmark reclamation pauses for safety while any of these are
    /// outstanding.
    pub(crate) reservations: Reservations,
    /// The published content and causal ceiling.
    pub(crate) tree: Tree<T>,
    /// Serializes local tree preparation and the fallback gossip join.
    ///
    /// Party changes and optimistic swaps take it shared; local preparation and
    /// the fallback join take it exclusively. Acquire it before any watch
    /// guard. Snapshot readers do not take it. The gate contains no data to
    /// repair after a panic, so poison is ignored.
    commit_gate: Arc<RwLock<()>>,
}

/// Try the session result, then one rebase, before excluding competing writers.
///
/// More optimistic attempts can avoid excluding writers, but each failed rebase
/// wastes a join. This bounds speculative work without tuning a public
/// contract.
const OPTIMISTIC_ATTEMPTS: usize = 2;

/// Publish content and change identity ownership under the replica's locks.
impl<T: Send + Sync + 'static> Inner<T> {
    /// Construct a replica whose commits share one writer gate.
    pub(crate) fn new(party: Party, tree: Tree<T>) -> Self {
        tree.warm_memos();
        Self {
            party,
            reservations: Reservations::default(),
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

    /// Change party custody at the published frontier without notifying
    /// readers.
    ///
    /// A local commit must publish before a fork can inherit its party and
    /// history. The shared writer gate orders these changes with preparation.
    /// Callers needing a bookmark guard must acquire it before this method.
    fn update_party(sender: &watch::Sender<Self>, update: impl FnOnce(&mut Self)) {
        let gate = sender.borrow().commit_gate.clone();
        let _hold = gate.read().unwrap_or_else(PoisonError::into_inner);
        sender.send_if_modified(|inner| {
            update(inner);
            false
        });
    }

    /// Split off a fork to donate to a bootstrapping counterparty; reclamation
    /// pauses until the returned reservation drops, or until reunion forgets it
    /// ([`Reservations::release_abandoned`]).
    fn reserve(&mut self) -> (Party, Reservation) {
        (self.party.fork(), self.reservations.reserve())
    }

    /// Clone the published tree without carrying its read guard into the
    /// caller.
    fn snapshot(sender: &watch::Sender<Self>) -> Tree<T> {
        // An assignment may drop the caller's old tree. End the borrow here so
        // that a destructor triggered by that assignment runs outside this lock.
        sender.borrow().tree.clone()
    }

    /// Publish reconciled content without walking the tree under the watch
    /// lock.
    ///
    /// The session result already includes `prior`. If that snapshot is still
    /// current, swap in the result directly. Otherwise join it with a fresh
    /// snapshot outside the lock and retry. After repeated conflicts, exclude
    /// other writers for one join and swap; readers remain free to take
    /// snapshots.
    ///
    /// Keep every input and displaced root alive until both locks are released,
    /// including on unwind: this guards against a pathological case where their
    /// payload destructors may access this replica, which would otherwise
    /// deadlock. `update` runs once, under the watch lock, to publish an
    /// identity change alongside the tree. Only a tree change notifies
    /// observers.
    fn publish<E>(
        sender: &watch::Sender<Self>,
        prior: &Tree<T>,
        reconciled: &Tree<T>,
        mut update: impl FnMut(&mut Self) -> Result<(), E>,
    ) -> Result<(), E> {
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

    /// Join and publish while excluding other writers, without blocking
    /// snapshots.
    fn publish_exclusive<E>(
        sender: &watch::Sender<Self>,
        reconciled: &Tree<T>,
        gate: &RwLock<()>,
        update: &mut impl FnMut(&mut Self) -> Result<(), E>,
    ) -> Result<(), E> {
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
    /// The caller holds the writer gate and has warmed `candidate`. A conflict
    /// leaves `candidate` and `update` untouched; success moves the displaced
    /// tree into `candidate` for destruction after the gate is released.
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

/// A bounded replica summary, independent of `T: Debug`.
impl<T: Send + Sync + 'static, B: Bookmark> std::fmt::Debug for Peer<T, B> {
    /// Summarize the replica without printing payloads.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fmt_summary("Peer", f)
    }
}

impl<T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static> Peer<T, NoBookmark> {
    /// Create a new gossip network containing only this peer.
    ///
    /// Call once per network; other participants join through
    /// [`bootstrap`](Self::bootstrap). Independently seeded peers cannot gossip
    /// with each other. The payload type must follow the [payload
    /// contract](crate#choosing-a-payload-type).
    ///
    /// # Panics
    ///
    /// Panics if the operating system's random source fails.
    pub fn seed() -> Self {
        // The secret comes from the operating system on every call, never
        // from a generator whose state lives in this process: a forked
        // child inherits that state, so siblings forked after one shared
        // draw would seed networks with one secret, one identifier and one
        // seed party, and gossip as one network.
        Self::seed_from_rng(&mut UnwrapErr(SysRng))
    }

    /// Create a network with a caller-supplied random source.
    ///
    /// This test-support hook makes network secrets, and with them network
    /// identifiers, reproducible.
    #[doc(hidden)]
    #[cfg(any(test, feature = "test-internals"))]
    pub fn seed_rng<R: CryptoRng + ?Sized>(rng: &mut R) -> Self {
        Self::seed_from_rng(rng)
    }

    /// Create a new network using `rng` for its secret.
    fn seed_from_rng<R: CryptoRng + ?Sized>(rng: &mut R) -> Self {
        let keys = Arc::new(NetworkKeys::new(NetworkSecret::from_rng(rng)));
        Self {
            keys: Arc::clone(&keys),
            settings: SessionSettings::default(),
            inner: watch::Sender::new(Inner::new(Party::seed(), Tree::new(keys))),
            bookmark: BookmarkHandle::new(NoBookmark),
            codec: PayloadCodec::new::<T>(PayloadDepthLimit::default()),
        }
    }
}

impl<T: Send + Sync + 'static> Peer<T> {
    /// Configure a join to an existing gossip network.
    ///
    /// Call [`Bootstrap::join`] with a connection to an established member. The
    /// returned peer retains the builder's settings. Failed sessions return the
    /// builder for retry. See [`Joined`] for the possible outcomes.
    pub fn bootstrap() -> Bootstrap<T> {
        Bootstrap::new()
    }
}

/// Share peer internals when cloning a [`Rumors`] handle, and repair what
/// leaked sessions left behind when reclaiming the peer.
impl<T: Send + Sync + 'static, B: Bookmark> Peer<T, B> {
    /// Share this peer's replica, configuration, and bookmark with a new
    /// handle.
    ///
    /// Only [`Rumors::clone`] uses this internal operation; the public [`Peer`]
    /// remains uniquely owned.
    pub(crate) fn share(&self) -> Self {
        Self {
            keys: Arc::clone(&self.keys),
            settings: self.settings.clone(),
            inner: self.inner.clone(),
            bookmark: self.bookmark.share(),
            codec: self.codec,
        }
    }

    /// Forget the fork reservations of session futures that can never run
    /// again, so that bookmark reclamation resumes.
    ///
    /// Call this only on a peer just reclaimed from its handles; see
    /// [`Reservations::release_abandoned`] for why that is the condition.
    pub(crate) fn release_abandoned_reservations(&self) {
        Inner::update_party(&self.inner, |inner| inner.reservations.release_abandoned());
    }
}

/// Retire an exclusively held replica.
impl<T: Send + Sync + 'static, B: Bookmark> Peer<T, B> {
    /// Leave the gossip network after synchronizing with a remote member.
    ///
    /// Retiring helps keep message versions compact as peers come and go. The
    /// remote member runs ordinary [`gossip`](Rumors::gossip); it needs no
    /// special call to accept the retirement.
    ///
    /// `read` and `write` are the two halves of one connection to the remote
    /// member; see the [transport requirements](crate#transport). Pass `&mut`
    /// halves to keep the connection after the session.
    ///
    /// [`Retire`] reports whether this peer left the network or remains to keep
    /// gossiping or retry, including after
    /// [`session_deadline`](Self::session_deadline) expires, and whether the
    /// connection can carry another session. See the [lifecycle example](Peer)
    /// and [what a session promises](crate#what-a-session-promises).
    pub async fn retire<R, W>(self, read: R, write: W) -> Retire<T, R, W, B>
    where
        R: AsyncBufRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        self.retire_inner(read, write).await
    }
}

/// Inspect the network and configure replication behavior.
impl<T: Send + Sync + 'static, B: Bookmark> Peer<T, B> {
    /// Return the gossip network this replica belongs to.
    pub fn network(&self) -> Network {
        self.keys.network()
    }

    /// Return the local settings used to configure synchronization sessions.
    ///
    /// The message-size target and the memory budget are returned as sessions
    /// run on them, whichever order the two were set in: the target saturated
    /// at [`MAX_RUN_BUDGET_BYTES`] and clamped to what the budget affords,
    /// matching the value advertised to peers, and the budget raised to the
    /// minimum for that target.
    pub fn synchronization_settings(&self) -> SynchronizationSettings {
        let sizing = self.settings.sizing;
        let limits = sizing.limits();
        SynchronizationSettings {
            sync_memory_budget: limits.budget.memory,
            target_message_size: limits.target.bytes(),
            write_batch_range: sizing.write_batch.range(),
            payload_depth_limit: self.codec.limit().get(),
        }
    }

    /// Format the bounded summary shared by [`Peer`] and [`Rumors`].
    pub(crate) fn fmt_summary(
        &self,
        name: &str,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let inner = self.inner.borrow();
        let settings = self.synchronization_settings();
        f.debug_struct(name)
            .field("network", &self.network())
            .field("latest", inner.tree.latest())
            .field("len", &inner.tree.len())
            .field("sync_memory_budget", &settings.sync_memory_budget)
            .field("target_message_size", &settings.target_message_size)
            .field("write_batch_range", &settings.write_batch_range)
            .field("payload_depth_limit", &settings.payload_depth_limit)
            .finish_non_exhaustive()
    }

    /// Select how each connection initiates gossip. By default it follows
    /// changes.
    ///
    /// The factory receives a fresh [`Changes`] subscription for each
    /// [`Rumors::gossip`](crate::Rumors::gossip) driver. It can adapt that
    /// stream, combine it with periodic heartbeat requests, or ignore it and
    /// provide its own policy. An always-pending stream only serves remote
    /// initiations. An endless, always-ready stream can busy-loop.
    ///
    /// Each item converts to [`Gossip`]; `()` means initiate if this connection
    /// has new local state. The driver suppresses echoes of its own completed
    /// sessions. Remote initiations are always served, regardless of the local
    /// policy. Ending the policy stream ends the driver cleanly after any
    /// active session.
    ///
    /// The factory is shared by all handles and may be called concurrently for
    /// different connections. Each returned stream belongs to one driver.
    ///
    /// # Graceful shutdown
    ///
    /// Capture a shared shutdown signal in the factory to end the policy
    /// streams for every clone's gossip drivers. Each driver observes the
    /// signal between sessions; keep polling it until it ends so an active
    /// exchange can finish. Use [`session_deadline`](Self::session_deadline) to
    /// bound a stalled exchange.
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
    /// let (connection, _remote) = tokio::io::duplex(64 * 1024);
    /// let (read, write) = tokio::io::split(connection);
    /// let read = tokio::io::BufReader::new(read);
    /// let handle = rumors.clone();
    /// let drive = async move {
    ///     let mut sessions = handle.gossip(read, write);
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
    /// Shutdown remains in effect for new drivers using this policy; replace
    /// the policy before restarting gossip. Shutdown does not force a final
    /// synchronization of pending local changes, retire the peer, or affect
    /// explicit [`Rumors::gossip_once`] calls.
    pub fn gossip_when<F, S>(mut self, when: F) -> Self
    where
        F: Fn(crate::Changes<T>) -> S + Send + Sync + 'static,
        S: futures::Stream + Send + 'static,
        S::Item: Into<crate::Gossip>,
    {
        self.settings.gossip_policy.set_when(when);
        self
    }

    /// Supply a fresh deadline for each wire session. Disabled by default.
    ///
    /// The deadline covers gossip, bootstrap, and retirement through final
    /// confirmation. It starts when an explicit call or the local policy
    /// initiates a session, or when the first remote bytes arrive. Idle waits
    /// are untimed, and traffic does not reset the deadline.
    ///
    /// Expiry reports
    /// [`Error::DeadlineExceeded`](crate::Error::DeadlineExceeded) and leaves
    /// the connection at an unknown position: discard it. A completed local
    /// commit is not undone. [`Retire`] preserves a usable peer when recovery
    /// is safe; otherwise it reports an uncertain retirement. A gossip driver
    /// ends after the error.
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
        self.settings.gossip_policy.set_deadline(deadline);
        self
    }

    /// Limit the complete encoded bookmark, including its integrity frame.
    ///
    /// The default is [`default::BOOKMARK_SIZE_LIMIT`] (16 MiB). The budget is
    /// shared by all networks retained in this bookmark. Updates discard the
    /// oldest identities from the least recently used network, removing each
    /// network only after its last identity is gone. Eviction only loses
    /// opportunities to recycle identity space; it cannot lose replicated
    /// messages or permit version reuse.
    ///
    /// Set this before [`bookmark`](Peer::bookmark) to apply it to attachment,
    /// or afterwards to apply it at the next checkpoint.
    /// [`Bootstrap::bookmark_size_limit`] configures it before joining. Smaller
    /// values are raised to the empty frame size (43 bytes). A limit that
    /// cannot hold one identity keeps no recovery rights. Capacity is not
    /// reserved up front.
    ///
    /// This bounds stored bytes, not peak working memory: loading and encoding
    /// use temporary allocations, and an existing record may exceed a newly
    /// lowered limit until the next successful store.
    pub fn bookmark_size_limit(mut self, bytes: usize) -> Self {
        self.bookmark.set_size_limit(bytes);
        self
    }

    /// Set the memory budget for each synchronization session: an approximate
    /// limit on the memory a session holds, its work in flight and its messages
    /// on their way included.
    ///
    /// A session overlaps its round trips by keeping work in flight while
    /// replies cross the link, and that work costs memory: what the session has
    /// built, and room set aside for the replies on their way. The budget caps
    /// that memory as the session counts it: a session on a budget of 512 MiB
    /// counts at most 512 MiB of it at once, as [Accuracy](#accuracy)
    /// describes. Within it, a session keeps in flight as much as it can put to
    /// use.
    ///
    /// Part of every budget is set aside for the work in flight at each level
    /// of the session's comparison, up to a bound for each kind of work at each
    /// level, which a session can always keep, whatever else waits: the first
    /// piece of work at each level is always admitted, which is how a session
    /// always makes progress. The bounds together come to about 26.5 MiB, of
    /// which a session realistically uses a few mebibytes: about 5 MiB between
    /// replicas of millions of messages that differ throughout, and less
    /// between replicas that mostly agree. Part is set aside for the session's
    /// fixed state, which it holds whatever its budget and the sets' sizes:
    /// about 7 MiB on the in-memory replica, as [the session's fixed
    /// state](#the-sessions-fixed-state) lists. Part is set aside for the
    /// messages on their way through the connection, about two messages' worth
    /// at the peer's [`target_message_size`](Self::target_message_size) in each
    /// direction: about 1 MiB at the default target, as [messages on their
    /// way](#messages-on-their-way) describes. The rest of the budget pays for
    /// pipelining beyond all three, and always has room for at least the
    /// largest single piece of work. A session raises a budget below the
    /// minimum, zero included, to that minimum: [`MIN_SYNC_MEMORY_BUDGET`] at
    /// the default target or a larger one, and a little less at a smaller
    /// target, whose messages need less room. The target never raises the
    /// budget beyond that, since a session lowers a target whose room the
    /// budget cannot afford ([`target_message_size`](Self::target_message_size)
    /// says how). So a session counts at most the larger of `budget_bytes` and
    /// [`MIN_SYNC_MEMORY_BUDGET`], whatever the target;
    /// [`synchronization_settings`](Self::synchronization_settings) reports the
    /// budget and the target it runs on. Both read the two settings when a
    /// session starts, so the order in which you set them doesn't matter. A
    /// smaller budget therefore makes a session slower, never failed: the first
    /// piece of work at each level is still always admitted.
    ///
    /// More budget keeps more in flight, up to the point where nothing waits;
    /// at or beyond it, a session runs exactly as it would with an unlimited
    /// budget. That point can lie somewhat above the most memory the session
    /// would use with no budget at all, since the budget counts room for work
    /// before the session knows whether it needs it. Below that point, a larger
    /// budget can still lengthen a session, because a different budget changes
    /// the order in which the session admits its work. In every session
    /// measured so far, no larger budget has made a session more than one round
    /// trip slower than a smaller one; that is observed behavior, not a
    /// guarantee. A session reports what waited in
    /// [`SessionStats::budget_waits`](crate::SessionStats::budget_waits), zero
    /// when the budget did not limit it. Most sessions come nowhere near the
    /// budget: a session's need grows with how much of the two sets differs,
    /// and is large only when two large replicas differ throughout, as when a
    /// peer has fallen far behind on a large set or two peers accumulated many
    /// messages apart. A peer joining with an empty replica receives the set as
    /// a stream and needs little.
    ///
    /// The budget counts the room set aside for each reply on its way at that
    /// room's full size, though the reply may fill less of it. A session also
    /// keeps no more in flight than the budget would let it act on as the
    /// replies arrive, since work beyond that would only wait its turn. So a
    /// session can wait for budget before its memory use reaches the budget,
    /// most of all between nearly identical replicas.
    ///
    /// Each peer's budget bounds its own side of a session: your peer's budget,
    /// smaller or larger, does not change what your side holds, though a peer
    /// with a smaller budget can slow the session. The budget applies to each
    /// session separately and is not reserved up front: sessions running at
    /// once, to one peer or to several, can each use up to it.
    ///
    /// The default is [`default::SYNC_MEMORY_BUDGET`] (512 MiB), and the least
    /// is [`MIN_SYNC_MEMORY_BUDGET`] at the default target or a larger one. All
    /// [`Rumors`] clones inherit this setting, and it is preserved when
    /// returning to a `Peer`, attaching a bookmark, or retiring. [Choosing a
    /// synchronization budget](crate::sizing) explains how to choose one.
    ///
    /// # The session's fixed state
    ///
    /// Every budget sets aside room for what a session holds whatever its
    /// budget and the sets' sizes, each part bounded by fixed capacities:
    ///
    /// - **Working buffers.** A session fills small buffers as it builds a
    ///   question or answer, before it counts the result against the budget,
    ///   and keeps a few briefly after.
    /// - **Subtrees under construction.** The in-memory replica builds each
    ///   subtree a session receives whole as its leaves arrive, keeping only
    ///   the branches still open along the path of the latest leaf: at most one
    ///   partly built branch per byte of a leaf's path, whatever the subtree's
    ///   size. A session builds at most one such subtree per stream it receives
    ///   on at a time, and queues a few finished ones between the levels it
    ///   assembles.
    /// - **Decoded leaves on their way to assembly.** A session decodes the
    ///   leaves of each subtree it receives a little ahead of building it,
    ///   holding at most one more decoded leaf per stream it receives on than
    ///   the number of parts a round divides a part into. The messages those
    ///   leaves carry join the replica, which the budget never covers.
    /// - **Hashing.** The in-memory replica computes a node's hash over a
    ///   record of each of its children, and frees the record once the hash is
    ///   computed: at most one record per byte of a path at once.
    /// - **The connection's own state.** Sending, a session copies each message
    ///   smaller than about 63 KiB into a buffer of that size before writing it
    ///   to your write half; receiving, it reads a comparison's listing through
    ///   a small buffer. It also sets up a route for each of the streams its
    ///   messages travel on.
    ///
    /// # Messages on their way
    ///
    /// A session holds each message it sends until your write half takes it,
    /// and each message it receives until it has decoded it. A message that
    /// carries one comparison is a few kilobytes at most, and is counted
    /// against the budget with the work it belongs to. A message that carries
    /// supplied records holds up to
    /// [`target_message_size`](Self::target_message_size) bytes, unless one
    /// record alone outgrows the target, and nothing bounds how many a session
    /// sends, so every budget sets aside room for them by direction, however
    /// many streams of the connection they travel on: about two messages' worth
    /// at this peer's own target in each direction. A session builds such a
    /// message only once its room is free, and reads one from your read half
    /// only once its room is free; neither waits on anything but the
    /// connection, since the room a message holds returns as soon as the
    /// message is written or decoded. A message that one record makes larger
    /// than the room takes the whole room in its direction.
    ///
    /// # What the budget does not cover
    ///
    /// - **Messages larger than [`MAX_BUDGETED_MESSAGE_SIZE`]** at the default
    ///   target, about twice the target at others. While a session sends or
    ///   receives such a message, which only a single record larger than the
    ///   target makes, it holds the message's excess over that size beyond the
    ///   budget, for at most one message in each direction at once.
    /// - **Building a message of supplied records.** A session builds each such
    ///   message in a buffer that grows by doubling as records join it, up to
    ///   the message's room. Each time the buffer grows, the outgrown buffer
    ///   stays live until its contents move to the new one, so a message being
    ///   built can briefly hold up to just under its room again beyond the
    ///   budget.
    /// - **Reading the greeting and a hand-off.** A session reads the greeting
    ///   its peer opens with, which carries the peer's version, and, when a
    ///   peer joins or leaves the network through the session, the identity
    ///   handed over, into buffers that grow by doubling from 64 KiB as bytes
    ///   arrive, the outgrown buffer and its replacement briefly both live. The
    ///   greeting grows with the peer's version, which nothing caps.
    /// - **Honoring redactions in an answer.** When a session prunes part of
    ///   its replica to what the peer lacks, dropping what the peer has seen or
    ///   redacted, each level the pruning descends holds lists of that level's
    ///   children beyond the budget.
    /// - **The replica's growth.** Messages a session receives become part of
    ///   the replica, so after a session the replica can be larger than before
    ///   by more than the budget.
    /// - **Decoding received messages.** A session decodes each message it
    ///   receives into your payload type, to check it, and the decoded value
    ///   joins the replica with the message. What your type's deserialization
    ///   allocates along the way and frees before it returns, such as the
    ///   smaller buffers a large vector outgrows, is held beyond the budget,
    ///   for one message at a time.
    /// - **Data superseded during a session.** A session keeps the replica as
    ///   it was when the session began until the session ends. If the replica
    ///   changes meanwhile, through local sends or another session finishing,
    ///   the data those changes replace stays in memory until this session
    ///   ends. The same holds for the nodes the session itself rebuilds: the
    ///   new copies join the replica, and the originals they replace stay in
    ///   memory until the session ends.
    /// - **The session's tasks.** Each session sets up its tasks before it
    ///   compares anything: their own state, what your executor allocates to
    ///   run them, and the description of its replica's root it sends the peer,
    ///   which holds shared handles to the root's children rather than copies.
    ///   That is a fixed amount, whatever the budget and the sets' sizes. So is
    ///   what a session keeps for work waiting for room in the budget: a few
    ///   hundred bytes for each waiting part, and at most one part waits per
    ///   level of the tree, a few kilobytes in all.
    /// - **Your transport's own buffers,** such as socket buffers, which are
    ///   outside the library.
    /// - **Observers.** Backlogs of observers such as [`CausalMessages`] grow
    ///   with what you have not yet consumed, and while wire observers are
    ///   attached ([`observe`](Self::observe)), a session holds one extra copy
    ///   of each frame as it shows it to them.
    ///
    /// # Accuracy
    ///
    /// A session counts what it holds and sets aside for the budget by the
    /// in-memory size of each item it builds, and the count is exact for what
    /// it covers. The budget as a whole is approximate: it applies backpressure
    /// as a session's counted memory nears the budget, and it does not bound
    /// everything a session allocates, as [what the budget does not
    /// cover](#what-the-budget-does-not-cover) lists. Allocator rounding falls
    /// outside the count, and so does the state some platforms allocate for a
    /// lock the first time it is used, which an item's in-memory size does not
    /// include.
    #[must_use]
    pub fn sync_memory_budget(mut self, budget_bytes: usize) -> Self {
        self.settings.sizing.sync_memory_budget = MemoryBudget::Requested(budget_bytes);
        self
    }

    /// Add a wire observer to this peer's future sessions.
    ///
    /// For every session the peer enters — gossip, bootstrap serving, and
    /// retirement alike — each observer is asked for a per-session observer,
    /// which sees each directed stream's protocol messages as raw CBOR items.
    /// The full contract (the three handler levels, the ordering and
    /// back-pressure rules, what exactly is observed) is the
    /// [`observe`](crate::observe) module's.
    ///
    /// Observation never changes the wire: an observed session's bytes are
    /// identical to an unobserved one's. The choice follows the peer through
    /// [`into_rumors`](Self::into_rumors), cloning and reunion, bookmarking,
    /// and retirement; every [`Rumors`] clone shares all observers. Calling
    /// `observe` again adds another; callbacks visit them in registration
    /// order. To observe a joining peer's own bootstrap session, attach on the
    /// builder instead ([`Bootstrap::observe`]).
    #[must_use]
    pub fn observe(mut self, observer: Arc<dyn Observer>) -> Self {
        self.settings.observe.attach(observer);
        self
    }

    /// Bound the encoded size of the batched messages this peer sends.
    ///
    /// When the default protocol supplies a subtree the counterparty lacks, its
    /// leaves ship as *runs*: one wire message carrying a delimited sequence of
    /// leaf records. Batching is chunked by bytes: a run flushes once appending
    /// the next leaf would push the message's full encoded size, framing
    /// included, past `bytes`. Every run carries at least one leaf, so a
    /// message whose single leaf alone outgrows the target ships anyway and
    /// exceeds it. Runs never span reconciliation units: batching stops at each
    /// supplied subtree's last leaf.
    ///
    /// # Memory
    ///
    /// The target sizes the room a session sets aside, out of
    /// [`sync_memory_budget`](Self::sync_memory_budget), for the runs on their
    /// way through the connection: about two runs' worth at this peer's own
    /// target in each direction, as the budget's section on messages on their
    /// way describes. That room never takes more than the budget affords: a
    /// session runs at the largest target, at most yours, whose room fits the
    /// budget beside everything else it sets aside, a budget below
    /// [`MIN_SYNC_MEMORY_BUDGET`] counting as that minimum. Every target at or
    /// below the default fits, so a session lowers only a larger one, and never
    /// below the default. It advertises the target it runs at, which
    /// [`synchronization_settings`](Self::synchronization_settings) reports. A
    /// lowered target leaves the session's work only the room for its largest
    /// single piece beyond what every budget sets aside, so such a session
    /// keeps little in flight: raise the budget with the target. Each run holds
    /// at most the target's worth of bytes, framing aside, unless its one leaf
    /// alone outgrows the target; such a run takes its direction's whole room,
    /// and one larger than [`MAX_BUDGETED_MESSAGE_SIZE`] at the default target
    /// holds its excess beyond the budget while it crosses. A run whose frame
    /// (the run and a few bytes of framing) is smaller than the sending side's
    /// write buffer, about 63 KiB, is copied into it as it is written. The
    /// receiver decodes each run's encoded bytes as it goes, handing each leaf
    /// to the storage backend as it is read; a decoded leaf joins the replica,
    /// which the budget never covers.
    ///
    /// Each session runs at the **minimum** of the two ends' targets: the
    /// greeting carries each side's setting, each side's *encoder* batches
    /// within that minimum, and each side's decoder refuses a larger run unless
    /// it holds one leaf. Your setting thus bounds the frames you build and,
    /// through the minimum, the frames a conforming peer sends you, so the room
    /// your side sets aside at the target it advertises covers both directions:
    /// the more memory-constrained peer sets the pace. Peers with different
    /// settings interoperate.
    ///
    /// The default, [`default::TARGET_MESSAGE_SIZE`], is 256 KiB, the size by
    /// which batching a supplied subtree's records saves no more processor time
    /// in the workloads measured; its documentation gives the measurements. Any
    /// value is safe: zero degrades to one leaf per message, and values above
    /// the wire's run byte cap ([`MAX_RUN_BUDGET_BYTES`]) saturate to it, so a
    /// run built within the target always fits the cap.
    ///
    /// The choice follows the peer through [`into_rumors`](Self::into_rumors),
    /// cloning and reunion, bookmarking, and retirement.
    #[must_use]
    pub fn target_message_size(mut self, bytes: usize) -> Self {
        self.settings.sizing.run_budget = RunBudget::from_bytes(bytes);
        self
    }

    /// Choose how many bytes of messages each write to your write half carries,
    /// so the writes suit your transport.
    ///
    /// A session sends many small messages, and each write to your write half
    /// costs your transport something whatever its size: a system call on a
    /// socket, a record on a TLS stream. So a session gathers the messages it
    /// builds into *batches*, and writes and flushes each batch at once. The
    /// range's start is the *segment size*, the most a batch of small messages
    /// carries, so that each such write fits one network segment; its end is
    /// the *largest batch*, the most any batch carries.
    ///
    /// - A batch whose messages each fit the segment size stays within it: the
    ///   session flushes before a message that would push the batch past the
    ///   segment size, and as soon as the batch reaches it.
    /// - A message larger than the segment size spans segments however it is
    ///   written. The session flushes the batch before it, keeping that batch's
    ///   one-segment write, and the large message starts a batch that the
    ///   messages after it share, up to the largest batch.
    /// - While your write half is full, small writes buy no latency, since
    ///   bytes queue anyway. Once writing out a batch has had to wait for your
    ///   write half, each next batch grows up to the largest batch, until one
    ///   is written out without waiting.
    /// - While the session readies messages back to back, small writes buy no
    ///   latency either: each would only delay the messages queued behind it.
    ///   Once two batches in a row have been flushed at their ceiling, each
    ///   next batch grows up to the largest batch, until one ends because
    ///   nothing more was ready. A single burst a little over the segment size
    ///   keeps its one-segment writes.
    ///
    /// Either way the session also flushes whenever it has nothing more ready
    /// to send. It never waits for more messages to fill a batch, so both
    /// bounds are ceilings, not targets, and a message larger than its batch's
    /// ceiling is written and flushed alone.
    ///
    /// A peer that never calls this uses [`default::tls::WRITE_BATCH_RANGE`],
    /// from 1,426 to 16,384 bytes, because TLS over TCP is the common
    /// deployment. Over TLS 1.3 on a 1,500-byte Ethernet link, each batch of
    /// small messages that keeps to the segment size then leaves as one record
    /// that fills at most one TCP segment, and every other batch as one record
    /// of at most TLS's largest plaintext. [`default::tls`] gives the
    /// arithmetic and how to adjust both bounds for other links and transports.
    ///
    /// # How the bounds convert
    ///
    /// Inclusive and exclusive bounds both work: an excluded start of `s` bytes
    /// means `s + 1`, and an excluded end of `e` bytes means `e - 1`.
    ///
    /// - An unbounded start means a segment size of zero: every batch grows to
    ///   the largest batch.
    /// - A start above the end is lowered to the end.
    /// - An end above the session's write buffer, about 63 KiB, saturates to
    ///   it.
    /// - An end of zero, as in `..=0` or the empty `..0`, turns batching across
    ///   the session's work off. The session flushes whenever it finds no
    ///   message ready, without first letting its other work build more, so a
    ///   write carries only the messages ready together, and the write buffer
    ///   writes itself out whenever the next message does not fit beside the
    ///   buffered ones.
    /// - An unbounded end means no largest batch. A batch of small messages
    ///   still keeps to the segment size, but any other batch grows until the
    ///   session has nothing more ready to send, and the write buffer writes
    ///   itself out whenever it fills. You rarely want that: it suits a write
    ///   half that gains nothing from smaller writes and holds nothing back
    ///   until flushed, such as a raw socket carrying a bulk transfer. It gives
    ///   up the bound on how long a written message waits for its flush, which
    ///   then lasts as long as the session keeps readying messages, and a write
    ///   half that holds bytes until flushed, such as an encrypting or
    ///   compressing layer, holds them that long.
    ///
    /// The setting chooses only when the write buffer is flushed, never its
    /// size, so it changes no memory bound:
    /// [`sync_memory_budget`](Self::sync_memory_budget), its minimum, and the
    /// buffers beside it are the same whatever you set here. It is local to
    /// this peer and is not negotiated with the counterparty.
    ///
    /// The choice follows the peer through [`into_rumors`](Self::into_rumors),
    /// cloning and reunion, bookmarking, and retirement.
    #[must_use]
    pub fn write_batch_range(mut self, range: impl RangeBounds<usize>) -> Self {
        self.settings.sizing.write_batch = WriteBatch::from_range(range);
        self
    }

    /// Bound the nesting depth of the message payloads this peer sends and
    /// accepts.
    ///
    /// A payload value is accepted only if decoding its CBOR encoding as the
    /// peer's payload type recurses at most `limit` steps. What consumes one
    /// step is the decode engine's own accounting for that type — arrays, maps,
    /// and tags each do, and so can type-driven wrappers such as an enum's
    /// variant scope — so the bound is engine-defined, not a structural count
    /// of the bytes. The default, [`default::PAYLOAD_DEPTH_LIMIT`], is 256
    /// steps: exactly the bound the decoder applies by default, so a fleet at
    /// the default sees no acceptance change on existing content.
    ///
    /// Three points enforce the one bound:
    ///
    /// - **Send** ([`Rumors::send`](crate::Rumors::send),
    ///   [`Batch::send`](crate::Batch::send)): admission runs the exact decode
    ///   every receiver's wire ingress runs — same payload type, same limit,
    ///   same engine — so an over-deep value is rejected at its author, at the
    ///   moment of choice, with a typed [`EncodeError`].
    /// - **Handshake**: the greeting carries each side's configured limit, and
    ///   a session proceeds only if the two are exactly equal; a mismatch in
    ///   either direction aborts both sides with
    ///   [`Mismatch::PayloadDepth`](crate::error::Mismatch::PayloadDepth)
    ///   before anything else — the converged-session short-circuit included —
    ///   so a mixed configuration is caught at every pairing.
    /// - **Wire ingress**: every payload decode runs under this same limit, so
    ///   over-deep *content* supplied by a nonconforming implementation fails
    ///   its session with a typed decode error. The bound governs the decode's
    ///   recursion, not the bytes' shape: deep byte patterns the engine
    ///   consumes without recursing (a tag chain in a scalar position, say)
    ///   decode fine and are harmless.
    ///
    /// Together those establish the invariant this setting exists for: between
    /// conforming peers, no session can fail on payload depth at all — true by
    /// construction within a decode-engine version, because admission and
    /// ingress are one computation, not two accountings held in agreement.
    /// Over-deep values are rejected at their author, and mismatched fleets are
    /// rejected at the handshake. (A fleet mixing builds whose CBOR engine
    /// versions account recursion differently could still diverge; upgrading
    /// the engine is a fleet-coordination event in the same register as
    /// changing this limit.) The limit is a property of the *shared set* —
    /// every replica must be able to hold and forward all content — which is
    /// why the handshake demands equality rather than negotiating: a peer whose
    /// session bound dropped below its own configured limit could already hold
    /// messages deeper than the negotiated bound, which it would then not be
    /// allowed to gossip. Changing the limit is therefore a fleet-coordinated
    /// configuration event, like the [`Protocol`](crate::Protocol) deployed
    /// across the fleet, never a per-peer tuning parameter.
    ///
    /// The choice follows the peer through [`into_rumors`](Self::into_rumors),
    /// cloning and reunion, bookmarking, and retirement.
    #[must_use]
    pub fn payload_depth_limit(mut self, limit: u64) -> Self {
        self.codec = self.codec.with_limit(PayloadDepthLimit::new(limit));
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

    /// Create an empty batch against this replica.
    fn new_batch(&self) -> Batch<'_, T> {
        Batch::new(&self.inner, self.codec)
    }

    /// Commit one message and return its stamped version.
    pub(crate) fn send(&self, message: T) -> Result<Version, EncodeError> {
        let mut batch = self.new_batch();
        batch.send(message)?;
        Ok(batch.commit().latest)
    }

    /// Commit one redaction, returning whether it removed a message.
    pub(crate) fn redact(&self, version: &Version) -> bool {
        let mut batch = self.new_batch();
        batch.redact(version);
        batch.commit().redacted == 1
    }

    /// Commit the changes queued by a successful batch closure.
    pub(crate) fn batch<R, E, F>(&self, f: F) -> Result<Committed<R>, E>
    where
        F: for<'s> FnOnce(&'s mut Batch<'_, T>) -> Result<R, E>,
    {
        let mut batch = self.new_batch();
        let value = f(&mut batch)?;
        let redacted = batch.commit().redacted;
        Ok(Committed { value, redacted })
    }

    /// Commit every message from an iterator as one batch.
    pub(crate) fn send_all<I>(&self, messages: I) -> Result<(), EncodeError>
    where
        I: IntoIterator<Item = T>,
    {
        let mut batch = self.new_batch();
        batch.send_all(messages)?;
        batch.commit();
        Ok(())
    }

    /// Commit every redaction from an iterator as one batch, returning how many
    /// messages it removed.
    pub(crate) fn redact_all<I>(&self, versions: I) -> usize
    where
        I: IntoIterator,
        I::Item: Borrow<Version>,
    {
        let mut batch = self.new_batch();
        batch.redact_all(versions);
        batch.commit().redacted
    }

    /// Capture the replica's current immutable state.
    pub(crate) fn snapshot(&self) -> Snapshot<T> {
        Snapshot::new(self.inner.borrow().tree.clone())
    }

    /// Subscribe to every current and future live message in arbitrary order.
    pub(crate) fn unordered_messages(&self) -> UnorderedMessages<T> {
        self.unordered_messages_since(Version::new())
    }

    /// Subscribe in arbitrary order beyond a causal checkpoint.
    pub(crate) fn unordered_messages_since(&self, since: Version) -> UnorderedMessages<T> {
        UnorderedMessages::subscribe(&self.inner, since)
    }

    /// Subscribe to every current and future live message in causal order.
    pub(crate) fn causal_messages(&self) -> CausalMessages<T> {
        self.causal_messages_since(Version::new())
    }

    /// Subscribe in causal order beyond a causal checkpoint.
    pub(crate) fn causal_messages_since(&self, since: Version) -> CausalMessages<T> {
        CausalMessages::subscribe(&self.inner, since)
    }

    /// Subscribe to coalesced changes in the replica's causal frontier.
    pub(crate) fn changes(&self) -> Changes<T> {
        Changes::subscribe(&self.inner)
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
