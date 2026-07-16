//! The streaming mirror: fixed-memory reconciliation over lazy node streams.
//!
//! The drivers here run any two protocol implementors against each other
//! ([`mirror`], or [`handshake`] then [`Handshaken::reconcile`] separately
//! around the version exchange); implementors backed by trees start with
//! either [`materialized::Handshaking::start`] or
//! [`remote::Handshaking::start`].
//!
//! On a wire connection, the peer-level driver first exchanges the shared
//! fixed [`super::handshake`] preamble. Network and intent therefore resolve
//! before the atomic tree snapshot/party fork; this module begins with the
//! subsequent causal-version handshake, exactly as [`super::alternating`] does.

// TODO: remove this when integrated
#![allow(dead_code, unused_imports)]
// Where we're going, we need to write some Complex Types.
#![allow(clippy::type_complexity)]

mod backend;
mod channel;
mod convert;
mod driver;
pub mod materialized;
mod message;
mod protocol;
pub mod remote;
mod tasks;
#[cfg(test)]
mod testing;

pub use backend::{Backend, Leaf, Local, Node, Root};
#[cfg(test)]
pub use testing::{Failing, FailingNode, Failure, Faulting, Operation};

use std::cmp::Ordering;

use super::Error;
use crate::{
    Version,
    tree::typed::height::{Height, Z},
};
use driver::{mirror_connected, try_join_mapped};
use protocol::*;

type ClientConnected<C, B, T> = <<C as Connect<B, T>>::Next as CompleteConnect<B, T>>::Next;
type ServerConnected<S, B, T> = <S as Accept<B, T>>::Next;

pub(crate) struct Handshaken<C, S, B, T>
where
    T: Send + Sync + 'static,
    B: Backend<T, Node<Z>: Leaf<T>>,
    C: Client<B, T>,
    S: Server<B, T>,
{
    client: ClientConnected<C, B, T>,
    server: ServerConnected<S, B, T>,
    our_version: Version,
    peer: message::Handshake,
}

impl<C, S, B, T> Handshaken<C, S, B, T>
where
    T: Send + Sync + 'static,
    B: Backend<T, Node<Z>: Leaf<T>>,
    C: Client<B, T>,
    S: Server<B, T>,
{
    pub(crate) fn peer(&self) -> &message::Handshake {
        let Handshaken { peer, .. } = self;
        peer
    }

    /// Reconcile the two connected sessions, returning both sides' outputs.
    ///
    /// Equal handshake versions resolve each connected state directly to its
    /// output without opening the descent.
    pub(crate) async fn reconcile(
        self,
    ) -> Result<(C::Output, S::Output), Error<C::Error, S::Error>> {
        let Handshaken {
            client: local,
            server: remote,
            our_version,
            peer,
        } = self;
        descend(local, remote, our_version, peer.version).await
    }
}

/// Run two arbitrary protocol implementations through the full schedule.
///
/// Both implementations share one backend `B`, whose node types are the
/// vocabulary crossing between them. Equal handshake versions resolve both
/// connected states without opening the descent.
pub(crate) async fn mirror<C, S, B, T>(
    client: C,
    server: S,
) -> Result<(C::Output, S::Output), Error<C::Error, S::Error>>
where
    T: Send + Sync + 'static,
    B: Backend<T, Node<Z>: Leaf<T>>,
    C: Client<B, T>,
    S: Server<B, T>,
{
    handshake(client, server).await?.reconcile().await
}

/// Exchange versions and return both connected protocol states.
pub(crate) async fn handshake<C, S, B, T>(
    client: C,
    server: S,
) -> Result<Handshaken<C, S, B, T>, Error<C::Error, S::Error>>
where
    T: Send + Sync + 'static,
    B: Backend<T, Node<Z>: Leaf<T>>,
    C: Client<B, T>,
    S: Server<B, T>,
{
    let (our_handshake, client) = client.connect().await.map_err(Error::Client)?;
    let our_version = our_handshake.version.clone();
    let (peer, server) = server.accept(our_handshake).await.map_err(Error::Server)?;
    let client = client
        .complete_connect(peer.version.clone())
        .await
        .map_err(Error::Client)?;

    Ok(Handshaken {
        client,
        server,
        our_version,
        peer,
    })
}

/// Elect the initiator from exchanged versions and reconcile or complete.
pub(crate) async fn descend<L, R, B, T>(
    local: L,
    remote: R,
    local_version: Version,
    remote_version: Version,
) -> Result<(L::Output, R::Output), Error<L::Error, R::Error>>
where
    T: Send + Sync + 'static,
    B: Backend<T, Node<Z>: Leaf<T>>,
    L: Peer<B, T>,
    R: Peer<B, T>,
{
    // Causal versions are only partially ordered, so canonical bytes provide
    // an arbitrary but total and deterministic role tiebreak.
    match remote_version.as_bytes().cmp(local_version.as_bytes()) {
        Ordering::Less => mirror_connected(local, remote).await,
        // Flip the remotely initiated result back into caller order.
        Ordering::Greater => mirror_connected(remote, local)
            .await
            .map(|(theirs, ours)| (ours, theirs))
            .map_err(Error::flip),
        Ordering::Equal => {
            try_join_mapped(
                local.complete_equal(),
                Error::Client,
                remote.complete_equal(),
                Error::Server,
            )
            .await
        }
    }
}

#[cfg(test)]
mod tests;
