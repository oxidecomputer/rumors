//! The local set of messages: [`Peer`] and its synchronized state, plus the
//! local API for sending, redacting, and observing messages. The wire-session
//! drivers (join, gossip, retire) live in [`gossip`].

use std::{
    borrow::Borrow,
    ops::Bound,
    sync::{Arc, PoisonError, RwLock},
};

use before::Party;
use tokio::io::{AsyncBufRead, AsyncWrite};
use tokio::sync::watch;

use crate::bookmark::{
    BookmarkHandle, MaybeBookmark, NoBookmark, NullBookmark, Reservation, Reservations,
};
use crate::counter::widen;
use crate::default;
use crate::message::{EncodeError, MessageDepthLimit, PayloadCodec};
use crate::network::{NetworkKeys, NetworkSecret};
use crate::observe::Attachment;
use crate::tree::mirror::streaming::remote::{RunBudget, WriteBatch};
use crate::tree::mirror::streaming::{
    Local,
    materialized::{self, SessionBudget},
};
use crate::tree::{self, Tree};
use crate::{Batch, Committed, Network, Snapshot, Version};

use serde::{Serialize, de::DeserializeOwned};

/// The minimum memory budget at the default message-size target
/// ([`default::TARGET_MESSAGE_SIZE`]): about 36.4 MiB on 64-bit targets.
///
/// `MemoryBudget::session` clamps the target to what the larger of the
/// requested budget and this affords, so a session counts at most that larger
/// value, whatever the target. The public docs state the figure in prose, and
/// `the_minimum_budget_is_the_documented_figure` checks it; an application
/// reads the exact minimum for its target from [`Peer::settings`] at a budget
/// of zero.
pub const MIN_SYNC_MEMORY_BUDGET: usize =
    materialized::Allowances::of::<Local>(default::TARGET_MESSAGE_SIZE).minimum();

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
    /// ([`ledger_budget`](crate::PeerBuilder::ledger_budget)): sessions run on the
    /// memory budget that leaves their ledgers exactly this, at whatever
    /// target, below the minimum included.
    #[cfg(any(test, feature = "test-internals"))]
    Ledger(usize),
    /// The serial configuration the test path sets
    /// ([`serial_budget`](crate::PeerBuilder::serial_budget)): one unit of each
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
    /// The memory budget selected by [`PeerBuilder::sync_memory_budget`], as given.
    pub(crate) sync_memory_budget: MemoryBudget,
    /// The supply-run byte budget selected by [`PeerBuilder::target_message_size`], as given.
    pub(crate) run_budget: RunBudget,
    /// The write batch range selected by [`PeerBuilder::write_batch_range`].
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

/// The settings a peer's sessions run under, which a [`PeerBuilder`]
/// selects and the peer it joins inherits whole.
///
/// The message nesting limit is not among them: a peer holds it inside its
/// payload codec, which only a peer's construction can build, while a builder
/// holds the limit alone.
pub(crate) struct SessionSettings<T: Send + Sync + 'static> {
    /// How the sessions size their memory and their writes.
    pub(crate) sizing: SessionSizing,
    /// Initiation and deadline factories inherited by every gossip handle.
    pub(crate) gossip_policy: policy::Policy<T>,
    /// The wire observers added through [`PeerBuilder::observe`].
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
mod builder;
mod connection;
mod gossip;
mod join;
mod messages;
mod policy;
mod reunion;

pub use bookmark::Unbookmarked;
pub use builder::PeerBuilder;
pub use connection::Connection;
#[cfg(any(test, feature = "test-internals"))]
pub(crate) use gossip::COMPLETION_MARKER;
pub use gossip::{Gossip, Gossiped, Participant, Retire};
pub use join::Join;
pub use messages::{CausalMessages, Changes, TryNext, TryTick, UnorderedMessages};
use reunion::Reunion;

/// A handle for [`send`](Peer::send)ing and [`redact`](Peer::redact)ing
/// messages, and [`gossip`](Peer::gossip)ing the results with peers.
///
/// [`Peer`] is [`Clone`]: any number of tasks may interact
/// with clones concurrently. All synchronization is internal and automatic:
/// anything one [`Peer`] learns, all clones of it do.
///
/// Create a new gossip network with [`Peer::seed`](Self::seed), or join an
/// established peer's network with [`Peer::join`](Self::join) ---
/// both result in a [`Peer`].
///
/// [`Peer::retire`](Self::retire) waits until every other handle to that
/// replica has been dropped or is also retiring. This API design ensures that
/// when a [`Peer`] [`retire`](Self::retire)s, it cannot interrupt another
/// handle's gossip sessions.
///
/// When finished, it is preferable to [`retire`](Peer::retire) the [`Peer`]
/// before stopping your application. Doing so recycles "identity space" and
/// keeps the space consumed by [`Version`](crate::Version)s smaller.
///
/// # Example
///
/// The lifecycle of a [`Peer`] usually looks something like this:
///
/// ```
/// use rumors::{Connection, Join, Peer, Retire};
/// use tokio::io::{BufReader, duplex, split};
///
/// # tokio::runtime::Builder::new_current_thread()
/// #     .build()
/// #     .unwrap()
/// #     .block_on(async {
/// // To connect to a peer, we pass the read/write halves of one connection.
/// // In this example, we assume we've already set up a connection to some remote peer.
/// # let counterparty = Peer::<String>::seed();
/// # let (connection, far) = duplex(64 * 1024);
/// let (read, write) = split(connection);
/// let read = BufReader::new(read);
/// # let serve = counterparty.clone();
/// # tokio::spawn(async move {
/// #     let (far_read, far_write) = tokio::io::split(far);
/// #     let far_read = tokio::io::BufReader::new(far_read);
/// #     serve.gossip_once(far_read, far_write).await.unwrap();
/// # });
///
/// // Join the network through an existing peer.
/// let Join::Connected { peer, .. } = Peer::<String>::join(read, write).await else {
///     panic!("the established counterparty must serve the joining party");
/// };
///
/// // Clones of the `Peer` send, listen, and gossip concurrently.
/// let other = peer.clone();
///
/// // ... use the handles concurrently however you want ...
///
/// // Retirement waits until every other handle has been dropped.
/// drop(other);
///
/// // Leave the network by retiring into any peer; it need not be the one we joined.
/// // Here, we pretend we have another connection to another peer already in hand.
/// # let (connection, far) = duplex(64 * 1024);
/// let (read, write) = split(connection);
/// let read = BufReader::new(read);
///
/// # tokio::spawn(async move {
/// #     let (far_read, far_write) = split(far);
/// #     let far_read = BufReader::new(far_read);
/// #     counterparty.gossip_once(far_read, far_write).await.unwrap();
/// # });
/// let retry = match peer.retire(read, write).await {
///     // We have now retired from the gossip network.
///     Retire::Retired { connection } => {
///         if let Connection::Lost { error } = connection {
///             eprintln!("error after retiring: {error}");
///         }
///         None
///     }
///     // The remote peer was also retiring; retry retiring into a different peer.
///     Retire::Declined { peer, .. } => Some(peer),
///     // Retirement failed part-way through; retry using a fresh connection, or another peer.
///     Retire::Recovered { peer, error: _ } => Some(peer),
/// };
/// # assert!(retry.is_none(), "the example's retirement succeeds");
/// # });
/// ```
///
/// # Bootstrapping a network without consensus
///
/// If your application admits a distinguished "first peer" (for example, via
/// leader election), have that peer call [`Peer::seed`].
///
/// Absent any true consensus on who should create the gossip network, another
/// reasonable approach to bootstrapping a [`Network`] is for *every* [`Peer`]
/// to initially call [`Peer::seed`] and attempt to
/// [`gossip`](Peer::gossip) with all others. Initially, peers will
/// each report [`Mismatch::Network`](crate::error::Mismatch::Network); whenever
/// a peer observes such an error, it can use a deterministic metric to decide
/// whether it or its peer should dominate.
///
/// A reasonable such metric is provided by metrics provided in the error
/// itself: compare its local
/// [`min_events`](crate::error::NetworkPrecedence::min_events) against the
/// remote one. Let
/// the greater count win, with ties broken by comparing the two [`Network`]
/// identifiers. The winner then stays in its network, and the loser drops its
/// [`Peer`] and attempts to re-[`join`](Peer::join) from the winning peer.
///
/// If peers are reasonably well-connected as the network gets started, this
/// protocol quickly reaches a stable steady state, which is henceforth
/// disrupted only if a group of new peers spends a long time partitioned before
/// uniting with the rest of the network.
///
/// A tweak to this scheme can guarantee the absence of data loss, at the cost
/// of network coherence: only allow the greater-count peer to "win" if the
/// losing peer has *zero* local events. This alternative convergence rule will
/// result in a gossip network fragmented into "largest coherent partitions".
/// Which version of this rule to pick (or whether some other convergence
/// mechanism is desirable) is a decision `rumors` doesn't try to make for
/// you --- your own application and its constraints should dictate the
/// trade-offs here.
pub struct Peer<T: Send + Sync + 'static, B: MaybeBookmark = NoBookmark> {
    /// A peer view sharing this replica's state and configuration.
    ///
    /// CAUTION: LOAD-BEARING DROP ORDER: This is declared before `reunion`, so
    /// that dropping a handle drops `replica` before its `alive` sender, as
    /// [`Reunion`] requires.
    pub(crate) replica: Replica<T, B>,
    /// Tracks live handles and holds the view that reunion hands back.
    reunion: Reunion<T, B>,
}

/// A peer view sharing this replica's state and configuration.
pub struct Replica<T: Send + Sync + 'static, B: MaybeBookmark = NoBookmark> {
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
    /// behind an async mutex and shared with every [`Peer`] clone, beside the
    /// size limit selected by [`bookmark_size_limit`].
    ///
    /// Separate from `inner` because persisting is `async` and the record is
    /// `!Clone`; see [`BookmarkHandle`].
    ///
    /// [`bookmark_size_limit`]: PeerBuilder::bookmark_size_limit
    pub(crate) bookmark: BookmarkHandle<B::Storage>,
    /// The payload codec built at construction: the typed ingress every gossip
    /// session's supplied leaf records decode through.
    ///
    /// Carries the [`message_depth_limit`](PeerBuilder::message_depth_limit) beside
    /// the codec's fn pointers (see [`PayloadCodec`]).
    pub(crate) codec: PayloadCodec,
}

/// The settings configured on a [`Peer`].
///
/// These are queryable via [`Peer::settings`].
///
/// Non-inspectable gossip policies set via [`gossip_when`](PeerBuilder::gossip_when)
/// and [`gossip_until`](PeerBuilder::gossip_until), and
/// [`gossip_timeout`](PeerBuilder::gossip_timeout) are not represented here.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    /// The configured [`PeerBuilder::sync_memory_budget`], adjusted for what is
    /// possible to achieve based on internal constraints.
    pub sync_memory_budget: usize,
    /// The configured [`PeerBuilder::target_message_size`].
    pub target_message_size: usize,
    /// The configured [`PeerBuilder::write_batch_range`], normalized to a pair of
    /// non-crossed bounds.
    pub write_batch_range: (Bound<usize>, Bound<usize>),
    /// The configured [`PeerBuilder::message_depth_limit`], required to match between
    /// any two peers which gossip.
    pub message_depth_limit: u64,
}

/// Compute the settings a [`Peer`] or a [`PeerBuilder`] reports.
impl Settings {
    /// The effective settings for `sizing` and `message_depth_limit`: the
    /// memory budget raised to the minimum its target needs, and the target
    /// as the budget clamps it.
    pub(crate) fn effective(sizing: SessionSizing, message_depth_limit: MessageDepthLimit) -> Self {
        let limits = sizing.limits();
        Self {
            sync_memory_budget: limits.budget.memory,
            target_message_size: limits.target.bytes(),
            write_batch_range: sizing.write_batch.range(),
            message_depth_limit: message_depth_limit.get(),
        }
    }
}

/// The replica's identity and content, shared through a watch channel.
///
/// Retirement owns the consumed `Peer`, excluding other writable handles.
pub(crate) struct Inner<T: Send + Sync + 'static> {
    /// Identity used to stamp local events.
    pub(crate) party: Party,
    /// Forks reserved for donations to joining counterparties, not yet
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

    /// Split off a fork to donate to a joining counterparty; reclamation
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

/// A bounded summary, independent of `T: Debug`.
impl<T: Send + Sync + 'static, B: MaybeBookmark> std::fmt::Debug for Replica<T, B> {
    /// Summarize the replica without printing its contents.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fmt_summary("Peer", f)
    }
}

/// A summary, independent of `T: Debug`.
impl<T: Send + Sync + 'static, B: MaybeBookmark> std::fmt::Debug for Peer<T, B> {
    /// Debug the replica without printing payloads.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.replica.fmt_summary("Peer", f)
    }
}

impl<T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static> Peer<T, NoBookmark> {
    /// Create a new peer belonging to a fresh gossip network which initially
    /// contains only that peer.
    ///
    /// Independently seeded peers cannot ever gossip with each other, so you
    /// must call this function *once per network*. Subsequently, other peers
    /// should join the same gossip network through [`Peer::join`].
    ///
    /// # Panics
    ///
    /// Panics if the operating system's random number generator fails.
    pub fn seed() -> Self {
        PeerBuilder::new().seed()
    }

    /// Create a new peer belonging to an existing gossip network by connecting
    /// to a peer who is already in it.
    ///
    /// To configure options prior to joining the network, use
    /// [`Peer::builder`].
    pub async fn join<R, W>(read: R, write: W) -> Join<T, R, W>
    where
        R: AsyncBufRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        PeerBuilder::new().join(read, write).await
    }

    /// Configure options before seeding a gossip network or joining one.
    pub fn builder() -> PeerBuilder<T> {
        PeerBuilder::new()
    }
}

/// Seed the internal view a new handle wraps.
impl<T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static> Replica<T, NoBookmark> {
    /// Create a new network from `secret`, carrying the builder's settings.
    fn seeded(builder: PeerBuilder<T>, secret: NetworkSecret) -> Self {
        let keys = Arc::new(NetworkKeys::new(secret));
        let bookmark = BookmarkHandle::new(NullBookmark, builder.bookmark_size_limit);
        Self {
            keys: Arc::clone(&keys),
            settings: builder.settings,
            inner: watch::Sender::new(Inner::new(Party::seed(), Tree::new(keys))),
            bookmark,
            codec: PayloadCodec::new::<T>(builder.message_depth_limit),
        }
    }
}

/// Share peer internals when cloning a [`Peer`] handle, and repair what
/// leaked sessions left behind when reclaiming the peer.
impl<T: Send + Sync + 'static, B: MaybeBookmark> Replica<T, B> {
    /// Share this peer's replica, configuration, and bookmark with a new
    /// handle.
    ///
    /// Only [`Peer::clone`] uses this internal operation.
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

impl<T: Send + Sync + 'static, B: MaybeBookmark> Peer<T, B> {
    /// Return the globally unique identifier of the gossip network to which
    /// this replica belongs.
    pub fn network(&self) -> Network {
        self.replica.network()
    }

    /// Return the local settings used to configure gossip sessions.
    ///
    /// This excludes the gossip initiation and termination policies defined by
    /// [`gossip_when`](crate::PeerBuilder::gossip_when) [`gossip_until`](crate::PeerBuilder::gossip_until),
    /// and [`gossip_timeout`](crate::PeerBuilder::gossip_timeout), which are specified as
    /// async closures and therefore cannot be meaningfully inspected.
    pub fn settings(&self) -> Settings {
        self.replica.settings()
    }

    /// Alias the live identity for invariant assertions in tests.
    ///
    /// Compare it or use it in an accounting fold; never tick with it. An alias
    /// shares the original identity's space and cannot act as another peer.
    #[cfg(any(test, feature = "test-internals"))]
    #[doc(hidden)]
    pub fn dangerously_alias_party(&self) -> before::Party {
        self.replica.dangerously_alias_party()
    }
}

/// Inspect settings for gossip.
impl<T: Send + Sync + 'static, B: MaybeBookmark> Replica<T, B> {
    /// Return the gossip network to which this replica belongs.
    pub(crate) fn network(&self) -> Network {
        self.keys.network()
    }

    /// Return the local settings used to configure the underlying
    /// [`Peer`].
    pub(crate) fn settings(&self) -> Settings {
        Settings::effective(self.settings.sizing, self.codec.limit())
    }

    /// Format the bounded summary of a [`Peer`].
    pub(crate) fn fmt_summary(
        &self,
        name: &str,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let inner = self.inner.borrow();
        let settings = self.settings();
        f.debug_struct(name)
            .field("network", &self.network())
            .field("latest", inner.tree.latest())
            .field("len", &inner.tree.len())
            .field("sync_memory_budget", &settings.sync_memory_budget)
            .field("target_message_size", &settings.target_message_size)
            .field("write_batch_range", &settings.write_batch_range)
            .field("message_depth_limit", &settings.message_depth_limit)
            .finish_non_exhaustive()
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
        let commit = batch.commit();
        Ok(Committed {
            value,
            sent: commit.sent,
            redacted: commit.redacted,
            latest: commit.latest,
        })
    }

    /// Commit every message from an iterator as one batch, returning the
    /// replica's frontier right after the commit if the batch sent anything.
    pub(crate) fn send_all<I>(&self, messages: I) -> Result<Option<Version>, EncodeError>
    where
        I: IntoIterator<Item = T>,
    {
        let mut batch = self.new_batch();
        batch.send_all(messages)?;
        let commit = batch.commit();
        Ok((commit.sent > 0).then_some(commit.latest))
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

    /// Alias this set's live party for invariant assertions in tests; see
    /// [`Peer::dangerously_alias_party`] for what the caller must uphold.
    #[cfg(any(test, feature = "test-internals"))]
    pub(crate) fn dangerously_alias_party(&self) -> Party {
        self.inner.borrow().party.dangerously_alias()
    }
}
