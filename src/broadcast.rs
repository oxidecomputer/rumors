mod acausal;
mod causal;

pub use acausal::Messages;
pub use causal::CausalMessages;

use crate::{Batch, Error, Key, Known, Network, Snapshot, Version};
use borsh::{BorshDeserialize, BorshSerialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::watch,
};

/// A broadcast handle for a set of rumors.
pub struct Broadcast<T> {
    known: Known<T>,
    /// This handle's claim to existence; see [`Extant`].
    extant: Extant,
}

/// One handle's share of a broadcast generation's existence. The `token`
/// [`Arc`]'s strong count *is* the number of extant handles (a pending
/// [`reunite`](Broadcast::reunite) has already shed its share), so the count
/// reaching zero is the moment the generation has quiesced and the [`Known`]
/// may be reclaimed.
#[derive(Clone)]
struct Extant {
    /// The extancy token. An `Option` only so [`Drop`] can shed it *before*
    /// waking waiters on `drops`: a reuniter woken by that send must already
    /// observe the decremented strong count. Always `Some` outside `Drop`.
    token: Option<Arc<()>>,
    /// The exactly-once claim on the reclaimed [`Known`]: among reuniters
    /// that observe quiescence concurrently, the one that wins this flag is
    /// handed the `Known`; the rest resolve `None`.
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

impl<T> Clone for Broadcast<T> {
    fn clone(&self) -> Self {
        Self {
            known: Known {
                network: self.known.network,
                inner: self.known.inner.clone(),
            },
            extant: self.extant.clone(),
        }
    }
}

/// A summary view (network, latest version, live-message count), independent
/// of `T: Debug`: the messages themselves are not printed.
impl<T> std::fmt::Debug for Broadcast<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.known.inner.borrow();
        f.debug_struct("Broadcast")
            .field("network", &self.known.network)
            .field("latest", inner.tree.latest())
            .field("len", &inner.tree.len())
            .finish_non_exhaustive()
    }
}

impl<T> Broadcast<T> {
    /// Assemble the first handle of a fresh broadcast generation around
    /// `known`, the only constructor: every other handle is a [`Clone`] of
    /// this one, so the token count faithfully counts handles.
    pub(crate) fn new(known: Known<T>) -> Self {
        Self {
            known,
            extant: Extant {
                token: Some(Arc::new(())),
                claimed: Arc::new(AtomicBool::new(false)),
                drops: watch::Sender::new(()),
            },
        }
    }

    /// Give up this handle and reclaim the [`Known`] once the set quiesces:
    /// resolves when no [`Broadcast`] for this set remains, handing the
    /// `Known` to exactly one caller.
    ///
    /// The semantics follow [`Arc::into_inner`]: calling `reunite` sheds
    /// this handle's share immediately (a pending reunite no longer counts
    /// as extant, and there is no way back to the `Broadcast`); when the
    /// last remaining handle drops or reunites, every pending `reunite`
    /// resolves at once — one receives `Some`, the rest `None`. Concurrent
    /// reuniters therefore never deadlock, and "last one out" works: N
    /// tasks can each call `reunite` as they finish, and exactly one is
    /// handed the `Known` to retire.
    ///
    /// Cancelling a pending `reunite` abandons its claim: the handle was
    /// already consumed, so dropping the future is no different from having
    /// dropped the `Broadcast`. If every handle goes away with no reunite
    /// pending, the `Known` is gone for good and the set closes: observers
    /// drain the final state and end.
    pub async fn reunite(self) -> Option<Known<T>> {
        let Self { known, extant } = self;
        let token = Arc::downgrade(extant.token.as_ref().expect("Some outside Drop"));
        let claimed = Arc::clone(&extant.claimed);
        // Subscribe before shedding our token, so no later drop's wake can
        // be missed; our own shed below wakes us once, harmlessly.
        let mut drops = extant.drops.subscribe();
        drop(extant);
        loop {
            // Monotone once zero: minting a token takes a live `Broadcast`
            // to clone, and every reuniter has already shed its own.
            if token.strong_count() == 0 {
                // Exactly one reuniter wins the claim; the Known/Broadcast
                // XOR is restored the instant this swap succeeds.
                return claimed
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                    .then_some(known);
            }
            // `Err` here means every sender — every `Extant` — is gone, so
            // the count re-check above terminates the loop.
            let _ = drops.changed().await;
        }
    }

    /// Send a message to all listeners.
    ///
    /// Returns a [`Batch`] that commits when dropped: a bare
    /// `broadcast.send(message);` commits at the end of the statement, and
    /// chaining further [`send`](Batch::send)s and
    /// [`redact`](Batch::redact)s accumulates them into one commit. Building
    /// holds no lock; see [`batch`](Self::batch).
    pub fn send(&self, message: T) -> Batch<'_, T>
    where
        T: BorshSerialize + Send + Sync,
    {
        self.known.send(message)
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
        self.known.redact(key)
    }

    /// Start an empty [`Batch`] of insertions and redactions, committed
    /// atomically when dropped: one tree traversal, one change notification.
    /// Building holds no lock — the rumor set is locked only inside the
    /// commit.
    pub fn batch(&self) -> Batch<'_, T>
    where
        T: Send + Sync,
    {
        self.known.batch()
    }

    /// Gossip with a remote peer to synchronize rumor sets.
    pub async fn gossip<'a, R, W>(&mut self, read: &'a mut R, write: &'a mut W) -> Result<(), Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        self.known.gossip(read, write).await
    }

    /// The identifier shared by every peer that descends from the same
    /// [`seed`](Known::seed).
    pub fn network(&self) -> Network {
        self.known.network()
    }

    /// The latest version of any message ever tracked by this [`Known`].
    pub fn latest(&self) -> Version {
        self.known.latest()
    }

    /// The earliest version of any message currently present in this [`Known`], or
    /// `None` if it has never seen a message.
    pub fn earliest(&self) -> Option<Version> {
        self.known.earliest()
    }

    /// Determine if there are any current messages in this [`Known`].
    pub fn is_empty(&self) -> bool {
        self.known.is_empty()
    }

    /// The number of live messages in this [`Known`].
    pub fn len(&self) -> usize {
        self.known.len()
    }

    /// Take a consistent snapshot of the current state.
    pub fn snapshot(&self) -> Snapshot<T> {
        self.known.snapshot()
    }

    /// The observable root hash of this set: a 32-byte digest of its live
    /// content, independent of party identity and insertion order. Two sets
    /// with equal hashes hold the same live messages. Gossip converges on
    /// causal versions rather than hashes: peers with equal hashes but
    /// different versions (for example, after an insert that was then
    /// redacted) still run a reconciliation pass.
    pub fn hash(&self) -> [u8; 32] {
        self.known.hash()
    }

    /// Look up a single live message by its [`Key`]: one `O(depth)` descent
    /// (the key *is* the leaf's content-addressed path), never a scan.
    /// Returns owned handles cloned out of the synchronized state; `None`
    /// when no live message has that key — never inserted, or since
    /// redacted.
    pub fn get(&self, key: &Key) -> Option<(Version, Arc<T>)> {
        self.known.get(key)
    }

    /// Observe every message in this rumor set, from genesis onward. See
    /// [`Messages`] for the contract; equivalent to
    /// [`messages_from`](Self::messages_from) at [`Version::new`].
    pub fn messages(&self) -> Messages<T>
    where
        T: Send + Sync,
    {
        self.known.messages()
    }

    /// Observe every message not already causally contained in `since`. See
    /// [`Messages`] for the contract.
    pub fn messages_since(&self, since: Version) -> Messages<T>
    where
        T: Send + Sync,
    {
        self.known.messages_since(since)
    }

    /// Observe every message in this rumor set in *causal order*, from
    /// genesis onward. See [`CausalMessages`] for the contract; equivalent
    /// to [`causal_messages_from`](Self::causal_messages_from) at
    /// [`Version::new`].
    pub fn causal_messages(&self) -> CausalMessages<T>
    where
        T: Send + Sync,
    {
        self.known.causal_messages()
    }

    /// Observe every message not already causally contained in `since`, in
    /// *causal order*. See [`CausalMessages`] for the contract.
    pub fn causal_messages_since(&self, since: Version) -> CausalMessages<T>
    where
        T: Send + Sync,
    {
        self.known.causal_messages_since(since)
    }

    /// Force this set's tree to compute its lazy structural memos (observable
    /// hash and ceiling/floor version bounds), so a subsequent operation is
    /// timed against its own work. For benchmark and test calibration only.
    #[doc(hidden)]
    pub fn warm_caches(&self) {
        self.known.warm_caches();
    }

    /// Alias this set's live party for invariant assertions in tests; see
    /// [`Known::dangerously_alias_party`] for the contract.
    #[cfg(any(test, feature = "test-internals"))]
    #[doc(hidden)]
    pub fn dangerously_alias_party(&self) -> Option<before::Party> {
        self.known.dangerously_alias_party()
    }
}
