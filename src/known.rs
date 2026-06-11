//! The local rumor set: [`Known`] and its synchronized state, plus the local
//! API for sending, redacting, and observing messages. The wire-session
//! drivers (bootstrap, gossip, retire) live in [`gossip`].

use std::sync::Arc;

use before::Party;
use borsh::BorshSerialize;
use rand::{RngCore, rngs::OsRng};
use tokio::sync::watch;

use crate::tree::Tree;
use crate::{Batch, Broadcast, CausalMessages, Key, Messages, Network, Snapshot, Version};

mod gossip;

pub use gossip::{PROTOCOL_MAGIC, PROTOCOL_VERSION, Retire};

/// A local set of rumors.
pub struct Known<T> {
    pub(crate) network: Network,
    pub(crate) inner: watch::Sender<Inner<T>>,
}

pub(crate) struct Inner<T> {
    pub(crate) party: Option<Party>,
    pub(crate) tree: Tree<T>,
}

/// A summary view (network, latest version, live-message count), independent
/// of `T: Debug`: the messages themselves are not printed.
impl<T> std::fmt::Debug for Known<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner.borrow();
        f.debug_struct("Known")
            .field("network", &self.network)
            .field("latest", inner.tree.latest())
            .field("len", &inner.tree.len())
            .finish_non_exhaustive()
    }
}

impl<T> Known<T> {
    /// Create the distinguished seed rumor set: the single root from which
    /// every other participant must [`bootstrap`](Known::bootstrap).
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
            inner: watch::Sender::new(Inner {
                party: Some(Party::seed()),
                tree: Tree::new(),
            }),
        }
    }

    /// Send a message to all listeners.
    ///
    /// Returns a [`Batch`] that commits when dropped: a bare
    /// `known.send(message);` commits at the end of the statement, and
    /// chaining further [`send`](Batch::send)s and
    /// [`redact`](Batch::redact)s accumulates them into one commit. Building
    /// holds no lock; see [`batch`](Self::batch).
    pub fn send(&self, message: T) -> Batch<'_, T>
    where
        T: BorshSerialize + Send + Sync,
    {
        let mut batch = self.batch();
        batch.send(message);
        batch
    }

    /// Redact a message for all listeners: it is contagiously purged from
    /// the [`Known`] set for all peers who gossip with us, and will be
    /// unobserved by any future peers who did not already observe it.
    ///
    /// Returns a [`Batch`] that commits when dropped, exactly as
    /// [`send`](Self::send) does.
    pub fn redact(&self, key: Key) -> Batch<'_, T>
    where
        T: Send + Sync,
    {
        let mut batch = self.batch();
        batch.redact(key);
        batch
    }

    /// Start an empty [`Batch`] of insertions and redactions, committed
    /// atomically when dropped: one tree traversal, one change notification.
    /// Building holds no lock — the rumor set is locked only inside the
    /// commit.
    pub fn batch(&self) -> Batch<'_, T>
    where
        T: Send + Sync,
    {
        Batch::new(&self.inner)
    }

    /// Trade this [`Known`] for a [`Clone`]-able [`Broadcast`] handle which
    /// can be used concurrently.
    ///
    /// While any [`Broadcast`] handle exists for this set, the underlying
    /// [`Known`] is inaccessible, because retirement cannot happen concurrent
    /// to gossip sessions: a usable `Known` and a `Broadcast` for the same
    /// set never coexist. The way back is [`Broadcast::reunite`], which
    /// resolves once every handle is gone and hands the `Known` to exactly
    /// one caller; if every handle is instead dropped, the set closes.
    pub fn broadcast(self) -> Broadcast<T> {
        Broadcast::new(self)
    }

    /// Take a consistent snapshot of the current state.
    pub fn snapshot(&self) -> Snapshot<T> {
        Snapshot::new(self.network, self.inner.borrow().tree.clone())
    }

    /// Observe every message in this rumor set, from genesis onward. See
    /// [`Messages`] for the contract; equivalent to
    /// [`messages_since`](Self::messages_since) at [`Version::new`].
    pub fn messages(&self) -> Messages<T>
    where
        T: Send + Sync,
    {
        self.messages_since(Version::new())
    }

    /// Observe every message not already causally contained in `since`. See
    /// [`Messages`] for the contract.
    pub fn messages_since(&self, since: Version) -> Messages<T>
    where
        T: Send + Sync,
    {
        Messages::subscribe(&self.inner, since)
    }

    /// Observe every message in this rumor set in *causal order*, from
    /// genesis onward. See [`CausalMessages`] for the contract; equivalent
    /// to [`causal_messages_since`](Self::causal_messages_since) at
    /// [`Version::new`].
    pub fn causal_messages(&self) -> CausalMessages<T>
    where
        T: Send + Sync,
    {
        self.causal_messages_since(Version::new())
    }

    /// Observe every message not already causally contained in `since`, in
    /// *causal order*: each message is delivered after every delivered
    /// message it causally depends on, at the cost of an idle state that
    /// holds the undelivered backlog rather than [`Messages`]' constant
    /// spine. See [`CausalMessages`] for the contract.
    pub fn causal_messages_since(&self, since: Version) -> CausalMessages<T>
    where
        T: Send + Sync,
    {
        CausalMessages::subscribe(&self.inner, since)
    }

    /// The identifier shared by every peer that descends from the same
    /// [`seed`](Known::seed).
    pub fn network(&self) -> Network {
        self.network
    }

    /// The latest version of any message ever tracked by this [`Known`].
    pub fn latest(&self) -> Version {
        self.inner.borrow().tree.latest().clone()
    }

    /// The earliest version of any message currently present in this [`Known`], or
    /// `None` if it has never seen a message.
    pub fn earliest(&self) -> Option<Version> {
        self.inner.borrow().tree.earliest().cloned()
    }

    /// Determine if there are any current messages in this [`Known`].
    pub fn is_empty(&self) -> bool {
        self.inner.borrow().tree.is_empty()
    }

    /// The number of live messages in this [`Known`].
    pub fn len(&self) -> usize {
        self.inner.borrow().tree.len()
    }

    /// The observable root hash of this set: a 32-byte digest of its live
    /// content, independent of party identity and insertion order. Two sets
    /// with equal hashes hold the same live messages. Gossip converges on
    /// causal versions rather than hashes: peers with equal hashes but
    /// different versions (for example, after an insert that was then
    /// redacted) still run a reconciliation pass.
    pub fn hash(&self) -> [u8; 32] {
        self.inner.borrow().tree.hash()
    }

    /// Look up a single live message by its [`Key`]: one `O(depth)` descent
    /// (the key *is* the leaf's content-addressed path), never a scan.
    /// Returns owned handles cloned out of the synchronized state; `None`
    /// when no live message has that key — never inserted, or since
    /// redacted.
    pub fn get(&self, key: &Key) -> Option<(Version, Arc<T>)> {
        self.inner
            .borrow()
            .tree
            .get(key)
            .map(|(version, message)| (version.clone(), message.clone()))
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
