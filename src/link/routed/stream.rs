//! One link's stream supply: dial-per-open out, routed queue in.
//!
//! Completion recovers the connection on both sides. The write half
//! goes back to its [`Dial`] through [`Dial::recycle`], and the read
//! half returns to the router, which reads its next connect header
//! there. A dropped half drops its connection instead, whose close is
//! the transport half-close the peer observes as an abort.

use std::io;

use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

use super::header::{self, Token};
use super::router::Registration;
use super::{Acceptor, Conn, Connector, Dial, Done};

/// A routed link's [`Connector`]: every open dials one connection to
/// the peer's router and labels it with the link's token.
///
/// Whether "dials" means a fresh connection or a recovered one is the
/// [`Dial`]'s policy: a dial that pools what [`Dial::recycle`] hands
/// back pays no new connection setup for the next stream.
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

    async fn connect(&self) -> io::Result<(Self::Tx, Done<Self::Tx>)> {
        let mut conn = self.dial.dial(&self.peer).await?;
        conn.write_all(&header::stream_header(&self.token)).await?;
        let dial = self.dial.clone();
        let peer = self.peer.clone();
        Ok((conn, Done::new(move |conn| dial.recycle(&peer, conn))))
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
    streams: mpsc::Receiver<(C, Done<C>)>,
    /// Revokes this link's token when the acceptor drops.
    _registration: Registration<C>,
}

impl<C> StreamAcceptor<C> {
    /// Bundle a link's incoming supply around its routed queue and
    /// token claim.
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
    type Rx = C;

    async fn accept(&mut self) -> io::Result<(Self::Rx, Done<Self::Rx>)> {
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
