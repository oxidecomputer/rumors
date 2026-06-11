//! Unordered gossip with redaction.

// Static assertions uses #[allow(unsafe_code)], so we allow it only in tests
#![cfg_attr(not(test), forbid(unsafe_code))]
// Programmer error in recursive async traits can create large futures, so we
// check to make sure it's not an issue
#![deny(clippy::large_futures)]

use std::future::ready;
use std::sync::Arc;

use before::Party;
use borsh::{BorshDeserialize, BorshSerialize};
use rand::{RngCore, rngs::OsRng};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::watch,
};

pub mod sync;

mod bookmark;
mod broadcast;
mod message;
mod network;
mod snapshot;
mod tree;
mod version;

#[cfg(test)]
mod tests;

use message::Message;
use tree::{Action, Tree, mirror};

pub use broadcast::Broadcast;
pub use network::Network;
pub use snapshot::Snapshot;

/// Magic bytes that prefix every `rumors` gossip session.
pub const PROTOCOL_MAGIC: [u8; 6] = *b"RUMORS";

/// On-the-wire protocol version that follows [`PROTOCOL_MAGIC`].
///
/// Bumped whenever the wire format changes. A peer whose version differs is
/// rejected with [`Error::VersionMismatch`].
pub const PROTOCOL_VERSION: u16 = 1;

/// A local set of rumors.
pub struct Known<T> {
    network: Network,
    inner: watch::Sender<Inner<T>>,
}

struct Inner<T> {
    party: Option<Party>,
    tree: Tree<T>,
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

/// The error type returned by [`Known::gossip`].
pub use mirror::remote::Error;

/// An opaque identifier for a single message in a [`Known`] rumor set.
pub use tree::Key;

/// A causal version vector tagging when a message was observed.
pub use version::Version;

/// Named, composable constructors for causal [`Version`] ranges
/// (re-exported from [`before`](::before)): the vocabulary for
/// [`Snapshot::range`] and [`Broadcast::listen_from`] — e.g.
/// `causally::since(&cursor)` or `causally::not_before(&s).known_at(&e)`.
pub use before::causally;

/// The [`borsh`] crate, re-exported.
///
/// Message types must implement [`BorshSerialize`] and [`BorshDeserialize`];
/// re-exporting borsh here lets callers derive both without a separate
/// dependency.
pub use ::borsh;

use crate::tree::mirror::{
    local::{self, Silent},
    message::Intent,
    remote,
};

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

    /// Bootstrap a brand-new rumor set from a remote peer.
    pub async fn bootstrap<'a, R, W>(
        read: &'a mut R,
        write: &'a mut W,
    ) -> Result<Option<Self>, Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        // Raw magic/version/network/intent preamble first.
        let (remote_network, remote_intent) =
            remote::preamble(Network::BOOTSTRAP, Intent::Remain, read, write).await?;

        // In the bootstrap case, it doesn't matter whether the remote intends
        // to remain or retire; they will hand us a party regardless, and we can
        // absorb it.
        let _ = remote_intent;

        // We hold nothing: we will run the mirror protocol from an *empty* tree
        // to receive all content on the remote side.
        let l = local::Exchange::start(tree::Root::default(), None::<Silent<T>>, None::<Silent<T>>);
        let r = remote::Exchange::start(read, write);

        // After the connect phase, a peer that is *also* bootstrapping means
        // there is nothing to receive: bail symmetrically.
        let handshaken = mirror::handshake(l, r).await.map_err(server_error)?;
        if remote_network.is_bootstrap() {
            return Ok(None);
        }

        // Otherwise reconcile, pulling the provider's whole tree through the
        // descent, then read the provider's party frame off the same reader,
        // and adopt its network alongside.
        // Boxed: the descent state machine is a large future, and the codec
        // buffers inflate it past the crate-wide `large_futures` ceiling.
        let (root, (mut reader, _writer)) = Box::pin(handshaken.reconcile())
            .await
            .map_err(server_error)?;
        let party = remote::recv_party(&mut reader).await?;
        Ok(Some(Self {
            network: remote_network,
            inner: watch::Sender::new(Inner {
                party: Some(party),
                tree: Tree { root },
            }),
        }))
    }

    /// Retire this rumor set into a remote peer, handing it our identity so
    /// that it can be recycled by the network.
    pub async fn retire<'a, R, W>(
        mut self,
        read: &'a mut R,
        write: &'a mut W,
    ) -> (Option<Self>, Result<(), Error>)
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        let (outcome, result) = self.gossip_inner(Intent::Retire, read, write).await;
        (
            match outcome {
                Intent::Remain => Some(self),
                Intent::Retire => None,
            },
            result,
        )
    }

    /// Send messages to all listeners.
    pub fn send<'a, I>(&'a mut self, messages: I)
    where
        T: BorshSerialize + Send + Sync + 'a,
        I: IntoIterator<Item = T> + Send,
        I::IntoIter: Send,
    {
        self.inner.send_if_modified(|inner| {
            if let Some(party) = inner.party.as_ref() {
                let hash_before = inner.tree.hash();
                pollster::block_on(inner.tree.act(
                    |batch| {
                        batch.tick(party);
                    },
                    messages.into_iter().map(Message::from).map(Action::Insert),
                    |_, _, _| ready(()),
                ));
                inner.tree.hash() != hash_before
            } else {
                false
            }
        });
    }

    /// Redact the given keys for all listeners.
    ///
    /// The corresponding messages will be contagiously purged from the
    /// [`Known`] set for all peers who gossip with us, and will be unobserved
    /// by any future peers who did not already observe the messages.
    pub fn redact<I: IntoIterator<Item = Key>>(&mut self, redacted: I)
    where
        T: Send + Sync,
    {
        self.inner.send_if_modified(|inner| {
            if let Some(party) = inner.party.as_ref() {
                let hash_before = inner.tree.hash();
                pollster::block_on(inner.tree.act(
                    |batch| {
                        batch.tick(party);
                    },
                    redacted.into_iter().map(Action::Forget),
                    |_, _, _| ready(()),
                ));
                inner.tree.hash() != hash_before
            } else {
                false
            }
        });
    }

    /// Gossip with a remote peer to synchronize rumor sets.
    pub async fn gossip<'a, R, W>(&mut self, read: &'a mut R, write: &'a mut W) -> Result<(), Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        self.gossip_inner(Intent::Remain, read, write).await.1
    }

    /// Synchronize with a remote peer, optionally trying to retire afterwards.
    ///
    /// The returned `Intent` is *always* `Intent::Remain` if the provided
    /// intent is `Intent::Remain`, and is `Intent::Retire` *only if* the result
    /// of the gossip was the hand-off of the entire local party via retirement
    /// to the remote counterparty. It is possible to return `Intent::Retire`
    /// *and also* an error, in the case that donating our local party itself
    /// fails with an error -- we can't know whether the remote received it or
    /// not, so we have to assume they might have.
    async fn gossip_inner<'a, R, W>(
        &mut self,
        intent: Intent,
        read: &'a mut R,
        write: &'a mut W,
    ) -> (Intent, Result<(), Error>)
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        // Raw magic/version preamble: reject a non-rumors or incompatible peer
        // before the codec trusts any peer-supplied frame length.
        let (remote_network, remote_intent) =
            match remote::preamble(self.network, intent, read, write).await {
                Err(e) => return (Intent::Remain, Err(e)),
                Ok(output) => output,
            };
        let peer_bootstrapping = remote_network.is_bootstrap();
        let self_retiring = intent == Intent::Retire;
        let peer_retiring = remote_intent == Intent::Retire;

        // Stop cleanly, early if we're both trying to retire into each other
        if self_retiring && peer_retiring {
            return (Intent::Remain, Ok(()));
        }

        // Clone out the most-recent tree and *speculatively* remove any party
        // we will donate, both in the same critical section, so that there's
        // no lag between the snapshot of the version we send to our
        // counterparty and the fork of the party. Failing to do both at the
        // same time means a concurrent `send` could introduce messages with a
        // version that exceeds the version communicated to a bootstrapping
        // party, violating party disjointness.
        let mut guarded = PartyGuard {
            party: None,
            recover: self.inner.clone(),
        };
        let mut prior_tree = None;
        self.inner.send_if_modified(|inner| {
            prior_tree = Some(inner.tree.clone());
            guarded.party = if self_retiring {
                // Retiring donates our *whole* identity, not a fork of it.
                //
                // We only can have our hands on a `Known` when there are no
                // extant `Broadcast`s, which means that we aren't stepping on
                // anyone's toes by doing this.
                inner.party.take()
            } else if peer_bootstrapping {
                // Serving a bootstrap donates a fork of our identity.
                inner.party.as_mut().map(Party::fork)
            } else {
                // Plain gossip moves no party at all.
                None
            };
            // We modified the watched party only if we removed something.
            guarded.party.is_some()
        });
        let prior_tree = prior_tree.expect("set in closure");

        // Run the connect phase, which exchanges `message::Handshake`s (the
        // causal version; network and intent already rode the raw preamble).
        let l = local::Exchange::start(prior_tree.root, None::<Silent<T>>, None::<Silent<T>>);
        let r = remote::Exchange::start(read, write);

        // Run the initial handshake to determine if and how to gossip.
        //
        // We do this *even if we already know the networks mismatch* because we
        // want to receive the peer's version, so we can report the
        // `remote_min_events` count computed over the peer's version.
        let handshaken = match mirror::handshake(l, r).await.map_err(server_error) {
            Err(e) => return (Intent::Remain, Err(e)),
            Ok(handshaken) => handshaken,
        };

        // Abort if the networks mismatch
        if !peer_bootstrapping && remote_network != self.network {
            return (
                Intent::Remain,
                Err(Error::NetworkMismatch {
                    remote_network,
                    remote_min_events: handshaken.peer().version.min_ticks(),
                }),
            );
        }

        // Run content reconciliation, so that we both have exactly the same
        // version and messages
        let (root, (mut reader, mut writer)) =
            match Box::pin(handshaken.reconcile()).await.map_err(server_error) {
                Err(e) => return (Intent::Remain, Err(e)),
                Ok(outcome) => outcome,
            };

        // The reconciliation has made both sides causally converged; what
        // remains is the party hand-off, if either side is donating one.
        let mut absorbed = None;
        let mut outcome = Intent::Remain;
        if peer_retiring {
            // The peer is retiring: the reconciliation just made us a causal
            // superset of it, so it now ships its party as one trailing frame
            // on the same wire the descent used, and drops its own copy.
            //
            // The preamble rejects a peer that claims to both bootstrap and
            // retire, and we bailed early if we were retiring too, so no
            // party of ours is in flight here: `guarded.party` is `None`.
            absorbed = match remote::recv_party(&mut reader).await {
                Err(e) => return (Intent::Remain, Err(e)),
                Ok(donated_party) => Some(donated_party),
            };
        } else if let Some(donated) = guarded.party.take() {
            // We are donating: our whole party if we are retiring, or a fresh
            // fork of it if the peer is bootstrapping from us. Taking it out
            // of the guard defuses drop-recovery: from here the peer may hold
            // the party even if the send errors, so it can never be safely
            // re-joined.
            match remote::send_party(donated, &mut writer).await {
                Err(e) => {
                    // A retiring donation in limbo must be assumed received:
                    // report `Intent::Retire` alongside the error so that the
                    // `Known` is not handed back. A lost fork merely leaks its
                    // region; we remain.
                    let outcome = if self_retiring {
                        Intent::Retire
                    } else {
                        Intent::Remain
                    };
                    return (outcome, Err(e));
                }
                Ok(()) => {
                    if self_retiring {
                        // The point of no return: the peer holds our whole
                        // party, so this `Known` must not survive the session.
                        outcome = Intent::Retire;
                    }
                }
            }
        }

        // Write back our (potentially changed) tree and any party absorbed
        // from a retiring peer, notifying when either changes. An overlapping
        // donated party is a protocol violation: we leave our own party
        // untouched, commit nothing, and abort the session.
        let mut party_overlap = false;
        self.inner.send_if_modified(|inner| {
            if let Some(party) = absorbed.take() {
                match inner.party.as_mut() {
                    Some(existing) => {
                        if existing.join(party).is_err() {
                            party_overlap = true;
                            return false;
                        }
                    }
                    // Unreachable in practice: we hold a live `Known` and are
                    // not retiring, so our party is present. Adopting the
                    // donation keeps the arm total without a panic path.
                    None => inner.party = Some(party),
                }
            }

            // Join the tree we got via gossip. The join is async only for the
            // sake of its observation callbacks; with both elided it never
            // yields, so driving it with `pollster` inside the critical
            // section completes synchronously, as in `send` and `redact`.
            let prior_hash = inner.tree.hash();
            pollster::block_on(inner.tree.join(
                Tree { root },
                None::<Silent<T>>,
                None::<Silent<T>>,
            ));

            // We've modified the watch if the peer retired or the tree changed
            peer_retiring || prior_hash != inner.tree.hash()
        });
        if party_overlap {
            return (Intent::Remain, Err(Error::PartyOverlap));
        }

        // In the case where we successfully retired (only callable on the
        // !Clone `Known<T>`), we've given away our inner party and no more
        // actions are possible, so don't hand back the `Known`.
        (outcome, Ok(()))
    }

    /// Get a [`Clone`]-able broadcast handle which can be used concurrently.
    ///
    /// While any [`Broadcast`] handle exists for this [`Known`], it is illegal
    /// to access the underlying [`Known`], because retirement cannot happen
    /// concurrent to gossip sessions. As a consequence, this function returns a
    /// future which yields `self` precisely when there remain no extant
    /// [`Broadcast`]s.
    pub fn broadcast(self) -> (Broadcast<T>, impl Future<Output = Self> + Send)
    where
        T: Send + Sync,
    {
        // The Known/Broadcast XOR: every `Broadcast` clone holds a receiver on
        // this channel (on which nothing is ever sent), and the only sender is
        // captured by the future below, which hands `self` back exactly when
        // `closed()` resolves: when the last receiver, and thus the last
        // `Broadcast`, has dropped. Until then `self` sits inert inside the
        // pending future, so a usable `Known` and a `Broadcast` for the same
        // set never coexist.
        let (extant, alive) = watch::channel(());
        let broadcast = Broadcast {
            known: Self {
                network: self.network,
                inner: self.inner.clone(),
            },
            alive,
        };
        let until_no_broadcasts = async move {
            extant.closed().await;
            self
        };
        (broadcast, until_no_broadcasts)
    }

    /// Take a consistent snapshot of the current state.
    pub fn snapshot(&self) -> Snapshot<T> {
        Snapshot::new(self.network, self.inner.borrow().tree.clone())
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
}

// To ensure that a speculatively forked party always snaps back in place, even
// if we return an error or panic, we place it in a drop-guard that joins it
// back into the remaining party in the `inner` if we don't donate it
// successfully along any return path.
struct PartyGuard<T> {
    pub(crate) party: Option<Party>,
    pub(crate) recover: watch::Sender<Inner<T>>,
}

impl<T> Drop for PartyGuard<T> {
    fn drop(&mut self) {
        if let Some(party) = self.party.take() {
            self.recover
                .send_modify(|inner| match inner.party.as_mut() {
                    // Re-joining a fork we split off this very party: disjoint by
                    // construction, so the join cannot fail in a well-formed
                    // universe. The join must run unconditionally (it is the
                    // recovery), so it cannot live inside a `debug_assert!`.
                    Some(existing) => {
                        if existing.join(party).is_err() {
                            debug_assert!(false, "non-disjoint party in `PartyGuard`");
                        }
                    }
                    // We took the whole party (a retire that failed before the
                    // hand-off): put it back.
                    None => inner.party = Some(party),
                });
        }
    }
}

/// Collapse a mirror error down to the wire-bound server error. The client side
/// of every wire session is the in-memory local exchange, whose error type is
/// [`Infallible`](std::convert::Infallible), so the
/// [`Client`](mirror::Error::Client) arm is uninhabitable.
fn server_error(e: mirror::Error<std::convert::Infallible, Error>) -> Error {
    match e {
        mirror::Error::Server(e) => e,
        mirror::Error::Client(never) => match never {},
    }
}
