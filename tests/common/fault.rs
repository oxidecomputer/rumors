//! Wire-fault injection for the disruption simulations: deterministic,
//! byte-budgeted severing of either direction of a gossip channel.
//!
//! A "dropped channel" in the simulations is one or both of these wrappers
//! tripping at an arbitrary byte offset mid-session:
//!
//! - [`Fuse`] forwards writes until its budget is exhausted, then fails
//!   every write with [`BrokenPipe`] — the connection died under our pen.
//! - [`Cut`] forwards reads until its budget is exhausted, then fails every
//!   read with [`ConnectionReset`] — the connection died under our eyes.
//!
//! The wrapped side observes the cut as an error; its counterparty observes
//! it as EOF (and a truncated frame) once the failing side's halves drop,
//! or as its own write error against the closed transport. Either way the
//! session dies somewhere the protocol did not choose, which is exactly the
//! disruption the simulations are after. The same wrappers fit an in-memory
//! [`duplex`](tokio::io::duplex) half and a real [`TcpStream`]
//! (tokio::net::TcpStream) alike.
//!
//! [`BrokenPipe`]: std::io::ErrorKind::BrokenPipe
//! [`ConnectionReset`]: std::io::ErrorKind::ConnectionReset

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// One endpoint's fault plan: byte budgets after which its write
/// (respectively read) half fails. `None` means that direction never fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FaultPlan {
    /// Bytes this endpoint may write before its writer fails.
    pub write_cut: Option<usize>,
    /// Bytes this endpoint may read before its reader fails.
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

/// Split a bidirectional stream and wrap each half in `plan`'s budgets.
/// A clean plan still wraps (with effectively-infinite budgets), so every
/// call site handles one pair of types regardless of whether it faults.
pub fn faulty<S>(
    stream: S,
    plan: FaultPlan,
) -> (Cut<tokio::io::ReadHalf<S>>, Fuse<tokio::io::WriteHalf<S>>)
where
    S: AsyncRead + AsyncWrite,
{
    let (read, write) = tokio::io::split(stream);
    (
        Cut::new(read, plan.read_cut),
        Fuse::new(write, plan.write_cut),
    )
}

/// An [`AsyncWrite`] that forwards writes until a byte budget is exhausted,
/// then fails every write with [`BrokenPipe`]: a deterministic stand-in for
/// a connection severed at a chosen point in the session.
///
/// [`BrokenPipe`]: std::io::ErrorKind::BrokenPipe
pub struct Fuse<W> {
    inner: W,
    remaining: usize,
}

impl<W> Fuse<W> {
    /// Budget `None` means the fuse never blows.
    pub fn new(inner: W, budget: Option<usize>) -> Self {
        Self {
            inner,
            remaining: budget.unwrap_or(usize::MAX),
        }
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for Fuse<W> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        if this.remaining == 0 {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "fault injection: write budget exhausted",
            )));
        }
        // Admit at most the remaining budget; the writer's retry of the
        // unwritten tail then trips the exhausted fuse above.
        let admitted = buf.len().min(this.remaining);
        match Pin::new(&mut this.inner).poll_write(cx, &buf[..admitted]) {
            Poll::Ready(Ok(n)) => {
                this.remaining -= n;
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

/// An [`AsyncRead`] that forwards reads until a byte budget is exhausted,
/// then fails every read with [`ConnectionReset`]: the read-side twin of
/// [`Fuse`], for sessions that die while a frame is in flight toward us.
///
/// [`ConnectionReset`]: std::io::ErrorKind::ConnectionReset
pub struct Cut<R> {
    inner: R,
    remaining: usize,
}

impl<R> Cut<R> {
    /// Budget `None` means the reader is never cut.
    pub fn new(inner: R, budget: Option<usize>) -> Self {
        Self {
            inner,
            remaining: budget.unwrap_or(usize::MAX),
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
        if this.remaining == 0 {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::ConnectionReset,
                "fault injection: read budget exhausted",
            )));
        }
        // Read through a budget-limited window over `buf`'s unfilled
        // region, then advance `buf` by however much actually arrived.
        let limit = this.remaining.min(buf.remaining());
        let window = buf.initialize_unfilled_to(limit);
        let mut limited = ReadBuf::new(window);
        match Pin::new(&mut this.inner).poll_read(cx, &mut limited) {
            Poll::Ready(Ok(())) => {
                let n = limited.filled().len();
                this.remaining -= n;
                buf.advance(n);
                Poll::Ready(Ok(()))
            }
            other => other,
        }
    }
}
