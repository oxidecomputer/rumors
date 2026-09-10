//! [`Link`]s over transports such as TCP: one connection per stream,
//! with a shared listener routing incoming connections to their links.
//!
//! Supply a [`Dial`]/[`Listen`] pair to [`Endpoint::new`]. Each link uses
//! a persistent duplex control connection and opens independent data
//! connections as needed. This preserves the transport's per-connection
//! flow control and close behavior without multiplexing stream payloads.
//!
//! # Driving the endpoint
//!
//! `Endpoint::new` returns an endpoint, an [`Incoming`] link supply, and
//! a router future. Drive that future for the endpoint's lifetime. It
//! resolves on listener failure; dropping it stops the router. An undriven
//! router cannot establish links or deliver new data streams.
//!
//! Call [`Endpoint::link`] to establish a link; the peer receives its end
//! through [`Incoming::accept`]. Both ends must advertise names the other
//! can dial, since data connections may originate on either side. Link
//! establishment registers both ends before allowing data streams to open.
//! Concurrent establishments create independent links, including when two
//! peers call `link` toward each other. The application chooses which to keep.
//!
//! # Connection reuse
//!
//! By default, each link reuses connections from its completed data streams.
//! Reuse avoids repeated connection setup, which can be expensive for
//! authenticated transports. Each link holds at most
//! [`STREAM_COUNT`](crate::link::STREAM_COUNT) idle connections per direction.
//! See [`Config::pooling`] to disable outgoing reuse.
//!
//! Reuse never waits for another stream's consumer. If no completed connection
//! is ready, a fresh one is dialed when needed. Abandoning a stream closes its
//! connection, delivering its accepted bytes followed by EOF.
//!
//! # Failures and transport obligations
//!
//! Unknown link tokens and malformed headers cause a connection to close. A
//! full incoming stream queue closes that link's supply: exceeding the stream
//! limit means the peer violated the Rumors protocol. The router continues
//! serving other links in this case.
//!
//! [`Config::pending_headers`] bounds connections undergoing initial routing.
//! At capacity, acceptance pauses while admitted attempts and connection reuse
//! continue. A stalled attempt retains its slot until its I/O finishes or
//! fails, or its [`Listen::routing_deadline`] expires. That deadline covers
//! initial routing only; established links and idle pooled connections can wait
//! without a time limit between gossip rounds.
//!
//! The adapter discards idle connections whose failure is already visible. A
//! failure discovered during use fails the session. This adapter supplies no
//! clock or background liveness checks: if you need these, implement them in
//! your underlying transport, e.g. by enabling TCP keepalive.
//!
//! Connections must be authenticated and authorized; this adapter provides
//! neither.
//!
//! Validate your own transport with the `conformance` feature's suite of tests
//! for links.
//!
//! # Example: a toy TCP instantiation
//!
//! You would usually not want to do this in production, because plain TCP is
//! neither authenticated nor authorized. Anyone along the network path between
//! gossip nodes could therefore arbitrarily read and write to the set of
//! rumors. A deployment in an untrusted environment should use mutual TLS or
//! similar.
//!
//! ```
//! # tokio::runtime::Builder::new_current_thread().enable_io().build().unwrap().block_on(async {
//! use std::io;
//! use std::net::SocketAddr;
//!
//! use rumors::link::routed::{Config, Dial, Endpoint, Listen};
//! use tokio::net::{TcpListener, TcpStream};
//!
//! /// Opens an outgoing TCP connection.
//! #[derive(Clone)]
//! struct TcpDial;
//!
//! impl Dial for TcpDial {
//!     /// The peer's TCP listen address.
//!     type Addr = SocketAddr;
//!     /// An independent TCP connection.
//!     type Conn = TcpStream;
//!
//!     /// Connect to the peer with Nagle's algorithm disabled.
//!     async fn dial(&self, addr: &SocketAddr) -> io::Result<TcpStream> {
//!         let conn = TcpStream::connect(*addr).await?;
//!         conn.set_nodelay(true)?;
//!         Ok(conn)
//!     }
//! }
//!
//! /// Yields this process's inbound connections to the router.
//! struct TcpListen(TcpListener);
//!
//! impl Listen for TcpListen {
//!     /// An accepted TCP connection.
//!     type Conn = TcpStream;
//!
//!     /// Accept a connection and disable Nagle's algorithm.
//!     async fn accept(&mut self) -> io::Result<TcpStream> {
//!         let conn = self.0.accept().await?.0;
//!         conn.set_nodelay(true)?;
//!         Ok(conn)
//!     }
//! }
//!
//! // Two endpoints in one process, for the example's sake; in a
//! // deployment each process runs one.
//! let a_listener = TcpListener::bind("127.0.0.1:0").await?;
//! let a_addr = a_listener.local_addr()?;
//! let (_a, mut a_incoming, a_router) =
//!     Endpoint::new(TcpListen(a_listener), a_addr, TcpDial, Config::default())
//!         .expect("an unscoped loopback name is routable");
//! tokio::spawn(a_router);
//!
//! let b_listener = TcpListener::bind("127.0.0.1:0").await?;
//! let b_addr = b_listener.local_addr()?;
//! let (b, _b_incoming, b_router) =
//!     Endpoint::new(TcpListen(b_listener), b_addr, TcpDial, Config::default())
//!         .expect("an unscoped loopback name is routable");
//! tokio::spawn(b_router);
//!
//! // b initiates; a's application receives the other end.
//! let (linked, arrival) = tokio::join!(b.link(a_addr), a_incoming.accept());
//! let mut link_at_b = linked.expect("a's router accepts the link");
//! let (info, mut link_at_a) = arrival.expect("router is live");
//! assert_eq!(info.peer, b_addr);
//! // Each side now runs sessions on its link:
//! // `peer.rumors().gossip(&mut link_at_b)`, etc.
//! # let _ = (&mut link_at_a, &mut link_at_b);
//! # io::Result::Ok(()) }).unwrap();
//! ```
//!
//! Validate any instantiation with the `conformance` feature's link
//! suite, exactly as for a hand-built [`Link`].

use std::future::Future;
use std::io;

use tokio::io::{AsyncRead, AsyncWrite};

use super::{Acceptor, Connector, Done, Link};

mod endpoint;
mod header;
mod router;
mod stream;

#[cfg(test)]
mod tests;

pub use endpoint::{Config, Endpoint, EndpointError, Incoming, LinkError, LinkInfo, RoutedLink};
pub use header::{Addr, MAX_ADDR_LEN, Token, Unencodable};
pub use stream::{StreamAcceptor, StreamConnector};

/// A duplex connection with independent flow control.
///
/// Blanket-implemented for types with the required bounds. Routing writes
/// are flushed before waiting for a reply or handing off the connection.
///
/// Dropping a connection must deliver accepted bytes followed by EOF, so
/// abandoning a stream signals its end to the peer. `tokio::net::TcpStream`
/// satisfies this requirement.
pub trait Conn: AsyncRead + AsyncWrite + Unpin + Send + 'static {}

impl<T: AsyncRead + AsyncWrite + Unpin + Send + 'static> Conn for T {}

/// Opens fresh outgoing connections to peers' routers.
///
/// Implementations should authenticate and authorize the peer and configure
/// their own connection, e.g. choosing socket options, keepalive, and dial
/// timeouts. Calls may run concurrently through clones. Each connection must
/// satisfy [`Conn`]'s behavioral contract.
///
/// A pending dial is cancelled if its link establishment or stream open is
/// cancelled. If the transport's handshake cannot safely be cancelled, the
/// implementation must run it in a separate task that can finish independently.
pub trait Dial: Clone + Send + Sync + 'static {
    /// The transport's peer address type.
    type Addr: Addr;

    /// The connection a successful dial yields.
    type Conn: Conn;

    /// Connect to the router at `addr`. An error fails the link establishment
    /// or stream open; the adapter does not retry failed dials.
    fn dial(&self, addr: &Self::Addr) -> impl Future<Output = io::Result<Self::Conn>> + Send;
}

/// Supplies inbound connections to an endpoint's router.
///
/// An accept error stops the router. Handle transient failures inside the
/// implementation if accepting should continue after them.
///
/// Accept must be cancellation-safe: the router repeatedly drops pending
/// calls, and the next call must still be able to receive any arriving
/// connection.
pub trait Listen: Send + 'static {
    /// The connection an accept yields.
    type Conn: Conn;

    /// Accept the next inbound connection.
    fn accept(&mut self) -> impl Future<Output = io::Result<Self::Conn>> + Send;

    /// Time-bound the initial routing of a freshly accepted connection.
    ///
    /// The router calls this method once after each successful
    /// [`accept`](Self::accept). If it completes before the connection supplies
    /// a routing header and flushes its corresponding acknowledgement, the
    /// router closes that connection attempt and releases its slot without
    /// stopping the listener.
    ///
    /// Successful routing discards the deadline. It does not apply to
    /// established links or pooled reuse, so idle waits between gossip rounds
    /// remain unrestricted.
    ///
    /// The default never expires. Override it with your runtime's timer, such
    /// as `tokio::time::sleep(limit)`; Rumors supplies no clock.
    fn routing_deadline(&self) -> impl Future<Output = ()> + Send + 'static {
        std::future::pending()
    }
}
