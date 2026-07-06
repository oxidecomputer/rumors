//! The root-height stages: version exchange, then the asymmetric opening.
//!
//! [`Handshaking`] holds the whole tree intact while the parties trade
//! versions; the [`responder`](protocol::Responder::responder) and
//! [`open_initiator`](protocol::OpenInitiator::open_initiator) steps then
//! disassemble it into streams, handing off to a [`Descending`] stage. The
//! opening walks themselves live in [`reconcile`].

use futures::channel::mpsc;
use futures::stream::{self, StreamExt};

use crate::tree::mirror::streaming::BoxMessages;
use crate::{
    Version,
    tree::typed::height::{self, UnderRoot, UnderUnderRoot, Z},
};

use super::super::backend::{Backend, Leaf, Material, Node, Root};
use super::super::convert::converted;
use super::super::merge::merge_disjoint;
use super::super::message;
use super::super::protocol::{self, Messages, OutputError, Requests};
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
/// [`responder`](protocol::Responder::responder). `into` is the
/// counterparty's backend: the session's node-carrying output is
/// [converted](super::super::convert) into its node types as it is produced.
pub struct Handshaking<B, O, T, V>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    O: Backend<T, Node<Z>: Leaf<T>>,
{
    backend: B,
    into: O,
    versions: V,
    root: Root<B, T>,
}

impl<B, O, T> Handshaking<B, O, T, Start>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    O: Backend<T, Node<Z>: Leaf<T>>,
{
    pub fn start(backend: B, into: O, root: Root<B, T>) -> Self {
        Self {
            backend,
            into,
            versions: Start {
                our_version: root.ceiling.clone(),
            },
            root,
        }
    }
}

impl<B, O, T, V> protocol::Protocol for Handshaking<B, O, T, V>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    O: Backend<T, Node<Z>: Leaf<T>>,
{
    type Height = height::Root;
    type Output = Root<B, T>;
    type Error = B::Error;
}

impl<B, O, T> protocol::Connect<B, T> for Handshaking<B, O, T, Start>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    O: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Handshaking<B, O, T, Connecting>;

    async fn connect(self) -> Result<(message::Handshake, Self::Next), Self::Error> {
        let Start { our_version } = self.versions;

        let handshake = message::Handshake {
            version: our_version.clone(),
        };
        let next = Handshaking {
            backend: self.backend,
            into: self.into,
            versions: Connecting { our_version },
            root: self.root,
        };
        Ok((handshake, next))
    }
}

impl<B, O, T> protocol::CompleteConnect<B, T> for Handshaking<B, O, T, Connecting>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    O: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Handshaking<B, O, T, Connected>;

    async fn complete_connect(self, their_version: Version) -> Result<Self::Next, Self::Error> {
        Ok(Handshaking {
            backend: self.backend,
            into: self.into,
            versions: Connected {
                our_version: self.versions.our_version,
                their_version,
            },
            root: self.root,
        })
    }
}

impl<B, O, T> protocol::Accept<B, T> for Handshaking<B, O, T, Start>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    O: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Handshaking<B, O, T, Connected>;

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
            into: self.into,
            versions: Connected {
                our_version,
                their_version: request.version,
            },
            root: self.root,
        };
        Ok((handshake, next))
    }
}

impl<B, O, T> protocol::Initiator<B, T> for Handshaking<B, O, T, Connected>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    O: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Self;

    fn initiator(self) -> (impl Messages<message::Initiate, Self::Error>, Self::Next) {
        let initiate = self
            .root
            .root
            .as_ref()
            .map(|node| Ok(message::Initiate::Uncertain(node.hash())));
        (stream::iter(initiate), self)
    }
}

impl<B, O, T> protocol::Responder<B, T> for Handshaking<B, O, T, Connected>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    O: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Descending<B, O, T, UnderRoot>;

    fn responder(
        self,
        requests: impl Requests<message::Initiate>,
    ) -> (
        impl Messages<message::Opening, Self::Error> + 'static,
        Self::Next,
    ) {
        let Handshaking {
            backend,
            into,
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
            into,
            versions.their_version,
            down_rx,
            up_tx,
            work,
            finish,
        );
        (sending, next)
    }
}

impl<B, O, T> protocol::OpenInitiator<B, O, T> for Handshaking<B, O, T, Connected>
where
    B: Backend<T, Materialized = Material, Node<Z>: Leaf<T>>,
    O: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Descending<B, O, T, UnderUnderRoot>;

    fn open_initiator(
        self,
        requests: impl Requests<message::Opening>,
    ) -> (
        BoxMessages<message::Exchanged<O, T, UnderRoot>, OutputError<Self, O, T>>,
        Self::Next,
    ) {
        let Handshaking {
            backend,
            into,
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
        let sending = outgoing(&mut work, converted(backend.clone(), into.clone(), opening));

        let next = Descending::new(
            backend,
            into,
            versions.their_version,
            down_rx,
            up_tx,
            work,
            finish,
        );
        (Box::pin(sending), next)
    }
}
