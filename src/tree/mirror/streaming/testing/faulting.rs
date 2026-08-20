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

/// One injected counterparty fault: a reply-stream corruption scheduled by
/// the phase countdown, or a greeting lie told at the handshake.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    /// Corrupt one outgoing reply phase to commit the selected violation.
    Reply(Violation),
    /// Tell one lie in the outgoing greeting.
    ///
    /// The inner state keeps behaving from its true tree, so the
    /// declaration and the behavior disagree — the shape a
    /// greeting-premise guard exists to catch. Fires at the handshake;
    /// the phase countdown does not apply.
    Greeting(GreetingLie),
}

/// One dishonest field in an otherwise honest greeting.
///
/// The shrunken directions are detectable: honest behavior overruns the
/// declaration, and the deceived counterparty must fail the session with
/// the named violation. The inflated directions are tolerated: a
/// declaration is an upper premise, so the session must complete cleanly
/// — the no-false-positive half of each guard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GreetingLie {
    /// Declare an empty set: the first absorbed honest supply overruns
    /// the declared length ([`Violation::OverdrawnSupply`]).
    ShrunkenSetLen,
    /// Declare a single leaf while holding more: honest supply overruns
    /// the nonzero allowance mid-session ([`Violation::OverdrawnSupply`]).
    ///
    /// The walk-side face of the lie the wire decoder catches within one
    /// still-open reply: here the allowance admits supply before the
    /// ledger's accumulation trips, unlike the zero declaration's
    /// first-charge rejection.
    UnderdeclaredSetLen,
    /// Declare more leaves than the tree holds; the session must
    /// complete cleanly.
    InflatedSetLen,
    /// Declare the empty version: every honest supply escapes it
    /// ([`Violation::UncontainedSupply`]).
    ShrunkenVersion,
    /// Declare a version above the tree's truth (a tick on a party no
    /// fixture uses); the deceived side absorbs it into its ceiling by
    /// definition (`ours | declared`), and the reconciled content must
    /// not move.
    InflatedVersion,
}

/// Apply one lie to an outgoing greeting.
fn tell(greeting: &mut message::Greeting, lie: GreetingLie) {
    match lie {
        GreetingLie::ShrunkenSetLen => greeting.set_len = 0,
        GreetingLie::UnderdeclaredSetLen => greeting.set_len = 1,
        GreetingLie::InflatedSetLen => greeting.set_len = greeting.set_len * 2 + 1,
        GreetingLie::ShrunkenVersion => greeting.version = Version::new(),
        GreetingLie::InflatedVersion => {
            greeting.version = greeting.version.clone() | &escaped_version();
        }
    }
}

/// An honest protocol state wrapped to fault once — a reply corruption after
/// a selected number of outgoing phases, or a greeting lie at the handshake —
/// then continue normally.
pub struct Faulting<P> {
    inner: P,
    remaining: usize,
    fault: Option<Fault>,
}

impl<P> Faulting<P> {
    pub fn new(inner: P, remaining: usize, fault: Option<Fault>) -> Self {
        Self {
            inner,
            remaining,
            fault,
        }
    }
}

/// One tick of the canonical test party: contained in any fixture ceiling
/// whose tree ticked that party at least once.
fn contained_version() -> Version {
    let mut version = Version::new();
    version.tick(&nth_party(0));
    version
}

/// A version outside any fixture's declared ceiling: one tick on a party
/// index no fixture ticks, so disjointness alone defeats containment.
fn escaped_version() -> Version {
    let mut version = Version::new();
    version.tick(&nth_party(31));
    version
}

/// Construct a valid supplied node at any reply height.
trait FaultHeight: height::Height + Sized {
    /// A node whose single leaf carries `version`.
    fn node_at(version: Version) -> typed::Node<Self>;

    /// A node at the canonical contained version.
    fn node() -> typed::Node<Self> {
        Self::node_at(contained_version())
    }
}

/// A backend whose test node handles can wrap the canonical local fixture.
trait FaultBackend: Backend<Node<Z>: Leaf> {
    fn node<H: FaultHeight>() -> Self::Node<H>;

    /// A node whose version no fixture's declared ceiling contains.
    fn escaped<H: FaultHeight>() -> Self::Node<H>;
}

impl FaultBackend for Local {
    fn node<H: FaultHeight>() -> Self::Node<H> {
        H::node()
    }

    fn escaped<H: FaultHeight>() -> Self::Node<H> {
        H::node_at(escaped_version())
    }
}

impl<B: FaultBackend> FaultBackend for Failing<B> {
    fn node<H: FaultHeight>() -> Self::Node<H> {
        FailingNode::new(B::node::<H>())
    }

    fn escaped<H: FaultHeight>() -> Self::Node<H> {
        FailingNode::new(B::escaped::<H>())
    }
}

impl FaultHeight for height::Z {
    fn node_at(version: Version) -> typed::Node<Self> {
        typed::Node::leaf(version, Message::new(()))
    }
}

impl<H: FaultHeight> FaultHeight for height::S<H>
where
    height::S<H>: height::Height,
{
    fn node_at(version: Version) -> typed::Node<Self> {
        typed::Node::beneath(H::node_at(version), 0)
    }
}

/// Turn an honest reply stream into one which commits the selected semantic
/// fault for its counterparty to detect.
fn malformed_responses<B, H, R>(
    responses: R,
    violation: Violation,
) -> BoxResponses<B, H, MaterializedError<B::Error>>
where
    B: FaultBackend,
    H: FaultHeight,
    R: Responses<B, H, MaterializedError<B::Error>>,
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
            Violation::UncontainedSupply => {
                // Appended past the honest reply, which covers the whole
                // held fan, so the supply is structurally valid and only
                // its escaped version is at fault. Radix 0xff assumes no
                // fixture holds that child, which every current fixture
                // satisfies.
                reply
                    .replies
                    .push(message::Reaction::Supply(0xff, B::escaped::<H>()));
            }
            Violation::OverdrawnSupply => {
                unreachable!("a set-length overrun is a greeting lie (Fault::Greeting), never a reply corruption")
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
    fault: Option<Fault>,
) -> (BoxResponses<B, H, MaterializedError<B::Error>>, Faulting<N>)
where
    B: FaultBackend,
    H: FaultHeight,
    R: Responses<B, H, MaterializedError<B::Error>>,
{
    if let (0, Some(Fault::Reply(violation))) = (remaining, fault) {
        (
            malformed_responses::<B, _, _>(responses, violation),
            Faulting::new(next, 0, None),
        )
    } else {
        (
            Box::pin(responses),
            Faulting::new(next, remaining.saturating_sub(1), fault),
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

impl<B, P> Connect<B> for Faulting<P>
where
    B: FaultBackend,
    P: Connect<B> + protocol::Protocol<Error = MaterializedError<B::Error>>,
{
    type Next = Faulting<P::Next>;

    async fn connect(self) -> Result<(message::Greeting, Self::Next), Self::Error> {
        let Faulting {
            inner,
            remaining,
            fault,
        } = self;
        let (mut handshake, next) = inner.connect().await?;
        if let Some(Fault::Greeting(lie)) = fault {
            tell(&mut handshake, lie);
        }
        Ok((handshake, Faulting::new(next, remaining, fault)))
    }
}

impl<B, P> CompleteConnect<B> for Faulting<P>
where
    B: FaultBackend,
    P: CompleteConnect<B> + protocol::Protocol<Error = MaterializedError<B::Error>>,
{
    type Next = Faulting<P::Next>;

    async fn complete_connect(self, theirs: message::Greeting) -> Result<Self::Next, Self::Error> {
        let Faulting {
            inner,
            remaining,
            fault,
        } = self;
        let next = inner.complete_connect(theirs).await?;
        Ok(Faulting::new(next, remaining, fault))
    }
}

impl<B, P> protocol::Accept<B> for Faulting<P>
where
    B: FaultBackend,
    P: protocol::Accept<B> + protocol::Protocol<Error = MaterializedError<B::Error>>,
{
    type Next = Faulting<P::Next>;

    async fn accept(
        self,
        request: message::Greeting,
    ) -> Result<(message::Greeting, Self::Next), Self::Error> {
        let Faulting {
            inner,
            remaining,
            fault,
        } = self;
        let (mut handshake, next) = inner.accept(request).await?;
        if let Some(Fault::Greeting(lie)) = fault {
            tell(&mut handshake, lie);
        }
        Ok((handshake, Faulting::new(next, remaining, fault)))
    }
}

impl<B, P> CompleteEqual<B> for Faulting<P>
where
    B: FaultBackend,
    P: CompleteEqual<B> + protocol::Protocol<Error = MaterializedError<B::Error>>,
{
    async fn complete_equal(self) -> Result<Self::Output, Self::Error> {
        self.inner.complete_equal().await
    }
}

impl<B, P> Initiator<B> for Faulting<P>
where
    B: FaultBackend,
    P: Initiator<B> + protocol::Protocol<Error = MaterializedError<B::Error>>,
{
    type Next = Faulting<P::Next>;

    fn initiator(self) -> (BoxResponses<B, height::UnderRoot, Self::Error>, Self::Next) {
        let (responses, next) = self.inner.initiator();
        fault_phase(responses, next, self.remaining, self.fault)
    }
}

impl<B, P> Responder<B> for Faulting<P>
where
    B: FaultBackend,
    P: Responder<B> + protocol::Protocol<Error = MaterializedError<B::Error>>,
{
    type Next = Faulting<P::Next>;

    fn responder(
        self,
        requests: impl Requests<B, height::UnderRoot>,
    ) -> (BoxResponses<B, height::UnderRoot, Self::Error>, Self::Next) {
        let (responses, next) = self.inner.responder(requests);
        fault_phase(responses, next, self.remaining, self.fault)
    }
}

impl<B, P> Reply<B> for Faulting<P>
where
    B: FaultBackend,
    P: Reply<B> + protocol::Protocol<Error = MaterializedError<B::Error>>,
    <P::Height as protocol::ReplyHeight>::Output: FaultHeight,
{
    type Next = Faulting<P::Next>;

    fn reply(
        self,
        requests: impl Requests<B, Self::Height>,
    ) -> (
        BoxResponses<B, <Self::Height as protocol::ReplyHeight>::Output, Self::Error>,
        Self::Next,
    ) {
        let (responses, next) = self.inner.reply(requests);
        fault_phase(responses, next, self.remaining, self.fault)
    }
}

impl<B, P> CompleteResponder<B> for Faulting<P>
where
    B: FaultBackend,
    P: CompleteResponder<B> + protocol::Protocol<Error = MaterializedError<B::Error>>,
{
    fn complete_responder(
        self,
        requests: impl Requests<B, height::Z>,
    ) -> (
        BoxResponses<B, height::Z, Self::Error>,
        impl Future<Output = Result<Self::Output, Self::Error>> + Send,
    ) {
        let (responses, output) = self.inner.complete_responder(requests);
        let responses = if let (0, Some(Fault::Reply(violation))) = (self.remaining, self.fault) {
            malformed_responses::<B, _, _>(responses, violation)
        } else {
            Box::pin(responses)
        };
        (responses, output)
    }
}

impl<B, P> CompleteInitiator<B> for Faulting<P>
where
    B: FaultBackend,
    P: CompleteInitiator<B> + protocol::Protocol<Error = MaterializedError<B::Error>>,
{
    async fn complete_initiator(
        self,
        requests: impl Requests<B, height::Z>,
    ) -> Result<Self::Output, Self::Error> {
        self.inner.complete_initiator(requests).await
    }
}
