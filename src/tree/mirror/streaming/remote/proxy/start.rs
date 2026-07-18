//! The wire participant's protocol handshake states.

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
                    codec::{RunBudget, Speaker},
                    proxy::{
                        Connected, Error,
                        work::{Physical, Work},
                    },
                    streams::{AcceptDriver, claims, error_route},
                },
                window::Window,
            },
        },
        typed::height::{Root, Z},
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
    /// The session's pipeline window; see
    /// [`window`](crate::tree::mirror::streaming::window).
    window: Window,
    /// The encoder's supply-run byte budget; see [`RunBudget`].
    budget: RunBudget,
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
            window: Window::default(),
            budget: RunBudget::default(),
            marker: PhantomData,
        }
    }

    /// Select this session's pipeline window; see
    /// [`window`](crate::tree::mirror::streaming::window).
    pub fn window(mut self, window: Window) -> Self {
        self.window = window;
        self
    }

    /// Select this session's supply-run byte budget; see [`RunBudget`].
    pub fn run_budget(mut self, budget: RunBudget) -> Self {
        self.budget = budget;
        self
    }
}

/// Handshake state before this participant has sent its version.
pub struct Start;

/// The peer version received before the local server produces its response.
pub struct Connecting {
    remote: Version,
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
        let handshake = Handshake {
            version: remote.clone(),
        };
        let next = Handshaking {
            backend: self.backend,
            link: self.link,
            versions: Connecting { remote },
            window: self.window,
            budget: self.budget,
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
    async fn complete_connect(mut self, local_version: Version) -> Result<Self::Next, Self::Error> {
        send::<B::Error, _>(&local_version, &mut self.link.control_write).await?;
        Ok(connected(
            self.backend,
            self.window,
            self.budget,
            local_version,
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
        let send = send::<B::Error, _>(&request.version, &mut self.link.control_write);
        let receive = receive::<B::Error, _>(&mut self.link.control_read);
        let (_, remote) = futures_util::future::try_join(send, receive).await?;
        let handshake = Handshake {
            version: remote.clone(),
        };
        let next = connected(
            self.backend,
            self.window,
            self.budget,
            request.version,
            remote,
            self.link,
        );
        Ok((handshake, next))
    }
}

/// Send one exactly bounded causal-version handshake frame.
async fn send<E, W>(version: &Version, write: &mut W) -> Result<(), Error<E>>
where
    W: AsyncWrite + Unpin,
{
    framing::FrameWrite::new(write)
        .frame(version.as_bytes())
        .await
        .map_err(Error::HandshakeWrite)
}

/// Receive and canonically decode one causal-version handshake frame.
async fn receive<E, R>(read: &mut R) -> Result<Version, Error<E>>
where
    R: AsyncRead + Unpin,
{
    let bytes = framing::FrameRead::new(read)
        .frame()
        .await
        .map_err(Error::HandshakeRead)?;
    Version::try_from_slice(&bytes).map_err(Error::HandshakeDecode)
}

/// Return untouched control halves on equality, otherwise open the session.
fn connected<B, T, R, W, C, A>(
    backend: B,
    window: Window,
    budget: RunBudget,
    local_version: Version,
    remote_version: Version,
    link: Link<R, W, C, A>,
) -> Connected<B, T, R, W, C, A>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: BorshDeserialize + Send + Sync + 'static,
    C: Connector,
    A: Acceptor,
{
    if local_version == remote_version {
        return Connected::equal(link.control_read, link.control_write);
    }
    let local = local_speaker(&local_version, &remote_version);
    open(backend, window, budget, local, link)
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
fn open<B, T, R, W, C, A>(
    backend: B,
    window: Window,
    budget: RunBudget,
    local: Speaker,
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
    let epoch = session.epoch;
    let remote = local.other();
    let (slots, claims) = claims();
    let (route, errors) = error_route();
    let accept = AcceptDriver::new(acceptor, epoch, remote, slots, route.clone());
    let work = Work::new(
        backend,
        window,
        budget,
        Physical {
            control_read,
            control_write,
            accept,
            errors,
        },
    );
    Connected::new(remote, epoch, connector, claims, route, work)
}
