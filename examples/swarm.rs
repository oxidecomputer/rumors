//! An interactive gossip swarm: a virtual network of parties churning messages
//! and reconciling over the real wire protocol, steered live from a terminal
//! UI.
//!
//! # What it does
//!
//! `seed_messages` random `Vec<u8>` payloads are inserted into one seed
//! [`Rumors`] *before any fork*, so every party starts from the same shared
//! observations. The seed is then forked (via `bootstrap_fork`) into `parties`
//! disjoint peers, one per OS thread, and the party count is itself tunable
//! live (see *Dynamic membership* below). Each party thread runs a tight loop:
//!
//! 1. **Serve** any inbound sync requests waiting in its inbox (it is the
//!    *responder* for a peer that chose it).
//! 2. If its Poisson sync timer has fired, **initiate** a sync with a random
//!    other party (it is the *initiator*).
//! 3. Otherwise, run the **steady-state controller**: compare the number of
//!    messages it currently knows about to the target and, with a probability
//!    derived from that gap, either inject a fresh random message or redact a
//!    key it already knows about.
//!
//! Each party keeps its *own* `Vec<Key>` of every key it has observed — fed
//! by a [`Messages`] observer that replays its rumor set from genesis and
//! then yields its own inserts and everything learned over the wire alike —
//! so redactions may evict messages originally published by *other* parties,
//! and the contagion spreads on the next sync. The key vector is strictly
//! per-thread: there is no shared rumor-set state and therefore no lock
//! contention on the hot path.
//!
//! # Steady-state controller
//!
//! Left to a fair coin, the live-message count random-walks. Instead each node
//! sets its probability of *adding* (versus redacting) from its current live
//! count `L` against the target `T`:
//!
//! ```text
//! p_add = T / (T + L)
//! ```
//!
//! At `L = 0` it always adds; at `L = T` the odds are even; as `L` grows past
//! `T` adding becomes rare. The fixed point is `L = T`, so the network's live
//! set settles around the target. Because redactions and inserts both
//! propagate, every node's `L` tracks the *global* live count once converged —
//! so `T` is effectively the steady-state size of the whole network's rumor
//! set, tunable live from the UI.
//!
//! # Synchronization uses the wire protocol
//!
//! Syncs go through [`Rumors::gossip`] over an in-memory [`tokio::io::duplex`]
//! pipe — the *same* bytes-on-the-wire path a TCP peer would drive. Both ends
//! of a session run `gossip` concurrently on their own thread's
//! current-thread runtime, exactly as two networked peers would.
//!
//! # Rendezvous without deadlock
//!
//! Two parties that pick each other at the same instant must not both block
//! waiting for the other to respond. A single [`AtomicBool`] per party — its
//! *engaged* flag — makes each party a participant in at most one session at a
//! time. An initiator claims itself and its peer with compare-and-swap before
//! sending; a party that is already engaged (claimed as a responder, or
//! initiating elsewhere) cannot be claimed, so the initiator simply backs off
//! and does local work instead. No party is ever simultaneously a blocked
//! initiator *and* an owed responder, so the wait-for graph has no cycle.
//!
//! # Dynamic membership
//!
//! The party count is live-tunable, and growing or shrinking it exercises the
//! bootstrap/[`retire`](Peer::retire) algebra directly. A single coordinator
//! thread — the only writer of the peer directory — watches the desired count
//! against the live one and reconciles a step at a time:
//!
//! - **Grow:** pick a random live party, hand it a *fork* command. It mints a
//!   disjoint child of its own [`Rumors`] (via `bootstrap_fork`), ships the child
//!   back to the coordinator, and keeps running; the coordinator spawns a fresh
//!   thread for the child. The parent and child are disjoint sub-parties, so the
//!   directory stays a partition of the seed's party space.
//! - **Shrink:** pick two random parties, hand each a *wind-down* command.
//!   Each finishes any owed session, locks itself out of new ones, ships its
//!   [`Rumors`] back, and exits. The coordinator [`retire`](Peer::retire)s
//!   one into the other over an in-memory wire — the session's gossip round
//!   carries any divergent content across, and the survivor absorbs the
//!   retiree's party region, so the id-space is reclaimed rather than leaked
//!   — and starts the survivor in a new thread, for a net loss of one.
//!
//! Because every live party is a disjoint fork of the common seed, any two can
//! always reconcile, so shrink never fails. The directory itself is an
//! [`ArcSwap`], so the sync hot path reads it without locking; only the
//! coordinator ever swaps it, one membership change at a time. The floor is two
//! parties — there is no one to gossip with below that.
//!
//! # The readout
//!
//! The UI reports, as windowed rates over each refresh interval:
//!
//! - **local ops/s** — inserts + redactions, per party and in aggregate;
//! - **wire bandwidth** — bytes/s per direction, averaged across every wire
//!   and both directions, auto-scaled to B/KiB/MiB/…;
//! - **sync latency** — mean wall-clock duration of one end-to-end gossip
//!   session, measured by the initiator from the instant the exchange begins
//!   to the instant it returns (never derived from the Poisson schedule);
//! - **roundtrips/sync** — mean number of request→response turns per session,
//!   counted from write→read direction flips on the initiator's I/O;
//! - **live messages/node** — the current rumor-set size, charted against the
//!   target so you can watch the controller converge.
//!
//! # Controls
//!
//! `↑`/`↓` select a parameter, `←`/`→` adjust it (`Shift` for a coarse step),
//! `space` pauses all churn, `q` quits.

use std::collections::VecDeque;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::task::{Context, Poll};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use arc_swap::ArcSwap;
use clap::Parser;
use futures::FutureExt;
use rand::rngs::SmallRng;
use rand::{Rng, RngCore, SeedableRng};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Sparkline};
use rumors::{Key, Messages, Peer, Retire, Rumors};
use tokio::io::{AsyncRead, AsyncWrite, DuplexStream, ReadBuf};

/// Mint a genuine party-disjoint peer that inherits `parent`'s content.
///
/// Every party in the swarm — which independently `send`s, `redact`s, and
/// `gossip`s — needs its own disjoint Interval Tree Clock region. We mint one
/// by serving a bootstrap from `parent` over an in-memory duplex: the
/// newcomer pulls `parent`'s whole tree through the ordinary mirror descent
/// and is handed a fresh disjoint party, forked in the same critical section
/// that snapshots the served tree. Both halves run concurrently on
/// `runtime`'s single current thread via [`tokio::join!`].
fn bootstrap_fork(runtime: &tokio::runtime::Runtime, parent: &Rumors<Payload>) -> Rumors<Payload> {
    let (a, b) = tokio::io::duplex(16 * 1024);
    let (mut a_r, mut a_w) = tokio::io::split(a);
    let (mut b_r, mut b_w) = tokio::io::split(b);
    let (served, newcomer) = runtime.block_on(async {
        tokio::join!(
            parent.gossip(&mut a_r, &mut a_w),
            Peer::<Payload>::bootstrap(&mut b_r, &mut b_w),
        )
    });
    served.expect("serve bootstrap");
    newcomer
        .expect("bootstrap newcomer")
        .expect("provider served bootstrap")
        .into_rumors()
}

/// Message payload type: opaque, randomized bytes. Borsh serializes `Vec<u8>`
/// as a length-prefixed blob, so the wire cost tracks the message size
/// directly.
type Payload = Vec<u8>;

/// One endpoint of a sync session, handed from an initiator to the responder
/// it claimed. The responder splits it and drives `gossip` on its own thread.
type SessionEnd = DuplexStream;

/// An interactive gossip swarm with a live throughput readout.
#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// Initial number of parties (one OS thread each). Must be at least 2.
    /// Adjustable live: growing forks a new party, shrinking joins two.
    #[arg(long, default_value_t = 8)]
    parties: usize,

    /// Initial expected seconds between a party's syncs. Inter-sync gaps are
    /// drawn from an exponential distribution with this mean, so syncs form a
    /// Poisson process per party. Adjustable live.
    #[arg(long, default_value_t = 0.25)]
    sync_interval: f64,

    /// Messages inserted into the shared seed before the first fork.
    #[arg(long, default_value_t = 100)]
    seed_messages: usize,

    /// Initial steady-state target for the network's live message count.
    /// Adjustable live.
    #[arg(long, default_value_t = 100)]
    target: u64,

    /// Initial size in bytes of each randomized message payload. Adjustable
    /// live.
    #[arg(long, default_value_t = 256)]
    message_size: usize,

    /// Capacity in bytes of each in-memory duplex pipe. Smaller values
    /// exercise more backpressure; larger values fewer roundtrips.
    #[arg(long, default_value_t = 16 * 1024)]
    duplex_capacity: usize,

    /// UI refresh interval in milliseconds.
    #[arg(long, default_value_t = 200)]
    refresh_ms: u64,
}

/// Live-tunable knobs shared with every party thread. Read on the hot path, so
/// every field is a lock-free atomic.
struct Controls {
    /// Mean microseconds between a party's syncs (exponential inter-arrival).
    sync_interval_us: AtomicU64,
    /// Per-node target live-message count for the steady-state controller.
    target: AtomicU64,
    /// Size in bytes of freshly injected messages.
    message_size: AtomicU64,
    /// Desired number of parties. The coordinator reconciles the live party
    /// count toward this by forking (to grow) or joining (to shrink).
    parties: AtomicU64,
    /// While true, parties stop churning and initiating (but still serve
    /// in-flight syncs).
    paused: AtomicBool,
}

/// Process-wide counters, sampled by the UI. All monotonic since start; the UI
/// differences successive snapshots to get windowed rates.
#[derive(Default)]
struct Metrics {
    /// Local inserts + redactions completed across all parties.
    local_ops: AtomicU64,
    /// Total bytes written to every wire, both directions (each byte counted
    /// once, on the writer that produced it).
    wire_bytes: AtomicU64,
    /// Sum over completed sessions of `2 * duration_nanos`: one direction-span
    /// per direction. Pairs with `wire_bytes` to give a time-weighted mean
    /// per-direction bandwidth.
    wire_direction_nanos: AtomicU64,
    /// Number of completed sync sessions (counted once each, by the initiator).
    syncs: AtomicU64,
    /// Sum of session wall-clock durations in nanos (end-to-end latency).
    sync_nanos: AtomicU64,
    /// Sum of request→response roundtrips across all sessions.
    roundtrips: AtomicU64,
    /// Sessions still in flight. Drains to zero during shutdown before any
    /// thread is allowed to exit, so no party is left blocked on a peer that
    /// already quit.
    inflight: AtomicU64,
}

impl Metrics {
    /// Record one completed session from the initiator's vantage: its
    /// wall-clock `duration` and the `roundtrips` observed on its I/O.
    fn record_sync(&self, duration: Duration, roundtrips: u64) {
        let nanos = duration.as_nanos() as u64;
        self.syncs.fetch_add(1, Ordering::Relaxed);
        self.sync_nanos.fetch_add(nanos, Ordering::Relaxed);
        // Two directions, each spanning the whole session.
        self.wire_direction_nanos
            .fetch_add(nanos.saturating_mul(2), Ordering::Relaxed);
        self.roundtrips.fetch_add(roundtrips, Ordering::Relaxed);
    }
}

/// Per-party coordination state, shared across all threads. Holds no rumor-set
/// data — only the inbox to deliver session endpoints, the engaged flag that
/// serializes each party into one session at a time, and a gauge of its
/// current live-message count for the UI.
struct SwarmPeer {
    /// Stable identity for this party, unique across the whole run (never
    /// reused, even after a party retires). Used to exclude self when picking a
    /// peer and to splice the directory on membership changes.
    id: u64,
    /// Set while this party is a participant in a session (initiator or
    /// responder). A party can be claimed only when this is `false`. A retiring
    /// party also sets it, permanently, to lock out new claims before it exits.
    engaged: AtomicBool,
    /// Inbound session endpoints, pushed by initiators that claimed this party.
    inbox: Sender<SessionEnd>,
    /// Membership commands from the coordinator (fork to grow, retire to
    /// shrink). Served at the top of the party loop, between sessions.
    control: Sender<Command>,
    /// This party's current live-message count, republished every loop.
    live: AtomicU64,
}

/// A membership command sent by the coordinator to a single party. Each carries
/// a one-shot reply channel for the party to hand its [`Rumors`] back.
enum Command {
    /// Fork off a child party and reply with it; the recipient keeps running.
    Fork { reply: Sender<Donation> },
    /// Finish any owed session, lock out new ones, reply with this party's own
    /// state, and exit the thread.
    WindDown { reply: Sender<Donation> },
}

/// A party's [`Rumors`] handed back to the coordinator. The key pool is not
/// carried along: the receiving thread rebuilds it by replaying the set
/// through a fresh [`Messages`] observer.
struct Donation {
    rumors: Rumors<Payload>,
}

/// Everything the party threads share: the peer directory, live controls,
/// run/stop flags, and the metrics counters.
struct Net {
    /// The live peer directory. Read locklessly on the sync hot path; swapped
    /// only by the coordinator, one membership change at a time. Each entry is
    /// an `Arc` so a party claimed for a session survives its removal from the
    /// directory.
    peers: ArcSwap<Vec<Arc<SwarmPeer>>>,
    controls: Controls,
    metrics: Metrics,
    /// Capacity of each in-memory duplex pipe. Immutable for the run.
    duplex_capacity: usize,
    /// While true, parties may initiate new syncs. Cleared first at shutdown.
    running: AtomicBool,
    /// While false, parties keep looping; set once all in-flight sessions have
    /// drained, after which every thread breaks.
    shutdown: AtomicBool,
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    assert!(args.parties >= 2, "need at least 2 parties to gossip");
    assert!(args.message_size > 0, "message size must be positive");

    // Seed the shared rumor set, then fork one disjoint party per thread. The
    // seed's keys are shared by every party, so any party may redact them
    // (each thread learns them by replaying its set through an observer).
    let seed_runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("build seed runtime");
    let seed: Rumors<Payload> = Peer::seed().into_rumors();
    {
        let mut rng = SmallRng::from_entropy();
        let mut batch = seed.batch();
        for _ in 0..args.seed_messages {
            batch.send(random_message(&mut rng, args.message_size));
        }
    }
    // Every party starts as a disjoint fork of the seed: same observations,
    // its own party region. The seed party itself only serves the initial
    // bootstraps; its forks do all the gossiping.
    let initial: Vec<Donation> = (0..args.parties)
        .map(|_| Donation {
            rumors: bootstrap_fork(&seed_runtime, &seed),
        })
        .collect();
    drop(seed);

    // The directory starts empty; the coordinator populates it as it launches
    // the initial parties, then keeps it reconciled with the desired count.
    let net = Arc::new(Net {
        peers: ArcSwap::from_pointee(Vec::new()),
        controls: Controls {
            sync_interval_us: AtomicU64::new((args.sync_interval * 1e6) as u64),
            target: AtomicU64::new(args.target),
            message_size: AtomicU64::new(args.message_size as u64),
            parties: AtomicU64::new(args.parties as u64),
            paused: AtomicBool::new(false),
        },
        metrics: Metrics::default(),
        duplex_capacity: args.duplex_capacity,
        running: AtomicBool::new(true),
        shutdown: AtomicBool::new(false),
    });

    // The coordinator owns every party thread's lifecycle: it launches the
    // initial set, forks/joins to track the desired count, and hands back all
    // outstanding join handles once `running` is cleared.
    let coordinator = {
        let net = Arc::clone(&net);
        thread::Builder::new()
            .name("coordinator".to_string())
            .spawn(move || run_coordinator(net, initial))
            .expect("spawn coordinator thread")
    };

    // Run the interactive UI on the main thread until the user quits. Always
    // restore the terminal, even on error or panic.
    let result = run_ui(&net, &args);

    // Shutdown: stop new syncs and membership changes. Clearing `running`
    // ends the coordinator's reconcile loop; it hands back every party's join
    // handle. Then wait for in-flight sessions to drain so no party is blocked
    // on a peer that already quit, and finally release the threads.
    net.running.store(false, Ordering::SeqCst);
    let handles = coordinator.join().expect("join coordinator thread");
    while net.metrics.inflight.load(Ordering::SeqCst) > 0 {
        thread::sleep(Duration::from_millis(1));
    }
    net.shutdown.store(true, Ordering::SeqCst);
    for handle in handles {
        handle.join().expect("join party thread");
    }

    result
}

// --- party engine ----------------------------------------------------------

/// One party's main loop: serve inbound syncs, obey membership commands, fire
/// scheduled syncs, and otherwise churn the local rumor set under the
/// steady-state controller, until wound down or shut down.
///
/// `me` is this party's own directory entry, held directly so the hot path
/// never has to look itself up.
///
/// The redaction key pool is fed by a [`Messages`] observer from genesis: the
/// initial drain replays everything the party inherited (the seed content, or
/// a fork parent's whole set), and each loop's drain picks up its own inserts
/// and everything learned over the wire, exactly once each.
fn run_party(
    net: Arc<Net>,
    me: Arc<SwarmPeer>,
    rumors: Rumors<Payload>,
    inbox: Receiver<SessionEnd>,
    control: Receiver<Command>,
) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("build party runtime");
    let mut rng = SmallRng::from_entropy();
    let mut next_sync = Instant::now() + exponential(&mut rng, &net.controls);
    let mut observer = rumors.messages();
    let mut keys: Vec<Key> = Vec::new();

    loop {
        // Catch the key pool up with everything observed since the last turn,
        // and republish our live-message count for the UI gauge.
        drain_keys(&mut observer, &mut keys);
        me.live
            .store(rumors.snapshot().len() as u64, Ordering::Relaxed);

        // 1. Serve every inbound session. Our engaged flag was set true by the
        //    initiator that claimed us; clear it once we have reconciled.
        while let Ok(end) = inbox.try_recv() {
            serve_sync(&runtime, &net, &rumors, end);
            me.engaged.store(false, Ordering::Release);
        }

        // 2. Obey membership commands from the coordinator, between sessions.
        while let Ok(cmd) = control.try_recv() {
            match cmd {
                Command::Fork { reply } => {
                    // Mint a genuine disjoint child that inherits our content,
                    // so it can independently churn and gossip; its thread
                    // rebuilds the key pool by observer replay. We keep
                    // running unchanged.
                    let child = bootstrap_fork(&runtime, &rumors);
                    let _ = reply.send(Donation { rumors: child });
                }
                Command::WindDown { reply } => {
                    // Finish anything owed, lock ourselves out of new claims,
                    // then hand our whole state over and exit.
                    let donation = wind_down(&runtime, &net, &me, &inbox, rumors);
                    let _ = reply.send(donation);
                    return;
                }
            }
        }

        if net.shutdown.load(Ordering::SeqCst) {
            break;
        }

        // While paused, keep serving inbound (above) but generate nothing.
        if net.controls.paused.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(1));
            continue;
        }

        // 3. Time to initiate a sync? Only while still running, and only if we
        //    can claim both ourselves and a peer.
        if net.running.load(Ordering::SeqCst) && Instant::now() >= next_sync {
            if try_initiate(&runtime, &net, &me, &mut rng, &rumors) {
                next_sync = Instant::now() + exponential(&mut rng, &net.controls);
            }
            // If the claim failed (peer busy), fall through to local work and
            // retry on the next iteration; next_sync stays in the past.
            continue;
        }

        // 4. Local churn under the steady-state controller.
        local_op(&net, &mut rng, &rumors, &mut keys);
    }
}

/// Pull every message the observer has pending — without blocking — and push
/// its key into the pool. Each message is yielded exactly once across the
/// party's lifetime, so the pool never holds duplicates.
fn drain_keys(observer: &mut Messages<Payload>, keys: &mut Vec<Key>) {
    while let Some(Some((key, _, _))) = observer.borrow_next().now_or_never() {
        keys.push(key);
    }
}

/// Wind a party down so the coordinator can absorb it. Serves any owed session,
/// then claims our own engaged flag — permanently — so no initiator can open a
/// new session with us. Because an initiator sets a peer's flag *before*
/// delivering the session, a successful claim here proves nothing is owed; if
/// the claim loses to an initiator, we serve that session and retry. Returns
/// our state for the coordinator to fork from or retire.
fn wind_down(
    runtime: &tokio::runtime::Runtime,
    net: &Net,
    me: &SwarmPeer,
    inbox: &Receiver<SessionEnd>,
    rumors: Rumors<Payload>,
) -> Donation {
    loop {
        while let Ok(end) = inbox.try_recv() {
            serve_sync(runtime, net, &rumors, end);
            me.engaged.store(false, Ordering::Release);
        }
        if me
            .engaged
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            break;
        }
        // An initiator holds us; its session is arriving. Yield and serve it.
        thread::yield_now();
    }
    // Locked: nothing new can arrive. Drain any straggler for good measure.
    while let Ok(end) = inbox.try_recv() {
        serve_sync(runtime, net, &rumors, end);
    }
    Donation { rumors }
}

/// Attempt to initiate a sync with a random other party. Returns `true` if a
/// session actually ran (both claims succeeded), `false` if the peer was busy
/// or there was no one to pick.
fn try_initiate(
    runtime: &tokio::runtime::Runtime,
    net: &Net,
    me: &Arc<SwarmPeer>,
    rng: &mut SmallRng,
    rumors: &Rumors<Payload>,
) -> bool {
    // Claim ourselves first. If we are already engaged (a responder claimed us
    // between the inbox drain and now), back off.
    if me
        .engaged
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return false;
    }

    // Snapshot the directory and pick a peer that is not us. Cloning the chosen
    // `Arc` lets us drop the snapshot immediately; the peer stays alive for the
    // whole session even if the coordinator removes it from the directory.
    let peer = match pick_peer(rng, &net.peers.load(), me.id) {
        Some(peer) => peer,
        None => {
            me.engaged.store(false, Ordering::Release);
            return false;
        }
    };

    // Claim the peer. If it is engaged, release ourselves and back off.
    if peer
        .engaged
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        me.engaged.store(false, Ordering::Release);
        return false;
    }

    // Both claimed. Reserve an in-flight slot before the final `running` check:
    // shutdown first clears `running`, then waits for this counter to drain, so
    // it cannot miss a session that is about to start.
    net.metrics.inflight.fetch_add(1, Ordering::SeqCst);
    if !net.running.load(Ordering::SeqCst) {
        peer.engaged.store(false, Ordering::Release);
        me.engaged.store(false, Ordering::Release);
        net.metrics.inflight.fetch_sub(1, Ordering::SeqCst);
        return false;
    }

    // Hand the peer one end of a fresh pipe and gossip the other.
    let (mine, theirs) = tokio::io::duplex(net.duplex_capacity);
    if peer.inbox.send(theirs).is_err() {
        peer.engaged.store(false, Ordering::Release);
        me.engaged.store(false, Ordering::Release);
        net.metrics.inflight.fetch_sub(1, Ordering::SeqCst);
        return false;
    }

    let rounds = Arc::new(Rounds::default());
    let (read_half, write_half) = tokio::io::split(mine);
    let mut reader = CountRead {
        inner: read_half,
        rounds: Some(Arc::clone(&rounds)),
    };
    let mut writer = CountWrite {
        inner: write_half,
        wire_bytes: &net.metrics.wire_bytes,
        rounds: Some(Arc::clone(&rounds)),
    };

    // Latency is the wall-clock span of the gossip exchange itself: `start` is
    // taken immediately before the protocol runs and `elapsed` immediately
    // after it returns. It is never derived from the Poisson schedule.
    // (Learned keys surface through the party's observer on its next drain.)
    let start = Instant::now();
    runtime
        .block_on(rumors.gossip(&mut reader, &mut writer))
        .expect("initiator gossip");
    let elapsed = start.elapsed();

    net.metrics.record_sync(elapsed, rounds.roundtrips());
    me.engaged.store(false, Ordering::Release);
    net.metrics.inflight.fetch_sub(1, Ordering::SeqCst);
    true
}

/// Drive the responder side of a session that some initiator opened with us.
/// (Learned keys surface through the party's observer on its next drain.)
fn serve_sync(
    runtime: &tokio::runtime::Runtime,
    net: &Net,
    rumors: &Rumors<Payload>,
    end: SessionEnd,
) {
    let (read_half, write_half) = tokio::io::split(end);
    // The responder counts only the bytes it writes (its outbound direction);
    // the initiator counts the other direction. Roundtrips are tallied by the
    // initiator alone, so the responder needs no `Rounds`.
    let mut reader = read_half;
    let mut writer = CountWrite {
        inner: write_half,
        wire_bytes: &net.metrics.wire_bytes,
        rounds: None,
    };
    runtime
        .block_on(rumors.gossip(&mut reader, &mut writer))
        .expect("responder gossip");
}

/// Perform one unit of local churn under the steady-state controller. The
/// probability of adding (rather than redacting) is `target / (target + live)`,
/// so the live set is driven toward `target`. Falls back to an insert when
/// there is nothing to redact.
fn local_op(net: &Net, rng: &mut SmallRng, rumors: &Rumors<Payload>, keys: &mut Vec<Key>) {
    let target = net.controls.target.load(Ordering::Relaxed) as f64;
    let live = rumors.snapshot().len() as f64;
    // p_add = T / (T + L): 1.0 when empty, 0.5 at target, → 0 when far over.
    let p_add = if target <= 0.0 {
        0.0
    } else {
        target / (target + live)
    };

    if keys.is_empty() || rng.gen_bool(p_add.clamp(0.0, 1.0)) {
        let size = net.controls.message_size.load(Ordering::Relaxed) as usize;
        // The minted key reaches the pool through the observer's next drain.
        rumors.send(random_message(rng, size));
    } else {
        // Swap-remove a random key and redact it: the key leaves our local
        // view and the redaction propagates on our next sync. A key already
        // redacted by another party simply makes this a no-op, and either way
        // it leaves our vector, so stale keys drain out over time.
        let idx = rng.gen_range(0..keys.len());
        let key = keys.swap_remove(idx);
        rumors.redact(key);
    }
    net.metrics.local_ops.fetch_add(1, Ordering::Relaxed);
}

/// Draw an exponential inter-arrival time with the current mean, so successive
/// syncs form a Poisson process. `1 - u` keeps the log argument in `(0, 1]`,
/// avoiding `ln(0)`.
fn exponential(rng: &mut SmallRng, controls: &Controls) -> Duration {
    let mean_secs = controls.sync_interval_us.load(Ordering::Relaxed) as f64 / 1e6;
    let u: f64 = rng.r#gen();
    let secs = -mean_secs * (1.0 - u).ln();
    Duration::from_secs_f64(secs.max(0.0))
}

/// A fresh random payload of `size` bytes.
fn random_message(rng: &mut SmallRng, size: usize) -> Payload {
    let mut buf = vec![0u8; size];
    rng.fill_bytes(&mut buf);
    buf
}

/// Uniformly pick a directory entry whose `id` is not `me`, cloning its `Arc`.
/// Returns `None` when there is no other party to pick (fewer than two live).
fn pick_peer(rng: &mut SmallRng, dir: &[Arc<SwarmPeer>], me: u64) -> Option<Arc<SwarmPeer>> {
    if dir.len() < 2 {
        return None;
    }
    loop {
        let peer = &dir[rng.gen_range(0..dir.len())];
        if peer.id != me {
            return Some(Arc::clone(peer));
        }
    }
}

// --- membership coordinator ------------------------------------------------

/// The coordinator thread. The sole writer of the peer directory: it launches
/// the initial parties, then reconciles the live party count toward the desired
/// one by forking (to grow) or retiring one party into another (to shrink),
/// one step per iteration.
///
/// Returns every outstanding party join handle once `running` is cleared, so
/// the main thread can join them after the in-flight sessions drain.
fn run_coordinator(net: Arc<Net>, initial: Vec<Donation>) -> Vec<JoinHandle<()>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("build coordinator runtime");
    let mut next_id: u64 = 0;
    let mut handles = Vec::with_capacity(initial.len());
    let mut peers = Vec::with_capacity(initial.len());
    for donation in initial {
        let (peer, handle) = launch_party(&net, &mut next_id, donation);
        peers.push(peer);
        handles.push(handle);
    }
    net.peers.store(Arc::new(peers));

    let mut rng = SmallRng::from_entropy();
    while net.running.load(Ordering::SeqCst) {
        let desired = net.controls.parties.load(Ordering::Relaxed) as usize;
        let current = net.peers.load().len();
        if desired > current {
            grow(&net, &mut rng, &mut next_id, &mut handles);
        } else if desired < current {
            shrink(&runtime, &net, &mut rng, &mut next_id, &mut handles);
        } else {
            // Balanced: nothing to do until the knob or the loser of a claim
            // race changes things. Poll at a human-noticeable cadence.
            thread::sleep(Duration::from_millis(20));
        }
    }
    handles
}

/// Build a party's directory entry and channels, spawn its thread, and return
/// the shared [`SwarmPeer`] alongside the join handle. Assigns the next unique
/// id.
fn launch_party(
    net: &Arc<Net>,
    next_id: &mut u64,
    donation: Donation,
) -> (Arc<SwarmPeer>, JoinHandle<()>) {
    let id = *next_id;
    *next_id += 1;
    let (inbox_tx, inbox_rx) = channel::<SessionEnd>();
    let (control_tx, control_rx) = channel::<Command>();
    let peer = Arc::new(SwarmPeer {
        id,
        engaged: AtomicBool::new(false),
        inbox: inbox_tx,
        control: control_tx,
        live: AtomicU64::new(donation.rumors.snapshot().len() as u64),
    });
    let handle = {
        let net = Arc::clone(net);
        let peer = Arc::clone(&peer);
        let Donation { rumors } = donation;
        thread::Builder::new()
            .name(format!("party-{id}"))
            .spawn(move || run_party(net, peer, rumors, inbox_rx, control_rx))
            .expect("spawn party thread")
    };
    (peer, handle)
}

/// Grow by one: fork a random live party's [`Rumors`] and run the child in a
/// new thread. The child and parent are disjoint sub-parties, so the directory
/// stays a valid partition of the seed's party space.
fn grow(net: &Arc<Net>, rng: &mut SmallRng, next_id: &mut u64, handles: &mut Vec<JoinHandle<()>>) {
    let dir = net.peers.load_full();
    if dir.is_empty() {
        return;
    }
    let parent = &dir[rng.gen_range(0..dir.len())];
    let (reply_tx, reply_rx) = channel::<Donation>();
    if parent
        .control
        .send(Command::Fork { reply: reply_tx })
        .is_err()
    {
        return; // parent already gone; try again next tick
    }
    let Ok(child) = reply_rx.recv() else {
        return;
    };
    let (peer, handle) = launch_party(net, next_id, child);
    handles.push(handle);

    let mut peers = (*dir).clone();
    peers.push(peer);
    net.peers.store(Arc::new(peers));
}

/// Shrink by one: wind down two random parties, [`retire`](Peer::retire)
/// one into the other over an in-memory wire, and run the survivor in a new
/// thread. The retire session's gossip round carries any divergent content
/// across before the party hand-off, and the survivor absorbs the retiree's
/// id-region — the merge is leak-free. Any two live parties are disjoint
/// forks of the common seed, so the session always commits.
fn shrink(
    runtime: &tokio::runtime::Runtime,
    net: &Arc<Net>,
    rng: &mut SmallRng,
    next_id: &mut u64,
    handles: &mut Vec<JoinHandle<()>>,
) {
    let dir = net.peers.load_full();
    if dir.len() <= 2 {
        return; // keep at least two parties: there must be someone to gossip with
    }
    // Pick two distinct entries.
    let i = rng.gen_range(0..dir.len());
    let mut j = rng.gen_range(0..dir.len() - 1);
    if j >= i {
        j += 1;
    }
    let (a, b) = (Arc::clone(&dir[i]), Arc::clone(&dir[j]));

    // Wind both down. Sending both commands before awaiting either reply lets
    // the two parties finish their owed sessions concurrently.
    let (a_tx, a_rx) = channel::<Donation>();
    let (b_tx, b_rx) = channel::<Donation>();
    if a.control.send(Command::WindDown { reply: a_tx }).is_err()
        || b.control.send(Command::WindDown { reply: b_tx }).is_err()
    {
        return; // a party already gone; try again next tick
    }
    let (Ok(da), Ok(db)) = (a_rx.recv(), b_rx.recv()) else {
        return;
    };

    // Merge: retire b into a over an in-memory wire. The survivor's key pool
    // is rebuilt by observer replay in its new thread, so nothing but the
    // `Rumors` needs to move. Retiring requires the unique [`Peer`] handle;
    // the wound-down party's `Rumors` is the last one standing, so reclaiming
    // it resolves immediately.
    let rumors = da.rumors;
    let retiree = runtime
        .block_on(db.rumors.try_into_peer())
        .expect("a wound-down party's handle is unique");
    let (a_side, b_side) = tokio::io::duplex(net.duplex_capacity);
    let (mut a_r, mut a_w) = tokio::io::split(a_side);
    let (mut b_r, mut b_w) = tokio::io::split(b_side);
    let (retired, survived) = runtime.block_on(async {
        tokio::join!(
            retiree.retire(&mut b_r, &mut b_w),
            rumors.gossip(&mut a_r, &mut a_w),
        )
    });
    survived.expect("survivor gossip");
    assert!(
        matches!(retired, Retire::Retired),
        "two live swarm parties always reconcile, got {retired:?}"
    );

    let (peer, handle) = launch_party(net, next_id, Donation { rumors });
    handles.push(handle);

    // Swap in a directory without the two retired parties, plus the merged one.
    let mut peers: Vec<Arc<SwarmPeer>> = dir
        .iter()
        .filter(|p| p.id != a.id && p.id != b.id)
        .cloned()
        .collect();
    peers.push(peer);
    net.peers.store(Arc::new(peers));
}

// --- byte- and roundtrip-counting I/O wrappers -----------------------------

/// Direction-flip counter shared between an initiator's reader and writer. A
/// write→read flip marks one completed request→response roundtrip.
#[derive(Default)]
struct Rounds {
    inner: std::sync::Mutex<RoundState>,
}

#[derive(Default)]
struct RoundState {
    /// Whether the last I/O on this session was a write.
    last_was_write: bool,
    /// Completed write→read roundtrips.
    roundtrips: u64,
}

impl Rounds {
    fn on_write(&self) {
        self.inner.lock().unwrap().last_was_write = true;
    }

    fn on_read(&self) {
        let mut s = self.inner.lock().unwrap();
        if s.last_was_write {
            s.roundtrips += 1;
            s.last_was_write = false;
        }
    }

    fn roundtrips(&self) -> u64 {
        self.inner.lock().unwrap().roundtrips
    }
}

/// `AsyncWrite` wrapper that tallies bytes into a shared counter and, when a
/// `Rounds` is attached, records the write phase for roundtrip counting.
struct CountWrite<'a, W> {
    inner: W,
    wire_bytes: &'a AtomicU64,
    rounds: Option<Arc<Rounds>>,
}

impl<W: AsyncWrite + Unpin> AsyncWrite for CountWrite<'_, W> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        match Pin::new(&mut this.inner).poll_write(cx, buf) {
            Poll::Ready(Ok(n)) => {
                this.wire_bytes.fetch_add(n as u64, Ordering::Relaxed);
                if let Some(rounds) = &this.rounds {
                    rounds.on_write();
                }
                Poll::Ready(Ok(n))
            }
            other => other,
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}

/// `AsyncRead` wrapper that records the read phase for roundtrip counting. It
/// does not tally bytes: each byte is counted once, on the writer that sent it.
struct CountRead<R> {
    inner: R,
    rounds: Option<Arc<Rounds>>,
}

impl<R: AsyncRead + Unpin> AsyncRead for CountRead<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let before = buf.filled().len();
        match Pin::new(&mut this.inner).poll_read(cx, buf) {
            Poll::Ready(Ok(())) => {
                if buf.filled().len() > before
                    && let Some(rounds) = &this.rounds
                {
                    rounds.on_read();
                }
                Poll::Ready(Ok(()))
            }
            other => other,
        }
    }
}

// --- interactive UI --------------------------------------------------------

/// Largest party count the UI will dial up to. A guard against accidentally
/// spawning an unreasonable number of OS threads, not a protocol limit.
const MAX_PARTIES: u64 = 64;

/// The live-adjustable parameters, in selection order.
#[derive(Copy, Clone)]
enum Field {
    Parties,
    SyncInterval,
    Target,
    MessageSize,
}

impl Field {
    const ALL: [Field; 4] = [
        Field::Parties,
        Field::SyncInterval,
        Field::Target,
        Field::MessageSize,
    ];

    fn label(self) -> &'static str {
        match self {
            Field::Parties => "parties",
            Field::SyncInterval => "sync interval",
            Field::Target => "target msgs",
            Field::MessageSize => "message size",
        }
    }

    /// The current value of this field, formatted for display.
    fn value(self, controls: &Controls) -> String {
        match self {
            Field::Parties => format!("{}", controls.parties.load(Ordering::Relaxed)),
            Field::SyncInterval => {
                let ms = controls.sync_interval_us.load(Ordering::Relaxed) as f64 / 1000.0;
                format!("{ms:.0} ms")
            }
            Field::Target => format!("{}", controls.target.load(Ordering::Relaxed)),
            Field::MessageSize => format!("{} B", controls.message_size.load(Ordering::Relaxed)),
        }
    }

    /// Nudge this field. `increase` chooses direction; `coarse` chooses a
    /// larger (roughly 8–10×) step. Values are clamped to sane bounds.
    fn adjust(self, controls: &Controls, increase: bool, coarse: bool) {
        match self {
            Field::Parties => {
                let step = if coarse { 8 } else { 1 };
                // Floor of two: a lone party has no one to gossip with.
                bump(&controls.parties, increase, step, 2, MAX_PARTIES);
            }
            Field::SyncInterval => {
                let step = if coarse { 50_000 } else { 5_000 }; // microseconds
                bump(&controls.sync_interval_us, increase, step, 1_000, 5_000_000);
            }
            Field::Target => {
                let step = if coarse { 100 } else { 10 };
                bump(&controls.target, increase, step, 0, 1_000_000);
            }
            Field::MessageSize => {
                let step = if coarse { 256 } else { 32 };
                bump(&controls.message_size, increase, step, 1, 65_536);
            }
        }
    }
}

/// Add or subtract `step` from `value`, clamped to `[min, max]`.
fn bump(value: &AtomicU64, increase: bool, step: u64, min: u64, max: u64) {
    let cur = value.load(Ordering::Relaxed);
    let next = if increase {
        cur.saturating_add(step).min(max)
    } else {
        cur.saturating_sub(step).max(min)
    };
    value.store(next, Ordering::Relaxed);
}

/// A point-in-time read of the counters plus the instant it was taken.
struct Snapshot {
    at: Instant,
    local_ops: u64,
    wire_bytes: u64,
    wire_direction_nanos: u64,
    syncs: u64,
    sync_nanos: u64,
    roundtrips: u64,
}

impl Snapshot {
    fn take(net: &Net) -> Self {
        let m = &net.metrics;
        Snapshot {
            at: Instant::now(),
            local_ops: m.local_ops.load(Ordering::Relaxed),
            wire_bytes: m.wire_bytes.load(Ordering::Relaxed),
            wire_direction_nanos: m.wire_direction_nanos.load(Ordering::Relaxed),
            syncs: m.syncs.load(Ordering::Relaxed),
            sync_nanos: m.sync_nanos.load(Ordering::Relaxed),
            roundtrips: m.roundtrips.load(Ordering::Relaxed),
        }
    }
}

/// Windowed statistics derived from two snapshots and the live gauges.
struct Stats {
    ops_per_party: f64,
    ops_total: f64,
    bandwidth: f64,
    latency: String,
    sync_rate: f64,
    roundtrips: String,
    avg_live: f64,
    syncs_total: u64,
}

/// Compute windowed rates between two snapshots.
fn compute(net: &Net, prev: &Snapshot, now: &Snapshot) -> Stats {
    let dir = net.peers.load();
    let parties = (dir.len() as f64).max(1.0);
    let dt = now
        .at
        .duration_since(prev.at)
        .as_secs_f64()
        .max(f64::MIN_POSITIVE);

    let d_ops = now.local_ops - prev.local_ops;
    let ops_total = d_ops as f64 / dt;

    let d_bytes = now.wire_bytes - prev.wire_bytes;
    let d_dir_nanos = now.wire_direction_nanos - prev.wire_direction_nanos;
    let bandwidth = if d_dir_nanos > 0 {
        d_bytes as f64 / (d_dir_nanos as f64 / 1e9)
    } else {
        0.0
    };

    let d_syncs = now.syncs - prev.syncs;
    let (latency, roundtrips, sync_rate) = if d_syncs > 0 {
        let lat = Duration::from_nanos((now.sync_nanos - prev.sync_nanos) / d_syncs);
        let rt = (now.roundtrips - prev.roundtrips) as f64 / d_syncs as f64;
        (
            format_duration(lat),
            format!("{rt:.1}"),
            d_syncs as f64 / dt,
        )
    } else {
        ("--".to_string(), "--".to_string(), 0.0)
    };

    let live_total: u64 = dir.iter().map(|p| p.live.load(Ordering::Relaxed)).sum();

    Stats {
        ops_per_party: ops_total / parties,
        ops_total,
        bandwidth,
        latency,
        sync_rate,
        roundtrips,
        avg_live: live_total as f64 / parties,
        syncs_total: now.syncs,
    }
}

/// Bounded history rings for the charts.
struct History {
    live: VecDeque<u64>,
    ops: VecDeque<u64>,
    cap: usize,
}

impl History {
    fn new(cap: usize) -> Self {
        History {
            live: VecDeque::with_capacity(cap),
            ops: VecDeque::with_capacity(cap),
            cap,
        }
    }

    fn push(&mut self, live: u64, ops: u64) {
        for (ring, v) in [(&mut self.live, live), (&mut self.ops, ops)] {
            if ring.len() == self.cap {
                ring.pop_front();
            }
            ring.push_back(v);
        }
    }
}

/// Run the terminal UI until the user quits. Sets up and tears down raw mode
/// and the alternate screen, restoring the terminal on any exit path.
fn run_ui(net: &Net, args: &Args) -> io::Result<()> {
    // Restore the terminal even if a party thread (or this one) panics.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        default_hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = ui_loop(&mut terminal, net, args);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

/// The event/render loop. Renders every wake; resamples the windowed stats on
/// a steady cadence so event-driven redraws (e.g. a key press) don't compute
/// rates over a vanishingly small window.
fn ui_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    net: &Net,
    args: &Args,
) -> io::Result<()> {
    let sample_interval = Duration::from_millis(args.refresh_ms);
    let mut selected = 0usize;
    let mut history = History::new(160);
    let mut prev = Snapshot::take(net);
    let mut stats = compute(net, &prev, &Snapshot::take(net));
    let mut last_sample = Instant::now();
    let started = Instant::now();

    loop {
        if event::poll(sample_interval)?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            let coarse = key.modifiers.contains(KeyModifiers::SHIFT);
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    break;
                }
                KeyCode::Up => selected = selected.saturating_sub(1),
                KeyCode::Down => selected = (selected + 1).min(Field::ALL.len() - 1),
                KeyCode::Left => Field::ALL[selected].adjust(&net.controls, false, coarse),
                KeyCode::Right => Field::ALL[selected].adjust(&net.controls, true, coarse),
                KeyCode::Char(' ') => {
                    let p = &net.controls.paused;
                    p.store(!p.load(Ordering::SeqCst), Ordering::SeqCst);
                }
                _ => {}
            }
        }

        if last_sample.elapsed() >= sample_interval {
            let now = Snapshot::take(net);
            stats = compute(net, &prev, &now);
            history.push(
                stats.avg_live.round() as u64,
                stats.ops_total.round() as u64,
            );
            prev = now;
            last_sample = Instant::now();
        }

        terminal.draw(|frame| {
            draw(frame, net, selected, &stats, &history, started.elapsed());
        })?;
    }
    Ok(())
}

/// Paint one frame.
fn draw(
    frame: &mut ratatui::Frame,
    net: &Net,
    selected: usize,
    stats: &Stats,
    history: &History,
    elapsed: Duration,
) {
    let rows = Layout::vertical([
        Constraint::Length(3), // header
        Constraint::Length(7), // params | stats
        Constraint::Min(7),    // charts
        Constraint::Length(1), // footer
    ])
    .split(frame.area());

    draw_header(frame, rows[0], net, elapsed);

    let mid =
        Layout::horizontal([Constraint::Percentage(42), Constraint::Percentage(58)]).split(rows[1]);
    draw_params(frame, mid[0], net, selected);
    draw_stats(frame, mid[1], stats);

    draw_charts(frame, rows[2], net, stats, history);
    draw_footer(frame, rows[3]);
}

fn draw_header(frame: &mut ratatui::Frame, area: Rect, net: &Net, elapsed: Duration) {
    let paused = net.controls.paused.load(Ordering::SeqCst);
    let (status, status_style) = if paused {
        ("PAUSED", Style::default().fg(Color::Yellow))
    } else {
        ("running", Style::default().fg(Color::Green))
    };
    let line = Line::from(vec![
        Span::styled(
            " rumors swarm ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("  {} parties   ", net.peers.load().len())),
        Span::styled(status, status_style),
        Span::raw(format!("   {:.0}s", elapsed.as_secs_f64())),
    ]);
    frame.render_widget(
        Paragraph::new(line).block(Block::default().borders(Borders::ALL)),
        area,
    );
}

fn draw_params(frame: &mut ratatui::Frame, area: Rect, net: &Net, selected: usize) {
    let lines: Vec<Line> = Field::ALL
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let value = field.value(&net.controls);
            let selected = i == selected;
            let marker = if selected { "▸ " } else { "  " };
            let style = if selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Line::from(Span::styled(
                format!("{marker}{:<14}{:>10}", field.label(), value),
                style,
            ))
        })
        .collect();
    frame.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" parameters ")),
        area,
    );
}

fn draw_stats(frame: &mut ratatui::Frame, area: Rect, stats: &Stats) {
    let kv = |k: &str, v: String| {
        Line::from(vec![
            Span::styled(format!("{k:<16}"), Style::default().fg(Color::DarkGray)),
            Span::styled(v, Style::default().fg(Color::White)),
        ])
    };
    let target_hint = format!("{:.0}/node", stats.avg_live);
    let lines = vec![
        kv(
            "local ops/s",
            format!(
                "{:.0} /party  ({:.0} total)",
                stats.ops_per_party, stats.ops_total
            ),
        ),
        kv(
            "wire bandwidth",
            format!("{} /dir", format_rate(stats.bandwidth)),
        ),
        kv(
            "sync latency",
            format!("{}  ({:.1}/s)", stats.latency, stats.sync_rate),
        ),
        kv("roundtrips/sync", stats.roundtrips.clone()),
        kv("live messages", target_hint),
        kv("syncs total", format!("{}", stats.syncs_total)),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" live stats ")),
        area,
    );
}

fn draw_charts(
    frame: &mut ratatui::Frame,
    area: Rect,
    net: &Net,
    stats: &Stats,
    history: &History,
) {
    let halves =
        Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).split(area);

    let target = net.controls.target.load(Ordering::Relaxed);
    let live: Vec<u64> = history.live.iter().copied().collect();
    frame.render_widget(
        Sparkline::default()
            .block(Block::default().borders(Borders::ALL).title(format!(
                " live messages/node: {:.0}   (target {target}) ",
                stats.avg_live
            )))
            .data(&live)
            .style(Style::default().fg(Color::Cyan)),
        halves[0],
    );

    let ops: Vec<u64> = history.ops.iter().copied().collect();
    frame.render_widget(
        Sparkline::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" local ops/s: {:.0} ", stats.ops_total)),
            )
            .data(&ops)
            .style(Style::default().fg(Color::Magenta)),
        halves[1],
    );
}

fn draw_footer(frame: &mut ratatui::Frame, area: Rect) {
    let hint = Line::from(vec![
        Span::styled("  ↑/↓ ", Style::default().fg(Color::Cyan)),
        Span::raw("select   "),
        Span::styled("←/→ ", Style::default().fg(Color::Cyan)),
        Span::raw("adjust (Shift = coarse)   "),
        Span::styled("space ", Style::default().fg(Color::Cyan)),
        Span::raw("pause   "),
        Span::styled("q ", Style::default().fg(Color::Cyan)),
        Span::raw("quit"),
    ]);
    frame.render_widget(Paragraph::new(hint), area);
}

/// Format a byte-rate with a binary unit prefix.
fn format_rate(bytes_per_sec: f64) -> String {
    const UNITS: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    if !bytes_per_sec.is_finite() || bytes_per_sec <= 0.0 {
        return "0 B/s".to_string();
    }
    let mut value = bytes_per_sec;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}/s", UNITS[unit])
}

/// Format a short duration with an adaptive unit (ns / µs / ms / s).
fn format_duration(d: Duration) -> String {
    let nanos = d.as_nanos();
    if nanos < 1_000 {
        format!("{nanos} ns")
    } else if nanos < 1_000_000 {
        format!("{:.1} µs", nanos as f64 / 1e3)
    } else if nanos < 1_000_000_000 {
        format!("{:.2} ms", nanos as f64 / 1e6)
    } else {
        format!("{:.2} s", nanos as f64 / 1e9)
    }
}
