//! The root-height stages: version exchange, then the asymmetric opening.
//!
//! [`Handshaking`] holds the whole tree intact while the parties trade
//! versions; the [`initiator`](protocol::Initiator::initiator) and
//! [`open_responder`](protocol::OpenResponder::open_responder) steps then
//! disassemble it into streams, handing off to a [`Descending`] stage. The
//! opening walks themselves live in [`reconcile`].

use std::pin::pin;

use futures::future::BoxFuture;
use futures::stream::StreamExt;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_stream::wrappers::ReceiverStream;

use crate::tree::mirror::streaming::backend::OptionNodeStream;
use crate::tree::mirror::streaming::protocol::BoxResponses;
use crate::{
    Version,
    tree::typed::height::{self, UnderRoot, UnderUnderRoot, Z},
};

use super::super::backend::{Backend, Leaf, Root};
use super::super::message;
use super::super::protocol::{self, Requests, Responses};
use super::FAN;

/// The version state of a stage that has been opened but has not yet sent its
/// handshake.
pub struct Start {
    our_version: Version,
}

/// The version state of a stage that has sent its version but not yet received
/// the peer's.
pub struct Connecting {
    our_version: Version,
}

/// The version state of a stage that has exchanged versions with its peer and
/// can proceed with reconciliation.
pub struct Connected {
    our_version: Version,
    their_version: Version,
}

/// A mirror stage still at [`Root`](height::Root) height: the handshake
/// phases, before the tree has been disassembled into streams.
///
/// `V` is the version state ([`Start`] → [`Connecting`] → [`Connected`]). The
/// whole tree is held intact as `root` until reconciliation begins at
/// [`initiator`](protocol::Initiator::initiator) /
/// [`open_responder`](protocol::OpenResponder::open_responder). The session's
/// outgoing messages carry `backend`'s own node types, which are the ones its
/// counterparty reads.
pub struct Handshaking<B, T, V>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
{
    backend: B,
    versions: V,
    root: Root<B, T>,
}

impl<B, T> Handshaking<B, T, Start>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
{
    pub fn start(backend: B, root: Root<B, T>) -> Self {
        Self {
            backend,
            versions: Start {
                our_version: root.ceiling.clone(),
            },
            root,
        }
    }
}

impl<B, T, V: Send> protocol::Protocol for Handshaking<B, T, V>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
{
    type Height = height::Root;
    type Output = Root<B, T>;
    type Error = B::Error;
}

impl<B, T> protocol::Connect<B, T> for Handshaking<B, T, Start>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Handshaking<B, T, Connecting>;

    async fn connect(self) -> Result<(message::Handshake, Self::Next), Self::Error> {
        let Start { our_version } = self.versions;

        let handshake = message::Handshake {
            version: our_version.clone(),
        };
        let next = Handshaking {
            backend: self.backend,
            versions: Connecting { our_version },
            root: self.root,
        };
        Ok((handshake, next))
    }
}

impl<B, T> protocol::CompleteConnect<B, T> for Handshaking<B, T, Connecting>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Handshaking<B, T, Connected>;

    async fn complete_connect(self, their_version: Version) -> Result<Self::Next, Self::Error> {
        Ok(Handshaking {
            backend: self.backend,
            versions: Connected {
                our_version: self.versions.our_version,
                their_version,
            },
            root: self.root,
        })
    }
}

impl<B, T> protocol::Accept<B, T> for Handshaking<B, T, Start>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Handshaking<B, T, Connected>;

    async fn accept(
        self,
        request: message::Handshake,
    ) -> Result<(message::Handshake, Self::Next), Self::Error> {
        let Start { our_version } = self.versions;

        let handshake = message::Handshake {
            version: our_version.clone(),
        };
        let next = Handshaking {
            backend: self.backend,
            versions: Connected {
                our_version,
                their_version: request.version,
            },
            root: self.root,
        };
        Ok((handshake, next))
    }
}

impl<B, T> protocol::Initiator<B, T> for Handshaking<B, T, Connected>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Descending<B, T, UnderRoot>;

    fn initiator(
        self,
    ) -> (
        impl Responses<message::Initiate, Self::Error> + 'static,
        Self::Next,
    ) {
        let Handshaking {
            backend,
            versions,
            root,
        } = self;

        todo!()
    }
}

impl<B, T> protocol::Responder<B, T> for Handshaking<B, T, Connected>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Descending<B, T, UnderUnderRoot>;

    fn responder(
        self,
        requests: impl Requests<message::Initiate>,
    ) -> (
        BoxResponses<message::Reply<B, T, UnderRoot>, Self::Error>,
        Self::Next,
    ) {
        let Handshaking {
            backend,
            versions,
            root,
        } = self;

        todo!()
    }
}
