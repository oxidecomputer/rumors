//! The iroh transport: dialing, accepting, the partition-merge dance, the
//! Poisson gossip scheduler, and retirement.
//!
//! There is no application-level handshake. A session feeds the raw QUIC
//! bi-stream straight into [`Rumors::gossip`]; the rumors protocol's own
//! preamble and greeting carry everything, including the one piece of
//! information the merge needs: a gossip attempt against a *different
//! universe* fails symmetrically on both ends with
//! [`Error::NetworkMismatch`], which names the remote's [`Network`] and a
//! floor on how many events it has ever recorded. Both sides plug those into
//! the same pure [`decide`] rule, so both agree who wins without exchanging
//! another byte: the loser opens a fresh stream and bootstraps into the
//! winner's universe, resetting itself wholesale ([`Command::Reset`]).
//!
//! Gossip initiations form a Poisson process (exponentially distributed
//! delays, a uniformly random live peer each event): independent nodes never
//! beat in lockstep, and random pairing over the whole roster keeps the mesh
//! connected. The roster itself comes from the replicated Presence state —
//! after the first manual contact, peers are discovered through the very
//! state being synchronized.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context as _;
use iroh::endpoint::{Connection, presets};
use iroh::{Endpoint, EndpointId};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rumors::{Error, Network, Peer, Retire, Rumors};
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::{JoinHandle, JoinSet};
use tokio::time::{Instant, timeout};

use crate::entry::{Entry, PeerId};
use crate::owner::Command;
use crate::timers;
use crate::view::View;

/// The ALPN identifying rumormill sessions.
pub const ALPN: &[u8] = b"rumormill/0";

/// Concurrent inbound sessions served at once; excess streams wait.
const MAX_INBOUND: usize = 8;

/// How long [`settle`] waits for the peer's FIN before giving up and
/// letting the connection close anyway.
const TEARDOWN_GRACE: Duration = Duration::from_secs(5);

/// Bind an endpoint with the default n0 infrastructure (relays + DNS
/// discovery): peers are dialable by `EndpointId` alone.
pub async fn bind() -> anyhow::Result<Endpoint> {
    Endpoint::builder(presets::N0)
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await
        .context("binding the iroh endpoint")
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
/// broken by the greater [`Network`] id. Pure and symmetric — both sides
/// evaluate it on the same `(min_events, network)` pair (each side's own
/// versus the one [`Error::NetworkMismatch`] reported) and reach opposite
/// verdicts, so exactly one side resets. The caller guarantees the networks
/// differ (equal networks gossip normally and never raise the mismatch).
pub fn decide(ours: (u64, Network), theirs: (u64, Network)) -> Verdict {
    debug_assert_ne!(ours.1, theirs.1, "same-universe peers never reach decide");
    if theirs > ours {
        Verdict::Lose
    } else {
        Verdict::Win
    }
}

/// Run one gossip session over an established stream pair, handling the
/// partition-merge dance if the peer turns out to live in another universe.
async fn run_stream(
    conn: &Connection,
    mut send: iroh::endpoint::SendStream,
    mut recv: iroh::endpoint::RecvStream,
    cmd: &mpsc::Sender<Command>,
) -> anyhow::Result<()> {
    let handle = request_handle(cmd).await?;
    // Our side of the merge comparison, from before the attempt.
    let ours = (handle.snapshot().latest().min_ticks(), handle.network());

    match handle.gossip(&mut recv, &mut send).await {
        Ok(()) => {
            // Whatever the session learned is already in the shared set,
            // on its way to the owner through its observer.
            settle(&mut send, &mut recv).await;
            Ok(())
        }
        Err(Error::NetworkMismatch {
            remote_network,
            remote_min_events,
        }) => {
            match decide(ours, (remote_min_events, remote_network)) {
                // The peer saw the same mismatch and the opposite verdict:
                // it will open a fresh stream and bootstrap from us.
                Verdict::Win => serve_merge(conn, cmd).await,
                Verdict::Lose => request_merge(conn, cmd, ours.1).await,
            }
        }
        Err(e) => Err(e).context("gossip session"),
    }
}

/// Merge, winning side: accept the loser's fresh stream and serve it with
/// plain gossip, which hands a bootstrapper our whole tree and forks it a
/// party transparently.
async fn serve_merge(conn: &Connection, cmd: &mpsc::Sender<Command>) -> anyhow::Result<()> {
    let (mut send, mut recv) = timeout(timers::SESSION_TIMEOUT, conn.accept_bi())
        .await
        .context("waiting for the loser's bootstrap stream")??;
    let handle = request_handle(cmd).await?;
    handle
        .gossip(&mut recv, &mut send)
        .await
        .context("serving the merge bootstrap")?;
    settle(&mut send, &mut recv).await;
    Ok(())
}

/// Merge, losing side: bootstrap a brand-new `Peer` from the winner over a
/// fresh stream (the mismatched one died mid-protocol) and hand it to the
/// owner as a [`Command::Reset`]. `abandoned` is the universe the verdict
/// was computed against; the owner adopts only while it is still in it, and
/// observes the adopted content by replaying it through a fresh observer.
async fn request_merge(
    conn: &Connection,
    cmd: &mpsc::Sender<Command>,
    abandoned: Network,
) -> anyhow::Result<()> {
    let (mut send, mut recv) = conn
        .open_bi()
        .await
        .context("opening the bootstrap stream")?;
    let known = Peer::<Entry>::bootstrap(&mut recv, &mut send)
        .await
        .context("bootstrapping into the winning universe")?
        .context("the winner was itself bootstrapping")?;
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
/// peer's. EOF from the peer proves it finished writing and QUIC delivered
/// everything; only then is it safe for either side to close the
/// connection (`Connection::close` discards in-flight data, so closing
/// before the FIN exchange races the peer's final frames — the bootstrap
/// party frame especially).
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

/// Dial `peer` and run one session.
async fn dial_session(
    endpoint: &Endpoint,
    peer: EndpointId,
    cmd: &mpsc::Sender<Command>,
) -> anyhow::Result<()> {
    let conn = timeout(timers::DIAL_TIMEOUT, endpoint.connect(peer, ALPN))
        .await
        .context("dial timed out")?
        .context("dial failed")?;
    let (send, recv) = conn.open_bi().await.context("opening the gossip stream")?;
    let result = run_stream(&conn, send, recv, cmd).await;
    conn.close(0u32.into(), b"done");
    result
}

/// Serve inbound connections until the task is aborted.
///
/// Aborting the returned handle aborts every in-flight session with it (the
/// `JoinSet` drops), which releases their snapshots — the exclusivity
/// [`retire`] requires.
pub fn spawn_accept_loop(endpoint: Endpoint, cmd: mpsc::Sender<Command>) -> JoinHandle<()> {
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
            sessions.spawn(async move {
                let _permit = permit;
                let Ok(conn) = incoming.await else { return };
                let Ok((send, recv)) = timeout(timers::SESSION_TIMEOUT, conn.accept_bi())
                    .await
                    .map_err(anyhow::Error::from)
                    .and_then(|r| r.map_err(Into::into))
                else {
                    return;
                };
                let ok = timeout(timers::SESSION_TIMEOUT, run_stream(&conn, send, recv, &cmd))
                    .await
                    .map(|r| r.is_ok())
                    .unwrap_or(false);
                let _ = cmd.send(Command::SessionOutcome { ok }).await;
            });
        }
    })
}

/// Initiate gossip as a Poisson process: sleep an exponentially distributed
/// delay, pick a uniformly random live peer (replicated presence plus the
/// manual dial targets), run one session, repeat. Failed peers are left
/// alone for [`timers::PEER_BACKOFF`].
pub fn spawn_scheduler(
    endpoint: Endpoint,
    cmd: mpsc::Sender<Command>,
    view: watch::Receiver<Arc<View>>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let me = endpoint.id();
        let mut rng = StdRng::from_entropy();
        let mut backoff: HashMap<PeerId, Instant> = HashMap::new();
        loop {
            tokio::time::sleep(poisson_delay(&mut rng)).await;
            let now = Instant::now();
            backoff.retain(|_, until| *until > now);
            let Some(peer) = pick_peer(&view.borrow(), &backoff, &me, &mut rng) else {
                continue;
            };
            let Ok(target) = EndpointId::from_bytes(&peer) else {
                // Replicated bytes that are not a valid public key: a peer
                // published garbage. Skip it forever via backoff.
                backoff.insert(peer, now + Duration::from_secs(u64::MAX / 4));
                continue;
            };
            let ok = timeout(
                timers::SESSION_TIMEOUT,
                dial_session(&endpoint, target, &cmd),
            )
            .await
            .map(|r| r.is_ok())
            .unwrap_or(false);
            if !ok {
                backoff.insert(peer, Instant::now() + timers::PEER_BACKOFF);
            }
            if cmd.send(Command::SessionOutcome { ok }).await.is_err() {
                return; // owner gone: shutting down
            }
        }
    })
}

/// One exponentially distributed gossip delay with mean
/// [`timers::GOSSIP_MEAN_INTERVAL`], clamped to the configured bounds.
fn poisson_delay(rng: &mut StdRng) -> Duration {
    // (0, 1]: never ln(0). (`r#gen`: `gen` is a keyword in edition 2024.)
    let u: f64 = 1.0 - rng.r#gen::<f64>();
    let mean = timers::GOSSIP_MEAN_INTERVAL.as_secs_f64();
    Duration::from_secs_f64(-mean * u.ln())
        .clamp(timers::GOSSIP_DELAY_MIN, timers::GOSSIP_DELAY_MAX)
}

/// A uniformly random gossip target: anyone in the roster or the manual
/// dial targets, except ourselves and anyone backed off.
fn pick_peer(
    view: &View,
    backoff: &HashMap<PeerId, Instant>,
    me: &EndpointId,
    rng: &mut StdRng,
) -> Option<PeerId> {
    let candidates: Vec<PeerId> = view
        .roster
        .iter()
        .map(|p| p.peer)
        .chain(view.dial_targets.iter().copied())
        .filter(|p| p != me.as_bytes() && !backoff.contains_key(p))
        .collect();
    if candidates.is_empty() {
        None
    } else {
        Some(candidates[rng.gen_range(0..candidates.len())])
    }
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
    /// hold our party, so we must not retry (two generals). At worst the
    /// region leaks; it is never duplicated.
    Uncertain,
    /// No candidate could absorb us; the region leaks with us.
    Leaked,
}

/// Retire into the first willing candidate, walking the list in presence
/// recency order. The caller hands us the unique `Peer` — the
/// `Peer`/`Rumors` XOR means no session can be using the set while we
/// hold it, so retirement's exclusivity holds by construction.
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
        let Ok((mut send, mut recv)) = conn.open_bi().await else {
            continue;
        };

        match retiree.retire(&mut recv, &mut send).await {
            // Retired: we are gone; the peer holds our party.
            Retire::Retired => {
                let _ = send.finish();
                conn.close(0u32.into(), b"retired");
                return Departure::Retired { into: target };
            }
            // Declined (the peer was itself retiring): intact, try the
            // next candidate.
            Retire::Declined { peer: intact } => {
                retiree = intact;
                conn.close(0u32.into(), b"declined");
            }
            // Failed before the party frame: intact, try elsewhere.
            Retire::Recovered { peer: intact, .. } => {
                retiree = intact;
                conn.close(0u32.into(), b"recovered");
            }
            // Failed during the party frame: the peer may hold our
            // party. Retrying could duplicate the region; stop here.
            Retire::Uncertain { .. } => {
                conn.close(0u32.into(), b"uncertain");
                return Departure::Uncertain;
            }
        }
    }
    Departure::Leaked
}

#[cfg(test)]
mod tests;
