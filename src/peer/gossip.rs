//! The wire-session drivers for [`Peer`]: [`bootstrap`](Peer::bootstrap),
//! [`gossip`](crate::Rumors::gossip), and [`retire`](Peer::retire).
//!
//! Plus the
//! preamble constants every session leads with and the [`PartyGuard`]
//! that snaps a speculatively-donated party back in place on failure.

use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;

use before::Party;
use borsh::{BorshDeserialize, BorshSerialize};
use futures_util::{Stream, StreamExt};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::{Mutex, watch},
};

use crate::mode::{Async, Mode};
use crate::tree::{self, Tree};
use crate::{Error, Network, Version};
use crate::{
    bookmark::{Bookmark, BookmarkError, BookmarkIo, Bookmarked, NoBookmark, Persist},
    tree::mirror::{
        alternating::{self, local, remote},
        handshake::{self, Intent},
    },
};

use super::{Inner, Peer};

/// Magic bytes that open every `rumors` gossip session's preamble frame.
pub const PROTOCOL_MAGIC: [u8; 6] = *b"RUMORS";

/// On-the-wire protocol version that follows [`PROTOCOL_MAGIC`].
///
/// Bumped whenever the wire format changes. A peer whose version differs is
/// rejected with [`Error::VersionMismatch`].
pub const PROTOCOL_VERSION: u16 = 1;

/// The outcome of [`Peer::retire`].
///
/// Marked `must_use` because two variants carry the intact [`Peer`]: silently
/// dropping the result of a declined or recovered retirement destroys the
/// identity that the call was specifically trying to preserve.
#[must_use = "a declined or recovered retirement hands the Peer back; dropping it leaks the identity"]
#[derive(Debug)]
pub enum Retire<T, B: BookmarkError = NoBookmark, M: Mode = Async> {
    /// **Retired.** The peer reconciled with us and absorbed our identity;
    /// this replica has left the universe.
    Retired,
    /// **Declined, unchanged.** The peer was itself retiring, so nothing our
    /// replica is handed back intact, to try retiring elsewhere.
    Declined {
        /// The intact retiree.
        peer: Peer<T, B, M>,
    },
    /// **Recovered, unchanged.** The session failed *before* our identity ever
    /// crossed the wire; the replica is handed back intact, to try retiring
    /// elsewhere.
    Recovered {
        /// The intact retiree.
        peer: Peer<T, B, M>,
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
pub struct Unbookmarked<T, B: BookmarkError, M: Mode = Async> {
    /// The peer, its identity intact and no bookmark attached.
    pub peer: Peer<T, NoBookmark, M>,
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

impl<T, M: Mode> Peer<T, NoBookmark, M> {
    /// The mode-agnostic engine behind [`bootstrap`](Peer::bootstrap): runs the
    /// join over any [`AsyncRead`]/[`AsyncWrite`] pair and builds a peer in
    /// mode `M`.
    ///
    /// The async face awaits it; the blocking face drives it to
    /// completion over [`std::io`].
    pub(crate) async fn bootstrap_inner<'a, R, W>(
        read: &'a mut R,
        write: &'a mut W,
    ) -> Result<Option<Self>, Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        // Magic/version/network/intent preamble first, before either protocol
        // is allowed to trust peer-declared frame lengths.
        let mut staged = handshake::Staged::new();
        let remote =
            handshake::preamble(Network::BOOTSTRAP, Intent::Remain, &mut staged, read, write)
                .await
                .map_err(remote::Error::from)?;
        let reader = remote::FrameRead::new(read);
        let writer = remote::FrameWrite::new(write);

        // In the bootstrap case, it doesn't matter whether the remote intends
        // to remain or retire; they will hand us a party regardless, and we can
        // absorb it.
        let _ = remote.intent;

        // We hold nothing: we will run the mirror protocol from an *empty* tree
        // to receive all content on the remote side.
        let l = local::Exchange::start(tree::Root::default());
        let r = remote::Exchange::start(reader, writer);

        // After the connect phase, a peer that is *also* bootstrapping means
        // there is nothing to receive: bail symmetrically.
        let handshaken = alternating::handshake(l, r).await.map_err(server_error)?;
        if remote.network.is_bootstrap() {
            return Ok(None);
        }

        // Otherwise reconcile, pulling the provider's whole tree through the
        // descent, then read the provider's party frame off the same reader,
        // and adopt its network alongside.
        //
        // Boxed: the descent state machine is a large future, and the codec
        // buffers inflate it past the crate-wide `large_futures` ceiling.
        let (root, (mut reader, _writer)) = Box::pin(handshaken.reconcile())
            .await
            .map_err(server_error)?;
        let party = remote::recv_party(&mut reader).await?;
        let peer = Self {
            network: remote.network,
            inner: watch::Sender::new(Inner {
                party: Some(party),
                tree: Tree { root },
            }),
            bookmark: Arc::new(Mutex::new(Bookmarked::new(NoBookmark))),
            marker: PhantomData,
        };
        Ok(Some(peer))
    }

    /// The mode-agnostic engine behind [`bookmark`](Peer::bookmark): attaches
    /// `bookmark` and eagerly persists, preserving the peer's mode `M`.
    ///
    /// It
    /// drives the bookmark through [`Persist<M>`](Persist), so the async face
    /// awaits real I/O and the blocking face runs the same body to completion
    /// over the synchronous calls — with no wrapper type, the stored `B` is the
    /// caller's own bookmark either way.
    pub(crate) async fn bookmark_inner<B: Persist<M>>(
        self,
        bookmark: B,
    ) -> Result<Peer<T, B, M>, Unbookmarked<T, B, M>> {
        let Peer { network, inner, .. } = self;
        let peer = Peer {
            network,
            inner,
            bookmark: Arc::new(Mutex::new(Bookmarked::new(bookmark))),
            marker: PhantomData,
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
                    inner: peer.inner,
                    bookmark: Arc::new(Mutex::new(Bookmarked::new(NoBookmark))),
                    marker: PhantomData,
                },
                error,
            }),
        }
    }
}

// `Persist` is the crate-internal I/O driver, but it constrains `B` in the
// public `Peer<T, B, M>` self type, so `private_bounds` flags it. It is not a
// leak: every method here is `pub(crate)`, and the public entry points
// (`gossip`, `retire`, `bookmark`, ...) bind the public `Bookmark` /
// `sync::Bookmark` faces, so `Persist` never appears in the public API.
#[allow(private_bounds)]
impl<T, B: Persist<M>, M: Mode> Peer<T, B, M> {
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
    /// The mode-agnostic body behind the async and blocking
    /// [`retire`](Peer::retire); see those for the public contract.
    pub(crate) async fn retire_inner<'a, R, W>(
        self,
        read: &'a mut R,
        write: &'a mut W,
    ) -> Retire<T, B, M>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
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
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
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
    async fn gossip_inner<'a, R, W>(
        &self,
        intent: Intent,
        staged: &mut handshake::Staged,
        read: &'a mut R,
        write: &'a mut W,
    ) -> (Intent, Result<Version, Error<B>>)
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        // Magic/version preamble: reject a non-rumors or incompatible peer
        // before the framing trusts any peer-supplied frame length.
        let remote = match handshake::preamble(self.network, intent, staged, read, write).await {
            Err(error) => return (Intent::Remain, Err(remote::Error::from(error).widen())),
            Ok(remote) => remote,
        };
        let reader = remote::FrameRead::new(read);
        let writer = remote::FrameWrite::new(write);
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

        // Run the connect phase, which exchanges `message::Handshake`s (the
        // causal version; network and intent already rode the preamble).
        let l = local::Exchange::start(prior_tree.root);
        let r = remote::Exchange::start(reader, writer);

        // Run the initial handshake to determine if and how to gossip.
        //
        // We do this *even if we already know the networks mismatch* because we
        // want to receive the peer's version, so we can report the
        // `remote_min_events` count computed over the peer's version.
        let handshaken = match alternating::handshake(l, r).await.map_err(server_error) {
            Err(e) => return (Intent::Remain, Err(e.widen())),
            Ok(handshaken) => handshaken,
        };

        // Abort if the networks mismatch
        if !peer_bootstrapping && remote.network != self.network {
            return (
                Intent::Remain,
                Err(Error::NetworkMismatch {
                    remote_network: remote.network,
                    remote_min_events: handshaken.peer().version.min_ticks(),
                }),
            );
        }

        // Run content reconciliation, so that we both have exactly the same
        // version and messages
        let (root, (mut reader, mut writer)) =
            match Box::pin(handshaken.reconcile()).await.map_err(server_error) {
                Err(e) => return (Intent::Remain, Err(e.widen())),
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
            match remote::send_party(donated, &mut writer).await {
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

impl<T, B: Bookmark> Peer<T, B, Async> {
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
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
        S: Stream<Item = ()> + 'a,
    {
        let drive = Drive {
            peer: self,
            read,
            write,
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
                            return Some((Err(remote::Error::from(e).widen()), drive));
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
struct Drive<'a, T, B: BookmarkError, R, W, S> {
    peer: &'a Peer<T, B>,
    read: &'a mut R,
    write: &'a mut W,
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

/// Collapse a mirror error down to the wire-bound server error.
///
/// The client side
/// of every wire session is the in-memory local exchange, whose error type is
/// [`Infallible`](std::convert::Infallible), so the
/// [`Client`](alternating::Error::Client) arm is uninhabitable.
fn server_error(e: alternating::Error<std::convert::Infallible, Error>) -> Error {
    match e {
        alternating::Error::Server(e) => e,
        alternating::Error::Client(never) => match never {},
    }
}
