//! Measurements from a completed synchronization session.
//!
//! [`SessionStats`] reports latency, changes to the rumor set, network traffic,
//! and memory-budget pressure while joining, gossiping, or leaving a network.
//! It is returned on [`Gossiped`](crate::Gossiped) and sent to
//! [`SessionObserver::finished`](crate::observe::SessionObserver::finished).
//! Each peer measures its own side, and collection adds no protocol traffic.
//! [`Recorder`] is the internal counter shared by the reconciliation layers.

use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// Local measurements from one completed session while joining, gossiping, or
/// leaving a network.
///
/// The two peers can report different values because each measures its own
/// work and traffic. Field documentation identifies the boundaries needed to
/// compare the two sides or aggregate sessions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct SessionStats {
    /// Time from the start of the exchange until both peers confirm completion.
    ///
    /// This is the session latency measured with a monotonic clock. It includes
    /// transport waits and bookmark I/O, but excludes time spent waiting for a
    /// continuous gossip policy to initiate the session.
    pub elapsed: Duration,
    /// Portions of the rumor set this peer compared where both peers had
    /// messages but their contents differed.
    ///
    /// This indicates how much searching gossip required, separate from
    /// transferring messages held by only one peer. The peers alternate these
    /// comparisons, so add their values for the session total. An
    /// already-converged session and a bootstrap report zero.
    pub disputed_scopes: u64,
    /// Live messages learned from the peer.
    ///
    /// Together with [`messages_shed`](Self::messages_shed), this shows how the
    /// session changed the local rumor set. Modulo concurrent gossip sessions,
    /// its net size change is `messages_gained - messages_shed`.
    pub messages_gained: u64,
    /// Live messages removed because the peer had already seen and deleted
    /// them.
    ///
    /// Rumors communicates redaction without retaining tombstones. This count
    /// distinguishes deletions learned from the peer from messages gained
    /// during the same session.
    pub messages_shed: u64,
    /// Encoded bytes sent to compare and transfer messages.
    ///
    /// This measures the session's outgoing data traffic. It excludes session
    /// setup and completion, and the labels that identify independent streams.
    /// On a completed lossless link, one peer's `bytes_sent` equals the other's
    /// [`bytes_received`](Self::bytes_received).
    pub bytes_sent: u64,
    /// Complete protocol frames sent while comparing and transferring messages.
    ///
    /// This includes one end marker for each independent stream. Compare it
    /// with [`bytes_sent`](Self::bytes_sent) to inspect batching and average
    /// frame size under [`target_message_size`](crate::Peer::target_message_size).
    pub frames_sent: u64,
    /// Encoded bytes received to compare and transfer messages.
    ///
    /// This is the incoming counterpart to [`bytes_sent`](Self::bytes_sent),
    /// with the same exclusions and traffic interpretation.
    pub bytes_received: u64,
    /// Complete protocol frames received while comparing and transferring messages.
    ///
    /// This is the incoming counterpart to [`frames_sent`](Self::frames_sent),
    /// including independent-stream end markers.
    pub frames_received: u64,
    /// Largest comparison window granted during the session.
    ///
    /// Gossip compares the rumor set in several steps. Each step limits how
    /// many comparisons it may keep in flight while waiting for replies; that
    /// limit is its *window*. Rumors derives each window from
    /// [`sync_memory_budget`](crate::Peer::sync_memory_budget), and this field
    /// reports the largest.
    ///
    /// This is available capacity, not observed use. Larger windows can hide
    /// more network latency but allow more buffering. Compare
    /// [`window_stalls`](Self::window_stalls) to learn whether gossip reached a
    /// window limit. One means comparisons were fully serialized; zero means
    /// no comparison was needed.
    pub window_granted: u64,
    /// Times gossip reached its limit on comparisons in flight.
    ///
    /// Zero means the synchronization memory budget caused no observed
    /// backpressure. A nonzero value suggests that a larger budget might
    /// improve overlap. The count depends on scheduling, so treat it as a
    /// pressure signal rather than a reproducible workload total.
    pub window_stalls: u64,
}

/// The write side of one session's [`SessionStats`]: cheaply cloneable,
/// shared by the walk, the window solve, and the codec's byte counters.
///
/// Counters are atomics only to satisfy `Send + Sync` across the session's
/// concurrently polled tasks; all ordering is `Relaxed` because each
/// counter is a sum with no cross-counter invariant enforced mid-session.
/// The one read ([`snapshot`](Self::snapshot)) happens after the session's
/// tasks have completed.
#[derive(Debug, Clone, Default)]
pub struct Recorder {
    inner: Arc<Counters>,
}

#[derive(Debug, Default)]
struct Counters {
    disputed_scopes: AtomicU64,
    messages_gained: AtomicU64,
    messages_shed: AtomicU64,
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,
    frames_sent: AtomicU64,
    frames_received: AtomicU64,
    window_granted: AtomicU64,
    window_stalls: AtomicU64,
}

impl Recorder {
    /// Count one scope resolved with a disputed outcome; see
    /// [`SessionStats::disputed_scopes`] for the definition.
    pub fn disputed_scope(&self) {
        self.inner.disputed_scopes.fetch_add(1, Ordering::Relaxed);
    }

    /// Count `messages` live leaves absorbed from the peer's supplies.
    pub fn gained(&self, messages: u64) {
        self.inner
            .messages_gained
            .fetch_add(messages, Ordering::Relaxed);
    }

    /// Count `messages` live leaves dropped by the deletion-honoring filter.
    pub fn shed(&self, messages: u64) {
        self.inner
            .messages_shed
            .fetch_add(messages, Ordering::Relaxed);
    }

    /// Record the widest per-stage capacity the window solve granted.
    ///
    /// A session resolves its window exactly once, so this is a store,
    /// not an accumulation.
    pub fn window_granted(&self, scopes: u64) {
        self.inner.window_granted.store(scopes, Ordering::Relaxed);
    }

    /// Count one complete frame written by the data-stream codec.
    pub fn frame_sent(&self) {
        self.inner.frames_sent.fetch_add(1, Ordering::Relaxed);
    }

    /// Count one complete frame accepted by the data-stream codec.
    pub fn frame_received(&self) {
        self.inner.frames_received.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a send that encountered a full window-controlled queue.
    pub fn window_stall(&self) {
        self.inner.window_stalls.fetch_add(1, Ordering::Relaxed);
    }

    /// Read the counters into the session's [`SessionStats`].
    pub fn snapshot(&self) -> SessionStats {
        SessionStats {
            elapsed: Duration::ZERO,
            disputed_scopes: self.inner.disputed_scopes.load(Ordering::Relaxed),
            messages_gained: self.inner.messages_gained.load(Ordering::Relaxed),
            messages_shed: self.inner.messages_shed.load(Ordering::Relaxed),
            bytes_sent: self.inner.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.inner.bytes_received.load(Ordering::Relaxed),
            frames_sent: self.inner.frames_sent.load(Ordering::Relaxed),
            frames_received: self.inner.frames_received.load(Ordering::Relaxed),
            window_granted: self.inner.window_granted.load(Ordering::Relaxed),
            window_stalls: self.inner.window_stalls.load(Ordering::Relaxed),
        }
    }
}

/// A transport write half that adds every accepted byte to
/// [`SessionStats::bytes_sent`].
///
/// Wraps a stream's write half between the frame codec and the transport,
/// after the stream's label has been written on the raw half, so the count
/// is exactly the codec's frame bytes.
pub struct CountedWrite<W> {
    inner: W,
    recorder: Recorder,
}

impl<W> CountedWrite<W> {
    /// Wrap `inner`, crediting its accepted bytes to `recorder`.
    pub fn new(inner: W, recorder: Recorder) -> Self {
        Self { inner, recorder }
    }

    /// Recover the transport half; the counter holds no bytes.
    pub fn into_inner(self) -> W {
        self.inner
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for CountedWrite<W> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let this = &mut *self;
        let poll = Pin::new(&mut this.inner).poll_write(cx, buf);
        if let Poll::Ready(Ok(accepted)) = &poll {
            this.recorder
                .inner
                .bytes_sent
                .fetch_add(*accepted as u64, Ordering::Relaxed);
        }
        poll
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// A transport read half that adds every delivered byte to
/// [`SessionStats::bytes_received`].
///
/// Wraps a stream's read half between the transport and the frame decoder,
/// after the accept driver has consumed the stream's label, so the count is
/// exactly the codec's frame bytes.
pub struct CountedRead<R> {
    inner: R,
    recorder: Recorder,
}

impl<R> CountedRead<R> {
    /// Wrap `inner`, crediting its delivered bytes to `recorder`.
    pub fn new(inner: R, recorder: Recorder) -> Self {
        Self { inner, recorder }
    }

    /// Recover the transport half; the counter holds no bytes.
    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for CountedRead<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = &mut *self;
        let before = buf.filled().len();
        let poll = Pin::new(&mut this.inner).poll_read(cx, buf);
        if let Poll::Ready(Ok(())) = &poll {
            let delivered = buf.filled().len() - before;
            this.recorder
                .inner
                .bytes_received
                .fetch_add(delivered as u64, Ordering::Relaxed);
        }
        poll
    }
}
