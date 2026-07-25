//! The streaming mirror: fixed-memory reconciliation over lazy node streams.
//!
//! The streaming mirror reconciles two replicas under a fixed memory bound,
//! descending the tree over lazily opened per-level streams. Two implementor
//! roles recur through every layer below: *the walk* is the in-process
//! participant, the protocol run directly over a backend's tree; *the proxy*
//! is the wire-bound participant, the same protocol driven by frames on a
//! link. The layers, each separate for one reason:
//!
//! - [`backend`]: materiality — what a node *is* and what holding one costs.
//! - [`protocol`]: the type-level phase schedule every implementor advances
//!   through.
//! - [`materialized`]: the walk, and the home of the deadlock-freedom
//!   argument.
//! - [`remote`]: the proxy — codec, proxy state machine, adapter, stream
//!   binding.
//! - [`window`]: how one byte budget becomes per-height channel capacities.
//! - [`message`]: the wire vocabulary, the greeting included.
//! - [`convert`]: the leaf conversion boundary between backends.
//! - [`driver`], [`channel`], [`tasks`]: plumbing — phase scheduling and
//!   error routing, named bounded edges, task completion.
//!
//! Start reading at [`materialized`]'s session-dataflow section, then
//! [`remote`].
//!
//! The drivers here run any two protocol implementors against each other. The
//! peer path uses [`handshake`] then [`Handshaken::reconcile`] around the
//! version exchange; tests also expose a whole-session convenience.
//! Implementors backed by trees start with either
//! [`materialized::Handshaking::start`] or [`remote::Handshaking::start`].
//!
//! On a wire connection, the peer-level driver first exchanges the shared
//! fixed [`super::handshake`] preamble. Network and intent therefore resolve
//! before the atomic tree snapshot/party fork; this module begins with the
//! subsequent greeting exchange, the one message each side sends before any
//! frame flows. What the greeting carries — and why each field rides this
//! early — is documented at its definition, [`message::Greeting`]. The
//! selectable V1 alternating protocol is an independent behavioral oracle
//! with a greeting of its own.

// Where we're going, we need to write some Complex Types.
#![allow(clippy::type_complexity)]

mod backend;
mod channel;
pub(crate) mod convert;
mod driver;
pub mod materialized;
mod message;
mod protocol;
pub mod remote;
mod tasks;
#[cfg(test)]
mod testing;
pub(crate) mod window;

pub use backend::{Backend, Leaf, Local, Node, Root};
// The stream vocabulary the backend conformance suite decorates with;
// crate-visible alongside the suite itself.
#[cfg(test)]
pub(crate) use backend::BoxNodeStream;
#[cfg(test)]
pub use backend::NodeStream;
#[cfg(test)]
pub use testing::{Failing, FailingNode, Failure, Faulting, Operation};

use futures::future::BoxFuture;

use super::Error;
use crate::{Version, tree::typed::height::Z};
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
    /// Our advertised live message count: our half of the role election's
    /// primary key ([`message::initiates`]).
    our_len: u64,
    peer: message::Greeting,
}

impl<C, S, B, T> Handshaken<C, S, B, T>
where
    T: Send + Sync + 'static,
    B: Backend<T, Node<Z>: Leaf<T>>,
    C: Client<B, T>,
    S: Server<B, T>,
{
    pub(crate) fn peer(&self) -> &message::Greeting {
        let Handshaken { peer, .. } = self;
        peer
    }

    /// Reconcile the two connected sessions, returning both sides' outputs.
    ///
    /// Equal handshake versions resolve each connected state directly to its
    /// output without opening the descent.
    pub(crate) fn reconcile<'a>(
        self,
    ) -> BoxFuture<'a, Result<(C::Output, S::Output), Error<C::Error, S::Error>>>
    where
        Self: 'a,
    {
        Box::pin(async move {
            let Handshaken {
                client: local,
                server: remote,
                our_version,
                our_len,
                peer,
            } = self;
            descend(
                local,
                remote,
                our_version,
                our_len,
                peer.version,
                peer.set_len,
            )
            .await
        })
    }
}

/// Run two arbitrary protocol implementations through the full schedule.
///
/// Both implementations share one backend `B`, whose node types are the
/// vocabulary crossing between them. Equal handshake versions resolve both
/// connected states without opening the descent.
#[cfg(test)]
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
    let our_len = our_handshake.set_len;
    let (peer, server) = server.accept(our_handshake).await.map_err(Error::Server)?;
    let client = client
        .complete_connect(peer.clone())
        .await
        .map_err(Error::Client)?;

    Ok(Handshaken {
        client,
        server,
        our_version,
        our_len,
        peer,
    })
}

/// Elect the initiator from the exchanged greetings and reconcile or complete.
pub(crate) async fn descend<L, R, B, T>(
    local: L,
    remote: R,
    local_version: Version,
    local_len: u64,
    remote_version: Version,
    remote_len: u64,
) -> Result<(L::Output, R::Output), Error<L::Error, R::Error>>
where
    T: Send + Sync + 'static,
    B: Backend<T, Node<Z>: Leaf<T>>,
    L: Peer<B, T>,
    R: Peer<B, T>,
{
    if local_version == remote_version {
        return try_join_mapped(
            local.complete_equal(),
            Error::Client,
            remote.complete_equal(),
            Error::Server,
        )
        .await;
    }
    // The role election of record: the smaller exchanged set initiates,
    // canonical version bytes break ties (`message::initiates`).
    if message::initiates(local_len, &local_version, remote_len, &remote_version) {
        mirror_connected(local, remote).await
    } else {
        // Flip the remotely initiated result back into caller order.
        mirror_connected(remote, local)
            .await
            .map(|(theirs, ours)| (ours, theirs))
            .map_err(Error::flip)
    }
}

#[cfg(test)]
mod tests;
