//! The wire-session drivers for [`Peer`]: [`join`](crate::PeerBuilder::join),
//! [`gossip_stream`](crate::Peer::gossip_stream), and
//! [`retire`](Peer::retire).
//!
//! A [`ForkGuard`] restores donation forks abandoned before transmission.
//! Retirement owns its consumed `Peer` until the outcome determines whether the
//! identity can be returned to the caller.

use std::{fmt, sync::Arc, time::Instant};

use before::{Count, Party};
use futures::StreamExt;
use futures::{future::BoxFuture, stream::FusedStream};
use serde::{Serialize, de::DeserializeOwned};
use tokio::{
    io::{AsyncBufRead, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::watch,
};

use crate::error::{Mismatch, NetworkMismatch, NetworkPrecedence, Phase, TransportOperation as Op};
use crate::message::PayloadCodec;
use crate::network::NetworkKeys;
use crate::observe::{SessionCompletion, SessionHandle, SessionKind, SessionOutcome};
use crate::tree::{self, Tree};
use crate::{Error, Network, Version};
use crate::{
    bookmark::{
        BookmarkHandle, MaybeBookmark, NoBookmark, NullBookmark, Reservation, StorageIo, Token,
    },
    tree::mirror::{
        party,
        preamble::{self, Counterparty, Intent},
        secret,
        streaming::{
            self, Local, materialized,
            remote::{self as streaming_remote, PipeRead},
            stats::{Recorder, SessionStats},
        },
    },
};

use super::{Inner, Replica, SessionSizing, builder::PeerBuilder, connection::Connection};
use crate::Peer;

/// The completion marker each side writes after all of its session work, in the
/// closing exchange: the CBOR text item `"."`.
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
/// Erasure costs one virtual call per read or write poll beneath the framing
/// buffers.
type DynRead<'a> = &'a mut (dyn AsyncBufRead + Unpin + Send + 'a);

/// A session's write half with its concrete transport type erased.
///
/// See [`DynRead`] for why the erasure exists and what it costs.
type DynWrite<'a> = &'a mut (dyn AsyncWrite + Unpin + Send + 'a);

/// Reconciled content and the pipe's halves, ready for identity transfer or
/// completion.
type Reconciled<'a> = (tree::Root, PipeRead<DynRead<'a>>, DynWrite<'a>);

/// The in-process participant every reconciliation starts: the walk over the
/// local replica's tree.
type LocalParticipant = materialized::Handshaking<Local, materialized::Start>;

/// The wire-bound participant every reconciliation starts: the proxy over the
/// session's erased halves.
type RemoteParticipant<'a> = streaming_remote::Handshaking<Local, DynRead<'a>, DynWrite<'a>>;

/// The result of [`Peer::retire`].
///
/// - [`Retired`](Self::Retired) means this peer successfully retired from the
///   network, and carries a [`Connection`] that indicates whether the
///   connection can be safely reused.
/// - [`Declined`](Self::Declined) and [`Recovered`](Self::Recovered) both
///   represent failures to retire, and both return the original
///   [`Peer`], so it can retry retirement; dropping either result
///   discards that peer.
///   - [`Declined`](Retire::Declined) indicates that the counterparty was also
///     attempting to retire, which means you probably shouldn't retry retiring
///     into it again, since this error would likely occur again.
///   - [`Recovered`](Retire::Recovered) indicates that there was an error
///     during final synchronization, but before retirement completed. This
///     could be transient, so it would be reasonable to retry with the same
///     counterparty in some circumstances.
#[must_use = "retirement may return a peer that can keep gossiping or retry"]
pub enum Retire<T: Send + Sync + 'static, R, W, B: MaybeBookmark = NoBookmark> {
    /// This [`Peer`] has successfully retired from the gossip network.
    ///
    /// The peer cannot resume gossip or retry. With [`Connection::Reusable`],
    /// the remote peer confirmed completion and the session finished. With
    /// [`Connection::Lost`], the session failed sometime after retirement
    /// began, so the connection is in an unknown state and should be discarded.
    Retired {
        /// Whether the connection can carry another gossip session.
        connection: Connection<R, W, B>,
    },
    /// The remote peer was also retiring, so the session changed neither
    /// [`Peer`]. Retry by retiring into a different remote peer.
    Declined {
        /// The unchanged original [`Peer`], still un-retired and usable.
        peer: Peer<T, B>,
        /// The connection's read half, ready for another session with the
        /// remote peer (though you probably don't want this, since the remote
        /// peer was attempting to retire).
        read: R,
        /// The connection's write half, ready for another session with the
        /// remote peer (though you probably don't want this, since the remote
        /// peer was attempting to retire).
        write: W,
    },
    /// Retirement failed before the critical point of no return.
    ///
    /// The [`Peer`] may keep gossiping or retry retirement on a fresh
    /// connection. The original connection to the remote peer is failed and in
    /// an unknown state; discard it.
    Recovered {
        /// The unchanged original [`Peer`], still un-retired and usable.
        peer: Peer<T, B>,
        /// The failure which occurred.
        error: Error<B>,
    },
}

/// Format retirement outcomes without requiring any generic types to be
/// debuggable.
impl<T: Send + Sync + 'static, R, W, B: MaybeBookmark> fmt::Debug for Retire<T, R, W, B> {
    /// Formats the result and any returned peer, connection, or error.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Retired { connection } => f
                .debug_struct("Retired")
                .field("connection", connection)
                .finish(),
            Self::Declined { peer, .. } => f
                .debug_struct("Declined")
                .field("peer", peer)
                .finish_non_exhaustive(),
            Self::Recovered { peer, error } => f
                .debug_struct("Recovered")
                .field("peer", peer)
                .field("error", error)
                .finish(),
        }
    }
}

/// Information about one completed gossip session.
///
/// This is returned by [`gossip_once`](crate::Peer::gossip_once) or yielded
/// by [`gossip_stream`](crate::Peer::gossip_stream) repeatedly.
///
/// This report originates only from successful gossip sessions; failures report
/// an `Err` instead.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Gossiped {
    /// The greatest [`Version`] now known to both peers who
    /// just gossiped.
    ///
    /// Both sides absorbed all messages in the causal past or equal to this
    /// [`Version`]. Concurrency may mean that by the time the
    /// session commits, the overall [`Version`] of either side may be greater
    /// than this, due to absorbing other changes.
    pub converged: Version,
    /// The local side's perspective about which side initiated this round of
    /// gossip.
    ///
    /// Both sides may believe they initiated: if both race to initiate, the two
    /// symmetrical initiations merge into one gossip session.
    ///
    /// If the local peer reports that it [`Participant::Local`], the
    /// remote peer may or may not also believe the same.
    ///
    /// If the local peer reports that the remote
    /// [`Participant::Remote`], then the remote definitely believes
    /// that it [`Participant::Local`]; two peers cannot both
    /// simultaneously believe that the other side initiated.
    pub led: Participant,
    /// Local statistics about the gossip session, useful for reporting
    /// metrics.
    ///
    /// See [`SessionStats`] for each field's meaning.
    pub stats: SessionStats,
}

/// One of the two peers in a gossip session, seen from the local peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Participant {
    /// The local peer.
    Local,
    /// The remote peer.
    Remote,
}

/// The policy by which a stream item for
/// [`PeerBuilder::gossip_when`](crate::PeerBuilder::gossip_when) requests a gossip session.
///
/// Note that `(): Into<Gossip>` where `().into() == Gossip::WhenChanged`, so an
/// ordinary stream of `()` acts as a sequence of "ticks" which initiate gossip
/// only if there are known changes to broadcast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Gossip {
    /// Initiate a gossip session only if the local set of messages has changed
    /// since this connection last [`converged`](Gossiped::converged).
    WhenChanged,
    /// Initiate a session whether or not the local set of messages has changed.
    Unconditionally,
}

/// Unit-valued streams, [`Changes`](crate::Changes), can be used directly as
/// conditional gossip initiation policies.
impl From<()> for Gossip {
    /// Treat a change notification as a conditional initiation request by
    /// converting it into [`Gossip::WhenChanged`].
    fn from((): ()) -> Self {
        Gossip::WhenChanged
    }
}

/// What a joiner holds once its whole party has arrived: everything it needs to
/// become a peer, whether or not the completion exchange that follows succeeds.
struct Received {
    /// The network's keys, derived from the provider's sealed secret item.
    keys: Arc<NetworkKeys>,
    /// The donated identity.
    party: Party,
    /// The provider's content, reconciled into the empty tree.
    root: tree::Root,
}

/// Create joined peers and attach bookmark storage.
impl<T: Send + Sync + 'static> Replica<T, NoBookmark> {
    /// Run a join session over any connection, returning the new peer, if this
    /// side received its whole identity, beside the session's result.
    ///
    /// The peer is independent of the result: once the whole identity has
    /// arrived, this side keeps it however the session ends. No peer with `Ok`
    /// means the counterparty was joining too.
    ///
    /// A thin generic funnel: the only monomorphized-per-transport code is the
    /// erasure of the two halves here.
    pub(crate) fn join_inner<'a, R, W>(
        config: PeerBuilder<T>,
        read: &'a mut R,
        write: &'a mut W,
    ) -> BoxFuture<'a, (Option<Self>, Result<(), Error>)>
    where
        T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
        R: AsyncBufRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        Box::pin(async move {
            let policy = config.settings.gossip_policy.clone();
            let started = Instant::now();
            let completion =
                SessionCompletion::new(config.settings.observe.begin(SessionKind::Join));
            let observe = completion.handle();
            let stats = Recorder::default();
            // The peer-to-be's payload codec, built before it exists:
            // the join session's ingress decodes through it, and the
            // constructed peer inherits it.
            let codec = PayloadCodec::new::<T>(config.message_depth_limit);
            // The received party lives here, outside the session future, so
            // it survives that future's failure or its cancellation by the
            // deadline.
            let mut received = None;
            let result = policy
                .run(Self::join_erased(
                    config.settings.sizing,
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
            // `join_erased`).
            let peer = received.map(|received| Self::joined(config, codec, received));
            match &result {
                Ok(()) => {
                    // Two joining peers converge on nothing.
                    let nothing = Version::new();
                    let replica = peer.as_ref().map(|peer| peer.inner.borrow());
                    completion.finish(SessionOutcome::Completed {
                        converged: replica
                            .as_ref()
                            .map_or(&nothing, |inner| inner.tree.latest()),
                        stats: measured,
                    });
                }
                Err(error) => completion.finish(SessionOutcome::Failed(error.session_error_kind())),
            }
            (peer, result)
        })
    }

    /// The peer a join that received its whole party becomes, carrying the
    /// builder's settings.
    fn joined(config: PeerBuilder<T>, codec: PayloadCodec, received: Received) -> Self {
        let Received { keys, party, root } = received;
        let bookmark = BookmarkHandle::new(NullBookmark, config.bookmark_size_limit);
        Self {
            keys: Arc::clone(&keys),
            settings: config.settings,
            inner: watch::Sender::new(Inner::new(party, Tree::from_root(keys, root))),
            bookmark,
            codec,
        }
    }

    /// The transport-erased join body behind [`join_inner`].
    ///
    /// Leaves the joiner's network keys, party, and content in `received` once
    /// the whole party has arrived, then confirms completion. `Ok` with nothing
    /// received means the counterparty was joining too.
    ///
    /// [`join_inner`]: Self::join_inner
    fn join_erased<'a>(
        sizing: SessionSizing,
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
                Network::JOINING,
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
            // leaves it supplies. Another joining peer has no secret
            // and supplies nothing; its key item, sent as ours was, is read
            // and discarded.
            let (received_keys, remote_role) = match counterparty {
                Counterparty::Joining(item) => {
                    secret::decode_key(&item)?;
                    (None, RemoteRole::Joining)
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
            // donated identity. Two joining peers instead finish without one.
            // The received keys place each supplied leaf; without them, the
            // session admits no supply.
            let reconciliation = Reconciliation {
                root: tree::Root::default(),
                read,
                write,
                codec,
                keys: received_keys.clone(),
                sizing,
                remote_role,
                stats,
                observe: observe.clone(),
            };
            let (root, mut read, mut write) = reconciliation.reconcile().await?;
            // Two joining peers have no identity to exchange.
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
impl<T: Send + Sync + 'static, B: MaybeBookmark> Replica<T, B> {
    /// Reconcile and donate.
    ///
    /// Reconciliation publishes on both sides. Surviving message observers
    /// drain the retiring peer's final merged state before they end.
    pub(crate) async fn retire_inner<R, W>(
        &self,
        outcome: &mut Intent,
        read: &mut R,
        write: &mut W,
    ) -> Result<(), Error<B>>
    where
        R: AsyncBufRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        let mut staged = preamble::Staged::new();
        self.session(Intent::Retire, outcome, &mut staged, read, write)
            .await
            .map(drop)
    }

    /// Gossip with a remote peer to synchronize sets of messages.
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
        let result = self
            .session(
                Intent::Remain,
                &mut Intent::Remain,
                &mut staged,
                &mut read,
                &mut write,
            )
            .await;
        // A one-shot session is always locally led: the call itself is
        // this side's trigger. A remote preamble already in flight merges
        // into the same session (the preamble exchange is symmetric),
        // exactly as it does when two `gossip` triggers race.
        let (converged, stats) = result?;
        Ok((
            Gossiped {
                converged,
                led: Participant::Local,
                stats,
            },
            read,
            write,
        ))
    }

    /// Time an established peer's session while retaining its recovery outcome.
    ///
    /// The outcome must outlive the protocol future: after deadline
    /// cancellation, retirement can return the peer only if its handoff never
    /// started.
    async fn session(
        &self,
        intent: Intent,
        outcome: &mut Intent,
        staged: &mut preamble::Staged,
        read: DynRead<'_>,
        write: DynWrite<'_>,
    ) -> Result<(Version, SessionStats), Error<B>> {
        let kind = match intent {
            Intent::Remain => SessionKind::Gossip,
            Intent::Retire => SessionKind::Retire,
        };
        let started = Instant::now();
        let completion = SessionCompletion::new(self.settings.observe.begin(kind));
        let observe = completion.handle();
        let stats = Recorder::default();
        let mut result = self
            .settings
            .gossip_policy
            .run(self.gossip_inner(intent, outcome, staged, read, write, observe, stats))
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
        result
    }

    /// Reclaim caught-up identities and checkpoint our own writes when needed.
    ///
    /// Hold the bookmark mutex through persistence, taking the replica lock
    /// only to read its current version and update the party. This records all
    /// local events visible at that instant without holding the replica during
    /// I/O. Remote-only version advances leave our checkpoint current.
    async fn bookmark_update(&self) -> Result<(), StorageIo<B>> {
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
    /// `Peer`, so no writable handle can coexist with the donation. `outcome`
    /// becomes `Intent::Retire` when the peer must be consumed, even on error
    /// or timeout.
    ///
    /// Set `outcome` before first polling the identity write: after that point
    /// the recipient may hold it. The deadline wrapper owns this flag so
    /// dropping this future cannot lose the handoff decision.
    ///
    /// Success includes session statistics and the converged frontier, before
    /// concurrent local commits. The
    /// [`gossip_stream`](crate::Peer::gossip_stream) driver uses that frontier
    /// to decide whether further gossip is needed.
    ///
    /// `staged` holds any preamble bytes already read by the continuous gossip
    /// driver. The erased halves and [`Reconciliation`] keep protocol code
    /// generation independent of the caller's concrete transport type.
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
        let peer_joining = counterparty.joining();
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

        // A joining counterparty needs the network's secret before the
        // first supply arrives, to place each leaf it receives, and leaves
        // the session holding it. Its key item followed its preamble; seal
        // the secret for that key, ahead of the greeting.
        if let Counterparty::Joining(item) = &counterparty {
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
            recover: &self.inner,
        };
        let mut prior_tree = None;
        {
            let mut bookmark = self.bookmark.lock().await;
            let loaded = bookmark
                .load()
                .await
                .map_err(|error| Error::Bookmark(B::failure(error, Token(()))))?;
            let mut store = None;
            Inner::update_party(&self.inner, |inner| {
                store = loaded.checkpoint(
                    self.network(),
                    &mut inner.party,
                    inner.tree.latest(),
                    &inner.reservations,
                );
                prior_tree = Some(inner.tree.clone());
                if peer_joining && !self_retiring {
                    guarded.fork = Some(inner.reserve());
                }
            });
            if let Some(store) = store {
                store
                    .await
                    .map_err(|error| Error::Bookmark(B::failure(error, Token(()))))?;
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
            sizing: self.settings.sizing,
            stats: stats.clone(),
            observe: observe.clone(),
            remote_role: match counterparty {
                Counterparty::Joining(_) => RemoteRole::Joining,
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
            // identity. Neither side can also be holding a donation fork,
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
            // its Peer and a join's guard still owns its fork, which the
            // next checkpoint records even if the removal reached storage.
            {
                let mut bookmark = self.bookmark.lock().await;
                let loaded = bookmark
                    .load()
                    .await
                    .map_err(|error| Error::Bookmark(B::failure(error, Token(()))))?;
                let store = if self_retiring {
                    loaded.donate(self.network(), &self.inner.borrow().party)
                } else {
                    let (fork, _) = guarded.fork.as_ref().expect("donation fork");
                    loaded.donate(self.network(), fork)
                };
                store
                    .await
                    .map_err(|error| Error::Bookmark(B::failure(error, Token(()))))?;
            }

            // `send` encodes immediately and retains no party borrow. Release
            // the watch lock before I/O, or discard the fork's recovery guard.
            let send = if self_retiring {
                let inner = self.inner.borrow();
                party::send(&inner.party, write, &observe)
            } else {
                // The removal of the party is now durable, so releasing the
                // reservation can no longer permit any checkpoint offer this
                // fork across a restart. It is released here, before the
                // fork's first byte is written, so an outstanding reservation
                // always belongs to a fork never sent, which is what lets
                // reunion forget a leaked session's reservation.
                let (donated, _reservation) = guarded.fork.take().expect("donation fork");
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
            self.bookmark_update()
                .await
                .map_err(|error| Error::Bookmark(B::failure(error, Token(()))))?;
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
impl<T: Send + Sync + 'static, B: MaybeBookmark> Replica<T, B> {
    /// Run the continuous driver under this peer's initiation policy.
    #[must_use = "the driver does nothing until the returned stream is polled"]
    pub(crate) fn gossip_driver<'a, R, W>(
        &'a self,
        read: R,
        write: W,
    ) -> impl FusedStream<Item = Result<Gossiped, Error<B>>> + Unpin + 'a
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
                .settings
                .gossip_policy
                .when(crate::Changes::subscribe(&self.inner)),
            staged: preamble::Staged::new(),
            converged: None,
            done: false,
        };
        // Boxed for two reasons: the box makes the returned stream `Unpin`
        // (callers consume it directly, no `pin!` ceremony), and it moves
        // the driver's in-flight session future — a large state machine —
        // off the caller's stack. `unfold` must not be polled after it ends,
        // so it is fused inside the box, keeping the returned stream one
        // pointer.
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
                    Trigger::Arrival(Ok(true)) => Participant::Remote,
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
                        Participant::Local
                    }
                    // An unconditional request's purpose is a session that
                    // may have nothing to say: the round-trip is the
                    // liveness probe, and the convergence is the
                    // anti-entropy pull.
                    Trigger::Tick(Some(Gossip::Unconditionally)) => Participant::Local,
                };

                // `unfold` keeps the session and its deadline alive when
                // the caller drops a `next()` future between polls.
                let result = drive
                    .peer
                    .session(
                        Intent::Remain,
                        &mut Intent::Remain,
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
        })
        .fuse())
    }
}

/// One reconciliation's fully erased inputs.
///
/// Gossip and join sessions both assemble this bundle and consume it through
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
    /// Supplied leaves decode through it at wire ingress, and egress replays
    /// only bytes it previously admitted (at send, or at an earlier session's
    /// ingress). Its depth limit rides the greeting, where the counterparty's
    /// must match.
    codec: PayloadCodec,
    /// The keys of the network whose tree `root` is, or `None` for a
    /// joining peer meeting another joining peer.
    ///
    /// Their path key places supplied leaves, and their hash key is the one
    /// `root`'s nodes cache their digests under, which this side lists and
    /// compares. A joining peer meeting another holds no network secret,
    /// lists nothing, and can be supplied nothing.
    keys: Option<Arc<NetworkKeys>>,
    /// The peer's sizing as set, from which the session starts its local
    /// participant ([`SessionSizing::start`]).
    ///
    /// It also gives the bytes of frames one flush of the multiplexer may
    /// carry.
    sizing: SessionSizing,
    /// The session's stats recorder, shared by both protocol participants.
    stats: Recorder,
    /// The session's observation handle: inert unless a handler is attached,
    /// and shared, like the recorder, by every layer that moves a wire item.
    observe: SessionHandle,
    /// The remote's role and the context needed to validate its greeting.
    remote_role: RemoteRole,
}

/// How the remote peer participates in this session.
enum RemoteRole {
    /// The remote provides membership and content to a joining peer.
    Providing,
    /// The remote is joining and must have no causal history.
    Joining,
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
            RemoteRole::Joining => joiner_is_newborn(remote_version),
            RemoteRole::Gossiping {
                remote_network,
                local_network,
                local_min_events,
            } if remote_network != local_network => {
                Err(Error::Mismatch(Mismatch::Network(NetworkMismatch {
                    local: NetworkPrecedence::new(local_network, local_min_events),
                    remote: NetworkPrecedence::new(remote_network, remote_version.min_ticks()),
                })))
            }
            RemoteRole::Gossiping { .. } => Ok(()),
        }
    }
}

impl<'a> Reconciliation<'a> {
    /// Reconcile content and return the pipe's halves, with any byte read ahead
    /// of the closing items retained.
    ///
    /// Box the protocol future at this non-generic boundary so consumers do not
    /// instantiate its state machine. Prevent inlining from moving that erasure
    /// back into downstream code.
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
            sizing,
            stats,
            observe,
            remote_role,
        } = self;
        let local: LocalParticipant = sizing.start(root, keys.clone()).stats(stats.clone());
        let proxy: RemoteParticipant<'a> =
            streaming_remote::Handshaking::start(Local, read, write, codec, keys)
                .stats(stats)
                .observe(observe)
                .write_batch(sizing.write_batch);
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
/// history to infer deletions. A valid joiner claims an empty version.
fn joiner_is_newborn(claimed: &Version) -> Result<(), Error> {
    if claimed.is_empty() {
        Ok(())
    } else {
        Err(Error::violation(
            Phase::Greeting,
            SessionDefect::JoinerHistory {
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
    #[error("a newly-joining peer declared extant history (at least {claimed_min_events} events)")]
    JoinerHistory {
        /// Lower bound derived from the claimant's version.
        claimed_min_events: crate::Count,
    },
    /// A complete closing item differs from the protocol's marker.
    #[error("invalid completion marker: {0:02x?}")]
    CompletionMarker([u8; 2]),
}

/// Finish the session with the closing exchange, read through the pipe's read
/// half.
///
/// The pipe consumes exactly the bytes of each item it reads, and only looks at
/// the next item's first byte in the read half's buffer, so after the epilogue
/// the read half rests exactly at the session boundary, any bytes beyond it
/// still in its buffer.
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
/// Existing replicas have committed at this point, and a joiner already holds
/// its whole party, which it keeps if the exchange fails.
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
    /// Received initiation bytes (`true`), clean EOF (`false`), or an I/O
    /// error.
    Arrival(Result<bool, preamble::Error>),
    /// The local policy produced an item or ended.
    Tick(Option<Gossip>),
}

/// State retained between sessions; `unfold` also retains the active future.
struct Drive<'a, T: Send + Sync + 'static, B: MaybeBookmark> {
    /// The replica served by every session on this connection.
    peer: &'a Replica<T, B>,
    /// The connection's read half, retained across session boundaries and boxed
    /// so the driver is not generic over the transport.
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

/// Restore a donation fork unless its transmission has started.
///
/// A fork restored after its removal reached storage stays unrecorded until the
/// peer's next checkpoint, so a crash before then would drop it forever.
struct ForkGuard<'a, T: Send + Sync + 'static> {
    /// The unsent fork and its hold on reclamation.
    fork: Option<(Party, Reservation)>,
    /// The replica from which the fork was split.
    ///
    /// Borrowed rather than a sender of its own: every session builds this
    /// guard, and a session future leaked instead of dropped never drops it, so
    /// an owned sender would keep the replica's observers waiting after every
    /// handle is gone.
    recover: &'a watch::Sender<Inner<T>>,
}

/// Return an abandoned fork without waking content observers.
impl<T: Send + Sync + 'static> Drop for ForkGuard<'_, T> {
    /// Rejoin the disjoint fork on cancellation, failure, or unwind.
    fn drop(&mut self) {
        if let Some((party, _reservation)) = self.fork.take() {
            // Hold the reservation until the fork is back in the writable party.
            Inner::update_party(self.recover, |inner| {
                // This fork came from the resident party. Overlap indicates a
                // custody bug; it must not silently discard identity space.
                assert!(
                    inner.party.join(party).is_ok(),
                    "donation fork overlaps its owner"
                );
            });
        }
    }
}

/// Gossip.
impl<T: Send + Sync + 'static, B: MaybeBookmark> Peer<T, B> {
    /// Gossip over one connection until it ends.
    ///
    /// To observe details about each completed gossip session, use
    /// [`gossip_stream`](Self::gossip_stream) and iterate over its returned
    /// stream of [`Gossiped`] status reports.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[expect(dead_code)]
    /// # async fn example(
    /// #     rumors: &rumors::Peer<String>,
    /// #     connection: impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Unpin,
    /// # ) -> Result<(), rumors::Error> {
    /// let (read, write) = tokio::io::split(connection);
    /// let read = tokio::io::BufReader::new(read);
    ///
    /// rumors.gossip(read, write).await
    /// # }
    /// ```
    pub async fn gossip<R, W>(&self, read: R, write: W) -> Result<(), Error<B>>
    where
        R: AsyncBufRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        let mut sessions = self.gossip_stream(read, write).await;
        while let Some(session) = sessions.next().await {
            session?;
        }
        Ok(())
    }

    /// Gossip over one connection until it ends, yielding a report after each
    /// gossip session.
    ///
    /// Each successful [`Gossiped`] reports the converged [`Version`], and
    /// various statistics about the session. **You must keep polling the
    /// resultant stream to drive the connection.**
    ///
    /// To force one immediate gossip session regardless of the configured
    /// policies on the [`Peer`], use [`gossip_once`](Self::gossip_once). To
    /// drive the connection without inspecting each session, use
    /// [`gossip`](Self::gossip).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[expect(dead_code)]
    /// # async fn example(
    /// #     rumors: &rumors::Peer<String>,
    /// #     connection: impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Unpin,
    /// # ) -> Result<(), rumors::Error> {
    /// use futures::StreamExt;
    ///
    /// let (read, write) = tokio::io::split(connection);
    /// let read = tokio::io::BufReader::new(read);
    ///
    /// let mut sessions = rumors.gossip_stream(read, write).await;
    /// while let Some(session) = sessions.next().await {
    ///     let gossiped = session?;
    ///     // Inspect `gossiped` or continue driving the connection.
    ///     # let _ = gossiped;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "the driver does nothing until the returned stream is polled"]
    pub async fn gossip_stream<'a, R, W>(
        &'a self,
        read: R,
        write: W,
    ) -> impl FusedStream<Item = Result<Gossiped, Error<B>>> + Unpin + 'a
    where
        R: AsyncBufRead + Unpin + Send + 'a,
        W: AsyncWrite + Unpin + Send + 'a,
    {
        self.replica.gossip_driver(read, write)
    }

    /// Run a single gossip session with a remote peer.
    ///
    /// The configured
    /// [`PeerBuilder::gossip_timeout`](crate::PeerBuilder::gossip_timeout) applies
    /// to this exchange. The initiation policy does not: this call always starts
    /// a gossip session, regardless of
    /// [`PeerBuilder::gossip_when`](crate::PeerBuilder::gossip_when) and
    /// [`PeerBuilder::gossip_until`](crate::PeerBuilder::gossip_until).
    pub async fn gossip_once<R, W>(&self, read: R, write: W) -> Result<(Gossiped, R, W), Error<B>>
    where
        R: AsyncBufRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        self.replica.gossip_once(read, write).await
    }
}

#[cfg(test)]
mod tests;
