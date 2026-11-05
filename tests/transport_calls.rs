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
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};

use rumors::testing::{MemoryEnd, memory_connection};
use rumors::{DEFAULT_SYNC_MEMORY_BUDGET, Rumors};
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
        let poll = completed(&self.counters.vectored_writes, poll);
        self.accepted(poll)
    }

    /// Whether this half takes vectored writes whole.
    fn is_write_vectored(&self) -> bool {
        self.vectored
    }

    /// Flushes through, counting the call if it completes.
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let poll = Pin::new(&mut self.inner).poll_flush(cx);
        completed(&self.counters.flushes, poll)
    }

    /// Shuts down through, uncounted: a session never shuts its write half.
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// Gossip once over `end` through counting halves, returning the calls and
/// the frames this side sent.
///
/// The read half's buffer is rebuilt over a counted transport, as
/// `memory_connection` builds it (a `BufReader` of Tokio's default
/// capacity), so the reads it makes of the transport are counted apart
/// from the session's calls on it.
async fn gossip(peer: Rumors<String>, end: MemoryEnd, vectored: bool) -> Calls {
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
    Calls {
        transport_reads: load(&beneath.reads),
        reads: load(&counters.reads),
        fills: load(&counters.fills),
        consumes: load(&counters.consumes),
        writes: load(&counters.writes),
        vectored_writes: load(&counters.vectored_writes),
        flushes: load(&counters.flushes),
        written_bytes: load(&counters.written_bytes),
        frames: gossiped.stats.frames_sent,
    }
}

/// Reconcile the pair `shape` describes once, each peer in its own task,
/// over write halves that do or do not take vectored writes whole, and
/// return the calls both sides made.
///
/// Building the pair runs the peers' own joining sessions, before counting
/// starts.
fn session(shape: Shape, vectored: bool) -> Calls {
    let (a, b) = pair(shape, DEFAULT_SYNC_MEMORY_BUDGET);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("build a current-thread runtime");
    let (a_end, b_end) = memory_connection(CAPACITY);
    let (a, b) = runtime.block_on(async {
        let a = tokio::spawn(gossip(a, a_end, vectored));
        let b = tokio::spawn(gossip(b, b_end, vectored));
        (
            a.await.expect("peer A's task completes"),
            b.await.expect("peer B's task completes"),
        )
    });
    Calls {
        transport_reads: a.transport_reads + b.transport_reads,
        reads: a.reads + b.reads,
        fills: a.fills + b.fills,
        consumes: a.consumes + b.consumes,
        writes: a.writes + b.writes,
        vectored_writes: a.vectored_writes + b.vectored_writes,
        flushes: a.flushes + b.flushes,
        written_bytes: a.written_bytes + b.written_bytes,
        frames: a.frames + b.frames,
    }
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
        ("disjoint", Shape::symmetric(5_000)),
    ]
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
/// after writing an item. No fixture here sends a frame near the buffer's
/// size or readies that much between flushes, so on both kinds of write
/// half every write is one plain write followed by its own flush: the
/// plain writes equal the flushes, no vectored write reaches either half,
/// and the two kinds see the same calls. Those equalities rest on the
/// fixtures as well as on how the session writes, and fail once a fixture
/// outgrows the buffer. Every flush follows at least one item, so the
/// writes number at most the items, and at least the control items. A
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
/// The multiplexer flushes whenever it finds no frame ready, so frames
/// ready together share a flush, about two per flush in these sessions,
/// and its buffer reaches the transport as one plain write per flush,
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
            calls(210, 4_259, 3_381, 1_715, 0, 1_715, 96_831, 3_380),
            calls(210, 4_259, 3_381, 1_715, 0, 1_715, 96_831, 3_380),
        ),
        (
            calls(142, 1_688, 258, 135, 0, 135, 1_129_235, 257),
            calls(142, 1_688, 258, 135, 0, 135, 1_129_235, 257),
        ),
        (
            calls(5_915, 66_204, 15_254, 8_061, 0, 8_061, 1_341_502, 15_253),
            calls(5_915, 66_204, 15_254, 8_061, 0, 8_061, 1_341_502, 15_253),
        ),
    ];
    for ((name, shape), (whole, sliced)) in fixtures().into_iter().zip(baseline) {
        assert_eq!(
            session(shape, true),
            whole,
            "{name}, vectored writes taken whole"
        );
        assert_eq!(session(shape, false), sliced, "{name}, first slices only");
    }
}
