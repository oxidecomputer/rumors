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

use rumors::link::{Acceptor, Connector, Link, STREAM_COUNT};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{
    TcpListener, TcpSocket, TcpStream,
    tcp::{OwnedReadHalf, OwnedWriteHalf},
};

/// The TCP link built by [`link`].
pub type TcpLink = Link<OwnedReadHalf, OwnedWriteHalf, TcpConnector, TcpAcceptor>;

/// Listener backlog for the sized per-stream listener.
///
/// Sized from the protocol's own concurrency bound: a session opens at most
/// [`STREAM_COUNT`] streams per direction, and lazy opens can burst before
/// the acceptor task runs, so the backlog holds a whole complement with
/// room to spare.
const STREAM_BACKLOG: u32 = STREAM_COUNT as u32 * 4;

/// Turn one established TCP connection into a session's [`Link`].
///
/// Both ends of the connection must call this before starting a session:
/// the listener-port swap consumes the connection's first two bytes in each
/// direction.
pub async fn link(control: TcpStream) -> io::Result<TcpLink> {
    link_with_stream_buffers(control, None).await
}

/// [`link`], with each per-stream socket's kernel buffers clamped toward `bytes`.
///
/// A stream here is one unidirectional socket, so its capacity is the
/// dialer's send buffer plus the listener side's receive buffer; requesting
/// `bytes` for both shrinks per-stream capacity so buffering-sensitive
/// checks engage backpressure sooner. The OS rounds the request up to its
/// own floor: the resulting capacity is smaller than the default, not
/// exact. `None` keeps the platform defaults.
pub async fn link_with_stream_buffers(
    control: TcpStream,
    bytes: Option<u32>,
) -> io::Result<TcpLink> {
    let listener = match bytes {
        None => TcpListener::bind("127.0.0.1:0").await?,
        Some(recv) => {
            // Accepted sockets inherit the listener's receive buffer, so
            // the clamp must land before `listen`.
            let socket = TcpSocket::new_v4()?;
            socket.set_recv_buffer_size(recv)?;
            socket.bind(SocketAddr::from(([127, 0, 0, 1], 0)))?;
            socket.listen(STREAM_BACKLOG)?
        }
    };
    let port = listener.local_addr()?.port();

    let (mut control_read, mut control_write) = control.into_split();
    control_write.write_all(&port.to_be_bytes()).await?;
    let mut peer_port = [0u8; 2];
    control_read.read_exact(&mut peer_port).await?;
    let peer = SocketAddr::from(([127, 0, 0, 1], u16::from_be_bytes(peer_port)));

    Ok(Link::new(
        control_read,
        control_write,
        TcpConnector(Arc::new(Target {
            peer,
            send_buffer: bytes,
        })),
        TcpAcceptor(listener),
    ))
}

/// Where [`TcpConnector`] dials, and how its sockets are sized.
struct Target {
    /// The peer's per-stream listener.
    peer: SocketAddr,
    /// Send-buffer clamp for each dialed socket; `None` keeps the default.
    send_buffer: Option<u32>,
}

/// Dials one TCP connection per outgoing stream.
#[derive(Clone)]
pub struct TcpConnector(Arc<Target>);

impl Connector for TcpConnector {
    type Tx = OwnedWriteHalf;

    async fn connect(&self) -> io::Result<Self::Tx> {
        let stream = match self.0.send_buffer {
            None => TcpStream::connect(self.0.peer).await?,
            Some(send) => {
                let socket = TcpSocket::new_v4()?;
                socket.set_send_buffer_size(send)?;
                socket.connect(self.0.peer).await?
            }
        };
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
