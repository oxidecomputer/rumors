//! How many calls a session makes on the transport halves it is given, on
//! fixed sessions.
//!
//! A session's read half and write half are whatever the application
//! supplies: a TCP socket, a TLS stream, a QUIC stream. Each call the
//! session makes on them that completes costs that transport something
//! whatever it is: a system call per write or read on a raw socket, a
//! record per write on an unbuffered TLS stream. This suite counts the
//! completed calls at that boundary (`poll_read`, `poll_fill_buf`,
//! `consume`, `poll_write`, `poll_write_vectored`, `poll_flush`) on
//! deterministic sessions, so a change to how a session writes or reads
//! moves a committed number. A session reads through a buffer it is
//! given (`AsyncBufRead`), whose calls cost nothing beyond a copy, so the
//! suite also counts the completed reads beneath that buffer, from the
//! transport itself: those are what a raw socket pays a system call for.
//!
//! A call completes when it returns `Ready`. A call that returns `Pending`
//! waits for the peer; it is not counted, because how often a session
//! waits depends on how its tasks interleave rather than on how it writes
//! or reads. A wait usually costs a Tokio socket no system call, since the
//! socket answers it from the readiness state its event loop keeps; after
//! a read that exactly filled the caller's buffer, it costs one `read`
//! that finds nothing (`EAGAIN`). `consume` always completes.
//!
//! Each session runs both peers in their own tasks on a current-thread
//! runtime, over an in-memory connection whose directions buffer more than
//! any fixture sends, so no write or flush waits for room; reads wait
//! whenever the peer has sent nothing new. The write halves come in two
//! kinds: one that
//! takes a vectored write whole, as a TCP socket, a TLS stream, or a
//! `BufWriter` does, and one that, like the trait's default, takes only a
//! vectored write's first slice.

use std::io::IoSlice;
use std::ops::{Bound, RangeBounds};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use rumors::default::tls::WRITE_BATCH_RANGE;
use rumors::observe::{
    Direction, Observer, SessionInfo, SessionObserver, StreamInfo, StreamObserver,
};
use rumors::testing::{CUTS_TO_GROW, MemoryEnd, memory_connection};
use rumors::{Rumors, default};
use rumors_testkit::bench::session::{Shape, pair};
use tokio::io::{AsyncBufRead, AsyncRead, AsyncWrite, BufReader, ReadBuf};

/// Bytes each direction of the connection buffers: more than any fixture
/// sends in one direction, so no write waits for its reader.
const CAPACITY: usize = 4 << 20;

/// The calls one session made on its transport halves, summed over both
/// sides, beside the frames both sides' statistics count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Calls {
    /// Completed `poll_read` calls on the transport beneath the read half's
    /// buffer.
    transport_reads: u64,
    /// Completed `poll_read` calls on the read half.
    reads: u64,
    /// Completed `poll_fill_buf` calls.
    fills: u64,
    /// `consume` calls.
    consumes: u64,
    /// Completed `poll_write` calls.
    writes: u64,
    /// Completed `poll_write_vectored` calls the write half took whole.
    vectored_writes: u64,
    /// Completed `poll_flush` calls.
    flushes: u64,
    /// Bytes the write halves accepted.
    written_bytes: u64,
    /// Data frames sent, by the sessions' own statistics.
    frames: u64,
}

/// The shared counters one side's two halves add to.
#[derive(Debug, Default)]
struct Counters {
    /// Completed `poll_read` calls.
    reads: AtomicU64,
    /// Completed `poll_fill_buf` calls.
    fills: AtomicU64,
    /// `consume` calls.
    consumes: AtomicU64,
    /// Completed `poll_write` calls.
    writes: AtomicU64,
    /// Completed `poll_write_vectored` calls passed through whole.
    vectored_writes: AtomicU64,
    /// Completed `poll_flush` calls.
    flushes: AtomicU64,
    /// Bytes accepted by writes of either kind.
    written_bytes: AtomicU64,
    /// Writes and flushes that returned `Pending`.
    waits: AtomicU64,
    /// The bytes written before each completed flush, in order.
    flush_marks: Mutex<Vec<u64>>,
}

/// A transport half that counts the calls a session makes on it that
/// complete.
struct Counted<T> {
    /// The in-memory half beneath.
    inner: T,
    /// Where the calls are counted.
    counters: Arc<Counters>,
    /// Whether this half takes vectored writes whole; if not, it takes a
    /// vectored write's first nonempty slice as one `poll_write`, as the
    /// trait's default does.
    vectored: bool,
}

/// Add one to `count` if `poll` completed, passing it through.
fn completed<T>(count: &AtomicU64, poll: Poll<T>) -> Poll<T> {
    if poll.is_ready() {
        count.fetch_add(1, Ordering::Relaxed);
    }
    poll
}

/// Add one to `waits` if `poll` waited, passing it through.
fn waited<T>(waits: &AtomicU64, poll: Poll<T>) -> Poll<T> {
    if poll.is_pending() {
        waits.fetch_add(1, Ordering::Relaxed);
    }
    poll
}

/// Count each completed read, passing it through.
impl<T: AsyncRead + Unpin> AsyncRead for Counted<T> {
    /// Reads through, counting the call if it completes.
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let poll = Pin::new(&mut self.inner).poll_read(cx, buf);
        completed(&self.counters.reads, poll)
    }
}

/// Count each completed look into the buffer and each consumption,
/// passing them through.
impl<T: AsyncBufRead + Unpin> AsyncBufRead for Counted<T> {
    /// Fills through, counting the call if it completes.
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<&[u8]>> {
        let this = self.get_mut();
        let poll = Pin::new(&mut this.inner).poll_fill_buf(cx);
        completed(&this.counters.fills, poll)
    }

    /// Counts the call, then consumes through.
    fn consume(mut self: Pin<&mut Self>, amount: usize) {
        self.counters.consumes.fetch_add(1, Ordering::Relaxed);
        Pin::new(&mut self.inner).consume(amount);
    }
}

impl<T> Counted<T> {
    /// Count the bytes a write accepted, passing its outcome through.
    fn accepted(&self, poll: Poll<std::io::Result<usize>>) -> Poll<std::io::Result<usize>> {
        if let Poll::Ready(Ok(accepted)) = &poll {
            self.counters
                .written_bytes
                .fetch_add(*accepted as u64, Ordering::Relaxed);
        }
        poll
    }
}

/// Count each completed write and flush, passing it through.
impl<T: AsyncWrite + Unpin> AsyncWrite for Counted<T> {
    /// Writes through, counting the call if it completes.
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let poll = Pin::new(&mut self.inner).poll_write(cx, buf);
        let poll = waited(&self.counters.waits, poll);
        let poll = completed(&self.counters.writes, poll);
        self.accepted(poll)
    }

    /// Passes the slices through whole on a vectored half, counting the
    /// call if it completes; otherwise writes the first nonempty slice as a
    /// `poll_write`.
    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<std::io::Result<usize>> {
        if !self.vectored {
            let first = bufs
                .iter()
                .find(|buf| !buf.is_empty())
                .map_or(&[][..], |buf| &**buf);
            return self.poll_write(cx, first);
        }
        let poll = Pin::new(&mut self.inner).poll_write_vectored(cx, bufs);
        let poll = waited(&self.counters.waits, poll);
        let poll = completed(&self.counters.vectored_writes, poll);
        self.accepted(poll)
    }

    /// Whether this half takes vectored writes whole.
    fn is_write_vectored(&self) -> bool {
        self.vectored
    }

    /// Flushes through, counting the call and marking how many bytes it
    /// follows if it completes.
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let poll = Pin::new(&mut self.inner).poll_flush(cx);
        let poll = waited(&self.counters.waits, poll);
        if poll.is_ready() {
            let written = self.counters.written_bytes.load(Ordering::Relaxed);
            self.counters
                .flush_marks
                .lock()
                .expect("flush marks lock")
                .push(written);
        }
        completed(&self.counters.flushes, poll)
    }

    /// Shuts down through, uncounted: a session never shuts its write half.
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// What one side wrote, item by item and flush by flush.
#[derive(Debug, Default)]
struct Trace {
    /// The length of each item the side wrote, control items and frames
    /// alike, in the order it wrote them.
    items: Vec<usize>,
    /// The bytes each flush carried: those written since the flush before
    /// it, omitting flushes that carried nothing.
    flushes: Vec<usize>,
    /// The writes and flushes on the side's write half that waited.
    waits: u64,
}

/// An observer that records the length of every item its sessions send,
/// in the order they are written.
///
/// Every sent stream's handler runs in the task that writes that stream's
/// items, the control items before and after the frames and every frame
/// in the multiplexer's order, so one shared list records the write order.
#[derive(Clone, Default)]
struct ItemLog(Arc<Mutex<Vec<usize>>>);

/// Record every session's sent items.
impl Observer for ItemLog {
    fn session(&self, _: &SessionInfo) -> Option<Box<dyn SessionObserver>> {
        Some(Box::new(self.clone()))
    }
}

/// Record every sent stream's items, and no received stream's.
impl SessionObserver for ItemLog {
    fn stream(&self, stream: &StreamInfo) -> Option<Box<dyn StreamObserver>> {
        (stream.direction == Direction::Sent).then(|| Box::new(self.clone()) as _)
    }
}

/// Append each item's length.
impl StreamObserver for ItemLog {
    fn message(&mut self, bytes: &[u8]) {
        self.0.lock().expect("item log lock").push(bytes.len());
    }
}

/// Gossip once over `end` through counting halves, with the write batch
/// range `batch`, returning the calls, the frames this side sent, and, if
/// `traced`, what it wrote item by item and flush by flush.
///
/// The read half's buffer is rebuilt over a counted transport, as
/// `memory_connection` builds it (a `BufReader` of Tokio's default
/// capacity), so the reads it makes of the transport are counted apart
/// from the session's calls on it. Only a traced session is observed.
async fn gossip(
    peer: Rumors<String>,
    end: MemoryEnd,
    vectored: bool,
    batch: (Bound<usize>, Bound<usize>),
    traced: bool,
) -> (Calls, Trace) {
    let mut peer = peer
        .try_into_peer()
        .await
        .expect("a fixture peer has one handle")
        .write_batch_range(batch);
    let items = ItemLog::default();
    if traced {
        peer = peer.observe(Arc::new(items.clone()));
    }
    let peer = peer.into_rumors();
    let counters = Arc::new(Counters::default());
    let beneath = Arc::new(Counters::default());
    let transport = Counted {
        inner: end.read.into_inner(),
        counters: Arc::clone(&beneath),
        vectored,
    };
    let read = Counted {
        inner: BufReader::new(transport),
        counters: Arc::clone(&counters),
        vectored,
    };
    let write = Counted {
        inner: end.write,
        counters: Arc::clone(&counters),
        vectored,
    };
    let (gossiped, ..) = peer
        .gossip_once(read, write)
        .await
        .expect("an in-memory session completes");
    let load = |count: &AtomicU64| count.load(Ordering::Relaxed);
    let calls = Calls {
        transport_reads: load(&beneath.reads),
        reads: load(&counters.reads),
        fills: load(&counters.fills),
        consumes: load(&counters.consumes),
        writes: load(&counters.writes),
        vectored_writes: load(&counters.vectored_writes),
        flushes: load(&counters.flushes),
        written_bytes: load(&counters.written_bytes),
        frames: gossiped.stats.frames_sent,
    };
    let mut previous = 0;
    let mut flushes = Vec::new();
    for &mark in counters
        .flush_marks
        .lock()
        .expect("flush marks lock")
        .iter()
    {
        if mark > previous {
            flushes.push(usize::try_from(mark - previous).expect("a flush fits in memory"));
        }
        previous = mark;
    }
    let items = items.0.lock().expect("item log lock").clone();
    let waits = load(&counters.waits);
    (
        calls,
        Trace {
            items,
            flushes,
            waits,
        },
    )
}

/// Reconcile the pair `shape` describes once at the write batch range
/// `batch`, and return the calls both sides made and, if `traced`, what
/// each side wrote.
///
/// Each peer runs in its own task, over write halves that do or do not
/// take vectored writes whole.
///
/// Building the pair runs the peers' own joining sessions, before counting
/// starts.
fn session_traced(
    shape: Shape,
    vectored: bool,
    batch: impl RangeBounds<usize>,
    traced: bool,
) -> (Calls, [Trace; 2]) {
    let batch = (batch.start_bound().cloned(), batch.end_bound().cloned());
    let (a, b) = pair(shape, default::SYNC_MEMORY_BUDGET);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("build a current-thread runtime");
    let (a_end, b_end) = memory_connection(CAPACITY);
    let ((a, a_trace), (b, b_trace)) = runtime.block_on(async {
        let a = tokio::spawn(gossip(a, a_end, vectored, batch, traced));
        let b = tokio::spawn(gossip(b, b_end, vectored, batch, traced));
        (
            a.await.expect("peer A's task completes"),
            b.await.expect("peer B's task completes"),
        )
    });
    let calls = Calls {
        transport_reads: a.transport_reads + b.transport_reads,
        reads: a.reads + b.reads,
        fills: a.fills + b.fills,
        consumes: a.consumes + b.consumes,
        writes: a.writes + b.writes,
        vectored_writes: a.vectored_writes + b.vectored_writes,
        flushes: a.flushes + b.flushes,
        written_bytes: a.written_bytes + b.written_bytes,
        frames: a.frames + b.frames,
    };
    (calls, [a_trace, b_trace])
}

/// Reconcile the pair `shape` describes once, untraced, at the write batch
/// range `batch`, and return the calls both sides made.
fn session_at(shape: Shape, vectored: bool, batch: impl RangeBounds<usize>) -> Calls {
    session_traced(shape, vectored, batch, false).0
}

/// Reconcile the pair `shape` describes once, untraced, at the default
/// write batch range, and return the calls both sides made.
fn session(shape: Shape, vectored: bool) -> Calls {
    session_at(shape, vectored, WRITE_BATCH_RANGE)
}

/// The fixed sessions, each a name and its pair: identical replicas, a
/// hundred differences beside a shared set, a catch-up, and two disjoint
/// sets.
fn fixtures() -> [(&'static str, Shape); 4] {
    let shared = 10_000;
    [
        ("identical", Shape::overlapping(shared, 100)),
        (
            "hundred differences",
            Shape {
                shared,
                a: 50,
                b: 50,
                ..Shape::catch_up(0)
            },
        ),
        ("catch-up", Shape::catch_up(10_000)),
        ("disjoint", disjoint()),
    ]
}

/// The fixture with the most frames, nearly all of them small: two
/// disjoint sets of 5,000 messages, which the sides exchange in full.
fn disjoint() -> Shape {
    Shape::symmetric(5_000)
}

/// The control items each side writes outside the frames, each with its
/// own flush: its preamble, its greeting, and its completion marker.
const CONTROL_ITEMS: u64 = 3;

/// The sides of a session.
const SIDES: u64 = 2;

/// The calls follow from how a session writes, whatever the baseline.
///
/// A session writes each control item with a plain write of its own,
/// followed by a flush. It copies its frames into its write buffer, of
/// about 63 KiB, and writes the buffer out as one plain write at each
/// flush, and before any frame that does not fit beside the buffered ones;
/// a frame at least the buffer's size goes to the write half directly, as
/// one vectored write where the half takes them whole. It flushes only
/// after writing an item, and each flush writes out what the buffer holds.
/// No fixture here sends a frame near the buffer's size, so on both kinds
/// of write half every write is a plain one: no vectored write reaches
/// either half, and the two kinds see the same calls. Those equalities
/// rest on the fixtures as well as on how the session writes, and fail
/// once a fixture sends a frame that large. Every flush writes the buffer
/// out, and the buffer never writes itself out between flushes: the
/// multiplexer flushes before a frame that would push its batch past the
/// batch's ceiling, which under the default write batch range is at most
/// its largest batch, 16 KiB, below the buffer's capacity. So the plain
/// writes equal the flushes. Every write carries at least one item, so
/// the writes number at most the items, and at least the control items. A
/// session that sends frames flushes them, so the counters are counting.
///
/// The read half's buffer reads the transport only within one of the
/// session's calls on it, so the transport reads number at most the
/// session's reads and fills, and at least one: every session reads its
/// peer's preamble.
#[test]
fn calls_follow_from_how_a_session_writes() {
    let controls = CONTROL_ITEMS * SIDES;
    for (name, shape) in fixtures() {
        let whole = session(shape, true);
        let sliced = session(shape, false);
        for calls in [whole, sliced] {
            assert_eq!(calls.writes, calls.flushes, "{name}: {calls:?}");
            assert_eq!(calls.vectored_writes, 0, "{name}: {calls:?}");
            assert!(calls.writes <= calls.frames + controls, "{name}: {calls:?}");
            assert!(calls.writes >= controls, "{name}: {calls:?}");
            assert!(calls.transport_reads >= 1, "{name}: {calls:?}");
            assert!(
                calls.transport_reads <= calls.reads + calls.fills,
                "{name}: {calls:?}"
            );
        }
        assert_eq!(sliced, whole, "{name}");
        if whole.frames > 0 {
            assert!(whole.flushes > controls, "{name}: {whole:?}");
        }
    }
}

/// The fixed sessions' calls equal the committed baseline.
///
/// The multiplexer flushes once a poll finds no frame ready after it has
/// yielded to the session's other work, and before a frame that would
/// push its batch past the batch's ceiling under the default write batch
/// range: the segment size, about 1.4 KB, for a batch of frames that each
/// fit it, and the largest batch, 16 KiB, for a batch that starts with a
/// larger frame. So the frames the session readies over consecutive
/// passes of its task set share a flush, and in the catch-up, whose supply
/// runs average about 4.4 KB, consecutive runs share flushes of up to
/// 16 KiB. Its buffer reaches the transport as one plain write per flush,
/// whichever kind of write half it is. The demultiplexer looks at each
/// item's first byte in the read half's buffer (`poll_fill_buf`), without
/// consuming it, and the session then reads each item's parts with calls
/// of their own, a few per frame received, which consume what they copy:
/// it never calls `consume` itself. Identical replicas exchange only the
/// control items, which their readers read without looking first.
///
/// Beneath the read half's buffer, of Tokio's default capacity, 8 KiB, the
/// transport is read when a call finds the buffer empty: to refill it, or
/// directly for a read of at least its capacity. A read that finds fewer
/// bytes buffered than it asks for takes only those, so an item that
/// straddles the end of a refill costs the session a second read. In the
/// catch-up, whose supply runs average about 4.4 KB, about one run per
/// refill straddles one.
///
/// The counts are deterministic: the fixtures' networks are seeded, the
/// runtime polls its tasks in a fixed order, no write waits, and nothing in
/// a session reads a clock. They move when how a session writes, reads, or
/// schedules its work changes, or a fixture does, and a commit that moves
/// them says why.
#[test]
fn calls_match_the_baseline() {
    let calls =
        |transport_reads, reads, fills, writes, vectored_writes, flushes, written_bytes, frames| {
            Calls {
                transport_reads,
                reads,
                fills,
                consumes: 0,
                writes,
                vectored_writes,
                flushes,
                written_bytes,
                frames,
            }
        };
    let baseline = [
        (
            calls(4, 14, 0, 6, 0, 6, 11_536, 0),
            calls(4, 14, 0, 6, 0, 6, 11_536, 0),
        ),
        (
            calls(27, 1_434, 336, 26, 0, 26, 86_058, 335),
            calls(27, 1_434, 336, 26, 0, 26, 86_058, 335),
        ),
        (
            calls(144, 1_688, 258, 88, 0, 88, 1_129_236, 257),
            calls(144, 1_688, 258, 88, 0, 88, 1_129_236, 257),
        ),
        (
            calls(235, 66_472, 15_289, 344, 0, 344, 1_342_271, 15_288),
            calls(235, 66_472, 15_289, 344, 0, 344, 1_342_271, 15_288),
        ),
    ];
    let actual = fixtures().map(|(_, shape)| (session(shape, true), session(shape, false)));
    assert_eq!(
        actual, baseline,
        "per fixture, in order: vectored writes taken whole, then first slices only"
    );
}

/// The segment size of the default write batch range: the ceiling of a
/// batch that starts with a frame at most this large.
const SEGMENT: usize = *WRITE_BATCH_RANGE.start();

/// The largest batch of the default write batch range: the ceiling of a
/// batch that starts with a frame larger than [`SEGMENT`].
const LARGEST: usize = *WRITE_BATCH_RANGE.end();

/// What [`check_flushes`] counted on one side's trace.
#[derive(Clone, Copy, Debug, Default)]
struct Flushes {
    /// Flushes of frames that each fit [`SEGMENT`], which the segment size
    /// ended: the next item would not have fitted beside them.
    ended_by_segment: u64,
    /// Flushes whose first item is larger than [`SEGMENT`] and that carry
    /// more items after it.
    shared_large: u64,
}

/// Check that every flush in `trace` carries whole items within its
/// batch's ceiling, or one item larger than it, and count how its flushes
/// ended.
///
/// `trace` is a side's trace at the default write batch range over a
/// write half that never waited. Since nothing waited, a batch's ceiling is
/// [`SEGMENT`] when its first item fits it and fewer than [`CUTS_TO_GROW`]
/// batches in a row before it were cut at their ceiling, and [`LARGEST`]
/// otherwise. A batch was cut at its ceiling only if the item after it
/// would not have fitted beside it under that ceiling, so only if it would
/// not have fitted under the smallest ceiling the batch could have had:
/// [`SEGMENT`] when its first item fits it. The check counts the flushes in
/// a row that may have been cut by that measure, which are at least as
/// many as were, and predicts [`SEGMENT`] for every flush whose first item
/// fits it and that follows fewer than [`CUTS_TO_GROW`] of them: there the
/// session must have used it, and elsewhere [`LARGEST`] bounds either
/// ceiling. Every item must reach a flush.
fn check_flushes(trace: &Trace) -> Flushes {
    assert_eq!(trace.waits, 0, "the write half never waits");
    let mut counted = Flushes::default();
    let mut items = trace.items.iter().copied().peekable();
    let mut possible_cuts = 0;
    for &flush in &trace.flushes {
        let mut carried = Vec::new();
        while carried.iter().sum::<usize>() < flush {
            carried.push(items.next().expect("a flush carries only items written"));
        }
        assert_eq!(
            carried.iter().sum::<usize>(),
            flush,
            "a flush ends at an item boundary"
        );
        let smallest = if carried[0] <= SEGMENT {
            SEGMENT
        } else {
            LARGEST
        };
        let ceiling = if smallest == SEGMENT && possible_cuts < CUTS_TO_GROW {
            SEGMENT
        } else {
            LARGEST
        };
        assert!(
            flush <= ceiling || carried.len() == 1,
            "a flush of {flush} bytes in {} items, against a ceiling of {ceiling}: {carried:?}",
            carried.len()
        );
        let possibly_cut = items.peek().is_some_and(|&next| flush + next > smallest);
        possible_cuts = if possibly_cut {
            (possible_cuts + 1).min(CUTS_TO_GROW)
        } else {
            0
        };
        if ceiling == SEGMENT && possibly_cut {
            counted.ended_by_segment += 1;
        }
        if carried[0] > SEGMENT && carried.len() > 1 {
            counted.shared_large += 1;
        }
    }
    assert!(items.next().is_none(), "every item was flushed");
    counted
}

/// The frames per flush beyond the control items' flushes, in hundredths
/// of a frame.
fn hundredths_per_flush(calls: Calls) -> u64 {
    let controls = CONTROL_ITEMS * SIDES;
    assert!(calls.flushes > controls, "{calls:?}");
    calls.frames * 100 / (calls.flushes - controls)
}

/// In the disjoint session at the default write batch range, every flush
/// keeps to its batch's ceiling, some reach the segment size, and the
/// yield batches more frames per flush than flushing at every empty poll.
///
/// Each flush writes the session's buffer out as one plain write, which a
/// raw socket pays a system call for, so what a flush carries is the
/// batching a session achieves on the wire. This session sends many small
/// frames (about 88 bytes each) across large divergence, as a session
/// reconciling millions of small differences does.
///
/// The multiplexer shares one task set with the encoders that feed it, and
/// taking a frame from a lane wakes that lane's encoder, which runs only
/// once the multiplexer's poll returns. So when a poll finds every lane
/// empty with frames unflushed, the multiplexer yields to the set, a few
/// passes in a row, and flushes only if none of those passes brings a
/// frame. Its batch grows across passes of the set until a run of passes
/// brings nothing or the next frame would push it past its ceiling.
///
/// The test checks each side's flushes against the items it wrote, which
/// an observer records in write order ([`check_flushes`]). Some flushes
/// must be ones the segment size ended, where the next item would not
/// have fitted, so the ceiling is exercised rather than met vacuously by
/// small flushes.
///
/// The floor compares two runs of this session that differ only in the
/// yield. The unbounded range `..` sets no segment size and no largest
/// batch, so no ceiling ends a batch; `..=0` sets no ceiling either, and
/// flushes at every poll that finds no frame ready, without yielding. In
/// both, the buffer writes itself out only when full. The yielding run must
/// carry more frames per flush, beyond the control items' flushes: if it
/// carries no more, its flushes no longer wait for the encoders the
/// multiplexer woke. Both runs must flush beyond the control items, so
/// neither count can stop counting unnoticed.
#[test]
fn flushes_batch_frames_within_the_write_batch_range() {
    let (_, traces) = session_traced(disjoint(), true, WRITE_BATCH_RANGE, true);
    let ended_by_segment: u64 = traces
        .iter()
        .map(|trace| check_flushes(trace).ended_by_segment)
        .sum();
    assert!(ended_by_segment > 0, "no flush reached the segment size");
    let yielding = hundredths_per_flush(session_at(disjoint(), true, ..));
    let eager = hundredths_per_flush(session_at(disjoint(), true, ..=0));
    assert!(
        yielding > eager,
        "{yielding} hundredths of a frame per flush with the yield, {eager} without"
    );
}

/// In the catch-up at the default write batch range, supply runs larger
/// than the segment size share flushes with the frames after them, up to
/// the largest batch, so the session flushes less often than it sends
/// frames.
///
/// The catch-up's supply runs average about 4.4 KB, above the segment
/// size, about 1.4 KB, so each starts a batch whose ceiling is the largest
/// batch, 16 KiB, after the batch before it is flushed. Every flush must
/// keep to its ceiling ([`check_flushes`]), some flushes must carry a
/// large run and items after it, and the flushes beyond the control items'
/// must number fewer than the frames.
#[test]
fn large_frames_share_flushes_in_the_catch_up() {
    let (calls, traces) = session_traced(Shape::catch_up(10_000), true, WRITE_BATCH_RANGE, true);
    let shared_large: u64 = traces
        .iter()
        .map(|trace| check_flushes(trace).shared_large)
        .sum();
    assert!(shared_large > 0, "no large run shared a flush");
    assert!(
        calls.flushes - CONTROL_ITEMS * SIDES < calls.frames,
        "{calls:?}"
    );
}

/// At the write batch range `..=0`, the fixed sessions' calls equal those
/// of a multiplexer that flushes at every poll finding no frame ready,
/// without yielding first, with no ceiling on a flush.
///
/// An end of zero selects exactly that rule, so the counts are the ones
/// that rule produces on these deterministic sessions. A flush carries
/// the frames ready within one poll, as many as the lanes then hold, and
/// the buffer writes itself out whenever the next frame does not fit, as
/// the catch-up's writes outnumbering its flushes show. The counts must
/// not move while an end of zero keeps its meaning; they move when the
/// lanes' capacities or the session's scheduling change what one poll
/// finds ready.
#[test]
fn a_zero_batch_flushes_at_every_empty_poll() {
    let calls =
        |transport_reads, reads, fills, writes, vectored_writes, flushes, written_bytes, frames| {
            Calls {
                transport_reads,
                reads,
                fills,
                consumes: 0,
                writes,
                vectored_writes,
                flushes,
                written_bytes,
                frames,
            }
        };
    let expected = [
        calls(4, 14, 0, 6, 0, 6, 11_536, 0),
        calls(39, 1_435, 336, 33, 0, 33, 86_058, 335),
        calls(143, 1_688, 258, 26, 0, 13, 1_129_236, 257),
        calls(453, 66_471, 15_289, 1_581, 0, 1_580, 1_342_271, 15_288),
    ];
    for vectored in [true, false] {
        let actual = fixtures().map(|(_, shape)| session_at(shape, vectored, ..=0));
        assert_eq!(actual, expected, "vectored writes taken whole: {vectored}");
    }
}
