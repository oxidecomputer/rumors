//! [`Link`]s over accept/connect transports: one connection per stream,
//! routed by a per-process listener.
//!
//! A [`Link`] wants one persistent control stream plus lazily opened,
//! independently flow-controlled data streams. Transports with native
//! substreams (QUIC) map onto that directly; an accept/connect transport
//! (TCP and everything shaped like it) offers exactly one primitive:
//! dial a name, get a byte stream. This module is the adapter between
//! the two shapes, generic over the transport: the caller supplies a
//! [`Dial`]/[`Listen`] pair, and the adapter maps **every link stream to
//! its own connection**, so per-stream flow control and half-close are
//! the transport's own, which is precisely what the link contract's
//! independence clause asks for.
//!
//! What makes that routable is a small connect header. Every dialed
//! connection opens by naming the link it belongs to (a 16-byte random
//! *token* minted at link establishment), and one **router** per
//! [`Endpoint`] owns the listener: it reads each arriving connection's
//! header and hands the connection — whole, never its bytes — to that
//! link's bounded queue, where the link's [`Acceptor`] collects it. A
//! link's first connection carries the control stream; each later one
//! carries a single unidirectional data stream at a time.
//!
//! A data stream ends one of two ways, the link contract's completion
//! clause. Completed at its clean end, the connection outlives the
//! stream: the write half goes back to its [`Dial`] (see
//! [`Dial::recycle`]), the read half back to the router, which reads
//! the connection's next header there — so a dial that pools recycled
//! connections carries stream after stream over one connection, paying
//! connection setup once. Dropped instead, the connection closes with
//! the stream, which is how the peer observes the abort.
//!
//! # Driving the router
//!
//! The crate requires no runtime, so the router is a future the caller
//! drives: [`Endpoint::new`] returns it alongside the endpoint, and the
//! caller spawns it (or selects over it) for the endpoint's lifetime.
//! An undriven router accepts no links and routes no streams — sessions
//! on existing links stall until the caller's timeout cancels them. The
//! future resolves only on listener failure ([`Listen::accept`]
//! erroring is fatal to the endpoint); dropping it is the shutdown.
//!
//! # Establishing links
//!
//! [`Endpoint::link`] dials the peer's router, sends the link's token
//! together with this endpoint's own advertised name, and waits for the
//! peer router's one-byte acknowledgement; the peer's application
//! receives the other end from [`Incoming::accept`]. The advertised
//! name rides along because the accept side must dial back for its own
//! outgoing data streams, and only the dialer knows the name it is
//! reachable at (the accepted connection's source is an ephemeral
//! port). The acknowledgement makes token registration on both routers
//! happen strictly before either end can open a data stream, so a
//! stream dial can never race ahead of its own link.
//!
//! Both ends may [`link`](Endpoint::link) toward each other
//! concurrently; the result is two independent links. The adapter does
//! not deduplicate: which links to hold and gossip on is application
//! policy, as on every other transport.
//!
//! # Failure modes
//!
//! - A connection bearing an unknown token (a link the application
//!   already discarded, a dial chasing a stale name) is dropped; the
//!   dialing side's session fails as transport failure, poisoning its
//!   link, and the application re-links. Failures heal at session
//!   granularity.
//! - A link whose stream queue overflows is evicted wholesale: an
//!   honest peer never has more than a session's worth of streams in
//!   flight, so overflow proves misbehavior (or a local bug), and
//!   evicting the link turns it into an ordinary transport failure
//!   instead of a silent lost delivery. The router never blocks on a
//!   full queue.
//! - A silently dead path hangs a stream open or write until the
//!   caller's session timeout cancels the session, the same backstop
//!   every transport relies on. Liveness probing (keepalive) belongs in
//!   the caller's [`Dial`] — as does discovering that a pooled
//!   connection died while idle, which surfaces as the recycled
//!   stream's transport failure and heals at session granularity.
//! - Every lazy stream open pays one dial. Where the dial itself is
//!   the expensive step (an authenticating transport, say), a [`Dial`]
//!   that pools recycled connections pays it once per connection
//!   rather than once per stream; a transport with native substreams
//!   avoids the per-stream open entirely.
//!
//! # What the transport must provide
//!
//! Two obligations on [`Dial::Conn`] reach beyond its trait bounds, and
//! the conformance suite observes both (see [`Conn`]):
//!
//! - bytes accepted by `poll_write` become visible to the peer without
//!   an explicit flush;
//! - dropping a connection delivers already-written bytes and then
//!   end-of-stream to the peer.
//!
//! Security is the transport's, per the [link contract's security
//! division](crate::link#what-securing-the-transport-means): the
//! adapter assumes `Dial`/`Listen` connections are authenticated,
//! integrity-protected, and fresh. The token routes connections; it is
//! not a credential, and the router's violation handling (dropping
//! malformed or unknown arrivals) is a conformance bug detector, not a
//! security boundary.
//!
//! # Example: a TCP instantiation
//!
//! ```
//! # tokio::runtime::Builder::new_current_thread().enable_io().build().unwrap().block_on(async {
//! use std::io;
//! use std::net::SocketAddr;
//!
//! use rumors::link::routed::{Config, Dial, Endpoint, Listen};
//! use tokio::net::{TcpListener, TcpStream};
//!
//! /// Dials one TCP connection per link stream.
//! #[derive(Clone)]
//! struct TcpDial;
//!
//! impl Dial for TcpDial {
//!     type Addr = SocketAddr;
//!     type Conn = TcpStream;
//!
//!     async fn dial(&self, addr: &SocketAddr) -> io::Result<TcpStream> {
//!         TcpStream::connect(*addr).await
//!     }
//! }
//!
//! /// Yields this process's inbound connections to the router.
//! struct TcpListen(TcpListener);
//!
//! impl Listen for TcpListen {
//!     type Conn = TcpStream;
//!
//!     async fn accept(&mut self) -> io::Result<TcpStream> {
//!         Ok(self.0.accept().await?.0)
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

/// A byte-stream connection the adapter can route: one per link stream.
///
/// Blanket-implemented for every type with the bounds; the real
/// contract is two obligations the bounds cannot express, both load-
/// bearing for the link contract:
///
/// - **No hidden write buffering.** Bytes accepted by `poll_write`
///   must become visible to the peer without an explicit flush: the
///   session (and the conformance suite) awaits peer reactions to
///   unflushed writes. A connection wrapped in a write buffer that
///   holds bytes until it fills stalls the first such exchange.
/// - **Drop is half-close.** Dropping the connection must deliver all
///   already-written bytes and then end-of-stream to the peer: an
///   aborted data stream ends by dropping its connection, and the
///   peer reads to end-of-stream. A drop that discards queued bytes,
///   or never signals the peer, breaks stream teardown. (A *completed*
///   stream never drops its connection — the adapter recovers it; see
///   [`Dial::recycle`].)
///
/// `tokio::net::TcpStream` satisfies both, as does anything else whose
/// writes land in the transport as they are accepted.
pub trait Conn: AsyncRead + AsyncWrite + Unpin + Send + 'static {}

impl<T: AsyncRead + AsyncWrite + Unpin + Send + 'static> Conn for T {}

/// Opens outgoing connections to peers' routers, by name.
///
/// This is the adapter's outgoing seam, and the place transport policy
/// lives: socket options, liveness probing (keepalive), security
/// wrapping, and dial timeouts are all the implementation's own. Dials
/// arrive concurrently through clones (one per in-flight stream open),
/// so implementations hold shared state behind cheap handles.
///
/// # Errors
///
/// A dial failure fails the stream open (and with it the session)
/// as transport failure; the adapter never retries.
pub trait Dial: Clone + Send + Sync + 'static {
    /// The name this transport dials by; see [`Addr`].
    type Addr: Addr;

    /// The connection a dial yields.
    type Conn: Conn;

    /// Open one connection to the router reachable at `addr`.
    fn dial(&self, addr: &Self::Addr) -> impl Future<Output = io::Result<Self::Conn>> + Send;

    /// Take back a connection to `peer` whose stream completed cleanly.
    ///
    /// The connection rests exactly where a fresh dial's would: the
    /// peer's router is reading for its next connect header, so a dial
    /// that hands it out again reuses the connection for another stream
    /// in place of a new connection's setup. The default drops it,
    /// which suits transports whose connections are cheap; implement it
    /// (a pool keyed by peer, typically) where connection setup is the
    /// latency that matters. A connection whose stream failed or was
    /// abandoned never comes back through here: the adapter drops it,
    /// so a recycled connection is never mid-stream.
    ///
    /// Two cautions. Recycle runs on the session's task at the
    /// stream's completion, so it must not block. And recycling
    /// certifies nothing about liveness: the peer's router evicts idle
    /// connections by count, so a pooled connection may be dead and
    /// discovered only by the stream that draws it.
    fn recycle(&self, _peer: &Self::Addr, conn: Self::Conn) {
        drop(conn);
    }
}

/// Yields inbound connections to an endpoint's router.
///
/// The router is this listener's only consumer, selecting over
/// [`accept`](Self::accept) in its loop.
///
/// # Errors
///
/// An accept failure is fatal to the endpoint: the router resolves
/// with the error and stops routing. A transport whose accept can fail
/// transiently (resource exhaustion, say) handles the retry inside its
/// own `accept`.
///
/// # Cancel safety
///
/// The router holds `accept` in a `select!` loop, so the future is
/// dropped and re-created continuously; a connection mid-accept must
/// not be lost to the drop.
pub trait Listen: Send + 'static {
    /// The connection an accept yields.
    type Conn: Conn;

    /// Accept the next inbound connection.
    fn accept(&mut self) -> impl Future<Output = io::Result<Self::Conn>> + Send;
}
