//! Byte-exact wire-capture helpers for `insta` golden snapshots of one
//! session between two peers.
//!
//! Where [`super::wire`] only checks that the two peers *converge*, this
//! helper records the *entire conversation*: every byte each peer puts on
//! the wire. Re-accept a snapshot only after a deliberate protocol change.
//!
//! # What a capture pins, and what it discards
//!
//! V2 traffic rides a [`rumors::Link`], which keeps logical streams
//! physically separate, so a capture is already demultiplexed: the control
//! stream's exact bytes plus each opened data stream's exact bytes. Two
//! kinds of incidental nondeterminism are erased so snapshots stay stable:
//!
//! - **Read/write framing**: neither representation retains incidental
//!   boundaries between individual `poll_write` or `poll_read` calls — each V2
//!   capture concatenates every byte sent per stream before parsing, and
//!   V1 collapses consecutive events in the same direction.
//! - **Cross-stream scheduling**: independent streams may be polled in
//!   different orders, so the V2 renderer keys stream groups by their
//!   labeled index — the protocol's deterministic observable ordering —
//!   while preserving every exact byte and the complete order within each
//!   group.
//!
//! Representative V1 tests instead retain the control stream's strict
//! send/receive timeline: V1 is strictly alternating, and its two peers are
//! driven by `tokio::join!` on a deterministic executor, so the
//! direction-switching timeline is reproducible.
//!
//! # Interposition
//!
//! Each side's in-memory link is rebuilt with recording parts: the control
//! halves are wrapped in a [`Recorder`] logging every accepted write and
//! delivered read into one shared, ordered [`Log`] (the V1 timeline needs
//! both directions), and the connector is wrapped so each opened data
//! stream's accepted writes accumulate in a per-stream buffer.

use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use rumors::link::{Connector, Done, Link, LinkParts, MemoryAcceptor, MemoryConnector};
use rumors::observe::{
    Direction, Observer, Role, SessionInfo, SessionObserver, StreamId, StreamInfo, StreamObserver,
};
use rumors::{
    Rumors,
    testing::{
        HookCapture, HookStream, LinkCapture, assert_items_account_for, render_hook_capture,
        stream_label,
    },
};
use tokio::io::{AsyncRead, AsyncWrite, DuplexStream, ReadBuf};

use crate::common::wire::block_on;

use serde::Serialize;
use serde::de::DeserializeOwned;
/// Whether a logged byte run was put on the wire or taken off it, from the
/// perspective of the peer that performed the I/O.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Op {
    /// Bytes accepted by `poll_write`: the peer sent them.
    Send,
    /// Bytes delivered by `poll_read`: the peer received them.
    Recv,
}

/// One recorded control-stream I/O event: a contiguous run of bytes that a
/// single peer sent or received in one `poll_write` / `poll_read`.
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

/// Re-exported so byte-level suites can hold captures directly.
pub use rumors::testing::LinkCapture as CapturedLink;

/// An [`AsyncRead`] + [`AsyncWrite`] wrapper around one control half that
/// records every byte crossing it into a shared [`Log`].
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

/// One data stream's accumulated outgoing bytes.
type StreamBuf = Arc<Mutex<Vec<u8>>>;

/// A connector that gives each opened stream a fresh capture buffer,
/// registered in open order on the side's list.
#[derive(Clone)]
pub struct CaptureConnector {
    inner: MemoryConnector,
    streams: Arc<Mutex<Vec<StreamBuf>>>,
}

impl Connector for CaptureConnector {
    type Tx = CaptureWrite;

    async fn connect(&self) -> io::Result<(Self::Tx, Done<Self::Tx>)> {
        let (tx, _) = self.inner.connect().await?;
        let buffer = StreamBuf::default();
        self.streams.lock().unwrap().push(buffer.clone());
        Ok((CaptureWrite { inner: tx, buffer }, Done::discard()))
    }
}

/// A data-stream writer that appends every accepted byte to its buffer.
pub struct CaptureWrite {
    inner: DuplexStream,
    buffer: StreamBuf,
}

impl AsyncWrite for CaptureWrite {
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
            this.buffer.lock().unwrap().extend_from_slice(&buf[..*n]);
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

/// The recording link handed to each session driver.
pub type CaptureLink = Link<Recorder, Recorder, CaptureConnector, MemoryAcceptor>;

/// One side's capture state, harvested after the session completes.
struct Side {
    log: Log,
    peer: &'static str,
    streams: Arc<Mutex<Vec<StreamBuf>>>,
}

impl Side {
    /// Wrap one memory-link end in recording parts.
    fn wrap(link: rumors::link::MemoryLink, peer: &'static str, log: Log) -> (CaptureLink, Self) {
        let parts = link.into_parts();
        let streams = Arc::new(Mutex::new(Vec::new()));
        let side = Side {
            log: log.clone(),
            peer,
            streams: streams.clone(),
        };
        let link = LinkParts {
            control_read: Recorder {
                inner: parts.control_read,
                peer,
                log: log.clone(),
            },
            control_write: Recorder {
                inner: parts.control_write,
                peer,
                log,
            },
            connector: CaptureConnector {
                inner: parts.connector,
                streams,
            },
            acceptor: parts.acceptor,
            session: parts.session,
        }
        .into_link();
        (link, side)
    }

    /// Harvest this side's complete outgoing capture.
    fn capture(&self) -> LinkCapture {
        LinkCapture {
            control: sent(self.peer, &self.log.0.lock().unwrap()),
            streams: self
                .streams
                .lock()
                .unwrap()
                .iter()
                .map(|buffer| buffer.lock().unwrap().clone())
                .collect(),
        }
    }
}

/// The hook-side recorder a capture driver attaches to its peer.
///
/// Records one session: its elected role and every *sent* directed
/// stream's items. The received directions are declined — each side of
/// a capture renders what it sent, and the two sides together cover
/// both directions.
#[derive(Default)]
pub struct HookRecorder {
    sessions: Mutex<Vec<Arc<RecordedSession>>>,
}

/// One observed session's recording.
#[derive(Default)]
struct RecordedSession {
    role: Mutex<Option<Role>>,
    streams: Mutex<Vec<Arc<RecordedStream>>>,
}

/// One observed sent stream: its identity and its items, in order.
struct RecordedStream {
    id: StreamId,
    items: Mutex<Vec<Vec<u8>>>,
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
        Some(Box::new(RecordStream(recorded)))
    }
}

struct RecordStream(Arc<RecordedStream>);

impl StreamObserver for RecordStream {
    fn message(&mut self, bytes: &[u8]) {
        self.0.items.lock().unwrap().push(bytes.to_vec());
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
/// The stream label plus the concatenated observed items must
/// reproduce each directed stream's wire bytes exactly. That assertion
/// is what licenses rendering hook items as a pin of wire bytes.
fn hook_capture(recorder: &HookRecorder, transport: &LinkCapture) -> HookCapture {
    let sessions = recorder.sessions.lock().unwrap();
    assert_eq!(sessions.len(), 1, "one captured session per driver");
    let session = &sessions[0];
    let role = *session.role.lock().unwrap();
    let streams = session.streams.lock().unwrap();

    let control: Vec<Vec<u8>> = streams
        .iter()
        .find(|stream| matches!(stream.id, StreamId::Control))
        .map(|stream| stream.items.lock().unwrap().clone())
        .expect("every session observes its sent control stream");
    assert_items_account_for(&control, &transport.control);

    let mut data = Vec::new();
    for wire in &transport.streams {
        let ((epoch, index), label_len) = stream_label(wire);
        let recorded = streams
            .iter()
            .find(|stream| matches!(stream.id, StreamId::Data { index: i, .. } if i == index))
            .unwrap_or_else(|| panic!("no hook stream observed wire stream {index}"));
        let StreamId::Data { speaker, .. } = recorded.id else {
            unreachable!("matched a data stream id");
        };
        let items = recorded.items.lock().unwrap().clone();
        assert_items_account_for(&items, &wire[label_len..]);
        data.push(HookStream {
            index,
            speaker,
            epoch,
            wire_len: wire.len(),
            items,
        });
    }
    let observed_data = streams
        .iter()
        .filter(|stream| matches!(stream.id, StreamId::Data { .. }))
        .count();
    assert_eq!(
        observed_data,
        data.len(),
        "every observed data stream has transport bytes"
    );

    HookCapture {
        role,
        control,
        streams: data,
    }
}

/// Capture and render an arbitrary pair of V2 protocol sessions.
///
/// Each side is a closure handed its recorded link end and its hook
/// recorder; it attaches the recorder to its peer (or builder) and
/// returns the future that drives its role (`gossip`, `bootstrap`,
/// `retire`, …). Rendering consumes the observation hook's items — the
/// instrument enters through the public door — while the transport
/// capture stays as the totality oracle behind the byte-pin claim. The
/// renderer preserves exact items per stream but keys data streams by
/// their labeled index, which is the V2 protocol's deterministic
/// observable ordering. A driver must run its session to completion
/// and assert its own outcome.
///
/// [`capture_gossip`] is the gossip/gossip specialization; the bootstrap and
/// retire snapshot suites build the asymmetric pairings on top of this.
pub fn capture_session<DriveA, DriveB, FutA, FutB>(drive_a: DriveA, drive_b: DriveB) -> String
where
    DriveA: FnOnce(CaptureLink, Arc<HookRecorder>) -> FutA,
    DriveB: FnOnce(CaptureLink, Arc<HookRecorder>) -> FutB,
    FutA: Future<Output = ()>,
    FutB: Future<Output = ()>,
{
    let hook_a = Arc::new(HookRecorder::default());
    let hook_b = Arc::new(HookRecorder::default());
    let (a, b) = {
        let (hook_a, hook_b) = (hook_a.clone(), hook_b.clone());
        capture_sides(
            move |link| drive_a(link, hook_a),
            move |link| drive_b(link, hook_b),
        )
    };
    render_hook_capture(&hook_capture(&hook_a, &a), &hook_capture(&hook_b, &b))
}

/// Capture one V1 session in its strict direction-switching timeline.
pub fn capture_session_v1<DriveA, DriveB, FutA, FutB>(drive_a: DriveA, drive_b: DriveB) -> String
where
    DriveA: FnOnce(CaptureLink) -> FutA,
    DriveB: FnOnce(CaptureLink) -> FutB,
    FutA: Future<Output = ()>,
    FutB: Future<Output = ()>,
{
    let log = Log::default();
    let events = capture_events(drive_a, drive_b, &log);
    render_v1(&events)
}

/// Drive both roles over recording links and return both sides' captures.
///
/// The raw form of [`capture_session`], for suites that inspect the
/// captured bytes themselves (the wire-legibility property) rather than
/// the rendered transcript.
pub fn capture_sides<DriveA, DriveB, FutA, FutB>(
    drive_a: DriveA,
    drive_b: DriveB,
) -> (LinkCapture, LinkCapture)
where
    DriveA: FnOnce(CaptureLink) -> FutA,
    DriveB: FnOnce(CaptureLink) -> FutB,
    FutA: Future<Output = ()>,
    FutB: Future<Output = ()>,
{
    let log = Log::default();
    let (a_link, b_link) = rumors::link::memory();
    let (a_link, a_side) = Side::wrap(a_link, "A", log.clone());
    let (b_link, b_side) = Side::wrap(b_link, "B", log.clone());
    block_on(async {
        tokio::join!(drive_a(a_link), drive_b(b_link));
    });
    assert_control_drained(&log.0.lock().unwrap());
    (a_side.capture(), b_side.capture())
}

/// Assert the captured session drained the control stream in both
/// directions: every byte each peer sent, the other received.
///
/// The same invariant `wire::assert_control_drained` probes on a bare
/// memory link, in the form this harness's plumbing affords: the shared
/// [`Log`] records both peers' control I/O, so per direction the received
/// total must equal the sent total (delivery preserves order, so equal
/// totals mean the streams rest at the session boundary). Every capture
/// driver asserts a successful outcome, so the drain invariant applies to
/// every captured session — V1's frozen wire included.
fn assert_control_drained(events: &[Event]) {
    for (sender, receiver) in [("A", "B"), ("B", "A")] {
        let sent = sent(sender, events);
        let received = received(receiver, events);
        assert!(
            sent.len() == received.len(),
            "control stream not drained: {sender} sent {} byte(s) that {receiver} never read \
             (leftover tail {:02x?})",
            sent.len() - received.len(),
            &sent[received.len()..],
        );
    }
}

/// Drive both roles and return the control-stream I/O event log.
fn capture_events<DriveA, DriveB, FutA, FutB>(
    drive_a: DriveA,
    drive_b: DriveB,
    log: &Log,
) -> Vec<Event>
where
    DriveA: FnOnce(CaptureLink) -> FutA,
    DriveB: FnOnce(CaptureLink) -> FutB,
    FutA: Future<Output = ()>,
    FutB: Future<Output = ()>,
{
    let (a_link, b_link) = rumors::link::memory();
    let (a_link, _a_side) = Side::wrap(a_link, "A", log.clone());
    let (b_link, _b_side) = Side::wrap(b_link, "B", log.clone());
    block_on(async {
        tokio::join!(drive_a(a_link), drive_b(b_link));
    });

    let events = {
        let mut events = log.0.lock().unwrap();
        std::mem::take(&mut *events)
    };
    assert_control_drained(&events);
    events
}

/// Gossip `a` and `b` through recording links (the gossip/gossip
/// specialization of [`capture_session`]). The two sets are expected to
/// reconcile cleanly; a gossip error panics the helper.
pub fn capture_gossip<T>(a: Rumors<T>, b: Rumors<T>) -> String
where
    T: Serialize + DeserializeOwned + Send + Sync + 'static,
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
    T: Serialize + DeserializeOwned + Send + Sync + 'static,
{
    let a_out = Arc::new(Mutex::new(None));
    let b_out = Arc::new(Mutex::new(None));
    let rendered = {
        let (a_out, b_out) = (a_out.clone(), b_out.clone());
        capture_session(
            move |mut link, hook| async move {
                let a = observed(a, hook).await;
                a.gossip(&mut link).await.expect("gossip A");
                *a_out.lock().unwrap() = Some(a);
            },
            move |mut link, hook| async move {
                let b = observed(b, hook).await;
                b.gossip(&mut link).await.expect("gossip B");
                *b_out.lock().unwrap() = Some(b);
            },
        )
    };
    let a = a_out.lock().unwrap().take().expect("driver A completed");
    let b = b_out.lock().unwrap().take().expect("driver B completed");
    (rendered, a, b)
}

/// Capture the strict V1 timeline for a gossip/gossip session.
pub fn capture_gossip_v1<T>(a: Rumors<T>, b: Rumors<T>) -> String
where
    T: Serialize + DeserializeOwned + Send + Sync + 'static,
{
    capture_session_v1(
        move |mut link| async move {
            a.gossip(&mut link).await.expect("V1 gossip A");
        },
        move |mut link| async move {
            b.gossip(&mut link).await.expect("V1 gossip B");
        },
    )
}

/// Render the event log as two per-party transcripts laid out side by side,
/// each collapsed by direction (see the module docs).
fn render_v1(events: &[Event]) -> String {
    let left = transcript("A", events);
    let right = transcript("B", events);
    side_by_side(&left, &right)
}

/// Concatenate every control byte one party sent, erasing write chunking.
fn sent(peer: &str, events: &[Event]) -> Vec<u8> {
    events
        .iter()
        .filter(|event| event.peer == peer && event.op == Op::Send)
        .flat_map(|event| event.bytes.iter().copied())
        .collect()
}

/// Concatenate every control byte one party received, erasing read chunking.
fn received(peer: &str, events: &[Event]) -> Vec<u8> {
    events
        .iter()
        .filter(|event| event.peer == peer && event.op == Op::Recv)
        .flat_map(|event| event.bytes.iter().copied())
        .collect()
}

/// Build one party's transcript as a list of text lines: a column header
/// and rule, then one stanza per direction-run.
///
/// Consecutive same-direction events are coalesced into a single block
/// before rendering, so buffer-level chunk boundaries leave no trace.
fn transcript(peer: &str, events: &[Event]) -> Vec<String> {
    // Coalesce consecutive same-`Op` events for this party into runs.
    let mut runs: Vec<(Op, Vec<u8>)> = Vec::new();
    for event in events.iter().filter(|e| e.peer == peer) {
        match runs.last_mut() {
            Some((op, bytes)) if *op == event.op => bytes.extend_from_slice(&event.bytes),
            _ => runs.push((event.op, event.bytes.clone())),
        }
    }

    let mut body: Vec<String> = Vec::new();
    if runs.is_empty() {
        body.push("(no traffic)".to_string());
    }
    for (op, bytes) in &runs {
        let label = match op {
            Op::Send => "sent",
            Op::Recv => "received",
        };
        body.push(format!("{label} {} bytes", bytes.len()));
        body.extend(hex_lines(bytes));
    }

    // Header and a rule sized to the widest body line, so the two columns read
    // as titled, ruled-off panels.
    let width = body.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let mut lines = vec![format!("party {peer}"), "─".repeat(width.max(1))];
    lines.extend(body);
    lines
}

/// `hexdump`-style body lines: 8 bytes per line (narrow enough to sit two
/// transcripts side by side within a terminal), each with a `0000:`-style
/// offset and indented to set it apart from its stanza header.
fn hex_lines(bytes: &[u8]) -> Vec<String> {
    bytes
        .chunks(8)
        .enumerate()
        .map(|(line, chunk)| {
            let mut s = format!("  {:04x}:", line * 8);
            for byte in chunk {
                s.push_str(&format!(" {byte:02x}"));
            }
            s
        })
        .collect()
}

/// Lay two columns of lines beside each other, separated by ` │ `. The left
/// column is padded to its widest line so the separator stays aligned; the
/// shorter column is padded with blank rows. Trailing whitespace is trimmed.
fn side_by_side(left: &[String], right: &[String]) -> String {
    let width = left.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let rows = left.len().max(right.len());
    let mut out = String::new();
    for row in 0..rows {
        let l = left.get(row).map(String::as_str).unwrap_or("");
        let r = right.get(row).map(String::as_str).unwrap_or("");
        let pad = " ".repeat(width - l.chars().count());
        let line = format!("{l}{pad} │ {r}");
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}
