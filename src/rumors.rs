mod acausal;
mod causal;

pub use acausal::Messages;
pub use causal::CausalMessages;

use crate::{Batch, Error, Key, Network, Peer, Snapshot, Version};
use borsh::{BorshDeserialize, BorshSerialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::watch,
};

/// A handle for [`send`](Rumors::send)ing and [`redact`](Rumors::redact)ing
/// messages, and [`gossip`](Rumors::gossip)ing the result with peers.
///
/// Unlike [`Peer`], [`Rumors`] is [`Clone`], which means that any
/// number of tasks may concurrently interact with the set of rumors,
/// arbitrarily. Synchronization is internal: anything one clone learns, all do.
pub struct Rumors<T> {
    peer: Peer<T>,
    /// This handle's claim to existence; see [`Extant`].
    extant: Extant,
}

/// One handle's share of a broadcast generation's existence. The `token`
/// [`Arc`]'s strong count *is* the number of extant handles (a pending
/// [`try_into_peer`](Rumors::try_into_peer) has already shed its share), so
/// the count reaching zero is the moment the generation has quiesced and the
/// [`Peer`] may be reclaimed.
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

impl<T> Clone for Rumors<T> {
    fn clone(&self) -> Self {
        Self {
            peer: Peer {
                network: self.peer.network,
                inner: self.peer.inner.clone(),
            },
            extant: self.extant.clone(),
        }
    }
}

/// A summary view (network, latest version, live-message count), independent
/// of `T: Debug`: the messages themselves are not printed.
impl<T> std::fmt::Debug for Rumors<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.peer.inner.borrow();
        f.debug_struct("Rumors")
            .field("network", &self.peer.network)
            .field("latest", inner.tree.latest())
            .field("len", &inner.tree.len())
            .finish_non_exhaustive()
    }
}

impl<T> Rumors<T> {
    /// Assemble the first handle of a fresh broadcast generation around
    /// `peer`, the only constructor: every other handle is a [`Clone`] of
    /// this one, so the token count faithfully counts handles.
    pub(crate) fn new(peer: Peer<T>) -> Self {
        Self {
            peer,
            extant: Extant {
                token: Some(Arc::new(())),
                claimed: Arc::new(AtomicBool::new(false)),
                drops: watch::Sender::new(()),
            },
        }
    }

    /// Give up this handle and reclaim the [`Peer`] once no more other handles
    /// exist: resolves when no [`Rumors`] for this set remains, handing the
    /// `Peer` to exactly one caller.
    ///
    /// Cancelling a pending [`try_into_peer`](Self::try_into_peer) abandons its
    /// claim: the handle was already consumed, so dropping the future is no
    /// different from having dropped the `Rumors`. If every handle goes away
    /// with no reunite pending, the `Peer` is gone for good and the set
    /// closes: observers drain the final state and end.
    pub async fn try_into_peer(self) -> Option<Peer<T>> {
        let Self { peer, extant } = self;
        let token = Arc::downgrade(extant.token.as_ref().expect("Some outside Drop"));
        let claimed = Arc::clone(&extant.claimed);
        // Subscribe before shedding our token, so no later drop's wake can be
        // missed; our own shed below wakes us once, harmlessly.
        let mut drops = extant.drops.subscribe();
        drop(extant);
        loop {
            // Monotone once zero: minting a token takes a live `Rumors` to
            // clone, and every reuniter has already shed its own.
            if token.strong_count() == 0 {
                // Exactly one reuniter wins the claim; the Peer/Rumors
                // XOR is restored the instant this swap succeeds.
                return claimed
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                    .then_some(peer);
            }
            // `Err` here means every sender — every `Extant` — is gone, so
            // the count re-check above terminates the loop.
            let _ = drops.changed().await;
        }
    }

    /// Send a message.
    ///
    /// Returns a [`Batch`] that commits when dropped: a bare
    /// `broadcast.send(message);` commits at the end of the statement, and
    /// chaining further [`send`](Batch::send)s and [`redact`](Batch::redact)s
    /// accumulates them into one commit.
    pub fn send(&self, message: T) -> Batch<'_, T>
    where
        T: BorshSerialize + Send + Sync,
    {
        self.peer.send(message)
    }

    /// Redact a message.
    ///
    /// Returns a [`Batch`] that commits when dropped: a bare
    /// `broadcast.send(message);` commits at the end of the statement, and
    /// chaining further [`send`](Batch::send)s and [`redact`](Batch::redact)s
    /// accumulates them into one commit.
    pub fn redact(&self, key: Key) -> Batch<'_, T>
    where
        T: Send + Sync,
    {
        self.peer.redact(key)
    }

    /// Start an empty [`Batch`].
    pub fn batch(&self) -> Batch<'_, T>
    where
        T: Send + Sync,
    {
        self.peer.batch()
    }

    /// Gossip with a remote peer to synchronize rumor sets.
    pub async fn gossip<'a, R, W>(&self, read: &'a mut R, write: &'a mut W) -> Result<(), Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        self.peer.gossip(read, write).await
    }

    /// The identifier shared by every peer that descends from the same
    /// [`seed`](Peer::seed).
    pub fn network(&self) -> Network {
        self.peer.network()
    }

    /// Take a consistent snapshot of the current state.
    pub fn snapshot(&self) -> Snapshot<T> {
        self.peer.snapshot()
    }

    /// Observe every message in this rumor set, from genesis onward. See
    /// [`Messages`] for the contract; equivalent to
    /// [`messages_since`](Self::messages_since) at [`Version::new`].
    pub fn messages(&self) -> Messages<T>
    where
        T: Send + Sync,
    {
        self.peer.messages()
    }

    /// Observe every message not already causally contained in `since`. See
    /// [`Messages`].
    pub fn messages_since(&self, since: Version) -> Messages<T>
    where
        T: Send + Sync,
    {
        self.peer.messages_since(since)
    }

    /// Observe every message in this rumor set in *causal order*, from genesis
    /// onward. See [`CausalMessages`]; equivalent to
    /// [`causal_messages_since`](Self::causal_messages_since) at
    /// [`Version::new`].
    pub fn causal_messages(&self) -> CausalMessages<T>
    where
        T: Send + Sync,
    {
        self.peer.causal_messages()
    }

    /// Observe every message not already causally contained in `since`, in
    /// *causal order*. See [`CausalMessages`].
    pub fn causal_messages_since(&self, since: Version) -> CausalMessages<T>
    where
        T: Send + Sync,
    {
        self.peer.causal_messages_since(since)
    }

    /// Force this set's tree to compute its lazy structural memos (observable
    /// hash and ceiling/floor version bounds), so a subsequent operation is
    /// timed against its own work. For benchmark and test calibration only.
    #[doc(hidden)]
    pub fn warm_caches(&self) {
        self.peer.warm_caches();
    }

    /// Alias this set's live party for invariant assertions in tests; see
    /// [`Peer::dangerously_alias_party`] for the contract.
    #[cfg(any(test, feature = "test-internals"))]
    #[doc(hidden)]
    pub fn dangerously_alias_party(&self) -> Option<before::Party> {
        self.peer.dangerously_alias_party()
    }
}
