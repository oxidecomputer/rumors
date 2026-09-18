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
//! - [`erased`]: the height-erased seam both implementors run on — the
//!   wire vocabulary's erased twin, its typed exits, and the dispatch
//!   back into the height-typed backend surface.
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
//! fixed [`super::preamble`]. Network and intent therefore resolve
//! before the atomic tree snapshot/party fork; this module begins with the
//! subsequent greeting exchange, the one message each side sends before any
//! frame flows. What the greeting carries — and why each field rides this
//! early — is documented at its definition, [`message::Greeting`].

// Where we're going, we need to write some Complex Types.
#![allow(clippy::type_complexity)]

mod backend;
mod channel;
pub(crate) mod convert;
mod driver;
mod erased;
pub mod materialized;
pub(crate) mod message;
mod protocol;
pub mod remote;
pub mod stats;
mod tasks;
#[cfg(test)]
mod testing;
pub(crate) mod window;

pub use backend::{Backend, ErasedNode, Leaf, Local, Node, Root};
// The stream vocabulary the backend conformance suite decorates with;
// crate-visible alongside the suite itself.
#[cfg(test)]
pub(crate) use backend::BoxNodeStream;
#[cfg(test)]
pub use backend::NodeStream;
#[cfg(test)]
pub use testing::{
    Failing, FailingNode, Failure, Fault, Faulting, GreetingLie, Operation, ReplyCorruption,
};

use futures::future::BoxFuture;

use super::Error;
use crate::tree::typed::height::Z;
use driver::{mirror_connected, try_join_mapped};
use message::RoleKey;
use protocol::*;

/// A client after both greetings have been exchanged.
type ClientConnected<C, B> = <<C as Connect<B>>::Next as CompleteConnect<B>>::Next;
/// A server after both greetings have been exchanged.
type ServerConnected<S, B> = <S as Accept<B>>::Next;

/// Both participants after the greeting exchange, with the role-election keys.
pub(crate) struct Handshaken<C, S, B>
where
    B: Backend<Node<Z>: Leaf>,
    C: Client<B>,
    S: Server<B>,
{
    /// The connected client participant.
    client: ClientConnected<C, B>,
    /// The connected server participant.
    server: ServerConnected<S, B>,
    /// The local greeting fields used to elect roles.
    local_key: RoleKey,
    /// The peer's greeting fields used to elect roles.
    remote_key: RoleKey,
}

/// Operations available after both greetings have been exchanged.
impl<C, S, B> Handshaken<C, S, B>
where
    B: Backend<Node<Z>: Leaf>,
    C: Client<B>,
    S: Server<B>,
{
    /// Return the version advertised by the peer.
    pub(crate) fn remote_version(&self) -> &crate::Version {
        self.remote_key.version()
    }

    /// Reconcile the two connected sessions, returning both sides' outputs.
    ///
    /// Equal greeting versions resolve each connected state directly to its
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
                local_key,
                remote_key,
            } = self;
            descend(local, remote, local_key, remote_key).await
        })
    }
}

/// Run two arbitrary protocol implementations through the full schedule.
///
/// Both implementations share one backend `B`, whose node types are the
/// vocabulary crossing between them. Equal greeting versions resolve both
/// connected states without opening the descent.
#[cfg(test)]
pub(crate) async fn mirror<C, S, B>(
    client: C,
    server: S,
) -> Result<(C::Output, S::Output), Error<C::Error, S::Error>>
where
    B: Backend<Node<Z>: Leaf>,
    C: Client<B>,
    S: Server<B>,
{
    handshake(client, server).await?.reconcile().await
}

/// Exchange greetings and return both connected protocol states.
pub(crate) async fn handshake<C, S, B>(
    client: C,
    server: S,
) -> Result<Handshaken<C, S, B>, Error<C::Error, S::Error>>
where
    B: Backend<Node<Z>: Leaf>,
    C: Client<B>,
    S: Server<B>,
{
    let (our_greeting, client) = client.connect().await.map_err(Error::Client)?;
    let local_key = our_greeting.role_key();
    let (peer, server) = server.accept(our_greeting).await.map_err(Error::Server)?;
    let remote_key = peer.role_key();
    let client = client.complete_connect(peer).await.map_err(Error::Client)?;

    Ok(Handshaken {
        client,
        server,
        local_key,
        remote_key,
    })
}

/// Elect the initiator from the exchanged greetings and reconcile or complete.
pub(crate) async fn descend<L, R, B>(
    local: L,
    remote: R,
    local_key: RoleKey,
    remote_key: RoleKey,
) -> Result<(L::Output, R::Output), Error<L::Error, R::Error>>
where
    B: Backend<Node<Z>: Leaf>,
    L: Peer<B>,
    R: Peer<B>,
{
    if local_key.version() == remote_key.version() {
        return try_join_mapped(
            local.complete_equal(),
            Error::Client,
            remote.complete_equal(),
            Error::Server,
        )
        .await;
    }
    if local_key.initiates(&remote_key) {
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
