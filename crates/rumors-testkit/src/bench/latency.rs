//! A latency-injecting in-memory connection, which gives benchmarks a
//! configurable one-way wire delay.
//!
//! [`crate::bench::latency::delayed_pair`] mirrors the topology of
//! [`rumors::testing::memory_connection`], one pipe per direction, but builds
//! each direction from a *delayed pipe*: bytes written at instant `t` become
//! readable at `t + delay`, under a byte-bounded in-flight window. `delay` is
//! the connection's one-way latency, so a blocking request/response exchange
//! pays `2 * delay`.
//!
//! # The measurement model
//!
//! [`crate::bench::latency::DelayedWire`] drives each session end as its own
//! task on a current-thread Tokio runtime whose clock is **paused**: pipe
//! arrival deadlines live in virtual time, which advances only when the
//! runtime's one thread has no ready task left and parks on its own timer
//! driver. A session's cost splits into two components:
//!
//! - the *virtual* component: time the session spent with every task blocked on
//!   the wire — `delay` times the length of the session's longest serialized
//!   chain of hops. Compute costs zero virtual time, no thread outside the
//!   runtime participates, and the library arms no timers of its own, so the
//!   figure is a deterministic function of the session shape: OS descheduling
//!   can starve the thread of CPU, never convince the runtime that a pending
//!   task is idle, so machine load cannot move it. Every pipe deadline is
//!   `delay` past an instant that is itself a whole number of delays past the
//!   runtime's epoch, so the figure lands on the delay lattice: dividing by
//!   `delay` reads serialized one-way hops exactly.
//!   [`crate::bench::latency::DelayedWire::round_trip_virtual`] reports this
//!   component alone; it is the load-independent measurement the budget and
//!   hop-trace suites pin their bounds on.
//! - the *wall* component: real CPU time spent computing — both peers
//!   serialized on one thread, as in the zero-latency harness. It moves with
//!   machine load.
//!
//! Each peer runs in its own task because an application runs each peer in its
//! own task, so a wake polls only the peer it is for. Joined into one task, the
//! two peers would both be polled on every wake of either: each wake of the
//! busy peer would re-poll the idle one, adding wall time that belongs to
//! neither peer's work. Separate tasks change the order in which the work ready
//! at one virtual instant runs, never when the clock moves on, which waits
//! until no task is ready.
//!
//! [`crate::bench::latency::DelayedWire::round_trip`] reports their sum, which
//! approximates session completion time on a connection with the given latency
//! (an overestimate insofar as a real deployment overlaps one peer's compute
//! with the other's wait). At `delay = 0` the virtual component vanishes and
//! the sum degenerates to the zero-latency harness's wall measure. A sweep over
//! `delay` separates the two costs in expectation — the intercept is
//! computational overhead, the slope is latency sensitivity — which suits the
//! benches, where load noise averages out across samples; assertions belong on
//! the virtual figure instead. On the
//! [`new_wall_clock`](crate::bench::latency::DelayedWire::new_wall_clock)
//! cross-check variant the clocks coincide, and
//! [`crate::bench::latency::DelayedWire::round_trip`]'s report is real elapsed
//! time alone.
//!
//! Tokio's timer wheel has millisecond granularity, so arrival deadlines round
//! up to the next millisecond: choose whole-millisecond delays.
//!
//! The `capacity` argument bounds bytes in flight per direction (written but
//! not yet read), which models a receive window: sustained throughput in each
//! direction is capped at `capacity / delay`. Pass a capacity comfortably above
//! the largest one-direction transfer to isolate round-trip structure from
//! bandwidth-delay throttling, or a tight one to study the window.
//!
//! The two directions are separate pipes, so each progresses independently and
//! buffers at most `capacity` bytes, as the transport requirements demand.

use std::collections::VecDeque;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use rumors::Rumors;
use tokio::io::{AsyncRead, AsyncWrite, BufReader, ReadBuf};
use tokio::time::{Instant, Sleep};

/// One in-flight write: its bytes and the virtual instant they arrive.
struct Chunk {
    arrives: Instant,
    data: Vec<u8>,
    /// Bytes of `data` already consumed by the reader (partial reads split a
    /// chunk without re-queueing).
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

/// The write half of a delayed pipe; dropping it closes the direction.
pub struct DelayedWriter {
    shared: Arc<Mutex<Shared>>,
    delay: Duration,
}

/// The read half of a delayed pipe: bytes surface `delay` after the write.
pub struct DelayedReader {
    shared: Arc<Mutex<Shared>>,
    /// Timer armed for the head chunk's arrival. Created lazily on first need:
    /// a `Sleep` must be created inside a runtime with a time driver, and pipes
    /// are constructed outside one.
    timer: Option<Pin<Box<Sleep>>>,
}

/// Create one unidirectional delayed pipe.
///
/// Bytes written become readable `delay` later; at most `capacity` bytes may be
/// in flight (written but unread) before the writer blocks.
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

/// One end of a delayed in-memory connection: the two halves a session takes.
pub struct DelayedEnd {
    /// The read half, buffered as a session requires: the peer's writes surface
    /// here `delay` late.
    pub read: BufReader<DelayedReader>,
    /// The write half.
    pub write: DelayedWriter,
}

/// Create a connected pair of in-memory connection ends with one-way latency
/// `delay`.
///
/// Each direction is a delayed pipe with the given `capacity` and `delay`.
pub fn delayed_pair(capacity: usize, delay: Duration) -> (DelayedEnd, DelayedEnd) {
    let (a_write, b_read) = delayed_pipe(capacity, delay);
    let (b_write, a_read) = delayed_pipe(capacity, delay);
    (
        DelayedEnd {
            read: BufReader::new(a_read),
            write: a_write,
        },
        DelayedEnd {
            read: BufReader::new(b_read),
            write: b_write,
        },
    )
}

/// A persistent latency-injecting connection plus its paused-clock runtime.
///
/// The counterpart of `wire::Wire` for the latency sweeps: one connection
/// reused at clean session boundaries, each session's peers driven as two tasks
/// on a current-thread runtime whose clock starts paused so pipe delays cost
/// virtual — not wall — time.
pub struct DelayedWire {
    runtime: tokio::runtime::Runtime,
    /// The connection's ends, A's and then B's, between sessions: a session's
    /// tasks own them while it runs.
    ends: Option<(DelayedEnd, DelayedEnd)>,
    /// Whether the runtime's clock started paused.
    ///
    /// [`round_trip`](Self::round_trip) selects its cost model by this: paused
    /// wires partition cost into disjoint wall and virtual components; on a
    /// running clock the virtual component tracks the real one, so only real
    /// elapsed time is reported.
    paused: bool,
}

impl DelayedWire {
    /// Allocate the runtime and one delayed connection, one end per side.
    pub fn new(capacity: usize, delay: Duration) -> Self {
        Self::with_clock(capacity, delay, true)
    }

    /// [`new`](Self::new) on a running clock: pipe delays burn wall time.
    ///
    /// The cross-check counterpart of the paused default — a wall-clock
    /// measurement over the same pipes tests whether the virtual model's
    /// figures survive real timer scheduling.
    pub fn new_wall_clock(capacity: usize, delay: Duration) -> Self {
        Self::with_clock(capacity, delay, false)
    }

    fn with_clock(capacity: usize, delay: Duration, paused: bool) -> Self {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .start_paused(paused)
            .build()
            .expect("build current-thread runtime");
        Self {
            runtime,
            ends: Some(delayed_pair(capacity, delay)),
            paused,
        }
    }

    /// Reconcile one pair, returning the handles and the session's cost.
    ///
    /// On the paused-clock default the reported duration is wall compute plus
    /// virtual wire stall; see the [module docs](self) for the model and its
    /// epistemic status. On a [`new_wall_clock`](Self::new_wall_clock) wire the
    /// virtual clock tracks the real one — the two components overlap instead
    /// of partitioning — so the report is real elapsed time alone.
    pub fn round_trip<T>(
        &mut self,
        a: Rumors<T>,
        b: Rumors<T>,
    ) -> ((Rumors<T>, Rumors<T>), Duration)
    where
        T: serde::Serialize + serde::de::DeserializeOwned + Eq + Send + Sync + 'static,
    {
        let wall_start = std::time::Instant::now();
        let (pair, virtual_elapsed) = self.reconcile(a, b);
        let elapsed = if self.paused {
            wall_start.elapsed() + virtual_elapsed
        } else {
            // A running virtual clock advances with the real one, so wall
            // and virtual are two measurements of the same interval:
            // summing them would double-count. Real elapsed time is the
            // whole cost.
            wall_start.elapsed()
        };
        (pair, elapsed)
    }

    /// Reconcile one pair, returning the handles and the session's wire cost
    /// alone: the virtual component, exact and load-independent.
    ///
    /// This is the measurement to assert on: a deterministic function of the
    /// session shape, landing on the delay lattice, unmoved by machine load
    /// (see the [module docs](self) for the argument).
    ///
    /// # Panics
    ///
    /// On a [`new_wall_clock`](Self::new_wall_clock) wire, where the virtual
    /// clock tracks the real one and the component is wall time in disguise:
    /// load-independence is the figure's contract, so a wire that cannot honor
    /// it must refuse loudly.
    pub fn round_trip_virtual<T>(
        &mut self,
        a: Rumors<T>,
        b: Rumors<T>,
    ) -> ((Rumors<T>, Rumors<T>), Duration)
    where
        T: serde::Serialize + serde::de::DeserializeOwned + Eq + Send + Sync + 'static,
    {
        assert!(
            self.paused,
            "virtual wire cost is only meaningful on a paused clock: \
             a running clock's virtual component is wall time in disguise",
        );
        self.reconcile(a, b)
    }

    /// Drive one gossip session to completion, each peer as its own task,
    /// timing it in virtual time.
    ///
    /// # Panics
    ///
    /// If either peer's session fails, with that peer's panic; and on a wire
    /// whose earlier session panicked, which kept the connection's ends.
    fn reconcile<T>(&mut self, a: Rumors<T>, b: Rumors<T>) -> ((Rumors<T>, Rumors<T>), Duration)
    where
        T: serde::Serialize + serde::de::DeserializeOwned + Eq + Send + Sync + 'static,
    {
        let (a_end, b_end) = self
            .ends
            .take()
            .expect("a wire whose session panicked holds no connection");
        let (a, b, a_end, b_end, virtual_elapsed) = self.runtime.block_on(async {
            let virtual_start = Instant::now();
            let a_task = tokio::spawn(gossip(a, a_end, "peer A gossip"));
            let b_task = tokio::spawn(gossip(b, b_end, "peer B gossip"));
            let (a, a_end) = finished(a_task.await);
            let (b, b_end) = finished(b_task.await);
            (a, b, a_end, b_end, virtual_start.elapsed())
        });
        self.ends = Some((a_end, b_end));
        ((a, b), virtual_elapsed)
    }
}

/// Gossip once over `end`, as one peer's task, handing back the peer and the
/// end for the wire's next session.
///
/// # Panics
///
/// If the session fails, with `peer_name` in the message.
async fn gossip<T>(
    peer: Rumors<T>,
    mut end: DelayedEnd,
    peer_name: &'static str,
) -> (Rumors<T>, DelayedEnd)
where
    T: serde::Serialize + serde::de::DeserializeOwned + Eq + Send + Sync + 'static,
{
    peer.gossip_once(&mut end.read, &mut end.write)
        .await
        .expect(peer_name);
    (peer, end)
}

/// The output of a peer's finished task, or its panic, resumed on the thread
/// that joined it so the session's caller sees the peer's own message.
fn finished<O>(joined: Result<O, tokio::task::JoinError>) -> O {
    joined.unwrap_or_else(|error| match error.try_into_panic() {
        Ok(panic) => std::panic::resume_unwind(panic),
        Err(error) => panic!("nothing cancels a peer's task, so it finishes or panics: {error}"),
    })
}

/// The budget suites' measurement primitive: one session's exact hops.
///
/// Reconciles the pair over a fresh paused-clock wire with the given
/// per-direction `capacity` and one-way `delay`, and reads the virtual wire
/// cost as serialized one-way hops: deterministic and load-independent (see the
/// [module docs](self) for the argument).
pub fn session_hops<T>(capacity: usize, delay: Duration, (a, b): (Rumors<T>, Rumors<T>)) -> u32
where
    T: serde::Serialize + serde::de::DeserializeOwned + Eq + Send + Sync + 'static,
{
    let mut wire = DelayedWire::new(capacity, delay);
    let (_pair, elapsed) = wire.round_trip_virtual(a, b);
    hops_on_lattice(elapsed, delay)
}

/// Read a virtual wire cost as a whole number of one-way hops at `delay`.
///
/// # Panics
///
/// If `delay` is zero or not a whole number of milliseconds — Tokio's timer
/// wheel quantizes below the millisecond, so no lattice exists there — or if
/// `elapsed` is off the delay lattice: exactness is the invariant the
/// measurement model rests on (see the [module docs](self)), so drift fails
/// loudly instead of rounding silently.
pub fn hops_on_lattice(elapsed: Duration, delay: Duration) -> u32 {
    // The timer wheel's grain: a lattice delay is a whole multiple.
    const WHEEL_GRAIN: Duration = Duration::from_millis(1);
    assert!(
        !delay.is_zero() && delay.as_nanos().is_multiple_of(WHEEL_GRAIN.as_nanos()),
        "the delay lattice needs a whole-millisecond one-way delay: \
         got {delay:?}, and the timer wheel quantizes below {WHEEL_GRAIN:?}",
    );
    assert_eq!(
        elapsed.as_nanos() % delay.as_nanos(),
        0,
        "virtual wire cost must land on the delay lattice: \
         {elapsed:?} at one-way delay {delay:?}",
    );
    u32::try_from(elapsed.as_nanos() / delay.as_nanos()).expect("bounded hop count")
}
