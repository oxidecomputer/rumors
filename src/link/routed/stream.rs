//! One link's stream supply: dial-per-open out, routed queue in.

use std::io;

use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

use super::header::{self, Token};
use super::router::Registration;
use super::{Acceptor, Conn, Connector, Dial};

/// A routed link's [`Connector`]: every open dials one fresh
/// connection to the peer's router and labels it with the link's
/// token.
///
/// The returned stream *is* the connection, so dropping it is the
/// transport half-close (the peer reads the final bytes, then
/// end-of-stream), and no open ever waits on another stream's
/// progress: the opens share nothing but the dialer.
pub struct StreamConnector<D: Dial> {
    dial: D,
    peer: D::Addr,
    token: Token,
}

impl<D: Dial> StreamConnector<D> {
    /// Bundle a link's outgoing supply: dial `peer`, quoting `token`.
    pub(super) fn new(dial: D, peer: D::Addr, token: Token) -> Self {
        StreamConnector { dial, peer, token }
    }
}

impl<D: Dial> Clone for StreamConnector<D> {
    fn clone(&self) -> Self {
        StreamConnector {
            dial: self.dial.clone(),
            peer: self.peer.clone(),
            token: self.token,
        }
    }
}

impl<D: Dial> Connector for StreamConnector<D> {
    type Tx = D::Conn;

    async fn connect(&self) -> io::Result<Self::Tx> {
        let mut conn = self.dial.dial(&self.peer).await?;
        conn.write_all(&header::stream_header(&self.token)).await?;
        Ok(conn)
    }
}

/// A routed link's [`Acceptor`]: drains the bounded queue the router
/// fills with this link's inbound stream connections.
///
/// Receiving from the queue is cancel-safe (an undelivered connection
/// stays queued for the next accept), and the acceptor carries the
/// link's claim on its routing token, tying the routing to the link's
/// own lifetime: dropping the link revokes its token at that moment.
pub struct StreamAcceptor<C> {
    streams: mpsc::Receiver<C>,
    /// Revokes this link's token when the acceptor drops.
    _registration: Registration<C>,
}

impl<C> StreamAcceptor<C> {
    /// Bundle a link's incoming supply around its routed queue and
    /// token claim.
    pub(super) fn new(streams: mpsc::Receiver<C>, registration: Registration<C>) -> Self {
        StreamAcceptor {
            streams,
            _registration: registration,
        }
    }
}

impl<C: Conn> Acceptor for StreamAcceptor<C> {
    type Rx = C;

    async fn accept(&mut self) -> io::Result<Self::Rx> {
        // The router holds this queue's only sender, and removes it
        // exactly when it evicts the link (a queue overflow, which
        // proves peer misbehavior). Queued deliveries drain first, so
        // eviction surfaces on the first accept past them.
        self.streams.recv().await.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "link evicted by the router: stream queue overflowed",
            )
        })
    }
}
