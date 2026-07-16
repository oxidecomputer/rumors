//! The wire participant's protocol handshake states.

use std::marker::PhantomData;

use tokio::io::{AsyncRead, AsyncWrite};

use crate::{
    Version,
    tree::{
        mirror::streaming::{
            Backend, Leaf, Node,
            message::Handshake,
            protocol::{self, Accept, CompleteConnect, Connect},
            remote::{
                codec::Speaker,
                proxy::{Connected, Error},
                session::Drivers,
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
    version: Version,
    read: R,
    write: W,
    state: PhantomData<fn() -> (T, V)>,
}

impl<B, T, R, W> Handshaking<B, T, R, W>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    /// Bind a known remote version to its ordered transport halves.
    pub fn start(backend: B, version: Version, read: R, write: W) -> Self {
        Self {
            backend,
            version,
            read,
            write,
            state: PhantomData,
        }
    }
}

/// Handshake state before this participant has sent its version.
pub struct Start;

/// Handshake state after this participant has sent its version as the client.
pub struct Connecting;

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

    /// Present the already-received remote version to the local server.
    async fn connect(self) -> Result<(Handshake, Self::Next), Self::Error> {
        let handshake = Handshake {
            version: self.version.clone(),
        };
        let next = Handshaking {
            backend: self.backend,
            version: self.version,
            read: self.read,
            write: self.write,
            state: PhantomData,
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

    /// Elect the local server's role and open the physical session.
    async fn complete_connect(self, local_version: Version) -> Result<Self::Next, Self::Error> {
        let local = local_speaker(&local_version, &self.version, Speaker::Responder);
        Ok(open(self.backend, local, self.read, self.write))
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

    /// Elect the local client's role and return the known remote version.
    async fn accept(self, request: Handshake) -> Result<(Handshake, Self::Next), Self::Error> {
        let local = local_speaker(&request.version, &self.version, Speaker::Initiator);
        let handshake = Handshake {
            version: self.version,
        };
        Ok((handshake, open(self.backend, local, self.read, self.write)))
    }
}

/// Elect one local speaker with a connection-side default for equal versions.
fn local_speaker(local: &Version, remote: &Version, equal: Speaker) -> Speaker {
    match remote.as_bytes().cmp(local.as_bytes()) {
        std::cmp::Ordering::Less => Speaker::Initiator,
        std::cmp::Ordering::Greater => Speaker::Responder,
        std::cmp::Ordering::Equal => equal,
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
