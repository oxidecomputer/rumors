//! The endpoint: one process's routed-link identity.

use std::io;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::sync::mpsc;

use super::header::{self, Addr, Token};
use super::router::{self, Table};
use super::stream::{StreamAcceptor, StreamConnector};
use super::{Dial, Link, Listen};

/// The [`Link`] type the adapter builds over dialer `D`.
///
/// Both ends of a routed link have this type: the control stream is
/// the split establishment connection, and the stream supply dials
/// [`D::Conn`](Dial::Conn) connections one per stream.
pub type RoutedLink<D> = Link<
    ReadHalf<<D as Dial>::Conn>,
    WriteHalf<<D as Dial>::Conn>,
    StreamConnector<D>,
    StreamAcceptor<<D as Dial>::Conn>,
>;

/// What [`Incoming`] yields per peer-established link.
pub(super) type Arrival<D> = (LinkInfo<<D as Dial>::Addr>, RoutedLink<D>);

/// Capacity knobs of an endpoint's router; [`Config::default`] suits
/// most deployments.
#[derive(Clone, Copy, Debug)]
pub struct Config {
    /// Peer-established links the router holds while the application
    /// catches up on [`Incoming::accept`].
    ///
    /// When the backlog is full, further establishment attempts are
    /// rejected (the dialer's [`Endpoint::link`] fails) rather than
    /// queued without bound; an application that accepts promptly
    /// never fills it.
    pub incoming_backlog: usize,
    /// Inbound connections the router will hold mid-header before it
    /// starts evicting the oldest.
    ///
    /// The bound is hygiene against connections that stall inside
    /// their connect header (the router has no clock, so it evicts by
    /// count, oldest first); wall-clock deadlines belong in the
    /// caller's [`Listen`] wrapper. Anything past the burst of
    /// simultaneous dials the deployment expects is enough.
    pub pending_headers: usize,
}

/// Default [`Config::incoming_backlog`]: a burst of simultaneous
/// peers, not a queueing tier.
const DEFAULT_INCOMING_BACKLOG: usize = 16;

/// Default [`Config::pending_headers`]: comfortably past a full
/// session complement of simultaneous dials from several peers.
const DEFAULT_PENDING_HEADERS: usize = 64;

impl Default for Config {
    fn default() -> Self {
        Config {
            incoming_backlog: DEFAULT_INCOMING_BACKLOG,
            pending_headers: DEFAULT_PENDING_HEADERS,
        }
    }
}

/// How establishing a link can fail; see [`Endpoint::link`].
#[derive(Debug, thiserror::Error)]
pub enum LinkError {
    /// The transport failed under the establishment: the dial itself,
    /// or reading and writing the establishment connection.
    #[error("link establishment transport failure")]
    Io(#[from] io::Error),
    /// The peer's router answered but did not accept the link: its
    /// application is not accepting links, or the listener is not a
    /// routed-link router at all.
    #[error("the peer's router rejected the link")]
    Rejected,
}

/// The identity of a peer-established link, from [`Incoming::accept`].
#[derive(Clone, Debug)]
pub struct LinkInfo<A> {
    /// The establishing peer's advertised name: where this link's
    /// outgoing data streams dial, and the name to re-link with if
    /// the link poisons.
    pub peer: A,
    /// The link's routing identity, unique per link on this endpoint.
    pub token: Token,
}

/// One process's routed-link identity: establishes outbound links and,
/// through the router it is constructed with, terminates inbound ones.
///
/// Cloning is cheap and clones are interchangeable handles onto the
/// same endpoint. See the [module docs](super) for the architecture
/// and an instantiation example.
pub struct Endpoint<D: Dial> {
    inner: Arc<Inner<D>>,
}

impl<D: Dial> Clone for Endpoint<D> {
    fn clone(&self) -> Self {
        Endpoint {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// State shared by an endpoint's clones and its router.
struct Inner<D: Dial> {
    table: Table<D::Conn>,
    dial: D,
    /// The endpoint's advertised name, pre-encoded (validated at
    /// construction) for the `LINK` headers this endpoint writes.
    advertised: Vec<u8>,
}

impl<D: Dial> Endpoint<D> {
    /// Build an endpoint around its transport.
    ///
    /// `advertised` is the name peers dial this endpoint's `listen` at
    /// — it is caller-supplied because it cannot be derived (a bound
    /// address may be unroutable from outside; only the deployment
    /// knows the reachable name). Returns the endpoint, the stream of
    /// peer-established links, and the router future, which the caller
    /// must drive for the endpoint's lifetime (see the [module
    /// docs](super#driving-the-router)).
    ///
    /// # Panics
    ///
    /// If `advertised` encodes to nothing or to more than
    /// [`MAX_ADDR_LEN`](super::MAX_ADDR_LEN) bytes, or if either
    /// [`Config`] bound is zero: each is a configuration bug caught at
    /// construction rather than at the first link.
    pub fn new(
        listen: impl Listen<Conn = D::Conn>,
        advertised: D::Addr,
        dial: D,
        config: Config,
    ) -> (
        Self,
        Incoming<D>,
        impl Future<Output = io::Result<()>> + Send + 'static,
    ) {
        let encoded = advertised.encode();
        assert!(
            (1..=header::MAX_ADDR_LEN).contains(&encoded.len()),
            "advertised name must encode to 1..=255 bytes"
        );
        assert!(
            config.incoming_backlog > 0,
            "incoming backlog must admit a link"
        );
        assert!(
            config.pending_headers > 0,
            "pending headers must admit a connection"
        );
        let table: Table<D::Conn> = Arc::new(Mutex::new(std::collections::HashMap::new()));
        let (arrivals, incoming) = mpsc::channel(config.incoming_backlog);
        let endpoint = Endpoint {
            inner: Arc::new(Inner {
                table: table.clone(),
                dial: dial.clone(),
                advertised: encoded,
            }),
        };
        let router = router::drive(listen, dial, table, arrivals, config.pending_headers);
        (endpoint, Incoming { links: incoming }, router)
    }

    /// Establish one link to the peer reachable at `peer`.
    ///
    /// One round trip: dial the peer's router, announce the link (its
    /// fresh token, this endpoint's advertised name), and wait for the
    /// acknowledgement that the peer's end is registered and on its
    /// way to the peer's application. The connection then carries the
    /// link's control stream.
    ///
    /// Concurrent calls (including both ends linking toward each
    /// other) establish that many independent links; deduplication is
    /// application policy.
    ///
    /// # Errors
    ///
    /// [`LinkError::Io`] for transport failure, [`LinkError::Rejected`]
    /// when the peer answered without acknowledging (its application's
    /// backlog is full, or the listener does not speak this wire).
    /// Either way no link exists and nothing needs cleaning up; retry
    /// policy is the caller's.
    pub async fn link(&self, peer: D::Addr) -> Result<RoutedLink<D>, LinkError> {
        // Register before the header goes out: the peer's reverse
        // dials can only follow its read of the header, so they always
        // find the token routable.
        let (token, registration, streams) = router::register(&self.inner.table);
        let mut conn = self.inner.dial.dial(&peer).await?;
        conn.write_all(&header::link_header(&token, &self.inner.advertised))
            .await?;
        let mut ack = [0; 1];
        match conn.read_exact(&mut ack).await {
            Ok(_) if ack[0] == header::ACK => {}
            // A clean close or a non-acknowledgement byte is the
            // peer's router declining; transport trouble stays Io.
            Ok(_) => return Err(LinkError::Rejected),
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {
                return Err(LinkError::Rejected);
            }
            Err(error) => return Err(LinkError::Io(error)),
        }
        let (control_read, control_write) = tokio::io::split(conn);
        Ok(Link::new(
            control_read,
            control_write,
            StreamConnector::new(self.inner.dial.clone(), peer, token),
            StreamAcceptor::new(streams, registration),
        ))
    }
}

/// The links peers establish toward an endpoint, in arrival order.
///
/// Returned by [`Endpoint::new`]; there is exactly one per endpoint,
/// and dropping it makes the router reject all further establishment
/// attempts (existing links keep routing).
pub struct Incoming<D: Dial> {
    links: mpsc::Receiver<Arrival<D>>,
}

impl<D: Dial> Incoming<D> {
    /// Receive the next peer-established link, or `None` once the
    /// router has stopped (its future resolved or was dropped).
    ///
    /// # Cancel safety
    ///
    /// Dropping the future loses nothing: an undelivered link stays
    /// queued for the next call.
    pub async fn accept(&mut self) -> Option<Arrival<D>> {
        self.links.recv().await
    }
}
