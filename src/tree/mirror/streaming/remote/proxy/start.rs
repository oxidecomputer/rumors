//! The wire participant's protocol handshake states.

use std::marker::PhantomData;

use borsh::BorshDeserialize;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::{
    Version,
    tree::{
        mirror::{
            framing,
            streaming::{
                Backend, Leaf,
                message::Handshake,
                protocol::{self, Accept, CompleteConnect, Connect},
                remote::{
                    codec::Speaker,
                    proxy::{Connected, Error},
                    session::Drivers,
                },
            },
        },
        typed::height::{Root, Z},
    },
};

/// A wire-bound protocol participant ready for the version handshake.
pub struct Handshaking<B, T, R, W, V = Start>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    backend: B,
    read: R,
    write: W,
    versions: V,
    marker: PhantomData<fn() -> T>,
}

impl<B, T, R, W> Handshaking<B, T, R, W>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    /// Bind the ordered transport halves before exchanging causal versions.
    pub fn start(backend: B, read: R, write: W) -> Self {
        Self {
            backend,
            read,
            write,
            versions: Start,
            marker: PhantomData,
        }
    }
}

/// Handshake state before this participant has sent its version.
pub struct Start;

/// The peer version received before the local server produces its response.
pub struct Connecting {
    remote: Version,
}

impl<B, T, R, W, V> protocol::Protocol for Handshaking<B, T, R, W, V>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    R: Send,
    W: Send,
    V: Send,
{
    type Height = Root;
    type Error = Error<B::Error>;
    type Output = (R, W);
}

impl<B, T, R, W> Connect<B, T> for Handshaking<B, T, R, W>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    type Next = Handshaking<B, T, R, W, Connecting>;

    /// Receive the remote greeting before asking the local server to answer it.
    async fn connect(mut self) -> Result<(Handshake, Self::Next), Self::Error> {
        let remote = receive::<B::Error, _>(&mut self.read).await?;
        let handshake = Handshake {
            version: remote.clone(),
        };
        let next = Handshaking {
            backend: self.backend,
            read: self.read,
            write: self.write,
            versions: Connecting { remote },
            marker: PhantomData,
        };
        Ok((handshake, next))
    }
}

impl<B, T, R, W> CompleteConnect<B, T> for Handshaking<B, T, R, W, Connecting>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    type Next = Connected<B, T, R, W>;

    /// Send the local server's greeting, then open only if versions differ.
    async fn complete_connect(mut self, local_version: Version) -> Result<Self::Next, Self::Error> {
        send::<B::Error, _>(&local_version, &mut self.write).await?;
        Ok(connected(
            self.backend,
            local_version,
            self.versions.remote,
            self.read,
            self.write,
        ))
    }
}

impl<B, T, R, W> Accept<B, T> for Handshaking<B, T, R, W>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    type Next = Connected<B, T, R, W>;

    /// Exchange greetings concurrently, then open only if versions differ.
    async fn accept(mut self, request: Handshake) -> Result<(Handshake, Self::Next), Self::Error> {
        let send = send::<B::Error, _>(&request.version, &mut self.write);
        let receive = receive::<B::Error, _>(&mut self.read);
        let (_, remote) = futures_util::future::try_join(send, receive).await?;
        let handshake = Handshake {
            version: remote.clone(),
        };
        let next = connected(self.backend, request.version, remote, self.read, self.write);
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

/// Return untouched transport on equality, otherwise open the elected session.
fn connected<B, T, R, W>(
    backend: B,
    local_version: Version,
    remote_version: Version,
    read: R,
    write: W,
) -> Connected<B, T, R, W>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    if local_version == remote_version {
        return Connected::equal(read, write);
    }
    let local = local_speaker(&local_version, &remote_version);
    open(backend, local, read, write)
}

/// Elect the local physical speaker from the total canonical version order.
fn local_speaker(local: &Version, remote: &Version) -> Speaker {
    match remote.as_bytes().cmp(local.as_bytes()) {
        std::cmp::Ordering::Less => Speaker::Initiator,
        std::cmp::Ordering::Greater => Speaker::Responder,
        std::cmp::Ordering::Equal => unreachable!("equal versions do not open a session"),
    }
}

/// Create logical endpoints and retain their drivers as protocol work.
fn open<B, T, R, W>(backend: B, local: Speaker, read: R, write: W) -> Connected<B, T, R, W>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    let (drivers, incoming, outgoing) = Drivers::new(local, read, write);
    Connected::new(backend, local.other(), incoming, outgoing, drivers)
}
