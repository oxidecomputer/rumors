//! Frame-level hop tracing over the delayed link: the hop-count instrument.
//!
//! Wraps every pipe of a delayed link pair in a byte-level tracer and runs
//! representative divergent sessions at a fixed one-way delay under the
//! paused clock. Because virtual time only advances while every task is
//! blocked on the wire, all compute is instantaneous in virtual terms and
//! every traced event lands at an exact multiple of the delay: bucket `k`
//! holds precisely the frames whose earliest enabling arrival was `k` hops
//! into the session. Printing the per-bucket, per-stream traffic therefore
//! *is* the serialized critical path — no inference required.
//!
//! Stream identity: each logical stream's first two written bytes are its
//! `[epoch, index]` label (`remote/streams.rs`), and the index maps to a
//! protocol phase per `Stream::height` (`remote/codec/signal.rs`). The
//! tracer captures each data pipe's label prefix and renders both speaker
//! interpretations; which side is initiator is evident from the trace
//! (the opening question rides the greeting, so the earliest data-stream
//! writes are the *responder's* opening reply and, when the initiator
//! holds exclusive root children, the initiator's opening supplies, each
//! on its own index 0).
//!
//! Run with:
//!
//!     cargo nextest run -E 'binary(hop_trace)' --no-capture

// Only the pipe layer is reused; the wire driver here is trace-aware.
#[allow(dead_code)]
#[path = "../benches/support/latency.rs"]
mod latency;

use std::collections::BTreeMap;
use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::{RngCore, SeedableRng};
use rumors::link::{Acceptor, Connector, Done, Link, STREAM_COUNT};
use rumors::{DEFAULT_SYNC_MEMORY_BUDGET, Peer, Protocol, Rumors, Version};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::mpsc;
use tokio::time::Instant;

use latency::{DelayedReader, DelayedWriter, delayed_pipe};

use serde::Serialize;
use serde::de::DeserializeOwned;
/// One-way link delay; whole milliseconds per the timer wheel's grain.
const DELAY: Duration = Duration::from_millis(10);

/// Per-stream in-flight byte window, far above any transfer here.
const CAPACITY: usize = 8 * 1024 * 1024;

/// Identity of one unidirectional pipe: the writing side and a serial.
///
/// Serial `CONTROL` is the side's control stream; data serials count from
/// zero in open order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct PipeId {
    side: char,
    serial: u8,
}

const CONTROL: u8 = 0xFF;

/// One completed read or write on a traced pipe.
struct Event {
    at: Duration,
    pipe: PipeId,
    write: bool,
    len: usize,
}

#[derive(Default)]
struct TraceInner {
    start: Option<Instant>,
    events: Vec<Event>,
    /// First bytes written per pipe: the `[epoch, index]` stream label.
    prefixes: BTreeMap<PipeId, Vec<u8>>,
}

/// The shared trace log; the session's virtual origin is its first event.
#[derive(Clone, Default)]
struct Trace(Arc<Mutex<TraceInner>>);

impl Trace {
    fn at(inner: &mut TraceInner) -> Duration {
        let now = Instant::now();
        let start = *inner.start.get_or_insert(now);
        now - start
    }

    fn record_write(&self, pipe: PipeId, bytes: &[u8]) {
        let mut inner = self.0.lock().unwrap();
        let at = Self::at(&mut inner);
        let prefix = inner.prefixes.entry(pipe).or_default();
        if prefix.len() < 2 {
            let take = (2 - prefix.len()).min(bytes.len());
            prefix.extend_from_slice(&bytes[..take]);
        }
        inner.events.push(Event {
            at,
            pipe,
            write: true,
            len: bytes.len(),
        });
    }

    fn record_read(&self, pipe: PipeId, len: usize) {
        let mut inner = self.0.lock().unwrap();
        let at = Self::at(&mut inner);
        inner.events.push(Event {
            at,
            pipe,
            write: false,
            len,
        });
    }

    /// The session's serialized hop count: the last event's delay multiple.
    ///
    /// Under the paused clock every event lands on an exact multiple of the
    /// one-way delay, so this is exact, not a rounding.
    fn hops(&self) -> u64 {
        let inner = self.0.lock().unwrap();
        let last = inner
            .events
            .iter()
            .map(|event| event.at)
            .max()
            .unwrap_or_default();
        let millis = last.as_millis() as u64;
        assert_eq!(
            millis % DELAY.as_millis() as u64,
            0,
            "every traced event lands on an exact delay multiple"
        );
        millis / DELAY.as_millis() as u64
    }

    /// The delay multiple of the first write on `side`'s data pipe whose
    /// label names logical stream `index`.
    fn first_write_hop(&self, side: char, index: u8) -> u64 {
        let inner = self.0.lock().unwrap();
        let pipe = inner
            .prefixes
            .iter()
            .find_map(|(pipe, prefix)| {
                (pipe.side == side && pipe.serial != CONTROL && prefix.get(1) == Some(&index))
                    .then_some(*pipe)
            })
            .unwrap_or_else(|| panic!("side {side} opened no stream labeled {index}"));
        let first = inner
            .events
            .iter()
            .filter(|event| event.pipe == pipe && event.write)
            .map(|event| event.at)
            .min()
            .expect("an opened pipe has at least its label write");
        first.as_millis() as u64 / DELAY.as_millis() as u64
    }
}

/// A write half that logs every completed write against its pipe identity.
struct TracedWriter {
    inner: DelayedWriter,
    pipe: PipeId,
    trace: Trace,
}

impl AsyncWrite for TracedWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match Pin::new(&mut self.inner).poll_write(cx, buf) {
            Poll::Ready(Ok(n)) => {
                self.trace.record_write(self.pipe, &buf[..n]);
                Poll::Ready(Ok(n))
            }
            other => other,
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// A read half that logs every nonempty delivery against its pipe identity.
struct TracedReader {
    inner: DelayedReader,
    pipe: PipeId,
    trace: Trace,
}

impl AsyncRead for TracedReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let before = buf.filled().len();
        let this = &mut *self;
        match Pin::new(&mut this.inner).poll_read(cx, buf) {
            Poll::Ready(Ok(())) => {
                let delivered = buf.filled().len() - before;
                if delivered > 0 {
                    this.trace.record_read(this.pipe, delivered);
                }
                Poll::Ready(Ok(()))
            }
            other => other,
        }
    }
}

/// The traced delayed-pipe connector: labels each minted pipe in open order.
#[derive(Clone)]
struct TracedConnector {
    announce: mpsc::Sender<TracedReader>,
    side: char,
    next: Arc<AtomicU8>,
    trace: Trace,
}

impl Connector for TracedConnector {
    type Tx = TracedWriter;

    async fn connect(&self) -> io::Result<(Self::Tx, Done<Self::Tx>)> {
        let (tx, rx) = delayed_pipe(CAPACITY, DELAY);
        let pipe = PipeId {
            side: self.side,
            serial: self.next.fetch_add(1, Ordering::Relaxed),
        };
        self.announce
            .send(TracedReader {
                inner: rx,
                pipe,
                trace: self.trace.clone(),
            })
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "peer link is gone"))?;
        Ok((
            TracedWriter {
                inner: tx,
                pipe,
                trace: self.trace.clone(),
            },
            Done::discard(),
        ))
    }
}

/// The traced acceptor: yields the peer's announced traced streams.
struct TracedAcceptor {
    streams: mpsc::Receiver<TracedReader>,
}

impl Acceptor for TracedAcceptor {
    type Rx = TracedReader;

    async fn accept(&mut self) -> io::Result<(Self::Rx, Done<Self::Rx>)> {
        self.streams
            .recv()
            .await
            .map(|rx| (rx, Done::discard()))
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "peer link is gone"))
    }
}

type TracedLink = Link<TracedReader, TracedWriter, TracedConnector, TracedAcceptor>;

/// A traced counterpart of `delayed_pair`, sharing one trace log.
fn traced_pair(trace: &Trace) -> (TracedLink, TracedLink) {
    let control = |side: char| PipeId {
        side,
        serial: CONTROL,
    };
    let (a_control_write, b_control_read) = delayed_pipe(CAPACITY, DELAY);
    let (b_control_write, a_control_read) = delayed_pipe(CAPACITY, DELAY);
    let (a_announce, b_streams) = mpsc::channel(STREAM_COUNT);
    let (b_announce, a_streams) = mpsc::channel(STREAM_COUNT);
    let wrap_w = |inner, pipe| TracedWriter {
        inner,
        pipe,
        trace: trace.clone(),
    };
    let wrap_r = |inner, pipe| TracedReader {
        inner,
        pipe,
        trace: trace.clone(),
    };
    (
        Link::new(
            wrap_r(a_control_read, control('B')),
            wrap_w(a_control_write, control('A')),
            TracedConnector {
                announce: a_announce,
                side: 'A',
                next: Arc::new(AtomicU8::new(0)),
                trace: trace.clone(),
            },
            TracedAcceptor { streams: a_streams },
        ),
        Link::new(
            wrap_r(b_control_read, control('A')),
            wrap_w(b_control_write, control('B')),
            TracedConnector {
                announce: b_announce,
                side: 'B',
                next: Arc::new(AtomicU8::new(0)),
                trace: trace.clone(),
            },
            TracedAcceptor { streams: b_streams },
        ),
    )
}

/// Gossip one pair over a traced delayed link and return the trace.
fn traced_session<T>(a: Rumors<T>, b: Rumors<T>) -> Trace
where
    T: Serialize + DeserializeOwned + Send + Sync + 'static,
{
    let trace = Trace::default();
    let (mut a_link, mut b_link) = traced_pair(&trace);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .expect("build paused current-thread runtime");
    runtime.block_on(async {
        let (a_result, b_result) = tokio::join!(a.gossip(&mut a_link), b.gossip(&mut b_link));
        a_result.expect("peer A gossip");
        b_result.expect("peer B gossip");
    });
    trace
}

/// Render the phase a data stream index carries under each speaker role.
fn describe(index: u8) -> String {
    let as_initiator = match index {
        0 => "I:opening-supplies(h31)".to_string(),
        16 => "I:leaf-parent-replies(h0)".to_string(),
        i => format!("I:interior(h{})", 32 - 2 * usize::from(i)),
    };
    let as_responder = match index {
        0 => "R:opening-reply(h31)".to_string(),
        16 => "R:terminal-leaf-replies(h0)".to_string(),
        i => format!("R:interior(h{})", 31 - 2 * usize::from(i)),
    };
    format!("{as_initiator} | {as_responder}")
}

/// Print the trace as per-hop buckets of per-stream traffic.
fn report(name: &str, trace: &Trace) {
    let inner = trace.0.lock().unwrap();
    println!("\n=== {name} ===");

    // Stream label legend, per pipe.
    println!("-- streams --");
    for (pipe, prefix) in &inner.prefixes {
        if pipe.serial == CONTROL {
            println!("  {}#ctrl: control", pipe.side);
        } else if let [epoch, index] = prefix.as_slice() {
            println!(
                "  {}#{:02}: epoch={epoch} idx={index}  {}",
                pipe.side,
                pipe.serial,
                describe(*index),
            );
        } else {
            println!(
                "  {}#{:02}: label incomplete {prefix:?}",
                pipe.side, pipe.serial
            );
        }
    }

    // Per-bucket totals: (pipe, write?) -> (events, bytes).
    #[allow(clippy::type_complexity)]
    let mut buckets: BTreeMap<u64, BTreeMap<(PipeId, bool), (usize, usize)>> = BTreeMap::new();
    for event in &inner.events {
        let bucket = event.at.as_millis() as u64;
        let entry = buckets
            .entry(bucket)
            .or_default()
            .entry((event.pipe, event.write))
            .or_default();
        entry.0 += 1;
        entry.1 += event.len;
    }

    println!(
        "-- hops (bucket = virtual ms; delay = {} ms) --",
        DELAY.as_millis()
    );
    for (bucket, traffic) in &buckets {
        let hop = *bucket as f64 / DELAY.as_millis() as f64;
        println!("  t={bucket:>4}ms (hop {hop:>4.1}):");
        for ((pipe, write), (events, bytes)) in traffic {
            let kind = if *write { "W" } else { "R" };
            let serial = if pipe.serial == CONTROL {
                "ctrl".to_string()
            } else {
                let label = inner
                    .prefixes
                    .get(pipe)
                    .and_then(|p| p.get(1).copied())
                    .map(|i| format!("idx{i:02}"))
                    .unwrap_or_else(|| "?".to_string());
                format!("#{:02}/{label}", pipe.serial)
            };
            println!(
                "      {} {}{:<9} {:>8} B in {:>4} events",
                kind, pipe.side, serial, bytes, events
            );
        }
    }
    let last = buckets.keys().last().copied().unwrap_or(0);
    println!(
        "-- total: last event at {last} ms = {:.1} hops --",
        last as f64 / DELAY.as_millis() as f64
    );
}

/// Two production-window V2 peers with a shared prefix and divergence on
/// both sides, mirroring the pipelining test's shape at reduced scale.
fn diverged_insertions() -> (Rumors<u64>, Rumors<u64>) {
    const COMMON: usize = 2_048;
    const DIVERGENT_PER_SIDE: usize = 512;

    let left = Peer::seed()
        .sync_memory_budget(DEFAULT_SYNC_MEMORY_BUDGET)
        .into_rumors();
    let mut rng = SmallRng::seed_from_u64(0x9e37_79b9_7f4a_7c15);
    send_random(&left, COMMON, &mut rng);
    let right = bootstrap_fork(&left);

    send_random(&left, DIVERGENT_PER_SIDE, &mut rng);
    send_random(&right, DIVERGENT_PER_SIDE, &mut rng);
    (left, right)
}

/// Two production-window V2 peers diverged purely by disjoint redactions.
fn diverged_redactions() -> (Rumors<u64>, Rumors<u64>) {
    const COMMON: usize = 2_048;
    const REDACT_PER_SIDE: usize = 256;

    let left = Peer::seed()
        .sync_memory_budget(DEFAULT_SYNC_MEMORY_BUDGET)
        .into_rumors();
    let mut rng = SmallRng::seed_from_u64(0x2545_f491_4f6c_dd1d);
    send_random(&left, COMMON, &mut rng);
    let right = bootstrap_fork(&left);

    let versions: Vec<Version> = left.snapshot().iter().map(|(v, _)| v.clone()).collect();
    let mut shuffled = versions;
    shuffled.shuffle(&mut SmallRng::seed_from_u64(0x84f6_7932_1265_9eec));
    redact_all(&left, &shuffled[..REDACT_PER_SIDE]);
    redact_all(&right, &shuffled[REDACT_PER_SIDE..2 * REDACT_PER_SIDE]);
    (left, right)
}

fn send_random(rumors: &Rumors<u64>, count: usize, rng: &mut SmallRng) {
    let mut batch = rumors.batch();
    for _ in 0..count {
        batch.send(rng.next_u64());
    }
}

fn redact_all(rumors: &Rumors<u64>, versions: &[Version]) {
    let mut batch = rumors.batch();
    for version in versions {
        batch.redact(version);
    }
}

fn bootstrap_fork(parent: &Rumors<u64>) -> Rumors<u64> {
    pollster::block_on(async {
        let (mut parent_link, mut newcomer_link) = rumors::link::memory_with_capacity(CAPACITY);
        let (served, newcomer) = tokio::join!(
            parent.gossip(&mut parent_link),
            Peer::<u64>::bootstrap()
                .protocol(Protocol::V2)
                .join(&mut newcomer_link),
        );
        served.expect("serve bootstrap");
        newcomer
            .expect("bootstrap newcomer")
            .expect("provider is established")
            .sync_memory_budget(DEFAULT_SYNC_MEMORY_BUDGET)
            .into_rumors()
    })
}

/// Traces the serialized hop structure of an insertion-shaped divergent
/// session and pins its exact hop count: with the opening question riding
/// the greeting, this shape completes in 7 serialized hops.
#[test]
fn trace_insertion_session() {
    let (left, right) = diverged_insertions();
    let trace = traced_session(left.clone(), right.clone());
    report("insertions: 2048 common + 512/side", &trace);
    assert_eq!(left.snapshot().len(), right.snapshot().len());
    assert_eq!(trace.hops(), 7, "insertion-shaped session hop count");
}

/// Traces the serialized hop structure of a redaction-shaped divergent
/// session and pins its exact hop count: 7 hops, like the insertion shape
/// (the redaction ladder bottoms out at the same depth at this scale).
#[test]
fn trace_redaction_session() {
    let (left, right) = diverged_redactions();
    let trace = traced_session(left.clone(), right.clone());
    report("redactions: 2048 common, 256 redacted/side", &trace);
    assert_eq!(left.snapshot().len(), right.snapshot().len());
    assert_eq!(trace.hops(), 7, "redaction-shaped session hop count");
}

/// Two peers with disjoint exclusive content and no dispute, the smaller
/// side holding a two-leaf subtree under one root radix.
///
/// The staging requires the initiator's two leaf paths to share their
/// first byte while its ballast counterpart's three land under other root
/// radices, so no root child is populated on both sides and the session is
/// pure transfer in both directions. A leaf's path is the BLAKE3 hash of
/// its version's canonical bytes, so the shape is a property of the minted
/// version sequence; the self-checks below verify it.
fn transfer_pair() -> (Rumors<u64>, Rumors<u64>) {
    // Stage by pool search: paths are version-derived, so the shape is a
    // deterministic function of the seeded universe and send order — mint
    // a pool, pick the versions whose paths land the shape, redact the
    // rest. The left peer keeps exactly two leaves sharing a root radix;
    // the right keeps three ballast leaves outside it and advertises the
    // larger set.
    let path_radix = |version: &Version| blake3::hash(version.as_bytes()).as_bytes()[0];
    let keep_only = |rumors: &Rumors<u64>, keep: &[u64]| {
        let losers: Vec<Version> = rumors
            .snapshot()
            .iter()
            .filter(|(_, m)| !keep.contains(m))
            .map(|(v, _)| v.clone())
            .collect();
        let mut batch = rumors.batch();
        for version in &losers {
            batch.redact(version);
        }
    };

    let left = Peer::seed_rng(&mut SmallRng::seed_from_u64(0))
        .sync_memory_budget(DEFAULT_SYNC_MEMORY_BUDGET)
        .into_rumors();
    let right = bootstrap_fork(&left);
    {
        let mut batch = left.batch();
        for value in 0..64u64 {
            batch.send(value);
        }
    }
    let mut pool: Vec<(u64, u8)> = left
        .snapshot()
        .iter()
        .map(|(v, m)| (**m, path_radix(v)))
        .collect();
    pool.sort_unstable();
    let (first, second) = pool
        .iter()
        .find_map(|&(value, radix)| {
            pool.iter()
                .find(|&&(other, r)| other > value && r == radix)
                .map(|&(other, _)| (value, other))
        })
        .expect("some pool pair shares a root radix");
    keep_only(&left, &[first, second]);
    let radix = pool
        .iter()
        .find_map(|&(value, radix)| (value == first).then_some(radix))
        .expect("the kept pair is in the pool");
    {
        let mut batch = right.batch();
        for value in 10_000..10_016u64 {
            batch.send(value);
        }
    }
    let ballast: Vec<u64> = right
        .snapshot()
        .iter()
        .filter(|(v, _)| path_radix(v) != radix)
        .take(3)
        .map(|(_, m)| **m)
        .collect();
    assert_eq!(ballast.len(), 3, "the ballast pool cannot fill its quota");
    keep_only(&right, &ballast);

    // Fixture self-checks: mirror the required shape so drift in hashing
    // or version assignment fails here, not in the hop arithmetic.
    let radices: Vec<u8> = left.snapshot().iter().map(|(v, _)| path_radix(v)).collect();
    assert_eq!(radices.len(), 2, "the left peer holds exactly the pair");
    assert_eq!(radices.first(), radices.last(), "one exclusive subtree");
    assert!(
        right
            .snapshot()
            .iter()
            .all(|(v, _)| path_radix(v) != radices[0]),
        "no root child is populated on both sides"
    );
    assert!(
        left.snapshot().len() < right.snapshot().len(),
        "the subtree holder advertises the smaller set and initiates"
    );
    (left, right)
}

/// Traces a pure transfer whose initiator holds exclusive bulk, pinning the
/// hop ladder of the supply-only opening.
///
/// Derived before running: preamble (hop 1), greetings (hop 2), then the
/// initiator's opening supplies depart *with* the responder's opening reply
/// — both writes at hop 2, landing at hop 3 — the initiator's bare empty
/// reply to the responder's one whole-subtree request lands at hop 4, and
/// the session epilogue closes at hop 5. The pinned departure is the win:
/// the initiator's exclusive content is written at hop 2, in the greeting
/// response window, rather than waiting a further hop for the
/// responder's request to arrive.
#[test]
fn trace_bulk_initiator_session() {
    let (left, right) = transfer_pair();
    let trace = traced_session(left.clone(), right.clone());
    report("pure transfer: 2 vs 3 exclusive messages", &trace);
    assert_eq!(left.snapshot().len(), right.snapshot().len());
    assert_eq!(trace.hops(), 5, "bulk-initiator session hop count");
    assert_eq!(
        trace.first_write_hop('A', 0),
        2,
        "the opening supplies depart in the greeting response window"
    );
}

/// Traces the empty session (identical peers): the protocol floor every
/// divergent trace is read against.
///
/// Pinned at 3 hops: the root-fan listing
/// rides the greeting hop itself, and converged peers ask no question
/// either way, so no further hop exists to pay.
#[test]
fn trace_empty_session() {
    let left = Peer::seed()
        .sync_memory_budget(DEFAULT_SYNC_MEMORY_BUDGET)
        .into_rumors();
    let mut rng = SmallRng::seed_from_u64(0x1234_5678_9abc_def0);
    send_random(&left, 64, &mut rng);
    let right = bootstrap_fork(&left);
    let trace = traced_session(left.clone(), right.clone());
    report("empty divergence: identical 64-message peers", &trace);
    assert_eq!(left.snapshot().len(), right.snapshot().len());
    assert_eq!(trace.hops(), 3, "converged-session hop count");
}
