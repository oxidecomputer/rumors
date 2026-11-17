//! Measurements from a completed synchronization session.
//!
//! [`SessionStats`] reports latency, changes to the rumor set, and the traffic
//! of the session's data frames while joining, gossiping, or leaving a network;
//! the items before and after those frames are not counted. It is returned on
//! [`Gossiped`](crate::Gossiped) and sent to
//! [`SessionObserver::finished`](crate::observe::SessionObserver::finished).
//! Each peer measures its own side, and collection adds no protocol traffic.
//! [`Recorder`] is the internal counter shared by the reconciliation layers.

use std::io::IoSlice;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use crate::counter::widen;

/// Local metrics from one completed gossip round.
///
/// The two peers may report different values because each measures its own work
/// and traffic.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct SessionStats {
    /// Time from the start to the end of the round of gossip.
    ///
    /// This does not include time spent waiting for the next round of gossip.
    pub elapsed: Duration,
    /// Portions of the message set this peer compared where both peers had
    /// messages but their contents differed.
    ///
    /// This can serve as a proxy for how much work was required to isolate
    /// divergences, separately from the work involved in transferring messages
    /// held by only one peer. Naturally, an already-converged session and a
    /// joining session both will report zero, since neither requires this
    /// kind of work.
    pub disputed_scopes: u64,
    /// New messages learned from the peer.
    ///
    /// Together with [`messages_shed`](Self::messages_shed), this shows how the
    /// round of gossip altered the local message set. Modulo concurrent gossip
    /// sessions, the net size change effected by this round of gossip is
    /// `messages_gained - messages_shed`.
    pub messages_gained: u64,
    /// Existing messages removed because the peer had already seen and redacted
    /// them.
    ///
    /// Together with [`messages_gained`](Self::messages_gained), this shows how
    /// the round of gossip altered the local message set. Modulo concurrent
    /// gossip sessions, the net size change effected by this round of gossip is
    /// `messages_gained - messages_shed`.
    pub messages_shed: u64,
    /// Bytes sent to compare and transfer messages *specifically*.
    ///
    /// This measures the session's outgoing data. It excludes fixed transfer
    /// associated with session establishment and completion, and cannot measure
    /// transport overhead. For a completed session, one peer's
    /// [`bytes_sent`](Self::bytes_sent) equals the other's
    /// [`bytes_received`](Self::bytes_received).
    pub bytes_sent: u64,
    /// Complete protocol frames sent while comparing and transferring messages
    /// *specifically*.
    ///
    /// Compare it with [`bytes_sent`](Self::bytes_sent) to observe batching and
    /// average frame size under
    /// [`target_message_size`](crate::Peer::target_message_size).
    pub frames_sent: u64,
    /// Bytes received to compare and transfer messages *specifically*.
    ///
    /// This measures the session's incoming data. It excludes fixed transfer
    /// associated with session establishment and completion, and cannot measure
    /// transport overhead. For a completed session, one peer's
    /// [`bytes_received`](Self::bytes_received) equals the other's
    /// [`bytes_sent`](Self::bytes_sent).
    pub bytes_received: u64,
    /// Complete protocol frames received while comparing and transferring
    /// messages *specifically*.
    ///
    /// Compare it with [`bytes_received`](Self::bytes_received) to observe
    /// batching and average frame size under
    /// [`target_message_size`](crate::Peer::target_message_size).
    pub frames_received: u64,
    /// Number of times this peer applied internal backpressure until its
    /// [memory budget](crate::Peer::sync_memory_budget) had room to continue.
    ///
    /// This measure shows whether (and to what extent) the configured budget
    /// limited pipelining during the round of gossip. At zero, the session kept
    /// in flight everything it was ready to send, behaving identically to a
    /// session with unlimited budget. The greater this value is, the more the
    /// pipeline stalled to prevent exceeding the memory budget.
    ///
    /// It is not necessarily the case that a round of gossip with a lot of
    /// budget waits was *actually much slower* than it would have been with a
    /// higher memory budget. The speed of a session slowly decreases as the
    /// budget constricts it, so a session with a medium number of scattered
    /// waits can "bury" the latency in the rest of its work. However, if you
    /// slower synchronization than you hope for, a large number of budget waits
    /// can indicate that your configured memory budget is imposing excessive
    /// latency.
    pub budget_waits: u64,
}

/// The write side of one session's [`SessionStats`]: cheaply cloneable, shared
/// by the walk and the codec's byte counters.
///
/// Counters are atomics only to satisfy `Send + Sync` across the session's
/// concurrently polled tasks; all ordering is `Relaxed` because each counter is
/// a sum with no cross-counter invariant enforced mid-session. The one read
/// ([`snapshot`](Self::snapshot)) happens after the session's tasks have
/// completed.
#[derive(Debug, Clone, Default)]
pub struct Recorder {
    /// The counters every clone of this recorder adds to.
    inner: Arc<Counters>,
}

/// One session's running counts, one per counted field of [`SessionStats`],
/// which [`Recorder::snapshot`] copies out.
#[derive(Debug, Default)]
struct Counters {
    /// The count behind [`SessionStats::disputed_scopes`].
    disputed_scopes: AtomicU64,
    /// The count behind [`SessionStats::messages_gained`].
    messages_gained: AtomicU64,
    /// The count behind [`SessionStats::messages_shed`].
    messages_shed: AtomicU64,
    /// The count behind [`SessionStats::bytes_sent`].
    bytes_sent: AtomicU64,
    /// The count behind [`SessionStats::bytes_received`].
    bytes_received: AtomicU64,
    /// The count behind [`SessionStats::frames_sent`].
    frames_sent: AtomicU64,
    /// The count behind [`SessionStats::frames_received`].
    frames_received: AtomicU64,
    /// The count behind [`SessionStats::budget_waits`].
    budget_waits: AtomicU64,
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

    /// Count one complete frame written by the data-stream codec.
    pub fn frame_sent(&self) {
        self.inner.frames_sent.fetch_add(1, Ordering::Relaxed);
    }

    /// Count one complete frame accepted by the data-stream codec.
    pub fn frame_received(&self) {
        self.inner.frames_received.fetch_add(1, Ordering::Relaxed);
    }

    /// Count `units` the walk's ledger held back for budget; see
    /// [`SessionStats::budget_waits`] for the definition.
    pub fn held_back(&self, units: u64) {
        self.inner.budget_waits.fetch_add(units, Ordering::Relaxed);
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
            budget_waits: self.inner.budget_waits.load(Ordering::Relaxed),
        }
    }
}

/// A transport write half that adds every accepted byte to
/// [`SessionStats::bytes_sent`].
///
/// Wraps the pipe's write half between the frame codec and the transport, and
/// only while the multiplexer writes frames, so the count is exactly the
/// codec's frame bytes. Vectored writes pass through to the transport, so a
/// frame too large for the multiplexer's buffer stays one write operation where
/// the transport supports it.
pub struct CountedWrite<W> {
    inner: W,
    recorder: Recorder,
}

impl<W> CountedWrite<W> {
    /// Wrap `inner`, crediting its accepted bytes to `recorder`.
    pub fn new(inner: W, recorder: Recorder) -> Self {
        Self { inner, recorder }
    }

    /// Credit the bytes a write accepted, passing its outcome through.
    fn credit(&self, poll: Poll<std::io::Result<usize>>) -> Poll<std::io::Result<usize>> {
        if let Poll::Ready(Ok(accepted)) = &poll {
            self.recorder
                .inner
                .bytes_sent
                .fetch_add(widen(*accepted), Ordering::Relaxed);
        }
        poll
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for CountedWrite<W> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let poll = Pin::new(&mut self.inner).poll_write(cx, buf);
        self.credit(poll)
    }

    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<std::io::Result<usize>> {
        let poll = Pin::new(&mut self.inner).poll_write_vectored(cx, bufs);
        self.credit(poll)
    }

    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored()
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
/// Wraps the pipe's read half between the transport and the frame decoder, and
/// only while the demultiplexer reads frames, so the count is exactly the
/// codec's frame bytes.
pub struct CountedRead<R> {
    inner: R,
    recorder: Recorder,
}

impl<R> CountedRead<R> {
    /// Wrap `inner`, crediting its delivered bytes to `recorder`.
    pub fn new(inner: R, recorder: Recorder) -> Self {
        Self { inner, recorder }
    }

    /// Borrow the transport half; bytes read through the borrow are not
    /// counted.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
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
                .fetch_add(widen(delivered), Ordering::Relaxed);
        }
        poll
    }
}
