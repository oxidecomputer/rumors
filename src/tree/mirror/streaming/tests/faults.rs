//! Connected-session abort routing and lifecycle atomicity.

use std::{convert::Infallible, future::Future};

use futures::StreamExt;
use proptest::prelude::*;

use super::{
    fixtures::{LeafOrder, full_depth_comb_pair, one_sided_pair},
    run_to_quiescence, streaming_mirror_sides,
};
use crate::{
    Version,
    message::Message,
    tree::{
        arb::nth_party,
        mirror::{
            Error as MirrorError,
            streaming::{
                Handshaking, Local, Root as StreamingRoot,
                materialized::{Error as MaterializedError, Violation, channel::with_observation},
                message, mirror as drive_streaming,
                protocol::{
                    self, BoxResponses, CompleteConnect, CompleteEqual, CompleteInitiator,
                    CompleteResponder, Connect, Initiator, Reply, Requests, Responder, Responses,
                },
            },
        },
        typed::{self, height},
    },
};

/// An honest protocol state wrapped to fault once after a selected number of
/// outgoing phases, then continue normally.
struct Faulting<P> {
    inner: P,
    remaining: usize,
    violation: Option<Violation>,
}

impl<P> Faulting<P> {
    fn new(inner: P, remaining: usize, violation: Option<Violation>) -> Self {
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
fn malformed_responses<H, R>(
    responses: R,
    violation: Violation,
) -> BoxResponses<Local, (), H, MaterializedError<Infallible>>
where
    H: FaultHeight,
    R: Responses<Local, (), H, MaterializedError<Infallible>>,
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
                reply.replies.insert(0, message::Reaction::Supply(0, H::node()));
            }
            Violation::InvalidSupply => {
                let node = H::node();
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
fn fault_phase<H, R, N>(
    responses: R,
    next: N,
    remaining: usize,
    violation: Option<Violation>,
) -> (
    BoxResponses<Local, (), H, MaterializedError<Infallible>>,
    Faulting<N>,
)
where
    H: FaultHeight,
    R: Responses<Local, (), H, MaterializedError<Infallible>>,
{
    if let (0, Some(violation)) = (remaining, violation) {
        (
            malformed_responses(responses, violation),
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
    P: protocol::Protocol<Error = MaterializedError<Infallible>>,
{
    type Height = P::Height;
    type Error = MaterializedError<Infallible>;
    type Output = P::Output;
}

impl<P> Connect<Local, ()> for Faulting<P>
where
    P: Connect<Local, ()> + protocol::Protocol<Error = MaterializedError<Infallible>>,
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

impl<P> CompleteConnect<Local, ()> for Faulting<P>
where
    P: CompleteConnect<Local, ()> + protocol::Protocol<Error = MaterializedError<Infallible>>,
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

impl<P> protocol::Accept<Local, ()> for Faulting<P>
where
    P: protocol::Accept<Local, ()> + protocol::Protocol<Error = MaterializedError<Infallible>>,
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

impl<P> CompleteEqual<Local, ()> for Faulting<P>
where
    P: CompleteEqual<Local, ()> + protocol::Protocol<Error = MaterializedError<Infallible>>,
{
    async fn complete_equal(self) -> Result<Self::Output, Self::Error> {
        self.inner.complete_equal().await
    }
}

impl<P> Initiator<Local, ()> for Faulting<P>
where
    P: Initiator<Local, ()> + protocol::Protocol<Error = MaterializedError<Infallible>>,
{
    type Next = Faulting<P::Next>;

    fn initiator(
        self,
    ) -> (
        impl Responses<Local, (), height::UnderRoot, Self::Error>,
        Self::Next,
    ) {
        let (responses, next) = self.inner.initiator();
        fault_phase(responses, next, self.remaining, self.violation)
    }
}

impl<P> Responder<Local, ()> for Faulting<P>
where
    P: Responder<Local, ()> + protocol::Protocol<Error = MaterializedError<Infallible>>,
{
    type Next = Faulting<P::Next>;

    fn responder(
        self,
        requests: impl Requests<Local, (), height::UnderRoot>,
    ) -> (
        BoxResponses<Local, (), height::UnderRoot, Self::Error>,
        Self::Next,
    ) {
        let (responses, next) = self.inner.responder(requests);
        fault_phase(responses, next, self.remaining, self.violation)
    }
}

impl<P> Reply<Local, ()> for Faulting<P>
where
    P: Reply<Local, ()> + protocol::Protocol<Error = MaterializedError<Infallible>>,
    <P::Height as protocol::ReplyHeight>::Output: FaultHeight,
{
    type Next = Faulting<P::Next>;

    fn reply(
        self,
        requests: impl Requests<Local, (), Self::Height>,
    ) -> (
        BoxResponses<Local, (), <Self::Height as protocol::ReplyHeight>::Output, Self::Error>,
        Self::Next,
    ) {
        let (responses, next) = self.inner.reply(requests);
        fault_phase(responses, next, self.remaining, self.violation)
    }
}

impl<P> CompleteResponder<Local, ()> for Faulting<P>
where
    P: CompleteResponder<Local, ()> + protocol::Protocol<Error = MaterializedError<Infallible>>,
{
    fn complete_responder(
        self,
        requests: impl Requests<Local, (), height::Z>,
    ) -> (
        BoxResponses<Local, (), height::Z, Self::Error>,
        impl Future<Output = Result<Self::Output, Self::Error>> + Send,
    ) {
        let (responses, output) = self.inner.complete_responder(requests);
        let responses = if let (0, Some(violation)) = (self.remaining, self.violation) {
            malformed_responses(responses, violation)
        } else {
            Box::pin(responses)
        };
        (responses, output)
    }
}

impl<P> CompleteInitiator<Local, ()> for Faulting<P>
where
    P: CompleteInitiator<Local, ()> + protocol::Protocol<Error = MaterializedError<Infallible>>,
{
    async fn complete_initiator(
        self,
        requests: impl Requests<Local, (), height::Z>,
    ) -> Result<Self::Output, Self::Error> {
        self.inner.complete_initiator(requests).await
    }
}

proptest! {
    /// A genuine malformed reply crosses the fully connected driver as its
    /// detected violation while both materialized input roots remain untouched.
    #[test]
    fn connected_violation_aborts_without_mutating_root(
        server_steps in 0usize..=15,
        client_steps in 0usize..=15,
    ) {
        let violation = Violation::UnexpectedQuery;
        let (client_root, server_root) =
            full_depth_comb_pair(2, LeafOrder::Interleaved);
        let before = (client_root.clone(), server_root.clone());
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("the test runtime should build");

        let local = Handshaking::start(Local, StreamingRoot::from(client_root.clone()));
        let honest_server = Handshaking::start(Local, StreamingRoot::from(server_root.clone()));
        let faulting_server = Faulting::new(honest_server, server_steps, Some(violation));
        let result = run_to_quiescence(&runtime, drive_streaming(local, faulting_server))
            .expect("the connected driver must surface the fault, not stall");
        match result {
            Err(MirrorError::Client(MaterializedError::Violation(actual))) => {
                prop_assert_eq!(actual, violation);
            }
            Err(other) => prop_assert!(false, "unexpected driver error: {other:?}"),
            Ok(_) => prop_assert!(false, "the faulting counterparty unexpectedly completed"),
        }

        // Reversing the handshake sides also reverses initiator order: the
        // driver's frame-relative error is flipped back to the original client.
        let honest_client = Handshaking::start(Local, StreamingRoot::from(client_root.clone()));
        let faulting_client = Faulting::new(honest_client, client_steps, Some(violation));
        let local = Handshaking::start(Local, StreamingRoot::from(server_root.clone()));
        let result = run_to_quiescence(&runtime, drive_streaming(faulting_client, local))
            .expect("the reversed connected driver must surface the fault, not stall");
        match result {
            Err(MirrorError::Server(MaterializedError::Violation(actual))) => {
                prop_assert_eq!(actual, violation);
            }
            Err(other) => prop_assert!(false, "unexpected reversed driver error: {other:?}"),
            Ok(_) => prop_assert!(false, "the reversed faulting counterparty unexpectedly completed"),
        }

        prop_assert_eq!((client_root, server_root), before);
    }
}

/// Equal versions return both connected states' outputs without opening the
/// descent.
#[test]
fn equal_versions_return_outputs_without_descent() {
    let (_, root) = one_sided_pair(&[(0x20, 2, 1)]);
    let ((ours, theirs), report) =
        with_observation(|| streaming_mirror_sides(root.clone(), root.clone()));

    assert_eq!(ours, root);
    assert_eq!(theirs, root);
    assert_eq!(
        report.roles().count(),
        0,
        "the equal-version path must not construct descent queues",
    );
}
