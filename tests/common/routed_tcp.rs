//! The TCP instantiation of the routed link's transport seam.
//!
//! [`rumors::link::routed`] is generic over a [`Dial`]/[`Listen`] pair;
//! this is that pair over real sockets, with optional kernel-buffer
//! clamps so the conformance suite can shrink per-stream capacity to
//! the OS floor. Socket policy stops here: the adapter above sees only
//! byte streams.

use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use rumors::link::routed::{Dial, Listen};
use tokio::net::{TcpListener, TcpSocket, TcpStream};

/// Listener backlog: a full session complement of simultaneous stream
/// dials from a handful of peers, with room to spare.
const BACKLOG: u32 = 256;

/// Dials one TCP connection per routed-link connection.
#[derive(Clone)]
pub struct TcpDial {
    /// Send-buffer clamp for each dialed socket; `None` keeps the
    /// platform default.
    pub send_buffer: Option<u32>,
}

impl Dial for TcpDial {
    type Addr = SocketAddr;
    type Conn = TcpStream;

    async fn dial(&self, addr: &SocketAddr) -> io::Result<TcpStream> {
        match self.send_buffer {
            None => TcpStream::connect(*addr).await,
            Some(send) => {
                let socket = TcpSocket::new_v4()?;
                socket.set_send_buffer_size(send)?;
                socket.connect(*addr).await
            }
        }
    }
}

/// A TCP dialer pooling recycled connections per peer, so completed
/// streams ride recycled connections instead of fresh dials.
#[derive(Clone, Default)]
pub struct PoolingTcpDial {
    pool: Arc<Mutex<HashMap<SocketAddr, Vec<TcpStream>>>>,
}

impl Dial for PoolingTcpDial {
    type Addr = SocketAddr;
    type Conn = TcpStream;

    async fn dial(&self, addr: &SocketAddr) -> io::Result<TcpStream> {
        let pooled = self
            .pool
            .lock()
            .expect("pool lock")
            .get_mut(addr)
            .and_then(Vec::pop);
        match pooled {
            Some(conn) => Ok(conn),
            None => TcpStream::connect(*addr).await,
        }
    }

    fn recycle(&self, peer: &SocketAddr, conn: TcpStream) {
        self.pool
            .lock()
            .expect("pool lock")
            .entry(*peer)
            .or_default()
            .push(conn);
    }
}

/// Accepts one process's inbound routed-link connections.
pub struct TcpListen(TcpListener);

impl TcpListen {
    /// Bind a fresh loopback listener, reporting the address peers
    /// dial (and the endpoint advertises).
    ///
    /// `recv_buffer` clamps each accepted socket's receive buffer
    /// toward the request (accepted sockets inherit the listener's, so
    /// the clamp must land before `listen`); `None` keeps the platform
    /// default.
    pub async fn bind(recv_buffer: Option<u32>) -> io::Result<(Self, SocketAddr)> {
        let listener = match recv_buffer {
            None => TcpListener::bind("127.0.0.1:0").await?,
            Some(recv) => {
                let socket = TcpSocket::new_v4()?;
                socket.set_recv_buffer_size(recv)?;
                socket.bind(SocketAddr::from(([127, 0, 0, 1], 0)))?;
                socket.listen(BACKLOG)?
            }
        };
        let addr = listener.local_addr()?;
        Ok((TcpListen(listener), addr))
    }
}

impl Listen for TcpListen {
    type Conn = TcpStream;

    async fn accept(&mut self) -> io::Result<TcpStream> {
        Ok(self.0.accept().await?.0)
    }
}
