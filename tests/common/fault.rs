//! Wire-fault injection for the disruption simulation: deterministic,
//! byte-budgeted severing of either direction of a gossip link.
//!
//! A "dropped connection" in the simulation is one or both directions of a
//! [`rumors::Link`] tripping at an arbitrary byte offset mid-session:
//!
//! - [`Fuse`] forwards writes until its budget is exhausted, then fails
//!   every write with [`BrokenPipe`] — the connection died under our pen.
//! - [`Cut`] forwards reads until its budget is exhausted, then fails every
//!   read with [`ConnectionReset`] — the connection died under our eyes.
//!
//! Each budget is shared across every stream of its direction -- the control
//! half and each data stream draw on one counter -- so the cut lands at a
//! chosen offset in the endpoint's total traffic, wherever that byte
//! happens to travel. A severed direction also refuses new streams: once
//! its budget is exhausted, [`FaultConnector::connect`] fails alongside the
//! writers and [`FaultAcceptor::accept`] alongside the readers, because a
//! dead connection cannot open or deliver streams any more than it can
//! carry bytes. The wrapped side observes the cut as an error; its
//! counterparty observes it as end-of-stream (and a truncated frame) once
//! the failing side's link drops, or as its own write error against the
//! closed transport. Either way the session dies somewhere the protocol did
//! not choose, which is exactly the disruption the simulation is after.
//!
//! A *vanish* is the other way a peer dies: not a wire error it can
//! observe, but the peer itself gone mid-protocol, as a crashed process
//! is. At its [`Vanish`] point the endpoint's session is dropped where it
//! stands, its link halves with it and without any shutdown, and every
//! stream it owes is never opened. Its counterparty sees exactly what a
//! vanished socket gives: end-of-stream on what was open, a refused open
//! toward the dead peer, and an accept that waits forever for a stream
//! nobody will open. [`drive`] runs a session under such a plan.
//!
//! [`BrokenPipe`]: std::io::ErrorKind::BrokenPipe
//! [`ConnectionReset`]: std::io::ErrorKind::ConnectionReset

use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use rumors::link::{
    Acceptor, Connector, Done, Link, LinkParts, MemoryAcceptor, MemoryConnector, MemoryLink,
};
use tokio::io::{AsyncRead, AsyncWrite, DuplexStream, ReadBuf};
use tokio::sync::Notify;

/// One endpoint's fault plan: byte budgets after which its write
/// (respectively read) direction fails, and the point at which the
/// endpoint vanishes. `None` means never.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FaultPlan {
    /// Bytes this endpoint may write before its writers fail.
    pub write_cut: Option<usize>,
    /// Bytes this endpoint may read before its readers fail.
    pub read_cut: Option<usize>,
    /// Where this endpoint vanishes, if it does.
    pub vanish: Option<Vanish>,
}

/// The point at which an endpoint vanishes: its session is dropped there
/// without any shutdown, its link halves with it, and any stream it owes
/// is never opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vanish {
    /// While writing the `index`-th data stream it opened, `offset` bytes
    /// into that stream.
    ///
    /// The next write there is where the endpoint dies, mid-frame like a
    /// cut. An endpoint that opens fewer streams, or writes no more than
    /// `offset` bytes on that one, never reaches the point.
    OnStream { index: usize, offset: usize },
    /// At its first outgoing stream open: after the handshake, which rides
    /// the control half, and before its first data stream.
    AtFirstConnect,
    /// While writing the control stream, after `offset` bytes.
    OnControl { offset: usize },
}

impl FaultPlan {
    /// A clean endpoint: neither direction ever fails, and it never
    /// vanishes.
    pub const NONE: Self = Self {
        write_cut: None,
        read_cut: None,
        vanish: None,
    };
}

/// The faulted shape of one in-memory link endpoint.
pub type FaultyLink = Link<
    Cut<DuplexStream>,
    Fuse<DuplexStream>,
    FaultConnector<MemoryConnector>,
    FaultAcceptor<MemoryAcceptor>,
>;

/// Wrap one in-memory link endpoint in `plan`'s budgets.
///
/// A clean plan still wraps (with effectively-infinite budgets), so every
/// call site handles one pair of types regardless of whether it faults.
///
/// # Panics
///
/// If the plan vanishes: a vanishing endpoint's session must be driven by
/// [`drive`], which owns dropping it at the point.
pub fn faulty(link: MemoryLink, plan: FaultPlan) -> FaultyLink {
    assert!(
        plan.vanish.is_none(),
        "a vanishing endpoint's session is driven by `drive`"
    );
    wrap(link, plan).0
}

/// What [`drive`] leaves behind.
///
/// Keep it alive until the counterparty's session has ended: after a
/// vanish it holds the vanished endpoint's stream supply open, so the
/// counterparty's accepts wait as they would on a dead peer's listener
/// instead of erroring on a closed supply.
pub struct Driven<Out> {
    /// The session's outcome, or `None` if the endpoint vanished first.
    pub outcome: Option<Out>,
    _supply: Option<MemoryConnector>,
}

impl<Out> Driven<Out> {
    /// Whether the endpoint reached its vanish point.
    pub fn vanished(&self) -> bool {
        self.outcome.is_none()
    }
}

/// Run `session` over `link` under `plan`, vanishing at the plan's point.
///
/// Without a vanish this is `session` over [`faulty`]'s link. With one,
/// the session is polled until the endpoint reaches its point, where its
/// session future and its link halves are dropped with no shutdown: the
/// counterparty then reads end-of-stream on the control half and on every
/// open stream, its opens toward the vanished peer fail, and its accepts
/// wait for streams that never come (see [`Driven`]).
pub async fn drive<Out>(
    link: MemoryLink,
    plan: FaultPlan,
    session: impl AsyncFnOnce(&mut FaultyLink) -> Out,
) -> Driven<Out> {
    let (mut link, vanish, supply) = wrap(link, plan);
    let Some(vanish) = vanish else {
        return Driven {
            outcome: Some(session(&mut link).await),
            _supply: None,
        };
    };
    let outcome = tokio::select! {
        biased;
        outcome = session(&mut link) => Some(outcome),
        () = vanish.vanished() => None,
    };
    let vanished = outcome.is_none();
    drop(link);
    Driven {
        outcome,
        _supply: vanished.then_some(supply),
    }
}

/// Observer for the bytes one endpoint has moved through its fault
/// wrappers: the same counters a [`FaultPlan`]'s cuts spend, read out as
/// totals.
///
/// This is how a measurement establishes where in an endpoint's
/// byte stream a cut offset can land — the meter and the cut draw on
/// identical accounting, so a measured extent is directly a bound on
/// meaningful cut offsets.
pub struct ByteMeter {
    write: Budget,
    read: Budget,
    streams: Streams,
}

impl ByteMeter {
    /// Total bytes the endpoint has written, across the control half and
    /// every data stream.
    pub fn written(&self) -> usize {
        usize::MAX - *self.write.lock().expect("write budget lock")
    }

    /// Total bytes the endpoint has read, across the control half and
    /// every data stream: the counter a `read_cut` spends.
    pub fn read(&self) -> usize {
        usize::MAX - *self.read.lock().expect("read budget lock")
    }

    /// Data streams the endpoint has opened: the ordinals a
    /// [`Vanish::OnStream`] can name.
    pub fn streams_opened(&self) -> usize {
        self.streams.lock().expect("stream ledger lock").len()
    }

    /// Most bytes the endpoint wrote on any one data stream: the offsets
    /// a [`Vanish::OnStream`] can reach.
    pub fn widest_stream(&self) -> usize {
        self.streams
            .lock()
            .expect("stream ledger lock")
            .iter()
            .copied()
            .max()
            .unwrap_or(0)
    }
}

/// Wrap one clean in-memory link endpoint with byte metering: no fault
/// ever fires (the budgets are effectively infinite), and the returned
/// [`ByteMeter`] reads out the endpoint's cumulative traffic.
pub fn metered(link: MemoryLink) -> (FaultyLink, ByteMeter) {
    let (link, (write, read, streams), _) = wrap_with(link, budget(None), budget(None), None);
    (
        link,
        ByteMeter {
            write,
            read,
            streams,
        },
    )
}

/// Wrap `link` under `plan`: the wrapped link, its vanish state if the plan
/// vanishes, and a clone of the endpoint's stream supply.
fn wrap(
    link: MemoryLink,
    plan: FaultPlan,
) -> (FaultyLink, Option<Arc<VanishState>>, MemoryConnector) {
    let vanish = plan.vanish.map(VanishState::new);
    let (link, _, supply) = wrap_with(
        link,
        budget(plan.write_cut),
        budget(plan.read_cut),
        vanish.clone(),
    );
    (link, vanish, supply)
}

fn wrap_with(
    link: MemoryLink,
    write: Budget,
    read: Budget,
    vanish: Option<Arc<VanishState>>,
) -> (FaultyLink, (Budget, Budget, Streams), MemoryConnector) {
    let parts = link.into_parts();
    let supply = parts.connector.clone();
    let streams: Streams = Arc::new(Mutex::new(Vec::new()));
    let link = LinkParts {
        control_read: Cut::new(parts.control_read, read.clone(), vanish.clone()),
        control_write: Fuse::new(parts.control_write, write.clone(), vanish.clone(), None),
        connector: FaultConnector {
            inner: parts.connector,
            budget: write.clone(),
            vanish: vanish.clone(),
            streams: streams.clone(),
        },
        acceptor: FaultAcceptor {
            inner: parts.acceptor,
            budget: read.clone(),
            vanish,
        },
        session: parts.session,
    }
    .into_link();
    (link, (write, read, streams), supply)
}

/// One endpoint's data streams in open order, each with the bytes written
/// on it: the ledger a [`Vanish::OnStream`] indexes and a [`ByteMeter`]
/// reads out.
type Streams = Arc<Mutex<Vec<usize>>>;

/// A direction's shared byte budget.
type Budget = Arc<Mutex<usize>>;

fn budget(cut: Option<usize>) -> Budget {
    Arc::new(Mutex::new(cut.unwrap_or(usize::MAX)))
}

/// One endpoint's progress toward its [`Vanish`] point, shared by every
/// wrapper of its link.
///
/// Reaching the point *trips* the state: the tripping operation returns
/// `Pending` without arranging a wake, every later operation does the
/// same, and [`vanished`](Self::vanished) resolves so the driver can drop
/// the session. Nothing this endpoint owns makes progress again.
struct VanishState {
    point: Vanish,
    /// Bytes the named data or control stream can write before vanishing.
    remaining: Mutex<usize>,
    tripped: AtomicBool,
    notify: Notify,
}

impl VanishState {
    /// Share the departure trigger and its remaining byte budget.
    fn new(point: Vanish) -> Arc<Self> {
        Arc::new(Self {
            point,
            remaining: Mutex::new(match point {
                Vanish::OnStream { offset, .. } | Vanish::OnControl { offset } => offset,
                Vanish::AtFirstConnect => usize::MAX,
            }),
            tripped: AtomicBool::new(false),
            notify: Notify::new(),
        })
    }

    /// Whether the session must stop all further I/O.
    fn tripped(&self) -> bool {
        self.tripped.load(Ordering::Acquire)
    }

    /// Stop the endpoint and wake its session owner.
    fn trip(&self) {
        self.tripped.store(true, Ordering::Release);
        self.notify.notify_one();
    }

    /// Resolves once the point is reached.
    async fn vanished(&self) {
        loop {
            if self.tripped() {
                return;
            }
            self.notify.notified().await;
        }
    }

    /// Whether `stream` (a data stream's ordinal; `None` is the control
    /// half) is the one the point names.
    fn at_point(&self, stream: Option<usize>) -> bool {
        match self.point {
            Vanish::OnStream { index, .. } => stream == Some(index),
            Vanish::OnControl { .. } => stream.is_none(),
            Vanish::AtFirstConnect => false,
        }
    }

    /// How many of `len` bytes a write on `stream` may admit, or `None`
    /// once the endpoint has vanished (tripping it if this write is the
    /// point).
    fn admit(&self, stream: Option<usize>, len: usize) -> Option<usize> {
        if self.tripped() {
            return None;
        }
        if !self.at_point(stream) {
            return Some(len);
        }
        let remaining = *self.remaining.lock().expect("vanish budget lock");
        if remaining == 0 {
            self.trip();
            return None;
        }
        Some(len.min(remaining))
    }

    /// Charge bytes actually written on the selected stream.
    fn wrote(&self, stream: Option<usize>, bytes: usize) {
        if self.at_point(stream) {
            *self.remaining.lock().expect("vanish budget lock") -= bytes;
        }
    }

    /// Whether a stream open may proceed (tripping if the open is the
    /// point).
    fn may_connect(&self) -> bool {
        if self.tripped() {
            return false;
        }
        if let Vanish::AtFirstConnect = self.point {
            self.trip();
            return false;
        }
        true
    }
}

/// Whether the endpoint has vanished: its every operation then parks.
fn vanished(vanish: &Option<Arc<VanishState>>) -> bool {
    vanish.as_ref().is_some_and(|v| v.tripped())
}

/// The failure every write-direction surface reports once its budget is
/// exhausted: writers and the connector fail identically.
fn write_severed() -> io::Error {
    io::Error::new(
        io::ErrorKind::BrokenPipe,
        "fault injection: write budget exhausted",
    )
}

/// The failure every read-direction surface reports once its budget is
/// exhausted: readers and the acceptor fail identically.
fn read_severed() -> io::Error {
    io::Error::new(
        io::ErrorKind::ConnectionReset,
        "fault injection: read budget exhausted",
    )
}

/// A connector whose opened streams draw on the endpoint's write budget,
/// and which itself fails once that budget is exhausted (or parks once
/// the endpoint has vanished).
pub struct FaultConnector<C> {
    inner: C,
    budget: Budget,
    vanish: Option<Arc<VanishState>>,
    streams: Streams,
}

impl<C: Clone> Clone for FaultConnector<C> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            budget: self.budget.clone(),
            vanish: self.vanish.clone(),
            streams: self.streams.clone(),
        }
    }
}

impl<C: Connector> Connector for FaultConnector<C> {
    type Tx = Fuse<C::Tx>;

    async fn connect(&self) -> io::Result<(Self::Tx, Done<Self::Tx>)> {
        if let Some(vanish) = &self.vanish
            && !vanish.may_connect()
        {
            std::future::pending::<()>().await;
            unreachable!("a vanished endpoint never resumes");
        }
        // A dead write direction cannot open new streams either; this is
        // what lets a cut exercise `SendError::Connect` deterministically
        // instead of only through real-transport races.
        if *self.budget.lock().expect("write budget lock") == 0 {
            return Err(write_severed());
        }
        let (tx, done) = self.inner.connect().await?;
        let ordinal = {
            let mut streams = self.streams.lock().expect("stream ledger lock");
            streams.push(0);
            streams.len() - 1
        };
        // Completion unwraps the fuse and passes the half through.
        Ok((
            Fuse::new(
                tx,
                self.budget.clone(),
                self.vanish.clone(),
                Some((ordinal, self.streams.clone())),
            ),
            Done::new(move |fuse: Fuse<C::Tx>| done.complete(fuse.inner)),
        ))
    }
}

/// An acceptor whose accepted streams draw on the endpoint's read budget,
/// and which itself fails once that budget is exhausted (or parks once
/// the endpoint has vanished).
pub struct FaultAcceptor<A> {
    inner: A,
    budget: Budget,
    vanish: Option<Arc<VanishState>>,
}

impl<A: Acceptor> Acceptor for FaultAcceptor<A> {
    type Rx = Cut<A::Rx>;

    async fn accept(&mut self) -> io::Result<(Self::Rx, Done<Self::Rx>)> {
        if vanished(&self.vanish) {
            std::future::pending::<()>().await;
        }
        // A dead read direction cannot deliver new streams either; this
        // reaches the session's deferred supply-failure path (the parked
        // accept driver) deterministically rather than only via races.
        if *self.budget.lock().expect("read budget lock") == 0 {
            return Err(read_severed());
        }
        let (rx, done) = self.inner.accept().await?;
        // Completion unwraps the cut and passes the half through.
        Ok((
            Cut::new(rx, self.budget.clone(), self.vanish.clone()),
            Done::new(move |cut: Cut<A::Rx>| done.complete(cut.inner)),
        ))
    }
}

/// An [`AsyncWrite`] that forwards writes until a shared byte budget is
/// exhausted, then fails every write with [`BrokenPipe`]: a deterministic
/// stand-in for a connection severed at a chosen point in the session.
///
/// [`BrokenPipe`]: std::io::ErrorKind::BrokenPipe
pub struct Fuse<W> {
    inner: W,
    remaining: Budget,
    vanish: Option<Arc<VanishState>>,
    /// Which data stream this writer is and the ledger it reports to;
    /// `None` is the control half.
    stream: Option<(usize, Streams)>,
}

impl<W> Fuse<W> {
    /// Combine the writer with its cut and departure budgets.
    fn new(
        inner: W,
        remaining: Budget,
        vanish: Option<Arc<VanishState>>,
        stream: Option<(usize, Streams)>,
    ) -> Self {
        Self {
            inner,
            remaining,
            vanish,
            stream,
        }
    }

    /// The data stream index, or `None` for control traffic.
    fn ordinal(&self) -> Option<usize> {
        self.stream.as_ref().map(|(ordinal, _)| *ordinal)
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for Fuse<W> {
    /// Write up to the next fault boundary and record actual progress.
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        // A vanish point inside this write parks it (and every later
        // operation) with no wake; the driver drops the session.
        let before_vanish = match &this.vanish {
            Some(vanish) => match vanish.admit(this.ordinal(), buf.len()) {
                Some(admitted) => admitted,
                None => return Poll::Pending,
            },
            None => buf.len(),
        };
        let mut remaining = this.remaining.lock().expect("write budget lock");
        if *remaining == 0 {
            return Poll::Ready(Err(write_severed()));
        }
        // Admit at most the remaining budget; the writer's retry of the
        // unwritten tail then trips the exhausted fuse above.
        let admitted = before_vanish.min(*remaining);
        match Pin::new(&mut this.inner).poll_write(cx, &buf[..admitted]) {
            Poll::Ready(Ok(n)) => {
                *remaining -= n;
                if let Some(vanish) = &this.vanish {
                    vanish.wrote(this.ordinal(), n);
                }
                if let Some((ordinal, streams)) = &this.stream {
                    streams.lock().expect("stream ledger lock")[*ordinal] += n;
                }
                Poll::Ready(Ok(n))
            }
            other => other,
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if vanished(&this.vanish) {
            return Poll::Pending;
        }
        Pin::new(&mut this.inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if vanished(&this.vanish) {
            return Poll::Pending;
        }
        Pin::new(&mut this.inner).poll_shutdown(cx)
    }
}

/// An [`AsyncRead`] that forwards reads until a shared byte budget is
/// exhausted, then fails every read with [`ConnectionReset`]: the read-side
/// twin of [`Fuse`], for sessions that die while a frame is in flight
/// toward us.
///
/// [`ConnectionReset`]: std::io::ErrorKind::ConnectionReset
pub struct Cut<R> {
    inner: R,
    remaining: Budget,
    vanish: Option<Arc<VanishState>>,
}

impl<R> Cut<R> {
    fn new(inner: R, remaining: Budget, vanish: Option<Arc<VanishState>>) -> Self {
        Self {
            inner,
            remaining,
            vanish,
        }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for Cut<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if vanished(&this.vanish) {
            return Poll::Pending;
        }
        let mut remaining = this.remaining.lock().expect("read budget lock");
        if *remaining == 0 {
            return Poll::Ready(Err(read_severed()));
        }
        // Read through a budget-limited window over `buf`'s unfilled
        // region, then advance `buf` by however much actually arrived.
        let limit = (*remaining).min(buf.remaining());
        let window = buf.initialize_unfilled_to(limit);
        let mut limited = ReadBuf::new(window);
        match Pin::new(&mut this.inner).poll_read(cx, &mut limited) {
            Poll::Ready(Ok(())) => {
                let n = limited.filled().len();
                *remaining -= n;
                buf.advance(n);
                Poll::Ready(Ok(()))
            }
            other => other,
        }
    }
}
