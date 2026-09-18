//! Type-erased stream supply for wire sessions.
//!
//! The reconciliation state machine is deeply generic. Carrying a concrete
//! [`Link`](super::Link) through it would compile another copy for every link
//! implementation in a downstream binary. Session entry points instead erase
//! the link's control halves, connector, and acceptor before entering the
//! protocol, so caller-defined link types no longer multiply copies of that
//! large body.
//!
//! A stream half and its [`Done`] callback have the same concrete type
//! parameter, so they must be erased together. [`HalfWithDone`] stores that
//! pair behind the read or write trait object. Completing the erased half
//! recovers the pair and invokes the original callback.
//!
//! Erasure adds dynamic dispatch beneath the frame codec. Each stream open or
//! accept also allocates a [`BoxFuture`] and a box for the returned stream
//! half.

use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::future::BoxFuture;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use super::{Acceptor, Connector, Done};

/// A stream half kept with the concrete callback that completes it.
struct HalfWithDone<H> {
    /// The stream half exposed through the erased read or write interface.
    half: H,
    /// The transport's action for a cleanly completed stream.
    done: Done<H>,
}

/// Recover and complete a stream after its erased user reaches the end.
impl<H> HalfWithDone<H> {
    /// Invoke the transport's completion action with the concrete half.
    fn complete(self) {
        self.done.complete(self.half);
    }
}

/// Forward erased writes to the concrete stream half.
impl<H: AsyncWrite + Unpin> AsyncWrite for HalfWithDone<H> {
    /// Write through to the concrete stream half.
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.half).poll_write(cx, buf)
    }

    /// Flush the concrete stream half.
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.half).poll_flush(cx)
    }

    /// Shut down the concrete stream half.
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.half).poll_shutdown(cx)
    }
}

/// Forward erased reads to the concrete stream half.
impl<H: AsyncRead + Unpin> AsyncRead for HalfWithDone<H> {
    /// Read through to the concrete stream half.
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.half).poll_read(cx, buf)
    }
}

/// A boxed outgoing half that can still run its concrete completion action.
pub(crate) trait TxDyn: AsyncWrite + Unpin + Send {
    /// Complete the stream and release its concrete half.
    fn complete(self: Box<Self>);
}

/// Preserve clean completion while erasing an outgoing half.
impl<H: AsyncWrite + Unpin + Send> TxDyn for HalfWithDone<H> {
    /// Recover the concrete half and its completion action.
    fn complete(self: Box<Self>) {
        HalfWithDone::complete(*self);
    }
}

/// A boxed incoming half that can still run its concrete completion action.
pub(crate) trait RxDyn: AsyncRead + Unpin + Send {
    /// Complete the stream and release its concrete half.
    fn complete(self: Box<Self>);
}

/// Preserve clean completion while erasing an incoming half.
impl<H: AsyncRead + Unpin + Send> RxDyn for HalfWithDone<H> {
    /// Recover the concrete half and its completion action.
    fn complete(self: Box<Self>) {
        HalfWithDone::complete(*self);
    }
}

/// An owned outgoing stream half with its concrete type erased.
pub(crate) type DynTx = Box<dyn TxDyn>;

/// An owned incoming stream half with its concrete type erased.
pub(crate) type DynRx = Box<dyn RxDyn>;

/// Object-safe [`Connector`], for erasure behind an [`Arc`].
trait ConnectDyn: Send + Sync {
    /// Open and erase one outgoing stream.
    fn connect_dyn(&self) -> BoxFuture<'_, io::Result<(DynTx, Done<DynTx>)>>;
}

/// Erase streams opened by any concrete connector.
impl<C: Connector> ConnectDyn for C {
    /// Keep the concrete completion action beside the half before boxing it.
    fn connect_dyn(&self) -> BoxFuture<'_, io::Result<(DynTx, Done<DynTx>)>> {
        Box::pin(async {
            let (half, done) = self.connect().await?;
            let erased: DynTx = Box::new(HalfWithDone { half, done });
            Ok((erased, Done::new(TxDyn::complete)))
        })
    }
}

/// A [`Connector`] handle with its concrete type erased.
///
/// [`Arc`] rather than [`Box`] because [`Connector`] requires [`Clone`]:
/// every stream producer owns a handle.
#[derive(Clone)]
pub(crate) struct DynConnector(Arc<dyn ConnectDyn>);

impl DynConnector {
    /// Erase `connector`, sharing it among all clones of the result.
    pub(crate) fn new<C: Connector>(connector: C) -> Self {
        Self(Arc::new(connector))
    }
}

/// Open outgoing streams through the erased connector.
impl Connector for DynConnector {
    /// The erased outgoing stream half.
    type Tx = DynTx;

    /// Dispatch an open through the concrete connector.
    async fn connect(&self) -> io::Result<(DynTx, Done<DynTx>)> {
        self.0.connect_dyn().await
    }
}

/// Object-safe [`Acceptor`], for erasure behind a `&mut` borrow.
pub(crate) trait AcceptDyn: Send {
    /// Accept and erase one incoming stream.
    fn accept_dyn(&mut self) -> BoxFuture<'_, io::Result<(DynRx, Done<DynRx>)>>;
}

/// Erase streams accepted by any concrete acceptor.
impl<A: Acceptor> AcceptDyn for A {
    /// Keep the concrete completion action beside the half before boxing it.
    fn accept_dyn(&mut self) -> BoxFuture<'_, io::Result<(DynRx, Done<DynRx>)>> {
        Box::pin(async {
            let (half, done) = self.accept().await?;
            let erased: DynRx = Box::new(HalfWithDone { half, done });
            Ok((erased, Done::new(RxDyn::complete)))
        })
    }
}

/// A borrowed [`Acceptor`] with its concrete type erased.
///
/// A borrow rather than an owned box because the acceptor, unlike the
/// connector, has a single consumer — the session's accept loop — and
/// returns to the caller's [`Link`](super::Link) between sessions.
pub(crate) type DynAcceptor<'a> = &'a mut (dyn AcceptDyn + 'a);

/// Accept incoming streams through the erased acceptor.
impl<'a, 'd> Acceptor for &'a mut (dyn AcceptDyn + 'd) {
    /// The erased incoming stream half.
    type Rx = DynRx;

    /// Dispatch an accept through the concrete acceptor.
    async fn accept(&mut self) -> io::Result<(Self::Rx, Done<Self::Rx>)> {
        // Dispatch through the object explicitly: plain method syntax would
        // resolve to the blanket `AcceptDyn` impl for `&mut dyn AcceptDyn`
        // (this very impl's `Acceptor`), recursing instead of erasing.
        AcceptDyn::accept_dyn(&mut **self).await
    }
}
