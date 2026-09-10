//! The endpoint: one process's routed-link identity.

use std::io;
use std::sync::{Arc, Weak};

use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf, split};
use tokio::sync::mpsc;

use super::header::{self, Addr, Token, Unencodable};
use super::router::{Router, Table};
use super::stream::{StreamAcceptor, StreamConnector};
use super::{Dial, Link, Listen};

/// A routed link over transport `D`, returned at both ends of establishment.
pub type RoutedLink<D> = Link<
    ReadHalf<<D as Dial>::Conn>,
    WriteHalf<<D as Dial>::Conn>,
    StreamConnector<D>,
    StreamAcceptor<<D as Dial>::Conn>,
>;

/// What [`Incoming`] yields per peer-established link.
pub(super) type Arrival<D> = (LinkInfo<<D as Dial>::Addr>, RoutedLink<D>);

/// Router capacities and outgoing connection reuse.
#[derive(Clone, Copy, Debug)]
pub struct Config {
    /// Maximum peer-established links waiting for [`Incoming::accept`].
    /// Further establishment attempts are rejected while the backlog is full.
    pub incoming_backlog: usize,
    /// Maximum fresh connections undergoing initial routing, including a new
    /// link's acknowledgement.
    ///
    /// At capacity, the router pauses [`Listen::accept`] until an attempt finishes
    /// or fails. The transport's backlog determines whether further arrivals
    /// wait or are refused.
    ///
    /// Stalled connections retain their slots until I/O fails or their
    /// [`Listen::routing_deadline`] expires. Idle connections from completed
    /// streams have a separate per-link bound and can still be reused at capacity.
    pub pending_headers: usize,
    /// Reuse completed outgoing connections within each link. Enabled by
    /// default; disable it when connection setup is cheap and retaining idle
    /// connections costs more than redialing.
    ///
    /// Each link retains at most [`STREAM_COUNT`](crate::link::STREAM_COUNT)
    /// outgoing connections until reuse or link teardown. The router also
    /// admits that many idle incoming connections per link, independently of
    /// this setting, so peers may choose whether to reuse their connections.
    pub pooling: bool,
}

impl Default for Config {
    /// Use the default queue capacities with outgoing pooling enabled.
    fn default() -> Self {
        Config {
            incoming_backlog: 16,
            pending_headers: 64,
            pooling: true,
        }
    }
}

/// Invalid endpoint configuration; see [`Endpoint::new`].
#[derive(Debug, thiserror::Error)]
pub enum EndpointError {
    /// The address type cannot faithfully encode the advertised name.
    #[error("the advertised name has no wire encoding")]
    Unencodable(#[from] Unencodable),
    /// The advertised name encodes outside 1..=[`MAX_ADDR_LEN`](super::MAX_ADDR_LEN) bytes.
    #[error(
        "the advertised name must encode to 1..={max} bytes, not {0}",
        max = header::MAX_ADDR_LEN
    )]
    NameLength(usize),
    /// [`Config::incoming_backlog`] is zero.
    #[error("incoming backlog must admit a link")]
    ZeroIncomingBacklog,
    /// [`Config::pending_headers`] is zero.
    #[error("pending headers must admit a connection")]
    ZeroPendingHeaders,
}

/// How establishing a link can fail; see [`Endpoint::link`].
#[derive(Debug, thiserror::Error)]
pub enum LinkError {
    /// The router stopped, or dialing or establishment I/O failed.
    #[error("link establishment transport failure")]
    Io(#[from] io::Error),
    /// The peer closed the connection or replied without accepting the link.
    #[error("the peer's router rejected the link")]
    Rejected,
}

/// The identity of a peer-established link, from [`Incoming::accept`].
#[derive(Clone, Debug)]
pub struct LinkInfo<A> {
    /// The peer's advertised name, used to open this link's outgoing streams.
    pub peer: A,
    /// The link's routing identity, unique per link on this endpoint.
    pub token: Token,
}

/// Establishes outgoing links and accepts incoming links through its router.
///
/// Clones share the same endpoint. See the [module example](super) for setup.
pub struct Endpoint<D: Dial> {
    /// Shared dialing configuration and access to the router's table.
    inner: Arc<Inner<D>>,
}

impl<D: Dial> Clone for Endpoint<D> {
    /// Share this endpoint's identity, dialer, and routing state.
    fn clone(&self) -> Self {
        Endpoint {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// State shared by an endpoint's clones and its router.
struct Inner<D: Dial> {
    /// Registers outgoing links without keeping a stopped router alive.
    table: Weak<Table<D::Conn>>,
    /// Opens each outgoing link's control connection and data streams.
    dial: D,
    /// The endpoint's advertised name, as given at construction.
    local_addr: D::Addr,
    /// Validated once at construction and reused in establishment headers.
    encoded: Vec<u8>,
    /// Outgoing reuse policy copied into each link's connector.
    pooling: bool,
}

impl<D: Dial> Endpoint<D> {
    /// Build an endpoint, its incoming link supply, and its router future.
    ///
    /// `advertised` must name this listener as seen by peers. It may differ
    /// from the local bind address. Drive the returned router for the
    /// endpoint's lifetime; see [driving the endpoint](super#driving-the-endpoint).
    ///
    /// # Errors
    ///
    /// Returns [`EndpointError`] if the advertised name cannot be encoded,
    /// its encoded length is outside 1..=[`MAX_ADDR_LEN`](super::MAX_ADDR_LEN),
    /// or either router capacity is zero.
    pub fn new(
        listen: impl Listen<Conn = D::Conn>,
        advertised: D::Addr,
        dial: D,
        config: Config,
    ) -> Result<
        (
            Self,
            Incoming<D>,
            impl Future<Output = io::Result<()>> + Send + 'static,
        ),
        EndpointError,
    > {
        let encoded = advertised.encode()?;
        if !(1..=header::MAX_ADDR_LEN).contains(&encoded.len()) {
            return Err(EndpointError::NameLength(encoded.len()));
        }
        if config.incoming_backlog == 0 {
            return Err(EndpointError::ZeroIncomingBacklog);
        }
        if config.pending_headers == 0 {
            return Err(EndpointError::ZeroPendingHeaders);
        }
        let table = Arc::new(Table::new());
        let (arrivals, incoming) = mpsc::channel(config.incoming_backlog);
        let endpoint = Endpoint {
            inner: Arc::new(Inner {
                table: Arc::downgrade(&table),
                dial: dial.clone(),
                local_addr: advertised,
                encoded,
                pooling: config.pooling,
            }),
        };
        let router = Router::new(dial, table, arrivals, config).run(listen);
        Ok((endpoint, Incoming { links: incoming }, router))
    }

    /// The name peers dial this endpoint at: `advertised`, as given at
    /// construction.
    pub fn local_addr(&self) -> &D::Addr {
        &self.inner.local_addr
    }

    /// Establish an independent link to `peer` using a fresh control connection.
    ///
    /// Returns a link ready for data streams. The peer receives its end
    /// through [`Incoming::accept`]. Concurrent calls create separate links;
    /// deduplication is application policy.
    /// Apply a caller-supplied timeout to this future to bound both dialing
    /// and routing; cancelling it closes the connection and releases the route.
    ///
    /// # Errors
    ///
    /// Returns [`LinkError::Io`] if the router has stopped or I/O fails,
    /// or [`LinkError::Rejected`] if the peer does not acknowledge the link.
    /// Failed attempts release their registration; retry policy belongs to
    /// the caller.
    pub async fn link(&self, peer: D::Addr) -> Result<RoutedLink<D>, LinkError> {
        // Register before sending: the peer may dial back as soon as it
        // receives this header.
        let (token, registration, streams) = {
            let table = self.inner.table.upgrade().ok_or_else(|| {
                io::Error::new(io::ErrorKind::BrokenPipe, "the router has stopped")
            })?;
            table.register()
        };
        let mut conn = self.inner.dial.dial(&peer).await?;
        conn.write_all(&header::link_header(&token, &self.inner.encoded))
            .await?;
        conn.flush().await?;
        let mut ack = [0; 1];
        match conn.read_exact(&mut ack).await {
            Ok(_) if ack[0] == header::ACK => {}
            // EOF or another byte rejects the link; other I/O errors retain
            // their transport cause.
            Ok(_) => return Err(LinkError::Rejected),
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {
                return Err(LinkError::Rejected);
            }
            Err(error) => return Err(LinkError::Io(error)),
        }
        let (control_read, control_write) = split(conn);
        Ok(Link::new(
            control_read,
            control_write,
            StreamConnector::new(self.inner.dial.clone(), peer, token, self.inner.pooling),
            StreamAcceptor::new(streams, registration),
        ))
    }
}

/// Incoming links queued by the router.
///
/// Dropping this supply rejects further establishments. Existing links
/// continue to route streams.
pub struct Incoming<D: Dial> {
    /// Links acknowledged by the router and waiting for application pickup.
    links: mpsc::Receiver<Arrival<D>>,
}

impl<D: Dial> Incoming<D> {
    /// Receive the next incoming link. Once the router stops, queued links
    /// remain available; calls return `None` after the queue drains.
    ///
    /// Cancelling the call preserves undelivered links for the next call.
    pub async fn accept(&mut self) -> Option<Arrival<D>> {
        self.links.recv().await
    }
}
