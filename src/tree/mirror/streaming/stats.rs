//! Per-session measurement: the counters one reconciliation session keeps
//! about itself, and the recorder the session's layers write them through.
//!
//! The public face is [`SessionStats`], the datum a completed session hands
//! back on [`Gossiped`](crate::Gossiped). Each field is a count taken at one
//! named seam of the session machinery; the field docs say which mechanism
//! produces the number and where it is counted, so a reader can hold the
//! number against the code that made it. The [`Recorder`] is the crate's
//! internal write side: one per session, shared by the materialized walk
//! (dispute, gain, and shed counts), the window solve (the granted width),
//! and the wire codec (byte counts), and snapshotted into a `SessionStats`
//! when the session commits.
//!
//! Nothing here touches the wire: every counter is a local observation of
//! work the session was already doing, so two ends of one session can
//! disagree (and for [`disputed_scopes`](SessionStats::disputed_scopes)
//! they systematically do; the field explains why).

use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// What one completed gossip session measured about itself.
///
/// Carried by [`Gossiped`](crate::Gossiped), one per successful session.
/// Every count is taken locally, at the seam named in its field docs, while
/// the session runs; nothing is exchanged to produce it, so the feature
/// costs no wire bytes and cannot change the protocol.
///
/// Two deliberate boundaries:
///
/// - **No duration field.** The caller owns the clock: wrap the `gossip`
///   call (or the `gossip_when` stream's polls) in whatever timing
///   instrument the application already uses. A duration measured inside
///   the crate would bake in one notion of time and satisfy nobody's.
/// - **[`Protocol::V1`](crate::Protocol::V1) sessions report zero in every
///   field.** The counters live in the streaming session's machinery; the
///   V1 implementation computes none of them, and each field states this.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct SessionStats {
    /// Scopes this side resolved as genuine disputes: questions it answered
    /// where both replicas held the subtree and their contents differed.
    ///
    /// The mechanism: reconciliation walks the content-addressed tree from
    /// the root, and at each *scope* (one prefix and whatever both sides
    /// hold under it) the side holding the scope answers the other's
    /// question by merge-joining the two child listings. This counter
    /// increments once per merge-join in which both listings were
    /// non-empty and at least one child failed to match: the definition
    /// of a disputed scope. A question about a subtree the answerer does
    /// not hold is a *request* (the content is simply supplied), and a
    /// merge-join where every child hash agrees is a confirmation; neither
    /// is counted. The count is taken at the walk's answering chokepoint
    /// (`answer::internal` and `answer::leaf_parent` in the materialized
    /// walk), which runs exactly once per scope this side resolves.
    ///
    /// The two ends of one session report *different* values here, by
    /// construction, and their sum is the session's total disputed scopes.
    /// The descent alternates: the initiator asks about the root, the
    /// responder answers it and asks about the disputed children one level
    /// down, the initiator answers those, and so on. Each side therefore
    /// resolves the disputes at every second level of the tree (the
    /// responder at even depths from the root, the initiator at odd ones),
    /// so neither side's count alone is the session total.
    ///
    /// Zero when the greeting versions were equal (the session ends before
    /// any descent), when either replica held nothing under every shared
    /// prefix (a bootstrap catch-up: supplies are not disputes), and for
    /// every [`Protocol::V1`](crate::Protocol::V1) session (the V1
    /// implementation does not compute this).
    pub disputed_scopes: u64,
    /// Live messages this replica learned from the peer during the session.
    ///
    /// The mechanism: every message this side learns arrives as a *supply*,
    /// a subtree or leaf the peer holds and this side lacks, shipped after
    /// the peer filtered it against this side's causal version. This
    /// counter adds each absorbed supply's exact live-leaf count at the
    /// moment the walk splices it into the reconciled tree (the reply
    /// resolver's supply arm, the initiator's terminal leaf absorption,
    /// and the opening's early supplies). With no writes committed
    /// concurrently with the session, the replica's live count moves by
    /// exactly `messages_gained - messages_shed`.
    ///
    /// Zero for every [`Protocol::V1`](crate::Protocol::V1) session (the
    /// V1 implementation does not compute this).
    pub messages_gained: u64,
    /// Live messages this replica dropped during the session because the
    /// peer had provably seen and deleted them: deletions honored.
    ///
    /// The mechanism: redaction leaves no tombstones, so deletion travels
    /// by inference. When this side holds a subtree the peer entirely
    /// lacks, it filters that subtree leaf by leaf against the peer's
    /// greeting version before supplying it. A leaf causally at or before
    /// that version was necessarily seen by the peer, and the peer no
    /// longer holds it, so it was deleted there; this side drops its own
    /// copy instead of transmitting it. This counter adds one per leaf the
    /// filter drops, counted where the filter renders its verdict (the
    /// streaming deletion-honoring filter and the leaf-level checks in the
    /// walk's answering paths). The filter prunes whole subtrees at a time
    /// when the cached version bounds already decide them; a pruned
    /// subtree adds its exact live-leaf count, so the number always reads
    /// in messages.
    ///
    /// Zero for every [`Protocol::V1`](crate::Protocol::V1) session (the
    /// V1 implementation does not compute this).
    pub messages_shed: u64,
    /// Bytes of reconciliation frames this side's frame codec wrote to the
    /// link's streams during the session.
    ///
    /// The mechanism: every question, confirmation, and supply of the
    /// descent crosses the wire as a frame of the streaming codec, and
    /// this counter is taken exactly at that seam, between the codec and
    /// the transport stream it writes: the same boundary the
    /// [`target_message_size`](crate::Peer::target_message_size) budget
    /// prices frames at. It therefore counts every reconciliation byte
    /// and nothing else: the session's fixed envelope (the transport
    /// preamble, the greeting, per-stream labels, any identity hand-off,
    /// and the epilogue marker) rides the control stream through a
    /// different framing layer and is excluded. Over a lossless link, one
    /// side's `bytes_sent` equals the other's
    /// [`bytes_received`](Self::bytes_received).
    ///
    /// Zero when the greeting versions were equal (no data stream ever
    /// opens), and for every [`Protocol::V1`](crate::Protocol::V1) session
    /// (V1 frames ride a different codec, which does not count).
    pub bytes_sent: u64,
    /// Bytes of reconciliation frames this side's frame codec read from the
    /// link's streams during the session.
    ///
    /// Counted at the same codec seam as [`bytes_sent`](Self::bytes_sent),
    /// on the read side: every byte the frame decoder consumed from the
    /// session's streams, and nothing else (the fixed session envelope is
    /// excluded there too). Bytes of a frame count when the decoder reads
    /// them, so a session that fails mid-frame has counted the prefix it
    /// consumed; on `Ok` every counted frame was a complete one.
    ///
    /// Zero when the greeting versions were equal, and for every
    /// [`Protocol::V1`](crate::Protocol::V1) session (V1 frames ride a
    /// different codec, which does not count).
    pub bytes_received: u64,
    /// The widest per-stage pipeline width the session's window solve
    /// granted, in disputed scopes.
    ///
    /// The mechanism: at greeting time the session turns the
    /// [`sync_memory_budget`](crate::Peer::sync_memory_budget) into static
    /// per-height channel capacities, clamping each height by both the
    /// budget and that depth's population envelope (deep, sparse stages
    /// get small bounds no budget can widen, because their populations
    /// cannot exist). This field is the maximum of those granted
    /// capacities: how many disputed scopes the most generously
    /// provisioned stage was allowed to hold in flight at once. It is a
    /// summary, not the whole vector; two sessions with equal values here
    /// can still differ at other heights. One means the session ran at
    /// the serialization floor: every stage one scope at a time, one wire
    /// round trip per disputed scope.
    ///
    /// Zero when the greeting versions were equal (the session ends
    /// before any window is derived), and for every
    /// [`Protocol::V1`](crate::Protocol::V1) session (V1 has no window).
    pub window_granted: u64,
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
    window_granted: AtomicU64,
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

    /// Read the counters into the session's [`SessionStats`].
    pub fn snapshot(&self) -> SessionStats {
        SessionStats {
            disputed_scopes: self.inner.disputed_scopes.load(Ordering::Relaxed),
            messages_gained: self.inner.messages_gained.load(Ordering::Relaxed),
            messages_shed: self.inner.messages_shed.load(Ordering::Relaxed),
            bytes_sent: self.inner.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.inner.bytes_received.load(Ordering::Relaxed),
            window_granted: self.inner.window_granted.load(Ordering::Relaxed),
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
