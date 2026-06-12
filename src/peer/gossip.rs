//! The wire-session drivers for [`Peer`]: [`bootstrap`](Peer::bootstrap),
//! [`gossip`](crate::Rumors::gossip), and [`retire`](Peer::retire), plus the
//! preamble constants every session leads with and the [`PartyGuard`]
//! that snaps a speculatively-donated party back in place on failure.

use std::pin::Pin;

use before::Party;
use borsh::{BorshDeserialize, BorshSerialize};
use futures_util::{Stream, StreamExt};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::watch,
};

use crate::tree::mirror::{local, message::Intent, remote};
use crate::tree::{self, Tree, mirror};
use crate::{Error, Network, Version};

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
pub enum Retire<T> {
    /// **Retired.** The peer reconciled with us and absorbed our identity;
    /// this replica has left the universe.
    Retired,
    /// **Declined, unchanged.** The peer cannot absorb an identity — it was
    /// itself retiring — so nothing touched the wire and the replica is
    /// handed back intact, to retry elsewhere.
    Declined {
        /// The intact retiree.
        peer: Peer<T>,
    },
    /// **Recovered, unchanged.** The session failed *before* our identity
    /// ever crossed the wire; the replica is handed back intact, to retry
    /// elsewhere. Nothing was lost.
    Recovered {
        /// The intact retiree.
        peer: Peer<T>,
        /// What failed the session.
        error: Error,
    },
    /// **Uncertain.** The session failed while the identity itself was in
    /// flight: the peer may or may not hold it, so the retiree is consumed
    /// rather than risk the same identity living twice.
    Uncertain {
        /// What failed the session.
        error: Error,
    },
}

/// One completed session of a
/// [`gossip_when`](crate::Rumors::gossip_when) driver: the driver's stream
/// yields one of these per session, successful sessions only (a failed
/// session is the stream's terminal `Err`).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Session {
    /// The causal frontier the two replicas converged on: at the instant
    /// the session committed, both held exactly this version. It is also
    /// the driver's suppression token — a later tick initiates only once
    /// the local frontier has moved past it — so a consumer watching these
    /// values watches the suppression contract itself.
    pub converged: Version,
    /// Which trigger entered the session on this side.
    pub led: Led,
}

/// Which trigger entered a [`gossip_when`](crate::Rumors::gossip_when)
/// session on this side of the connection: diagnostic, not protocol. The
/// session itself is symmetric, and when both sides' triggers fire close
/// together, each side may record `Local` for what becomes one session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Led {
    /// The `when` stream yielded: this side initiated.
    Local,
    /// The remote's preamble arrived first: this side responded.
    Remote,
}

impl<T> Peer<T> {
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
        // Magic/version/network/intent preamble first, through the same
        // exact-read framing every later frame uses.
        let mut staged = remote::Staged::new();
        let mut reader = remote::FrameRead::new(read);
        let mut writer = remote::FrameWrite::new(write);
        let (remote_network, remote_intent) = remote::preamble(
            Network::BOOTSTRAP,
            Intent::Remain,
            &mut staged,
            &mut reader,
            &mut writer,
        )
        .await?;

        // In the bootstrap case, it doesn't matter whether the remote intends
        // to remain or retire; they will hand us a party regardless, and we can
        // absorb it.
        let _ = remote_intent;

        // We hold nothing: we will run the mirror protocol from an *empty* tree
        // to receive all content on the remote side.
        let l = local::Exchange::start(tree::Root::default());
        let r = remote::Exchange::start(reader, writer);

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
    /// content exactly as [`gossip`](crate::Rumors::gossip) would, so everything we
    /// hold that the peer had not yet seen survives in it; the peer then
    /// absorbs our identity. A peer running ordinary gossip absorbs a retiree
    /// transparently, so the counterparty needs no special call. The four
    /// outcomes are the [`Retire`] variants; see each for what survived.
    ///
    /// The gossip round writes back into the retiring set too: observers of
    /// a retiring set ([`Messages`](crate::Messages),
    /// [`CausalMessages`](crate::CausalMessages)) drain the *reconciled*
    /// final state — everything the session learned included — before they
    /// end.
    pub async fn retire<'a, R, W>(self, read: &'a mut R, write: &'a mut W) -> Retire<T>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        let mut staged = remote::Staged::new();
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
    ) -> Result<(), Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        let mut staged = remote::Staged::new();
        self.gossip_inner(Intent::Remain, &mut staged, read, write)
            .await
            .1
            .map(|_converged| ())
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
        staged: &mut remote::Staged,
        read: &'a mut R,
        write: &'a mut W,
    ) -> (Intent, Result<Version, Error>)
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        // Magic/version preamble: reject a non-rumors or incompatible peer
        // before the framing trusts any peer-supplied frame length.
        let mut reader = remote::FrameRead::new(read);
        let mut writer = remote::FrameWrite::new(write);
        let (remote_network, remote_intent) =
            match remote::preamble(self.network, intent, staged, &mut reader, &mut writer).await {
                Err(e) => return (Intent::Remain, Err(e)),
                Ok(output) => output,
            };
        let peer_bootstrapping = remote_network.is_bootstrap();
        let self_retiring = intent == Intent::Retire;
        let peer_retiring = remote_intent == Intent::Retire;

        // Stop cleanly, early if we're both trying to retire into each other
        if self_retiring && peer_retiring {
            let unchanged = self.inner.borrow().tree.latest().clone();
            return (Intent::Remain, Ok(unchanged));
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
                    // `Peer` is not handed back. A lost fork merely leaks its
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

        // In the case where we successfully retired (only callable on the
        // !Clone `Peer<T>`), we've given away our inner party and no more
        // actions are possible, so don't hand back the `Peer`.
        (outcome, Ok(converged))
    }

    /// Run the change-driven gossip driver behind
    /// [`Rumors::gossip_when`](crate::Rumors::gossip_when); the public
    /// contract lives there.
    pub(crate) fn gossip_when<'a, R, W, S>(
        &'a self,
        when: S,
        read: &'a mut R,
        write: &'a mut W,
    ) -> impl Stream<Item = Result<Session, Error>> + Unpin + 'a
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
            staged: remote::Staged::new(),
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
                        let mut reader = remote::FrameRead::new(&mut *drive.read);
                        tokio::select! {
                            arrival = drive.staged.fill(&mut reader) => Trigger::Arrival(arrival),
                            tick = drive.when.next() => Trigger::Tick(tick),
                        }
                    };
                    let led = match trigger {
                        Trigger::Arrival(Err(e)) => {
                            drive.done = true;
                            return Some((Err(e), drive));
                        }
                        // A hang-up on an idle boundary — not one preamble byte
                        // arrived — is the peer's clean goodbye: end in kind.
                        // (Returning `None` is itself the unfold's terminal
                        // state; no latch needed on paths that end here.)
                        Trigger::Arrival(Ok(remote::Fill::Closed)) => return None,
                        Trigger::Arrival(Ok(remote::Fill::Filled)) => Led::Remote,
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
                            drive.staged = remote::Staged::new();
                            drive.converged = Some(converged.clone());
                            Some((Ok(Session { converged, led }), drive))
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
/// Materialized so the racing borrows end before the session consumes the
/// driver's transport halves.
enum Trigger {
    Arrival(Result<remote::Fill, Error>),
    Tick(Option<()>),
}

/// The state a [`gossip_when`](Peer::gossip_when) driver carries between
/// sessions: the transport halves, the policy stream, the preamble staging
/// buffer, and the suppression token.
struct Drive<'a, T, R, W, S> {
    peer: &'a Peer<T>,
    read: &'a mut R,
    write: &'a mut W,
    when: Pin<Box<S>>,
    staged: remote::Staged,
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
