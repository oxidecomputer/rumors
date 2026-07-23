//! The wire participant's protocol handshake states.

use std::io;
use std::marker::PhantomData;

use borsh::BorshDeserialize;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::{
    Version,
    link::{Acceptor, Connector, Link},
    tree::{
        mirror::{
            framing,
            streaming::{
                Backend, Leaf,
                message::Handshake,
                protocol::{self, Accept, CompleteConnect, Connect},
                remote::{
                    codec::{RunBudget, Speaker, validate_children},
                    proxy::{
                        Connected, Error,
                        work::{Physical, Work},
                    },
                    streams::{AcceptDriver, claims, error_route},
                },
                window::{Window, WindowConfig},
            },
        },
        typed::{
            Hash,
            height::{Root, Z},
        },
    },
};

/// A wire-bound protocol participant ready for the version handshake.
///
/// Consumes a [`Link`] carrier for one session: the control halves host the
/// causal-version handshake (and are the session's output), the connector
/// and acceptor supply the descent's data streams, and the carrier's epoch
/// labels every stream this session opens.
pub struct Handshaking<B, T, R, W, C, A, V = Start>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    backend: B,
    link: Link<R, W, C, A>,
    versions: V,
    /// The session's window choice, resolved against the greeting's
    /// exchanged set sizes; see
    /// [`window`](crate::tree::mirror::streaming::window).
    window: WindowConfig,
    marker: PhantomData<fn() -> T>,
}

impl<B, T, R, W, C, A> Handshaking<B, T, R, W, C, A>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    /// Bind one session's link carrier before exchanging causal versions.
    pub fn start(backend: B, link: Link<R, W, C, A>) -> Self {
        Self {
            backend,
            link,
            versions: Start,
            window: WindowConfig::default(),
            marker: PhantomData,
        }
    }

    /// Select this session's window choice; see
    /// [`window`](crate::tree::mirror::streaming::window).
    pub fn window(mut self, window: WindowConfig) -> Self {
        self.window = window;
        self
    }
}

/// Handshake state before this participant has sent its version.
pub struct Start;

/// The peer greeting received before the local server produces its response.
pub struct Connecting {
    remote: Handshake,
}

impl<B, T, R, W, C, A, V> protocol::Protocol for Handshaking<B, T, R, W, C, A, V>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    R: Send,
    W: Send,
    C: Send,
    A: Send,
    V: Send,
{
    type Height = Root;
    type Error = Error<B::Error>;
    type Output = (R, W);
}

impl<B, T, R, W, C, A> Connect<B, T> for Handshaking<B, T, R, W, C, A>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
    C: Connector,
    A: Acceptor,
{
    type Next = Handshaking<B, T, R, W, C, A, Connecting>;

    /// Receive the remote greeting before asking the local server to answer it.
    async fn connect(mut self) -> Result<(Handshake, Self::Next), Self::Error> {
        let remote = receive::<B::Error, _>(&mut self.link.control_read).await?;
        let handshake = remote.clone();
        let next = Handshaking {
            backend: self.backend,
            link: self.link,
            versions: Connecting { remote },
            window: self.window,
            marker: PhantomData,
        };
        Ok((handshake, next))
    }
}

impl<B, T, R, W, C, A> CompleteConnect<B, T> for Handshaking<B, T, R, W, C, A, Connecting>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
    C: Connector,
    A: Acceptor,
{
    type Next = Connected<B, T, R, W, C, A>;

    /// Send the local server's greeting, then open only if versions differ.
    async fn complete_connect(mut self, theirs: Handshake) -> Result<Self::Next, Self::Error> {
        send::<B::Error, _>(&theirs, &mut self.link.control_write).await?;
        let window = self.window.resolve(
            theirs.set_len,
            self.versions.remote.set_len,
            theirs.max_version_bytes,
            self.versions.remote.max_version_bytes,
            B::node_bytes,
        );
        let budget = run_budget(&theirs, &self.versions.remote);
        Ok(connected(
            self.backend,
            window,
            budget,
            theirs.version,
            self.versions.remote,
            self.link,
        ))
    }
}

impl<B, T, R, W, C, A> Accept<B, T> for Handshaking<B, T, R, W, C, A>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
    C: Connector,
    A: Acceptor,
{
    type Next = Connected<B, T, R, W, C, A>;

    /// Exchange greetings concurrently, then open only if versions differ.
    async fn accept(mut self, request: Handshake) -> Result<(Handshake, Self::Next), Self::Error> {
        let send = send::<B::Error, _>(&request, &mut self.link.control_write);
        let receive = receive::<B::Error, _>(&mut self.link.control_read);
        let (_, remote) = futures_util::future::try_join(send, receive).await?;
        let handshake = remote.clone();
        let window = self.window.resolve(
            request.set_len,
            remote.set_len,
            request.max_version_bytes,
            remote.max_version_bytes,
            B::node_bytes,
        );
        let budget = run_budget(&request, &remote);
        let next = connected(
            self.backend,
            window,
            budget,
            request.version,
            remote,
            self.link,
        );
        Ok((handshake, next))
    }
}

/// The session's supply-run budget: the smaller of the two greetings'
/// targets, so each side's setting bounds both what it builds and what
/// is built for it, and the more memory-constrained end sets the pace.
fn run_budget(ours: &Handshake, theirs: &Handshake) -> RunBudget {
    let bytes = ours.target_message_size.min(theirs.target_message_size);
    RunBudget::from_bytes(usize::try_from(bytes).unwrap_or(usize::MAX))
}

/// Send one greeting: the size-prefixed causal-version frame, then the
/// root-fan listing frame.
///
/// The first frame's body is `set_len (8 B LE) ‖ max_version_bytes
/// (8 B LE) ‖ target_message_size (8 B LE) ‖ version`. Both frames flush
/// on the same hop; the listing frame is the wire carriage of the
/// opening question's content (see [`Handshake`] for the always-carry
/// trade).
async fn send<E, W>(greeting: &Handshake, write: &mut W) -> Result<(), Error<E>>
where
    W: AsyncWrite + Unpin,
{
    let mut write = framing::FrameWrite::new(write);
    let mut first = Vec::with_capacity(24 + greeting.version.as_bytes().len());
    first.extend_from_slice(&greeting.set_len.to_le_bytes());
    first.extend_from_slice(&greeting.max_version_bytes.to_le_bytes());
    first.extend_from_slice(&greeting.target_message_size.to_le_bytes());
    first.extend_from_slice(greeting.version.as_bytes());
    write.frame(&first).await.map_err(Error::HandshakeWrite)?;
    let listing = borsh::to_vec(&greeting.listing).map_err(Error::HandshakeWrite)?;
    write.frame(&listing).await.map_err(Error::HandshakeWrite)
}

/// Receive and canonically decode one greeting: the size-prefixed
/// causal-version frame, then the root-fan listing frame.
///
/// The listing is peer-controlled, so its canonical strictly-ascending radix
/// order is enforced here — the same rule the frame codec applies to a wire
/// query — before any scope is built from it.
async fn receive<E, R>(read: &mut R) -> Result<Handshake, Error<E>>
where
    R: AsyncRead + Unpin,
{
    let mut read = framing::FrameRead::new(read);
    let bytes = read.frame().await.map_err(Error::HandshakeRead)?;
    let short = || {
        Error::HandshakeDecode(io::Error::new(
            io::ErrorKind::InvalidData,
            "greeting version frame is shorter than its size prefixes",
        ))
    };
    let word = |at: usize| {
        bytes
            .get(at..at + 8)
            .and_then(|prefix| <[u8; 8]>::try_from(prefix).ok())
            .map(u64::from_le_bytes)
    };
    let set_len = word(0).ok_or_else(short)?;
    let max_version_bytes = word(8).ok_or_else(short)?;
    let target_message_size = word(16).ok_or_else(short)?;
    let version = Version::try_from_slice(&bytes[24..]).map_err(Error::HandshakeDecode)?;
    let bytes = read.frame().await.map_err(Error::HandshakeRead)?;
    let listing = Vec::<(u8, Hash)>::try_from_slice(&bytes).map_err(Error::HandshakeDecode)?;
    validate_children(&listing).map_err(Error::HandshakeListing)?;
    Ok(Handshake {
        version,
        set_len,
        max_version_bytes,
        target_message_size,
        listing,
    })
}

/// Return untouched control halves on equality, otherwise open the session.
///
/// On equality both carried listings are dropped unused — the documented
/// price of carrying them unconditionally.
fn connected<B, T, R, W, C, A>(
    backend: B,
    window: Window,
    budget: RunBudget,
    local_version: Version,
    remote: Handshake,
    link: Link<R, W, C, A>,
) -> Connected<B, T, R, W, C, A>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: BorshDeserialize + Send + Sync + 'static,
    C: Connector,
    A: Acceptor,
{
    if local_version == remote.version {
        return Connected::equal(link.control_read, link.control_write);
    }
    let local = local_speaker(&local_version, &remote.version);
    open(backend, window, budget, local, remote.listing, link)
}

/// Elect the local physical speaker from the total canonical version order.
fn local_speaker(local: &Version, remote: &Version) -> Speaker {
    match remote.as_bytes().cmp(local.as_bytes()) {
        std::cmp::Ordering::Less => Speaker::Initiator,
        std::cmp::Ordering::Greater => Speaker::Responder,
        std::cmp::Ordering::Equal => unreachable!("equal versions do not open a session"),
    }
}

/// Allocate one session's claim table, error route, and accept driver.
///
/// `peer_listing` is the remote greeting's root-fan listing: the remote's
/// opening question, consumed only when the remote wins the initiator
/// election (and dead weight otherwise, by design).
fn open<B, T, R, W, C, A>(
    backend: B,
    window: Window,
    budget: RunBudget,
    local: Speaker,
    peer_listing: Vec<(u8, Hash)>,
    link: Link<R, W, C, A>,
) -> Connected<B, T, R, W, C, A>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: BorshDeserialize + Send + Sync + 'static,
    C: Connector,
    A: Acceptor,
{
    let Link {
        control_read,
        control_write,
        connector,
        acceptor,
        session,
    } = link;
    let epoch = session.epoch();
    let remote = local.other();
    let (slots, claims) = claims();
    let (route, errors) = error_route();
    let accept = AcceptDriver::new(acceptor, epoch, remote, slots, route.clone());
    let work = Work::new(
        backend,
        window,
        budget,
        peer_listing,
        Physical {
            control_read,
            control_write,
            accept,
            errors,
        },
    );
    Connected::new(remote, epoch, connector, claims, route, work)
}

#[cfg(test)]
mod tests;
