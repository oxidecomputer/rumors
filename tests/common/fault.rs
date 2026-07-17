//! Wire-fault injection for the disruption simulations: deterministic,
//! byte-budgeted severing of either direction of a gossip link.
//!
//! A "dropped connection" in the simulations is one or both directions of a
//! [`rumors::Link`] tripping at an arbitrary byte offset mid-session:
//!
//! - [`Fuse`] forwards writes until its budget is exhausted, then fails
//!   every write with [`BrokenPipe`] — the connection died under our pen.
//! - [`Cut`] forwards reads until its budget is exhausted, then fails every
//!   read with [`ConnectionReset`] — the connection died under our eyes.
//!
//! Each budget is shared across every stream of its direction — the control
//! half and each data stream draw on one counter — so the cut lands at a
//! chosen offset in the endpoint's total traffic, wherever that byte
//! happens to travel. The wrapped side observes the cut as an error; its
//! counterparty observes it as end-of-stream (and a truncated frame) once
//! the failing side's link drops, or as its own write error against the
//! closed transport. Either way the session dies somewhere the protocol did
//! not choose, which is exactly the disruption the simulations are after.
//!
//! [`BrokenPipe`]: std::io::ErrorKind::BrokenPipe
//! [`ConnectionReset`]: std::io::ErrorKind::ConnectionReset

use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use rumors::link::{
    Acceptor, Connector, Link, LinkParts, MemoryAcceptor, MemoryConnector, MemoryLink,
};
use tokio::io::{AsyncRead, AsyncWrite, DuplexStream, ReadBuf};

/// One endpoint's fault plan: byte budgets after which its write
/// (respectively read) direction fails. `None` means that direction never
/// fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FaultPlan {
    /// Bytes this endpoint may write before its writers fail.
    pub write_cut: Option<usize>,
    /// Bytes this endpoint may read before its readers fail.
    pub read_cut: Option<usize>,
}

impl FaultPlan {
    /// A clean endpoint: neither direction ever fails.
    pub const NONE: Self = Self {
        write_cut: None,
        read_cut: None,
    };

    /// Whether this plan injects any fault at all.
    pub fn is_clean(&self) -> bool {
        *self == Self::NONE
    }
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
pub fn faulty(link: MemoryLink, plan: FaultPlan) -> FaultyLink {
    faulty_link(link, plan)
}

/// [`faulty`] for any link shape, e.g. the inter-process TCP link.
pub fn faulty_link<CR, CW, C, A>(
    link: Link<CR, CW, C, A>,
    plan: FaultPlan,
) -> Link<Cut<CR>, Fuse<CW>, FaultConnector<C>, FaultAcceptor<A>>
where
    CR: AsyncRead + Unpin + Send,
    CW: AsyncWrite + Unpin + Send,
    C: Connector,
    A: Acceptor,
{
    let write_budget = budget(plan.write_cut);
    let read_budget = budget(plan.read_cut);
    let parts = link.into_parts();
    LinkParts {
        control_read: Cut::new(parts.control_read, read_budget.clone()),
        control_write: Fuse::new(parts.control_write, write_budget.clone()),
        connector: FaultConnector {
            inner: parts.connector,
            budget: write_budget,
        },
        acceptor: FaultAcceptor {
            inner: parts.acceptor,
            budget: read_budget,
        },
        epoch: parts.epoch,
    }
    .into_link()
}

/// A direction's shared byte budget.
type Budget = Arc<Mutex<usize>>;

fn budget(cut: Option<usize>) -> Budget {
    Arc::new(Mutex::new(cut.unwrap_or(usize::MAX)))
}

/// A connector whose opened streams draw on the endpoint's write budget.
pub struct FaultConnector<C> {
    inner: C,
    budget: Budget,
}

impl<C: Clone> Clone for FaultConnector<C> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            budget: self.budget.clone(),
        }
    }
}

impl<C: Connector> Connector for FaultConnector<C> {
    type Tx = Fuse<C::Tx>;

    async fn connect(&self) -> io::Result<Self::Tx> {
        let tx = self.inner.connect().await?;
        Ok(Fuse {
            inner: tx,
            remaining: self.budget.clone(),
        })
    }
}

/// An acceptor whose accepted streams draw on the endpoint's read budget.
pub struct FaultAcceptor<A> {
    inner: A,
    budget: Budget,
}

impl<A: Acceptor> Acceptor for FaultAcceptor<A> {
    type Rx = Cut<A::Rx>;

    async fn accept(&mut self) -> io::Result<Self::Rx> {
        let rx = self.inner.accept().await?;
        Ok(Cut {
            inner: rx,
            remaining: self.budget.clone(),
        })
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
}

impl<W> Fuse<W> {
    fn new(inner: W, remaining: Budget) -> Self {
        Self { inner, remaining }
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for Fuse<W> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        let mut remaining = this.remaining.lock().expect("write budget lock");
        if *remaining == 0 {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "fault injection: write budget exhausted",
            )));
        }
        // Admit at most the remaining budget; the writer's retry of the
        // unwritten tail then trips the exhausted fuse above.
        let admitted = buf.len().min(*remaining);
        match Pin::new(&mut this.inner).poll_write(cx, &buf[..admitted]) {
            Poll::Ready(Ok(n)) => {
                *remaining -= n;
                Poll::Ready(Ok(n))
            }
            other => other,
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
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
}

impl<R> Cut<R> {
    fn new(inner: R, remaining: Budget) -> Self {
        Self { inner, remaining }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for Cut<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let mut remaining = this.remaining.lock().expect("read budget lock");
        if *remaining == 0 {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::ConnectionReset,
                "fault injection: read budget exhausted",
            )));
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
