//! Frame-level hop tracing over the delayed pipe: the hop-count instrument.
//!
//! Wraps both halves of a delayed connection pair in a byte-level tracer,
//! attaches an observation hook to both peers, and runs representative
//! divergent sessions at a fixed one-way delay under the paused clock.
//! Because virtual time only advances while every task is blocked on the
//! wire, all compute is instantaneous in virtual terms and every traced
//! event lands at an exact multiple of the delay: bucket `k` holds
//! precisely the frames whose earliest enabling arrival was `k` hops into
//! the session. Printing the per-bucket, per-stream traffic therefore *is*
//! the serialized critical path — no inference required.
//!
//! Stream identity comes from the observation hook: each sent frame is
//! reported with the logical stream index it names, and the index maps to
//! a protocol phase per the speaker's role (see `describe`). Which side is
//! initiator is evident from the trace (the opening question rides the
//! greeting, so the earliest frame is the *responder's* opening reply on
//! its index 0; the initiator's index 0 stays unused).
//!
//! Run with:
//!
//!     cargo nextest run -E 'binary(hop_trace)' --no-capture

use std::collections::BTreeMap;
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use rand::seq::SliceRandom;
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaChaRng;
use rumors::observe::{
    Direction, Observer, SessionInfo, SessionObserver, StreamId, StreamInfo, StreamObserver,
};
use rumors::{DEFAULT_SYNC_MEMORY_BUDGET, Peer, Rumors, Version};
use rumors_testkit::bench::latency;
use sha3::Digest;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::time::Instant;

use latency::{DelayedReader, DelayedWriter, delayed_pair};

use serde::Serialize;
use serde::de::DeserializeOwned;
/// One-way delay; whole milliseconds per the timer wheel's grain.
const DELAY: Duration = Duration::from_millis(10);

/// Per-direction in-flight byte window, far above any transfer here.
const CAPACITY: usize = 8 * 1024 * 1024;

/// One completed read or write on a traced half.
struct Event {
    at: Duration,
    /// The side whose half performed the I/O.
    side: char,
    write: bool,
    len: usize,
}

/// One frame a side sent, as its observation hook reported it.
struct Sent {
    at: Duration,
    side: char,
    /// The logical stream the frame names.
    index: u8,
    len: usize,
}

#[derive(Default)]
struct TraceInner {
    start: Option<Instant>,
    events: Vec<Event>,
    frames: Vec<Sent>,
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

    fn record(&self, side: char, write: bool, len: usize) {
        let mut inner = self.0.lock().unwrap();
        let at = Self::at(&mut inner);
        inner.events.push(Event {
            at,
            side,
            write,
            len,
        });
    }

    fn record_frame(&self, side: char, index: u8, len: usize) {
        let mut inner = self.0.lock().unwrap();
        let at = Self::at(&mut inner);
        inner.frames.push(Sent {
            at,
            side,
            index,
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

    /// Whether `side` sent any frame on logical stream `index`.
    fn sent_on(&self, side: char, index: u8) -> bool {
        self.0
            .lock()
            .unwrap()
            .frames
            .iter()
            .any(|frame| frame.side == side && frame.index == index)
    }

    /// The delay multiple of `side`'s first frame on logical stream `index`.
    fn first_frame_hop(&self, side: char, index: u8) -> u64 {
        let inner = self.0.lock().unwrap();
        let first = inner
            .frames
            .iter()
            .filter(|frame| frame.side == side && frame.index == index)
            .map(|frame| frame.at)
            .min()
            .unwrap_or_else(|| panic!("side {side} sent no frame on stream {index}"));
        first.as_millis() as u64 / DELAY.as_millis() as u64
    }
}

/// An observation hook recording each frame its side sends into the trace.
struct FrameHook {
    side: char,
    trace: Trace,
}

impl Observer for FrameHook {
    fn session(&self, _: &SessionInfo) -> Option<Box<dyn SessionObserver>> {
        Some(Box::new(FrameHook {
            side: self.side,
            trace: self.trace.clone(),
        }))
    }
}

impl SessionObserver for FrameHook {
    fn stream(&self, stream: &StreamInfo) -> Option<Box<dyn StreamObserver>> {
        match stream.id {
            StreamId::Data { index, .. } if stream.direction == Direction::Sent => {
                Some(Box::new(FrameStream {
                    side: self.side,
                    index,
                    trace: self.trace.clone(),
                }))
            }
            _ => None,
        }
    }
}

/// One logical stream's sent frames, recorded with the virtual time each
/// was observed.
struct FrameStream {
    side: char,
    index: u8,
    trace: Trace,
}

impl StreamObserver for FrameStream {
    fn message(&mut self, bytes: &[u8]) {
        self.trace.record_frame(self.side, self.index, bytes.len());
    }
}

/// A write half that logs every completed write against its side.
struct TracedWriter {
    inner: DelayedWriter,
    side: char,
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
                self.trace.record(self.side, true, n);
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

/// A read half that logs every nonempty delivery against its side.
struct TracedReader {
    inner: DelayedReader,
    side: char,
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
                    this.trace.record(this.side, false, delivered);
                }
                Poll::Ready(Ok(()))
            }
            other => other,
        }
    }
}

/// Attach `side`'s frame hook to a sole handle.
async fn hooked<T>(rumors: Rumors<T>, side: char, trace: &Trace) -> Rumors<T>
where
    T: Send + Sync + 'static,
{
    rumors
        .try_into_peer()
        .await
        .expect("the traced session holds each peer's sole handle")
        .observe(Arc::new(FrameHook {
            side,
            trace: trace.clone(),
        }))
        .into_rumors()
}

/// Gossip one pair over a traced delayed pipe and return the trace and the
/// two handles.
///
/// Each handle must be its peer's sole one, so the hook can attach.
fn traced_session<T>(a: Rumors<T>, b: Rumors<T>) -> (Trace, Rumors<T>, Rumors<T>)
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    let trace = Trace::default();
    let (a_end, b_end) = delayed_pair(CAPACITY, DELAY);
    let traced = |end: latency::DelayedEnd, side| {
        (
            TracedReader {
                inner: end.read,
                side,
                trace: trace.clone(),
            },
            TracedWriter {
                inner: end.write,
                side,
                trace: trace.clone(),
            },
        )
    };
    let (a_read, a_write) = traced(a_end, 'A');
    let (b_read, b_write) = traced(b_end, 'B');
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .expect("build paused current-thread runtime");
    let (a, b) = runtime.block_on(async {
        let a = hooked(a, 'A', &trace).await;
        let b = hooked(b, 'B', &trace).await;
        let (a_result, b_result) = tokio::join!(
            a.gossip_once(a_read, a_write),
            b.gossip_once(b_read, b_write)
        );
        a_result.expect("peer A gossip");
        b_result.expect("peer B gossip");
        (a, b)
    });
    (trace, a, b)
}

/// Render the phase a data stream index carries under each speaker role.
fn describe(index: u8) -> String {
    let as_initiator = match index {
        0 => "I:unused".to_string(),
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

/// Print the trace as per-hop buckets of wire traffic and per-stream frames.
fn report(name: &str, trace: &Trace) {
    let inner = trace.0.lock().unwrap();
    println!("\n=== {name} ===");

    // Stream legend: every logical stream a side sent frames on.
    println!("-- streams --");
    let mut streams: BTreeMap<(char, u8), ()> = BTreeMap::new();
    for frame in &inner.frames {
        streams.insert((frame.side, frame.index), ());
    }
    for (side, index) in streams.keys() {
        println!("  {side}/idx{index:02}: {}", describe(*index));
    }

    // Per-bucket totals: wire I/O per side, and frames per stream.
    #[allow(clippy::type_complexity)]
    let mut wire: BTreeMap<u64, BTreeMap<(char, bool), (usize, usize)>> = BTreeMap::new();
    for event in &inner.events {
        let entry = wire
            .entry(event.at.as_millis() as u64)
            .or_default()
            .entry((event.side, event.write))
            .or_default();
        entry.0 += 1;
        entry.1 += event.len;
    }
    #[allow(clippy::type_complexity)]
    let mut sent: BTreeMap<u64, BTreeMap<(char, u8), (usize, usize)>> = BTreeMap::new();
    for frame in &inner.frames {
        let entry = sent
            .entry(frame.at.as_millis() as u64)
            .or_default()
            .entry((frame.side, frame.index))
            .or_default();
        entry.0 += 1;
        entry.1 += frame.len;
    }

    println!(
        "-- hops (bucket = virtual ms; delay = {} ms) --",
        DELAY.as_millis()
    );
    for (bucket, traffic) in &wire {
        let hop = *bucket as f64 / DELAY.as_millis() as f64;
        println!("  t={bucket:>4}ms (hop {hop:>4.1}):");
        for ((side, write), (events, bytes)) in traffic {
            let kind = if *write { "W" } else { "R" };
            println!("      {kind} {side}  {bytes:>8} B in {events:>4} events");
        }
        for ((side, index), (frames, bytes)) in sent.get(bucket).into_iter().flatten() {
            println!("        {side}/idx{index:02} {bytes:>8} B in {frames:>4} frames");
        }
    }
    let last = wire.keys().last().copied().unwrap_or(0);
    println!(
        "-- total: last event at {last} ms = {:.1} hops --",
        last as f64 / DELAY.as_millis() as f64
    );
}

/// Two default-budget V2 peers with a shared prefix and divergence on
/// both sides, mirroring the pipelining test's shape at reduced scale.
fn diverged_insertions() -> (Rumors<u64>, Rumors<u64>) {
    const COMMON: usize = 2_048;
    const DIVERGENT_PER_SIDE: usize = 512;

    let left = Peer::seed()
        .sync_memory_budget(DEFAULT_SYNC_MEMORY_BUDGET)
        .into_rumors();
    // Fixed, so the pinned hop count is measured on the same corpus every run.
    let mut rng = ChaChaRng::seed_from_u64(0x9e37_79b9_7f4a_7c15);
    send_random(&left, COMMON, &mut rng);
    let right = bootstrap_fork(&left);

    send_random(&left, DIVERGENT_PER_SIDE, &mut rng);
    send_random(&right, DIVERGENT_PER_SIDE, &mut rng);
    (left, right)
}

/// Two default-budget V2 peers diverged purely by disjoint redactions.
fn diverged_redactions() -> (Rumors<u64>, Rumors<u64>) {
    const COMMON: usize = 2_048;
    const REDACT_PER_SIDE: usize = 256;

    let left = Peer::seed()
        .sync_memory_budget(DEFAULT_SYNC_MEMORY_BUDGET)
        .into_rumors();
    // Fixed, as is the shuffle below, so the pinned hop count is measured on
    // the same corpus and the same redactions every run.
    let mut rng = ChaChaRng::seed_from_u64(0x2545_f491_4f6c_dd1d);
    send_random(&left, COMMON, &mut rng);
    let right = bootstrap_fork(&left);

    let versions: Vec<Version> = left.snapshot().versions().cloned().collect();
    let mut shuffled = versions;
    shuffled.shuffle(&mut ChaChaRng::seed_from_u64(0x84f6_7932_1265_9eec));
    left.redact_all(&shuffled[..REDACT_PER_SIDE]);
    right.redact_all(&shuffled[REDACT_PER_SIDE..2 * REDACT_PER_SIDE]);
    (left, right)
}

fn send_random(rumors: &Rumors<u64>, count: usize, rng: &mut ChaChaRng) {
    rumors.send_all((0..count).map(|_| rng.next_u64())).unwrap();
}

fn bootstrap_fork(parent: &Rumors<u64>) -> Rumors<u64> {
    pollster::block_on(async {
        let (parent_end, newcomer_end) = rumors::testing::memory_connection(CAPACITY);
        let (served, newcomer) = tokio::join!(
            parent.gossip_once(parent_end.read, parent_end.write),
            Peer::<u64>::bootstrap().join(newcomer_end.read, newcomer_end.write),
        );
        served.expect("serve bootstrap");
        (match newcomer {
            rumors::Joined::Joined { peer, .. } => peer,
            _ => panic!("provider is established"),
        })
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
    let (trace, left, right) = traced_session(left, right);
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
    let (trace, left, right) = traced_session(left, right);
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
/// pure transfer in both directions. A leaf's path is the SHA3-256 hash of
/// its version's canonical bytes, so the shape is a property of the created
/// version sequence; the self-checks below verify it.
fn transfer_pair() -> (Rumors<u64>, Rumors<u64>) {
    // Stage by pool search: paths are version-derived, and a universe's
    // identifier never enters a version, so the shape is a deterministic
    // function of the send order — send a pool, pick the versions whose
    // paths land the shape, redact the rest. The left peer keeps exactly two leaves sharing a root radix;
    // the right keeps three ballast leaves outside it and advertises the
    // larger set.
    let path_radix = |version: &Version| sha3::Sha3_256::digest(version.as_bytes())[0];
    let keep_only = |rumors: &Rumors<u64>, keep: &[u64]| {
        let losers: Vec<Version> = rumors
            .snapshot()
            .iter()
            .filter(|(_, m)| !keep.contains(m))
            .map(|(v, _)| v.clone())
            .collect();
        rumors.redact_all(&losers);
    };

    let left = Peer::seed()
        .sync_memory_budget(DEFAULT_SYNC_MEMORY_BUDGET)
        .into_rumors();
    let right = bootstrap_fork(&left);
    {
        left.send_all(0..64u64).unwrap();
    }
    let mut pool: Vec<(u64, u8)> = left
        .snapshot()
        .iter()
        .map(|(v, m)| (*m, path_radix(v)))
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
        right.send_all(10_000..10_016u64).unwrap();
    }
    let ballast: Vec<u64> = right
        .snapshot()
        .iter()
        .filter(|(v, _)| path_radix(v) != radix)
        .take(3)
        .map(|(_, m)| *m)
        .collect();
    assert_eq!(ballast.len(), 3, "the ballast pool cannot fill its quota");
    keep_only(&right, &ballast);

    // Fixture self-checks: mirror the required shape so drift in hashing
    // or version assignment fails here, not in the hop arithmetic.
    let radices: Vec<u8> = left.snapshot().versions().map(path_radix).collect();
    assert_eq!(radices.len(), 2, "the left peer holds exactly the pair");
    assert_eq!(radices.first(), radices.last(), "one exclusive subtree");
    assert!(
        right
            .snapshot()
            .versions()
            .all(|v| path_radix(v) != radices[0]),
        "no root child is populated on both sides"
    );
    assert!(
        left.snapshot().len() < right.snapshot().len(),
        "the subtree holder advertises the smaller set and initiates"
    );
    (left, right)
}

/// Traces a pure transfer whose initiator holds exclusive bulk, pinning the
/// hop ladder of its supplies.
///
/// Derived before running: preamble (hop 1), greetings (hop 2), then the
/// responder's opening reply departs at hop 2, requesting the initiator's
/// exclusive root child; the initiator's supplies answer it, departing at
/// hop 3 on its level-2 stream and landing at hop 4; the session epilogue
/// closes at hop 5. The initiator never sends a frame on its unused index 0.
#[test]
fn trace_bulk_initiator_session() {
    let (left, right) = transfer_pair();
    let (trace, left, right) = traced_session(left, right);
    report("pure transfer: 2 vs 3 exclusive messages", &trace);
    assert_eq!(left.snapshot().len(), right.snapshot().len());
    assert_eq!(trace.hops(), 5, "bulk-initiator session hop count");
    assert_eq!(
        trace.first_frame_hop('A', 1),
        3,
        "the supplies answer the responder's request as it arrives"
    );
    assert!(
        !trace.sent_on('A', 0),
        "the initiator never sends on its unused index 0"
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
    // Fixed, so the pinned hop count is measured on the same corpus every run.
    let mut rng = ChaChaRng::seed_from_u64(0x1234_5678_9abc_def0);
    send_random(&left, 64, &mut rng);
    let right = bootstrap_fork(&left);
    let (trace, left, right) = traced_session(left, right);
    report("empty divergence: identical 64-message peers", &trace);
    assert_eq!(left.snapshot().len(), right.snapshot().len());
    assert_eq!(trace.hops(), 3, "converged-session hop count");
}
