//! Byte-exact wire-capture helper for `insta` golden snapshots of a single
//! round of gossip between two [`rumors::Known`]s.
//!
//! Where [`super::wire`] only checks that the two peers *converge*, this
//! helper records the *entire conversation*: every byte each peer puts on the
//! wire and every byte it pulls back off. It is rendered as two transcripts
//! side by side, one per party, so a drift in framing, message content, or the
//! send/receive ordering of either party surfaces as a snapshot diff.
//! Re-accept a snapshot only after a deliberate protocol change.
//!
//! # Robustness to read/write framing
//!
//! A party's transcript is *collapsed by direction*: consecutive bytes it
//! sends with no intervening receive (and vice versa) are coalesced into one
//! block, regardless of how many `poll_write` / `poll_read` calls the
//! buffering happened to split them across. The capture therefore pins *what*
//! each party sent and received, and in *what order it switched direction* —
//! the protocol's observable behavior — without pinning the incidental chunk
//! boundaries an async reader/writer is free to choose. A party's own
//! send/receive order is fixed by its protocol logic, not by buffering, so the
//! collapsed transcript is both deterministic and framing-independent.
//!
//! # Determinism
//!
//! Both peers are driven by `tokio::join!` on a single-threaded current-thread
//! runtime (see [`super::wire::block_on`]). Cooperative polling on one thread
//! makes the order in which the two `gossip` futures make progress fully
//! deterministic, so the capture is reproducible run to run — the property a
//! golden snapshot needs. (The synchronous path in [`super::sync_wire`] uses
//! real OS threads, whose interleaving is *not* reproducible, and so is
//! unsuitable for golden capture.)
//!
//! # Interposition
//!
//! Each end of an in-memory `tokio::io::duplex` pipe is wrapped in a
//! [`Recorder`] before being split into the read/write halves handed to
//! `gossip`. The wrapper logs every byte accepted by `poll_write` (a *send* by
//! that peer) and every byte delivered by `poll_read` (a *receive* by that
//! peer) into one shared, ordered [`Log`], tagged with the acting party. The
//! renderer then demultiplexes the log into the two per-party transcripts.

use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use borsh::{BorshDeserialize, BorshSerialize};
use rumors::Known;
use tokio::io::{AsyncRead, AsyncWrite, DuplexStream, ReadBuf};

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
struct Recorder {
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

/// Gossip `a` and `b` through an interposed, recording duplex pipe and render
/// the captured conversation as a stable, human-legible timeline suitable for
/// `insta::assert_snapshot!`.
///
/// Both peers are driven concurrently on a current-thread runtime, so the
/// returned string is deterministic for a given pair of `Known`s and a given
/// build of the protocol. The two `Known`s are expected to reconcile cleanly;
/// a gossip error panics the helper.
pub fn capture_gossip<T>(mut a: Known<T>, mut b: Known<T>) -> String
where
    T: BorshSerialize + BorshDeserialize + Send + Sync + 'static,
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
        let (mut a_r, mut a_w) = tokio::io::split(a_rec);
        let (mut b_r, mut b_w) = tokio::io::split(b_rec);

        let (a_result, b_result) =
            tokio::join!(a.gossip(&mut a_r, &mut a_w), b.gossip(&mut b_r, &mut b_w));
        a_result.expect("gossip A");
        b_result.expect("gossip B");
    });

    render(&log.0.lock().unwrap())
}

/// Render the event log as two per-party transcripts laid out side by side,
/// each collapsed by direction (see the module docs).
fn render(events: &[Event]) -> String {
    let left = transcript("A", events);
    let right = transcript("B", events);
    side_by_side(&left, &right)
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
