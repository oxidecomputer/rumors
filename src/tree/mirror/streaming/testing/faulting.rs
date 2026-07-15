//! A composable protocol decorator which injects one semantic violation.

use std::future::Future;

use crate::{
    Version,
    message::Message,
    tree::{
        arb::nth_party,
        mirror::streaming::{
            Backend, Leaf, Local,
            materialized::{Error as MaterializedError, Violation},
            message,
            protocol::{
                self, BoxResponses, CompleteConnect, CompleteEqual, CompleteInitiator,
                CompleteResponder, Connect, Initiator, Reply, Requests, Responder, Responses,
            },
        },
        typed::{
            self,
            height::{self, Z},
        },
    },
};
use futures::StreamExt;

use super::failing::{Failing, FailingNode};

/// An honest protocol state wrapped to fault once after a selected number of
/// outgoing phases, then continue normally.
pub struct Faulting<P> {
    inner: P,
    remaining: usize,
    violation: Option<Violation>,
}

impl<P> Faulting<P> {
    pub fn new(inner: P, remaining: usize, violation: Option<Violation>) -> Self {
        Self {
            inner,
            remaining,
            violation,
        }
    }
}

/// Construct a valid supplied node at any reply height.
trait FaultHeight: height::Height + Sized {
    fn node() -> typed::Node<(), Self>;
}

/// A backend whose test node handles can wrap the canonical local fixture.
trait FaultBackend: Backend<(), Node<Z>: Leaf<()>> {
    fn node<H: FaultHeight>() -> Self::Node<H>;
}

impl FaultBackend for Local {
    fn node<H: FaultHeight>() -> Self::Node<H> {
        H::node()
    }
}

impl<B: FaultBackend> FaultBackend for Failing<B> {
    fn node<H: FaultHeight>() -> Self::Node<H> {
        FailingNode::new(B::node::<H>())
    }
}

impl FaultHeight for height::Z {
    fn node() -> typed::Node<(), Self> {
        let mut version = Version::new();
        version.tick(&nth_party(0));
        typed::Node::leaf(version, Message::new(()))
    }
}

impl<H: FaultHeight> FaultHeight for height::S<H>
where
    height::S<H>: height::Height,
{
    fn node() -> typed::Node<(), Self> {
        typed::Node::beneath(H::node(), 0)
    }
}

/// Turn an honest reply stream into one which commits the selected semantic
/// fault for its counterparty to detect.
fn malformed_responses<B, H, R>(
    responses: R,
    violation: Violation,
) -> BoxResponses<B, (), H, MaterializedError<B::Error>>
where
    B: FaultBackend,
    H: FaultHeight,
    R: Responses<B, (), H, MaterializedError<B::Error>>,
{
    Box::pin(async_stream::stream! {
        let mut responses = Box::pin(responses);

        if violation == Violation::UnansweredQuery {
            return;
        }

        if violation == Violation::UnaskedReply {
            if let Some(item) = responses.next().await {
                yield item;
            }
            yield Ok(message::Reply { replies: Vec::new() });
            return;
        }

        let Some(item) = responses.next().await else {
            return;
        };
        let Ok(mut reply) = item else {
            yield item;
            return;
        };

        match violation {
            Violation::UnfinishedReply => reply.replies.clear(),
            Violation::UnexpectedMatch => reply.replies.push(message::Reaction::Match),
            Violation::UnexpectedQuery => {
                reply.replies.push(message::Reaction::Query(Vec::new()));
            }
            Violation::UnexpectedSupply => {
                reply.replies.insert(0, message::Reaction::Supply(0, B::node::<H>()));
            }
            Violation::InvalidSupply => {
                let node = B::node::<H>();
                reply.replies.push(message::Reaction::Supply(0, node.clone()));
                reply.replies.push(message::Reaction::Supply(0, node));
            }
            Violation::UnaskedReply | Violation::UnansweredQuery => unreachable!(),
        }
        yield Ok(reply);
    })
}

/// Pass one honest outgoing phase through, or corrupt it with the selected
/// fault once its countdown reaches zero.
fn fault_phase<B, H, R, N>(
    responses: R,
    next: N,
    remaining: usize,
    violation: Option<Violation>,
) -> (
    BoxResponses<B, (), H, MaterializedError<B::Error>>,
    Faulting<N>,
)
where
    B: FaultBackend,
    H: FaultHeight,
    R: Responses<B, (), H, MaterializedError<B::Error>>,
{
    if let (0, Some(violation)) = (remaining, violation) {
        (
            malformed_responses::<B, _, _>(responses, violation),
            Faulting::new(next, 0, None),
        )
    } else {
        (
            Box::pin(responses),
            Faulting::new(next, remaining.saturating_sub(1), violation),
        )
    }
}

impl<P> protocol::Protocol for Faulting<P>
where
    P: protocol::Protocol,
{
    type Height = P::Height;
    type Error = P::Error;
    type Output = P::Output;
}

impl<B, P> Connect<B, ()> for Faulting<P>
where
    B: FaultBackend,
    P: Connect<B, ()> + protocol::Protocol<Error = MaterializedError<B::Error>>,
{
    type Next = Faulting<P::Next>;

    async fn connect(self) -> Result<(message::Handshake, Self::Next), Self::Error> {
        let Faulting {
            inner,
            remaining,
            violation,
        } = self;
        let (handshake, next) = inner.connect().await?;
        Ok((handshake, Faulting::new(next, remaining, violation)))
    }
}

impl<B, P> CompleteConnect<B, ()> for Faulting<P>
where
    B: FaultBackend,
    P: CompleteConnect<B, ()> + protocol::Protocol<Error = MaterializedError<B::Error>>,
{
    type Next = Faulting<P::Next>;

    async fn complete_connect(self, their_version: Version) -> Result<Self::Next, Self::Error> {
        let Faulting {
            inner,
            remaining,
            violation,
        } = self;
        let next = inner.complete_connect(their_version).await?;
        Ok(Faulting::new(next, remaining, violation))
    }
}

impl<B, P> protocol::Accept<B, ()> for Faulting<P>
where
    B: FaultBackend,
    P: protocol::Accept<B, ()> + protocol::Protocol<Error = MaterializedError<B::Error>>,
{
    type Next = Faulting<P::Next>;

    async fn accept(
        self,
        request: message::Handshake,
    ) -> Result<(message::Handshake, Self::Next), Self::Error> {
        let Faulting {
            inner,
            remaining,
            violation,
        } = self;
        let (handshake, next) = inner.accept(request).await?;
        Ok((handshake, Faulting::new(next, remaining, violation)))
    }
}

impl<B, P> CompleteEqual<B, ()> for Faulting<P>
where
    B: FaultBackend,
    P: CompleteEqual<B, ()> + protocol::Protocol<Error = MaterializedError<B::Error>>,
{
    async fn complete_equal(self) -> Result<Self::Output, Self::Error> {
        self.inner.complete_equal().await
    }
}

impl<B, P> Initiator<B, ()> for Faulting<P>
where
    B: FaultBackend,
    P: Initiator<B, ()> + protocol::Protocol<Error = MaterializedError<B::Error>>,
{
    type Next = Faulting<P::Next>;

    fn initiator(
        self,
    ) -> (
        impl Responses<B, (), height::UnderRoot, Self::Error>,
        Self::Next,
    ) {
        let (responses, next) = self.inner.initiator();
        fault_phase(responses, next, self.remaining, self.violation)
    }
}

impl<B, P> Responder<B, ()> for Faulting<P>
where
    B: FaultBackend,
    P: Responder<B, ()> + protocol::Protocol<Error = MaterializedError<B::Error>>,
{
    type Next = Faulting<P::Next>;

    fn responder(
        self,
        requests: impl Requests<B, (), height::UnderRoot>,
    ) -> (
        BoxResponses<B, (), height::UnderRoot, Self::Error>,
        Self::Next,
    ) {
        let (responses, next) = self.inner.responder(requests);
        fault_phase(responses, next, self.remaining, self.violation)
    }
}

impl<B, P> Reply<B, ()> for Faulting<P>
where
    B: FaultBackend,
    P: Reply<B, ()> + protocol::Protocol<Error = MaterializedError<B::Error>>,
    <P::Height as protocol::ReplyHeight>::Output: FaultHeight,
{
    type Next = Faulting<P::Next>;

    fn reply(
        self,
        requests: impl Requests<B, (), Self::Height>,
    ) -> (
        BoxResponses<B, (), <Self::Height as protocol::ReplyHeight>::Output, Self::Error>,
        Self::Next,
    ) {
        let (responses, next) = self.inner.reply(requests);
        fault_phase(responses, next, self.remaining, self.violation)
    }
}

impl<B, P> CompleteResponder<B, ()> for Faulting<P>
where
    B: FaultBackend,
    P: CompleteResponder<B, ()> + protocol::Protocol<Error = MaterializedError<B::Error>>,
{
    fn complete_responder(
        self,
        requests: impl Requests<B, (), height::Z>,
    ) -> (
        BoxResponses<B, (), height::Z, Self::Error>,
        impl Future<Output = Result<Self::Output, Self::Error>> + Send,
    ) {
        let (responses, output) = self.inner.complete_responder(requests);
        let responses = if let (0, Some(violation)) = (self.remaining, self.violation) {
            malformed_responses::<B, _, _>(responses, violation)
        } else {
            Box::pin(responses)
        };
        (responses, output)
    }
}

impl<B, P> CompleteInitiator<B, ()> for Faulting<P>
where
    B: FaultBackend,
    P: CompleteInitiator<B, ()> + protocol::Protocol<Error = MaterializedError<B::Error>>,
{
    async fn complete_initiator(
        self,
        requests: impl Requests<B, (), height::Z>,
    ) -> Result<Self::Output, Self::Error> {
        self.inner.complete_initiator(requests).await
    }
}
