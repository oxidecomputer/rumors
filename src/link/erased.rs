//! Type-erased stream supply, the monomorphization funnel for sessions.
//!
//! The protocol state machines carry their transport type parameters through
//! every height of the descent, so each distinct [`Link`](super::Link)
//! instantiation would re-instantiate both towers in each downstream binary.
//! Every session entry point therefore erases the link's stream supply here
//! — mirroring the `DynRead`/`DynWrite` erasure of the control halves — and
//! the towers instantiate once per payload type. The price is one vtable
//! call per `poll_read`/`poll_write` beneath the frame codec, and — per
//! stream open/accept — a vtable call plus a fresh `Box::pin` allocation
//! for the [`BoxFuture`] each erased `connect`/`accept` returns.

use std::io;

use futures::future::BoxFuture;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};

use super::{Acceptor, Connector};

/// An owned outgoing stream half with its concrete type erased.
pub(crate) type DynTx = Box<dyn AsyncWrite + Unpin + Send>;

/// An owned incoming stream half with its concrete type erased.
pub(crate) type DynRx = Box<dyn AsyncRead + Unpin + Send>;

/// Object-safe [`Connector`], for erasure behind an [`Arc`].
trait ConnectDyn: Send + Sync {
    fn connect_dyn(&self) -> BoxFuture<'_, io::Result<DynTx>>;
}

impl<C: Connector> ConnectDyn for C {
    fn connect_dyn(&self) -> BoxFuture<'_, io::Result<DynTx>> {
        Box::pin(async { self.connect().await.map(|tx| Box::new(tx) as DynTx) })
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

    async fn connect(&self) -> io::Result<Self::Tx> {
        self.0.connect_dyn().await
    }
}

/// Object-safe [`Acceptor`], for erasure behind a `&mut` borrow.
pub(crate) trait AcceptDyn: Send {
    fn accept_dyn(&mut self) -> BoxFuture<'_, io::Result<DynRx>>;
}

impl<A: Acceptor> AcceptDyn for A {
    fn accept_dyn(&mut self) -> BoxFuture<'_, io::Result<DynRx>> {
        Box::pin(async { self.accept().await.map(|rx| Box::new(rx) as DynRx) })
    }
}

/// A borrowed [`Acceptor`] with its concrete type erased.
///
/// A borrow rather than an owned box because the acceptor, unlike the
/// connector, has a single consumer — the session's accept loop — and
/// returns to the caller's [`Link`](super::Link) between sessions.
pub(crate) type DynAcceptor<'a> = &'a mut (dyn AcceptDyn + 'a);

/// A [`Link`](super::Link) with every transport type erased, owned.
///
/// Owned boxes rather than the session funnels' borrowed halves because
/// this carrier replaces a link a test *moves* into the protocol: the
/// session machinery must drop the sole connector handle when it finishes,
/// so the peer's acceptor observes the supply closing exactly as it would
/// with the concrete link. See [`Link::into_erased`](super::Link::into_erased).
#[cfg(test)]
pub(crate) type ErasedLink = super::Link<
    Box<dyn AsyncRead + Unpin + Send>,
    Box<dyn AsyncWrite + Unpin + Send>,
    DynConnector,
    Box<dyn AcceptDyn>,
>;

#[cfg(test)]
impl super::Acceptor for Box<dyn AcceptDyn> {
    type Rx = DynRx;

    async fn accept(&mut self) -> io::Result<Self::Rx> {
        // Explicit dispatch for the same reason as the `&mut dyn` impl
        // above: method syntax would land on the blanket `AcceptDyn` impl
        // for the box itself and recurse.
        AcceptDyn::accept_dyn(&mut **self).await
    }
}

#[cfg(test)]
impl<CR, CW, C, A> super::Link<CR, CW, C, A>
where
    CR: AsyncRead + Unpin + Send + 'static,
    CW: AsyncWrite + Unpin + Send + 'static,
    C: super::Connector,
    A: super::Acceptor + 'static,
{
    /// Erase this link into one owned session carrier.
    ///
    /// Test fixtures decorate links freely (reordering, fault-injecting,
    /// scripted transports), and every distinct `Link` type driven into the
    /// streaming protocol instantiates the whole per-height tower afresh —
    /// measured at roughly 0.7 GiB of rustc peak memory per instantiation
    /// in this crate's test binary (see `design/height-erasure.md`).
    /// Erasing here funnels every fixture into one carrier instantiation,
    /// so a new fixture costs a vtable, not a tower. Ownership moves with
    /// the erasure: the carrier holds the only handles, so session-end
    /// drop semantics (half-close, supply closure) match the concrete link
    /// exactly.
    pub(crate) fn into_erased(self) -> ErasedLink {
        super::Link::for_session(
            Box::new(self.control_read),
            Box::new(self.control_write),
            DynConnector::new(self.connector),
            Box::new(self.acceptor) as Box<dyn AcceptDyn>,
            self.session.epoch,
        )
    }
}

impl<'a, 'd> Acceptor for &'a mut (dyn AcceptDyn + 'd) {
    type Rx = DynRx;

    async fn accept(&mut self) -> io::Result<Self::Rx> {
        // Dispatch through the object explicitly: plain method syntax would
        // resolve to the blanket `AcceptDyn` impl for `&mut dyn AcceptDyn`
        // (this very impl's `Acceptor`), recursing instead of erasing.
        AcceptDyn::accept_dyn(&mut **self).await
    }
}
