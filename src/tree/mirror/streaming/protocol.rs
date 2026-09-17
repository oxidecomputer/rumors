//! The streaming protocol's traits, generic over the session's backend.
//!
//! Both parties of a session speak one backend `I`: the messages a stage
//! consumes and the messages it produces are keyed by the same node types.
//! A party that owns no tree — the [`remote`](crate::tree::mirror::streaming::remote) proxy — is
//! parameterized by its local counterparty's backend rather than defining one
//! of its own, which is what lets the node types meet in the middle without a
//! conversion anywhere in the schedule.

use std::pin::Pin;

use futures::Stream;

use crate::tree::{
    mirror::streaming::{Backend, Leaf, message},
    typed::height::{Height, Root, S, UnderRoot, UnderUnderRoot, Z},
};

/// Compile-time bounds for each participant's complete phase chain.
mod peer;
pub use peer::{Client, Peer, Server};

/// Common metadata for a state in the height-typed session schedule.
pub trait Phase: Send {
    /// The tree height processed by this phase.
    type Height: Height;
    /// The failure type carried by this phase and its outgoing streams.
    type Error: Send + 'static;
    /// The value produced when this participant completes the session.
    type Output: Send;
}

/// Trait synonym: non-erroring message streams, the shape of incoming streams.
pub trait Requests<B: Backend<Node<Z>: Leaf>, H: Height>:
    Stream<Item = message::Reply<B, H>> + Send + 'static
{
}
/// Any stream with the required item and lifetime bounds is a request stream.
impl<X, B: Backend<Node<Z>: Leaf>, H: Height> Requests<B, H> for X where
    X: Stream<Item = message::Reply<B, H>> + Send + 'static
{
}

/// Trait synonym: fallible message streams, the shape of outgoing streams.
pub trait Responses<B: Backend<Node<Z>: Leaf>, H: Height, E>:
    Stream<Item = Result<message::Reply<B, H>, E>> + Send + 'static
{
}
/// Any stream with the required item, error, and lifetime bounds is a response stream.
impl<X, B: Backend<Node<Z>: Leaf>, H: Height, E> Responses<B, H, E> for X where
    X: Stream<Item = Result<message::Reply<B, H>, E>> + Send + 'static
{
}

/// A response stream boxed to keep the recursively nested phase type bounded.
///
/// Without erasure here, every phase nests the next phase's stream type, so
/// the compiler's concrete type grows exponentially with tree height.
pub type BoxResponses<B, H, E> = Pin<Box<dyn Responses<B, H, E>>>;

/// The client's root phase: produce its greeting and await the peer's.
pub trait Connect<B: Backend<Node<Z>: Leaf>>: Phase<Height = Root> + Sized {
    /// The state which accepts the peer's greeting.
    type Next: CompleteConnect<B> + Phase<Height = Root, Output = Self::Output, Error = Self::Error>;

    /// Produce the client's greeting and advance to the receiving state.
    fn connect(
        self,
    ) -> impl Future<Output = Result<(message::Greeting, Self::Next), Self::Error>> + Send;
}

/// The server's root phase: accept the client's greeting and return its own.
pub trait Accept<B: Backend<Node<Z>: Leaf>>: Phase<Height = Root> + Sized {
    /// The state ready to resolve equality or begin the elected role.
    type Next: CompleteEqual<B>
        + Initiator<B>
        + Responder<B>
        + Phase<Height = Root, Output = Self::Output, Error = Self::Error>;

    /// Consume the client's greeting, produce the server's, and advance.
    fn accept(
        self,
        request: message::Greeting,
    ) -> impl Future<Output = Result<(message::Greeting, Self::Next), Self::Error>> + Send;
}

/// The client phase which consumes the server's greeting.
pub trait CompleteConnect<B: Backend<Node<Z>: Leaf>>: Phase<Height = Root> + Sized {
    /// The state ready to resolve equality or begin the elected role.
    type Next: CompleteEqual<B>
        + Initiator<B>
        + Responder<B>
        + Phase<Height = Root, Output = Self::Output, Error = Self::Error>;

    /// Consume the server's greeting and advance to role election.
    fn complete_connect(
        self,
        theirs: message::Greeting,
    ) -> impl Future<Output = Result<Self::Next, Self::Error>> + Send;
}

/// Resolve a connected session directly when the greeting versions match.
///
/// Equal versions prove that no descent is needed, but each connected state
/// must still be converted into its normal output. A materialized state returns
/// the root it already holds; a remote proxy returns its transport halves so
/// the caller can continue with trailing session frames.
pub trait CompleteEqual<B: Backend<Node<Z>: Leaf>>: Phase<Height = Root> + Sized {
    /// Produce this participant's result without opening the descent.
    fn complete_equal(self) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send;
}

/// The opening burst: the initiator speaks first, and unprompted.
///
/// Nothing precedes this stage. A root hash would be the natural thing to
/// send, and it is exactly what the session never needs: two roots hash
/// equal only when their versions are equal, and equal versions
/// short-circuit the session before it reaches the protocol at all. So the
/// initiator skips straight to its root's children — the same root-fan
/// listing its [`Greeting`](message::Greeting) already carried. On the
/// wire that makes this stage free: the remote proxy replays the greeting's
/// listing instead of spending a hop on a standalone opening frame, and only
/// the in-process message below actually flows.
pub trait Initiator<B: Backend<Node<Z>: Leaf>>: Phase<Height = Root> + Sized {
    /// The state awaiting replies one level below the root.
    type Next: Phase<Height = UnderRoot, Output = Self::Output, Error = Self::Error>;

    /// Produce the opening root-child query and advance into the descent.
    fn initiator(self) -> (BoxResponses<B, UnderRoot, Self::Error>, Self::Next);
}

/// The responder's opening phase: answer the initiator's root-child query.
pub trait Responder<B: Backend<Node<Z>: Leaf>>: Phase<Height = Root> + Sized {
    // Like [`Initiator::Next`], this is left un-bounded by [`Reply`]: both
    // openings hand off to the descent, but only [`Peer`] spells the chain out.
    // Naming `Reply` here instead would make it a bound `Accept::Next` and
    // `CompleteConnect::Next` must discharge, which no generic wrapper can do
    // without the concrete height.
    /// The state awaiting replies two levels below the root.
    type Next: Phase<Height = UnderUnderRoot, Output = Self::Output, Error = Self::Error>;

    /// Consume the opening query, produce its replies, and enter the descent.
    fn responder(
        self,
        requests: impl Requests<B, UnderRoot>,
    ) -> (BoxResponses<B, UnderRoot, Self::Error>, Self::Next);
}

/// The height transition of a reply phase.
pub trait ReplyHeight: Height {
    /// The height of replies produced in this phase.
    type Output: Height;
    /// The height processed by the next phase.
    type Next: Height;
}

/// A height-one phase produces and advances to leaf-height replies.
impl ReplyHeight for S<Z> {
    type Output = Z;
    type Next = Z;
}

/// Higher phases answer one level down and advance two levels down.
impl<H> ReplyHeight for S<S<H>>
where
    H: Height,
    S<H>: Height,
    S<S<H>>: Height,
{
    type Output = S<H>;
    type Next = H;
}

/// One descent phase: consume replies at this height and answer one level down.
pub trait Reply<B: Backend<Node<Z>: Leaf>>: Phase<Height: ReplyHeight> + Sized {
    /// The state which processes the next lower round.
    type Next: Phase<
            Height = <Self::Height as ReplyHeight>::Next,
            Output = Self::Output,
            Error = Self::Error,
        >;

    /// Consume this round's requests, produce its replies, and advance.
    fn reply(
        self,
        requests: impl Requests<B, Self::Height>,
    ) -> (
        BoxResponses<B, <Self::Height as ReplyHeight>::Output, Self::Error>,
        Self::Next,
    );
}

/// The responder's terminal: absorb the initiator's leaf replies and answer
/// each requested leaf with one leaf-height [`Reply`].
///
/// Each requested leaf is answered pruned against the initiator's version,
/// so a leaf the initiator deleted drops here instead of shipping.
pub trait CompleteResponder<B: Backend<Node<Z>: Leaf>>: Phase<Height = Z> + Sized {
    /// Answer the initiator's final leaf requests and complete in parallel.
    fn complete_responder(
        self,
        requests: impl Requests<B, Z>,
    ) -> (
        BoxResponses<B, Z, Self::Error>,
        impl Future<Output = Result<Self::Output, Self::Error>> + Send,
    );
}

/// The initiator's terminal: absorb the responder's final leaf replies and
/// resolve to the reconciled root.
pub trait CompleteInitiator<B: Backend<Node<Z>: Leaf>>: Phase<Height = Z> + Sized {
    /// Consume the responder's final leaf replies and produce the result.
    fn complete_initiator(
        self,
        requests: impl Requests<B, Z>,
    ) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send;
}
