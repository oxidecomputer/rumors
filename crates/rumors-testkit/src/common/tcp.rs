//! A loopback TCP connection split into the two halves a session takes.
//!
//! One TCP connection is exactly what a session needs: a reliable, ordered
//! duplex byte stream whose directions progress independently, with
//! receiver-paced backpressure and end-of-stream when the peer departs.
//! [`pair`] dials and accepts one loopback connection and splits each end.

use std::io;
use std::net::SocketAddr;

use tokio::net::{
    TcpListener, TcpSocket, TcpStream,
    tcp::{OwnedReadHalf, OwnedWriteHalf},
};

/// One end of a TCP connection: the two halves a session takes.
pub struct TcpEnd {
    /// The read half.
    pub read: OwnedReadHalf,
    /// The write half.
    pub write: OwnedWriteHalf,
}

impl TcpEnd {
    /// Split one connected stream into its halves.
    fn split(stream: TcpStream) -> Self {
        let (read, write) = stream.into_split();
        Self { read, write }
    }
}

/// Dial and accept one loopback TCP connection, returning both ends split.
///
/// `buffers` requests each socket's kernel send and receive buffers, in
/// bytes, so buffering-sensitive checks engage backpressure sooner. The OS
/// rounds the request up to its own floor: the resulting capacity is
/// smaller than the default, not exact. `None` keeps the platform defaults.
pub async fn pair(buffers: Option<u32>) -> io::Result<(TcpEnd, TcpEnd)> {
    let socket = || -> io::Result<TcpSocket> {
        let socket = TcpSocket::new_v4()?;
        if let Some(bytes) = buffers {
            socket.set_send_buffer_size(bytes)?;
            socket.set_recv_buffer_size(bytes)?;
        }
        Ok(socket)
    };
    // Accepted sockets inherit the listener's buffer sizes, so the clamp
    // must land before `listen`.
    let listening = socket()?;
    listening.bind(SocketAddr::from(([127, 0, 0, 1], 0)))?;
    let listener: TcpListener = listening.listen(1)?;
    let addr = listener.local_addr()?;
    let (dialed, accepted) = tokio::join!(socket()?.connect(addr), listener.accept());
    let (accepted, _) = accepted?;
    Ok((TcpEnd::split(dialed?), TcpEnd::split(accepted)))
}
