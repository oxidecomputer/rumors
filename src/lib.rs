//! Unordered gossip with redaction.

// Static assertions uses #[allow(unsafe_code)], so we allow it only in tests
#![cfg_attr(not(test), forbid(unsafe_code))]
// Programmer error in recursive async traits can create large futures, so we
// check to make sure it's not an issue
#![deny(clippy::large_futures)]

use std::sync::Arc;

use before::Party;
use borsh::{BorshDeserialize, BorshSerialize};
use rand::{RngCore, rngs::OsRng};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::watch,
};

pub mod sync;

mod batch;
mod bookmark;
mod broadcast;
mod message;
mod network;
pub mod snapshot;
mod tree;
mod version;

#[cfg(test)]
mod tests;

use tree::{Tree, mirror};

pub use batch::Batch;
pub use broadcast::{Broadcast, CausalMessages, Messages};
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

/// The outcome of [`Known::retire`]: whether the identity was handed off,
/// and what came back if not.
///
/// Marked `must_use` because two variants carry the intact [`Known`] —
/// silently dropping the result of a declined or recovered retirement
/// destroys the identity that the call was specifically trying to preserve,
/// leaking its id-region from the universe.
#[must_use = "a declined or recovered retirement hands the Known back; dropping it leaks the identity"]
#[derive(Debug)]
pub enum Retire<T> {
    /// **Retired.** The peer reconciled with us and absorbed our party; the
    /// rumor set has left the universe and its id-region is recycled.
    Retired,
    /// **Declined, unchanged.** The peer cannot absorb a party — it was
    /// itself retiring — so nothing touched the wire and the rumor set is
    /// handed back intact, to retry elsewhere.
    Declined {
        /// The intact retiree.
        known: Known<T>,
    },
    /// **Recovered, unchanged.** The session failed *before* our party ever
    /// crossed the wire; the rumor set is handed back intact, to retry
    /// elsewhere. Nothing was lost.
    Recovered {
        /// The intact retiree.
        known: Known<T>,
        /// What failed the session.
        error: Error,
    },
    /// **Uncertain.** The session failed while the party itself was in
    /// flight: the peer may or may not hold it, so the retiree is consumed
    /// rather than risk the same identity living twice.
    Uncertain {
        /// What failed the session.
        error: Error,
    },
}

/// The error type returned by [`Known::gossip`].
pub use mirror::remote::Error;

/// An opaque identifier for a single message in a [`Known`] rumor set.
pub use tree::Key;

/// A causal version vector tagging when a message was observed.
pub use version::Version;

/// Named, composable constructors for causal [`Version`] ranges
/// (re-exported from [`before`]): the vocabulary for
/// [`Snapshot::range`] and [`Known::messages_from`] — e.g.
/// `causally::since(&checkpoint)` or `causally::not_before(&s).known_at(&e)`.
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
    ///
    /// The session begins with a round of gossip: the two peers reconcile
    /// content exactly as [`gossip`](Self::gossip) would, so everything we
    /// hold that the peer had not yet seen survives in it; the peer then
    /// absorbs our party. A peer running ordinary gossip absorbs a retiree
    /// transparently, so the counterparty needs no special call. The four
    /// outcomes are the [`Retire`] variants; see each for what survived.
    ///
    /// The gossip round writes back into the retiring set too: observers of
    /// a retiring set ([`Messages`], [`CausalMessages`]) drain the
    /// *reconciled* final state — everything the session learned included —
    /// before they end.
    pub async fn retire<'a, R, W>(mut self, read: &'a mut R, write: &'a mut W) -> Retire<T>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        match self.gossip_inner(Intent::Retire, read, write).await {
            (Intent::Retire, Ok(())) => Retire::Retired,
            (Intent::Retire, Err(error)) => Retire::Uncertain { error },
            (Intent::Remain, Ok(())) => Retire::Declined { known: self },
            (Intent::Remain, Err(error)) => Retire::Recovered { known: self, error },
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
    /// [`messages_from`](Self::messages_from) at [`Version::new`].
    pub fn messages(&self) -> Messages<T>
    where
        T: Send + Sync,
    {
        self.messages_from(Version::new())
    }

    /// Observe every message not already causally contained in `since`. See
    /// [`Messages`] for the contract.
    pub fn messages_from(&self, since: Version) -> Messages<T>
    where
        T: Send + Sync,
    {
        Messages::subscribe(&self.inner, since)
    }

    /// Observe every message in this rumor set in *causal order*, from
    /// genesis onward. See [`CausalMessages`] for the contract; equivalent
    /// to [`causal_messages_from`](Self::causal_messages_from) at
    /// [`Version::new`].
    pub fn causal_messages(&self) -> CausalMessages<T>
    where
        T: Send + Sync,
    {
        self.causal_messages_from(Version::new())
    }

    /// Observe every message not already causally contained in `since`, in
    /// *causal order*: each message is delivered after every delivered
    /// message it causally depends on, at the cost of an idle state that
    /// holds the undelivered backlog rather than [`Messages`]' constant
    /// spine. See [`CausalMessages`] for the contract.
    pub fn causal_messages_from(&self, since: Version) -> CausalMessages<T>
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
