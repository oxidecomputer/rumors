//! A latency-injecting in-memory link: the wire-delay knob for benchmarks.
//!
//! [`delayed_pair`] mirrors the topology of [`rumors::link::memory`] — a
//! bidirectional control stream plus announced unidirectional data streams —
//! but builds every stream from a *delayed pipe*: bytes written at instant
//! `t` become readable at `t + delay`, under a byte-bounded in-flight
//! window. `delay` is the link's one-way latency, so a blocking
//! request/response exchange pays `2 * delay`.
//!
//! # The measurement model
//!
//! [`DelayedWire`] drives both session ends on a current-thread Tokio
//! runtime whose clock is **paused**: pipe arrival deadlines live in virtual
//! time, which advances only while every task is blocked waiting on one.
//! On a paused-clock wire, the duration [`DelayedWire::round_trip`]
//! reports is the sum of two disjoint components (on the
//! [`new_wall_clock`](DelayedWire::new_wall_clock) cross-check variant the
//! clocks coincide, and the report is real elapsed time alone):
//!
//! - the *wall* component: real CPU time spent computing — both peers
//!   serialized on one thread, the same convention as the zero-latency
//!   harness in [`wire.rs`](wire.rs); and
//! - the *virtual* component: time the session spent with every task
//!   blocked on the wire — `delay` times the length of the session's
//!   longest serialized chain of stream hops, plus any window stalls.
//!
//! The sum approximates session completion time on a link with the given
//! latency (an overestimate insofar as a real deployment overlaps one
//! peer's compute with the other's wait). At `delay = 0` the virtual
//! component vanishes and the figure degenerates to the zero-latency
//! harness's wall measure. A sweep over `delay` therefore separates the
//! two costs cleanly: the intercept is computational overhead, the slope
//! is latency sensitivity in units of serialized one-way hops.
//!
//! Two deliberate simplifications:
//!
//! - Stream *announcement* is instantaneous; only bytes are delayed. A real
//!   transport delivers a new stream's existence together with its first
//!   bytes, and the first bytes here already arrive `delay` late, so the
//!   opening hop is still paid exactly once.
//! - Tokio's timer wheel has millisecond granularity, so arrival deadlines
//!   round up to the next millisecond: choose whole-millisecond delays.
//!
//! The `capacity` argument bounds bytes in flight per stream (written but
//! not yet read), which models a receive window: sustained per-stream
//! throughput is capped at `capacity / delay`. Pass a capacity comfortably
//! above the largest per-stream transfer to isolate round-trip structure
//! from bandwidth-delay throttling, or a tight one to study the window.
//!
//! # Conformance
//!
//! `tests/latency_link.rs` runs this link through the public
//! [`rumors::conformance`] suite, at zero and at nonzero delay. The clauses
//! the suite cannot observe hold by construction: buffering is bounded by
//! `capacity` per stream, a stream mid-delivery when an `accept` future is
//! dropped is redelivered by the next `accept` (the announcement channel
//! owns it, and `recv` is cancel-safe), and `connect`/`accept` fail only
//! when the peer's link half is gone.

use std::collections::VecDeque;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use borsh::{BorshDeserialize, BorshSerialize};
use rumors::Rumors;
use rumors::link::{Acceptor, Connector, Link, STREAM_COUNT};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::mpsc;
use tokio::time::{Instant, Sleep};

/// One in-flight write: its bytes and the virtual instant they arrive.
struct Chunk {
    arrives: Instant,
    data: Vec<u8>,
    /// Bytes of `data` already consumed by the reader (partial reads split
    /// a chunk without re-queueing).
    read: usize,
}

/// State shared by the two halves of one delayed pipe.
struct Shared {
    queue: VecDeque<Chunk>,
    /// Bytes written but not yet read: the in-flight window occupancy.
    buffered: usize,
    capacity: usize,
    writer_gone: bool,
    reader_gone: bool,
    read_waker: Option<Waker>,
    write_waker: Option<Waker>,
}

impl Shared {
    fn wake_reader(&mut self) {
        if let Some(waker) = self.read_waker.take() {
            waker.wake();
        }
    }

    fn wake_writer(&mut self) {
        if let Some(waker) = self.write_waker.take() {
            waker.wake();
        }
    }
}

/// The write half of a delayed pipe; dropping it half-closes the stream.
pub struct DelayedWriter {
    shared: Arc<Mutex<Shared>>,
    delay: Duration,
}

/// The read half of a delayed pipe: bytes surface `delay` after the write.
pub struct DelayedReader {
    shared: Arc<Mutex<Shared>>,
    /// Timer armed for the head chunk's arrival. Minted lazily on first
    /// need: a `Sleep` must be created inside a runtime with a time driver,
    /// and pipes are constructed outside one.
    timer: Option<Pin<Box<Sleep>>>,
}

/// Create one unidirectional delayed pipe.
///
/// Bytes written become readable `delay` later; at most `capacity` bytes
/// may be in flight (written but unread) before the writer blocks.
///
/// # Panics
///
/// If `capacity` is zero: a zero-capacity pipe could never carry a byte.
pub fn delayed_pipe(capacity: usize, delay: Duration) -> (DelayedWriter, DelayedReader) {
    assert!(capacity > 0, "a delayed pipe must admit at least one byte");
    let shared = Arc::new(Mutex::new(Shared {
        queue: VecDeque::new(),
        buffered: 0,
        capacity,
        writer_gone: false,
        reader_gone: false,
        read_waker: None,
        write_waker: None,
    }));
    (
        DelayedWriter {
            shared: Arc::clone(&shared),
            delay,
        },
        DelayedReader {
            shared,
            timer: None,
        },
    )
}

impl AsyncWrite for DelayedWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let mut shared = self.shared.lock().unwrap();
        if shared.reader_gone {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "peer pipe is gone",
            )));
        }
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let free = shared.capacity - shared.buffered;
        if free == 0 {
            shared.write_waker = Some(cx.waker().clone());
            return Poll::Pending;
        }
        let n = free.min(buf.len());
        shared.queue.push_back(Chunk {
            arrives: Instant::now() + self.delay,
            data: buf[..n].to_vec(),
            read: 0,
        });
        shared.buffered += n;
        shared.wake_reader();
        Poll::Ready(Ok(n))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut shared = self.shared.lock().unwrap();
        shared.writer_gone = true;
        shared.wake_reader();
        Poll::Ready(Ok(()))
    }
}

impl Drop for DelayedWriter {
    fn drop(&mut self) {
        let mut shared = self.shared.lock().unwrap();
        shared.writer_gone = true;
        shared.wake_reader();
    }
}

impl Drop for DelayedReader {
    fn drop(&mut self) {
        let mut shared = self.shared.lock().unwrap();
        shared.reader_gone = true;
        shared.wake_writer();
    }
}

impl AsyncRead for DelayedReader {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let mut delivered = false;
        loop {
            let mut shared = this.shared.lock().unwrap();
            let Some(head) = shared.queue.front_mut() else {
                if delivered || shared.writer_gone {
                    // Bytes were delivered this poll, or a drained queue
                    // under a closed writer is end-of-stream.
                    return Poll::Ready(Ok(()));
                }
                shared.read_waker = Some(cx.waker().clone());
                return Poll::Pending;
            };
            let arrives = head.arrives;
            if arrives > Instant::now() {
                if delivered {
                    // Ripe bytes are already in the caller's buffer; the
                    // unripe head is the next poll's business.
                    return Poll::Ready(Ok(()));
                }
                drop(shared);
                let timer = match this.timer.as_mut() {
                    Some(timer) => {
                        if timer.deadline() != arrives {
                            timer.as_mut().reset(arrives);
                        }
                        timer
                    }
                    None => this
                        .timer
                        .insert(Box::pin(tokio::time::sleep_until(arrives))),
                };
                match timer.as_mut().poll(cx) {
                    // The arrival instant is here; re-take the lock and
                    // deliver on the next loop pass.
                    Poll::Ready(()) => continue,
                    Poll::Pending => return Poll::Pending,
                }
            }
            if buf.remaining() == 0 {
                return Poll::Ready(Ok(()));
            }
            let take = buf.remaining().min(head.data.len() - head.read);
            buf.put_slice(&head.data[head.read..head.read + take]);
            head.read += take;
            if head.read == head.data.len() {
                shared.queue.pop_front();
            }
            shared.buffered -= take;
            shared.wake_writer();
            delivered = true;
        }
    }
}

/// The delayed-pipe [`Connector`]: each open mints a pipe and announces the
/// read end to the peer's acceptor.
///
/// The announcement itself is undelayed; see the module docs for why no
/// hop is lost.
#[derive(Clone)]
pub struct DelayedConnector {
    announce: mpsc::Sender<DelayedReader>,
    capacity: usize,
    delay: Duration,
}

impl Connector for DelayedConnector {
    type Tx = DelayedWriter;

    async fn connect(&self) -> io::Result<Self::Tx> {
        let (tx, rx) = delayed_pipe(self.capacity, self.delay);
        self.announce
            .send(rx)
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "peer link is gone"))?;
        Ok(tx)
    }
}

/// The delayed-pipe [`Acceptor`]: yields the peer's announced streams in
/// open order.
pub struct DelayedAcceptor {
    streams: mpsc::Receiver<DelayedReader>,
}

impl Acceptor for DelayedAcceptor {
    type Rx = DelayedReader;

    async fn accept(&mut self) -> io::Result<Self::Rx> {
        self.streams
            .recv()
            .await
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "peer link is gone"))
    }
}

/// The delayed in-memory [`Link`] type: both ends of [`delayed_pair`].
pub type DelayedLink = Link<DelayedReader, DelayedWriter, DelayedConnector, DelayedAcceptor>;

/// Create a connected pair of in-memory links with one-way latency `delay`.
///
/// Every stream — the control stream's two halves and each data stream —
/// is a delayed pipe with the given `capacity` and `delay`.
pub fn delayed_pair(capacity: usize, delay: Duration) -> (DelayedLink, DelayedLink) {
    let (a_control_write, b_control_read) = delayed_pipe(capacity, delay);
    let (b_control_write, a_control_read) = delayed_pipe(capacity, delay);
    let (a_announce, b_streams) = mpsc::channel(STREAM_COUNT);
    let (b_announce, a_streams) = mpsc::channel(STREAM_COUNT);
    (
        Link::new(
            a_control_read,
            a_control_write,
            DelayedConnector {
                announce: a_announce,
                capacity,
                delay,
            },
            DelayedAcceptor { streams: a_streams },
        ),
        Link::new(
            b_control_read,
            b_control_write,
            DelayedConnector {
                announce: b_announce,
                capacity,
                delay,
            },
            DelayedAcceptor { streams: b_streams },
        ),
    )
}

/// A persistent latency-injecting connection plus its paused-clock runtime.
///
/// The counterpart of `wire::Wire` for the latency sweeps: one link pair
/// reused at clean session boundaries, driven on a current-thread runtime
/// whose clock starts paused so pipe delays cost virtual — not wall — time.
pub struct DelayedWire {
    runtime: tokio::runtime::Runtime,
    a_link: DelayedLink,
    b_link: DelayedLink,
    /// Whether the runtime's clock started paused. [`round_trip`]
    /// (Self::round_trip) selects its cost model by this: paused wires
    /// partition cost into disjoint wall and virtual components; on a
    /// running clock the virtual component tracks the real one, so only
    /// real elapsed time is reported.
    paused: bool,
}

impl DelayedWire {
    /// Allocate the runtime and one delayed link pair, one end per side.
    pub fn new(capacity: usize, delay: Duration) -> Self {
        Self::with_clock(capacity, delay, true)
    }

    /// [`new`](Self::new) on a running clock: pipe delays burn wall time.
    ///
    /// The cross-check counterpart of the paused default — a wall-clock
    /// measurement over the same pipes tests whether the virtual model's
    /// figures survive real timer scheduling.
    // Used only by `window_wallclock`; the module is `#[path]`-included by
    // several targets, each seeing its own copy's usage.
    #[allow(dead_code)]
    pub fn new_wall_clock(capacity: usize, delay: Duration) -> Self {
        Self::with_clock(capacity, delay, false)
    }

    fn with_clock(capacity: usize, delay: Duration, paused: bool) -> Self {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .start_paused(paused)
            .build()
            .expect("build current-thread runtime");
        let (a_link, b_link) = delayed_pair(capacity, delay);
        Self {
            runtime,
            a_link,
            b_link,
            paused,
        }
    }

    /// Reconcile one pair, returning the handles and the session's cost.
    ///
    /// On the paused-clock default the reported duration is wall compute
    /// plus virtual wire stall; see the [module docs](self) for the model
    /// and its epistemic status. On a
    /// [`new_wall_clock`](Self::new_wall_clock) wire the virtual clock
    /// tracks the real one — the two components overlap instead of
    /// partitioning — so the report is real elapsed time alone.
    pub fn round_trip<T>(
        &mut self,
        a: Rumors<T>,
        b: Rumors<T>,
    ) -> ((Rumors<T>, Rumors<T>), Duration)
    where
        T: BorshSerialize + BorshDeserialize + Send + Sync + 'static,
    {
        let Self {
            runtime,
            a_link,
            b_link,
            paused,
        } = self;
        let wall_start = std::time::Instant::now();
        let virtual_elapsed = runtime.block_on(async {
            let virtual_start = Instant::now();
            let (a_result, b_result) = tokio::join!(a.gossip(a_link), b.gossip(b_link));
            a_result.expect("peer A gossip");
            b_result.expect("peer B gossip");
            virtual_start.elapsed()
        });
        let elapsed = if *paused {
            wall_start.elapsed() + virtual_elapsed
        } else {
            // A running virtual clock advances with the real one, so wall
            // and virtual are two measurements of the same interval:
            // summing them would double-count. Real elapsed time is the
            // whole cost.
            wall_start.elapsed()
        };
        ((a, b), elapsed)
    }
}
