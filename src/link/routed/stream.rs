//! Per-link outgoing connection reuse and incoming stream delivery.
//!
//! Clean completion returns each connection to its owner: the connector keeps
//! the outgoing end, and the router keeps the incoming end. Each keeps at most
//! `STREAM_COUNT` idle connections per link. Dropping a stream without
//! completing it closes its connection instead.
//!
//! The router sends `READY` after admitting a returned connection. Until that
//! byte arrives, reuse would make a new stream wait for the previous consumer,
//! breaking stream independence. An attempt to open a connection probes the
//! pool without waiting and dials afresh if no connection is ready. A delayed
//! `READY` can therefore cost an extra dial, but cannot stall another stream.
//!
//! [`StreamConnector::connect`] takes a ready connection or dials one, writes
//! its routing header, and returns a `Done` that puts it back in the pool.
//! [`Pool::take_ready`] uses [`Pooled::probe`] to consume READY and discard
//! connections that have closed. On the receiving side, [`StreamAcceptor`]
//! takes streams and completion callbacks from the router; its registration
//! removes the route when the acceptor drops.

use std::collections::VecDeque;
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use tokio::io::{AsyncWriteExt, ReadBuf};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc};

use super::header::{self, Token};
use super::router::Registration;
use super::{Acceptor, Conn, Connector, Dial, Done};
use crate::link::STREAM_COUNT;

#[cfg(test)]
mod tests;

/// Opens outgoing data streams, reusing this link's completed connections
/// when [`Config::pooling`](super::Config::pooling) is enabled.
///
/// Clones share the pool. Idle connections close when the last clone drops.
pub struct StreamConnector<D: Dial> {
    /// Opens fresh connections when the pool has none ready.
    dial: D,
    /// The peer's advertised listener address.
    peer: D::Addr,
    /// Identifies this link in every outgoing stream header.
    token: Token,
    /// Shared by clones; `None` disables outgoing reuse.
    pool: Option<Arc<Pool<D::Conn>>>,
}

/// Retains idle connections while allowing transport callbacks to re-enter.
struct Pool<C> {
    /// Probed from the front; returns and pending connections join the back.
    idle: Mutex<VecDeque<Pooled<C>>>,
    /// Counts retained connections, including those currently being probed.
    slots: Arc<Semaphore>,
}

/// An outgoing connection waiting for reuse.
struct Pooled<C> {
    /// The transport returned by a completed stream.
    conn: C,
    /// Keeps this connection counted until it is reused or discarded.
    _slot: OwnedSemaphorePermit,
}

/// What a single read reveals about an idle connection.
enum Probe {
    /// No byte or terminal result is available yet.
    Pending,
    /// The peer has acknowledged completion with READY.
    Ready,
    /// The connection closed, failed, or carried an unexpected byte.
    Dead,
}

impl<C: Conn> Pooled<C> {
    /// Read once without waiting. A transport yield can cost an extra dial;
    /// the connection stays in the pool for a later open to probe again.
    fn probe(&mut self, cx: &mut Context<'_>) -> Probe {
        let mut byte = [0; 1];
        let mut buf = ReadBuf::new(&mut byte);
        match Pin::new(&mut self.conn).poll_read(cx, &mut buf) {
            Poll::Pending => Probe::Pending,
            Poll::Ready(Ok(())) if buf.filled() == [header::READY] => Probe::Ready,
            Poll::Ready(Err(error)) if error.kind() == io::ErrorKind::Interrupted => Probe::Pending,
            Poll::Ready(_) => Probe::Dead,
        }
    }
}

impl<C: Conn> Pool<C> {
    /// Create an empty pool bounded by the link's stream limit.
    fn new() -> Self {
        Self {
            idle: Mutex::default(),
            slots: Arc::new(Semaphore::new(STREAM_COUNT)),
        }
    }

    /// Admit a completed connection if there is room, or close it.
    fn put(&self, conn: C) {
        if let Ok(slot) = Arc::clone(&self.slots).try_acquire_owned() {
            self.idle
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .push_back(Pooled { conn, _slot: slot });
        }
    }

    /// Scan in queue order for a reusable connection, discarding dead ones.
    ///
    /// Returns join the back, so repeatedly reusing one connection cannot
    /// keep it ahead of others. Pending entries also move to the back: each
    /// gets another turn without holding up connections whose READY has arrived.
    /// Concurrent callers share this dequeue order, but can finish out of order.
    fn take_ready(&self) -> Option<C> {
        // Limit this call to the initial queue length. Pending entries and
        // concurrent returns must not turn a non-waiting scan into a loop.
        let attempts = self.idle.lock().unwrap_or_else(|p| p.into_inner()).len();
        let mut cx = Context::from_waker(Waker::noop());
        for _ in 0..attempts {
            // Release the mutex before polling or dropping the transport: its
            // callbacks may use this pool. The entry's permit still counts it.
            let mut entry = self
                .idle
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .pop_front()?;
            match entry.probe(&mut cx) {
                Probe::Pending => {
                    self.idle
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .push_back(entry);
                }
                // After READY, only silence is valid until reuse. Check for a
                // close or unexpected bytes before handing off the connection.
                Probe::Ready if matches!(entry.probe(&mut cx), Probe::Pending) => {
                    return Some(entry.conn);
                }
                Probe::Ready | Probe::Dead => {}
            }
        }
        None
    }
}

impl<D: Dial> StreamConnector<D> {
    /// Bind outgoing streams and optional connection reuse to one link.
    pub(super) fn new(dial: D, peer: D::Addr, token: Token, pooling: bool) -> Self {
        StreamConnector {
            dial,
            peer,
            token,
            pool: pooling.then(|| Arc::new(Pool::new())),
        }
    }
}

impl<D: Dial> Clone for StreamConnector<D> {
    /// Clone the dialer and share this link's pool.
    fn clone(&self) -> Self {
        StreamConnector {
            dial: self.dial.clone(),
            peer: self.peer.clone(),
            token: self.token,
            pool: self.pool.clone(),
        }
    }
}

impl<D: Dial> Connector for StreamConnector<D> {
    /// A transport connection carrying one outgoing data stream.
    type Tx = D::Conn;

    /// Route a ready or fresh connection and supply its completion callback.
    async fn connect(&self) -> io::Result<(Self::Tx, Done<Self::Tx>)> {
        let mut conn = match self.pool.as_ref().and_then(|pool| pool.take_ready()) {
            Some(conn) => conn,
            None => self.dial.dial(&self.peer).await?,
        };
        // Cancellation drops this connection, including a partial header.
        conn.write_all(&header::stream_header(&self.token)).await?;
        conn.flush().await?;
        let done = match &self.pool {
            Some(pool) => {
                let pool = Arc::downgrade(pool);
                Done::new(move |conn| {
                    if let Some(pool) = pool.upgrade() {
                        pool.put(conn);
                    }
                })
            }
            None => Done::discard(),
        };
        Ok((conn, done))
    }
}

/// Accepts this link's incoming data streams from the router.
///
/// Cancelling an accept preserves queued streams for the next call. Dropping
/// the acceptor revokes the route and releases idle incoming connections. Queue
/// overflow closes the supply, since it is a protocol violation to open more
/// than the fixed maximum of 17 streams per link. Already queued streams remain
/// available before the error is reported.
pub struct StreamAcceptor<C> {
    /// Streams already routed to this link, paired with their return callbacks.
    streams: mpsc::Receiver<(C, Done<C>)>,
    /// Owns the route's lifetime; dropping it releases idle incoming connections.
    _registration: Registration<C>,
}

impl<C> StreamAcceptor<C> {
    /// Pair the incoming stream queue with ownership of its route.
    pub(super) fn new(
        streams: mpsc::Receiver<(C, Done<C>)>,
        registration: Registration<C>,
    ) -> Self {
        StreamAcceptor {
            streams,
            _registration: registration,
        }
    }
}

impl<C: Conn> Acceptor for StreamAcceptor<C> {
    /// A transport connection carrying one incoming data stream.
    type Rx = C;

    /// Receive the next routed stream, or report that its supply has closed.
    async fn accept(&mut self) -> io::Result<(Self::Rx, Done<Self::Rx>)> {
        self.streams.recv().await.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "routed link's stream supply closed",
            )
        })
    }
}
