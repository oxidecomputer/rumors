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
use rumors::{
    Rumors,
    testing::{LinkCapture, render_v2_capture},
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

/// Capture and render an arbitrary pair of V2 protocol sessions.
///
/// Each side is a closure handed its recorded link end; it returns the
/// future that drives its role (`gossip`, `bootstrap`, `retire`, …). The
/// renderer preserves exact bytes per stream but keys data streams by their
/// labeled index, which is the V2 protocol's deterministic observable
/// ordering. A driver must run its session to completion and assert its own
/// outcome.
///
/// [`capture_gossip`] is the gossip/gossip specialization; the bootstrap and
/// retire snapshot suites build the asymmetric pairings on top of this.
pub fn capture_session<DriveA, DriveB, FutA, FutB>(drive_a: DriveA, drive_b: DriveB) -> String
where
    DriveA: FnOnce(CaptureLink) -> FutA,
    DriveB: FnOnce(CaptureLink) -> FutB,
    FutA: Future<Output = ()>,
    FutB: Future<Output = ()>,
{
    let (a, b) = capture_sides(drive_a, drive_b);
    render_v2_capture(&a, &b)
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
    capture_session(
        move |mut link| async move {
            a.gossip(&mut link).await.expect("gossip A");
        },
        move |mut link| async move {
            b.gossip(&mut link).await.expect("gossip B");
        },
    )
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
