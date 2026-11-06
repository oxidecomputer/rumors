//! The wire-session drivers for [`Peer`]: [`bootstrap`](Bootstrap::join),
//! [`gossip`](crate::Rumors::gossip), and [`retire`](Peer::retire).
//!
//! A [`ForkGuard`] restores bootstrap forks abandoned before transmission.
//! Retirement owns its consumed `Peer` until the outcome determines whether
//! the identity can be returned to the caller.

use std::{fmt, sync::Arc, time::Instant};

use before::{Count, Party};
use futures::StreamExt;
use futures::{Stream, future::BoxFuture};
use serde::{Serialize, de::DeserializeOwned};
use tokio::{
    io::{AsyncBufRead, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::{Mutex, watch},
};

use crate::counter::widen;
use crate::error::{Mismatch, Phase, TransportOperation as Op};
use crate::message::PayloadCodec;
use crate::network::NetworkKeys;
use crate::observe::{SessionCompletion, SessionHandle, SessionKind, SessionOutcome};
use crate::tree::{self, Tree};
use crate::{Error, Network, Version};
use crate::{
    bookmark::{Bookmark, BookmarkIo, Bookmarked, NoBookmark, Reservation},
    tree::mirror::{
        party,
        preamble::{self, Counterparty, Intent},
        secret,
        streaming::{
            self, Local, materialized,
            remote::{self as streaming_remote, PipeRead, RunBudget, WriteBatch},
            stats::{Recorder, SessionStats},
        },
    },
};

use super::{Inner, MemoryBudget, Peer, bootstrap::Bootstrap};

/// The completion marker each side writes after all of its session work,
/// in the closing exchange: the CBOR text item `"."`.
///
/// Receiving this marker confirms that the peer also committed. Its first byte
/// differs from a preamble's, so a premature next session is reported as a
/// protocol violation rather than mistaken for completion.
pub(crate) const COMPLETION_MARKER: [u8; 2] = [0x61, b'.'];

/// A session's read half with its concrete transport type erased.
///
/// Session entry points erase the caller's halves before reconciliation.
/// Otherwise each transport type would instantiate the protocol's deeply
/// generic state machine in every downstream binary. The non-generic,
/// non-inlined drivers keep one copy in this crate.
///
/// Erasure costs one virtual call per read or write poll beneath the
/// framing buffers.
type DynRead<'a> = &'a mut (dyn AsyncBufRead + Unpin + Send + 'a);

/// A session's write half with its concrete transport type erased.
///
/// See [`DynRead`] for why the erasure exists and what it costs.
type DynWrite<'a> = &'a mut (dyn AsyncWrite + Unpin + Send + 'a);

/// Reconciled content and the pipe's halves, ready for identity transfer
/// or completion.
type Reconciled<'a> = (tree::Root, PipeRead<DynRead<'a>>, DynWrite<'a>);

/// The in-process participant every reconciliation starts: the walk over
/// the local replica's tree.
type LocalParticipant = materialized::Handshaking<Local, materialized::Start>;

/// The wire-bound participant every reconciliation starts: the proxy over
/// the session's erased halves.
type RemoteParticipant<'a> = streaming_remote::Handshaking<Local, DynRead<'a>, DynWrite<'a>>;

/// The outcome of [`Peer::retire`].
///
/// [`Retired`](Self::Retired) and [`Declined`](Self::Declined) completed the
/// session and return the connection's halves, resting at the session
/// boundary. [`Declined`](Self::Declined) and [`Recovered`](Self::Recovered)
/// return the peer so it can keep gossiping or retry retirement. Dropping
/// either result discards that peer.
#[must_use = "retirement may return a peer that can keep gossiping or retry"]
pub enum Retire<T: Send + Sync + 'static, R, W, B: Bookmark = NoBookmark> {
    /// Retirement completed and this replica left the network.
    /// The remote peer confirmed completion.
    Retired {
        /// The connection's read half, ready for another session.
        read: R,
        /// The connection's write half, ready for another session.
        write: W,
    },
    /// The remote peer was also retiring, so the session changed neither replica.
    /// Retry retirement with another peer.
    Declined {
        /// The unchanged peer, still usable.
        peer: Peer<T, B>,
        /// The connection's read half, ready for another session.
        read: R,
        /// The connection's write half, ready for another session.
        write: W,
    },
    /// Retirement failed before the peer had to be consumed.
    /// The returned peer can keep gossiping or retry retirement on a fresh
    /// connection; discard the failed one.
    Recovered {
        /// The unchanged peer, still usable.
        peer: Peer<T, B>,
        /// The session failure.
        error: Error<B>,
    },
    /// Retirement started but could not be confirmed. This peer was consumed
    /// and cannot resume gossip or retry; the remote peer may have committed.
    /// Discard the connection.
    ///
    /// This includes failures while awaiting [`Phase::Completion`].
    Uncertain {
        /// The session failure.
        error: Error<B>,
    },
}

/// Format retirement outcomes without requiring debuggable payload, bookmark,
/// or transport types.
impl<T: Send + Sync + 'static, R, W, B: Bookmark> fmt::Debug for Retire<T, R, W, B> {
    /// Formats the outcome and any returned peer or error.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Retired { .. } => f.debug_struct("Retired").finish_non_exhaustive(),
            Self::Declined { peer, .. } => f
                .debug_struct("Declined")
                .field("peer", peer)
                .finish_non_exhaustive(),
            Self::Recovered { peer, error } => f
                .debug_struct("Recovered")
                .field("peer", peer)
                .field("error", error)
                .finish(),
            Self::Uncertain { error } => f.debug_struct("Uncertain").field("error", error).finish(),
        }
    }
}

/// One completed exchange, returned by [`gossip_once`](crate::Rumors::gossip_once)
/// or yielded by the [`gossip`](crate::Rumors::gossip) stream.
///
/// One of these exists per successful session; a failed session is an
/// `Err` instead (the terminal `Err` of the `gossip` stream).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Gossiped {
    /// The causal frontier the two replicas converged on.
    ///
    /// Both replicas absorbed this frontier. Concurrent local work may leave
    /// either replica ahead of it when the session commits.
    pub converged: Version,
    /// Which trigger initiated the session on this side.
    pub led: Led,
    /// What the session measured about itself; every count is local, so
    /// the two ends of one session report their own numbers.
    ///
    /// The byte and frame counts cover only the data frames that compare
    /// and transfer messages. The session's setup, any identity it donates,
    /// and its completion confirmation share the connection with those
    /// frames but are not counted, so the byte counts are less than the
    /// bytes the session moved over the connection.
    ///
    /// See [`SessionStats`] for each field's meaning and where it is
    /// counted at.
    pub stats: SessionStats,
}

/// Which side initiated a round of gossip.
///
/// The session protocol itself is symmetric, and when both sides' triggers fire
/// close together, each side may record `Local` for what becomes one session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Led {
    /// The local policy or an explicit [`gossip_once`](crate::Rumors::gossip_once)
    /// call initiated the session.
    Local,
    /// The remote's preamble arrived first: this side responded.
    Remote,
}

/// Whether a policy stream item requests gossip conditionally or unconditionally.
///
/// `()` converts to [`WhenChanged`](Self::WhenChanged), so the default changes
/// stream needs no adaptation for [`Peer::gossip_when`]. Unconditional requests
/// can add heartbeat sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Gossip {
    /// Initiate a session only if the local set has changed since this
    /// connection last [`converged`](Gossiped::converged).
    ///
    /// If nothing changed, the driver skips the session without sending bytes.
    /// In particular, a change notification from the session's own merge never
    /// starts another session on the connection that delivered it.
    WhenChanged,
    /// Initiate a session whether or not anything changed locally.
    ///
    /// A session between already-converged replicas is a short round-trip
    /// that confirms the connection works end to end, and a session
    /// between diverged ones converges them both ways — so an interval
    /// stream of unconditional requests doubles as a liveness probe and an
    /// anti-entropy net, pulling news a remote's own policy stream stayed
    /// quiet about.
    Unconditionally,
}

/// Use unit-valued streams, including change subscriptions, as conditional initiation requests.
impl From<()> for Gossip {
    /// Treat a change notification as a conditional initiation request.
    fn from((): ()) -> Self {
        Gossip::WhenChanged
    }
}

/// What a bootstrap session produced.
pub(crate) enum Bootstrapped<T: Send + Sync + 'static> {
    /// The session completed, and the provider confirmed it.
    Joined(Peer<T>),
    /// Both endpoints were bootstrapping, so neither could provide.
    Bailed,
    /// The joiner received its whole party, then the session failed before
    /// the provider confirmed completion. The peer is live all the same.
    Unconfirmed {
        /// The new peer.
        peer: Peer<T>,
        /// The failure after the party arrived.
        error: Error,
    },
}

/// What a joiner holds once its whole party has arrived: everything it
/// needs to become a peer, whether or not the completion exchange that
/// follows succeeds.
struct Received {
    /// The network's keys, derived from the provider's sealed secret item.
    keys: Arc<NetworkKeys>,
    /// The donated identity.
    party: Party,
    /// The provider's content, reconciled into the empty tree.
    root: tree::Root,
}

/// Create joined peers and attach bookmark storage.
impl<T: Send + Sync + 'static> Peer<T, NoBookmark> {
    /// Run bootstrap over any connection.
    ///
    /// A thin generic funnel: the only monomorphized-per-transport code is
    /// the erasure of the two halves here.
    pub(crate) fn bootstrap_inner<'a, R, W>(
        config: Bootstrap<T>,
        read: &'a mut R,
        write: &'a mut W,
    ) -> BoxFuture<'a, Result<Bootstrapped<T>, Error>>
    where
        T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
        R: AsyncBufRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        Box::pin(async move {
            let policy = config.gossip_policy.clone();
            let started = Instant::now();
            let completion = SessionCompletion::new(config.observe.begin(SessionKind::Bootstrap));
            let observe = completion.handle();
            let stats = Recorder::default();
            // The peer-to-be's payload codec, built before it exists:
            // the bootstrap session's ingress decodes through it, and the
            // constructed peer inherits it.
            let codec = PayloadCodec::new::<T>(config.payload_depth_limit);
            // The received party lives here, outside the session future, so
            // it survives that future's failure or its cancellation by the
            // deadline.
            let mut received = None;
            let result = policy
                .run(Self::bootstrap_erased(
                    config.sync_memory_budget,
                    config.run_budget,
                    config.write_batch,
                    codec,
                    &mut received,
                    read as DynRead<'_>,
                    write as DynWrite<'_>,
                    observe,
                    stats.clone(),
                ))
                .await;
            let mut measured = stats.snapshot();
            measured.elapsed = started.elapsed();
            // Once the whole party has arrived, the joiner keeps it however
            // the session ends: the provider removed it before sending it and
            // never restores it, so exactly one side holds it (see
            // `bootstrap_erased`).
            let outcome = match (result, received) {
                (Ok(()), Some(received)) => {
                    let peer = Self::joined(config, codec, received);
                    completion.finish(SessionOutcome::Completed {
                        converged: peer.inner.borrow().tree.latest(),
                        stats: measured,
                    });
                    Bootstrapped::Joined(peer)
                }
                (Ok(()), None) => {
                    completion.finish(SessionOutcome::Completed {
                        converged: &Version::new(),
                        stats: measured,
                    });
                    Bootstrapped::Bailed
                }
                (Err(error), Some(received)) => {
                    completion.finish(SessionOutcome::Failed(error.session_error_kind()));
                    Bootstrapped::Unconfirmed {
                        peer: Self::joined(config, codec, received),
                        error,
                    }
                }
                (Err(error), None) => {
                    completion.finish(SessionOutcome::Failed(error.session_error_kind()));
                    return Err(error);
                }
            };
            Ok(outcome)
        })
    }

    /// The peer a bootstrap that received its whole party becomes, carrying
    /// the builder's settings.
    fn joined(config: Bootstrap<T>, codec: PayloadCodec, received: Received) -> Self {
        let Received { keys, party, root } = received;
        Self {
            keys: Arc::clone(&keys),
            sync_memory_budget: config.sync_memory_budget,
            run_budget: config.run_budget,
            write_batch: config.write_batch,
            gossip_policy: config.gossip_policy,
            inner: watch::Sender::new(Inner::new(party, Tree::from_root(keys, root))),
            bookmark: Arc::new(Mutex::new(Bookmarked::new(NoBookmark))),
            codec,
            observe: config.observe,
        }
    }

    /// The transport-erased bootstrap body behind [`bootstrap_inner`].
    ///
    /// Leaves the joiner's network keys, party, and content in `received`
    /// once the whole party has arrived, then confirms completion. `Ok` with
    /// nothing received means the counterparty was bootstrapping too.
    ///
    /// [`bootstrap_inner`]: Self::bootstrap_inner
    // The arguments are the session's inputs, one per concern; bundling them
    // into a struct would only rename the arity.
    #[allow(clippy::too_many_arguments)]
    fn bootstrap_erased<'a>(
        sync_memory_budget: MemoryBudget,
        run_budget: RunBudget,
        write_batch: WriteBatch,
        codec: PayloadCodec,
        received: &'a mut Option<Received>,
        read: DynRead<'a>,
        write: DynWrite<'a>,
        observe: SessionHandle,
        stats: Recorder,
    ) -> BoxFuture<'a, Result<(), Error>> {
        Box::pin(async move {
            // Establish the protocol version and the provider's network
            // before interpreting the rest of the session. Our ephemeral key
            // travels right after our preamble, before we know whether the
            // counterparty provides, so the exchange that seals the
            // network's secret for us costs no round trip.
            let joining = secret::Joining::new();
            let mut staged = preamble::Staged::new();
            let counterparty = preamble::preamble(
                Network::BOOTSTRAP,
                Intent::Remain,
                &joining.item(),
                &mut staged,
                read,
                write,
                &observe,
            )
            .await
            .map_err(Error::from)?;

            // A provider sends its network's secret next, sealed for our
            // key, ahead of its greeting, so that this side can place the
            // leaves it supplies. Another bootstrapper has no secret
            // and supplies nothing; its key item, sent as ours was, is read
            // and discarded.
            let (received_keys, remote_role) = match counterparty {
                Counterparty::Bootstrapping(item) => {
                    secret::decode_key(&item)?;
                    (None, RemoteRole::Bootstrapping)
                }
                Counterparty::Established { network, .. } => {
                    let secret = joining.receive(&mut *read, network, &observe).await?;
                    (
                        Some(Arc::new(NetworkKeys::new(secret))),
                        RemoteRole::Providing,
                    )
                }
            };

            // Reconcile from an empty tree. On arrival, the returned root
            // holds the provider's content and the next item carries the
            // donated identity. Two bootstrappers instead finish without one.
            // The received keys place each supplied leaf; without them, the
            // session admits no supply.
            let reconciliation = Reconciliation {
                root: tree::Root::default(),
                read,
                write,
                codec,
                keys: received_keys.clone(),
                sync_memory_budget,
                run_budget,
                write_batch,
                remote_role,
                stats,
                observe: observe.clone(),
            };
            let (root, mut read, mut write) = reconciliation.reconcile().await?;
            // Two bootstrappers have no identity to exchange.
            let Some(keys) = received_keys else {
                return finish_session(&mut read, &mut write, &observe).await;
            };
            // A provider donates a fork or its whole retiring identity.
            let party = party::receive(&mut read, &observe).await?;
            // The whole party has arrived, and this side keeps it from here
            // on, even if the completion exchange below fails or the deadline
            // cancels it. That never duplicates the identity, because exactly
            // one side holds it at every point. A providing peer removes a
            // fork from its live party when it reserves it, makes the removal
            // durable in its bookmark before the first byte of the party item
            // is written, and takes the fork out of its recovery guard before
            // that write begins, so it never restores a fork it has started
            // to send. A retiring provider is consumed from its first send
            // poll. So once the party can have arrived here, the provider no
            // longer holds it, whatever its own session's outcome; a cut
            // before it arrives whole leaves this side without it.
            *received = Some(Received { keys, party, root });
            finish_session(&mut read, &mut write, &observe).await
        })
    }
}

/// Run sessions and maintain bookmarks across ownership changes.
impl<T: Send + Sync + 'static, B: Bookmark> Peer<T, B> {
    /// Reconcile and donate, returning the peer only when handoff has not begun.
    ///
    /// Reconciliation publishes on both sides. Surviving message observers
    /// drain the retiring peer's final merged state before they end.
    pub(crate) async fn retire_inner<R, W>(self, mut read: R, mut write: W) -> Retire<T, R, W, B>
    where
        R: AsyncBufRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        let mut staged = preamble::Staged::new();
        let (intent, result) = self
            .session(Intent::Retire, &mut staged, &mut read, &mut write)
            .await;
        match (intent, result) {
            (Intent::Retire, Ok(_)) => Retire::Retired { read, write },
            (Intent::Retire, Err(error)) => Retire::Uncertain { error },
            (Intent::Remain, Ok(_)) => Retire::Declined {
                peer: self,
                read,
                write,
            },
            (Intent::Remain, Err(error)) => Retire::Recovered { peer: self, error },
        }
    }

    /// Gossip with a remote peer to synchronize rumor sets.
    pub(crate) async fn gossip_once<R, W>(
        &self,
        mut read: R,
        mut write: W,
    ) -> Result<(Gossiped, R, W), Error<B>>
    where
        R: AsyncBufRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        let mut staged = preamble::Staged::new();
        let (_, result) = self
            .session(Intent::Remain, &mut staged, &mut read, &mut write)
            .await;
        // A one-shot session is always locally led: the call itself is
        // this side's trigger. A remote preamble already in flight merges
        // into the same session (the preamble exchange is symmetric),
        // exactly as it does when two `gossip` triggers race.
        let (converged, stats) = result?;
        Ok((
            Gossiped {
                converged,
                led: Led::Local,
                stats,
            },
            read,
            write,
        ))
    }

    /// Time an established peer's session while retaining its recovery outcome.
    ///
    /// The outcome must outlive the protocol future: after deadline cancellation,
    /// retirement can return the peer only if its handoff never started.
    async fn session(
        &self,
        intent: Intent,
        staged: &mut preamble::Staged,
        read: DynRead<'_>,
        write: DynWrite<'_>,
    ) -> (Intent, Result<(Version, SessionStats), Error<B>>) {
        let mut outcome = Intent::Remain;
        let kind = match intent {
            Intent::Remain => SessionKind::Gossip,
            Intent::Retire => SessionKind::Retire,
        };
        let started = Instant::now();
        let completion = SessionCompletion::new(self.observe.begin(kind));
        let observe = completion.handle();
        let stats = Recorder::default();
        let mut result = self
            .gossip_policy
            .run(self.gossip_inner(intent, &mut outcome, staged, read, write, observe, stats))
            .await;
        if let Ok((_, stats)) = &mut result {
            stats.elapsed = started.elapsed();
        }
        match &result {
            Ok((converged, stats)) => completion.finish(SessionOutcome::Completed {
                converged,
                stats: *stats,
            }),
            Err(error) => completion.finish(SessionOutcome::Failed(error.session_error_kind())),
        }
        (outcome, result)
    }

    /// Reclaim caught-up identities and checkpoint our own writes when needed.
    ///
    /// Hold the bookmark mutex through persistence, taking the replica lock
    /// only to read its current version and update the party. This records all
    /// local events visible at that instant without holding the replica during I/O.
    /// Remote-only version advances leave our checkpoint current.
    async fn bookmark_update(&self) -> Result<(), BookmarkIo<B::Error>> {
        let mut bookmark = self.bookmark.lock().await;
        let loaded = bookmark.load().await?;
        let mut store = None;
        Inner::update_party(&self.inner, |inner| {
            store = loaded.checkpoint(
                self.network(),
                &mut inner.party,
                inner.tree.latest(),
                &inner.reservations,
            );
        });
        match store {
            Some(store) => store.await,
            None => Ok(()),
        }
    }

    /// Synchronize with a remote peer, optionally donating our identity.
    ///
    /// Only `retire_inner` may pass `Intent::Retire`: it owns the consumed
    /// `Peer`, so no writable handle can coexist with the donation. `outcome` becomes
    /// `Intent::Retire` when the peer must be consumed, even on error or timeout.
    ///
    /// Set `outcome` before first polling the identity write: after that point
    /// the recipient may hold it. The deadline wrapper owns this flag so dropping
    /// this future cannot lose the handoff decision.
    ///
    /// Success includes session statistics and the converged frontier, before
    /// concurrent local commits. [`gossip`](crate::Rumors::gossip)
    /// uses that frontier to decide whether further gossip is needed.
    ///
    /// `staged` holds any preamble bytes already read by the continuous gossip driver.
    /// The erased halves and [`Reconciliation`] keep protocol code generation
    /// independent of the caller's concrete transport type.
    // The arguments are the session's inputs, one per concern; bundling them
    // into a struct would only rename the arity.
    #[allow(clippy::too_many_arguments)]
    async fn gossip_inner<'a>(
        &self,
        intent: Intent,
        outcome: &mut Intent,
        staged: &mut preamble::Staged,
        read: DynRead<'a>,
        write: DynWrite<'a>,
        observe: SessionHandle,
        stats: Recorder,
    ) -> Result<(Version, SessionStats), Error<B>> {
        let codec = self.codec;
        // Check protocol and network compatibility before reconciliation.
        let counterparty =
            preamble::preamble(self.network(), intent, &[], staged, read, write, &observe)
                .await
                .map_err(|error| Error::from(error).widen())?;
        let peer_bootstrapping = counterparty.bootstrapping();
        let self_retiring = intent == Intent::Retire;
        let peer_retiring = counterparty.retiring();

        // Stop cleanly, early if we're both trying to retire into each other.
        // Symmetric by construction: both sides take this same branch, so the
        // completion markers pair up with no session body between them.
        if self_retiring && peer_retiring {
            epilogue(read, write, &observe)
                .await
                .map_err(Error::widen)?;
            let unchanged = self.inner.borrow().tree.latest().clone();
            return Ok((unchanged, stats.snapshot()));
        }

        // A bootstrapping counterparty needs the network's secret before the
        // first supply arrives, to place each leaf it receives, and leaves
        // the session holding it. Its key item followed its preamble; seal
        // the secret for that key, ahead of the greeting.
        if let Counterparty::Bootstrapping(item) = &counterparty {
            let joiner = secret::decode_key(item).map_err(Error::widen)?;
            secret::send(self.keys.secret(), &joiner, self.network(), write, &observe)
                .await
                .map_err(Error::widen)?;
        }

        // Store before sharing: we checkpoint at the snapshot's frontier and
        // then transmit from that same snapshot, so that a restart which has
        // caught up with the stored progress knows every own event this
        // session shares. The checkpoint, the snapshot, and any fork
        // reservation happen under one replica lock, so a concurrent send
        // lands wholly before or wholly after all three and the newcomer
        // cannot inherit a party without its latest events. Lock order:
        // bookmark, writer gate, watch. Retirement reserves no fork, because
        // the consumed Peer keeps its whole party.
        let mut guarded = ForkGuard {
            fork: None,
            recover: self.inner.clone(),
        };
        let mut prior_tree = None;
        {
            let mut bookmark = self.bookmark.lock().await;
            let loaded = bookmark.load().await.map_err(Error::Bookmark)?;
            let mut store = None;
            Inner::update_party(&self.inner, |inner| {
                store = loaded.checkpoint(
                    self.network(),
                    &mut inner.party,
                    inner.tree.latest(),
                    &inner.reservations,
                );
                prior_tree = Some(inner.tree.clone());
                if peer_bootstrapping && !self_retiring {
                    guarded.fork = Some(inner.reserve());
                }
            });
            if let Some(store) = store {
                store.await.map_err(Error::Bookmark)?;
            }
        }
        let prior_tree = prior_tree.expect("set in closure");
        // Advertise the event floor of the snapshot used for reconciliation.
        let local_min_events = prior_tree.latest().min_ticks();

        // Retain the starting root so publication can detect concurrent changes.
        // Reconciliation owns a clone and returns the merged root and the
        // pipe's halves. Its boxed future keeps the protocol state out of this outer
        // future and the consumer crate that instantiates it.
        let reconciliation = Reconciliation {
            root: prior_tree.root.clone(),
            read,
            write,
            codec,
            keys: Some(Arc::clone(prior_tree.keys())),
            sync_memory_budget: self.sync_memory_budget,
            run_budget: self.run_budget,
            write_batch: self.write_batch,
            stats: stats.clone(),
            observe: observe.clone(),
            remote_role: match counterparty {
                Counterparty::Bootstrapping(_) => RemoteRole::Bootstrapping,
                Counterparty::Established { network, .. } => RemoteRole::Gossiping {
                    remote_network: network,
                    local_network: self.network(),
                    local_min_events,
                },
            },
        };
        let reconcile = reconciliation.reconcile();
        let (root, mut read, write) = reconcile.await.map_err(Error::widen)?;

        // The reconciliation has made both sides causally converged; what
        // remains is the party hand-off, if either side is donating one.
        let mut absorbed = None;
        if peer_retiring {
            // We now hold the retiree's history and can safely inherit its
            // identity. Neither side can also be donating a bootstrap fork,
            // and the mutual-retirement case returned before reconciliation.
            absorbed = Some(
                party::receive(&mut read, &observe)
                    .await
                    .map_err(Error::widen)?,
            );
        } else if self_retiring || guarded.fork.is_some() {
            // Remove the donated party before sending: we make the donation's
            // removal durable before any byte of it can reach the recipient. A
            // failure hands the recipient nothing: a retiring peer still owns
            // its Peer and a bootstrap's guard still owns its fork, which the
            // next checkpoint records even if the removal reached storage.
            {
                let mut bookmark = self.bookmark.lock().await;
                let loaded = bookmark.load().await.map_err(Error::Bookmark)?;
                let store = if self_retiring {
                    loaded.donate(self.network(), &self.inner.borrow().party)
                } else {
                    let (fork, _) = guarded.fork.as_ref().expect("bootstrap fork");
                    loaded.donate(self.network(), fork)
                };
                store.await.map_err(Error::Bookmark)?;
            }

            // `send` encodes immediately and retains no party borrow. Release
            // the watch lock before I/O, or discard the fork's recovery guard.
            let send = if self_retiring {
                let inner = self.inner.borrow();
                party::send(&inner.party, write, &observe)
            } else {
                // The removal of the party is now durable, so releasing the
                // reservation can no longer permit any checkpoint offer this
                // fork across a restart.
                let (donated, _reservation) = guarded.fork.take().expect("bootstrap fork");
                party::send(&donated, write, &observe)
            };
            if self_retiring {
                // From the first poll onward the peer may receive our identity.
                // Even an error must consume us; returning the Peer could give
                // two participants authority over the same identity space.
                *outcome = Intent::Retire;
            }
            send.await.map_err(Error::widen)?;
        }

        // Record the session's converged frontier before joining in local
        // changes made while it ran. Retain the incoming tree until the
        // commit unlocks: its discarded payloads may access this replica.
        let merged = Tree::from_root(Arc::clone(&self.keys), root);
        let converged = merged.latest().clone();
        let committed = Inner::publish(&self.inner, &prior_tree, &merged, |inner| {
            if let Some(party) = absorbed.take() {
                inner.party.join(party).map_err(|_| ())?;
            }

            Ok::<(), ()>(())
        });
        // Publication has released both locks. Free session roots before any
        // further I/O so a slow peer cannot prolong removed payloads' lifetimes.
        drop(merged);
        drop(prior_tree);
        if committed.is_err() {
            return Err(
                Error::violation(Phase::IdentityTransfer, SessionDefect::PartyOverlap).widen(),
            );
        }

        // The retiree already removed this identity from its bookmark. Persist
        // it in ours before confirming absorption, or a crash could lose it.
        // Report a failed write even though the in-memory join has committed.
        if peer_retiring {
            self.bookmark_update().await.map_err(Error::Bookmark)?;
        }

        // Confirm that both sides completed their commits, including any
        // attached bookmark writes. Preserve `outcome` on error: retirement
        // cannot return its identity merely because confirmation was lost.
        finish_session(&mut read, write, &observe)
            .await
            .map_err(Error::widen)?;

        // Retirement's caller consumes the Peer after any possible handoff.
        Ok((converged, stats.snapshot()))
    }
}

/// Drive repeated sessions over one connection with application-owned timing.
impl<T: Send + Sync + 'static, B: Bookmark> Peer<T, B> {
    /// Run the continuous driver under this peer's initiation policy.
    #[must_use = "the driver does nothing until the returned stream is polled"]
    pub(crate) fn gossip_driver<'a, R, W>(
        &'a self,
        read: R,
        write: W,
    ) -> impl Stream<Item = Result<Gossiped, Error<B>>> + Unpin + 'a
    where
        R: AsyncBufRead + Unpin + Send + 'a,
        W: AsyncWrite + Unpin + Send + 'a,
    {
        // Each driver gets its own change subscription and policy stream.
        // Factories are shared by the replica, but active state belongs here.
        let drive = Drive {
            peer: self,
            read: Box::new(read),
            write: Box::new(write),
            when: self
                .gossip_policy
                .when(crate::Changes::subscribe(&self.inner)),
            staged: preamble::Staged::new(),
            converged: None,
            done: false,
        };
        // Boxed for two reasons: the box makes the returned stream `Unpin`
        // (callers consume it directly, no `pin!` ceremony), and it moves
        // the driver's in-flight session future — a large state machine —
        // off the caller's stack.
        Box::pin(futures::stream::unfold(drive, |mut drive| async move {
            if drive.done {
                return None;
            }
            loop {
                // No timer runs here. Receiving the first bytes returns
                // immediately, so a partial preamble enters the timed
                // exchange instead of leaving the driver idle.
                let trigger = {
                    tokio::select! {
                        arrival = drive.staged.wait_for_start(&mut *drive.read) => Trigger::Arrival(arrival),
                        item = drive.when.next() => Trigger::Tick(item),
                    }
                };
                let led = match trigger {
                    Trigger::Arrival(Err(e)) => {
                        drive.done = true;
                        return Some((Err(Error::from(e).widen()), drive));
                    }
                    // EOF or policy exhaustion at an untouched boundary
                    // ends cleanly. Once any bytes arrive, this same poll
                    // proceeds to the session before it can suspend again.
                    Trigger::Arrival(Ok(false)) | Trigger::Tick(None) => return None,
                    Trigger::Arrival(Ok(true)) => Led::Remote,
                    Trigger::Tick(Some(Gossip::WhenChanged)) => {
                        // Suppression: a WhenChanged request initiates only
                        // if the local frontier has advanced past what
                        // this connection last converged on. The
                        // comparison is local-only — it can never block
                        // learning *remote* news, which always arrives
                        // remote-led.
                        let news = {
                            let inner = drive.peer.inner.borrow();
                            drive.converged.as_ref() != Some(inner.tree.latest())
                        };
                        if !news {
                            continue;
                        }
                        Led::Local
                    }
                    // An unconditional request's purpose is a session that
                    // may have nothing to say: the round-trip is the
                    // liveness probe, and the convergence is the
                    // anti-entropy pull.
                    Trigger::Tick(Some(Gossip::Unconditionally)) => Led::Local,
                };

                // `unfold` keeps the session and its deadline alive when
                // the caller drops a `next()` future between polls.
                let (_, result) = drive
                    .peer
                    .session(
                        Intent::Remain,
                        &mut drive.staged,
                        &mut *drive.read,
                        &mut *drive.write,
                    )
                    .await;
                return match result {
                    Ok((converged, stats)) => {
                        // Re-arm for the next session: a fresh staging
                        // buffer (this preamble is consumed), and the new
                        // suppression token.
                        drive.staged = preamble::Staged::new();
                        drive.converged = Some(converged.clone());
                        Some((
                            Ok(Gossiped {
                                converged,
                                led,
                                stats,
                            }),
                            drive,
                        ))
                    }
                    Err(e) => {
                        drive.done = true;
                        Some((Err(e), drive))
                    }
                };
            }
        }))
    }
}

/// One reconciliation's fully erased inputs.
///
/// Gossip and bootstrap both assemble this bundle and consume it through
/// [`reconcile`](Self::reconcile). That non-generic, non-inlined boundary keeps
/// the protocol state machine in this crate rather than instantiating it for
/// each downstream peer and transport type.
struct Reconciliation<'a> {
    /// The local replica's root, snapshotted inside the session transaction's
    /// critical section: exactly what the local participant reconciles from.
    root: tree::Root,
    /// The session's erased read half.
    read: DynRead<'a>,
    /// The session's erased write half.
    write: DynWrite<'a>,
    /// The peer's payload codec: the payload boundary in both directions.
    ///
    /// Supplied leaves decode through it at wire ingress, and egress
    /// replays only bytes it previously admitted (at send, or at an
    /// earlier session's ingress). Its depth limit rides the greeting,
    /// where the counterparty's must match.
    codec: PayloadCodec,
    /// The keys of the network whose tree `root` is, or `None` for a
    /// bootstrapper meeting another bootstrapper.
    ///
    /// Their path key places supplied leaves, and their hash key is the one
    /// `root`'s nodes cache their digests under, which this side lists and
    /// compares. A bootstrapper meeting a bootstrapper holds no network
    /// secret, lists nothing, and can be supplied nothing.
    keys: Option<Arc<NetworkKeys>>,
    /// The peer's memory budget as set, from which and `run_budget` the
    /// session derives the budget its ledger admits work against
    /// ([`MemoryBudget::session`]).
    sync_memory_budget: MemoryBudget,
    /// The supply-run sizing budget; its byte target rides the greeting.
    run_budget: RunBudget,
    /// How many bytes of frames one flush of the session's multiplexer may
    /// carry.
    write_batch: WriteBatch,
    /// The session's stats recorder, shared by both protocol participants.
    stats: Recorder,
    /// The session's observation handle: inert unless a handler is
    /// attached, and shared, like the recorder, by every layer that
    /// moves a wire item.
    observe: SessionHandle,
    /// The remote's role and the context needed to validate its greeting.
    remote_role: RemoteRole,
}

/// How the remote peer participates in this session.
enum RemoteRole {
    /// The remote provides membership and content to a joining peer.
    Providing,
    /// The remote is joining and must have no causal history.
    Bootstrapping,
    /// The remote is an established peer and must be in the same network.
    Gossiping {
        /// The network declared by the remote preamble.
        remote_network: Network,
        /// The local peer's network.
        local_network: Network,
        /// The local greeting frontier's event floor, for diagnostics.
        local_min_events: Count,
    },
}

/// Validate the greeting for each remote session role.
impl RemoteRole {
    /// Enforce the role's network and causal-history requirements.
    fn validate(self, remote_version: &Version) -> Result<(), Error> {
        match self {
            RemoteRole::Providing => Ok(()),
            RemoteRole::Bootstrapping => bootstrap_claimant_is_newborn(remote_version),
            RemoteRole::Gossiping {
                remote_network,
                local_network,
                local_min_events,
            } if remote_network != local_network => Err(Error::Mismatch(Mismatch::Network {
                local_network,
                local_min_events,
                remote_network,
                remote_min_events: remote_version.min_ticks(),
            })),
            RemoteRole::Gossiping { .. } => Ok(()),
        }
    }
}

impl<'a> Reconciliation<'a> {
    /// Reconcile content and return the pipe's halves, with any byte read
    /// ahead of the closing items retained.
    ///
    /// Box the protocol future at this non-generic boundary so consumers do
    /// not instantiate its state machine. Prevent inlining from moving that
    /// erasure back into downstream code.
    #[inline(never)]
    fn reconcile(self) -> BoxFuture<'a, Result<Reconciled<'a>, Error>> {
        Box::pin(self.session())
    }

    /// The reconciliation [`reconcile`](Self::reconcile) boxes, unboxed, so
    /// that a test can read its size from its type.
    async fn session(self) -> Result<Reconciled<'a>, Error> {
        let Self {
            root,
            read,
            write,
            codec,
            keys,
            sync_memory_budget,
            run_budget,
            write_batch,
            stats,
            observe,
            remote_role,
        } = self;
        let local: LocalParticipant = materialized::Handshaking::start(
            Local,
            root.into(),
            keys.clone(),
            widen(run_budget.bytes()),
        )
        .budget(sync_memory_budget.session(run_budget).ledger)
        .stats(stats.clone());
        let proxy: RemoteParticipant<'a> =
            streaming_remote::Handshaking::start(Local, read, write, codec, keys)
                .stats(stats)
                .observe(observe)
                .write_batch(write_batch);
        let handshaken = streaming::handshake(local, proxy)
            .await
            .map_err(streaming_remote::streaming_error(Phase::Greeting))?;
        remote_role.validate(handshaken.remote_version())?;
        let (root, (read, write)) = handshaken
            .reconcile()
            .await
            .map_err(streaming_remote::streaming_error(Phase::Reconciliation))?;
        Ok((root.into(), read, write))
    }
}

/// Reject a newcomer claiming causal history before reconciliation uses that
/// history to infer deletions. A valid bootstrap claimant has an empty version.
fn bootstrap_claimant_is_newborn(claimed: &Version) -> Result<(), Error> {
    if claimed.is_empty() {
        Ok(())
    } else {
        Err(Error::violation(
            Phase::Greeting,
            SessionDefect::BootstrapHistory {
                claimed_min_events: claimed.min_ticks(),
            },
        ))
    }
}

/// Identity and completion invariants checked by the session driver.
#[derive(Debug, thiserror::Error)]
enum SessionDefect {
    /// A donation overlaps identity already held by its recipient.
    #[error("retiring peer's party overlaps ours")]
    PartyOverlap,
    /// A newcomer claims events despite having no identity or history.
    #[error("bootstrap claimant declared history (at least {claimed_min_events} events)")]
    BootstrapHistory {
        /// Lower bound derived from the claimant's version.
        claimed_min_events: crate::Count,
    },
    /// A complete closing item differs from the protocol's marker.
    #[error("invalid completion marker: {0:02x?}")]
    CompletionMarker([u8; 2]),
}

/// Finish the session with the closing exchange, read through the pipe's
/// read half.
///
/// The pipe consumes exactly the bytes of each item it reads, and only
/// looks at the next item's first byte in the read half's buffer, so after
/// the epilogue the read half rests exactly at the session boundary, any
/// bytes beyond it still in its buffer.
async fn finish_session<R: AsyncBufRead + Unpin + Send>(
    read: &mut PipeRead<R>,
    write: &mut (dyn AsyncWrite + Unpin + Send + '_),
    observe: &SessionHandle,
) -> Result<(), Error> {
    epilogue(read, write, observe).await
}

/// Exchange completion markers concurrently so both sides can flush before
/// waiting for a response.
///
/// Existing replicas have committed at this point, and a joiner already
/// holds its whole party, which it keeps if the exchange fails.
async fn epilogue(
    read: &mut (dyn AsyncRead + Unpin + Send + '_),
    write: &mut (dyn AsyncWrite + Unpin + Send + '_),
    observe: &SessionHandle,
) -> Result<(), Error> {
    let send = async {
        write
            .write_all(&COMPLETION_MARKER)
            .await
            .map_err(|source| Error::transport(Phase::Completion, Op::Write, source))?;
        write
            .flush()
            .await
            .map_err(|source| Error::transport(Phase::Completion, Op::Flush, source))?;
        observe.control_sent(&COMPLETION_MARKER);
        Ok(())
    };
    let receive = async {
        let mut marker = [0u8; COMPLETION_MARKER.len()];
        read.read_exact(&mut marker)
            .await
            .map_err(|source| Error::transport(Phase::Completion, Op::Read, source))?;
        if marker != COMPLETION_MARKER {
            return Err(Error::violation(
                Phase::Completion,
                SessionDefect::CompletionMarker(marker),
            ));
        }
        observe.control_received(&marker);
        Ok(())
    };
    futures::future::try_join(send, receive)
        .await
        .map(|((), ())| ())
}

/// The event that ends the driver's idle wait, after its racing borrows end.
enum Trigger {
    /// Received initiation bytes (`true`), clean EOF (`false`), or an I/O error.
    Arrival(Result<bool, preamble::Error>),
    /// The local policy produced an item or ended.
    Tick(Option<Gossip>),
}

/// State retained between sessions; `unfold` also retains the active future.
struct Drive<'a, T: Send + Sync + 'static, B: Bookmark> {
    /// The replica served by every session on this connection.
    peer: &'a Peer<T, B>,
    /// The connection's read half, retained across session boundaries and
    /// boxed so the driver is not generic over the transport.
    read: Box<dyn AsyncBufRead + Unpin + Send + 'a>,
    /// The connection's write half, retained across session boundaries.
    write: Box<dyn AsyncWrite + Unpin + Send + 'a>,
    /// Local initiation policy; pinned independently of the driver state.
    when: futures::stream::BoxStream<'static, Gossip>,
    /// Initiation bytes consumed before the active exchange takes over.
    staged: preamble::Staged,
    /// The frontier this connection last converged on: a WhenChanged request
    /// initiates only once the local frontier differs.
    ///
    /// `None` until the first session, so a fresh driver's first policy item
    /// always initiates (the reconnect-convergence session).
    converged: Option<Version>,
    /// End after yielding the session's terminal error.
    done: bool,
}

/// Restore a bootstrap fork unless its transmission has started.
///
/// A fork restored after its removal reached storage stays unrecorded until
/// the peer's next checkpoint, so a crash before then would drop it forever.
struct ForkGuard<T: Send + Sync + 'static> {
    /// The unsent fork and its hold on reclamation.
    fork: Option<(Party, Reservation)>,
    /// The replica from which the fork was split.
    recover: watch::Sender<Inner<T>>,
}

/// Return an abandoned fork without waking content observers.
impl<T: Send + Sync + 'static> Drop for ForkGuard<T> {
    /// Rejoin the disjoint fork on cancellation, failure, or unwind.
    fn drop(&mut self) {
        if let Some((party, _reservation)) = self.fork.take() {
            // Hold the reservation until the fork is back in the writable party.
            Inner::update_party(&self.recover, |inner| {
                // This fork came from the resident party. Overlap indicates a
                // custody bug; it must not silently discard identity space.
                assert!(
                    inner.party.join(party).is_ok(),
                    "bootstrap fork overlaps its owner"
                );
            });
        }
    }
}

#[cfg(test)]
mod tests;
