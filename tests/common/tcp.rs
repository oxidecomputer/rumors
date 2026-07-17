//! A per-session TCP [`Link`] for the inter-process simulations.
//!
//! This is the "one connection per stream" instantiation of the link
//! contract at its simplest: every session gets its own dedicated listener
//! on each side, so no routing header or connection table is needed — a
//! connection arriving at a session's listener *is* one of that session's
//! streams. The caller supplies the initial TCP connection; [`link`] turns
//! it into the session's control stream and derives the stream supply:
//!
//! 1. each side binds a fresh ephemeral listener for its incoming streams;
//! 2. the two sides swap listener ports as the first bytes on the control
//!    connection (before any protocol byte);
//! 3. `connect` dials the peer's listener and keeps the write half;
//!    `accept` takes the read half of the next arriving connection.
//!
//! Per-stream flow control and half-close come from TCP itself, one socket
//! per stream, which is exactly what the contract's independence clause
//! asks for. A production deployment would keep one listener per *process*
//! and route by connect header instead (see the `link` module docs); the
//! per-session listener here trades that machinery for obviousness, which
//! is the right trade in a test.

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use rumors::link::{Acceptor, Connector, Link};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{
    TcpListener, TcpStream,
    tcp::{OwnedReadHalf, OwnedWriteHalf},
};

/// The TCP link built by [`link`].
pub type TcpLink = Link<OwnedReadHalf, OwnedWriteHalf, TcpConnector, TcpAcceptor>;

/// Turn one established TCP connection into a session's [`Link`].
///
/// Both ends of the connection must call this before starting a session:
/// the listener-port swap consumes the connection's first two bytes in each
/// direction.
pub async fn link(control: TcpStream) -> io::Result<TcpLink> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();

    let (mut control_read, mut control_write) = control.into_split();
    control_write.write_all(&port.to_be_bytes()).await?;
    let mut peer_port = [0u8; 2];
    control_read.read_exact(&mut peer_port).await?;
    let peer = SocketAddr::from(([127, 0, 0, 1], u16::from_be_bytes(peer_port)));

    Ok(Link::new(
        control_read,
        control_write,
        TcpConnector(Arc::new(peer)),
        TcpAcceptor(listener),
    ))
}

/// Dials one TCP connection per outgoing stream.
#[derive(Clone)]
pub struct TcpConnector(Arc<SocketAddr>);

impl Connector for TcpConnector {
    type Tx = OwnedWriteHalf;

    async fn connect(&self) -> io::Result<Self::Tx> {
        let stream = TcpStream::connect(*self.0).await?;
        let (read, write) = stream.into_split();
        // The stream is unidirectional: only the write half is used, and
        // dropping it later half-closes toward the peer. The unread half is
        // dropped now; the peer never writes on this socket.
        drop(read);
        Ok(write)
    }
}

/// Accepts one TCP connection per incoming stream.
pub struct TcpAcceptor(TcpListener);

impl Acceptor for TcpAcceptor {
    type Rx = OwnedReadHalf;

    async fn accept(&mut self) -> io::Result<Self::Rx> {
        let (stream, _) = self.0.accept().await?;
        let (read, write) = stream.into_split();
        // Half-close our unused direction immediately; the peer never reads
        // this socket, so the shutdown is invisible to it.
        drop(write);
        Ok(read)
    }
}
