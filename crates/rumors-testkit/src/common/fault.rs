//! Wire-fault injection for the disruption simulation: deterministic,
//! byte-budgeted severing of either direction of a gossip connection.
//!
//! A "dropped connection" in the simulation is one or both directions of a
//! session's connection tripping at an arbitrary byte offset mid-session:
//!
//! - [`Fuse`] forwards writes until its budget is exhausted, then fails
//!   every write with [`BrokenPipe`] — the connection died under our pen.
//! - [`Cut`] forwards reads until its budget is exhausted, then fails every
//!   read with [`ConnectionReset`] — the connection died under our eyes.
//!
//! Each budget counts its direction's bytes from the session's first, so
//! the cut lands at a chosen offset in the endpoint's traffic, whatever
//! item that byte belongs to. The wrapped side observes the cut as an
//! error; its counterparty observes it as end-of-stream (and a truncated
//! item) once the failing side's connection drops, or as its own write
//! error against the closed transport. Either way the session dies
//! somewhere the protocol did not choose, which is exactly the disruption
//! the simulation is after.
//!
//! A *vanish* is the other way a peer dies: not a wire error it can
//! observe, but the peer itself gone mid-protocol, as a crashed process
//! is. At its [`Vanish`] point the endpoint's session is dropped where it
//! stands, its connection halves with it and without any shutdown. Its
//! counterparty sees exactly what a vanished socket gives: end-of-stream.
//! [`drive`] runs a session under such a plan.
//!
//! [`BrokenPipe`]: std::io::ErrorKind::BrokenPipe
//! [`ConnectionReset`]: std::io::ErrorKind::ConnectionReset

use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use rumors::testing::MemoryEnd;
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
/// without any shutdown, its connection halves with it.
///
/// The endpoint vanishes at its first write after it has written `offset`
/// bytes, so a vanish can land anywhere in its outgoing traffic: inside the
/// preamble or greeting, inside a frame, between frames, or in the closing
/// items. An endpoint that writes no more than `offset` bytes never reaches
/// the point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vanish {
    /// Bytes successfully written before the endpoint vanishes.
    pub offset: usize,
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

/// One in-memory connection end under a fault plan: the two halves a
/// session takes, each drawing on its direction's budget.
pub struct FaultyEnd {
    /// The read half, failing once the read budget is spent.
    pub read: Cut<DuplexStream>,
    /// The write half, failing once the write budget is spent.
    pub write: Fuse<DuplexStream>,
}

/// Wrap one in-memory connection end in `plan`'s budgets.
///
/// A clean plan still wraps (with effectively-infinite budgets), so every
/// call site handles one pair of types regardless of whether it faults.
///
/// # Panics
///
/// If the plan vanishes: a vanishing endpoint's session must be driven by
/// [`drive`], which owns dropping it at the point.
pub fn faulty(end: MemoryEnd, plan: FaultPlan) -> FaultyEnd {
    assert!(
        plan.vanish.is_none(),
        "a vanishing endpoint's session is driven by `drive`"
    );
    wrap(end, plan).0
}

/// Stop I/O at a byte boundary and signal that the session has stalled.
/// Using the signal as a deadline tests real timeout recovery without injecting
/// a transport error or dropping the session from outside.
pub fn stall_at(end: MemoryEnd, point: Vanish) -> (FaultyEnd, impl Future<Output = ()> + Send) {
    let (end, stalled) = wrap(
        end,
        FaultPlan {
            vanish: Some(point),
            ..FaultPlan::NONE
        },
    );
    let stalled = stalled.expect("a stall point was supplied");
    (end, async move { stalled.vanished().await })
}

/// What [`drive`] leaves behind.
pub struct Driven<Out> {
    /// The session's outcome, or `None` if the endpoint vanished first.
    pub outcome: Option<Out>,
}

impl<Out> Driven<Out> {
    /// Whether the endpoint reached its vanish point.
    pub fn vanished(&self) -> bool {
        self.outcome.is_none()
    }
}

/// Run `session` over `end` under `plan`, vanishing at the plan's point.
///
/// The session owns the faulted end's halves. Without a vanish this is
/// `session` over [`faulty`]'s end. With one, the session is polled until
/// the endpoint reaches its point, where the session future, and the
/// connection halves it owns, are dropped with no shutdown: the
/// counterparty then reads end-of-stream.
pub async fn drive<Out>(
    end: MemoryEnd,
    plan: FaultPlan,
    session: impl AsyncFnOnce(FaultyEnd) -> Out,
) -> Driven<Out> {
    let (end, vanish) = wrap(end, plan);
    let Some(vanish) = vanish else {
        return Driven {
            outcome: Some(session(end).await),
        };
    };
    let outcome = tokio::select! {
        biased;
        outcome = session(end) => Some(outcome),
        () = vanish.vanished() => None,
    };
    Driven { outcome }
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
}

impl ByteMeter {
    /// Total bytes the endpoint has written: the counter a `write_cut` and
    /// a [`Vanish`] offset spend.
    pub fn written(&self) -> usize {
        usize::MAX - *self.write.lock().expect("write budget lock")
    }

    /// Total bytes the endpoint has read: the counter a `read_cut` spends.
    pub fn read(&self) -> usize {
        usize::MAX - *self.read.lock().expect("read budget lock")
    }
}

/// Wrap one clean in-memory connection end with byte metering: no fault
/// ever fires (the budgets are effectively infinite), and the returned
/// [`ByteMeter`] reads out the endpoint's cumulative traffic.
pub fn metered(end: MemoryEnd) -> (FaultyEnd, ByteMeter) {
    let write = budget(None);
    let read = budget(None);
    let end = wrap_with(end, write.clone(), read.clone(), None);
    (end, ByteMeter { write, read })
}

/// Wrap `end` under `plan`: the wrapped end, and its vanish state if the
/// plan vanishes.
fn wrap(end: MemoryEnd, plan: FaultPlan) -> (FaultyEnd, Option<Arc<VanishState>>) {
    let vanish = plan.vanish.map(VanishState::new);
    let end = wrap_with(
        end,
        budget(plan.write_cut),
        budget(plan.read_cut),
        vanish.clone(),
    );
    (end, vanish)
}

/// Wrap both halves of `end` in budgets and an optional vanish point.
fn wrap_with(
    end: MemoryEnd,
    write: Budget,
    read: Budget,
    vanish: Option<Arc<VanishState>>,
) -> FaultyEnd {
    FaultyEnd {
        read: Cut::new(end.read, read, vanish.clone()),
        write: Fuse::new(end.write, write, vanish),
    }
}

/// A direction's shared byte budget.
type Budget = Arc<Mutex<usize>>;

/// A byte budget that runs out after `cut` bytes, or effectively never.
fn budget(cut: Option<usize>) -> Budget {
    Arc::new(Mutex::new(cut.unwrap_or(usize::MAX)))
}

/// One endpoint's progress toward its [`Vanish`] point, shared by both
/// halves of its connection.
///
/// Reaching the point *trips* the state: the tripping operation returns
/// `Pending` without arranging a wake, every later operation does the
/// same, and [`vanished`](Self::vanished) resolves so the driver can drop
/// the session. Nothing this endpoint owns makes progress again.
struct VanishState {
    /// Bytes the endpoint can still write before vanishing.
    remaining: Mutex<usize>,
    tripped: AtomicBool,
    notify: Notify,
}

impl VanishState {
    /// Share the departure trigger and its remaining byte budget.
    fn new(point: Vanish) -> Arc<Self> {
        Arc::new(Self {
            remaining: Mutex::new(point.offset),
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

    /// How many of `len` bytes a write may admit, or `None` once the
    /// endpoint has vanished (tripping it if this write is the point).
    fn admit(&self, len: usize) -> Option<usize> {
        if self.tripped() {
            return None;
        }
        let remaining = *self.remaining.lock().expect("vanish budget lock");
        if remaining == 0 {
            self.trip();
            return None;
        }
        Some(len.min(remaining))
    }

    /// Charge bytes actually written.
    fn wrote(&self, bytes: usize) {
        *self.remaining.lock().expect("vanish budget lock") -= bytes;
    }
}

/// Whether the endpoint has vanished: its every operation then parks.
fn vanished(vanish: &Option<Arc<VanishState>>) -> bool {
    vanish.as_ref().is_some_and(|v| v.tripped())
}

/// The failure every write reports once the write budget is exhausted.
fn write_severed() -> io::Error {
    io::Error::new(
        io::ErrorKind::BrokenPipe,
        "fault injection: write budget exhausted",
    )
}

/// The failure every read reports once the read budget is exhausted.
fn read_severed() -> io::Error {
    io::Error::new(
        io::ErrorKind::ConnectionReset,
        "fault injection: read budget exhausted",
    )
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
}

impl<W> Fuse<W> {
    /// Combine the writer with its cut and departure budgets.
    fn new(inner: W, remaining: Budget, vanish: Option<Arc<VanishState>>) -> Self {
        Self {
            inner,
            remaining,
            vanish,
        }
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
            Some(vanish) => match vanish.admit(buf.len()) {
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
                    vanish.wrote(n);
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
