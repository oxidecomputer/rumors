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
//! The hop count of every session follows one rule, stated and argued in
//! [`rule`], which predicts it from the two replicas' leaf paths alone. The
//! property `hops_follow_the_rule` checks the traced count against that
//! prediction over generated replica pairs and every network secret. Hashed
//! leaves share only a few leading bytes, so those pairs part shallowly;
//! `deep_parting_depths_follow_the_rule` places leaves to reach every
//! deeper parting depth, to the leaves themselves. The point traces print
//! the critical path of representative sessions, each stating its expected
//! count through the rule.
//!
//! Run with:
//!
//!     cargo nextest run -E 'binary(hop_trace)' --no-capture

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::iter;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::Bytes;
use proptest::prelude::*;
use proptest::sample::Index;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use rumors::observe::{
    Direction, Observer, SessionInfo, SessionObserver, StreamId, StreamInfo, StreamObserver,
};
use rumors::{Peer, Rumors, SessionStats, Version, default};
use rumors_testkit::bench::latency;
use rumors_testkit::common::shape::path_radix;
use rumors_testkit::divided_proptest;
use tokio::io::{AsyncRead, AsyncWrite, BufReader, ReadBuf};
use tokio::time::Instant;

use latency::{DelayedReader, DelayedWriter, delayed_pair};

use rumors_testkit::common::seed::{NetworkSeed, network_seed};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// State the hop rule and predict hop counts from the replicas' leaf paths.
#[path = "hop_trace/rule.rs"]
mod rule;

use rule::{Prediction, Replica, Side};

/// The network of every point trace, fixed so that each trace prints the
/// same critical path on every run.
const NETWORK: NetworkSeed = NetworkSeed::new(0);

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

/// A traced session's record: its trace, both handles, and what each side's
/// session measured about itself.
struct Traced<T: Send + Sync + 'static> {
    /// The frame and wire-event log of the session.
    trace: Trace,
    /// Side `'A'`'s handle.
    a: Rumors<T>,
    /// Side `'B'`'s handle.
    b: Rumors<T>,
    /// Side `'A'`'s session statistics.
    a_stats: SessionStats,
    /// Side `'B'`'s session statistics.
    b_stats: SessionStats,
}

/// Gossip one pair over a traced delayed pipe and return the trace, the two
/// handles, and each side's session statistics.
///
/// Each handle must be its peer's sole one, so the hook can attach.
fn traced_session<T>(a: Rumors<T>, b: Rumors<T>) -> Traced<T>
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    let trace = Trace::default();
    let (a_end, b_end) = delayed_pair(CAPACITY, DELAY);
    let traced = |end: latency::DelayedEnd, side| {
        (
            // The tracer models the transport, beneath the session's read
            // buffer; the connection end is fresh, so its buffer is empty.
            BufReader::new(TracedReader {
                inner: end.read.into_inner(),
                side,
                trace: trace.clone(),
            }),
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
    runtime.block_on(async {
        let a = hooked(a, 'A', &trace).await;
        let b = hooked(b, 'B', &trace).await;
        let (a_result, b_result) = tokio::join!(
            a.gossip_once(a_read, a_write),
            b.gossip_once(b_read, b_write)
        );
        let (a_report, ..) = a_result.expect("peer A gossip");
        let (b_report, ..) = b_result.expect("peer B gossip");
        Traced {
            trace,
            a,
            b,
            a_stats: a_report.stats,
            b_stats: b_report.stats,
        }
    })
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

/// Two peers at the default budget with a shared history and insertions on
/// both sides, mirroring the pipelining test's shape at reduced scale.
fn diverged_insertions() -> (Rumors<u64>, Rumors<u64>) {
    const COMMON: usize = 2_048;
    const DIVERGENT_PER_SIDE: usize = 512;

    let left = NETWORK
        .peer()
        .sync_memory_budget(default::SYNC_MEMORY_BUDGET)
        .into_rumors();
    // Fixed, so the trace prints the same session every run.
    let mut rng = ChaChaRng::seed_from_u64(0x9e37_79b9_7f4a_7c15);
    send_random(&left, COMMON, &mut rng);
    let right = bootstrap_fork(&left, default::TARGET_MESSAGE_SIZE);

    send_random(&left, DIVERGENT_PER_SIDE, &mut rng);
    send_random(&right, DIVERGENT_PER_SIDE, &mut rng);
    (left, right)
}

/// Two peers at the default budget diverged purely by disjoint redactions.
fn diverged_redactions() -> (Rumors<u64>, Rumors<u64>) {
    const COMMON: usize = 2_048;
    const REDACT_PER_SIDE: usize = 256;

    let left = NETWORK
        .peer()
        .sync_memory_budget(default::SYNC_MEMORY_BUDGET)
        .into_rumors();
    // Fixed, as is the shuffle below, so the trace prints the same session,
    // with the same redactions, every run.
    let mut rng = ChaChaRng::seed_from_u64(0x2545_f491_4f6c_dd1d);
    send_random(&left, COMMON, &mut rng);
    let right = bootstrap_fork(&left, default::TARGET_MESSAGE_SIZE);

    let versions: Vec<Version> = left.snapshot().versions().cloned().collect();
    let mut shuffled = versions;
    shuffled.shuffle(&mut ChaChaRng::seed_from_u64(0x84f6_7932_1265_9eec));
    left.redact_all(&shuffled[..REDACT_PER_SIDE]);
    right.redact_all(&shuffled[REDACT_PER_SIDE..2 * REDACT_PER_SIDE]);
    (left, right)
}

/// Send `count` messages with payloads drawn from `rng`, as one batch.
fn send_random(rumors: &Rumors<u64>, count: usize, rng: &mut ChaChaRng) {
    rumors.send_all((0..count).map(|_| rng.next_u64())).unwrap();
}

/// Join `parent`'s network with a new peer holding its messages, at the
/// default memory budget and with the given target message size.
fn bootstrap_fork<T>(parent: &Rumors<T>, target_message_size: usize) -> Rumors<T>
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    pollster::block_on(async {
        let (parent_end, newcomer_end) = rumors::testing::memory_connection(CAPACITY);
        let (served, newcomer) = tokio::join!(
            parent.gossip_once(parent_end.read, parent_end.write),
            Peer::<T>::bootstrap().join(newcomer_end.read, newcomer_end.write),
        );
        served.expect("serve bootstrap");
        (match newcomer {
            rumors::Joined::Joined { peer, .. } => peer,
            _ => panic!("provider is established"),
        })
        .sync_memory_budget(default::SYNC_MEMORY_BUDGET)
        .target_message_size(target_message_size)
        .into_rumors()
    })
}

/// The largest share of the in-flight window one side's data frames may
/// total, as a fraction's denominator: the rest leaves room for the
/// preamble, the greeting, and the completion marker.
///
/// A side cannot hold more in flight than it sends in the whole session, so
/// a session within this share never fills the window.
const WINDOW_SHARE_DENOMINATOR: u64 = 2;

/// Check that a traced session met the rule's premises: no admission wait on
/// either side, and an in-flight window that never filled ([`rule`]'s
/// setting).
fn assert_in_scope<T: Send + Sync + 'static>(traced: &Traced<T>) {
    let window_share = CAPACITY as u64 / WINDOW_SHARE_DENOMINATOR;
    for (side, stats) in [('A', &traced.a_stats), ('B', &traced.b_stats)] {
        assert_eq!(
            stats.budget_waits, 0,
            "side {side}'s budget held work back, outside the rule's setting",
        );
        assert!(
            stats.bytes_sent <= window_share,
            "side {side} sent {} bytes, more than the window share {window_share} the \
             rule's setting allows",
            stats.bytes_sent,
        );
    }
}

/// Check that a traced session converged: both replicas hold the same set.
fn assert_converged<T: Send + Sync + 'static>(traced: &Traced<T>) {
    assert_eq!(
        traced.a.snapshot().hash(),
        traced.b.snapshot().hash(),
        "the session converges both replicas on one set",
    );
}

/// Trace a session between `a` and `b`, print its critical path under
/// `name` with the rule's prediction, and check that it converged within the
/// rule's setting.
fn trace_point<T>(name: &str, a: Rumors<T>, b: Rumors<T>) -> (Traced<T>, Prediction)
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    let predicted = rule::predict(&Replica::of(&a), &Replica::of(&b));
    let traced = traced_session(a, b);
    report(name, &traced.trace);
    println!("-- the rule predicts {predicted:?} --");
    assert_in_scope(&traced);
    assert_converged(&traced);
    (traced, predicted)
}

/// Traces the serialized hop structure of an insertion-shaped divergent
/// session, and checks that its hop count is the one the rule predicts from
/// the two replicas' leaf paths.
#[test]
fn trace_insertion_session() {
    let (left, right) = diverged_insertions();
    let (traced, predicted) = trace_point("insertions: 2048 common + 512/side", left, right);
    assert_eq!(
        traced.trace.hops(),
        predicted.hops,
        "insertion-shaped session hop count"
    );
}

/// Traces the serialized hop structure of a redaction-shaped divergent
/// session, and checks that its hop count is the one the rule predicts.
///
/// Each redacted leaf is exclusive to the side that kept it, and costs the
/// hops of any other exclusive leaf at its parting depth.
#[test]
fn trace_redaction_session() {
    let (left, right) = diverged_redactions();
    let (traced, predicted) =
        trace_point("redactions: 2048 common, 256 redacted/side", left, right);
    assert_eq!(
        traced.trace.hops(),
        predicted.hops,
        "redaction-shaped session hop count"
    );
}

/// Two peers with disjoint exclusive content and no dispute, the smaller
/// side holding a two-leaf subtree under one root radix.
///
/// The staging requires the initiator's two leaf paths to share their
/// first byte while its ballast counterpart's three land under other root
/// radices, so no root child is populated on both sides and the session is
/// pure transfer in both directions. A leaf's path is its version's hash
/// keyed by the network's secret, and the network is seeded, so the shape is
/// a property of the created version sequence; the self-checks below verify
/// it.
fn transfer_pair() -> (Rumors<u64>, Rumors<u64>) {
    // Stage by pool search: paths derive from versions under the seeded
    // network's secret, so the shape is a deterministic function of the
    // send order — send a pool, pick the versions whose
    // paths land the shape, redact the rest. The left peer keeps exactly two leaves sharing a root radix;
    // the right keeps three ballast leaves outside it and advertises the
    // larger set.
    let keep_only = |rumors: &Rumors<u64>, keep: &[u64]| {
        let losers: Vec<Version> = rumors
            .snapshot()
            .iter()
            .filter(|(_, m)| !keep.contains(m))
            .map(|(v, _)| v.clone())
            .collect();
        rumors.redact_all(&losers);
    };

    let left = NETWORK
        .peer()
        .sync_memory_budget(default::SYNC_MEMORY_BUDGET)
        .into_rumors();
    let right = bootstrap_fork(&left, default::TARGET_MESSAGE_SIZE);
    {
        left.send_all(0..64u64).unwrap();
    }
    let mut pool: Vec<(u64, u8)> = left
        .snapshot()
        .iter()
        .map(|(v, m)| (*m, path_radix(&left, v)))
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
        .filter(|(v, _)| path_radix(&right, v) != radix)
        .take(3)
        .map(|(_, m)| *m)
        .collect();
    assert_eq!(ballast.len(), 3, "the ballast pool cannot fill its quota");
    keep_only(&right, &ballast);

    // Fixture self-checks: mirror the required shape so drift in hashing
    // or version assignment fails here, not in the hop arithmetic.
    let radices: Vec<u8> = left
        .snapshot()
        .versions()
        .map(|v| path_radix(&left, v))
        .collect();
    assert_eq!(radices.len(), 2, "the left peer holds exactly the pair");
    assert_eq!(radices.first(), radices.last(), "one exclusive subtree");
    assert!(
        right
            .snapshot()
            .versions()
            .all(|v| path_radix(&right, v) != radices[0]),
        "no root child is populated on both sides"
    );
    assert!(
        left.snapshot().len() < right.snapshot().len(),
        "the subtree holder advertises the smaller set and initiates"
    );
    (left, right)
}

/// Traces a pure transfer whose initiator holds an exclusive root child,
/// and checks the hop ladder of its supplies.
///
/// This is the rule's case of a leaf at parting depth 1 whose holder, the
/// initiator, lists depth 1, so the session takes 3 + 1 + 1 = 5 hops. In
/// order: the preambles land at hop 1 and the greetings at hop 2. The
/// responder's opening reply departs at hop 2, requesting the initiator's
/// exclusive root child. The initiator's supplies answer it, departing at
/// hop 3 on its level-2 stream and landing at hop 4, and the completion
/// markers land at hop 5. The initiator never sends a frame on its unused
/// index 0.
#[test]
fn trace_bulk_initiator_session() {
    let (left, right) = transfer_pair();
    let (traced, predicted) = trace_point("pure transfer: 2 vs 3 exclusive messages", left, right);
    let critical = predicted
        .critical
        .expect("the staged pair holds exclusive leaves");
    assert_eq!(
        (predicted.initiator, critical.holder, critical.depth),
        (Some(Side::A), Side::A, 1),
        "the staged shape is an initiator-held leaf parting at depth 1",
    );
    assert_eq!(predicted.hops, 5, "the rule's count for this case");
    assert_eq!(traced.trace.hops(), 5, "bulk-initiator session hop count");
    assert_eq!(
        traced.trace.first_frame_hop('A', 1),
        3,
        "the supplies answer the responder's request as it arrives"
    );
    assert!(
        !traced.trace.sent_on('A', 0),
        "the initiator never sends on its unused index 0"
    );
}

/// Traces the session between identical peers: the protocol floor every
/// divergent trace is read against.
///
/// This is the rule's case of equal versions, which takes 3 hops: the
/// root-fan listing rides the greeting hop itself, and converged peers end
/// the session at the greeting, so only the completion markers follow.
#[test]
fn trace_empty_session() {
    let left = NETWORK
        .peer()
        .sync_memory_budget(default::SYNC_MEMORY_BUDGET)
        .into_rumors();
    // Fixed, so the trace prints the same session every run.
    let mut rng = ChaChaRng::seed_from_u64(0x1234_5678_9abc_def0);
    send_random(&left, 64, &mut rng);
    let right = bootstrap_fork(&left, default::TARGET_MESSAGE_SIZE);
    let (traced, predicted) =
        trace_point("empty divergence: identical 64-message peers", left, right);
    assert_eq!(predicted.hops, 3, "the rule's count for equal versions");
    assert_eq!(traced.trace.hops(), 3, "converged-session hop count");
}

/// The largest shared history a generated pair forks from.
///
/// Parting depths grow with the logarithm, base 256, of the other side's
/// leaf count, so histories up to this size draw exclusive leaves parting at
/// depths 1 through 3, and occasionally 4: both parities of the listing
/// side, under both roles.
const MAX_COMMON: usize = 2_048;

/// The most messages one side of a generated pair sends after the fork.
const MAX_INSERTS: usize = 256;

/// The most shared messages one side of a generated pair redacts.
const MAX_REDACTIONS: usize = 256;

/// The largest payload, in bytes, of a generated pair's messages.
///
/// With [`MAX_INSERTS`] and [`MAX_REDACTIONS`], it bounds what one side
/// supplies to about 1 MiB, within the window share
/// [`assert_in_scope`] checks.
const MAX_PAYLOAD: usize = 2_048;

/// The most bytes a supply frame grows to under a generated small target
/// message size, so that supply runs split across many frames.
const MAX_SMALL_TARGET: usize = 1_024;

/// The divisor of the case count of `hops_follow_the_rule`: each case runs
/// one bootstrap and one traced session, the session class.
///
/// The traced session's runtime has timers, but its clock is paused and
/// advances without waiting, so the timers cost no wall time.
const HOP_RULE_DIVISOR: u32 = rumors::testing::property::SESSION_DIVISOR;

/// How one side of a generated pair diverges from the shared history.
#[derive(Clone, Debug)]
struct Divergence {
    /// How many messages this side sends after the fork.
    inserts: usize,
    /// Positions, among the messages this side sent after the fork, of the
    /// ones it redacts again.
    retracted: Vec<Index>,
    /// Positions, among the shared messages, of the ones this side redacts.
    redacted: Vec<Index>,
    /// This side's target message size; the session runs at the smaller of
    /// the two.
    target_message_size: usize,
}

impl Divergence {
    /// Diverge `side` from the shared history held in `shared`, sending
    /// `payload` as every new message.
    fn apply(&self, side: &Rumors<Bytes>, shared: &rumors::Snapshot<Bytes>, payload: &Bytes) {
        side.send_all(iter::repeat_n(payload.clone(), self.inserts))
            .expect("a byte payload encodes");
        let sent: Vec<Version> = side
            .snapshot()
            .versions()
            .filter(|version| !shared.contains(version))
            .cloned()
            .collect();
        let shared: Vec<Version> = shared.versions().cloned().collect();
        let mut redacted = picked(&sent, &self.retracted);
        redacted.extend(picked(&shared, &self.redacted));
        side.redact_all(&redacted);
    }
}

/// The distinct versions `positions` pick out of `versions`; none when
/// `versions` is empty.
fn picked(versions: &[Version], positions: &[Index]) -> Vec<Version> {
    if versions.is_empty() {
        return Vec::new();
    }
    let distinct: BTreeSet<usize> = positions
        .iter()
        .map(|position| position.index(versions.len()))
        .collect();
    distinct.into_iter().map(|i| versions[i].clone()).collect()
}

/// A generated replica pair: a shared history, then each side's divergence.
#[derive(Clone, Debug)]
struct PairPlan {
    /// How many messages the network's first peer sends before the second
    /// joins it.
    common: usize,
    /// The payload length, in bytes, of every message.
    payload_len: usize,
    /// How side `'A'`, the network's first peer, diverges.
    a: Divergence,
    /// How side `'B'`, the joiner, diverges.
    b: Divergence,
}

impl PairPlan {
    /// Build the pair in the network `network` names: side `'A'` creates the
    /// network and sends the shared history, side `'B'` joins it by
    /// bootstrap, and then each side diverges.
    fn build(&self, network: NetworkSeed) -> (Rumors<Bytes>, Rumors<Bytes>) {
        let payload = Bytes::from(vec![0; self.payload_len]);
        let a = network
            .peer()
            .sync_memory_budget(default::SYNC_MEMORY_BUDGET)
            .target_message_size(self.a.target_message_size)
            .into_rumors();
        a.send_all(iter::repeat_n(payload.clone(), self.common))
            .expect("a byte payload encodes");
        let b = bootstrap_fork(&a, self.b.target_message_size);
        let shared = a.snapshot();
        self.a.apply(&a, &shared, &payload);
        self.b.apply(&b, &shared, &payload);
        (a, b)
    }
}

/// Counts up to `max`, spread across orders of magnitude: a few, dozens, or
/// up to `max`, so that small and large cases are both common.
fn spread(max: usize) -> impl Strategy<Value = usize> + Clone {
    prop_oneof![0..=4usize, 0..=64usize, 0..=max]
}

/// One side's divergence: insertions, redactions of its own new messages
/// and of shared ones, and a target message size that is the minimum, small,
/// or the default.
fn divergence() -> impl Strategy<Value = Divergence> {
    (
        spread(MAX_INSERTS),
        spread(MAX_INSERTS).prop_flat_map(|n| prop::collection::vec(any::<Index>(), n)),
        spread(MAX_REDACTIONS).prop_flat_map(|n| prop::collection::vec(any::<Index>(), n)),
        prop_oneof![
            Just(0),
            1..=MAX_SMALL_TARGET,
            Just(default::TARGET_MESSAGE_SIZE)
        ],
    )
        .prop_map(
            |(inserts, retracted, redacted, target_message_size)| Divergence {
                inserts,
                retracted,
                redacted,
                target_message_size,
            },
        )
}

/// A replica pair to trace: a shared history of up to [`MAX_COMMON`]
/// messages, payloads empty, small, or up to [`MAX_PAYLOAD`] bytes, and an
/// independent divergence on each side.
fn pair_plan() -> impl Strategy<Value = PairPlan> {
    (
        prop_oneof![0..=8usize, 0..=256usize, 0..=MAX_COMMON],
        prop_oneof![Just(0), 1..=64usize, 0..=MAX_PAYLOAD],
        divergence(),
        divergence(),
    )
        .prop_map(|(common, payload_len, a, b)| PairPlan {
            common,
            payload_len,
            a,
            b,
        })
}

divided_proptest! {
    #![divisor(HOP_RULE_DIVISOR)]

    /// A session's traced hop count equals the count the rule predicts from
    /// the two replicas' leaf paths and latest versions alone ([`rule`]),
    /// for every network secret.
    ///
    /// The generated pairs diverge by insertions and redactions on either
    /// side, with every payload size and target message size the strategy
    /// draws, so supply runs split across frames and replies across chunks.
    /// Each case also checks that it met the rule's premises, that the
    /// session elected the initiator the rule names, and that it converged.
    #[test]
    fn hops_follow_the_rule(plan in pair_plan(), network in network_seed()) {
        let (a, b) = plan.build(network);
        let predicted = rule::predict(&Replica::of(&a), &Replica::of(&b));
        let traced = traced_session(a, b);
        assert_in_scope(&traced);
        if let Some(initiator) = predicted.initiator {
            let responder = initiator.other().label();
            assert!(
                traced.trace.sent_on(responder, 0),
                "the responder the rule names, {responder}, sends the opening reply ({predicted:?})",
            );
        }
        assert_eq!(
            traced.trace.hops(),
            predicted.hops,
            "traced hops differ from the rule's prediction {predicted:?}",
        );
        assert_converged(&traced);
    }
}

/// The shallowest parting depth [`deep_parting_depths_follow_the_rule`]
/// places: the first beyond those [`hops_follow_the_rule`]'s hashed pairs
/// reach reliably ([`MAX_COMMON`]).
const PLACED_SHALLOWEST_DEPTH: usize = 5;

/// The depth of a leaf: the deepest a leaf can part, sharing all but its
/// last path byte with a leaf the other side holds.
const LEAF_DEPTH: usize = 32;

/// The messages both sides of a placed pair hold, by value; the first sits
/// at [`PLACED_BASE`].
const PLACED_SHARED: [u64; 3] = [0, 1, 2];

/// The one message only side `A` of a placed pair holds, the exclusive
/// leaf whose parting depth the pair places.
const PLACED_EXCLUSIVE: u64 = 100;

/// The messages side `B` of a placed pair sends when `A` is to initiate:
/// they make `B` hold more messages than `A`.
const PLACED_BALLAST: [u64; 2] = [200, 201];

/// The path of the first shared message, beside which the exclusive leaf
/// parts.
const PLACED_BASE: [u8; 32] = [0x10; 32];

/// The path the leaf of the message `value` takes in a placed pair whose
/// exclusive leaf parts at `depth`.
///
/// The exclusive leaf's path is [`PLACED_BASE`] with byte `depth - 1`
/// changed, so it shares exactly `depth - 1` bytes with a leaf both sides
/// hold, and parts at `depth`. Every other leaf takes a path whose bytes
/// all equal one root radix of its own, which no other leaf's path starts
/// with, so the ballast parts at depth 1 and its replies sit far shallower
/// than the exclusive leaf's.
fn placed_path(value: u64, depth: usize) -> [u8; 32] {
    let uniform = |radix: u8| [radix; 32];
    match value {
        PLACED_EXCLUSIVE => {
            let mut path = PLACED_BASE;
            path[depth - 1] += 1;
            path
        }
        shared if PLACED_SHARED.contains(&shared) => {
            uniform(PLACED_BASE[0] * (1 + u8::try_from(shared).expect("few shared messages")))
        }
        ballast if PLACED_BALLAST.contains(&ballast) => {
            uniform(0xf0 + u8::try_from(ballast - PLACED_BALLAST[0]).expect("few ballast messages"))
        }
        other => panic!("message {other} is not part of a placed pair"),
    }
}

/// Build a placed pair: side `A` creates the network and sends the shared
/// messages, side `B` joins it by bootstrap, `A` sends the exclusive
/// message, and, if `a_initiates`, `B` sends the ballast.
///
/// Inside a placement ([`with_leaf_paths`]), every leaf takes the path
/// [`placed_path`] gives its message.
fn placed_pair(a_initiates: bool) -> (Rumors<u64>, Rumors<u64>) {
    let a = NETWORK
        .peer()
        .sync_memory_budget(default::SYNC_MEMORY_BUDGET)
        .into_rumors();
    a.send_all(PLACED_SHARED).expect("a u64 encodes");
    let b = bootstrap_fork(&a, default::TARGET_MESSAGE_SIZE);
    a.send_all([PLACED_EXCLUSIVE]).expect("a u64 encodes");
    if a_initiates {
        b.send_all(PLACED_BALLAST).expect("a u64 encodes");
    }
    (a, b)
}

/// Every parting depth from [`PLACED_SHALLOWEST_DEPTH`] to the leaves takes
/// the hops the rule predicts, both when the exclusive leaf's holder lists
/// its parting depth and when the side lacking it does.
///
/// Hashing cannot place leaves that share more than a few bytes, so each
/// pair places them ([`placed_path`]): side `A` alone holds one leaf, which
/// shares all but its parting depth's byte with a leaf both sides hold. A
/// pair in which `B` also sends the ballast has `A` initiate; otherwise `A`
/// responds. The initiator lists the odd depths and the responder the even
/// ones, so the two roles cover both cases of the rule at every depth: the
/// session takes `3 + d` hops when the lacking side lists `d` and the
/// holder supplies the leaf in its reply about depth `d`, and `3 + d + 1`
/// when the holder lists `d` and the lacking side must request the node,
/// which the holder answers one level deeper. At depth 32 that request is
/// for the single leaf, answered on the leaf stream.
///
/// Each case checks the rule's prediction against the case it staged
/// (initiator, holder, parting depth, and who lists it) before checking the
/// traced count against the prediction, so a staging that drifted fails as
/// such, not as a miscount. A first build of each role's pair, outside the
/// placement, learns each message's version, since forking and sending a
/// seeded network are deterministic; the placement panics on any version it
/// does not list, so a rebuild that drifted fails loudly.
#[test]
fn deep_parting_depths_follow_the_rule() {
    for a_initiates in [true, false] {
        // Both sides hold the shared messages; each is listed once, by its
        // version's canonical bytes.
        let versions: BTreeMap<Vec<u8>, (Version, u64)> = {
            let (a, b) = placed_pair(a_initiates);
            [a, b]
                .iter()
                .flat_map(|side| {
                    side.snapshot()
                        .iter()
                        .map(|(version, value)| {
                            (version.as_bytes().to_vec(), (version.clone(), *value))
                        })
                        .collect::<Vec<_>>()
                })
                .collect()
        };
        for depth in PLACED_SHALLOWEST_DEPTH..=LEAF_DEPTH {
            let paths = versions
                .values()
                .map(|(version, value)| (version.clone(), placed_path(*value, depth)));
            rumors::testing::with_leaf_paths(paths, || {
                let (a, b) = placed_pair(a_initiates);
                let predicted = rule::predict(&Replica::of(&a), &Replica::of(&b));
                let holder_lists = a_initiates == (depth % 2 == 1);
                let staged = (
                    Some(if a_initiates { Side::A } else { Side::B }),
                    Some((Side::A, depth, holder_lists)),
                );
                assert_eq!(
                    (
                        predicted.initiator,
                        predicted
                            .critical
                            .map(|leaf| (leaf.holder, leaf.depth, leaf.holder_lists)),
                    ),
                    staged,
                    "the rule reads the staged case at depth {depth}: {predicted:?}",
                );
                assert_eq!(
                    predicted.hops,
                    rule::SETUP_AND_CLOSING_HOPS + (depth + usize::from(holder_lists)) as u64,
                    "the rule's count at depth {depth}",
                );
                let traced = traced_session(a, b);
                assert_in_scope(&traced);
                assert_eq!(
                    traced.trace.hops(),
                    predicted.hops,
                    "traced hops at parting depth {depth}, the holder listing it: {holder_lists}",
                );
                assert_converged(&traced);
            });
        }
    }
}
