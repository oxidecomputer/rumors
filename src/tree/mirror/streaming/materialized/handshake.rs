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

use super::super::backend::{Backend, Leaf, Root, fold_parents};
use super::super::message;
use super::super::protocol::{self, Requests, Responses};
use super::descend::Descending;
use super::merge::merge_disjoint;
use super::{FAN, reconcile};

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
        impl Responses<message::Opening, Self::Error> + 'static,
        Self::Next,
    ) {
        let Handshaking {
            backend,
            versions,
            root,
        } = self;
        let (down_tx, down_rx) = mpsc::channel(FAN);
        let (up_tx, up_rx) = mpsc::channel(FAN);

        let listing = reconcile::initiate(backend.clone(), root.root, down_tx);

        // The initiator's reconciled root-child level arrives on `up` whole:
        // one fold reassembles the root the terminal resolves to.
        let ceiling = versions.our_version | versions.their_version.clone();
        let top = fold_parents(
            backend.clone(),
            ReceiverStream::new(up_rx).map(Ok::<_, B::Error>),
        );
        let finish = reassemble(top, ceiling);
        let mut work = Vec::new();
        let sending = reconcile::outgoing(&mut work, listing);

        let next = Descending::new(
            backend,
            versions.their_version,
            down_rx,
            up_tx,
            work,
            finish,
        );
        (ReceiverStream::new(sending), next)
    }
}

impl<B, T> protocol::OpenResponder<B, T> for Handshaking<B, T, Connected>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    type Next = Descending<B, T, UnderUnderRoot>;

    fn open_responder(
        self,
        requests: impl Requests<message::Opening>,
    ) -> (
        BoxResponses<message::Exchange<B, T, UnderRoot>, Self::Error>,
        Self::Next,
    ) {
        let Handshaking {
            backend,
            versions,
            root,
        } = self;
        let (down_tx, down_rx) = mpsc::channel(FAN);
        let (level_tx, level_rx) = mpsc::channel(FAN);
        let (up_tx, up_rx) = mpsc::channel(FAN);

        let opening = reconcile::respond(
            backend.clone(),
            versions.their_version.clone(),
            root.root,
            requests,
            down_tx,
            level_tx,
        );

        // The responder's root-child level is the opening's verdicts
        // (`level`) joined with the resolved disputes climbing out of the
        // descent (`up`); two folds reassemble the root the terminal
        // resolves to.
        let ceiling = versions.our_version | versions.their_version.clone();
        let top = fold_parents(
            backend.clone(),
            merge_disjoint(
                ReceiverStream::new(level_rx).map(Ok),
                fold_parents(
                    backend.clone(),
                    ReceiverStream::new(up_rx).map(Ok::<_, B::Error>),
                ),
            ),
        );
        let finish = reassemble(top, ceiling);
        let mut work = Vec::new();
        let sending = reconcile::outgoing(&mut work, opening);

        let next = Descending::new(
            backend,
            versions.their_version,
            down_rx,
            up_tx,
            work,
            finish,
        );
        (Box::pin(ReceiverStream::new(sending)), next)
    }
}

/// Drain the reconciled root level into this side's reconciled [`Root`]: the
/// future the session's terminal resolves to.
///
/// The root level holds at most one real node — the reassembled root, or
/// nothing when the reconciled tree is empty — and the session's watermarks
/// end here: at root height there is no level above to release, so they are
/// dropped. A backend error means there is no trustworthy result.
fn reassemble<B, T>(
    top: impl OptionNodeStream<B, T, height::Root> + 'static,
    ceiling: Version,
) -> BoxFuture<'static, Result<Root<B, T>, B::Error>>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    Box::pin(async move {
        let mut top = pin!(top);
        let mut root = None;
        while let Some(item) = top.next().await {
            let (_prefix, node) = item?;
            let Some(node) = node else { continue };
            debug_assert!(
                root.is_none(),
                "upward reassembly produced more than one root node",
            );
            root = Some(node);
        }
        Ok(Root { ceiling, root })
    })
}
