//! Type-erased stream supply, the monomorphization funnel for sessions.
//!
//! The protocol state machines carry their transport type parameters through
//! every height of the descent, so each distinct [`Link`](super::Link)
//! instantiation would re-instantiate both towers in each downstream binary,
//! at a measured cost of about +0.7 GiB of rustc peak memory per additional
//! tower instantiation — the reason this funnel is load-bearing. Every
//! session entry point therefore erases the link's stream supply here
//! — mirroring the `DynRead`/`DynWrite` erasure of the control halves — and
//! the towers instantiate once per payload type.
//!
//! The price is one vtable call per `poll_read`/`poll_write` beneath the
//! frame codec, and — per stream open/accept — a vtable call plus two
//! allocations: the fresh `Box::pin` for the [`BoxFuture`] each erased
//! `connect`/`accept` returns, and the box that erases the stream half it
//! yields.

use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::future::BoxFuture;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use super::{Acceptor, Connector, Done};

/// A half boxed together with its [`Done`], so completion has the
/// concrete types.
struct Bundle<H> {
    half: H,
    done: Done<H>,
}

impl<H: AsyncWrite + Unpin> AsyncWrite for Bundle<H> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.half).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.half).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.half).poll_shutdown(cx)
    }
}

impl<H: AsyncRead + Unpin> AsyncRead for Bundle<H> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.half).poll_read(cx, buf)
    }
}

/// An erased outgoing half, completable through the box.
pub(crate) trait TxDyn: AsyncWrite + Unpin + Send {
    /// Release the bundled half at its stream's clean end.
    fn complete(self: Box<Self>);
}

impl<H: AsyncWrite + Unpin + Send> TxDyn for Bundle<H> {
    fn complete(self: Box<Self>) {
        let Bundle { half, done } = *self;
        done.complete(half);
    }
}

/// An erased incoming half, completable through the box.
pub(crate) trait RxDyn: AsyncRead + Unpin + Send {
    /// Release the bundled half at its stream's clean end.
    fn complete(self: Box<Self>);
}

impl<H: AsyncRead + Unpin + Send> RxDyn for Bundle<H> {
    fn complete(self: Box<Self>) {
        let Bundle { half, done } = *self;
        done.complete(half);
    }
}

/// An owned outgoing stream half with its concrete type erased.
pub(crate) type DynTx = Box<dyn TxDyn>;

/// An owned incoming stream half with its concrete type erased.
pub(crate) type DynRx = Box<dyn RxDyn>;

/// Object-safe [`Connector`], for erasure behind an [`Arc`].
trait ConnectDyn: Send + Sync {
    fn connect_dyn(&self) -> BoxFuture<'_, io::Result<(DynTx, Done<DynTx>)>>;
}

impl<C: Connector> ConnectDyn for C {
    fn connect_dyn(&self) -> BoxFuture<'_, io::Result<(DynTx, Done<DynTx>)>> {
        Box::pin(async {
            let (half, done) = self.connect().await?;
            let erased: DynTx = Box::new(Bundle { half, done });
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

impl Connector for DynConnector {
    type Tx = DynTx;

    async fn connect(&self) -> io::Result<(DynTx, Done<DynTx>)> {
        self.0.connect_dyn().await
    }
}

/// Object-safe [`Acceptor`], for erasure behind a `&mut` borrow.
pub(crate) trait AcceptDyn: Send {
    fn accept_dyn(&mut self) -> BoxFuture<'_, io::Result<(DynRx, Done<DynRx>)>>;
}

impl<A: Acceptor> AcceptDyn for A {
    fn accept_dyn(&mut self) -> BoxFuture<'_, io::Result<(DynRx, Done<DynRx>)>> {
        Box::pin(async {
            let (half, done) = self.accept().await?;
            let erased: DynRx = Box::new(Bundle { half, done });
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

impl<'a, 'd> Acceptor for &'a mut (dyn AcceptDyn + 'd) {
    type Rx = DynRx;

    async fn accept(&mut self) -> io::Result<(Self::Rx, Done<Self::Rx>)> {
        // Dispatch through the object explicitly: plain method syntax would
        // resolve to the blanket `AcceptDyn` impl for `&mut dyn AcceptDyn`
        // (this very impl's `Acceptor`), recursing instead of erasing.
        AcceptDyn::accept_dyn(&mut **self).await
    }
}
