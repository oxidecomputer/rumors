//! Byte-exact wire-capture helpers for `insta` golden snapshots of one
//! session between two peers.
//!
//! Where [`super::wire`] only checks that the two peers *converge*, this
//! helper records the *entire conversation*: every byte each peer puts on the
//! wire. V2 traffic is rendered by physical direction, then demultiplexed by
//! logical stream: ordering within each stream is exact, while incidental
//! scheduling between independent streams is discarded. Representative V1
//! tests retain its strict send/receive timeline. Re-accept a snapshot only
//! after a deliberate protocol change.
//!
//! # Robustness to read/write framing
//!
//! V2 concatenates every byte sent in each physical direction before parsing
//! it into the fixed preamble, exact-framed version, logical stream frames,
//! and any trailing party hand-off. V1 collapses consecutive events in the
//! same direction. Neither representation retains incidental boundaries
//! between individual `poll_write` or `poll_read` calls.
//!
//! # Determinism
//!
//! V2's independent streams may be scheduled in different physical orders,
//! so its renderer sorts stream groups while preserving every exact byte and
//! the complete order within each group. V1 is strictly alternating, and its
//! two peers are driven by `tokio::join!` on a single-threaded runtime so its
//! direction-switching timeline is reproducible.
//!
//! # Interposition
//!
//! Each end of an in-memory `tokio::io::duplex` pipe is wrapped in a
//! [`Recorder`] before being split into the read/write halves handed to the
//! session. The wrapper logs every accepted write and delivered read into one
//! shared, ordered [`Log`], tagged with the acting party. The selected
//! renderer then derives either the stable V2 streams or the V1 timeline.

use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use borsh::{BorshDeserialize, BorshSerialize};
use rumors::{Rumors, testing::render_v2_capture};
use tokio::io::{AsyncRead, AsyncWrite, DuplexStream, ReadBuf, ReadHalf, WriteHalf};

use crate::common::wire::block_on;

/// Capacity of the in-memory duplex pipe. Comfortably larger than any frame
/// the protocol emits for the small payloads these snapshots use, so each
/// logical frame is accepted by a single `poll_write` and the recorded chunk
/// boundaries track message boundaries rather than buffer pressure.
const DUPLEX_BUF: usize = 8 * 1024;

/// Whether a logged byte run was put on the wire or taken off it, from the
/// perspective of the peer that performed the I/O.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Op {
    /// Bytes accepted by `poll_write`: the peer sent them.
    Send,
    /// Bytes delivered by `poll_read`: the peer received them.
    Recv,
}

/// One recorded I/O event: a contiguous run of bytes that a single peer sent
/// or received in one `poll_write` / `poll_read`.
struct Event {
    /// The peer that performed the I/O (`"A"` or `"B"`).
    peer: &'static str,
    op: Op,
    bytes: Vec<u8>,
}

/// A shared, append-only, globally-ordered event log. The push order across
/// both peers *is* the captured interleaving.
///
/// `Arc<Mutex<…>>` rather than `Rc<RefCell<…>>` because `gossip` requires its
/// reader/writer to be `Send`; the mutex is uncontended in practice since the
/// current-thread runtime only ever polls one peer at a time.
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

/// An [`AsyncRead`] + [`AsyncWrite`] wrapper around one end of a duplex pipe
/// that records every byte crossing it into a shared [`Log`].
///
/// Public only so it can name the read/write halves a [`capture_session`]
/// driver receives; its fields and recording behavior stay private.
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

/// Capture and render an arbitrary pair of V2 protocol sessions.
///
/// Each side is a closure handed the read and write halves of its recorded
/// pipe end; it returns the future that drives its role (`gossip`,
/// `bootstrap`, `retire`, …). The renderer preserves exact physical bytes but
/// groups multiplexed frames by logical stream, whose internal order is the
/// V2 protocol's deterministic observable ordering. A driver must run its
/// session to completion and assert its own outcome.
///
/// [`capture_gossip`] is the gossip/gossip specialization; the bootstrap and
/// retire snapshot suites build the asymmetric pairings on top of this.
pub fn capture_session<DriveA, DriveB, FutA, FutB>(drive_a: DriveA, drive_b: DriveB) -> String
where
    DriveA: FnOnce(ReadHalf<Recorder>, WriteHalf<Recorder>) -> FutA,
    DriveB: FnOnce(ReadHalf<Recorder>, WriteHalf<Recorder>) -> FutB,
    FutA: Future<Output = ()>,
    FutB: Future<Output = ()>,
{
    let events = capture_events(drive_a, drive_b);
    render_v2_capture(&sent("A", &events), &sent("B", &events))
}

/// Capture one V1 session in its strict direction-switching timeline.
pub fn capture_session_v1<DriveA, DriveB, FutA, FutB>(drive_a: DriveA, drive_b: DriveB) -> String
where
    DriveA: FnOnce(ReadHalf<Recorder>, WriteHalf<Recorder>) -> FutA,
    DriveB: FnOnce(ReadHalf<Recorder>, WriteHalf<Recorder>) -> FutB,
    FutA: Future<Output = ()>,
    FutB: Future<Output = ()>,
{
    render_v1(&capture_events(drive_a, drive_b))
}

/// Drive both roles and return the complete physical I/O event log.
fn capture_events<DriveA, DriveB, FutA, FutB>(drive_a: DriveA, drive_b: DriveB) -> Vec<Event>
where
    DriveA: FnOnce(ReadHalf<Recorder>, WriteHalf<Recorder>) -> FutA,
    DriveB: FnOnce(ReadHalf<Recorder>, WriteHalf<Recorder>) -> FutB,
    FutA: Future<Output = ()>,
    FutB: Future<Output = ()>,
{
    let log = Log::default();
    block_on(async {
        let (a_end, b_end) = tokio::io::duplex(DUPLEX_BUF);
        let a_rec = Recorder {
            inner: a_end,
            peer: "A",
            log: log.clone(),
        };
        let b_rec = Recorder {
            inner: b_end,
            peer: "B",
            log: log.clone(),
        };
        let (a_r, a_w) = tokio::io::split(a_rec);
        let (b_r, b_w) = tokio::io::split(b_rec);

        tokio::join!(drive_a(a_r, a_w), drive_b(b_r, b_w));
    });

    let mut events = log.0.lock().unwrap();
    std::mem::take(&mut *events)
}

/// Gossip `a` and `b` through the recording pipe (the gossip/gossip
/// specialization of [`capture_session`]). The two sets are expected to
/// reconcile cleanly; a gossip error panics the helper.
pub fn capture_gossip<T>(a: Rumors<T>, b: Rumors<T>) -> String
where
    T: BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    capture_session(
        move |mut r, mut w| async move {
            a.gossip(&mut r, &mut w).await.expect("gossip A");
        },
        move |mut r, mut w| async move {
            b.gossip(&mut r, &mut w).await.expect("gossip B");
        },
    )
}

/// Capture the strict V1 timeline for a gossip/gossip session.
pub fn capture_gossip_v1<T>(a: Rumors<T>, b: Rumors<T>) -> String
where
    T: BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    capture_session_v1(
        move |mut r, mut w| async move {
            a.gossip(&mut r, &mut w).await.expect("V1 gossip A");
        },
        move |mut r, mut w| async move {
            b.gossip(&mut r, &mut w).await.expect("V1 gossip B");
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

/// Concatenate every byte one party sent, erasing physical write chunking.
fn sent(peer: &str, events: &[Event]) -> Vec<u8> {
    events
        .iter()
        .filter(|event| event.peer == peer && event.op == Op::Send)
        .flat_map(|event| event.bytes.iter().copied())
        .collect()
}

/// Build one party's transcript as a list of text lines: a column header and
/// rule, then one stanza per direction-run. Consecutive same-direction events
/// are coalesced into a single block before rendering, so buffer-level chunk
/// boundaries leave no trace.
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
