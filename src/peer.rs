//! The local rumor set: [`Peer`] and its synchronized state, plus the local API
//! for sending, redacting, and observing messages. The wire-session drivers
//! (join, gossip, retire) live in [`gossip`].

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
use crate::message::{EncodeError, MessageDepthLimit, PayloadCodec};
use crate::network::{NetworkKeys, NetworkSecret};
use crate::observe::{Attachment, Observer};
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
    /// [`Joining::sync_memory_budget`], as given.
    pub(crate) sync_memory_budget: MemoryBudget,
    /// The supply-run byte budget selected by [`Peer::target_message_size`] or
    /// [`Joining::target_message_size`], as given.
    pub(crate) run_budget: RunBudget,
    /// The write batch range selected by [`Peer::write_batch_range`] or
    /// [`Joining::write_batch_range`].
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

/// The settings a peer's sessions run under, which a [`Joining`] builder
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
    /// The wire observers added through [`Peer::observe`] or
    /// [`Joining::observe`].
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
mod connection;
mod gossip;
mod join;
mod policy;

pub use bookmark::Unbookmarked;
pub use connection::Connection;
#[cfg(any(test, feature = "test-internals"))]
pub(crate) use gossip::COMPLETION_MARKER;
pub use gossip::{Gossip, Gossiped, Led, Retire};
pub use join::{Join, Joining};

/// The start and end of the lifecycle for [`Rumors`].
///
/// Create a new gossip network with [`Peer::seed`](Self::seed), or join an
/// established peer's network with [`Peer::joining`](Self::joining) ---
/// both result in a [`Peer`], which can be additionally configured before
/// proceeding to using it.
///
///  A [`Peer`] cannot be cloned and exists only while no [`Rumors`] handles to
/// that replica remain. This API design ensures that when a [`Peer`]
/// [`retire`](Self::retire)s, it cannot interrupt another handle's gossip
/// sessions.
///
/// To use and clone the replica, convert the [`Peer`] into a [`Rumors`] using
/// [`into_rumors`](Peer::into_rumors), then use the [`Rumors`] API to interact
/// with the message set and gossip with other peers.
///
/// When finished, convert any
/// [`Rumors`] clone back into a [`Peer`] using
/// [`try_into_peer`](crate::Rumors::try_into_peer), and optionally
/// [`retire`](Peer::retire) the [`Peer`] before stopping your application.
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
/// # let counterparty = Peer::<String>::seed().into_rumors();
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
/// let Join::Connected { peer, .. } = Peer::<String>::joining().join(read, write).await else {
///     panic!("the established counterparty must serve the joining party");
/// };
///
/// // `Peer: !Clone`, so we trade it for `Rumors` to send, listen, and gossip concurrently.
/// let rumors = peer.into_rumors();
/// let other = rumors.clone();
///
/// // ... use the `Rumors` handles concurrently however you want ...
///
/// // Once all but one `Rumors` has been dropped, a unique `Peer` can be reclaimed.
/// drop(other);
/// let Some(peer) = rumors.try_into_peer().await else {
///     unreachable!("all other handles were dropped already");
/// };
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
///     // We have now left the gossip network.
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
/// [`gossip`](crate::Rumors::gossip) with all others. Initially, peers will
/// each report [`Mismatch::Network`](crate::error::Mismatch::Network); whenever
/// a peer observes such an error, it can use a deterministic metric to decide
/// whether it or its peer should dominate.
///
/// A reasonable such metric is provided by metrics provided in the error
/// itself: compare its `local_min_events` against the `remote_min_events`. Let
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
    /// Carries the [`message_depth_limit`](Self::message_depth_limit) beside
    /// the codec's fn pointers (see [`PayloadCodec`]).
    pub(crate) codec: PayloadCodec,
}

/// The settings configured on a [`Peer`].
///
/// These are queryable via [`Peer::settings`] or
/// [`Rumors::settings`](crate::Rumors::settings).
///
/// Non-inspectable gossip policies set via [`gossip_when`](Peer::gossip_when)
/// and [`gossip_until`](Peer::gossip_until), and
/// [`gossip_timeout`](Peer::gossip_timeout) are not represented here.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    /// The configured [`Peer::sync_memory_budget`], adjusted for what is
    /// possible to achieve based on internal constraints.
    pub sync_memory_budget: usize,
    /// The configured [`Peer::target_message_size`].
    pub target_message_size: usize,
    /// The configured [`Peer::write_batch_range`], normalized to a pair of
    /// non-crossed bounds.
    pub write_batch_range: (Bound<usize>, Bound<usize>),
    /// The configured [`Peer::message_depth_limit`], required to match between
    /// any two peers which gossip.
    pub message_depth_limit: u64,
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
impl<T: Send + Sync + 'static, B: Bookmark> std::fmt::Debug for Peer<T, B> {
    /// Summarize the replica without printing its contents.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fmt_summary("Peer", f)
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
        // The secret comes from the operating system on every call, never
        // from a generator whose state lives in this process: a forked
        // child inherits that state, so siblings forked after one shared
        // draw would seed networks with one secret, one identifier and one
        // seed party, and gossip as one network.
        Self::seed_from_rng(&mut UnwrapErr(SysRng))
    }

    /// Create a new peer belonging to an existing gossip network by connecting
    /// to a peer who is already in it.
    ///
    /// To configure options prior to joining the network, use
    /// [`Peer::joining`].
    pub async fn join<R, W>(read: R, write: W) -> Join<T, R, W>
    where
        R: AsyncBufRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        Joining::new().join(read, write).await
    }

    /// Configure options for joining an existing gossip network before
    /// connecting to a peer who is already in it.
    pub fn joining() -> Joining<T> {
        Joining::new()
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
            codec: PayloadCodec::new::<T>(MessageDepthLimit::default()),
        }
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

/// Retire an exclusively held [`Peer`].
impl<T: Send + Sync + 'static, B: Bookmark> Peer<T, B> {
    /// Leave the gossip network after synchronizing one final time with another
    /// peer.
    ///
    /// While it is possible to simply drop a [`Peer`] with no special ceremony,
    /// explicitly retiring the peer helps keep the representation of
    /// [`Version`]s efficient as peers come and go.
    ///
    /// The outcome of this function, [`Retire`], reports whether this peer
    /// successfully left the network, or whether it remains (e.g. due to a
    /// failure which could be retried), and whether the connection can carry
    /// another session.
    pub async fn retire<R, W>(self, read: R, write: W) -> Retire<T, R, W, B>
    where
        R: AsyncBufRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        self.retire_inner(read, write).await
    }
}

/// Inspect and configure settings for gossip.
impl<T: Send + Sync + 'static, B: Bookmark> Peer<T, B> {
    /// Return the gossip network to which this replica belongs.
    pub fn network(&self) -> Network {
        self.keys.network()
    }

    /// Return the local settings used to configure gossip sessions.
    ///
    /// This excludes the gossip initiation and termination policies defined by
    /// [`gossip_when`](Peer::gossip_when) [`gossip_until`](Peer::gossip_until),
    /// and [`gossip_timeout`](Peer::gossip_timeout), which are specified as
    /// async closures and therefore cannot be meaningfully inspected.
    pub fn settings(&self) -> Settings {
        let sizing = self.settings.sizing;
        let limits = sizing.limits();
        Settings {
            sync_memory_budget: limits.budget.memory,
            target_message_size: limits.target.bytes(),
            write_batch_range: sizing.write_batch.range(),
            message_depth_limit: self.codec.limit().get(),
        }
    }

    /// Format the bounded summary shared by [`Peer`] and [`Rumors`].
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

    /// Set the policy for when each [`Rumors`] will initiate
    /// gossip with a peer during [`Rumors::gossip`](crate::Rumors::gossip) or
    /// [`Rumors::gossip_stream`](crate::Rumors::gossip_stream).
    ///
    /// By default, gossip will initiate immediately any time the local set of
    /// messages changes.
    ///
    /// You might want to alter this policy if you want to debounce or
    /// rate-limit gossip, or gossip only on an interval or in response to some
    /// external, application-specified event.
    ///
    /// To set this policy, provide a function `when`, which accepts a stream of
    /// [`Changes`] and returns a stream of [`Gossip`]. Your
    /// function can adapt that stream, combine it with another, or ignore it
    /// entirely and provide its own policy.
    ///
    /// If you return [`Gossip::WhenChanged`] from
    /// your stream, gossip will only happen if there have been changes since
    /// the last gossip round with this peer; if you return
    /// [`Gossip::Unconditionally`], gossip will
    /// happen regardless of local changes. Note that `(): Into<Gossip>` where
    /// `().into() == Gossip::WhenChanged`, so an ordinary stream of `()` acts
    /// as a sequence of "ticks" which initiate gossip only if there are known
    /// changes to broadcast.
    ///
    /// Two extremes to be aware of:
    ///
    /// - A stream which always pends and never yields an item will only gossip
    ///   if the remote side initiates; if everyone has this policy, nobody will
    ///   ever gossip.
    /// - A stream which immediately yields every time it is polled will cause a
    ///   busy-loop. Don't do this!
    ///
    /// When the stream of [`Gossip`] ends, `rumors` will stop
    /// each gossip connection cleanly after any active session. That said, to
    /// end every driver's policy on a shared signal, use
    /// [`gossip_until`](Self::gossip_until) rather than building the signal
    /// into the factory.
    pub fn gossip_when<F, S>(mut self, when: F) -> Self
    where
        F: Fn(crate::Changes<T>) -> S + Send + Sync + 'static,
        S: futures::Stream + Send + 'static,
        S::Item: Into<crate::Gossip>,
    {
        self.settings.gossip_policy.set_when(when);
        self
    }

    /// Graceful shutdown: end every gossip driver's initiation policy once
    /// `stop` resolves.
    ///
    /// [`Rumors::try_into_peer`] waits for every other handle to be dropped, so
    /// invoking `stop` allows it to resolve and collect all the disparate
    /// gossiping clones back into a single [`Peer`].
    ///
    /// Once `stop` has resolved, it stays in effect: a driver started later
    /// ends at once. Replace it (e.g. with [`std::future::pending`] to reset
    /// the policy entirely) before restarting gossip.
    pub fn gossip_until<F>(mut self, stop: F) -> Self
    where
        F: Future + Send + 'static,
    {
        self.settings.gossip_policy.set_until(stop);
        self
    }

    /// Impose a timeout on the completion of each round of gossip. This is
    /// disabled by default.
    ///
    /// Because Rumors is runtime-independent, the application owns whatever
    /// clock is used for the deadline.
    ///
    /// For example, a one-second deadline with Tokio could be configured by
    /// passing the gossip timeout of:
    /// `|| tokio::time::sleep(Duration::from_secs(1))`.
    ///
    /// Before [`joining`](Peer::joining), configure
    /// [`Joining::gossip_timeout`]; it will apply to the initial joining
    /// session, and the returned [`Peer`] will inherit it henceforth.
    pub fn gossip_timeout<D, F>(mut self, timeout: D) -> Self
    where
        D: Fn() -> F + Send + Sync + 'static,
        F: Future<Output = ()> + Send + 'static,
    {
        self.settings.gossip_policy.set_deadline(timeout);
        self
    }

    /// Impose a maximum size limit on a stored [`Bookmark`].
    ///
    /// The default is [`default::BOOKMARK_SIZE_LIMIT`].
    ///
    /// Updates to a bookmark discard the oldest and least recently-used
    /// information once the size of the bookmark hits this capacity.
    ///
    ///
    /// Use [`Joining::bookmark_size_limit`] to configure this setting
    /// explicitly before joining a gossip network.
    pub fn bookmark_size_limit(mut self, bytes: usize) -> Self {
        self.bookmark.set_size_limit(bytes);
        self
    }

    /// Constrain the memory budget for each synchronization session.
    ///
    /// The default is [`default::SYNC_MEMORY_BUDGET`] (512 MiB), and the
    /// minimum is about 36.4 MiB.
    ///
    /// This setting is preserved indefinitely throughout the lifecycle of the
    /// [`Peer`], and all [`Rumors`] clones inherit it.
    ///
    /// For more guidance on how to configure this parameter, read the [guide to
    /// picking a memory budget](crate::sizing).
    #[must_use]
    pub fn sync_memory_budget(mut self, budget_bytes: usize) -> Self {
        self.settings.sizing.sync_memory_budget = MemoryBudget::Requested(budget_bytes);
        self
    }

    /// Add a wire [`Observer`] to this peer's gossip
    /// sessions.
    ///
    /// For every session the peer enters, each [`Observer`] is given the
    /// capability to see each directed stream's protocol messages as raw CBOR
    /// items. This can be useful to debug, test, or instrument the `rumors`
    /// protocol itself.
    ///
    /// Calling `observe` again adds another, and observers are invoked in
    /// registration order. To observe the session in which a peer joins,
    /// attach it instead using [`Joining::observe`].
    #[must_use]
    pub fn observe(mut self, observer: Arc<dyn Observer>) -> Self {
        self.settings.observe.attach(observer);
        self
    }

    /// Tune the target encoded size of the batched messages this peer sends.
    ///
    /// When a peer supplies messages which its counterparty lacks, those
    /// messages are coalesced into batches of records which are flushed if
    /// appending the next message would push the batch's size past the
    /// specified target `bytes`.
    ///
    /// Adjusting this setting can subtly trade between latency, CPU
    /// utilization, and memory. It is recommended to stick with the
    /// default, [`default::TARGET_MESSAGE_SIZE`], unless there is a compelling
    /// reason to differ. Any value is safe: zero degrades to one leaf per
    /// message, and any limit above the maximum possible ~4 GiB saturates.
    ///
    /// If two peers configure this differently, they interoperate by
    /// negotiating the minimum of their two target message sizes, so that the
    /// lesser-configured peer receives batches sized according to its own
    /// desires.
    ///
    /// This setting is preserved indefinitely throughout the lifecycle of the
    /// Peer, and all Rumors clones inherit it.
    #[must_use]
    pub fn target_message_size(mut self, bytes: usize) -> Self {
        self.settings.sizing.run_budget = RunBudget::from_bytes(bytes);
        self
    }

    /// Tune write-batching to optimize for your underlying transport.
    ///
    /// This range should usually run from the "largest smallest frame" to the
    /// "largest single frame" your transport allows, which economizes on system
    /// calls and optimizes the fullness of each individual transport frame.
    ///
    /// The default, [`default::tls::WRITE_BATCH_RANGE`], is tuned for TLS over
    /// TCP on a 1,500-byte MTU Ethernet link.
    ///
    /// This setting is preserved indefinitely throughout the lifecycle of the
    /// [`Peer`], and all [`Rumors`] clones inherit it.
    ///
    /// # When and how to change this
    ///
    /// Keeping the default will never cause correctness issues, but in other
    /// contexts, you may see improved performance by changing it.
    ///
    /// By way of example:
    ///
    /// - **Plain TCP** does not have a separate record framing layer.
    ///   Consequently, the write batch range should start at the TCP segment
    ///   payload size, 1,448 bytes, and can end wherever you choose. No record
    ///   limit applies in this setting, so a larger maximum will make fewer
    ///   writes, but may increase latency and memory usage if set exceptionally
    ///   high.
    /// - **IPv6** headers are 20 bytes longer than IPv4's, so subtract 20 bytes
    ///   from the minimum of the range, giving 1,406 bytes as an optimal
    ///   minimum for gossip over TLS.
    ///
    /// - **Larger MTUs**, such as 9,000-byte jumbo frames, should raise the
    ///   minimum by the same amount, for example 8,926 bytes as an optimal
    ///   minimum for gossip over TLS.
    ///
    /// # Settings to (probably) avoid
    ///
    /// - A crossed range like `hi..low` is corrected internally to `low..low`.
    ///   You're probably doing something wrong if you do this.
    /// - An unbounded-below range like `..max` or `0..max` means a "largest
    ///   smallest frame" of zero. This means every batch will try to grow to
    ///   the maximum size, rather than dispatching small batches quickly. This
    ///   usually harms performance!
    /// - A maximum set to zero, as in `..=0` or the empty `..0`, turns batching
    ///   off entirely. Sessions will flush immediately when they find no
    ///   message ready. This usually harms performance!
    /// - An unbounded maximum, or a maximum set above the internal write buffer
    ///   size, saturates at this inner capacity. A batch of small messages
    ///   still will flush when it reaches the minimum size, but any batch that
    ///   accumulates past this will continue to grow until the session has
    ///   nothing more ready to send, only flushing when the internal write
    ///   buffer fills. This usually harms performance!
    #[must_use]
    pub fn write_batch_range(mut self, range: impl RangeBounds<usize>) -> Self {
        self.settings.sizing.write_batch = WriteBatch::from_range(range);
        self
    }

    /// Bound the depth of the messages this peer sends and accepts.
    ///
    /// A message value is accepted only if decoding its CBOR encoding as the
    /// peer's message type recurs at most `limit` steps. The meaning of such a
    /// "recursion step" is internal to `rumors`, but is guaranteed to remain
    /// stable within a single wire protocol.
    ///
    /// Applying such a limit prevents a peer from "poisoning" the gossip set
    /// with an excessively nested encoded message, which otherwise could
    /// trigger a stack overflow on receipt by other peers.
    ///
    /// This limit is checked by peers both prior to
    /// [`send`](crate::Rumors::send), and also during message decoding, so that
    /// a conforming peer cannot even send a too-deep message, and a
    /// non-conforming peer which bypasses the internal check will succeed only
    /// at isolating itself from gossip, since everyone else will reject its bad
    /// message.
    ///
    /// The default is [`default::MESSAGE_DEPTH_LIMIT`], which should be ample
    /// for almost any type.
    ///
    /// This setting is preserved indefinitely throughout the lifecycle of the
    /// [`Peer`], and all [`Rumors`] clones inherit it.
    ///
    /// <div class="warning">
    ///
    /// **This setting must be *exactly the same* for all peers in the gossip
    /// network.** Peers will refuse to gossip with one another if they are
    /// configured with different message depth limits, *even if they have
    /// ostensibly compatible sets of messages.*
    ///
    /// </div>
    #[must_use]
    pub fn message_depth_limit(mut self, limit: u64) -> Self {
        self.codec = self.codec.with_limit(MessageDepthLimit::new(limit));
        self
    }

    /// Convert the [`Peer`] into a [`Rumors`] so it can [`send`](Rumors::send),
    /// [`redact`](Rumors::redact), and [`gossip`](Rumors::gossip) with other
    /// peers.
    ///
    /// Unlike [`Peer`], [`Rumors`] is [`Clone`], so that you can
    /// [`gossip`](crate::Rumors::gossip) with many other peers concurrently.
    /// Once a single [`Rumors`] handle remains,
    /// [`try_into_peer`](Rumors::try_into_peer) will convert it back into the
    /// original [`Peer`].
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
