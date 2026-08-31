//! The conformance suite validated in both directions.
//!
//! The in-memory instantiation passes — including adversarial-but-legal
//! variants the contract admits — and negative controls prove the suite
//! still catches what it claims to (see the negative-controls section:
//! contract-violating fixtures asserted to *fail* the checks).

use std::collections::{HashMap, VecDeque};
use std::io;
use std::pin::{Pin, pin};
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
use crate::testing::{Quiescence, run_to_quiescence};

/// The reference instantiation passes the whole suite under the
/// deterministic closed-world driver.
#[test]
fn memory_link_conforms() {
    run_to_quiescence(super::check(async || memory())).expect("the suite stays live");
}

/// One-byte stream buffers are legal: window size affects latency, never
/// conformance. The full suite passes at capacity one.
#[test]
fn one_byte_windows_conform() {
    run_to_quiescence(super::check(async || memory_with_capacity(1)))
        .expect("the suite stays live at one-byte windows");
}

/// An acceptor that delivers arrivals in batches of reversed order.
///
/// Legal under the contract — arrival order is the transport's own, and
/// no cross-stream ordering may be assumed — so the protocol must
/// tolerate it: the session's claim table pairs streams by label, not
/// position. Each released batch of two or more is a genuine inversion,
/// counted into the shared `reordered` counter; tests assert it is
/// nonzero, so degeneration to pass-through (reordering nothing) fails
/// loudly instead of silently.
struct ReversingAcceptor<A: Acceptor> {
    inner: A,
    held: VecDeque<(A::Rx, Done<A::Rx>)>,
    /// Arrivals buffered before each reversed release.
    batch: usize,
    /// Batches of two or more released: genuine inversions.
    reordered: Arc<AtomicUsize>,
}

impl<A: Acceptor> Acceptor for ReversingAcceptor<A> {
    type Rx = A::Rx;

    async fn accept(&mut self) -> io::Result<(Self::Rx, Done<Self::Rx>)> {
        if let Some(held) = self.held.pop_front() {
            return Ok(held);
        }
        // Await one arrival, then swallow whatever else is immediately
        // ready — without blocking, so a lone stream still flows — and
        // release the accumulated batch newest-first.
        let first = self.inner.accept().await?;
        self.held.push_front(first);
        for _ in 1..self.batch {
            let mut next = pin!(self.inner.accept());
            let waker = futures::task::noop_waker();
            let mut cx = Context::from_waker(&waker);
            match next.as_mut().poll(&mut cx) {
                Poll::Ready(Ok(rx)) => self.held.push_front(rx),
                // Pending or errored: stop batching and release what is
                // held. Swallowing an error here is sound for the wrapped
                // `MemoryAcceptor`, whose errors are persistent (a closed
                // channel errors on every later recv, so the next accept
                // resurfaces it); the fixture is not built for acceptors
                // with one-shot errors.
                _ => break,
            }
        }
        if self.held.len() > 1 {
            self.reordered.fetch_add(1, Ordering::Relaxed);
        }
        Ok(self.held.pop_front().expect("at least one arrival is held"))
    }
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

/// Reorder one memory end's arrivals in reversed batches, counting genuine
/// inversions into `reordered`.
fn reversing(
    link: MemoryLink,
    batch: usize,
    reordered: Arc<AtomicUsize>,
) -> Link<DuplexStream, DuplexStream, MemoryConnector, ReversingAcceptor<MemoryAcceptor>> {
    with_acceptor(link, |inner| ReversingAcceptor {
        inner,
        held: VecDeque::new(),
        batch,
        reordered,
    })
}

/// Worst-case accept reordering is adversarial but legal: streams are
/// anonymous and arrival order is the transport's own, so the whole suite
/// — the focused probes included — must stay live and convergent under it.
///
/// The final assertion proves the adversity fired: at least one batch was
/// genuinely released in inverted order somewhere across the suite, so a
/// pass certifies tolerance of real reordering, not of a decorator that
/// silently degenerated to pass-through.
#[test]
fn reordered_accepts_conform() {
    let reordered = Arc::new(AtomicUsize::new(0));
    let counter = reordered.clone();
    run_to_quiescence(super::check(async || {
        let (a, b) = memory();
        (
            reversing(a, 3, counter.clone()),
            reversing(b, 3, counter.clone()),
        )
    }))
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
    inner: A,
}

impl<A: Acceptor> Acceptor for LossyAcceptor<A> {
    type Rx = A::Rx;

    async fn accept(&mut self) -> io::Result<(Self::Rx, Done<Self::Rx>)> {
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
    with_acceptor(link, |inner| LossyAcceptor { inner })
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

/// Shared coupling state for one side's control halves: while the write
/// half is blocked, the read half parks.
struct CoupledControl {
    write_blocked: bool,
    parked_read: Option<Waker>,
}

/// The read half of a direction-coupled control stream; see [`coupled`].
struct CoupledRead<R> {
    inner: R,
    state: Arc<Mutex<CoupledControl>>,
}

impl<R: AsyncRead + Unpin> AsyncRead for CoupledRead<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        {
            let mut state = self.state.lock().expect("coupling state lock");
            if state.write_blocked {
                state.parked_read = Some(cx.waker().clone());
                return Poll::Pending;
            }
        }
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

/// The write half of a direction-coupled control stream; see [`coupled`].
struct CoupledWrite<W> {
    inner: W,
    state: Arc<Mutex<CoupledControl>>,
}

impl<W> CoupledWrite<W> {
    /// Record the write half's disposition and wake a parked read when the
    /// write unblocks.
    fn record(&self, blocked: bool) {
        let mut state = self.state.lock().expect("coupling state lock");
        state.write_blocked = blocked;
        if !blocked && let Some(waker) = state.parked_read.take() {
            waker.wake();
        }
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for CoupledWrite<W> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let result = Pin::new(&mut self.inner).poll_write(cx, buf);
        self.record(result.is_pending());
        result
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let result = Pin::new(&mut self.inner).poll_flush(cx);
        self.record(result.is_pending());
        result
    }

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
        parked_read: None,
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

// ─── Negative controls: every check proves its teeth ────────────────────────
//
// Each violating fixture must FAIL its check, and each legal-adversity
// fixture must pass with its adversity proven fired. A check whose negative
// control stops failing has lost its teeth — these assertions are what make
// a green suite mean anything.

/// Negative control: the shared-FIFO mux — the head-of-line architecture
/// the independence clause exists to exclude — is caught by the
/// independence probe.
///
/// The stalled stream's sustained writes fill its small per-stream queue,
/// the shared reader parks routing the next stalled frame, live deliveries
/// wedge behind it, and the deterministic harness witnesses the stall.
/// Sustained pressure is what makes the catch: a single unread byte would
/// sit absorbed in the per-stream queue and the coupling would stay hidden.
#[test]
fn shared_mux_coupling_is_caught() {
    let (a, b) = mux_pair();
    assert_eq!(
        run_to_quiescence(super::check_independence(a, b)),
        Err(Quiescence::Stalled),
        "the mux's head-of-line coupling must surface as a stall",
    );
}

/// Legal adversity: a conforming reordering acceptor passes the
/// independence probe.
///
/// Streams are classified by their in-band first-byte tag, never by the
/// order the acceptor yields them — a probe that assumed the stalled
/// stream arrived first would hang against this legal acceptor. The final
/// assertion proves the reordering genuinely fired — the probe's
/// concurrently connected streams queue at the acceptor, so batches form —
/// rather than the decorator degenerating to pass-through.
#[test]
fn reordering_acceptor_passes_independence() {
    let reordered = Arc::new(AtomicUsize::new(0));
    let (a, b) = memory();
    run_to_quiescence(super::check_independence(
        reversing(a, 3, reordered.clone()),
        reversing(b, 3, reordered.clone()),
    ))
    .expect("independence stays live under reordered accepts");
    assert!(
        reordered.load(Ordering::Relaxed) > 0,
        "the reordering adversity never fired: every batch released a lone stream",
    );
}

/// Negative control: the lossy dequeue-then-await acceptor is caught by
/// the cancellation probe.
///
/// With deliveries in flight, a poll-once-then-drop cycle catches the
/// acceptor holding a dequeued stream in the cancelled future; the stream
/// is lost with it and the collecting accept never resolves. Deliveries
/// must be genuinely in flight for the catch: a dropped accept that never
/// made internal progress has nothing to lose.
#[test]
fn lossy_accept_cancellation_is_caught() {
    let (a, b) = memory();
    assert_eq!(
        run_to_quiescence(super::check_accept_cancellation(a, lossy(b))),
        Err(Quiescence::Stalled),
        "the lost delivery must surface as a stall at the collecting accept",
    );
}

/// Negative control: a violation confined to the b-to-a direction is
/// still caught.
///
/// Every focused probe runs a role-swapped second pass, so a lossy
/// acceptor on the a side — untouched by the a-to-b pass — hangs the
/// cancellation probe's reverse direction.
#[test]
fn asymmetric_lossiness_is_caught() {
    let (a, b) = memory();
    assert_eq!(
        run_to_quiescence(super::check_accept_cancellation(lossy(a), b)),
        Err(Quiescence::Stalled),
        "the reverse-direction pass must catch the a-side acceptor",
    );
}

/// Negative control: a control carrier whose read parks while its own
/// write is blocked — the direction coupling the control-duplex clause
/// forbids — is caught by the control-duplex probe as a stall.
///
/// Both sides are coupled: once each side's fill overruns the small pipes,
/// each write blocks, each read parks behind its own side's write, and no
/// waker stays live — the deterministic harness witnesses the deadlock the
/// clause exists to exclude.
#[test]
fn coupled_control_duplex_is_caught() {
    let (a, b) = memory_with_capacity(COUPLED_CONTROL_CAPACITY);
    assert_eq!(
        run_to_quiescence(super::check_control_duplex(coupled(a), coupled(b))),
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

/// Negative control: a budget pooled across streams — connection-level
/// flow control — sized below the buffering it must cover is caught by
/// the independence check's pooled shape as a stall.
///
/// The pressured stalled complement absorbs the whole pool unread, the
/// live stream's write finds the window empty, and no reader remains to
/// release it: the deterministic harness witnesses the starvation. The
/// single-stalled shape alone cannot catch this fixture (one stream's
/// unread bytes are capped by its own pipe, leaving this pool headroom),
/// which is why the pooled shape exists.
#[test]
fn pooled_budget_below_the_bound_is_caught() {
    let (a, b) = windowed_pair(POOLED_BINDING_BUDGET, POOLED_STREAM_CAPACITY);
    assert_eq!(
        run_to_quiescence(super::check_independence(a, b)),
        Err(Quiescence::Stalled),
        "a pooled budget below the buffering it must cover must surface as a stall",
    );
}

/// A pooled budget at the contract's never-binding bound conforms.
///
/// With (STREAM_COUNT + 1) per-stream buffers of headroom per direction,
/// no stream's unread bytes can make the pool the binding constraint, so
/// every clause reduces to the per-stream case and the whole suite —
/// pooled independence shape included — passes.
#[test]
fn never_binding_pooled_budget_conforms() {
    let budget = (STREAM_COUNT + 1) * POOLED_STREAM_CAPACITY;
    run_to_quiescence(super::check(async || {
        windowed_pair(budget, POOLED_STREAM_CAPACITY)
    }))
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
        let newcomer = joined
            .expect("the bootstrap session completes")
            .expect("the seed serves the bootstrap")
            .sync_window_floor()
            .into_rumors();
        {
            seed.batch(|batch| {
                for payload in 0..2048u64 {
                    batch.send(payload)?;
                }
                Ok::<(), crate::message::EncodeError>(())
            })
            .expect("flat test payloads are within any depth limit");
        }
        {
            newcomer
                .batch(|batch| {
                    for payload in 2048..4096u64 {
                        batch.send(payload)?;
                    }
                    Ok::<(), crate::message::EncodeError>(())
                })
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

/// Negative control: a supply that caps concurrent streams below the
/// complement is caught by the concurrency probe as a stall at the capped
/// open.
///
/// The probe holds every opened stream alive, so the fixture's fifth open
/// waits forever on a permit no drop will release; with the acceptor
/// likewise parked, no waker stays live and the harness reports the stall.
#[test]
fn capped_stream_supply_is_caught() {
    let (a, b) = memory();
    assert_eq!(
        run_to_quiescence(super::check_concurrency(capped(a), b)),
        Err(Quiescence::Stalled),
        "a stream cap below the complement must surface as a stall",
    );
}
