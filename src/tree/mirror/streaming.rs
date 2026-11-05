//! The streaming mirror: fixed-memory reconciliation over lazy node streams.
//!
//! The streaming mirror reconciles two replicas under a fixed memory bound,
//! descending the tree over lazily populated per-level streams. Two
//! implementor roles recur through every layer below: *the walk* is the
//! in-process participant, the protocol run directly over a backend's tree;
//! *the proxy* is the wire-bound participant, the same protocol driven by
//! frames on one connection. The layers, each separate for one reason:
//!
//! - [`backend`]: materiality — what a node *is* and what holding one costs.
//! - [`protocol`]: the type-level phase schedule every implementor advances
//!   through.
//! - [`materialized`]: the walk, and the home of the deadlock-freedom
//!   argument.
//! - [`remote`]: the proxy — codec, proxy state machine, adapter, and the
//!   pipe that carries every logical stream.
//! - [`ledger`]: how each endpoint admits its work against its byte budget.
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

// The height-indexed protocol schedule necessarily composes many associated types.
#![allow(clippy::type_complexity)]

mod backend;
mod channel;
pub(crate) mod convert;
mod driver;
mod erased;
pub mod holding;
pub(crate) mod ledger;
pub mod materialized;
pub(crate) mod message;
mod protocol;
mod radices;
pub mod remote;
pub mod stats;
pub(crate) mod tasks;
#[cfg(test)]
mod testing;

/// The tree's maximum branching factor: one child per `u8` radix.
///
/// Also the capacity floor of the assembly fan queues: those channels must
/// admit one *full* fan of children, whatever the budget.
pub(crate) const FAN: usize = u8::MAX as usize + 1;

pub use backend::{Backend, ErasedNode, Leaf, Local, Node, Root};
// The send observation hook, and the edge names its reports carry.
#[cfg(any(test, feature = "test-internals"))]
pub use channel::{EdgeSend, EdgeSendHook, QueueKind, observe_edge_sends};
// The backend conformance suite runs one fixture with an edge capped.
#[cfg(test)]
pub(crate) use channel::with_kind_capacity;
pub use radices::Radices;
// The stream vocabulary the backend conformance suite decorates with;
// crate-visible alongside the suite itself.
#[cfg(any(test, feature = "test-internals"))]
pub(crate) use backend::BoxNodeStream;
#[cfg(test)]
pub use backend::NodeStream;
// The conformance suite recounts a node's grandchildren by exploding its
// children at their runtime height.
#[cfg(test)]
pub(crate) use erased::ops;
#[cfg(test)]
pub use testing::{
    Failing, FailingNode, Failure, Fault, Faulting, GreetingLie, Miscount, Misreport, Operation,
    ReplyCorruption,
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
        Box::pin(self.descent())
    }

    /// The reconciliation [`reconcile`](Self::reconcile) boxes, unboxed, so
    /// that a test can read its size from its type.
    ///
    /// This returns [`descend`]'s future itself: an `async fn` awaiting it
    /// would keep a second copy of the participants for the whole run.
    pub(crate) fn descent(
        self,
    ) -> impl Future<Output = Result<(C::Output, S::Output), Error<C::Error, S::Error>>> {
        let Handshaken {
            client: local,
            server: remote,
            local_key,
            remote_key,
        } = self;
        descend(local, remote, local_key, remote_key)
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
