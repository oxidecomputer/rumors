//! A duplex link with bandwidth, propagation delay, and kernel-style send
//! buffers, on simulated time.
//!
//! [`crate::bench::latency`] models a link's delay alone, on Tokio's paused
//! clock. This module adds the two quantities a throughput question needs:
//! the time each byte takes to serialize onto the wire, and the bytes a
//! sending kernel holds that the application has handed over but the wire
//! has not yet taken. [`drive`] runs two peers over a [`Link`] on a clock of
//! its own and reports when every byte left and arrived.
//!
//! # The time unit
//!
//! Time counts in *byte-times*: one byte-time is how long the link takes to
//! serialize one byte, the same in both directions. In that unit the
//! bandwidth is one byte per byte-time, so a link is described entirely by
//! its one-way delay, and its bandwidth-delay product is twice that delay,
//! in bytes. A 1 Gb/s link with a 100 ms round trip and a 100 Gb/s link
//! with a 1 ms round trip have the same 12.5 MB product, and so are the
//! same link here; a session's time on either is its byte-time count
//! divided by the bandwidth in bytes per second. Every quantity is an
//! integer, so a session's timing is exact.
//!
//! # The model of one direction
//!
//! Each direction behaves as a TCP connection's sending kernel, wire, and
//! receiving kernel, simplified to the parts that set timing:
//!
//! 1. A write enters the *unsent* queue: bytes the application has handed
//!    to the kernel that have not begun to serialize. The queue admits
//!    bytes while it holds fewer than its limit
//!    ([`LinkConfig::with_unsent_limit`]) plus one [`SEGMENT_BYTES`]
//!    segment; a writer finding it full waits. A limit of zero therefore
//!    models a socket with `TCP_NOTSENT_LOWAT` at one segment, and a limit
//!    of one bandwidth-delay product a socket whose send buffer holds that
//!    much beyond what is in flight.
//! 2. The wire takes the unsent queue's head one segment at a time, in
//!    order, as soon as it is idle and the in-flight window has room: up to
//!    [`SEGMENT_BYTES`], fewer if fewer are unsent. A segment of `n` bytes
//!    occupies the wire for `n` byte-times. Nothing reorders bytes once
//!    they are written, so whatever an application prioritizes, it can
//!    prioritize only among bytes it has not yet handed over.
//! 3. A segment arrives whole, one one-way delay after its last byte
//!    serialized, and waits in the receive buffer until the reader takes
//!    it.
//! 4. The receiver acknowledges bytes as its reader takes them, and the
//!    acknowledgement reaches the sender one one-way delay later. Bytes
//!    that began to serialize and are not yet acknowledged are *in flight*,
//!    and the wire starts no segment that would put more than one
//!    bandwidth-delay product plus two segments in flight. A sender writing
//!    back to back, to a reader taking bytes as they arrive, never meets
//!    that limit: its unacknowledged segments are those that finished
//!    serializing within the last round trip, which carry fewer than one
//!    product plus one segment of bytes, and its next segment adds one
//!    more. The limit binds only when the reader stops taking bytes, which
//!    is how a full receive buffer stops a TCP sender.
//!
//! The model leaves out loss, congestion control, and the slow start a new
//! connection pays: the in-flight window starts fully open.
//!
//! # The clock
//!
//! [`drive`] polls both peers on the calling thread, in a fixed order,
//! until neither can progress, then advances the clock to the link's next
//! event (a segment finishing serialization, arriving, or being
//! acknowledged) and resumes. Computation therefore takes no simulated
//! time, no timer or thread takes part, and a session's report is a
//! deterministic function of its fixture and link: machine load cannot
//! move it.
//!
//! A link built with [`LinkConfig::latency_only`] serializes in no time and
//! has no in-flight limit. A session's time over it is its critical path:
//! the one-way delay times the longest chain of messages each of which
//! waits for the one before.
//!
//! [`drive`]: crate::bench::link::drive
//! [`Link`]: crate::bench::link::Link
//! [`LinkConfig::with_unsent_limit`]: crate::bench::link::LinkConfig::with_unsent_limit
//! [`LinkConfig::latency_only`]: crate::bench::link::LinkConfig::latency_only
//! [`SEGMENT_BYTES`]: crate::bench::link::SEGMENT_BYTES

use std::collections::VecDeque;
use std::future::Future;
use std::io;
use std::pin::{Pin, pin};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll, Wake, Waker};

use tokio::io::{AsyncRead, AsyncWrite, BufReader, ReadBuf};

/// The most bytes the wire takes from the unsent queue as one segment:
/// 1,448, the TCP payload of a 1,500-byte Ethernet frame with timestamps.
///
/// A segment arrives whole, so timing within a direction resolves to a
/// segment's serialization time, which stays small beside the round trip
/// of any link a test scales down to.
pub const SEGMENT_BYTES: u64 = 1448;

/// One of the two peers on a link.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Side {
    /// The first peer, whose writes travel toward [`Side::B`].
    A,
    /// The second peer, whose writes travel toward [`Side::A`].
    B,
}

impl Side {
    /// Both sides, in the order [`drive`] polls them.
    pub const BOTH: [Self; 2] = [Self::A, Self::B];

    /// This side's position in [`Side::BOTH`], which also indexes the
    /// direction it writes into.
    pub const fn index(self) -> usize {
        match self {
            Self::A => 0,
            Self::B => 1,
        }
    }

    /// The other side.
    pub const fn peer(self) -> Self {
        match self {
            Self::A => Self::B,
            Self::B => Self::A,
        }
    }
}

/// A link's timing parameters, the same in both directions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LinkConfig {
    /// One-way propagation delay, in byte-times.
    one_way_delay: u64,
    /// Bytes the unsent queue holds beyond one segment before a writer
    /// waits.
    unsent_limit: u64,
    /// Whether a byte occupies the wire for one byte-time; a latency-only
    /// link serializes in no time.
    serializes: bool,
}

impl LinkConfig {
    /// A link whose bandwidth-delay product is `bdp` bytes, with an unsent
    /// limit of zero.
    ///
    /// # Panics
    ///
    /// If `bdp` is zero or odd: the one-way delay is half the product, in
    /// whole byte-times.
    pub fn with_bdp(bdp: u64) -> Self {
        assert!(
            bdp > 0 && bdp.is_multiple_of(2),
            "a link's bandwidth-delay product is twice its one-way delay \
             in whole byte-times: got {bdp}",
        );
        Self {
            one_way_delay: bdp / 2,
            unsent_limit: 0,
            serializes: true,
        }
    }

    /// The same link, with the unsent queue holding up to `bytes` beyond
    /// one segment.
    pub fn with_unsent_limit(self, bytes: u64) -> Self {
        Self {
            unsent_limit: bytes,
            ..self
        }
    }

    /// The same delay with infinite bandwidth: bytes serialize in no time,
    /// and nothing limits the bytes in flight or unsent.
    pub fn latency_only(self) -> Self {
        Self {
            serializes: false,
            ..self
        }
    }

    /// The bandwidth-delay product, in bytes.
    pub const fn bdp(&self) -> u64 {
        2 * self.one_way_delay
    }

    /// The round-trip time, in byte-times: numerically the
    /// bandwidth-delay product.
    pub const fn rtt(&self) -> u64 {
        2 * self.one_way_delay
    }

    /// The one-way propagation delay, in byte-times.
    pub const fn one_way_delay(&self) -> u64 {
        self.one_way_delay
    }

    /// The bytes the unsent queue holds beyond one segment before a writer
    /// waits.
    pub const fn unsent_limit(&self) -> u64 {
        self.unsent_limit
    }

    /// Whether bytes take time to serialize: false on a
    /// [latency-only](Self::latency_only) link.
    pub const fn serializes(&self) -> bool {
        self.serializes
    }

    /// The bytes the unsent queue may hold before a writer waits: the
    /// configured limit plus one segment, or no bound on a latency-only
    /// link.
    fn unsent_capacity(&self) -> u64 {
        if self.serializes {
            self.unsent_limit + SEGMENT_BYTES
        } else {
            u64::MAX
        }
    }

    /// The most bytes the wire puts in flight: one bandwidth-delay product
    /// plus two segments, or no bound on a latency-only link.
    fn in_flight_capacity(&self) -> u64 {
        if self.serializes {
            self.bdp() + 2 * SEGMENT_BYTES
        } else {
            u64::MAX
        }
    }

    /// The byte-times one segment of `bytes` occupies the wire.
    fn serialization(&self, bytes: u64) -> u64 {
        if self.serializes { bytes } else { 0 }
    }
}

/// Bytes written into one direction and not yet read, in order.
///
/// The link's timing tracks byte counts only; the bytes themselves wait
/// here from the write that hands them over to the read that takes them.
#[derive(Debug, Default)]
struct ByteQueue {
    /// The bytes, of which those before `head` are already read.
    bytes: Vec<u8>,
    /// The first unread byte.
    head: usize,
}

impl ByteQueue {
    /// Appends `data`.
    fn push(&mut self, data: &[u8]) {
        self.bytes.extend_from_slice(data);
    }

    /// Moves the next `count` bytes into `buf`.
    ///
    /// Read bytes are discarded once they outnumber the unread, so the
    /// queue's memory stays within twice what it holds.
    fn take_into(&mut self, count: usize, buf: &mut ReadBuf<'_>) {
        buf.put_slice(&self.bytes[self.head..self.head + count]);
        self.head += count;
        if self.head * 2 >= self.bytes.len() {
            self.bytes.drain(..self.head);
            self.head = 0;
        }
    }
}

/// One segment's passage through the wire.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Segment {
    /// When its first byte began to serialize.
    pub start: u64,
    /// When its last byte finished serializing.
    pub end: u64,
    /// Its bytes.
    pub bytes: u64,
}

/// One direction of the link: its byte counters, the events pending on
/// them, and the wakers waiting on them.
///
/// Bytes pass four counters in order, each at most the one before:
/// `written`, `departed` (began to serialize), `arrived`, and `read`.
#[derive(Debug, Default)]
struct Direction {
    /// Written and not yet read.
    bytes: ByteQueue,
    /// Bytes the writer has handed over.
    written: u64,
    /// Bytes that have begun to serialize.
    departed: u64,
    /// Bytes delivered to the receive buffer.
    arrived: u64,
    /// Bytes the reader has taken.
    read: u64,
    /// Bytes the sender knows the reader has taken.
    acknowledged: u64,
    /// The segment serializing now: when it finishes, and `departed` once
    /// it has.
    serializing: Option<(u64, u64)>,
    /// Serialized segments in propagation, oldest first: when each
    /// arrives, and `arrived` once it has.
    propagating: VecDeque<(u64, u64)>,
    /// Acknowledgements in propagation, oldest first: when each reaches
    /// the sender, and `acknowledged` once it has.
    acknowledgements: VecDeque<(u64, u64)>,
    /// Whether the writer has shut down or dropped its half.
    closed: bool,
    /// Whether the reader has dropped its half.
    reader_gone: bool,
    /// The reader's waker, while it waits for bytes.
    reader: Option<Waker>,
    /// The writer's waker, while it waits for room.
    writer: Option<Waker>,
    /// Every segment, in departure order.
    segments: Vec<Segment>,
    /// The unsent queue's length over time: each change, as the time it
    /// happened and the length after it.
    unsent: Vec<(u64, u64)>,
    /// The most bytes in flight at once.
    max_in_flight: u64,
}

impl Direction {
    /// Bytes written that have not begun to serialize.
    fn unsent(&self) -> u64 {
        self.written - self.departed
    }

    /// Records the unsent queue's length at `now`, after a change.
    fn record_unsent(&mut self, now: u64) {
        let unsent = self.unsent();
        match self.unsent.last_mut() {
            Some(last) if last.0 == now => last.1 = unsent,
            _ => self.unsent.push((now, unsent)),
        }
    }

    /// Bytes that have begun to serialize and are not yet acknowledged.
    fn in_flight(&self) -> u64 {
        self.departed - self.acknowledged
    }

    /// The earliest pending event on this direction, if any.
    fn next_event(&self) -> Option<u64> {
        [
            self.serializing.map(|(at, _)| at),
            self.propagating.front().map(|&(at, _)| at),
            self.acknowledgements.front().map(|&(at, _)| at),
        ]
        .into_iter()
        .flatten()
        .min()
    }

    /// Applies every event due by `now` and starts every segment that can
    /// start at `now`, waking the reader on new arrivals and the writer on
    /// new room.
    fn pump(&mut self, now: u64, config: &LinkConfig) {
        loop {
            let mut progressed = false;
            if let Some((at, departed)) = self.serializing
                && at <= now
            {
                self.propagating
                    .push_back((at + config.one_way_delay, departed));
                self.serializing = None;
                progressed = true;
            }
            while let Some(&(at, arrived)) = self.propagating.front()
                && at <= now
            {
                self.arrived = arrived;
                self.propagating.pop_front();
                wake(&mut self.reader);
                progressed = true;
            }
            while let Some(&(at, acknowledged)) = self.acknowledgements.front()
                && at <= now
            {
                self.acknowledged = acknowledged;
                self.acknowledgements.pop_front();
                progressed = true;
            }
            if self.serializing.is_none() {
                let room = config.in_flight_capacity().saturating_sub(self.in_flight());
                let bytes = self.unsent().min(SEGMENT_BYTES).min(room);
                if bytes > 0 {
                    let end = now + config.serialization(bytes);
                    self.departed += bytes;
                    self.serializing = Some((end, self.departed));
                    self.segments.push(Segment {
                        start: now,
                        end,
                        bytes,
                    });
                    self.max_in_flight = self.max_in_flight.max(self.in_flight());
                    self.record_unsent(now);
                    wake(&mut self.writer);
                    progressed = true;
                }
            }
            if !progressed {
                return;
            }
        }
    }
}

/// Wakes and clears `waker`, if one is waiting.
fn wake(waker: &mut Option<Waker>) {
    if let Some(waker) = waker.take() {
        waker.wake();
    }
}

/// The link's whole state: the clock and both directions.
#[derive(Debug)]
struct State {
    /// The link's parameters.
    config: LinkConfig,
    /// The current simulated time, in byte-times.
    now: u64,
    /// Both directions, indexed by the writing side's [`Side::index`].
    directions: [Direction; 2],
}

/// Shared ownership of the link's state.
type Shared = Arc<Mutex<State>>;

/// Locks the link's state.
fn lock(shared: &Shared) -> MutexGuard<'_, State> {
    shared
        .lock()
        .expect("no link operation panics while holding the lock")
}

/// A duplex link between two peers, from which each side's halves are
/// taken.
#[derive(Debug, Clone)]
pub struct Link {
    /// The clock and both directions.
    shared: Shared,
}

impl Link {
    /// Creates an idle link at time zero.
    pub fn new(config: LinkConfig) -> Self {
        Self {
            shared: Arc::new(Mutex::new(State {
                config,
                now: 0,
                directions: Default::default(),
            })),
        }
    }

    /// Returns `side`'s two halves.
    pub fn end(&self, side: Side) -> End {
        End {
            read: BufReader::new(Reader {
                shared: self.shared.clone(),
                from: side.peer().index(),
            }),
            write: Writer {
                shared: self.shared.clone(),
                into: side.index(),
            },
        }
    }

    /// The current simulated time, in byte-times.
    pub fn now(&self) -> u64 {
        lock(&self.shared).now
    }

    /// Advances the clock to the earliest pending event and applies every
    /// event due then, returning `false` if no event is pending.
    fn advance(&self) -> bool {
        let mut state = lock(&self.shared);
        let State {
            config,
            now,
            directions,
        } = &mut *state;
        let Some(next) = directions.iter().filter_map(Direction::next_event).min() else {
            return false;
        };
        *now = next;
        for direction in directions.iter_mut() {
            direction.pump(next, config);
        }
        true
    }

    /// What each direction carried, indexed by the writing side.
    fn reports(&self) -> [DirectionReport; 2] {
        let state = lock(&self.shared);
        state
            .directions
            .each_ref()
            .map(|direction| DirectionReport {
                segments: direction.segments.clone(),
                unsent: direction.unsent.clone(),
                max_in_flight: direction.max_in_flight,
            })
    }
}

/// One side's halves of a link.
#[derive(Debug)]
pub struct End {
    /// The read half, buffered as a session requires: bytes the peer
    /// wrote, after their delay.
    pub read: BufReader<Reader>,
    /// The write half.
    pub write: Writer,
}

/// A side's read half.
#[derive(Debug)]
pub struct Reader {
    /// The link's state.
    shared: Shared,
    /// The direction this half reads, indexed by its writer.
    from: usize,
}

/// Reads arrived bytes, waiting for an arrival when none remain.
impl AsyncRead for Reader {
    /// Copies arrived bytes into `buf` and schedules their
    /// acknowledgement, or reports end of stream once the writer has shut
    /// down and every byte it wrote has been read.
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let mut state = lock(&self.shared);
        let State {
            config,
            now,
            directions,
        } = &mut *state;
        let direction = &mut directions[self.from];
        let ready = direction.arrived - direction.read;
        if ready > 0 {
            let count = usize::try_from(ready)
                .unwrap_or(usize::MAX)
                .min(buf.remaining());
            direction.bytes.take_into(count, buf);
            direction.read += count as u64;
            let at = *now + config.one_way_delay;
            match direction.acknowledgements.back_mut() {
                Some(last) if last.0 == at => last.1 = direction.read,
                _ => direction.acknowledgements.push_back((at, direction.read)),
            }
            return Poll::Ready(Ok(()));
        }
        if direction.closed && direction.read == direction.written {
            return Poll::Ready(Ok(()));
        }
        direction.reader = Some(cx.waker().clone());
        Poll::Pending
    }
}

impl Drop for Reader {
    /// Marks the direction's reader gone, so its writer's next write fails.
    fn drop(&mut self) {
        let mut state = lock(&self.shared);
        let direction = &mut state.directions[self.from];
        direction.reader_gone = true;
        wake(&mut direction.writer);
    }
}

/// A side's write half.
#[derive(Debug)]
pub struct Writer {
    /// The link's state.
    shared: Shared,
    /// The direction this half writes, indexed by its writer.
    into: usize,
}

/// Hands bytes to the sending kernel, waiting while the unsent queue is
/// full.
impl AsyncWrite for Writer {
    /// Admits as much of `buf` as the unsent queue has room for, and starts
    /// serializing at once if the wire is idle.
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let mut state = lock(&self.shared);
        let State {
            config,
            now,
            directions,
        } = &mut *state;
        let direction = &mut directions[self.into];
        if direction.reader_gone {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "the peer's read half is gone",
            )));
        }
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let room = config.unsent_capacity().saturating_sub(direction.unsent());
        if room == 0 {
            direction.writer = Some(cx.waker().clone());
            return Poll::Pending;
        }
        let count = usize::try_from(room).unwrap_or(usize::MAX).min(buf.len());
        direction.bytes.push(&buf[..count]);
        direction.written += count as u64;
        direction.record_unsent(*now);
        direction.pump(*now, config);
        Poll::Ready(Ok(count))
    }

    /// Completes at once: the kernel sends without being flushed.
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    /// Marks the direction closed, so the reader sees end of stream after
    /// the last written byte.
    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut state = lock(&self.shared);
        let direction = &mut state.directions[self.into];
        direction.closed = true;
        wake(&mut direction.reader);
        Poll::Ready(Ok(()))
    }
}

impl Drop for Writer {
    /// Closes the direction, as a shutdown does.
    fn drop(&mut self) {
        let mut state = lock(&self.shared);
        let direction = &mut state.directions[self.into];
        direction.closed = true;
        wake(&mut direction.reader);
    }
}

/// What one direction carried over a driven session.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DirectionReport {
    /// Every segment, in departure order.
    pub segments: Vec<Segment>,
    /// The unsent queue's length over time: each change, as the time it
    /// happened and the length after it, in time order.
    pub unsent: Vec<(u64, u64)>,
    /// The most bytes in flight at once.
    pub max_in_flight: u64,
}

impl DirectionReport {
    /// Every byte the direction carried.
    pub fn bytes(&self) -> u64 {
        self.segments.iter().map(|segment| segment.bytes).sum()
    }

    /// When the last byte finished serializing, or zero if none was sent.
    pub fn last_serialized(&self) -> u64 {
        self.segments.last().map_or(0, |segment| segment.end)
    }

    /// The most bytes the unsent queue held at once.
    pub fn max_unsent(&self) -> u64 {
        self.unsent
            .iter()
            .map(|&(_, unsent)| unsent)
            .max()
            .unwrap_or(0)
    }

    /// The unsent queue's length integrated over `[from, to)`, in
    /// byte-times times bytes: divided by `to - from`, its mean length.
    pub fn unsent_area_between(&self, from: u64, to: u64) -> u128 {
        let mut area = 0u128;
        for (index, &(at, unsent)) in self.unsent.iter().enumerate() {
            // The length holds from this change until the next one, or
            // indefinitely after the last.
            let until = self
                .unsent
                .get(index + 1)
                .map_or(u64::MAX, |&(next, _)| next);
            let span = until.min(to).saturating_sub(at.max(from));
            area += u128::from(span) * u128::from(unsent);
        }
        area
    }

    /// The byte-times the wire spent serializing in `[from, to)`.
    pub fn busy_between(&self, from: u64, to: u64) -> u64 {
        self.segments
            .iter()
            .map(|segment| segment.end.min(to).saturating_sub(segment.start.max(from)))
            .sum()
    }
}

/// What a driven session did on the link.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Report {
    /// When each side's future completed, in [`Side::BOTH`] order.
    pub completed: [u64; 2],
    /// What each direction carried, indexed by the writing side's
    /// [`Side::index`].
    pub directions: [DirectionReport; 2],
}

impl Report {
    /// When the later side completed: the session's duration.
    pub fn duration(&self) -> u64 {
        self.completed[0].max(self.completed[1])
    }

    /// What `side` wrote.
    pub fn sent_by(&self, side: Side) -> &DirectionReport {
        &self.directions[side.index()]
    }
}

/// Records whether a task asked to be polled again.
#[derive(Debug, Default)]
struct WakeFlag(AtomicBool);

/// Turns a wake into a flag the driver reads.
impl Wake for WakeFlag {
    /// Records the wake.
    fn wake(self: Arc<Self>) {
        self.0.store(true, Ordering::Release);
    }

    /// Records the wake.
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.store(true, Ordering::Release);
    }
}

/// Polls `a` and `b` over `link` until both complete, advancing the link's
/// clock whenever neither can progress.
///
/// `a` should use [`Side::A`]'s halves and `b` [`Side::B`]'s. Each round
/// polls every side that was woken, in [`Side::BOTH`] order, and a side's
/// future is dropped as soon as it completes. When no side was woken, the
/// clock moves to the link's next event, whose effects wake whichever side
/// waits on them.
///
/// # Panics
///
/// Panics if a side still runs while no side was woken and no event is
/// pending: the session deadlocked.
pub fn drive<A: Future, B: Future>(link: &Link, a: A, b: B) -> (A::Output, B::Output, Report) {
    let flags = [Arc::new(WakeFlag::default()), Arc::new(WakeFlag::default())];
    let wakers = flags.clone().map(Waker::from);
    let mut a = pin!(Some(a));
    let mut b = pin!(Some(b));
    let (mut a_out, mut b_out) = (None, None);
    let mut completed = [0; 2];
    for flag in &flags {
        flag.0.store(true, Ordering::Release);
    }
    while a_out.is_none() || b_out.is_none() {
        let mut polled = false;
        for side in Side::BOTH {
            if !flags[side.index()].0.swap(false, Ordering::AcqRel) {
                continue;
            }
            let mut cx = Context::from_waker(&wakers[side.index()]);
            let finished = match side {
                Side::A => poll_slot(a.as_mut(), &mut cx, &mut a_out),
                Side::B => poll_slot(b.as_mut(), &mut cx, &mut b_out),
            };
            polled |= finished.is_some();
            if finished == Some(true) {
                completed[side.index()] = link.now();
            }
        }
        if polled {
            continue;
        }
        assert!(
            link.advance(),
            "both peers wait with no event pending on the link: the session deadlocked",
        );
    }
    (
        a_out.expect("the loop ends once A completes"),
        b_out.expect("the loop ends once B completes"),
        Report {
            completed,
            directions: link.reports(),
        },
    )
}

/// Polls the future in `slot`, if it has not completed; on completion,
/// stores its output in `out` and drops the future in place.
///
/// Returns `None` if the slot was already empty, and otherwise whether the
/// future completed.
fn poll_slot<F: Future>(
    mut slot: Pin<&mut Option<F>>,
    cx: &mut Context<'_>,
    out: &mut Option<F::Output>,
) -> Option<bool> {
    let future = slot.as_mut().as_pin_mut()?;
    match future.poll(cx) {
        Poll::Ready(output) => {
            *out = Some(output);
            slot.set(None);
            Some(true)
        }
        Poll::Pending => Some(false),
    }
}
