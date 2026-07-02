//! The root-height stages: version exchange, then the asymmetric opening.
//!
//! [`Handshaking`] holds the whole tree intact while the parties trade
//! versions; the [`responder`](protocol::Responder::responder) and
//! [`open_initiator`](protocol::OpenInitiator::open_initiator) steps then
//! disassemble it into streams, handing off to a [`Descending`] stage. The
//! opening walks themselves live in [`reconcile`].

use futures::channel::mpsc;
use futures::stream::{self, StreamExt};

use crate::{
    Version,
    tree::typed::height::{self, UnderRoot, UnderUnderRoot, Z},
};

use super::super::backend::{Backend, Leaf, Material, Node, Root};
use super::super::merge::merge_disjoint;
use super::super::message;
use super::super::protocol::{self, Messages};
use super::descend::Descending;
use super::{FAN, outgoing, reassemble, reconcile};

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
/// [`responder`](protocol::Responder::responder).
pub struct Handshaking<B, T, V>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
{
    backend: B,
    versions: V,
    root: Root<B, T>,
}

impl<B, T> Handshaking<B, T, Start>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
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

impl<B, T, V> protocol::Stage for Handshaking<B, T, V>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
{
    type Height = height::Root;
}

impl<B, T> protocol::Connect<B, T> for Handshaking<B, T, Start>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Handshaking<B, T, Connecting>;

    async fn connect<E>(self) -> Result<(message::Handshake, Self::Next), E>
    where
        E: From<B::Error> + Send + 'static,
    {
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
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Handshaking<B, T, Connected>;

    async fn complete_connect<E>(self, their_version: Version) -> Result<Self::Next, E>
    where
        E: From<B::Error> + Send + 'static,
    {
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
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Handshaking<B, T, Connected>;

    async fn accept<E>(
        self,
        request: message::Handshake,
    ) -> Result<(message::Handshake, Self::Next), E>
    where
        E: From<B::Error> + Send + 'static,
    {
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
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Self;

    fn initiator<E>(self) -> (impl Messages<message::Initiate, E> + 'static, Self::Next)
    where
        E: From<B::Error> + Send + 'static,
    {
        let initiate = self
            .root
            .root
            .as_ref()
            .map(|node| Ok(message::Initiate::Uncertain(node.hash())));
        (stream::iter(initiate), self)
    }
}

impl<B, T> protocol::Responder<B, T> for Handshaking<B, T, Connected>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Descending<B, T, UnderRoot>;

    fn responder<E>(
        self,
        requests: impl Messages<message::Initiate, E> + 'static,
    ) -> (impl Messages<message::Opening, E> + 'static, Self::Next)
    where
        E: From<B::Error> + Send + 'static,
    {
        let Handshaking {
            backend,
            versions,
            root,
        } = self;
        let (down_tx, down_rx) = mpsc::channel(FAN);
        let (up_tx, up_rx) = mpsc::channel(FAN);

        let listing = reconcile::respond(backend.clone(), root.root, requests, down_tx);

        // The responder's reconciled root-child level arrives on `up` whole:
        // one fold reassembles the root the terminal resolves to.
        let ceiling = versions.our_version | versions.their_version.clone();
        let top = backend.clone().parents(up_rx.map(Ok::<_, B::Error>));
        let finish = reassemble(top, ceiling);
        let mut work = Vec::new();
        let sending = outgoing(&mut work, listing);

        let next = Descending::new(
            backend,
            versions.their_version,
            down_rx,
            up_tx,
            work,
            finish,
        );
        (sending, next)
    }
}

impl<B, T> protocol::OpenInitiator<B, T> for Handshaking<B, T, Connected>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Descending<B, T, UnderUnderRoot>;

    fn open_initiator<E>(
        self,
        requests: impl Messages<message::Opening, E> + 'static,
    ) -> (
        impl Messages<message::Exchanged<B, T, UnderRoot>, E> + 'static,
        Self::Next,
    )
    where
        E: From<B::Error> + Send + 'static,
    {
        let Handshaking {
            backend,
            versions,
            root,
        } = self;
        let (down_tx, down_rx) = mpsc::channel(FAN);
        let (level_tx, level_rx) = mpsc::channel(FAN);
        let (up_tx, up_rx) = mpsc::channel(FAN);

        let opening = reconcile::open(
            backend.clone(),
            versions.their_version.clone(),
            root.root,
            requests,
            down_tx,
            level_tx,
        );

        // The initiator's root-child level is the opening's verdicts
        // (`level`) joined with the resolved disputes climbing out of the
        // descent (`up`); two folds reassemble the root the terminal
        // resolves to.
        let ceiling = versions.our_version | versions.their_version.clone();
        let top = backend.clone().parents(merge_disjoint(
            level_rx.map(Ok::<_, B::Error>),
            backend.clone().parents(up_rx.map(Ok)),
        ));
        let finish = reassemble(top, ceiling);
        let mut work = Vec::new();
        let sending = outgoing(&mut work, opening);

        let next = Descending::new(
            backend,
            versions.their_version,
            down_rx,
            up_tx,
            work,
            finish,
        );
        (sending, next)
    }
}
