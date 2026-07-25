//! The iroh transport: long-lived change-driven connections, the
//! partition-merge dance, and retirement.
//!
//! There is no application-level handshake. Each connection becomes one rumors
//! [`Link`]: its persistent QUIC bi-stream is the control stream and the
//! connection itself opens and accepts the session's unidirectional data
//! streams (see [`quic_link`]). That link feeds [`Rumors::gossip_when`]; the
//! rumors protocol's own preamble and greeting carry everything, including the
//! information the merge needs: a gossip attempt against a *different
//! universe* fails symmetrically on both ends with
//! [`Error::NetworkMismatch`], which carries the remote's [`Network`] and
//! both sides' handshake-declared event floors. Both sides plug the same two
//! floors into the same pure [`decide`] rule, so both agree who wins without
//! exchanging another byte: the loser opens a fresh stream and bootstraps
//! into the winner's universe, resetting itself wholesale
//! ([`Command::Reset`]).
//!
//! Gossip is change-driven, with no debounce: every connection runs
//! [`Rumors::gossip_when`] fed by [`Rumors::changes`], so a local commit is
//! on the wire the moment it lands, the driver's suppression keeps a
//! converged link silent, and the replicated presence heartbeat doubles as
//! a periodic anti-entropy tick. Connections are held open: for each
//! roster pair the smaller endpoint id dials (manual dial targets are
//! always ours to dial), so a pair settles on one connection. The roster
//! itself comes from the replicated Presence state — after the first manual
//! contact, peers are discovered through the very state being synchronized.
//!
//! Long-lived drives have one hazard of their own: a drive captures its
//! [`Rumors`] handle once, so a reset on *another* connection leaves it
//! gossiping the abandoned universe — agreeing with its peer about a dead
//! world, raising no mismatch, and stranding whoever is on the other end.
//! Every drive therefore watches the owner's published universe
//! ([`View::universe`]) and tears down the moment its handle goes stale;
//! the connector's backoff then redials and the fresh handles re-arbitrate.

use std::collections::{HashMap, HashSet};
use std::io;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context as _;
use futures::StreamExt;
use iroh::endpoint::{Connection, QuicTransportConfig, RecvStream, SendStream, presets};
use iroh::{Endpoint, EndpointId};
use iroh_mdns_address_lookup::MdnsAddressLookup;
use rumors::link::{Acceptor, Connector, Link};
use rumors::{Error, Network, Peer, Retire, Rumors};
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::{JoinHandle, JoinSet};
use tokio::time::{Instant, timeout};

use crate::entry::{Entry, PeerId};
use crate::owner::Command;
use crate::timers;
use crate::trace::trace;
use crate::view::View;

/// The ALPN identifying rumormill sessions.
pub const ALPN: &[u8] = b"rumormill/0";

/// Concurrent inbound connections held at once; excess waits behind a
/// blocked accept loop.
///
/// A permit lives as long as its connection, and
/// connections are long-lived, so this bounds the inbound half of the
/// mesh — and under the smaller-id-dials rule the largest endpoint id
/// accepts nearly every roster pair. The cap must therefore comfortably
/// exceed the room size: at the cap this node stops accepting entirely,
/// every dial toward it times out, and a newcomer whose only contact is
/// this node can never bootstrap.
const MAX_INBOUND: usize = 256;

/// How long [`settle`] waits for the peer's FIN before giving up and
/// letting the connection close anyway.
const TEARDOWN_GRACE: Duration = Duration::from_secs(5);

/// How many concurrent incoming unidirectional data streams a peer may open on
/// one connection.
///
/// The limit binds *incoming* streams only, so the requirement
/// is the 17 ([`STREAM_COUNT`](rumors::link::STREAM_COUNT)) data streams the
/// peer's half of one session may open toward us — not 34: our own opens count
/// against the peer's limit, not ours. Nor do sessions overlap on one
/// connection: a link runs sessions strictly in turn, a mismatched session
/// dies at the version handshake before any data stream opens (data streams
/// open lazily, on first frame), and retire rides a fresh connection. 64 is
/// simple headroom over 17 (it also absorbs any lag while a finished
/// session's closed streams drain); the QUIC default (100) already clears the
/// bar, but we pin it so a transport default change can never become the
/// throttle.
const MAX_CONCURRENT_UNI_STREAMS: u32 = 64;

/// Bind an endpoint with the default n0 infrastructure (relays + DNS
/// discovery) *plus* mDNS address lookup: peers are dialable by
/// `EndpointId` alone.
///
/// The mDNS path is what keeps a roomful of peers working: a crowd behind
/// one public IP (an office demo, a soak harness) rate-limits the shared
/// n0 DNS service — observed live as every dial failing with `Failed to
/// resolve TXT record` — while same-LAN resolution never leaves the
/// building. The n0 path remains for peers that are genuinely remote.
pub async fn bind() -> anyhow::Result<Endpoint> {
    // A rumors session maps each of its unidirectional data streams to a QUIC
    // uni stream (see [`QuicConnector`]/[`QuicAcceptor`]); pin the incoming
    // limit above what the protocol can demand so the transport never throttles
    // a session's descent.
    let transport = QuicTransportConfig::builder()
        .max_concurrent_uni_streams(MAX_CONCURRENT_UNI_STREAMS.into())
        .build();
    let endpoint = Endpoint::builder(presets::N0)
        .alpns(vec![ALPN.to_vec()])
        .transport_config(transport)
        .bind()
        .await
        .context("binding the iroh endpoint")?;
    let mdns = MdnsAddressLookup::builder()
        .build(endpoint.id())
        .context("starting mdns address lookup")?;
    endpoint
        .address_lookup()
        .context("registering the mdns address lookup")?
        .add(mdns);
    Ok(endpoint)
}

/// The [`Connector`] half of a native QUIC link: opens the session's outgoing
/// unidirectional data streams on one iroh [`Connection`]. Cloneable, `Send`,
/// and `Sync` because the connection handle is.
#[derive(Clone)]
struct QuicConnector(Connection);

impl Connector for QuicConnector {
    type Tx = SendStream;

    async fn connect(&self) -> io::Result<Self::Tx> {
        self.0.open_uni().await.map_err(io::Error::other)
    }
}

/// The [`Acceptor`] half of a native QUIC link: yields the peer's incoming
/// unidirectional data streams in arrival order.
struct QuicAcceptor(Connection);

impl Acceptor for QuicAcceptor {
    type Rx = RecvStream;

    async fn accept(&mut self) -> io::Result<Self::Rx> {
        self.0.accept_uni().await.map_err(io::Error::other)
    }
}

/// The link type one iroh connection carries: the connection's persistent
/// bidirectional gossip stream pair is the control stream, and the connection
/// itself opens and accepts every session's unidirectional data streams.
type QuicLink = Link<RecvStream, SendStream, QuicConnector, QuicAcceptor>;

/// Bundle one connection's gossip stream pair and its uni-stream supply into a
/// [`Link`].
///
/// The `recv`/`send` pair — opened with `open_bi`/`accept_bi` — is
/// the control stream; `conn` supplies the data streams for the session's
/// mirror descent.
fn quic_link(recv: RecvStream, send: SendStream, conn: &Connection) -> QuicLink {
    Link::new(
        recv,
        send,
        QuicConnector(conn.clone()),
        QuicAcceptor(conn.clone()),
    )
}

/// Who survives a meeting of two universes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Our universe wins; the peer resets and bootstraps from us.
    Win,
    /// Their universe wins; we reset and bootstrap from them.
    Lose,
}

/// The merge rule: the universe with the greater event floor wins, ties
/// broken by the greater [`Network`] id.
///
/// Pure and symmetric — both sides
/// evaluate it on the same two `(min_events, network)` pairs, both taken
/// from [`Error::NetworkMismatch`] (the floors each side *declared* in the
/// session's handshake), and reach opposite verdicts, so exactly one side
/// resets. Feeding it a fresh local floor instead of the declared one breaks
/// that symmetry: a commit landing mid-session can make both sides Win, and
/// each then serves a merge the other never requests. The caller guarantees
/// the networks differ (equal networks gossip normally and never raise the
/// mismatch).
pub fn decide(ours: (u64, Network), theirs: (u64, Network)) -> Verdict {
    debug_assert_ne!(ours.1, theirs.1, "same-universe peers never reach decide");
    if theirs > ours {
        Verdict::Lose
    } else {
        Verdict::Win
    }
}

/// How a drive ended (besides a transport-level error).
enum End {
    /// The peer said goodbye at a session boundary.
    Clean,
    /// A local reset made this drive's handle stale: it was gossiping a
    /// universe the owner has abandoned.
    ///
    /// Tear the connection down so the
    /// pair re-arbitrates on fresh handles — a stale drive never raises
    /// `NetworkMismatch` (both ends *agree* on the dead universe), so left
    /// alone it strands the remote in a world nobody else inhabits.
    Stale,
    /// The protocol reported an error (`NetworkMismatch` included).
    Failed(Error),
}

/// Drive one established connection with change-driven gossip until it
/// ends, reporting each completed session to the owner and handling the
/// partition-merge dance if the peer turns out to live in another universe.
///
/// Sessions learned content reaches the owner through its observer; this
/// loop only accounts for them. The driver's terminal `Err` ends it: the
/// link is poisoned afterwards, but the QUIC *connection* is fine, which is
/// what the merge dance rides on (it opens a fresh stream pair and builds a
/// fresh link).
///
/// The drive also watches the owner's published universe and ends
/// ([`End::Stale`]) when a reset on *another* connection abandons the
/// universe this drive's handle belongs to; see [`View::universe`].
async fn drive_connection(
    conn: &Connection,
    send: SendStream,
    recv: RecvStream,
    cmd: &mpsc::Sender<Command>,
    mut view: watch::Receiver<Arc<View>>,
) -> anyhow::Result<()> {
    let handle = request_handle(cmd).await?;
    let stale = |view: &watch::Receiver<Arc<View>>| {
        view.borrow()
            .universe
            .is_some_and(|u| u != handle.network())
    };
    let mut link = quic_link(recv, send, conn);
    let mut sessions = handle.gossip_when(handle.changes(), &mut link);
    let end = loop {
        // The handle was current when requested; a reset since then makes
        // every further session wasted work in a dead universe.
        if stale(&view) {
            break End::Stale;
        }
        // Racing `next()` is cancel-safe (driver state lives in the
        // stream), so the watch arm loses nothing mid-session.
        tokio::select! {
            next = sessions.next() => match next {
                Some(Ok(_session)) => {
                    let outcome = Command::SessionOutcome {
                        ok: true,
                        network: Some(handle.network()),
                    };
                    let _ = cmd.send(outcome).await;
                }
                Some(Err(e)) => break End::Failed(e),
                None => break End::Clean,
            },
            changed = view.changed() => {
                if changed.is_err() {
                    break End::Clean; // owner gone: shutting down
                }
            }
        }
    };
    drop(sessions);
    // Recover the control halves from the link now that no session borrows it;
    // the clean-teardown path still needs them to exchange FINs.
    let parts = link.into_parts();
    let (mut send, mut recv) = (parts.control_write, parts.control_read);

    match end {
        End::Clean => {
            trace(|| format!("drive {}: clean end", conn.remote_id().fmt_short()));
            settle(&mut send, &mut recv).await;
            Ok(())
        }
        // No settle: the peer isn't ending its half. Dropping the driver
        // forfeits the connection; the caller closes it, the connector's
        // backoff redials, and the fresh handles re-arbitrate the merge.
        End::Stale => {
            trace(|| {
                format!(
                    "drive {}: stale handle, torn down",
                    conn.remote_id().fmt_short()
                )
            });
            Ok(())
        }
        End::Failed(Error::NetworkMismatch {
            remote_network,
            remote_min_events,
            local_min_events,
        }) => {
            trace(|| {
                format!(
                    "drive {}: mismatch, theirs ({remote_min_events}, {remote_network:?})",
                    conn.remote_id().fmt_short()
                )
            });
            let outcome = Command::SessionOutcome {
                ok: false,
                network: Some(handle.network()),
            };
            let _ = cmd.send(outcome).await;
            // Our side of the merge comparison: the event floor this side
            // declared in the session's handshake — exactly the value the
            // peer's `decide` used as `theirs`. A fresh snapshot here would
            // race local commits: one landing mid-session on each side lifts
            // both fresh floors above both declared ones, both sides Win,
            // and each waits out a merge accept the other never attempts.
            // (After a concurrent reset the handle can be a stale universe;
            // the owner's reset guard refuses a double adoption, so the
            // worst case is one wasted bootstrap.)
            let ours = (local_min_events, handle.network());
            let verdict = decide(ours, (remote_min_events, remote_network));
            trace(|| {
                format!(
                    "merge {}: ours {ours:?}, verdict {verdict:?}",
                    conn.remote_id().fmt_short()
                )
            });
            let result = match verdict {
                // The peer saw the same mismatch and the opposite verdict:
                // it will open a fresh stream and bootstrap from us.
                Verdict::Win => serve_merge(conn, cmd).await,
                Verdict::Lose => request_merge(conn, cmd, ours.1).await,
            };
            if let Err(e) = &result {
                trace(|| {
                    format!(
                        "merge {}: {verdict:?} dance failed: {e:#}",
                        conn.remote_id().fmt_short()
                    )
                });
            }
            result
        }
        End::Failed(e) => {
            trace(|| format!("drive {}: failed: {e}", conn.remote_id().fmt_short()));
            let outcome = Command::SessionOutcome {
                ok: false,
                network: Some(handle.network()),
            };
            let _ = cmd.send(outcome).await;
            Err(e).context("gossip driver")
        }
    }
}

/// Merge, winning side: accept the loser's fresh stream and serve it with
/// plain gossip, which hands a bootstrapper our whole tree and forks it a
/// party transparently.
async fn serve_merge(conn: &Connection, cmd: &mpsc::Sender<Command>) -> anyhow::Result<()> {
    let (send, recv) = timeout(timers::SESSION_TIMEOUT, conn.accept_bi())
        .await
        .context("waiting for the loser's bootstrap stream timed out")?
        .context("accepting the loser's bootstrap stream")?;
    let handle = request_handle(cmd).await?;
    let mut link = quic_link(recv, send, conn);
    // Bounded: this serve holds a drive (and its accept-loop permit), and a
    // live-but-stalled loser must not pin it forever. The whole-tree budget
    // applies — a merge bootstrap ships our entire set, not a delta.
    timeout(timers::RECONCILE_TIMEOUT, handle.gossip(&mut link))
        .await
        .context("merge bootstrap serve timed out")?
        .context("serving the merge bootstrap")?;
    let parts = link.into_parts();
    let (mut send, mut recv) = (parts.control_write, parts.control_read);
    settle(&mut send, &mut recv).await;
    Ok(())
}

/// Merge, losing side: bootstrap a brand-new `Peer` from the winner over a
/// fresh stream (the mismatched one died mid-protocol) and hand it to the
/// owner as a [`Command::Reset`].
///
/// `abandoned` is the universe the verdict
/// was computed against; the owner adopts only while it is still in it, and
/// observes the adopted content by replaying it through a fresh observer.
async fn request_merge(
    conn: &Connection,
    cmd: &mpsc::Sender<Command>,
    abandoned: Network,
) -> anyhow::Result<()> {
    // Bounded like every wait in the merge dance: a winner that saw the
    // mismatch but never serves must not hang this drive forever.
    let (send, recv) = timeout(timers::SESSION_TIMEOUT, conn.open_bi())
        .await
        .context("opening the bootstrap stream timed out")?
        .context("opening the bootstrap stream")?;
    let mut link = quic_link(recv, send, conn);
    let known = timeout(
        timers::RECONCILE_TIMEOUT,
        Peer::<Entry>::bootstrap().join(&mut link),
    )
    .await
    .context("merge bootstrap timed out")?
    .context("bootstrapping into the winning universe")?
    .context("the winner was itself bootstrapping")?;
    let parts = link.into_parts();
    let (mut send, mut recv) = (parts.control_write, parts.control_read);
    settle(&mut send, &mut recv).await;
    cmd.send(Command::Reset {
        known: Box::new(known),
        abandoned,
    })
    .await
    .context("owner gone")?;
    Ok(())
}

/// Conclude a session stream gracefully: send our FIN, then wait for the
/// peer's.
///
/// EOF from the peer proves it finished writing and QUIC delivered
/// everything; only then is it safe for either side to close the
/// connection. The in-session frames are already protected by the protocol's
/// own epilogue exchange; this grace period exists so a clean teardown does
/// not race the peer's still-draining final bytes into a spurious
/// post-commit error on its side (`Connection::close` discards in-flight
/// data).
async fn settle(send: &mut iroh::endpoint::SendStream, recv: &mut iroh::endpoint::RecvStream) {
    let _ = send.finish();
    let _ = timeout(TEARDOWN_GRACE, recv.read_to_end(64)).await;
}

/// Ask the owner for a session handle (a `Rumors` clone of the current
/// universe's set).
async fn request_handle(cmd: &mpsc::Sender<Command>) -> anyhow::Result<Rumors<Entry>> {
    let (reply, rx) = oneshot::channel();
    cmd.send(Command::Handle { reply })
        .await
        .context("owner gone")?;
    rx.await.context("owner dropped the handle request")
}

/// Dial `peer`, open the connection's one gossip stream, and drive it with
/// change-driven sessions until it ends.
async fn dial_and_drive(
    endpoint: &Endpoint,
    peer: PeerId,
    cmd: &mpsc::Sender<Command>,
    view: watch::Receiver<Arc<View>>,
) -> anyhow::Result<()> {
    // Replicated bytes that are not a valid public key: a peer published
    // garbage; the backoff in the connector keeps the retry cost bounded.
    let target = EndpointId::from_bytes(&peer).context("peer id is not a valid key")?;
    let conn = match timeout(timers::DIAL_TIMEOUT, endpoint.connect(target, ALPN)).await {
        Ok(Ok(conn)) => conn,
        Ok(Err(e)) => {
            trace(|| format!("dial {}: failed: {e:#}", target.fmt_short()));
            return Err(e).context("dial failed");
        }
        Err(elapsed) => {
            trace(|| format!("dial {}: timed out", target.fmt_short()));
            return Err(elapsed).context("dial timed out");
        }
    };
    let (send, recv) = conn.open_bi().await.context("opening the gossip stream")?;
    let result = drive_connection(&conn, send, recv, cmd, view).await;
    conn.close(0u32.into(), b"done");
    result
}

/// Serve inbound connections until the task is aborted.
///
/// Aborting the returned handle aborts every in-flight session with it (the
/// `JoinSet` drops), which releases their snapshots — the exclusivity
/// [`retire`] requires.
pub fn spawn_accept_loop(
    endpoint: Endpoint,
    cmd: mpsc::Sender<Command>,
    view: watch::Receiver<Arc<View>>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let limit = Arc::new(tokio::sync::Semaphore::new(MAX_INBOUND));
        let mut sessions: JoinSet<()> = JoinSet::new();
        while let Some(incoming) = endpoint.accept().await {
            // Reap finished sessions so the set does not grow unboundedly.
            while sessions.try_join_next().is_some() {}
            let permit = match limit.clone().acquire_owned().await {
                Ok(permit) => permit,
                Err(_) => return,
            };
            let cmd = cmd.clone();
            let view = view.clone();
            sessions.spawn(async move {
                let _permit = permit;
                let Ok(conn) = incoming.await else { return };
                // The dialer opens the connection's one gossip stream
                // promptly; only that wait is bounded — the drive itself
                // lives as long as the connection.
                let Ok(Ok((send, recv))) = timeout(timers::SESSION_TIMEOUT, conn.accept_bi()).await
                else {
                    return;
                };
                // Per-session outcomes (and any terminal failure) are
                // reported from inside the drive.
                let _ = drive_connection(&conn, send, recv, &cmd, view).await;
                conn.close(0u32.into(), b"done");
            });
        }
    })
}

/// Maintain one live, change-driven connection per dialable peer: spawn a
/// [`dial_and_drive`] for every candidate not already connected, and react
/// to roster changes, finished drivers, and backoff expiry.
///
/// A peer whose
/// connection ends — failure or goodbye — is left alone for
/// [`timers::PEER_BACKOFF`] before redialing, so a flapping peer cannot
/// induce a dial storm.
pub fn spawn_connector(
    endpoint: Endpoint,
    cmd: mpsc::Sender<Command>,
    mut view: watch::Receiver<Arc<View>>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let me = endpoint.id();
        let mut backoff: HashMap<PeerId, Instant> = HashMap::new();
        let mut active: HashSet<PeerId> = HashSet::new();
        let mut drivers: JoinSet<(PeerId, bool)> = JoinSet::new();
        loop {
            let now = Instant::now();
            backoff.retain(|_, until| *until > now);
            for peer in dial_candidates(&view.borrow(), &active, &backoff, *me.as_bytes()) {
                active.insert(peer);
                let endpoint = endpoint.clone();
                let cmd = cmd.clone();
                let view = view.clone();
                drivers.spawn(async move {
                    let ok = dial_and_drive(&endpoint, peer, &cmd, view).await.is_ok();
                    (peer, ok)
                });
            }
            tokio::select! {
                changed = view.changed() => {
                    if changed.is_err() {
                        return; // owner gone: shutting down
                    }
                }
                Some(finished) = drivers.join_next(), if !drivers.is_empty() => {
                    if let Ok((peer, ok)) = finished {
                        active.remove(&peer);
                        backoff.insert(peer, Instant::now() + timers::PEER_BACKOFF);
                        // Dial failures never reach the per-session
                        // accounting inside the drive; count them here. No
                        // session means no universe to attribute.
                        let outcome = Command::SessionOutcome {
                            ok: false,
                            network: None,
                        };
                        if !ok && cmd.send(outcome).await.is_err() {
                            return;
                        }
                    }
                }
                // Backoff expiry has no event of its own: sweep for it.
                _ = tokio::time::sleep(timers::REDIAL_SWEEP) => {}
            }
        }
    })
}

/// Everyone we should be dialing right now: for roster pairs, the smaller
/// endpoint id dials (exactly one side of each pair, so the mesh settles on
/// one connection per pair).
///
/// Manual dial targets are always ours to dial
/// (the other side may not know us yet). Excludes ourselves, live
/// connections, and backed-off peers.
fn dial_candidates(
    view: &View,
    active: &HashSet<PeerId>,
    backoff: &HashMap<PeerId, Instant>,
    mine: PeerId,
) -> Vec<PeerId> {
    let candidates: HashSet<PeerId> = view
        .roster
        .iter()
        .map(|p| p.peer)
        .filter(|p| mine < *p)
        .chain(view.dial_targets.iter().copied())
        .filter(|p| *p != mine && !active.contains(p) && !backoff.contains_key(p))
        .collect();
    candidates.into_iter().collect()
}

/// How leaving the universe went.
#[derive(Debug)]
pub enum Departure {
    /// A peer absorbed our party; nothing was leaked.
    Retired {
        /// Who took it.
        into: EndpointId,
    },
    /// The failure struck during the party hand-off itself: the peer may
    /// hold our party, so we must not retry (two generals).
    ///
    /// At worst the
    /// region leaks; it is never duplicated. A retire attempt that outruns
    /// [`timers::RECONCILE_TIMEOUT`] lands here too: from outside the
    /// dropped session we cannot tell which phase it stalled in, so the
    /// timeout maps to `Uncertain` conservatively — never a retry.
    Uncertain,
    /// No candidate could absorb us; the region leaks with us.
    Leaked,
}

/// Retire into the first willing candidate, walking the list in presence
/// recency order.
///
/// The caller hands us the unique `Peer` — the
/// `Peer`/`Rumors` XOR means no session can be using the set while we
/// hold it, so retirement's exclusivity holds by construction. Every wait
/// is bounded: retirement runs on the daemon's shutdown path, and a
/// live-but-stalled absorber must not hang the exit forever.
pub async fn retire(
    endpoint: &Endpoint,
    mut retiree: Peer<Entry>,
    candidates: Vec<PeerId>,
) -> Departure {
    for peer in candidates {
        let Ok(target) = EndpointId::from_bytes(&peer) else {
            continue;
        };
        let Ok(Ok(conn)) = timeout(timers::DIAL_TIMEOUT, endpoint.connect(target, ALPN)).await
        else {
            continue;
        };
        // Bounded like request_merge's identical wait: `open_bi` waits
        // indefinitely for the peer's stream credit, so a live but
        // withholding candidate would otherwise park the shutdown path
        // here, before RECONCILE_TIMEOUT is ever armed.
        let Ok(Ok((send, recv))) = timeout(timers::SESSION_TIMEOUT, conn.open_bi()).await else {
            continue;
        };
        let mut link = quic_link(recv, send, &conn);

        // Bounded by the whole-tree budget: the retire session first
        // reconciles the full sets, then hands the party over.
        match timeout(timers::RECONCILE_TIMEOUT, retiree.retire(&mut link)).await {
            // Retired: we are gone; the peer holds our party. Settle like
            // every other clean teardown — an immediate close discards our
            // still-in-flight final bytes (`Connection::close` drops them),
            // which can cost the absorber a spurious post-commit
            // `Error::Epilogue` on a perfectly clean retirement.
            Ok(Retire::Retired) => {
                let parts = link.into_parts();
                let (mut send, mut recv) = (parts.control_write, parts.control_read);
                settle(&mut send, &mut recv).await;
                conn.close(0u32.into(), b"retired");
                return Departure::Retired { into: target };
            }
            // Declined (the peer was itself retiring): intact, try the
            // next candidate.
            Ok(Retire::Declined { peer: intact }) => {
                retiree = intact;
                conn.close(0u32.into(), b"declined");
            }
            // Failed before the party frame: intact, try elsewhere.
            Ok(Retire::Recovered { peer: intact, .. }) => {
                retiree = intact;
                conn.close(0u32.into(), b"recovered");
            }
            // Failed during the party frame: the peer may hold our
            // party. Retrying could duplicate the region; stop here.
            Ok(Retire::Uncertain { .. }) => {
                conn.close(0u32.into(), b"uncertain");
                return Departure::Uncertain;
            }
            // Timed out: the dropped session consumed the retiree, and from
            // out here we cannot tell whether the party frame had crossed.
            // Classify conservatively as Uncertain and never retry — a
            // retry against a peer that did absorb us would duplicate the
            // region, which is worse than leaking it.
            Err(_) => {
                conn.close(0u32.into(), b"retire timeout");
                return Departure::Uncertain;
            }
        }
    }
    Departure::Leaked
}

#[cfg(test)]
mod tests;
