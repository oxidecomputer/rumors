//! The conformance suite validated against the in-memory instantiation —
//! including adversarial-but-legal variants the contract admits.

use std::collections::{HashMap, VecDeque};
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};

use futures::future::BoxFuture;
use tokio::io::{AsyncRead, AsyncWrite, DuplexStream, ReadBuf};
use tokio::sync::mpsc;

use crate::link::{Acceptor, Connector, Link, LinkParts, MemoryLink, memory, memory_with_capacity};
use crate::testing::{Quiescence, run_to_quiescence};

/// The reference instantiation passes the whole suite under the
/// deterministic closed-world driver.
#[test]
fn memory_link_conforms() {
    run_to_quiescence(super::check(async || memory())).expect("the suite stays live");
}

/// A one-byte-buffered instantiation is legal: tiny windows slow streams
/// down but violate no clause, and full sessions still converge over it.
#[test]
fn one_byte_windows_conform() {
    run_to_quiescence(super::check(async || memory_with_capacity(1)))
        .expect("the suite stays live at one-byte windows");
}

/// An acceptor that delivers arrivals in batches of reversed order.
///
/// Legal under the contract — arrival order is whatever the transport says
/// it is, and no cross-stream ordering may be assumed — so the protocol
/// must tolerate it: the session's claim table pairs streams by label, not
/// position.
struct ReversingAcceptor<A: Acceptor> {
    inner: A,
    held: VecDeque<A::Rx>,
    /// Arrivals buffered before each reversed release.
    batch: usize,
}

impl<A: Acceptor> Acceptor for ReversingAcceptor<A> {
    type Rx = A::Rx;

    async fn accept(&mut self) -> io::Result<Self::Rx> {
        if let Some(held) = self.held.pop_front() {
            return Ok(held);
        }
        // Await one arrival, then swallow whatever else is immediately
        // ready — without blocking, so a lone stream still flows — and
        // release the accumulated batch newest-first.
        let first = self.inner.accept().await?;
        self.held.push_front(first);
        for _ in 1..self.batch {
            let mut next = std::pin::pin!(self.inner.accept());
            let waker = futures::task::noop_waker();
            let mut cx = std::task::Context::from_waker(&waker);
            match std::future::Future::poll(next.as_mut(), &mut cx) {
                std::task::Poll::Ready(Ok(rx)) => self.held.push_front(rx),
                // Pending or errored: stop batching; a real error resurfaces
                // from the next accept call.
                _ => break,
            }
        }
        Ok(self.held.pop_front().expect("at least one arrival is held"))
    }
}

/// Reorder one memory end's arrivals in reversed batches.
fn reversing(
    link: MemoryLink,
    batch: usize,
) -> Link<
    tokio::io::DuplexStream,
    tokio::io::DuplexStream,
    crate::link::MemoryConnector,
    ReversingAcceptor<crate::link::MemoryAcceptor>,
> {
    let parts = link.into_parts();
    LinkParts {
        control_read: parts.control_read,
        control_write: parts.control_write,
        connector: parts.connector,
        acceptor: ReversingAcceptor {
            inner: parts.acceptor,
            held: VecDeque::new(),
            batch,
        },
        session: parts.session,
    }
    .into_link()
}

/// Worst-case accept reordering is adversarial but legal: streams are
/// anonymous and arrival order is the transport's own, so the whole suite
/// — the focused probes included — must stay live and convergent under it.
#[test]
fn reordered_accepts_conform() {
    run_to_quiescence(super::check(async || {
        let (a, b) = memory();
        (reversing(a, 3), reversing(b, 3))
    }))
    .expect("the suite stays live under reordered accepts");
}

/// An acceptor that internally dequeues a delivery, then awaits once more
/// before returning it.
///
/// This is the router-helper shape `design/streaming-wire-deadlock.md`
/// §8.5 warns about: if the `accept` future is dropped between the internal
/// dequeue and the final yield — exactly what session teardown does to a
/// pending accept — the dequeued stream is dropped with it. The delivery is
/// silently lost while the link stays healthy, violating the contract's
/// cancellation clause.
struct LossyAcceptor<A: Acceptor> {
    inner: A,
}

impl<A: Acceptor> Acceptor for LossyAcceptor<A> {
    type Rx = A::Rx;

    async fn accept(&mut self) -> io::Result<Self::Rx> {
        let rx = self.inner.accept().await?;
        // The loss window: one self-waking yield with the dequeued stream
        // held only in this future's state.
        let mut yielded = false;
        std::future::poll_fn(|cx| {
            if std::mem::replace(&mut yielded, true) {
                Poll::Ready(())
            } else {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        })
        .await;
        Ok(rx)
    }
}

/// Wrap one memory end's acceptor in the lossy dequeue-then-await shape.
fn lossy(
    link: MemoryLink,
) -> Link<
    DuplexStream,
    DuplexStream,
    crate::link::MemoryConnector,
    LossyAcceptor<crate::link::MemoryAcceptor>,
> {
    let parts = link.into_parts();
    LinkParts {
        control_read: parts.control_read,
        control_write: parts.control_write,
        connector: parts.connector,
        acceptor: LossyAcceptor {
            inner: parts.acceptor,
        },
        session: parts.session,
    }
    .into_link()
}

// ─── The shared-FIFO mux: the deleted architecture, rebuilt as a fixture ────
//
// A minimal reconstruction of the design doc's §2 wire: every stream's
// frames ride one shared ordered FIFO per direction, and a single reader
// (driven cooperatively by whoever needs data) distributes them into small
// bounded per-stream queues. Routing into a full queue blocks the shared
// reader — the head-of-line coupling that deadlocked the streaming protocol
// and that the independence clause exists to exclude. The fixture violates
// the contract by construction; the suite must catch it.

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
    accepted: VecDeque<mpsc::Receiver<Vec<u8>>>,
}

/// The transport failure every mux operation maps peer loss to.
fn mux_gone() -> io::Error {
    io::Error::new(io::ErrorKind::BrokenPipe, "peer link is gone")
}

/// Pump one frame from the shared FIFO into its per-stream queue.
///
/// The demux lock is held across the routing send: this is the §2 coupling,
/// reproduced deliberately. When one stream's queue is full, the sole
/// shared reader parks on it, and every other stream's delivery waits
/// behind the parked frame.
async fn mux_pump(demux: &Arc<tokio::sync::Mutex<MuxDemux>>) -> io::Result<()> {
    let mut state = demux.lock().await;
    let Some(frame) = state.wire.recv().await else {
        return Err(mux_gone());
    };
    match frame {
        MuxFrame::Open(id) => {
            let (into_queue, queue) = mpsc::channel(MUX_STREAM_QUEUE);
            state.queues.insert(id, into_queue);
            state.accepted.push_back(queue);
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

    async fn connect(&self) -> io::Result<MuxTx> {
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
        Ok(MuxTx {
            id,
            wire: self.wire.clone(),
            close: Some(close),
            in_flight: None,
            claimed: 0,
        })
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

    async fn accept(&mut self) -> io::Result<MuxRx> {
        loop {
            {
                let mut state = self.demux.lock().await;
                if let Some(queue) = state.accepted.pop_front() {
                    return Ok(MuxRx {
                        queue,
                        demux: self.demux.clone(),
                        buffer: Vec::new(),
                        cursor: 0,
                        pump: None,
                        ended: false,
                    });
                }
            }
            mux_pump(&self.demux).await?;
        }
    }
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
            accepted: VecDeque::new(),
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

// ─── Regressions: the suite's former soundness holes, kept caught ───────────
//
// Each fixture above was first committed pinned to the suite's *unsound*
// behavior (the mux and the lossy acceptor passed; the legal reordering
// acceptor hung the independence check). The assertions below are those
// same fixtures flipped by the strengthened checks, kept as permanent
// proof that the suite catches what it once admitted.

/// Regression (hole H2b): the shared-FIFO mux — the deleted architecture of
/// `design/streaming-wire-deadlock.md` §2 — is caught by the independence
/// probe.
///
/// The stalled stream's sustained writes fill its small per-stream queue,
/// the shared reader parks routing the next stalled frame, live deliveries
/// wedge behind it, and the deterministic harness witnesses the stall. The
/// original probe wrote one stalled byte once, which any per-stream buffer
/// absorbed: the mux passed.
#[test]
fn shared_mux_coupling_is_caught() {
    let (a, b) = mux_pair();
    assert_eq!(
        run_to_quiescence(super::check_independence(a, b)),
        Err(Quiescence::Stalled),
        "the mux's head-of-line coupling must surface as a stall",
    );
}

/// Regression (hole H2a): a conforming reordering acceptor passes the
/// independence probe.
///
/// Streams are classified by their in-band first-byte tag, never by the
/// order the acceptor yields them. The original probe assumed the first
/// accept yielded the stalled stream, so this legal acceptor hung it.
#[test]
fn reordering_acceptor_passes_independence() {
    let (a, b) = memory();
    run_to_quiescence(super::check_independence(reversing(a, 3), reversing(b, 3)))
        .expect("independence stays live under reordered accepts");
}

/// Regression (hole H2c): the lossy dequeue-then-await acceptor is caught
/// by the cancellation probe.
///
/// With deliveries in flight, a poll-once-then-drop cycle catches the
/// acceptor holding a dequeued stream in the cancelled future; the stream
/// is lost with it and the collecting accept never resolves. The original
/// check only dropped an accept polled before any delivery existed, so
/// nothing could be lost and this acceptor passed.
#[test]
fn lossy_accept_cancellation_is_caught() {
    let (a, b) = memory();
    assert_eq!(
        run_to_quiescence(super::check_accept_cancellation(a, lossy(b))),
        Err(Quiescence::Stalled),
        "the lost delivery must surface as a stall at the collecting accept",
    );
}

/// Regression (hole H2d): a violation confined to the b-to-a direction is
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
