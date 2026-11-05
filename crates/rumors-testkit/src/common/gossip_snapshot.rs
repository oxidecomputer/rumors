//! Record protocol traffic for byte-level assertions and wire snapshots.
//!
//! A capture records every byte each side writes to and reads from its end
//! of an in-memory connection, independently of the observation hook. The
//! reads let the driver check that both peers consumed every byte before
//! returning.
//!
//! One pipe per direction carries the preamble, the greeting, every logical
//! stream's frames, and the closing items, so a capture preserves the
//! multiplexer's interleaving. The snapshot renderer groups frames by the
//! logical stream the observation hook names, so that interleaving never
//! changes a snapshot. Re-accept a snapshot only after a deliberate
//! protocol change.
//!
//! Each in-memory connection end is wrapped at both halves, which share a
//! `Log`. These transport records can then be compared with observation
//! callbacks or parsed without using the Rumors codec.

use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use proptest::collection::vec;
use proptest::prelude::*;

use rumors::observe::{
    Direction, Observer, Role, SessionInfo, SessionObserver, StreamId, StreamInfo, StreamObserver,
};
use rumors::{
    Rumors,
    testing::{
        HookCapture, HookStream, MemoryEnd, PipeCapture, assert_items_account_for,
        memory_connection, render_hook_capture,
    },
};
use tokio::io::{AsyncRead, AsyncWrite, BufReader, DuplexStream, ReadBuf};

use crate::common::seed::NetworkSeed;
use crate::common::wire::block_on;

use serde::Serialize;
use serde::de::DeserializeOwned;

/// The network seed of every snapshot fixture: the snapshots pin exact bytes,
/// which hold for one secret.
pub const SNAPSHOT_NETWORK_SEED: NetworkSeed = NetworkSeed::new(0);

/// Create the snapshots' first peer from [`SNAPSHOT_NETWORK_SEED`].
pub fn seeded<T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static>() -> Rumors<T> {
    SNAPSHOT_NETWORK_SEED
        .peer()
        .sync_memory_budget(0)
        .into_rumors()
}

/// Short byte-payload collections, including empty sets, for wire tests.
pub fn payloads() -> impl Strategy<Value = Vec<Vec<u8>>> + Clone {
    vec(vec(any::<u8>(), 0..48), 0..12)
}

/// Shared and per-peer payloads for equal or divergent message sets.
#[allow(clippy::type_complexity)]
pub fn corpora() -> impl Strategy<Value = (Vec<Vec<u8>>, Vec<Vec<u8>>, Vec<Vec<u8>>)> {
    (payloads(), payloads(), payloads())
}

/// Whether a logged byte run was put on the wire or taken off it, from the
/// perspective of the peer that performed the I/O.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Op {
    /// Bytes accepted by `poll_write`: the peer sent them.
    Send,
    /// Bytes delivered by `poll_read`: the peer received them.
    Recv,
}

/// One recorded I/O event: a contiguous run of bytes that a single peer
/// sent or received in one `poll_write` / `poll_read`.
struct Event {
    /// The peer that performed the I/O (`"A"` or `"B"`).
    peer: &'static str,
    op: Op,
    bytes: Vec<u8>,
}

/// A shared, append-only, globally-ordered event log. The push order across
/// both peers *is* the captured interleaving.
///
/// `Arc<Mutex<…>>` rather than `Rc<RefCell<…>>` because `gossip` requires
/// its transport to be `Send`; the mutex is uncontended in practice since
/// the deterministic executor only ever polls one peer at a time.
#[derive(Clone, Default)]
struct Log(Arc<Mutex<Vec<Event>>>);

impl Log {
    fn record(&self, peer: &'static str, op: Op, bytes: &[u8]) {
        self.0.lock().unwrap().push(Event {
            peer,
            op,
            bytes: bytes.to_vec(),
        });
    }
}

/// An [`AsyncRead`] + [`AsyncWrite`] wrapper around one half that records
/// every byte crossing it into a shared `Log`.
pub struct Recorder {
    inner: DuplexStream,
    peer: &'static str,
    log: Log,
}

impl AsyncRead for Recorder {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let before = buf.filled().len();
        let poll = Pin::new(&mut this.inner).poll_read(cx, buf);
        if let Poll::Ready(Ok(())) = &poll {
            let delivered = &buf.filled()[before..];
            if !delivered.is_empty() {
                this.log.record(this.peer, Op::Recv, delivered);
            }
        }
        poll
    }
}

impl AsyncWrite for Recorder {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        let poll = Pin::new(&mut this.inner).poll_write(cx, buf);
        if let Poll::Ready(Ok(n)) = &poll
            && *n > 0
        {
            this.log.record(this.peer, Op::Send, &buf[..*n]);
        }
        poll
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}

/// The recording connection end handed to each session driver.
pub struct CaptureEnd {
    /// The recorded read half, buffered as a session requires; the
    /// recording sees what the buffer reads.
    pub read: BufReader<Recorder>,
    /// The recorded write half.
    pub write: Recorder,
}

/// One side's capture state, harvested after the session completes.
struct Side {
    log: Log,
    peer: &'static str,
}

impl Side {
    /// Wrap one in-memory connection end in recording halves.
    fn wrap(end: MemoryEnd, peer: &'static str, log: Log) -> (CaptureEnd, Self) {
        let side = Side {
            log: log.clone(),
            peer,
        };
        let end = CaptureEnd {
            read: BufReader::new(Recorder {
                // The connection end is fresh, so its buffer holds nothing.
                inner: end.read.into_inner(),
                peer,
                log: log.clone(),
            }),
            write: Recorder {
                inner: end.write,
                peer,
                log,
            },
        };
        (end, side)
    }

    /// Harvest this side's complete outgoing capture.
    fn capture(&self) -> PipeCapture {
        PipeCapture {
            sent: sent(self.peer, &self.log.0.lock().unwrap()),
        }
    }
}

/// The hook-side recorder a capture driver attaches to its peer.
///
/// Records one session: its elected role and every *sent* directed
/// stream's items, each stamped from one session-scoped counter so their
/// observation order survives. The received directions are declined: each
/// side of a capture renders what it sent, and the two sides together
/// cover both directions.
#[derive(Default)]
pub struct HookRecorder {
    sessions: Mutex<Vec<Arc<RecordedSession>>>,
}

/// One observed session's recording.
#[derive(Default)]
struct RecordedSession {
    role: Mutex<Option<Role>>,
    streams: Mutex<Vec<Arc<RecordedStream>>>,
    /// The next observation stamp, shared by every stream of the session.
    next: Arc<AtomicUsize>,
}

/// One observed sent stream: its identity and its stamped items, in order.
struct RecordedStream {
    id: StreamId,
    items: Mutex<Vec<(usize, Vec<u8>)>>,
}

impl Observer for HookRecorder {
    fn session(&self, _: &SessionInfo) -> Option<Box<dyn SessionObserver>> {
        let session = Arc::new(RecordedSession::default());
        self.sessions.lock().unwrap().push(session.clone());
        Some(Box::new(RecordSession(session)))
    }
}

struct RecordSession(Arc<RecordedSession>);

impl SessionObserver for RecordSession {
    fn elected(&self, role: Role) {
        *self.0.role.lock().unwrap() = Some(role);
    }

    fn stream(&self, stream: &StreamInfo) -> Option<Box<dyn StreamObserver>> {
        if stream.direction != Direction::Sent {
            return None;
        }
        let recorded = Arc::new(RecordedStream {
            id: stream.id,
            items: Mutex::new(Vec::new()),
        });
        self.0.streams.lock().unwrap().push(recorded.clone());
        Some(Box::new(RecordStream {
            stream: recorded,
            next: self.0.next.clone(),
        }))
    }
}

struct RecordStream {
    stream: Arc<RecordedStream>,
    next: Arc<AtomicUsize>,
}

impl StreamObserver for RecordStream {
    fn message(&mut self, bytes: &[u8]) {
        let stamp = self.next.fetch_add(1, Ordering::Relaxed);
        self.stream
            .items
            .lock()
            .unwrap()
            .push((stamp, bytes.to_vec()));
    }
}

/// Attach `hook` to a sole-handle [`Rumors`], for drivers whose peer is
/// already past its builder. Panics if other clones are live: capture
/// drivers own their peers.
pub async fn observed<T>(rumors: Rumors<T>, hook: Arc<HookRecorder>) -> Rumors<T>
where
    T: Send + Sync + 'static,
{
    rumors
        .try_into_peer()
        .await
        .expect("capture drivers hold the sole handle")
        .observe(hook)
        .into_rumors()
}

/// Harvest one side's [`HookCapture`], held to the transport oracle.
///
/// Every observed item, control and frame alike, taken in observation
/// order, must reproduce the pipe's sent bytes exactly. That assertion is
/// what licenses rendering hook items as a pin of wire bytes.
fn hook_capture(recorder: &HookRecorder, transport: &PipeCapture) -> HookCapture {
    let sessions = recorder.sessions.lock().unwrap();
    assert_eq!(sessions.len(), 1, "one captured session per driver");
    let session = &sessions[0];
    let role = *session.role.lock().unwrap();
    let streams = session.streams.lock().unwrap();

    let mut stamped: Vec<(usize, Vec<u8>)> = streams
        .iter()
        .flat_map(|stream| stream.items.lock().unwrap().clone())
        .collect();
    stamped.sort_by_key(|(stamp, _)| *stamp);
    let in_order: Vec<Vec<u8>> = stamped.into_iter().map(|(_, item)| item).collect();
    assert_items_account_for(&in_order, &transport.sent);

    let items = |stream: &RecordedStream| -> Vec<Vec<u8>> {
        let items = stream.items.lock().unwrap();
        items.iter().map(|(_, item)| item.clone()).collect()
    };
    let control = streams
        .iter()
        .find(|stream| matches!(stream.id, StreamId::Control))
        .map(|stream| items(stream))
        .expect("every session observes its sent control items");
    let data = streams
        .iter()
        .filter_map(|stream| match stream.id {
            StreamId::Data { speaker, index } => Some(HookStream {
                index,
                speaker,
                items: items(stream),
            }),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        data.iter().all(|stream| !stream.items.is_empty()),
        "every observed logical stream carried a frame"
    );

    HookCapture {
        role,
        control,
        streams: data,
    }
}

/// Capture and render an arbitrary pair of V2 protocol sessions.
///
/// Each side is a closure handed its recorded connection end and its hook
/// recorder; it attaches the recorder to its peer (or builder) and
/// returns the future that drives its role (`gossip`, `bootstrap`,
/// `retire`, …). Rendering consumes the observation hook's items — the
/// instrument enters through the public door — while the transport
/// capture stays as the totality oracle behind the byte-pin claim. The
/// renderer preserves exact items per logical stream but keys the streams
/// by their observed index, which is the V2 protocol's deterministic
/// observable ordering. A driver must run its session to completion
/// and assert its own outcome.
///
/// [`capture_gossip`] is the gossip/gossip specialization; the bootstrap and
/// retire snapshot suites build the asymmetric pairings on top of this.
pub fn capture_session<DriveA, DriveB, FutA, FutB>(drive_a: DriveA, drive_b: DriveB) -> String
where
    DriveA: FnOnce(CaptureEnd, Arc<HookRecorder>) -> FutA,
    DriveB: FnOnce(CaptureEnd, Arc<HookRecorder>) -> FutB,
    FutA: Future<Output = ()>,
    FutB: Future<Output = ()>,
{
    let hook_a = Arc::new(HookRecorder::default());
    let hook_b = Arc::new(HookRecorder::default());
    let (a, b) = {
        let (hook_a, hook_b) = (hook_a.clone(), hook_b.clone());
        capture_sides(
            move |end| drive_a(end, hook_a),
            move |end| drive_b(end, hook_b),
        )
    };
    render_hook_capture(&hook_capture(&hook_a, &a), &hook_capture(&hook_b, &b))
}

/// Drive both roles over recording connections and return both sides'
/// captures.
///
/// The raw form of [`capture_session`], for suites that inspect the
/// captured bytes themselves (the wire-legibility property) rather than
/// the rendered transcript.
pub fn capture_sides<DriveA, DriveB, FutA, FutB>(
    drive_a: DriveA,
    drive_b: DriveB,
) -> (PipeCapture, PipeCapture)
where
    DriveA: FnOnce(CaptureEnd) -> FutA,
    DriveB: FnOnce(CaptureEnd) -> FutB,
    FutA: Future<Output = ()>,
    FutB: Future<Output = ()>,
{
    let log = Log::default();
    let (a_end, b_end) = memory_connection(rumors::testing::MEMORY_CAPACITY);
    let (a_end, a_side) = Side::wrap(a_end, "A", log.clone());
    let (b_end, b_side) = Side::wrap(b_end, "B", log.clone());
    block_on(async {
        tokio::join!(drive_a(a_end), drive_b(b_end));
    });
    assert_capture_drained(&log.0.lock().unwrap());
    (a_side.capture(), b_side.capture())
}

/// Assert the captured session drained the connection in both directions:
/// every byte each peer sent, the other received.
///
/// The same invariant [`assert_connection_drained`] probes on a bare
/// memory connection, in the form this harness can check: the
/// shared [`Log`] records both peers' I/O, so per direction the received
/// total must equal the sent total (delivery preserves order, so equal
/// totals mean the halves rest at the session boundary). Every capture
/// driver asserts a successful outcome, so the drain invariant applies to
/// every captured session.
///
/// [`assert_connection_drained`]: crate::common::wire::assert_connection_drained
fn assert_capture_drained(events: &[Event]) {
    for (sender, receiver) in [("A", "B"), ("B", "A")] {
        let sent = sent(sender, events);
        let received = received(receiver, events);
        assert!(
            sent.len() == received.len(),
            "connection not drained: {sender} sent {} byte(s) that {receiver} never read \
             (leftover tail {:02x?})",
            sent.len() - received.len(),
            &sent[received.len()..],
        );
    }
}

/// Gossip `a` and `b` through recording connections (the gossip/gossip
/// specialization of [`capture_session`]). The two sets are expected to
/// reconcile cleanly; a gossip error panics the helper.
pub fn capture_gossip<T>(a: Rumors<T>, b: Rumors<T>) -> String
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    capture_gossip_returning(a, b).0
}

/// [`capture_gossip`], handing the two live handles back after the
/// session, for tests that assert on post-session replica state.
///
/// The hook attaches by briefly reclaiming each `Peer` (the observation
/// hook's attach point), which requires the passed handle to be its
/// peer's sole one — so a caller wanting post-state must take its
/// handles back here rather than holding clones across the capture (a
/// held clone stalls the reclaim, and the deterministic executor
/// reports the whole session stalled).
pub fn capture_gossip_returning<T>(a: Rumors<T>, b: Rumors<T>) -> (String, Rumors<T>, Rumors<T>)
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    let a_out = Arc::new(Mutex::new(None));
    let b_out = Arc::new(Mutex::new(None));
    let rendered = {
        let (a_out, b_out) = (a_out.clone(), b_out.clone());
        capture_session(
            move |mut end, hook| async move {
                let a = observed(a, hook).await;
                a.gossip_once(&mut end.read, &mut end.write)
                    .await
                    .expect("gossip A");
                *a_out.lock().unwrap() = Some(a);
            },
            move |mut end, hook| async move {
                let b = observed(b, hook).await;
                b.gossip_once(&mut end.read, &mut end.write)
                    .await
                    .expect("gossip B");
                *b_out.lock().unwrap() = Some(b);
            },
        )
    };
    let a = a_out.lock().unwrap().take().expect("driver A completed");
    let b = b_out.lock().unwrap().take().expect("driver B completed");
    (rendered, a, b)
}

/// Concatenate every byte one party sent, erasing write chunking.
fn sent(peer: &str, events: &[Event]) -> Vec<u8> {
    events
        .iter()
        .filter(|event| event.peer == peer && event.op == Op::Send)
        .flat_map(|event| event.bytes.iter().copied())
        .collect()
}

/// Concatenate every byte one party received, erasing read chunking.
fn received(peer: &str, events: &[Event]) -> Vec<u8> {
    events
        .iter()
        .filter(|event| event.peer == peer && event.op == Op::Recv)
        .flat_map(|event| event.bytes.iter().copied())
        .collect()
}
