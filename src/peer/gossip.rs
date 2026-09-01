//! The wire-session drivers for [`Peer`]: [`bootstrap`](Bootstrap::join),
//! [`gossip`](crate::Rumors::gossip), and [`retire`](Peer::retire).
//!
//! Also here: the preamble constants every session leads with, and the
//! [`PartyGuard`] that snaps a speculatively donated party back in place
//! on failure.

use std::pin::Pin;
use std::sync::Arc;

use before::{Party, Ticks};
use futures::{Stream, future::BoxFuture};
use futures_util::StreamExt;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::{Mutex, watch},
};

use crate::link::{
    Acceptor, Connector, Link, SessionState,
    erased::{DynAcceptor, DynConnector},
};
use crate::message::PayloadCodec;
use crate::observe::{SessionHandle, SessionKind};
use crate::tree::{self, Tree};
use crate::{Error, Network, Version};
use crate::{
    bookmark::{Bookmark, BookmarkError, BookmarkIo, Bookmarked, NoBookmark, Persist},
    tree::mirror::{
        handshake::{self, Intent},
        party,
        streaming::{
            self, Local, materialized,
            remote::{self as streaming_remote, RunBudget},
            stats::{Recorder, SessionStats},
            window::WindowConfig,
        },
    },
};

use super::{Inner, Peer, bootstrap::Bootstrap};

use serde::Serialize;
use serde::de::DeserializeOwned;
/// The epilogue marker each side writes on the control stream after all
/// of its session work: the CBOR text item `"."`.
///
/// Reading the peer's marker is what lets `Ok` certify that the peer
/// completed and committed too. As an item, the marker keeps the control
/// stream a pure CBOR sequence; its leading byte is deliberately distinct
/// from the self-described tag opening a V2 preamble, so a desynchronized
/// peer that starts its next preamble where an epilogue belongs is
/// diagnosed as a protocol violation, not mistaken for completion.
const EPILOGUE_MARKER: [u8; 2] = [0x61, b'.'];

/// A session's control read half with its concrete transport type erased.
///
/// Every session entry point erases its caller's [`Link`] — the control
/// halves to this and [`DynWrite`], the stream supply to
/// [`DynConnector`]/[`DynAcceptor`] — before entering a reconciliation
/// protocol. The protocol state machines carry their transport type
/// parameters through every height of the descent, so each distinct link
/// instantiation would otherwise re-instantiate both towers — and, because
/// generic code monomorphizes in the crate that supplies the concrete types,
/// it would do so once per downstream binary per instantiation. Erasing here
/// — with the protocol bodies behind the non-generic [`Reconciliation`] and
/// bootstrap drivers — caps that at one instantiation, compiled once into
/// this crate. The price is one vtable
/// call per stream open/accept and per `poll_read`/`poll_write` beneath the
/// framing layers, which buffer whole frames on both sides.
type DynRead<'a> = &'a mut (dyn AsyncRead + Unpin + Send + 'a);

/// A session's control write half with its concrete transport type erased.
///
/// See [`DynRead`] for why the erasure exists and what it costs.
type DynWrite<'a> = &'a mut (dyn AsyncWrite + Unpin + Send + 'a);

/// One session's fully erased link parts, in [`Link`] field order: control
/// halves, connector, acceptor, and the session epoch.
///
/// The funnels produce this (via [`erase`]) and [`Peer::gossip_inner`]
/// consumes it; it stays a tuple of parts rather than an assembled [`Link`]
/// so the `gossip_when` driver can reborrow its halves one session at a
/// time.
type DynLinkParts<'a> = (DynRead<'a>, DynWrite<'a>, DynConnector, DynAcceptor<'a>, u8);

/// The outcome of [`Peer::retire`].
///
/// Marked `must_use` because two variants carry the intact [`Peer`]: silently
/// dropping the result of a declined or recovered retirement destroys the
/// identity that the call was specifically trying to preserve.
#[must_use = "a declined or recovered retirement hands the Peer back; dropping it leaks the identity"]
#[derive(Debug)]
pub enum Retire<T, B: BookmarkError = NoBookmark> {
    /// **Retired.** This replica has left the universe.
    ///
    /// The peer reconciled with us, absorbed our identity, and confirmed
    /// the absorption through the session epilogue. The link rests at a
    /// clean session boundary.
    Retired,
    /// **Declined, unchanged.** The peer was itself retiring, so nothing
    /// moved; our replica is handed back intact, to try retiring elsewhere.
    /// The session ended cleanly, so the link remains usable.
    Declined {
        /// The intact retiree.
        peer: Peer<T, B>,
    },
    /// **Recovered, unchanged.** The session failed *before* our identity
    /// ever crossed the wire; the replica is handed back intact, to try
    /// retiring elsewhere.
    ///
    /// Retry over a different link: this one is poisoned (or, on
    /// [`Error::LinkPoisoned`], already was), and its next session fails
    /// fast.
    Recovered {
        /// The intact retiree.
        peer: Peer<T, B>,
        /// What failed the session.
        error: Error<B>,
    },
    /// **Uncertain.** The session failed while our identity itself was in
    /// flight: the peer may or may not hold it, so our peer is consumed
    /// rather than risk the same identity living twice. The link is
    /// poisoned; discard it.
    ///
    /// In flight covers the party frame itself and everything after it:
    /// a failure while awaiting the peer's commit confirmation lands here
    /// too — the confirmation was lost and cannot be re-fetched
    /// ([`Error::Epilogue`] explains why that gap cannot be closed).
    Uncertain {
        /// What failed the session.
        error: Error<B>,
    },
}

/// A failed bookmark attach: the [`Peer`] handed back unchanged, still
/// unbookmarked.
///
/// The bookmark could not be read or persisted. Produced by
/// [`Peer::bookmark`] as its `Err`, and by a bookmarked bootstrap as
/// [`Joined::Unbookmarked`](super::Joined::Unbookmarked) — the same
/// failure at the same step, differing only in when the bookmark was
/// selected.
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

/// One completed gossip session: what [`gossip`](crate::Rumors::gossip)
/// returns and the [`gossip_when`](crate::Rumors::gossip_when) stream
/// yields.
///
/// One of these exists per successful session; a failed session is an
/// `Err` instead (the terminal `Err` of the `gossip_when` stream).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Gossiped {
    /// The causal frontier the two replicas converged on.
    ///
    /// At the instant the session committed, both held exactly this version.
    pub converged: Version,
    /// Which trigger initiated the session on this side.
    pub led: Led,
    /// What the session measured about itself; every count is local, so
    /// the two ends of one session report their own numbers.
    ///
    /// See [`SessionStats`] for each field's mechanism and the seam it is
    /// counted at.
    pub stats: SessionStats,
}

/// Which side initiated a round of gossip.
///
/// The session protocol itself is symmetric, and when both sides' triggers fire
/// close together, each side may record `Local` for what becomes one session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Led {
    /// This side initiated.
    ///
    /// Either its [`gossip_when`](crate::Rumors::gossip_when) `when`
    /// stream yielded a [`Gossip`] cue that initiated, or the caller
    /// invoked the one-shot [`gossip`](crate::Rumors::gossip): the call
    /// itself is the local trigger, so a one-shot session always reports
    /// `Local`.
    Local,
    /// The remote's preamble arrived first: this side responded.
    Remote,
}

/// One cue from a [`gossip_when`](crate::Rumors::gossip_when) policy stream:
/// whether this reason to gossip is conditional on local change.
///
/// The `when` stream's items convert into this (`Into<Gossip>`), and `()`
/// converts to [`WhenChanged`](Self::WhenChanged), so a
/// [`changes`](crate::Rumors::changes) stream plugs in directly as the
/// push-on-change policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gossip {
    /// Initiate a session only if the local set has changed since this
    /// connection last [`converged`](Gossiped::converged).
    ///
    /// A cue that finds nothing new costs nothing and crosses no wire; in
    /// particular, the cue a [`changes`](crate::Rumors::changes) stream
    /// fires after a session's own merge never echoes a second session
    /// back over the connection that delivered it.
    WhenChanged,
    /// Initiate a session whether or not anything changed locally.
    ///
    /// A session between already-converged replicas is a short round-trip
    /// that confirms the connection works end to end, and a session
    /// between diverged ones converges them both ways — so an interval
    /// stream of unconditional cues doubles as a liveness probe and an
    /// anti-entropy net, pulling news a remote's own policy stream stayed
    /// quiet about.
    Unconditionally,
}

/// `()` is the when-changed cue: [`changes`](crate::Rumors::changes) (and
/// any other unit stream) feeds [`gossip_when`](crate::Rumors::gossip_when)
/// the push-on-change policy without adaptation.
impl From<()> for Gossip {
    fn from((): ()) -> Self {
        Gossip::WhenChanged
    }
}

impl<T> Peer<T, NoBookmark> {
    /// Run bootstrap over any link.
    ///
    /// A thin generic funnel: the only monomorphized-per-link code is the
    /// erasure to [`DynLinkParts`] here.
    pub(crate) fn bootstrap_inner<'a, CR, CW, C, A>(
        config: Bootstrap<T>,
        link: &'a mut Link<CR, CW, C, A>,
    ) -> BoxFuture<'a, Result<Option<Self>, Error>>
    where
        T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
        CR: AsyncRead + Unpin + Send,
        CW: AsyncWrite + Unpin + Send,
        C: Connector,
        A: Acceptor,
    {
        Box::pin(async move {
            let parts = erase(&mut *link)?;
            let result = Self::bootstrap_erased(config, parts).await;
            // Un-poison on clean completion: both `Ok` arms — a completed
            // donation and a mutual-bootstrap bail — end with the epilogue
            // under V2, leaving the control stream at the session boundary.
            if result.is_ok() {
                link.session.finish();
            }
            result
        })
    }

    /// The link-erased bootstrap body behind [`bootstrap_inner`].
    ///
    /// [`bootstrap_inner`]: Self::bootstrap_inner
    fn bootstrap_erased<'a>(
        config: Bootstrap<T>,
        link: DynLinkParts<'a>,
    ) -> BoxFuture<'a, Result<Option<Self>, Error>>
    where
        T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
    {
        Box::pin(async move {
            let (read, write, connector, acceptor, epoch) = link;
            // The peer-to-be's payload codec, built before it exists:
            // the bootstrap session's ingress decodes through it, and the
            // constructed peer inherits it.
            let codec = PayloadCodec::new::<T>(config.payload_depth_limit);
            let observe = config.observe.begin(SessionKind::Bootstrap);
            // Magic/version/network/intent preamble first, before the
            // protocol is allowed to trust peer-declared frame lengths.
            let mut staged = handshake::Staged::new();
            let remote = handshake::preamble(
                Network::BOOTSTRAP,
                Intent::Remain,
                &mut staged,
                read,
                write,
                &observe,
            )
            .await
            .map_err(Error::from)?;

            // In the bootstrap case, it doesn't matter whether the remote intends
            // to remain or retire; they will hand us a party regardless, and we can
            // absorb it.
            let _ = remote.intent;

            // Reconcile from an empty tree; the lifecycle boundary is a
            // materialized root and the raw control halves positioned at
            // the trailing party frame. The protocol body is the
            // non-generic [`bootstrap_reconcile`], which returns its
            // future boxed; see [`Reconciliation::reconcile`] for the
            // boxing and inlining discipline.
            let both_bootstrapping = remote.network.is_bootstrap();
            let reconcile = bootstrap_reconcile(
                (read, write, connector, acceptor, epoch),
                codec,
                config.window,
                config.run_budget,
                both_bootstrapping,
                observe.clone(),
            );
            let Some((root, mut read, mut write)) = reconcile.await? else {
                return Ok(None);
            };
            let party = party::receive(&mut read, &observe).await?;
            // Our absorption of the received identity completes with the
            // in-memory `Peer` construction below, which cannot fail: certify
            // completion now, and require the provider's certificate so `Ok`
            // means it committed its donation. On `Err` the received fork
            // is dropped — its region leaks, benignly, like any fork lost
            // in flight.
            epilogue(&mut read, &mut write, &observe).await?;
            let peer = Self {
                network: remote.network,
                window: config.window,
                run_budget: config.run_budget,
                inner: watch::Sender::new(Inner {
                    party: Some(party),
                    tree: Tree::from_root(root),
                }),
                bookmark: Arc::new(Mutex::new(Bookmarked::new(NoBookmark))),
                codec,
                observe: config.observe,
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
            window,
            run_budget,
            inner,
            codec,
            observe,
            ..
        } = self;
        let peer = Peer {
            network,
            window,
            run_budget,
            inner,
            bookmark: Arc::new(Mutex::new(Bookmarked::new(bookmark))),
            codec,
            observe,
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
                    window: peer.window,
                    run_budget: peer.run_budget,
                    inner: peer.inner,
                    bookmark: Arc::new(Mutex::new(Bookmarked::new(NoBookmark))),
                    codec: peer.codec,
                    observe: peer.observe,
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
    /// Runs the transactional body behind [`retire`](Peer::retire).
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
    pub(crate) async fn retire_inner<CR, CW, C, A>(
        self,
        link: &mut Link<CR, CW, C, A>,
    ) -> Retire<T, B>
    where
        T: Send + Sync + 'static,
        CR: AsyncRead + Unpin + Send,
        CW: AsyncWrite + Unpin + Send,
        C: Connector,
        A: Acceptor,
    {
        let mut staged = handshake::Staged::new();
        let parts = match erase(link) {
            Ok(parts) => parts,
            // The fail-fast happened before any wire traffic: nothing of
            // ours was ever in flight, so the retiree is recovered intact.
            Err(error) => {
                return Retire::Recovered {
                    peer: self,
                    error: error.widen(),
                };
            }
        };
        let (intent, result) = self.gossip_inner(Intent::Retire, &mut staged, parts).await;
        // Un-poison on clean completion, before the outcome is shaped: every
        // `Ok` — retired, or declined by a mutually retiring peer — leaves
        // the control stream resting at the session boundary.
        if result.is_ok() {
            link.session.finish();
        }
        match (intent, result) {
            (Intent::Retire, Ok(_)) => Retire::Retired,
            (Intent::Retire, Err(error)) => Retire::Uncertain { error },
            (Intent::Remain, Ok(_)) => Retire::Declined { peer: self },
            (Intent::Remain, Err(error)) => Retire::Recovered { peer: self, error },
        }
    }

    /// Gossip with a remote peer to synchronize rumor sets.
    pub(crate) async fn gossip<CR, CW, C, A>(
        &self,
        link: &mut Link<CR, CW, C, A>,
    ) -> Result<Gossiped, Error<B>>
    where
        T: Send + Sync + 'static,
        CR: AsyncRead + Unpin + Send,
        CW: AsyncWrite + Unpin + Send,
        C: Connector,
        A: Acceptor,
    {
        let mut staged = handshake::Staged::new();
        let parts = erase(link).map_err(Error::widen)?;
        let (_intent, result) = self.gossip_inner(Intent::Remain, &mut staged, parts).await;
        // Un-poison on clean completion: the session's own `Ok` under V2 is
        // already epilogue-certified, so the control stream rests at the
        // session boundary.
        if result.is_ok() {
            link.session.finish();
        }
        // A one-shot session is always locally led: the call itself is
        // this side's trigger. A remote preamble already in flight merges
        // into the same session (the preamble exchange is symmetric),
        // exactly as it does when two `gossip_when` triggers race.
        result.map(|(converged, stats)| Gossiped {
            converged,
            led: Led::Local,
            stats,
        })
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

    /// Reflect the live identity into the bookmark and persist it.
    ///
    /// Reclaims every stranded identity the party has caught up to (growing
    /// the live party in place) and records the party at its frontier. The
    /// frontier is read *inside* the `watch` critical section the reclaim
    /// runs in, so the staged record never lags an event the caller has
    /// already committed — the record's own-party projection dominates every
    /// event that existed when the reclaim ran.
    ///
    /// Holds the bookmark mutex across that brief `watch` critical section —
    /// where the party grows atomically with the record — and the persisting
    /// write, so the two stores never diverge. The lock order is always
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
        bookmark.ensure_loaded().await?;

        let mut persist = false;
        self.inner.send_if_modified(|inner| {
            if let Some(party) = inner.party.as_mut() {
                let version = inner.tree.latest().clone();
                if !bookmark.is_current(party, &version) {
                    // `reclaim` stages the suppression token for this
                    // `(party, version)`; only the `write` below, completing
                    // `Ok`, commits it — a failed or cancelled write leaves
                    // no token, so the next update persists afresh.
                    bookmark.reclaim(self.network, party, &version);
                    persist = true;
                }
            }
            // Reclaiming widens the party's identity but records no new event,
            // so the observable frontier is unchanged: no observer wakeup is due.
            false
        });
        if persist {
            bookmark.write().await
        } else {
            Ok(())
        }
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
    /// The returned `Intent` is `Intent::Remain` whenever the provided intent
    /// was, and `Intent::Retire` *only if* the entire local party was handed
    /// off to the counterparty via retirement. `Intent::Retire` can arrive
    /// *with* an error: when sending the party itself fails, we cannot know
    /// whether the remote received it, so we must assume it might have.
    ///
    /// On success, returns the *converged* version: the causal frontier of
    /// the reconciled tree both replicas now hold, before any commits that
    /// ran concurrently with the session. The session's [`SessionStats`]
    /// ride along with it. [`gossip_when`] records the version as the
    /// suppression token for [`Gossip::WhenChanged`] cues — "the local
    /// frontier has advanced" means exactly "latest no longer equals this".
    ///
    /// `staged` is the remote preamble's staging buffer, usually empty; a
    /// [`gossip_when`] driver hands one that may already hold part (or all)
    /// of the remote's preamble.
    ///
    /// [`gossip_when`]: crate::Rumors::gossip_when
    ///
    /// Takes the link pre-erased ([`DynLinkParts`]): every generic caller
    /// funnels through here, and the reconciliation itself runs behind the
    /// non-generic [`Reconciliation`], so the protocol towers it drives
    /// codegen exactly once, in this crate.
    async fn gossip_inner<'a>(
        &self,
        intent: Intent,
        staged: &mut handshake::Staged,
        link: DynLinkParts<'a>,
    ) -> (Intent, Result<(Version, SessionStats), Error<B>>)
    where
        T: Send + Sync + 'static,
    {
        let (read, write, connector, acceptor, epoch) = link;
        let codec = self.codec;
        // The session's stats recorder: both protocol participants below
        // share it (the walk counts disputes, gains, sheds, and the
        // window grant; the proxy's codec seam counts bytes), and its
        // snapshot rides the `Ok`. A session that ends before
        // reconciliation reports zeros.
        let stats = Recorder::default();
        // The session's observation handle: inert unless a handler is
        // attached and the dialect is observable, and shared, like the
        // recorder, by every layer that moves a wire item.
        let kind = match intent {
            Intent::Remain => SessionKind::Gossip,
            Intent::Retire => SessionKind::Retire,
        };
        let observe = self.observe.begin(kind);
        // Magic/version preamble: reject a non-rumors or incompatible peer
        // before the framing trusts any peer-supplied frame length.
        let remote =
            match handshake::preamble(self.network, intent, staged, read, write, &observe).await {
                Err(error) => return (Intent::Remain, Err(Error::from(error).widen())),
                Ok(remote) => remote,
            };
        let peer_bootstrapping = remote.network.is_bootstrap();
        let self_retiring = intent == Intent::Retire;
        let peer_retiring = remote.intent == Intent::Retire;

        // Stop cleanly, early if we're both trying to retire into each other.
        // Symmetric by construction: both sides take this same branch, so the
        // epilogue markers pair up with no session body between them.
        if self_retiring && peer_retiring {
            if let Err(e) = epilogue(read, write, &observe).await {
                return (Intent::Remain, Err(e.widen()));
            }
            let unchanged = self.inner.borrow().tree.latest().clone();
            return (Intent::Remain, Ok((unchanged, stats.snapshot())));
        }

        // Reflect our identity into the bookmark, snapshot the session's
        // tree, and *speculatively* remove any party we will donate — all in
        // one `watch` critical section under the bookmark mutex. One critical
        // section carries two safety obligations at once:
        //
        // - The persisted record's own-party projection dominates the
        //   snapshot's own-party version, so every own event this session can
        //   transmit is durably accounted for before it crosses the wire, and
        //   a crash-and-reclaim can never reuse a causal coordinate some
        //   replica already holds. A `send` committed while the record's
        //   write is in flight lands *after* the snapshot: it stays out of
        //   this session and the next session's update covers it.
        //
        // - The donated party forks at the exact version the snapshot
        //   carries: no lag in which a concurrent `send` could stamp messages
        //   with a version exceeding the one communicated to a bootstrapping
        //   party, violating party disjointness. Reclaiming grows the live
        //   party in place, so it runs before the fork and a fork or donation
        //   carries the grown identity. (`retire` reaches here too, through
        //   its `gossip_inner` call, so a retiring set is bookmarked before
        //   donating itself.)
        //
        // The lock order is bookmark-then-`watch`, as everywhere. A failed
        // record write aborts the session before any wire traffic: dropping
        // `guarded` re-joins the speculative fork, and the next update
        // re-records what the reclaim already grew in memory.
        let mut guarded = PartyGuard {
            party: None,
            recover: self.inner.clone(),
        };
        let mut prior_tree = None;
        {
            let mut bookmark = self.bookmark.lock().await;
            if let Err(e) = bookmark.ensure_loaded().await {
                return (Intent::Remain, Err(Error::Bookmark(e)));
            }
            let mut persist = false;
            self.inner.send_if_modified(|inner| {
                if let Some(party) = inner.party.as_mut() {
                    let version = inner.tree.latest().clone();
                    if !bookmark.is_current(party, &version) {
                        // `reclaim` stages the suppression token for
                        // this `(party, version)`; only the `write` below,
                        // completing `Ok`, commits it — a failed or
                        // cancelled write leaves no token, so the next
                        // update persists afresh.
                        bookmark.reclaim(self.network, party, &version);
                        persist = true;
                    }
                }
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
            if persist && let Err(e) = bookmark.write().await {
                return (Intent::Remain, Err(Error::Bookmark(e)));
            }
        }
        let prior_tree = prior_tree.expect("set in closure");
        // The event floor this side's handshake declares: `prior_tree` is
        // exactly the root the local protocol participant starts from, so
        // its frontier is the version the greeting carries.
        let local_min_events = prior_tree.latest().min_ticks();

        // Reconcile using this peer's selected protocol. Both branches meet at
        // the lifecycle boundary the surrounding transaction needs: a local
        // root plus raw transport halves positioned after reconciliation.
        // The protocol bodies live behind the non-generic [`Reconciliation`],
        // whose methods return their futures boxed: neither concrete
        // protocol state machine becomes part of this outer session future,
        // or of the consumer crate that instantiates it.
        let reconciliation = Reconciliation {
            root: prior_tree.root,
            link: (read, write, connector, acceptor, epoch),
            codec,
            window: self.window,
            run_budget: self.run_budget,
            stats: stats.clone(),
            observe: observe.clone(),
            peer_bootstrapping,
            remote_network: remote.network,
            network: self.network,
            local_min_events,
        };
        let reconcile = reconciliation.reconcile();
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
            absorbed = match party::receive(read, &observe).await {
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
            match party::send(donated, write, &observe).await {
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
        let merged = Tree::from_root(root);
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
            //
            // We've modified the watch if the peer retired, the tree's
            // content changed (straight from `join`'s changed flag: no root
            // hash is read inside this critical section — `Tree::join`
            // states the flag's contract), or the causal ceiling advanced.
            // The ceiling term is ours to add, not the flag's: the flag
            // answers for the set's content, while the observer contract
            // (`Rumors::changes`: one tick per observed frontier advance)
            // counts ceiling-only advances too — a redaction's only wire
            // representation is such an advance. Computed by containment
            // before the join consumes the merged tree: the ceiling moves
            // exactly when the reconciled frontier is not already contained
            // in ours — strictly greater, or incomparable (commits that ran
            // concurrently with the session advance our frontier past the
            // session's view without containing it; the join then adds the
            // reconciled regions, so incomparable is an advance too). Both
            // frontiers are plain field reads, cheap under the lock.
            // Waking on a ceiling advance cannot loop: the driver's
            // suppression token is the converged frontier itself, so the
            // echo cue this notification queues on the session's own
            // connection is swallowed, and a fresh connection initiates once
            // and then quiesces (`tests/gossip_when.rs` pins the chain).
            // Unconditional cues bypass the token, but they come from the
            // caller's timers, never from this wake — and a session that
            // carried no news (nothing crossed, nobody retired) takes none
            // of the three wake branches below, so unconditional probing
            // feeds no wake either (pinned in the same suite).
            //
            // The join runs unconditionally — it must commit the merge even
            // when the retirement alone decides the notification.
            let ceiling_advancing = matches!(
                merged.latest().partial_cmp(inner.tree.latest()),
                None | Some(std::cmp::Ordering::Greater)
            );
            let tree_changed = inner.tree.join(merged);
            peer_retiring || tree_changed || ceiling_advancing
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

        // All local session work is done and committed: certify completion to
        // the peer and require its certificate in return, so `Ok` below means
        // the *peer* completed and committed too. This one insertion point
        // covers every side — plain gossip, serving a bootstrap, the retiree
        // (its party is sent above), and the absorber (its bookmark update is
        // committed above; under `NoBookmark` that commit is in-memory only).
        // The failure return must preserve `outcome`: a retiree whose party
        // crossed the wire but whose epilogue failed is post-hand-off, and
        // mapping it back to `Intent::Remain` would duplicate the identity.
        if let Err(e) = epilogue(read, write, &observe).await {
            return (outcome, Err(e.widen()));
        }

        // In the case where we successfully retired (only callable on the
        // !Clone `Peer<T>`), we've given away our inner party and no more
        // actions are possible, so don't hand back the `Peer`.
        (outcome, Ok((converged, stats.snapshot())))
    }
}

impl<T, B: Bookmark> Peer<T, B> {
    /// Run the cue-driven gossip driver behind
    /// [`Rumors::gossip_when`](crate::Rumors::gossip_when); the public
    /// contract lives there.
    #[must_use = "the driver does nothing until the returned stream is polled"]
    pub(crate) fn gossip_when<'a, CR, CW, C, A, S>(
        &'a self,
        when: S,
        link: &'a mut Link<CR, CW, C, A>,
    ) -> impl Stream<Item = Result<Gossiped, Error<B>>> + Unpin + 'a
    where
        T: Send + Sync + 'static,
        CR: AsyncRead + Unpin + Send,
        CW: AsyncWrite + Unpin + Send,
        C: Connector,
        A: Acceptor,
        S: Stream + 'a,
        S::Item: Into<Gossip>,
    {
        // The link erases here ([`DynRead`]'s contract); `when` stays
        // generic because erasing it would cost callers the stream's
        // auto-`Send`, and the driver below is all that re-instantiates.
        let drive = Drive {
            peer: self,
            read: &mut link.control_read as DynRead<'a>,
            write: &mut link.control_write as DynWrite<'a>,
            connector: DynConnector::new(link.connector.clone()),
            acceptor: &mut link.acceptor as DynAcceptor<'a>,
            state: &mut link.session,
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
                    // A driver started on a poisoned link must fail fast
                    // here, before the idle select: its session would fail
                    // the same way, but only once a trigger fired, leaving
                    // a driver that looks live while parked on a dead link.
                    if drive.state.poisoned() {
                        drive.done = true;
                        return Some((Err(Error::LinkPoisoned.widen()), drive));
                    }
                    // Wait for a reason to enter a session: the remote's
                    // preamble arriving, or the `when` stream yielding a tick.
                    // The staging buffer keeps the arrival's progress outside
                    // the racing futures, so the losing arm loses no bytes.
                    let trigger = {
                        tokio::select! {
                            arrival = drive.staged.fill(&mut *drive.read) => Trigger::Arrival(arrival),
                            cue = drive.when.next() => Trigger::Tick(cue.map(Into::into)),
                        }
                    };
                    let led = match trigger {
                        Trigger::Arrival(Err(e)) => {
                            // Poison even when the staging buffer is empty
                            // (zero bytes consumed): a transport that errored
                            // is not a link a later session should trust, and
                            // the contract promises every error terminal
                            // leaves the link poisoned. `Drive::drop`'s
                            // predicate covers the other case — a driver
                            // dropped with staged bytes it never replayed.
                            drive.state.poison();
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
                        Trigger::Tick(Some(Gossip::WhenChanged)) => {
                            // Suppression: a when-changed cue initiates only
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
                        // An unconditional cue's whole job is a session that
                        // may have nothing to say: the round-trip is the
                        // liveness probe, and the convergence is the
                        // anti-entropy pull.
                        Trigger::Tick(Some(Gossip::Unconditionally)) => Led::Local,
                    };

                    let epoch = match drive.state.begin() {
                        Ok(epoch) => epoch,
                        Err(e) => {
                            drive.done = true;
                            return Some((Err(e.widen()), drive));
                        }
                    };
                    let (_intent, result) = drive
                        .peer
                        .gossip_inner(
                            Intent::Remain,
                            &mut drive.staged,
                            (
                                &mut *drive.read,
                                &mut *drive.write,
                                drive.connector.clone(),
                                &mut *drive.acceptor,
                                epoch,
                            ),
                        )
                        .await;
                    return match result {
                        Ok((converged, stats)) => {
                            // Re-arm for the next session: un-poison the
                            // link (this session completed cleanly), a
                            // fresh staging buffer (this preamble is
                            // consumed), and the new suppression token.
                            drive.state.finish();
                            drive.staged = handshake::Staged::new();
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
            },
        ))
    }
}

/// One gossip reconciliation's inputs, fully erased.
///
/// [`Peer::gossip_inner`] assembles this and immediately consumes it through
/// [`reconcile`](Self::reconcile). The struct exists so that body is
/// non-generic: `gossip_inner` is generic over the payload and bookmark
/// types, and a reconciliation written inline there would monomorphize the
/// entire protocol tower it drives into every consumer crate, once per
/// instantiation. Behind this boundary the tower codegens exactly once, into
/// this crate's own object code.
struct Reconciliation<'a> {
    /// The local replica's root, snapshotted inside the session transaction's
    /// critical section: exactly what the local participant reconciles from.
    root: tree::Root,
    /// The session's erased link.
    link: DynLinkParts<'a>,
    /// The peer's payload codec: the payload boundary in both directions.
    ///
    /// Supplied leaves decode through it at wire ingress, and egress
    /// replays only bytes it previously admitted (at send, or at an
    /// earlier session's ingress). Its depth limit rides the greeting,
    /// where the counterparty's must match.
    codec: PayloadCodec,
    /// The window policy the session negotiates under.
    window: WindowConfig,
    /// The supply-run sizing budget; its byte target rides the greeting.
    run_budget: RunBudget,
    /// The session's stats recorder, shared by both protocol participants.
    stats: Recorder,
    /// The session's observation handle: inert unless a handler is
    /// attached, and shared, like the recorder, by every layer that
    /// moves a wire item.
    observe: SessionHandle,
    /// Whether the remote's preamble declared it a bootstrap claimant.
    peer_bootstrapping: bool,
    /// The network the remote's preamble declared.
    remote_network: Network,
    /// The local network, which a non-bootstrapping remote must match.
    network: Network,
    /// The local greeting frontier's event floor, for the mismatch report.
    local_min_events: Ticks,
}

impl<'a> Reconciliation<'a> {
    /// Drive one streaming reconciliation to the lifecycle boundary the
    /// session transaction resumes from: the reconciled local root plus the
    /// raw control halves, positioned after the descent.
    ///
    /// Returns the future boxed: an `async fn` body is codegen'd into
    /// whichever crate polls it, so a bare future here would hand the whole
    /// protocol state machine right back to every consumer. The `dyn`
    /// coercion pins it — vtable, poll, and everything the body awaits — in
    /// this crate's own object code. `inline(never)` guards the same
    /// boundary in optimized builds: the shell is small enough for rustc's
    /// automatic cross-crate MIR inlining, which would move the coercion —
    /// and the tower behind it — back into the consumer.
    #[inline(never)]
    fn reconcile(self) -> BoxFuture<'a, Result<(tree::Root, DynRead<'a>, DynWrite<'a>), Error>> {
        Box::pin(async move {
            let Self {
                root,
                link,
                codec,
                window,
                run_budget,
                stats,
                observe,
                peer_bootstrapping,
                remote_network,
                network,
                local_min_events,
            } = self;
            let (read, write, connector, acceptor, epoch) = link;
            let local = materialized::Handshaking::<_, _>::start(Local, root.into())
                .window(window)
                .target_message_size(run_budget.bytes() as u64)
                .stats(stats.clone());
            let carrier = Link::for_session(read, write, connector, acceptor, epoch);
            let proxy = streaming_remote::Handshaking::start(Local, carrier, codec)
                .window(window)
                .stats(stats)
                .observe(observe);
            let handshaken = streaming::handshake(local, proxy)
                .await
                .map_err(streaming_error)?;
            if peer_bootstrapping {
                bootstrap_claimant_is_newborn(&handshaken.peer().version)?;
            } else if remote_network != network {
                return Err(Error::NetworkMismatch {
                    remote_network,
                    remote_min_events: handshaken.peer().version.min_ticks(),
                    local_min_events,
                });
            }
            let descent: BoxFuture<'_, _> = Box::pin(handshaken.reconcile());
            let (root, (read, write)) = descent.await.map_err(streaming_error)?;
            Ok((root.into(), read, write))
        })
    }
}

/// Drive one bootstrap reconciliation from an empty local replica.
///
/// Free and non-generic for the same reason [`Reconciliation`] is: the
/// generic [`Peer::bootstrap_erased`] shell funnels through here, so the
/// protocol tower codegens once, in this crate.
///
/// `Ok(None)` is the mutual-bootstrap bail: the counterparty is itself
/// bootstrapping, so there is no donation to receive, and the epilogue has
/// already been exchanged. `Ok(Some(..))` hands back the reconciled root and
/// the control halves positioned at the trailing party frame.
///
/// Boxed and `inline(never)` for [`Reconciliation::reconcile`]'s reasons:
/// the `dyn` coercion is what pins the protocol state machine in this crate.
#[inline(never)]
#[allow(clippy::type_complexity)]
fn bootstrap_reconcile<'a>(
    link: DynLinkParts<'a>,
    codec: PayloadCodec,
    window: WindowConfig,
    run_budget: RunBudget,
    both_bootstrapping: bool,
    observe: SessionHandle,
) -> BoxFuture<'a, Result<Option<(tree::Root, DynRead<'a>, DynWrite<'a>)>, Error>> {
    Box::pin(async move {
        let (read, write, connector, acceptor, epoch) = link;
        let local_root: streaming::Root<Local> = tree::Root::default().into();
        // The window choice is passed for uniformity with gossip, but no
        // choice can widen this session: disputes require joint occupancy
        // and this side's replica is empty, so every derived capacity floors
        // at one slot regardless. The message-size target is the operative
        // knob: the greeting advertises it, and the provider's supply runs
        // are built at the exchanged minimum.
        let local = materialized::Handshaking::start(Local, local_root)
            .window(window)
            .target_message_size(run_budget.bytes() as u64);
        let carrier = Link::for_session(read, write, connector, acceptor, epoch);
        let proxy = streaming_remote::Handshaking::start(Local, carrier, codec)
            .window(window)
            .observe(observe.clone());
        let handshaken = streaming::handshake(local, proxy)
            .await
            .map_err(streaming_error)?;
        // A counterparty that is itself bootstrapping has nothing to hand
        // us, but the session still ends with the epilogue. Both trees are
        // empty, so the versions are equal and `reconcile` resolves to the
        // untouched control halves without opening a data stream; the marker
        // exchange then certifies the mutual bail to both sides. The
        // equal-version resolution is itself guarded: a fellow claimant must
        // be as newborn as we are.
        if both_bootstrapping {
            bootstrap_claimant_is_newborn(&handshaken.peer().version)?;
        }
        let descent: BoxFuture<'_, _> = Box::pin(handshaken.reconcile());
        let (root, (mut read, mut write)) = descent.await.map_err(streaming_error)?;
        if both_bootstrapping {
            epilogue(&mut read, &mut write, &observe).await?;
            return Ok(None);
        }
        Ok(Some((root.into(), read, write)))
    })
}

/// Erase a caller's link into one session's [`DynLinkParts`], opening the
/// session on the link's [`SessionState`]: each call is exactly one session.
///
/// Fails fast with [`Error::LinkPoisoned`] on a link whose previous session
/// was interrupted; on success the link is poisoned until its funnel
/// observes the session's clean completion and clears the latch.
fn erase<'a, CR, CW, C, A>(link: &'a mut Link<CR, CW, C, A>) -> Result<DynLinkParts<'a>, Error>
where
    CR: AsyncRead + Unpin + Send,
    CW: AsyncWrite + Unpin + Send,
    C: Connector,
    A: Acceptor,
{
    let epoch = link.session.begin()?;
    Ok((
        &mut link.control_read as DynRead<'a>,
        &mut link.control_write as DynWrite<'a>,
        DynConnector::new(link.connector.clone()),
        &mut link.acceptor as DynAcceptor<'a>,
        epoch,
    ))
}

/// Require a bootstrap claimant's greeting version to be empty, the version
/// a newborn replica has by construction.
///
/// A bootstrap claimant is definitionally a newborn replica with no causal
/// history, yet its greeting version feeds the deletion-honoring filter as
/// its causal frontier — a mis-declared frontier would make established
/// content read as deleted-there on both sides of the descent. Every
/// session facing a claimant runs this after the greeting and before
/// reconciliation, whichever protocol carries it: a failing session moves
/// nothing and poisons its link like any other pre-descent failure.
fn bootstrap_claimant_is_newborn(claimed: &Version) -> Result<(), Error> {
    if claimed.is_empty() {
        Ok(())
    } else {
        Err(Error::BootstrapHistoryConflict {
            claimed_min_events: claimed.min_ticks(),
        })
    }
}

/// Exchange the V2 session epilogue: write our completion marker, flush, and
/// read the peer's, concurrently (mirroring [`handshake::preamble`]).
///
/// Runs strictly after *all* local session work — the descent, any identity
/// hand-off, and the local commit — so a received marker certifies the peer
/// reached the same point. Both sides write and flush before either read
/// resolves, so the exchange cannot deadlock. Failure is [`Error::Epilogue`]:
/// post-commit by construction, with a non-marker byte surfaced as an
/// invalid-data protocol violation rather than an honest wire cut.
async fn epilogue(
    read: &mut (dyn AsyncRead + Unpin + Send + '_),
    write: &mut (dyn AsyncWrite + Unpin + Send + '_),
    observe: &SessionHandle,
) -> Result<(), Error> {
    let send = async {
        write.write_all(&EPILOGUE_MARKER).await?;
        write.flush().await?;
        observe.control_sent(&EPILOGUE_MARKER);
        Ok(())
    };
    let receive = async {
        let mut marker = [0u8; EPILOGUE_MARKER.len()];
        read.read_exact(&mut marker).await?;
        if marker != EPILOGUE_MARKER {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "peer wrote {:#04x} {:#04x} where the epilogue marker belongs",
                    marker[0], marker[1]
                ),
            ));
        }
        // The validated marker is byte-equal to the local constant, so
        // the constant is the received item.
        observe.control_received(&EPILOGUE_MARKER);
        Ok(())
    };
    futures_util::future::try_join(send, receive)
        .await
        .map(|((), ())| ())
        .map_err(Error::Epilogue)
}

/// What woke the [`gossip_when`](Peer::gossip_when) driver out of its idle
/// select: the remote's preamble (or its absence), or the `when` stream.
///
/// Materialized so the racing borrows end before the session consumes the
/// driver's transport halves.
enum Trigger {
    Arrival(Result<handshake::Fill, handshake::Error>),
    Tick(Option<Gossip>),
}

/// The state a [`gossip_when`](Peer::gossip_when) driver carries between
/// sessions: the erased link parts, the link's session state, the policy
/// stream, the preamble staging buffer, and the suppression token.
struct Drive<'a, T, B: BookmarkError, S> {
    peer: &'a Peer<T, B>,
    read: DynRead<'a>,
    write: DynWrite<'a>,
    connector: DynConnector,
    acceptor: DynAcceptor<'a>,
    /// The long-lived link's session state: the counter, advanced once per
    /// session so stream labels stay in lockstep with the remote's
    /// counting, and the poison latch the driver sets, clears, and obeys.
    state: &'a mut SessionState,
    when: Pin<Box<S>>,
    staged: handshake::Staged,
    /// The frontier this connection last converged on: a when-changed cue
    /// initiates only once the local frontier differs.
    ///
    /// `None` until the first session, so a fresh driver's first cue
    /// always initiates (the reconnect-convergence session).
    converged: Option<Version>,
    /// Terminal-state latch: set on error, clean remote goodbye, or `when`
    /// exhaustion, after which the stream yields nothing further.
    done: bool,
}

/// Dropping the driver poisons the link if the drop broke a session
/// boundary; it never clears the latch.
///
/// A driver dropped *inside* a session is already covered: `begin` poisoned
/// the link when the session opened. The case only this drop can see is a
/// driver dropped while idling with staged preamble bytes — the remote's
/// initiation was partially consumed out of the control stream, so a next
/// session would misread its remainder. Every clean termination path
/// (remote goodbye, `when` exhaustion at an empty boundary, a completed
/// session's re-arm) reaches this drop with the staging buffer empty and
/// leaves the latch alone, which is what keeps a cleanly ended connection
/// reusable.
impl<T, B: BookmarkError, S> Drop for Drive<'_, T, B, S> {
    fn drop(&mut self) {
        if !self.staged.is_empty() {
            self.state.poison();
        }
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
    // The depth-limit mismatch is a configuration diagnosis, not a
    // reconciliation failure: surface it as its own top-level variant.
    // Only the proxy (the server side of every production handshake)
    // detects it; the materialized participant has no wire.
    if let tree::mirror::Error::Server(streaming_remote::Error::PayloadDepthMismatch {
        local,
        remote,
    }) = error
    {
        return Error::PayloadDepthMismatch { local, remote };
    }
    Error::Mirror(error)
}

#[cfg(test)]
mod tests;
