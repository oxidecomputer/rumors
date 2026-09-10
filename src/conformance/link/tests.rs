//! Conforming transports must pass; deliberately faulty transports must fail.

use std::collections::{HashMap, VecDeque};
use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use futures::future::BoxFuture;
use tokio::io::{AsyncRead, AsyncWrite, DuplexStream, ReadBuf};
use tokio::sync::mpsc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::link::{
    Acceptor, Connector, Done, Link, LinkParts, MemoryAcceptor, MemoryConnector, MemoryLink,
    STREAM_COUNT, memory, memory_with_capacity,
};
use crate::testing::{Quiescence, reorder_accepts, run_to_quiescence};

mod coverage;

/// The memory link passes the whole suite under the deterministic driver.
#[test]
fn memory_link_conforms() {
    run_to_quiescence(super::check(async || memory(), std::future::pending))
        .expect("the suite stays live");
}

/// One-byte stream buffers are legal: window size affects latency, never
/// conformance. The full suite passes at capacity one.
#[test]
fn one_byte_windows_conform() {
    run_to_quiescence(super::check(
        async || memory_with_capacity(1),
        std::future::pending,
    ))
    .expect("the suite stays live at one-byte windows");
}

/// Decorate one memory end's acceptor, preserving every other part — the
/// session state included, so the wrapped link stays in lockstep with its
/// peer.
fn with_acceptor<A: Acceptor>(
    link: MemoryLink,
    wrap: impl FnOnce(MemoryAcceptor) -> A,
) -> Link<DuplexStream, DuplexStream, MemoryConnector, A> {
    let parts = link.into_parts();
    LinkParts {
        control_read: parts.control_read,
        control_write: parts.control_write,
        connector: parts.connector,
        acceptor: wrap(parts.acceptor),
        session: parts.session,
    }
    .into_link()
}

/// Arrivals the reordering acceptor holds before each newest-first
/// release: deep enough to invert bursts, small enough never to starve a
/// lone stream.
const REORDER_BATCH: usize = 3;

/// Worst-case accept reordering is adversarial but legal: streams are
/// anonymous and arrival order is the transport's own, so the whole suite
/// — the focused probes included — must stay live and convergent under it.
///
/// The adversity is the crate's `ReorderingAcceptor`, which holds each
/// arrival and waits a bounded budget of yields for company before
/// releasing the batch newest-first. The final assertion proves it fired
/// somewhere across the suite, so a pass certifies tolerance of real
/// reordering, not of a decorator degenerated to pass-through.
#[test]
fn reordered_accepts_conform() {
    let reordered = Arc::new(AtomicUsize::new(0));
    let counter = reordered.clone();
    run_to_quiescence(super::check(
        async || {
            let (a, b) = memory();
            (
                reorder_accepts(a, REORDER_BATCH, counter.clone()),
                reorder_accepts(b, REORDER_BATCH, counter.clone()),
            )
        },
        std::future::pending,
    ))
    .expect("the suite stays live under reordered accepts");
    assert!(
        reordered.load(Ordering::Relaxed) > 0,
        "the reordering adversity never fired: every batch released a lone stream",
    );
}

/// An acceptor that internally dequeues a delivery, then awaits once more
/// before returning it.
///
/// This is the router-helper shape the cancellation clause exists to
/// exclude: if the `accept` future is dropped between the internal
/// dequeue and the final yield — exactly what session teardown does to a
/// pending accept — the dequeued stream is dropped with it. The delivery is
/// silently lost while the link stays healthy, violating the contract's
/// cancellation clause.
struct LossyAcceptor<A: Acceptor> {
    /// The actual stream supply.
    inner: A,
    /// Delay dequeue so cancellation can encounter it on different polls.
    before_dequeue: usize,
}

impl<A: Acceptor> Acceptor for LossyAcceptor<A> {
    /// The underlying read half.
    type Rx = A::Rx;

    /// Keep a dequeued stream in the cancellable future across a yield.
    async fn accept(&mut self) -> io::Result<(Self::Rx, Done<Self::Rx>)> {
        for _ in 0..self.before_dequeue {
            super::yield_once().await;
        }
        let rx = self.inner.accept().await?;
        // The loss window: one self-waking yield with the dequeued stream
        // held only in this future's state.
        super::yield_once().await;
        Ok(rx)
    }
}

/// Wrap one memory end's acceptor in the lossy dequeue-then-await shape.
fn lossy(
    link: MemoryLink,
) -> Link<DuplexStream, DuplexStream, MemoryConnector, LossyAcceptor<MemoryAcceptor>> {
    with_acceptor(link, |inner| LossyAcceptor {
        inner,
        before_dequeue: 0,
    })
}

// ─── The shared-FIFO mux: head-of-line coupling built as a fixture ──────────
//
// A minimal mux wire: every stream's frames ride one shared ordered FIFO
// per direction, and a single reader (driven cooperatively by whoever needs
// data) distributes them into small bounded per-stream queues. Routing into
// a full queue blocks the shared reader — the head-of-line coupling the
// independence clause exists to exclude. The fixture violates the contract
// by construction; the suite must catch it.

/// Frame capacity of each per-stream queue behind the mux's shared reader:
/// small, so a stalled stream's undelivered frames fill it quickly.
const MUX_STREAM_QUEUE: usize = 2;

/// Frame capacity of the shared wire FIFO all of one direction's streams
/// ride.
const MUX_WIRE_FRAMES: usize = 8;

/// Byte capacity of the mux link's control pipes.
const MUX_CONTROL_CAPACITY: usize = 1024;

/// One frame on the mux's shared FIFO.
enum MuxFrame {
    /// A new stream: the receiving demux allocates its bounded queue.
    Open(u64),
    /// A chunk of one stream's bytes.
    Data(u64, Vec<u8>),
    /// The writer is done: the queue drains, then reads observe
    /// end-of-stream.
    Close(u64),
}

/// The receiving side's shared state: the sole reader of one direction's
/// FIFO plus the per-stream queues it distributes into.
struct MuxDemux {
    wire: mpsc::Receiver<MuxFrame>,
    queues: HashMap<u64, mpsc::Sender<Vec<u8>>>,
    /// Streams announced but not yet accepted, in arrival order.
    announced: VecDeque<mpsc::Receiver<Vec<u8>>>,
}

/// The transport failure every mux operation maps peer loss to.
fn mux_gone() -> io::Error {
    io::Error::new(io::ErrorKind::BrokenPipe, "peer link is gone")
}

/// Pump one frame from the shared FIFO into its per-stream queue.
///
/// The demux lock is held across the routing send — deliberately: when one
/// stream's queue is full, the sole shared reader parks on it, and every
/// other stream's delivery waits behind the parked frame.
async fn mux_pump(demux: &Arc<tokio::sync::Mutex<MuxDemux>>) -> io::Result<()> {
    let mut state = demux.lock().await;
    let Some(frame) = state.wire.recv().await else {
        return Err(mux_gone());
    };
    match frame {
        MuxFrame::Open(id) => {
            let (into_queue, queue) = mpsc::channel(MUX_STREAM_QUEUE);
            state.queues.insert(id, into_queue);
            state.announced.push_back(queue);
        }
        MuxFrame::Data(id, bytes) => {
            if let Some(queue) = state.queues.get(&id).cloned() {
                // A dropped receiver surfaces as an error here; bytes for a
                // stream the peer discarded are simply dropped.
                let _ = queue.send(bytes).await;
            }
        }
        MuxFrame::Close(id) => {
            // Dropping the queue sender lets buffered chunks drain first;
            // the receiver then observes end-of-stream.
            state.queues.remove(&id);
        }
    }
    Ok(())
}

/// The mux link's connector: each open announces a stream id on the shared
/// FIFO.
#[derive(Clone)]
struct MuxConnector {
    wire: mpsc::Sender<MuxFrame>,
    next_id: Arc<AtomicU64>,
}

impl Connector for MuxConnector {
    type Tx = MuxTx;

    async fn connect(&self) -> io::Result<(MuxTx, Done<MuxTx>)> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        // Reserve the close frame's FIFO slot up front, so the drop-time
        // close can be sent synchronously and is never lost to a full FIFO.
        let close = self
            .wire
            .clone()
            .reserve_owned()
            .await
            .map_err(|_| mux_gone())?;
        self.wire
            .send(MuxFrame::Open(id))
            .await
            .map_err(|_| mux_gone())?;
        Ok((
            MuxTx {
                id,
                wire: self.wire.clone(),
                close: Some(close),
                in_flight: None,
                claimed: 0,
            },
            Done::discard(),
        ))
    }
}

/// The write half of one mux stream: each write becomes one FIFO frame.
struct MuxTx {
    id: u64,
    wire: mpsc::Sender<MuxFrame>,
    /// The pre-reserved slot the drop-time close frame rides.
    close: Option<mpsc::OwnedPermit<MuxFrame>>,
    /// A FIFO send holding a copy of `claimed` bytes of the caller's
    /// buffer. Futures are inert, so the copy is only committed once this
    /// resolves under `poll_write`, which then reports those bytes written.
    in_flight: Option<BoxFuture<'static, Result<(), ()>>>,
    claimed: usize,
}

impl AsyncWrite for MuxTx {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = &mut *self;
        loop {
            if let Some(send) = &mut this.in_flight {
                return match send.as_mut().poll(cx) {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(Err(())) => Poll::Ready(Err(mux_gone())),
                    Poll::Ready(Ok(())) => {
                        this.in_flight = None;
                        // The caller re-offers the bytes an earlier pending
                        // poll claimed (`write_all` never advances past a
                        // `Pending`), so the committed length is theirs.
                        debug_assert!(this.claimed <= buf.len());
                        Poll::Ready(Ok(this.claimed))
                    }
                };
            }
            let frame = MuxFrame::Data(this.id, buf.to_vec());
            let wire = this.wire.clone();
            this.claimed = buf.len();
            this.in_flight = Some(Box::pin(
                async move { wire.send(frame).await.map_err(|_| ()) },
            ));
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // Nothing buffers behind the FIFO send itself; flushing only means
        // driving an in-flight frame to the FIFO.
        match &mut self.in_flight {
            None => Poll::Ready(Ok(())),
            Some(send) => match send.as_mut().poll(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(Err(())) => Poll::Ready(Err(mux_gone())),
                Poll::Ready(Ok(())) => {
                    self.in_flight = None;
                    Poll::Ready(Ok(()))
                }
            },
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        if let Some(close) = self.close.take() {
            close.send(MuxFrame::Close(self.id));
        }
        Poll::Ready(Ok(()))
    }
}

impl Drop for MuxTx {
    fn drop(&mut self) {
        if let Some(close) = self.close.take() {
            close.send(MuxFrame::Close(self.id));
        }
    }
}

/// The read half of one mux stream: drains its bounded queue, driving the
/// shared demux reader whenever the queue is empty.
struct MuxRx {
    queue: mpsc::Receiver<Vec<u8>>,
    demux: Arc<tokio::sync::Mutex<MuxDemux>>,
    buffer: Vec<u8>,
    cursor: usize,
    /// The shared-reader drive this stream is currently blocked on, if any.
    pump: Option<BoxFuture<'static, io::Result<()>>>,
    ended: bool,
}

impl AsyncRead for MuxRx {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = &mut *self;
        loop {
            if this.cursor < this.buffer.len() {
                let n = buf.remaining().min(this.buffer.len() - this.cursor);
                buf.put_slice(&this.buffer[this.cursor..this.cursor + n]);
                this.cursor += n;
                return Poll::Ready(Ok(()));
            }
            if this.ended {
                return Poll::Ready(Ok(()));
            }
            match this.queue.poll_recv(cx) {
                Poll::Ready(Some(bytes)) => {
                    this.buffer = bytes;
                    this.cursor = 0;
                }
                Poll::Ready(None) => this.ended = true,
                Poll::Pending => {
                    let pump = this.pump.get_or_insert_with(|| {
                        let demux = this.demux.clone();
                        Box::pin(async move { mux_pump(&demux).await })
                    });
                    match pump.as_mut().poll(cx) {
                        Poll::Ready(result) => {
                            this.pump = None;
                            result?;
                        }
                        Poll::Pending => return Poll::Pending,
                    }
                }
            }
        }
    }
}

/// The mux link's acceptor: yields announced streams in arrival order,
/// driving the shared reader while none is pending.
struct MuxAcceptor {
    demux: Arc<tokio::sync::Mutex<MuxDemux>>,
}

impl Acceptor for MuxAcceptor {
    type Rx = MuxRx;

    async fn accept(&mut self) -> io::Result<(MuxRx, Done<MuxRx>)> {
        loop {
            {
                let mut state = self.demux.lock().await;
                if let Some(queue) = state.announced.pop_front() {
                    return Ok((
                        MuxRx {
                            queue,
                            demux: self.demux.clone(),
                            buffer: Vec::new(),
                            cursor: 0,
                            pump: None,
                            ended: false,
                        },
                        Done::discard(),
                    ));
                }
            }
            mux_pump(&self.demux).await?;
        }
    }
}

// ─── Fixtures violating the control-duplex and concurrency clauses ──────────

/// Faulty coupling to one side's blocked control writer.
struct CoupledControl {
    /// Whether the last control write or flush was pending.
    write_blocked: bool,
    /// A read or open incorrectly waiting on that write.
    parked: Option<Waker>,
}

/// The read half of a direction-coupled control stream; see [`coupled`].
struct CoupledRead<R> {
    /// The actual control reader.
    inner: R,
    /// Whether this read must incorrectly wait on its writer.
    state: Arc<Mutex<CoupledControl>>,
}

impl<R: AsyncRead + Unpin> AsyncRead for CoupledRead<R> {
    /// Park behind a blocked write, even if incoming bytes are available.
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        {
            let mut state = self.state.lock().expect("coupling state lock");
            if state.write_blocked {
                state.parked = Some(cx.waker().clone());
                return Poll::Pending;
            }
        }
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

/// The write half of a direction-coupled control stream; see [`coupled`].
struct CoupledWrite<W> {
    /// The actual control writer.
    inner: W,
    /// The operation incorrectly waiting for this writer to unblock.
    state: Arc<Mutex<CoupledControl>>,
}

impl<W> CoupledWrite<W> {
    /// Record whether writing blocked and wake the coupled operation when it clears.
    fn record(&self, blocked: bool) {
        let mut state = self.state.lock().expect("coupling state lock");
        state.write_blocked = blocked;
        if !blocked && let Some(waker) = state.parked.take() {
            waker.wake();
        }
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for CoupledWrite<W> {
    /// Write while recording backpressure for the coupled operation.
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let result = Pin::new(&mut self.inner).poll_write(cx, buf);
        self.record(result.is_pending());
        result
    }

    /// Flush while recording backpressure for the coupled operation.
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let result = Pin::new(&mut self.inner).poll_flush(cx);
        self.record(result.is_pending());
        result
    }

    /// Close the writer while recording whether shutdown blocked.
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let result = Pin::new(&mut self.inner).poll_shutdown(cx);
        self.record(result.is_pending());
        result
    }
}

/// Byte capacity of the coupled fixture's underlying pipes: small, so the
/// control-duplex probe's fill overruns it almost immediately and both
/// sides' writes block while their reads sit parked.
const COUPLED_CONTROL_CAPACITY: usize = 1024;

/// Couple one memory end's control directions: its read parks whenever its
/// own write is blocked, the coupling the control-duplex clause forbids.
fn coupled(
    link: MemoryLink,
) -> Link<CoupledRead<DuplexStream>, CoupledWrite<DuplexStream>, MemoryConnector, MemoryAcceptor> {
    let parts = link.into_parts();
    let state = Arc::new(Mutex::new(CoupledControl {
        write_blocked: false,
        parked: None,
    }));
    LinkParts {
        control_read: CoupledRead {
            inner: parts.control_read,
            state: state.clone(),
        },
        control_write: CoupledWrite {
            inner: parts.control_write,
            state,
        },
        connector: parts.connector,
        acceptor: parts.acceptor,
        session: parts.session,
    }
    .into_link()
}

/// A connector admitting at most [`CAPPED_STREAMS`] concurrently open
/// streams: each open waits for a permit only a dropped stream releases —
/// the cap the concurrency clause forbids.
struct CappedConnector<C> {
    inner: C,
    permits: Arc<Semaphore>,
}

impl<C: Clone> Clone for CappedConnector<C> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            permits: self.permits.clone(),
        }
    }
}

/// A stream write half pinning one of the capped fixture's permits for its
/// lifetime.
struct CappedTx<T> {
    inner: T,
    _permit: OwnedSemaphorePermit,
}

impl<T: AsyncWrite + Unpin> AsyncWrite for CappedTx<T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

impl<C: Connector> Connector for CappedConnector<C> {
    type Tx = CappedTx<C::Tx>;

    async fn connect(&self) -> io::Result<(Self::Tx, Done<Self::Tx>)> {
        let permit = self
            .permits
            .clone()
            .acquire_owned()
            .await
            .expect("the fixture semaphore is never closed");
        let (inner, _) = self.inner.connect().await?;
        Ok((
            CappedTx {
                inner,
                _permit: permit,
            },
            Done::discard(),
        ))
    }
}

/// Streams the capped fixture admits concurrently: far below the complement
/// the contract requires.
const CAPPED_STREAMS: usize = 4;

/// Cap one memory end's connector at [`CAPPED_STREAMS`] concurrent streams.
fn capped(
    link: MemoryLink,
) -> Link<DuplexStream, DuplexStream, CappedConnector<MemoryConnector>, MemoryAcceptor> {
    let parts = link.into_parts();
    LinkParts {
        control_read: parts.control_read,
        control_write: parts.control_write,
        connector: CappedConnector {
            inner: parts.connector,
            permits: Arc::new(Semaphore::new(CAPPED_STREAMS)),
        },
        acceptor: parts.acceptor,
        session: parts.session,
    }
    .into_link()
}

// ─── A shared connection window: QUIC/HTTP2-style cross-stream budget ───────
//
// One byte budget per direction covers every stream of that direction —
// control and data alike: writes consume it, and it is released only when
// the receiving side actually reads the bytes, so written-but-unread bytes
// on ANY stream shrink what every other stream may write. This is the
// connection-level flow control real multiplexed transports layer over
// per-stream windows; whether (and at what size) it violates the contract
// is what the shared-budget experiments below establish.

/// One direction's shared connection window.
struct SharedWindow {
    /// Bytes the direction may still write before readers release more.
    available: usize,
    /// Writers parked on an exhausted window.
    writers: Vec<Waker>,
}

/// Handle to one direction's shared connection window.
type Window = Arc<Mutex<SharedWindow>>;

/// Create one direction's window with `budget` bytes available.
fn window(budget: usize) -> Window {
    Arc::new(Mutex::new(SharedWindow {
        available: budget,
        writers: Vec::new(),
    }))
}

/// A write half charging every byte to its direction's shared window.
struct WindowedTx<W> {
    inner: W,
    window: Window,
}

impl<W: AsyncWrite + Unpin> AsyncWrite for WindowedTx<W> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let grant = {
            let mut window = self.window.lock().expect("window lock");
            if window.available == 0 {
                window.writers.push(cx.waker().clone());
                return Poll::Pending;
            }
            window.available.min(buf.len())
        };
        let result = Pin::new(&mut self.inner).poll_write(cx, &buf[..grant]);
        if let Poll::Ready(Ok(written)) = &result {
            let mut window = self.window.lock().expect("window lock");
            window.available -= written;
        }
        result
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// A read half releasing its direction's shared window as bytes are read.
struct WindowedRx<R> {
    inner: R,
    window: Window,
}

impl<R: AsyncRead + Unpin> AsyncRead for WindowedRx<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let before = buf.filled().len();
        let result = Pin::new(&mut self.inner).poll_read(cx, buf);
        if let Poll::Ready(Ok(())) = &result {
            let read = buf.filled().len() - before;
            if read > 0 {
                let mut window = self.window.lock().expect("window lock");
                window.available += read;
                for waker in window.writers.drain(..) {
                    waker.wake();
                }
            }
        }
        result
    }
}

/// The windowed connector: every opened stream's writes charge the shared
/// window.
#[derive(Clone)]
struct WindowedConnector {
    inner: MemoryConnector,
    window: Window,
}

impl Connector for WindowedConnector {
    type Tx = WindowedTx<DuplexStream>;

    async fn connect(&self) -> io::Result<(Self::Tx, Done<Self::Tx>)> {
        let (inner, _) = self.inner.connect().await?;
        Ok((
            WindowedTx {
                inner,
                window: self.window.clone(),
            },
            Done::discard(),
        ))
    }
}

/// The windowed acceptor: every accepted stream's reads release the shared
/// window.
struct WindowedAcceptor {
    inner: MemoryAcceptor,
    window: Window,
}

impl Acceptor for WindowedAcceptor {
    type Rx = WindowedRx<DuplexStream>;

    async fn accept(&mut self) -> io::Result<(Self::Rx, Done<Self::Rx>)> {
        let (inner, _) = self.inner.accept().await?;
        Ok((
            WindowedRx {
                inner,
                window: self.window.clone(),
            },
            Done::discard(),
        ))
    }
}

/// One end of a shared-window link pair.
type WindowedLink =
    Link<WindowedRx<DuplexStream>, WindowedTx<DuplexStream>, WindowedConnector, WindowedAcceptor>;

/// A connected pair whose every direction shares one `budget`-byte
/// connection window across its control stream and all data streams, over
/// per-stream pipes of `capacity` bytes.
fn windowed_pair(budget: usize, capacity: usize) -> (WindowedLink, WindowedLink) {
    let (a, b) = memory_with_capacity(capacity);
    let a = a.into_parts();
    let b = b.into_parts();
    let ab = window(budget);
    let ba = window(budget);
    let rebuild =
        |parts: LinkParts<DuplexStream, DuplexStream, MemoryConnector, MemoryAcceptor>,
         writes: &Window,
         reads: &Window| {
            LinkParts {
                control_read: WindowedRx {
                    inner: parts.control_read,
                    window: reads.clone(),
                },
                control_write: WindowedTx {
                    inner: parts.control_write,
                    window: writes.clone(),
                },
                connector: WindowedConnector {
                    inner: parts.connector,
                    window: writes.clone(),
                },
                acceptor: WindowedAcceptor {
                    inner: parts.acceptor,
                    window: reads.clone(),
                },
                session: parts.session,
            }
            .into_link()
        };
    (rebuild(a, &ab, &ba), rebuild(b, &ba, &ab))
}

/// One end of the shared-FIFO mux link.
type MuxLink = Link<DuplexStream, DuplexStream, MuxConnector, MuxAcceptor>;

/// Create a connected pair of shared-FIFO mux links.
fn mux_pair() -> (MuxLink, MuxLink) {
    let (a_control_write, b_control_read) = tokio::io::duplex(MUX_CONTROL_CAPACITY);
    let (b_control_write, a_control_read) = tokio::io::duplex(MUX_CONTROL_CAPACITY);
    let (a_wire, b_incoming) = mpsc::channel(MUX_WIRE_FRAMES);
    let (b_wire, a_incoming) = mpsc::channel(MUX_WIRE_FRAMES);
    let demux = |wire| {
        Arc::new(tokio::sync::Mutex::new(MuxDemux {
            wire,
            queues: HashMap::new(),
            announced: VecDeque::new(),
        }))
    };
    (
        Link::new(
            a_control_read,
            a_control_write,
            MuxConnector {
                wire: a_wire,
                next_id: Arc::new(AtomicU64::new(0)),
            },
            MuxAcceptor {
                demux: demux(a_incoming),
            },
        ),
        Link::new(
            b_control_read,
            b_control_write,
            MuxConnector {
                wire: b_wire,
                next_id: Arc::new(AtomicU64::new(0)),
            },
            MuxAcceptor {
                demux: demux(b_incoming),
            },
        ),
    )
}

/// Filling one unread stream must expose a mux that blocks all delivery
/// behind it. A single unread byte would fit in its queue and hide the bug.
#[test]
fn shared_mux_coupling_is_caught() {
    let (a, b) = mux_pair();
    assert_eq!(
        run_to_quiescence(super::check_independence(
            async || (a, b),
            std::future::pending
        )),
        Err(Quiescence::Stalled),
        "the mux's head-of-line coupling must surface as a stall",
    );
}

/// Reordering accepted streams must pass: the probe identifies each stream
/// by its tag. The final assertion ensures reordering actually occurred.
#[test]
fn reordering_acceptor_passes_independence() {
    let reordered = Arc::new(AtomicUsize::new(0));
    let (a, b) = memory();
    run_to_quiescence(super::check_independence(
        async || {
            (
                reorder_accepts(a, REORDER_BATCH, reordered.clone()),
                reorder_accepts(b, REORDER_BATCH, reordered.clone()),
            )
        },
        std::future::pending,
    ))
    .expect("independence stays live under reordered accepts");
    assert!(
        reordered.load(Ordering::Relaxed) > 0,
        "the reordering adversity never fired: every batch released a lone stream",
    );
}

/// Cancelling an accept after it dequeues a stream must expose the lost
/// delivery: a later accept stalls while waiting for that stream.
#[test]
fn lossy_accept_cancellation_is_caught() {
    let (a, b) = memory();
    assert_eq!(
        run_to_quiescence(super::check_accept_cancellation(
            async || (a, lossy(b)),
            std::future::pending
        )),
        Err(Quiescence::Stalled),
        "the lost delivery must surface as a stall at the collecting accept",
    );
}

/// Loss confined to the reverse direction must still fail the check.
#[test]
fn asymmetric_lossiness_is_caught() {
    let (a, b) = memory();
    assert_eq!(
        run_to_quiescence(super::check_accept_cancellation(
            async || (lossy(a), b),
            std::future::pending
        )),
        Err(Quiescence::Stalled),
        "the reverse-direction pass must catch the a-side acceptor",
    );
}

/// Control reads that wait for their own blocked writes must fail the probe.
/// Both pipes fill, so neither side can read or write and the driver stalls.
#[test]
fn coupled_control_duplex_is_caught() {
    let (a, b) = memory_with_capacity(COUPLED_CONTROL_CAPACITY);
    assert_eq!(
        run_to_quiescence(super::check_control_duplex(
            async || (coupled(a), coupled(b)),
            std::future::pending
        )),
        Err(Quiescence::Stalled),
        "direction-coupled control halves must surface as a stall",
    );
}

/// Per-stream pipe capacity of the pooled-budget fixtures.
const POOLED_STREAM_CAPACITY: usize = 1024;

/// A pooled budget deliberately far below the stalled complement's
/// combined buffering, so the independence check's pooled shape must
/// starve.
const POOLED_BINDING_BUDGET: usize = 4 * POOLED_STREAM_CAPACITY;

/// Several unread streams exhaust an undersized shared pool and block the
/// live stream. One unread stream alone cannot fill this fixture's pool.
#[test]
fn pooled_budget_below_the_bound_is_caught() {
    let (a, b) = windowed_pair(POOLED_BINDING_BUDGET, POOLED_STREAM_CAPACITY);
    assert_eq!(
        run_to_quiescence(super::check_independence(
            async || (a, b),
            std::future::pending
        )),
        Err(Quiescence::Stalled),
        "a pooled budget below the buffering it must cover must surface as a stall",
    );
}

/// A pool with room for every data stream and control buffer passes the suite.
/// Unread bytes cannot exhaust the pool before their individual buffers fill.
#[test]
fn never_binding_pooled_budget_conforms() {
    let budget = (STREAM_COUNT + 1) * POOLED_STREAM_CAPACITY;
    run_to_quiescence(super::check(
        async || windowed_pair(budget, POOLED_STREAM_CAPACITY),
        std::future::pending,
    ))
    .expect("the suite stays live at the never-binding pooled budget");
}

/// Real sessions stay live over a pooled budget starved far below the
/// contract's bound: degradation is latency, never deadlock.
///
/// This pins observed protocol behavior, deliberately stronger than the
/// contract (the link docs promise only the never-binding bound): a deep
/// reconciliation — thousands of payloads, a multi-level trie, frames in
/// flight on several streams at once — converges over a 64-byte pool per
/// direction shared by the control stream and every data stream.
///
/// The sessions run at the serialization floor, the shape the link docs'
/// measured-tolerance sentence is denominated in: a sub-bound pool couples
/// streams, so a window wide enough to fill several streams at once can
/// genuinely wait-cycle through it — exactly the coupling the contract's
/// independence clause exists to exclude. If a protocol change trips
/// this, floor sessions have begun *depending* on pooled headroom, and
/// that sentence must be re-derived before this pin is loosened.
#[test]
fn starved_pool_degrades_latency_not_liveness() {
    let (mut a, mut b) = windowed_pair(64, POOLED_STREAM_CAPACITY);
    run_to_quiescence(async {
        let seed: crate::Rumors<u64> = crate::Peer::seed().sync_window_floor().into_rumors();
        let (served, joined) = futures::future::join(
            seed.gossip(&mut a),
            crate::Peer::<u64>::bootstrap().join(&mut b),
        )
        .await;
        served.expect("the bootstrap-serving session completes");
        let newcomer = (match joined {
            crate::Joined::Joined { peer } => peer,
            _ => panic!("the seed serves the bootstrap"),
        })
        .sync_window_floor()
        .into_rumors();
        {
            seed.send_all(0..2048u64)
                .expect("flat test payloads are within any depth limit");
        }
        {
            newcomer
                .send_all(2048..4096u64)
                .expect("flat test payloads are within any depth limit");
        }
        let (near, far) = futures::future::join(seed.gossip(&mut a), newcomer.gossip(&mut b)).await;
        near.expect("gossip completes over the starved pool");
        far.expect("gossip completes over the starved pool");
        assert_eq!(
            seed.snapshot(),
            newcomer.snapshot(),
            "reconciliation over the starved pool converged on the same set",
        );
    })
    .expect("deep sessions stay live over a 64-byte pooled budget");
}

/// A stream limit below the contract's requirement must fail the probe.
/// The fifth open stalls because earlier streams still hold every permit.
#[test]
fn capped_stream_supply_is_caught() {
    let (a, b) = memory();
    assert_eq!(
        run_to_quiescence(super::check_concurrency(
            async || (capped(a), b),
            std::future::pending
        )),
        Err(Quiescence::Stalled),
        "a stream cap below the complement must surface as a stall",
    );
}
