//! The wire-session drivers for [`Peer`]: [`bootstrap`](Peer::bootstrap),
//! [`gossip`](crate::Rumors::gossip), and [`retire`](Peer::retire).
//!
//! Plus the
//! preamble constants every session leads with and the [`PartyGuard`]
//! that snaps a speculatively-donated party back in place on failure.

use std::pin::Pin;
use std::sync::Arc;

use before::Party;
use borsh::{BorshDeserialize, BorshSerialize};
use futures::{Stream, future::BoxFuture};
use futures_util::StreamExt;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::{Mutex, watch},
};

#[cfg(any(test, feature = "protocol-v1"))]
use crate::tree::mirror::{
    alternating::{self, local as alternating_local, remote as alternating_remote},
    framing::{FrameRead, FrameWrite},
};
use crate::tree::{self, Tree};
use crate::{Error, Network, Protocol, Version};
use crate::{
    bookmark::{Bookmark, BookmarkError, BookmarkIo, Bookmarked, NoBookmark, Persist},
    tree::mirror::{
        handshake::{self, Intent},
        party,
        streaming::{self, Local, materialized, remote as streaming_remote},
    },
};

use super::{Inner, Peer};

/// Magic bytes that open every `rumors` gossip session's preamble frame.
pub const PROTOCOL_MAGIC: [u8; 6] = *b"RUMORS";

/// A session's read half with its concrete transport type erased.
///
/// Every session entry point coerces its caller's `&mut R` to this (and its
/// `&mut W` to [`DynWrite`]) before entering a reconciliation protocol. The
/// protocol state machines carry their transport type parameters through every
/// height of the descent, so each distinct transport type would otherwise
/// re-instantiate both towers — and, because generic code monomorphizes in the
/// crate that supplies the concrete types, it would do so once per downstream
/// binary per transport. Erasing here caps that at one instantiation per
/// payload type. The price is one vtable call per `poll_read`/`poll_write`
/// beneath the framing layers, which buffer whole frames on both sides.
type DynRead<'a> = &'a mut (dyn AsyncRead + Unpin + Send + 'a);

/// A session's write half with its concrete transport type erased.
///
/// See [`DynRead`] for why the erasure exists and what it costs.
type DynWrite<'a> = &'a mut (dyn AsyncWrite + Unpin + Send + 'a);

/// The outcome of [`Peer::retire`].
///
/// Marked `must_use` because two variants carry the intact [`Peer`]: silently
/// dropping the result of a declined or recovered retirement destroys the
/// identity that the call was specifically trying to preserve.
#[must_use = "a declined or recovered retirement hands the Peer back; dropping it leaks the identity"]
#[derive(Debug)]
pub enum Retire<T, B: BookmarkError = NoBookmark> {
    /// **Retired.** The peer reconciled with us and absorbed our identity;
    /// this replica has left the universe.
    Retired,
    /// **Declined, unchanged.** The peer was itself retiring, so nothing our
    /// replica is handed back intact, to try retiring elsewhere.
    Declined {
        /// The intact retiree.
        peer: Peer<T, B>,
    },
    /// **Recovered, unchanged.** The session failed *before* our identity ever
    /// crossed the wire; the replica is handed back intact, to try retiring
    /// elsewhere.
    Recovered {
        /// The intact retiree.
        peer: Peer<T, B>,
        /// What failed the session.
        error: Error<B>,
    },
    /// **Uncertain.** The session failed while our identity itself was in
    /// flight: the peer may or may not hold it, so our peer is consumed
    /// rather than risk the same identity living twice.
    Uncertain {
        /// What failed the session.
        error: Error<B>,
    },
}

/// The failure outcome of [`Peer::bookmark`].
///
/// This indicates that the bookmark could not be read or persisted, so the
/// [`Peer`] is handed back unchanged and still unbookmarked.
///
/// Marked `must_use` because dropping it discards the [`Peer`], the very
/// identity the failed call was trying to make durable. Take
/// [`peer`](Self::peer) back to drop it deliberately or to retry.
#[must_use = "a failed `Peer::bookmark` hands the `Peer` back; dropping it strands the identity"]
#[derive(Debug)]
pub struct Unbookmarked<T, B: BookmarkError> {
    /// The peer, its identity intact and no bookmark attached.
    pub peer: Peer<T, NoBookmark>,
    /// What the bookmark's [`load`](crate::Bookmark::load) or
    /// [`store`](crate::Bookmark::store) reported, or the framing failure the
    /// crate hit reading the stored bytes.
    pub error: BookmarkIo<B::Error>,
}

/// One completed session of [`gossip_when`](crate::Rumors::gossip_when).
///
/// The output stream from [`gossip_when`](crate::Rumors::gossip_when) yields
/// one of these each time a successful gossip session occurs (a failed session
/// is the stream's terminal `Err`).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Gossiped {
    /// The causal frontier the two replicas converged on.
    ///
    /// At the instant the session committed, both held exactly this version.
    pub converged: Version,
    /// Which trigger initiated the session on this side.
    pub led: Led,
}

/// Which side initiated a round of gossip during
/// [`gossip_when`](crate::Rumors::gossip_when).
///
/// The session protocol itself is symmetric, and when both sides' triggers fire
/// close together, each side may record `Local` for what becomes one session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Led {
    /// The `when` stream yielded `()`: this side initiated.
    Local,
    /// The remote's preamble arrived first: this side responded.
    Remote,
}

impl<T> Peer<T, NoBookmark> {
    /// Run bootstrap over any asynchronous transport pair.
    ///
    /// A thin generic funnel: the only monomorphized-per-transport code is
    /// the unsized coercion to [`DynRead`]/[`DynWrite`] here.
    pub(crate) fn bootstrap_inner<'a, R, W>(
        protocol: Protocol,
        read: &'a mut R,
        write: &'a mut W,
    ) -> BoxFuture<'a, Result<Option<Self>, Error>>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'static,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        Self::bootstrap_erased(protocol, read, write)
    }

    /// The transport-erased bootstrap body behind [`bootstrap_inner`].
    ///
    /// [`bootstrap_inner`]: Self::bootstrap_inner
    fn bootstrap_erased<'a>(
        protocol: Protocol,
        read: DynRead<'a>,
        write: DynWrite<'a>,
    ) -> BoxFuture<'a, Result<Option<Self>, Error>>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'static,
    {
        Box::pin(async move {
            // Magic/version/network/intent preamble first, before either protocol
            // is allowed to trust peer-declared frame lengths.
            let mut staged = handshake::Staged::new();
            let remote = handshake::preamble(
                protocol,
                Network::BOOTSTRAP,
                Intent::Remain,
                &mut staged,
                read,
                write,
            )
            .await
            .map_err(Error::from)?;

            // In the bootstrap case, it doesn't matter whether the remote intends
            // to remain or retire; they will hand us a party regardless, and we can
            // absorb it.
            let _ = remote.intent;

            // Reconcile from an empty tree using the selected wire protocol. Both
            // branches return the same lifecycle boundary: a materialized root and
            // the raw reader positioned at the trailing party frame.
            // `BoxFuture` is the compile-time boundary: `Box::pin` alone would
            // allocate the state while still exposing its enormous concrete type.
            #[allow(clippy::type_complexity)]
            let reconcile: BoxFuture<
                '_,
                Result<Option<(tree::Root<T>, DynRead<'a>)>, Error>,
            > = match protocol {
                Protocol::V2 => Box::pin(async move {
                    let local_root: streaming::Root<Local, T> = tree::Root::default().into();
                    let local = materialized::Handshaking::start(Local, local_root);
                    let proxy = streaming_remote::Handshaking::start(Local, read, write);
                    let handshaken = streaming::handshake(local, proxy)
                        .await
                        .map_err(streaming_error)?;
                    if remote.network.is_bootstrap() {
                        return Ok(None);
                    }
                    let descent: BoxFuture<'_, _> = Box::pin(handshaken.reconcile());
                    let (root, (read, _write)) = descent.await.map_err(streaming_error)?;
                    Ok(Some((root.into(), read)))
                }),
                #[cfg(any(test, feature = "protocol-v1"))]
                Protocol::V1 => Box::pin(async move {
                    let local = alternating_local::Exchange::start(tree::Root::default());
                    let proxy = alternating_remote::Exchange::start(
                        FrameRead::new(read),
                        FrameWrite::new(write),
                    );
                    let handshaken = alternating::handshake(local, proxy)
                        .await
                        .map_err(alternating_error)?;
                    if remote.network.is_bootstrap() {
                        return Ok(None);
                    }
                    let descent: BoxFuture<'_, _> = Box::pin(handshaken.reconcile());
                    let (root, (read, _write)) = descent.await.map_err(alternating_error)?;
                    Ok(Some((root, read.into_inner())))
                }),
            };
            let Some((root, read)) = reconcile.await? else {
                return Ok(None);
            };
            let party = party::receive(read).await?;
            let peer = Self {
                network: remote.network,
                protocol,
                inner: watch::Sender::new(Inner {
                    party: Some(party),
                    tree: Tree { root },
                }),
                bookmark: Arc::new(Mutex::new(Bookmarked::new(NoBookmark))),
            };
            Ok(Some(peer))
        })
    }

    /// Attach and eagerly persist an asynchronous bookmark.
    pub(crate) async fn bookmark_inner<B: Persist>(
        self,
        bookmark: B,
    ) -> Result<Peer<T, B>, Unbookmarked<T, B>> {
        let Peer {
            network,
            protocol,
            inner,
            ..
        } = self;
        let peer = Peer {
            network,
            protocol,
            inner,
            bookmark: Arc::new(Mutex::new(Bookmarked::new(bookmark))),
        };

        // A pristine seed has no identity worth recording yet; persisting it
        // would only force a write the lazy load already defers. Anything the
        // peer *knows* (any messages advancing the version, or a
        // forked/absorbed identity) must be made durable immediately.
        let pristine = {
            let inner = peer.inner.borrow();
            inner.tree.latest().is_empty() && inner.party.as_ref().is_some_and(Party::is_seed)
        };
        if pristine {
            return Ok(peer);
        }

        // Eagerly persist our own identity. `bookmark_record` never reclaims, so
        // it never grows the live party: on failure it has discarded the
        // in-memory record (nothing reached storage) and left the party exactly
        // as it was, so the handed-back peer is genuinely untouched.
        match peer.bookmark_record().await {
            Ok(()) => Ok(peer),
            Err(error) => Err(Unbookmarked {
                peer: Peer {
                    network: peer.network,
                    protocol: peer.protocol,
                    inner: peer.inner,
                    bookmark: Arc::new(Mutex::new(Bookmarked::new(NoBookmark))),
                },
                error,
            }),
        }
    }
}

// `Persist` is the crate-internal decoded driver, but it constrains `B` in the
// public `Peer<T, B>` self type. Every method here is crate-private; public
// entry points bind the public `Bookmark` trait.
#[allow(private_bounds)]
impl<T, B: Persist> Peer<T, B> {
    /// Retire this rumor set into a remote peer, handing it our identity so
    /// that it can be recycled by the network.
    ///
    /// The session begins with a round of gossip: the two peers reconcile
    /// content exactly as [`gossip`](crate::Rumors::gossip) would, so
    /// everything we hold that the peer had not yet seen survives in it; the
    /// peer then absorbs our identity. A peer running ordinary gossip absorbs a
    /// retiree transparently, so the counterparty needs no special call. The
    /// four outcomes are the [`Retire`] variants; see each for what survived.
    ///
    /// The gossip round writes back into the retiring set too: observers of a
    /// retiring set ([`UnorderedMessages`](crate::UnorderedMessages),
    /// [`CausalMessages`](crate::CausalMessages)) drain the *reconciled* final
    /// state — everything the session learned included — before they end.
    ///
    /// The shared transactional body behind [`retire`](Peer::retire).
    pub(crate) async fn retire_inner<'a, R, W>(
        self,
        read: &'a mut R,
        write: &'a mut W,
    ) -> Retire<T, B>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'static,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        let mut staged = handshake::Staged::new();
        match self
            .gossip_inner(Intent::Retire, &mut staged, read, write)
            .await
        {
            (Intent::Retire, Ok(_)) => Retire::Retired,
            (Intent::Retire, Err(error)) => Retire::Uncertain { error },
            (Intent::Remain, Ok(_)) => Retire::Declined { peer: self },
            (Intent::Remain, Err(error)) => Retire::Recovered { peer: self, error },
        }
    }

    /// Gossip with a remote peer to synchronize rumor sets.
    pub(crate) async fn gossip<'a, R, W>(
        &self,
        read: &'a mut R,
        write: &'a mut W,
    ) -> Result<(), Error<B>>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'static,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        let mut staged = handshake::Staged::new();
        self.gossip_inner(Intent::Remain, &mut staged, read, write)
            .await
            .1
            .map(|_converged| ())
    }

    /// Durably record this peer's *own* identity at its current version, without
    /// reclaiming anything: the attach-time persist behind
    /// [`bookmark`](Peer::bookmark).
    ///
    /// Unlike [`bookmark_update`](Self::bookmark_update), this never grows the
    /// live party — it only notes who we are, so a freshly received fork cannot
    /// strand on an early crash — and so a failed [`write`](Bookmarked::write)
    /// leaves the party exactly as it was. Reclaiming, with its party growth
    /// and the gating that protects it, is left to the first gossip. Holds the
    /// bookmark mutex across a brief `watch` borrow (read-only here) and the
    /// write; lock order is bookmark-then-`watch`, as everywhere.
    async fn bookmark_record(&self) -> Result<(), BookmarkIo<B::Error>> {
        let mut bookmark = self.bookmark.lock().await;
        bookmark.ensure_loaded().await?;
        {
            let inner = self.inner.borrow();
            if let Some(party) = inner.party.as_ref() {
                bookmark.record(self.network, party, inner.tree.latest());
            }
        }
        bookmark.write().await
    }

    /// Reflect the live identity into the bookmark before a session transmits
    /// versioned state: reclaim every stranded identity the party has caught
    /// up to (growing the live party in place) and persist.
    ///
    /// Holds the bookmark mutex across a brief `watch` critical section — where
    /// the party grows atomically with the record — and the persisting write,
    /// so the two stores never diverge. The lock order is always
    /// bookmark-then-`watch`; no path takes them the other way, so it cannot
    /// deadlock.
    ///
    /// Suppressed when the live `(party, version)` still matches what was last
    /// persisted: between updates nothing else touches the record, so re-running
    /// would reclaim nothing and re-record an identical alias. A change to
    /// *either* — the version advancing on new content, or the party growing on
    /// an absorbed retiree — defeats the suppression and persists afresh.
    async fn bookmark_update(&self) -> Result<(), BookmarkIo<B::Error>> {
        let mut bookmark = self.bookmark.lock().await;

        // Read the live frontier and party under one `watch` borrow, dropped
        // before any `send_if_modified` (holding it across one would deadlock).
        let (version, suppressed) = {
            let inner = self.inner.borrow();
            let version = inner.tree.latest().clone();
            let suppressed = inner
                .party
                .as_ref()
                .is_some_and(|party| bookmark.is_current(party, &version));
            (version, suppressed)
        };
        if suppressed {
            return Ok(());
        }

        bookmark.ensure_loaded().await?;
        self.inner.send_if_modified(|inner| {
            if let Some(party) = inner.party.as_mut() {
                // `reclaim` stages the suppression token for this
                // `(party, version)`; the `write` below commits it (or, on
                // failure, clears it so the next update retries).
                bookmark.reclaim(self.network, party, &version);
            }
            // Reclaiming widens the party's id-region but records no new event,
            // so the observable frontier is unchanged: no observer wakeup is due.
            false
        });
        bookmark.write().await
    }

    /// Slice a donated `party` out of the bookmark before it crosses the wire,
    /// and persist. The party has already left `Inner` (forked off or taken
    /// whole), so this needs no `watch` critical section.
    async fn bookmark_donate(&self, party: &Party) -> Result<(), BookmarkIo<B::Error>> {
        let mut bookmark = self.bookmark.lock().await;
        bookmark.ensure_loaded().await?;
        // Donating shrinks our identity, so `slice` invalidates the suppression
        // token; the next update re-records the true current identity.
        bookmark.slice(self.network, party);
        bookmark.write().await
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
    ///
    /// On success, returns the *converged* version: the causal frontier of
    /// the reconciled tree both replicas now hold, before any commits that
    /// ran concurrently with the session. [`gossip_when`] records it as the
    /// suppression token — "the local frontier has advanced" means exactly
    /// "latest no longer equals this".
    ///
    /// `staged` is the remote preamble's staging buffer, usually empty; a
    /// [`gossip_when`] driver hands one that may already hold part (or all)
    /// of the remote's greeting.
    ///
    /// [`gossip_when`]: crate::Rumors::gossip_when
    ///
    /// Takes the transport pre-erased ([`DynRead`]/[`DynWrite`]): every
    /// generic caller funnels through here, so the protocol towers this
    /// drives instantiate once per payload type, not once per transport.
    async fn gossip_inner<'a>(
        &self,
        intent: Intent,
        staged: &mut handshake::Staged,
        read: DynRead<'a>,
        write: DynWrite<'a>,
    ) -> (Intent, Result<Version, Error<B>>)
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'static,
    {
        // Magic/version preamble: reject a non-rumors or incompatible peer
        // before the framing trusts any peer-supplied frame length.
        let remote =
            match handshake::preamble(self.protocol, self.network, intent, staged, read, write)
                .await
            {
                Err(error) => return (Intent::Remain, Err(Error::from(error).widen())),
                Ok(remote) => remote,
            };
        let peer_bootstrapping = remote.network.is_bootstrap();
        let self_retiring = intent == Intent::Retire;
        let peer_retiring = remote.intent == Intent::Retire;

        // Stop cleanly, early if we're both trying to retire into each other
        if self_retiring && peer_retiring {
            let unchanged = self.inner.borrow().tree.latest().clone();
            return (Intent::Remain, Ok(unchanged));
        }

        // Bookmark our identity at its current version before any of it crosses
        // the wire, reclaiming any stranded identities we have since caught up
        // to. Reclaiming grows the live party in place, so it must precede the
        // speculative fork below: a fork or donation then carries the grown
        // identity. (`retire` reaches here too, through its `gossip_inner`
        // call, so a retiring set is bookmarked before donating itself.)
        if let Err(e) = self.bookmark_update().await {
            return (Intent::Remain, Err(Error::Bookmark(e)));
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
                // We only can have our hands on a `Peer` when there are no
                // extant `Rumors`, which means that we aren't stepping on
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

        // Reconcile using this peer's selected protocol. Both branches meet at
        // the lifecycle boundary the surrounding transaction needs: a local
        // root plus raw transport halves positioned after reconciliation.
        // The explicit `BoxFuture` coercion prevents either concrete protocol
        // state machine from becoming part of this outer session future.
        let network = self.network;
        #[allow(clippy::type_complexity)]
        let reconcile: BoxFuture<
            '_,
            Result<(tree::Root<T>, DynRead<'a>, DynWrite<'a>), Error>,
        > = match self.protocol {
            Protocol::V2 => Box::pin(async move {
                let local = materialized::Handshaking::start(Local, prior_tree.root.into());
                let proxy = streaming_remote::Handshaking::start(Local, read, write);
                let handshaken = streaming::handshake(local, proxy)
                    .await
                    .map_err(streaming_error)?;
                if !peer_bootstrapping && remote.network != network {
                    return Err(Error::NetworkMismatch {
                        remote_network: remote.network,
                        remote_min_events: handshaken.peer().version.min_ticks(),
                    });
                }
                let descent: BoxFuture<'_, _> = Box::pin(handshaken.reconcile());
                let (root, (read, write)) = descent.await.map_err(streaming_error)?;
                Ok((root.into(), read, write))
            }),
            #[cfg(any(test, feature = "protocol-v1"))]
            Protocol::V1 => Box::pin(async move {
                let local = alternating_local::Exchange::start(prior_tree.root);
                let proxy = alternating_remote::Exchange::start(
                    FrameRead::new(read),
                    FrameWrite::new(write),
                );
                let handshaken = alternating::handshake(local, proxy)
                    .await
                    .map_err(alternating_error)?;
                if !peer_bootstrapping && remote.network != network {
                    return Err(Error::NetworkMismatch {
                        remote_network: remote.network,
                        remote_min_events: handshaken.peer().version.min_ticks(),
                    });
                }
                let descent: BoxFuture<'_, _> = Box::pin(handshaken.reconcile());
                let (root, (read, write)) = descent.await.map_err(alternating_error)?;
                Ok((root, read.into_inner(), write.into_inner()))
            }),
        };
        let (root, read, write) = match reconcile.await {
            Ok(reconciled) => reconciled,
            Err(error) => return (Intent::Remain, Err(error.widen())),
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
            absorbed = match party::receive(read).await {
                Err(e) => return (Intent::Remain, Err(e.widen())),
                Ok(donated_party) => Some(donated_party),
            };
        } else if guarded.party.is_some() {
            // We are donating: our whole party if we are retiring, or a fresh
            // fork of it if the peer is bootstrapping from us.
            //
            // First slice the donation out of the bookmark, while it is still
            // held in the guard: if persisting fails we abort *before* the
            // party crosses the wire, and the guard re-joins it on the way out,
            // so a bookmark failure here never strands a region.
            let donated = guarded.party.as_ref().expect("is_some");
            if let Err(e) = self.bookmark_donate(donated).await {
                return (Intent::Remain, Err(Error::Bookmark(e)));
            }

            // Now take it out of the guard, defusing drop-recovery: from here
            // the peer may hold the party even if the send errors, so it can
            // never be safely re-joined.
            let donated = guarded.party.take().expect("is_some");
            match party::send(donated, write).await {
                Err(e) => {
                    // A retiring donation in limbo must be assumed received:
                    // report `Intent::Retire` alongside the error so that the
                    // `Peer` is not handed back. A lost fork merely leaks its
                    // region; we remain.
                    let outcome = if self_retiring {
                        Intent::Retire
                    } else {
                        Intent::Remain
                    };
                    return (outcome, Err(e.widen()));
                }
                Ok(()) => {
                    if self_retiring {
                        // The point of no return: the peer holds our whole
                        // party, so this `Peer` must not survive the session.
                        outcome = Intent::Retire;
                    }
                }
            }
        }

        // Write back our (potentially changed) tree and any party absorbed
        // from a retiring peer, notifying when either changes. An overlapping
        // donated party is a protocol violation: we leave our own party
        // untouched, commit nothing, and abort the session.
        //
        // The reconciled tree's frontier is the converged version: what both
        // replicas hold the instant this commits, *before* the join below
        // mixes in any commits that ran concurrently with the session.
        let merged = Tree { root };
        let converged = merged.latest().clone();
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
                    // Unreachable in practice: we hold a live `Peer` and are
                    // not retiring, so our party is present. Adopting the
                    // donation keeps the arm total without a panic path.
                    None => inner.party = Some(party),
                }
            }

            // Join the tree we got via gossip: a synchronous, in-memory
            // merge, run directly inside the critical section, as in `send`
            // and `redact`.
            let prior_hash = inner.tree.hash();
            inner.tree.join(merged);

            // We've modified the watch if the peer retired or the tree changed
            peer_retiring || prior_hash != inner.tree.hash()
        });
        if party_overlap {
            return (Intent::Remain, Err(Error::PartyOverlap));
        }

        // Persist an absorbed retiree's identity before declaring success. The
        // join above grew our live party in memory only; the retiree has
        // already sliced that region out of its own bookmark, so until we write
        // it down a crash here would strand it — held by no one, recorded
        // nowhere. This mirrors the eager persist `Peer::bookmark` does when a
        // freshly bootstrapped fork is bookmarked. A failed write surfaces as
        // an error rather than a silent leak: our caller learns the absorption
        // is not yet durable.
        if peer_retiring && let Err(e) = self.bookmark_update().await {
            return (Intent::Remain, Err(Error::Bookmark(e)));
        }

        // In the case where we successfully retired (only callable on the
        // !Clone `Peer<T>`), we've given away our inner party and no more
        // actions are possible, so don't hand back the `Peer`.
        (outcome, Ok(converged))
    }
}

impl<T, B: Bookmark> Peer<T, B> {
    /// Run the change-driven gossip driver behind
    /// [`Rumors::gossip_when`](crate::Rumors::gossip_when); the public
    /// contract lives there.
    pub(crate) fn gossip_when<'a, R, W, S>(
        &'a self,
        when: S,
        read: &'a mut R,
        write: &'a mut W,
    ) -> impl Stream<Item = Result<Gossiped, Error<B>>> + Unpin + 'a
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'static,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
        S: Stream<Item = ()> + 'a,
    {
        // The transport erases here ([`DynRead`]'s contract); `when` stays
        // generic because erasing it would cost callers the stream's
        // auto-`Send`, and the driver below is all that re-instantiates.
        let drive = Drive {
            peer: self,
            read: read as DynRead<'a>,
            write: write as DynWrite<'a>,
            when: Box::pin(when),
            staged: handshake::Staged::new(),
            converged: None,
            done: false,
        };
        // Boxed for two reasons: the box makes the returned stream `Unpin`
        // (callers consume it directly, no `pin!` ceremony), and it moves
        // the driver's in-flight session future — a large state machine —
        // off the caller's stack.
        Box::pin(futures_util::stream::unfold(
            drive,
            |mut drive| async move {
                if drive.done {
                    return None;
                }
                loop {
                    // Wait for a reason to enter a session: the remote's
                    // preamble arriving, or the `when` stream yielding a tick.
                    // The staging buffer keeps the arrival's progress outside
                    // the racing futures, so the losing arm loses no bytes.
                    let trigger = {
                        tokio::select! {
                            arrival = drive.staged.fill(&mut *drive.read) => Trigger::Arrival(arrival),
                            tick = drive.when.next() => Trigger::Tick(tick),
                        }
                    };
                    let led = match trigger {
                        Trigger::Arrival(Err(e)) => {
                            drive.done = true;
                            return Some((Err(Error::from(e).widen()), drive));
                        }
                        // A hang-up on an idle boundary — not one preamble byte
                        // arrived — is the peer's clean goodbye: end in kind.
                        // (Returning `None` is itself the unfold's terminal
                        // state; no latch needed on paths that end here.)
                        Trigger::Arrival(Ok(handshake::Fill::Closed)) => return None,
                        Trigger::Arrival(Ok(handshake::Fill::Filled)) => Led::Remote,
                        // The `when` stream is exhausted: end — after honoring
                        // a remote initiation already on the wire, whose bytes
                        // we may have consumed into the staging buffer.
                        Trigger::Tick(None) if drive.staged.is_empty() => return None,
                        Trigger::Tick(None) => {
                            drive.done = true;
                            Led::Remote
                        }
                        Trigger::Tick(Some(())) => {
                            // Suppression: a tick initiates only if the local
                            // frontier has advanced past what this connection
                            // last converged on. The comparison is local-only —
                            // it can never block learning *remote* news, which
                            // always arrives remote-led.
                            let news = {
                                let inner = drive.peer.inner.borrow();
                                drive.converged.as_ref() != Some(inner.tree.latest())
                            };
                            if !news {
                                continue;
                            }
                            Led::Local
                        }
                    };

                    let (_intent, result) = drive
                        .peer
                        .gossip_inner(
                            Intent::Remain,
                            &mut drive.staged,
                            &mut *drive.read,
                            &mut *drive.write,
                        )
                        .await;
                    return match result {
                        Ok(converged) => {
                            // Re-arm for the next session: a fresh staging
                            // buffer (this preamble is consumed) and the new
                            // suppression token.
                            drive.staged = handshake::Staged::new();
                            drive.converged = Some(converged.clone());
                            Some((Ok(Gossiped { converged, led }), drive))
                        }
                        Err(e) => {
                            drive.done = true;
                            Some((Err(e), drive))
                        }
                    };
                }
            },
        ))
    }
}

/// What woke the [`gossip_when`](Peer::gossip_when) driver out of its idle
/// select: the remote's preamble (or its absence), or the `when` stream.
///
/// Materialized so the racing borrows end before the session consumes the
/// driver's transport halves.
enum Trigger {
    Arrival(Result<handshake::Fill, handshake::Error>),
    Tick(Option<()>),
}

/// The state a [`gossip_when`](Peer::gossip_when) driver carries between
/// sessions: the transport halves, the policy stream, the preamble staging
/// buffer, and the suppression token.
struct Drive<'a, T, B: BookmarkError, S> {
    peer: &'a Peer<T, B>,
    read: DynRead<'a>,
    write: DynWrite<'a>,
    when: Pin<Box<S>>,
    staged: handshake::Staged,
    /// The frontier this connection last converged on: a tick initiates
    /// only once the local frontier differs. `None` until the first
    /// session, so a fresh driver's first tick always initiates (the
    /// reconnect-convergence session).
    converged: Option<Version>,
    /// Terminal-state latch: set on error, clean remote goodbye, or `when`
    /// exhaustion, after which the stream yields nothing further.
    done: bool,
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

/// Retain which streaming participant detected a reconciliation failure.
///
/// The local backend itself is infallible, but its materialized participant
/// can still diagnose semantic violations in peer-controlled replies. The
/// remote participant additionally retains adapter, codec, session, and
/// transport context, so neither side can be collapsed without losing useful
/// information.
fn streaming_error(
    error: tree::mirror::Error<
        materialized::Error<std::convert::Infallible>,
        streaming_remote::Error<std::convert::Infallible>,
    >,
) -> Error {
    Error::Mirror(error)
}

/// Collapse the alternating oracle's infallible local side to its wire error.
#[cfg(any(test, feature = "protocol-v1"))]
fn alternating_error(error: tree::mirror::Error<std::convert::Infallible, Error>) -> Error {
    match error {
        tree::mirror::Error::Client(never) => match never {},
        tree::mirror::Error::Server(error) => error,
    }
}
