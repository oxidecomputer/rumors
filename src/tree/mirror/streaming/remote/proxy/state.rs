//! Typed protocol states over an open per-stream session.
//!
//! Each proxy stage owns the one [`Scope`] queue needed to interpret the local
//! reply it will receive at that height. Its outgoing response stream pumps
//! those local replies to the wire while decoding the remote replies which
//! answer locally-created questions. The active [`Work`] response pump lets a
//! stage yield its reply before publishing the lower scopes derived from it,
//! so one-slot backpressure cannot withhold the reply which releases it.

use std::marker::PhantomData;

use crate::link::{Acceptor, Connector};
use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        channel::Receiver,
        protocol::{self, BoxResponses, Requests},
        remote::{
            adapter::Scope,
            codec::{RunBudget, Speaker, Stream},
            proxy::{Error, work::Work},
            streams::{Claims, ErrorRoute, StreamReceiver, StreamSender},
        },
        stats::Recorder,
    },
    typed::height::{Height, Root, S, UnderRoot, UnderUnderRoot, Z},
};

/// Session endpoints and backend shared by every state in one proxy chain.
struct Session<B, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
    A: Acceptor,
{
    remote: Speaker,
    epoch: u8,
    connector: C,
    claims: Claims<A::Rx>,
    route: ErrorRoute,
    /// The session's negotiated run budget, handed to every incoming
    /// stream this session binds so its codec enforces the budget at
    /// ingress (the outgoing side's copy lives in [`Work`]).
    budget: RunBudget,
    /// The session's stats recorder, handed to every stream this session
    /// binds so the codec seam's byte counts accumulate in one place.
    stats: Recorder,
    work: Work<B, R, W, A>,
}

impl<B, R, W, C, A> Session<B, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
    C: Connector,
    A: Acceptor,
{
    /// Bind the incoming logical stream spoken by the remote at `height`.
    fn incoming<H: Height>(&mut self) -> StreamReceiver<A::Rx> {
        let stream = stream_at::<H>(self.remote);
        StreamReceiver::new(
            self.claims.take(stream),
            self.remote,
            stream,
            self.budget,
            self.route.clone(),
            self.stats.clone(),
        )
    }

    /// Bind the outgoing logical stream spoken locally at `height`.
    fn outgoing<H: Height>(&mut self) -> StreamSender<C> {
        let local = self.remote.other();
        StreamSender::new(
            self.connector.clone(),
            self.epoch,
            local,
            stream_at::<H>(local),
            self.stats.clone(),
        )
    }
}

/// Find the logical stream assigned to one speaker and reply height.
fn stream_at<H: Height>(speaker: Speaker) -> Stream {
    Stream::at_height(speaker, H::HEIGHT).expect("every protocol reply height has one stream")
}

/// A proxy after the version exchange but before its elected role is known.
pub struct Connected<B, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
    A: Acceptor,
{
    state: ConnectedState<B, R, W, C, A>,
}

/// Equal versions need no data streams; divergent versions own a session.
enum ConnectedState<B, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
    A: Acceptor,
{
    Equal(R, W),
    Diverged(Box<Session<B, R, W, C, A>>),
}

impl<B, R, W, C, A> Connected<B, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
    C: Connector,
    A: Acceptor,
{
    /// Bind an elected session's parts to the remote elected speaker.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        remote: Speaker,
        epoch: u8,
        connector: C,
        claims: Claims<A::Rx>,
        route: ErrorRoute,
        budget: RunBudget,
        stats: Recorder,
        work: Work<B, R, W, A>,
    ) -> Self {
        Self {
            state: ConnectedState::Diverged(Box::new(Session {
                remote,
                epoch,
                connector,
                claims,
                route,
                budget,
                stats,
                work,
            })),
        }
    }

    /// Retain untouched control halves when the versions already agree.
    pub fn equal(read: R, write: W) -> Self {
        Self {
            state: ConnectedState::Equal(read, write),
        }
    }

    /// Extract the session guaranteed by the driver's divergent-version path.
    fn diverged(self) -> Session<B, R, W, C, A> {
        match self.state {
            ConnectedState::Diverged(session) => *session,
            ConnectedState::Equal(..) => unreachable!("descent opened for equal versions"),
        }
    }
}

impl<B, R, W, C, A> protocol::Protocol for Connected<B, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
    R: Send,
    W: Send,
    C: Send,
    A: Acceptor,
{
    type Height = Root;
    type Error = Error<B::Error>;
    type Output = (R, W);
}

/// A proxy inside the descent with scopes for the next local reply stream.
pub struct Descending<B, H, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
    H: Height,
    S<H>: Height,
    A: Acceptor,
{
    session: Session<B, R, W, C, A>,
    /// The next local reply's scopes, erased: the typestate's `H` is what
    /// pins this queue to the stage that consumes it at the right height,
    /// and every scope's parent prefix carries the runtime witness.
    scopes: Receiver<Scope>,
    /// The remote initiator's opening-supply stream (`None` below the
    /// stage right after the opening, the one whose scopes are root-level).
    ///
    /// The receiver claims its transport stream on first read, so a
    /// session without early supplies never touches it.
    early: Option<StreamReceiver<A::Rx>>,
    /// The stage's height, phantom (`fn() -> H` for the auto-trait
    /// shortcut; see [`typed::Node`](crate::tree::typed::Node)).
    height: PhantomData<fn() -> H>,
}

/// The initiator proxy's leaf terminal and accumulated transport work.
pub struct Completing<B, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
    A: Acceptor,
{
    session: Session<B, R, W, C, A>,
    scopes: Receiver<Scope>,
}

impl<B, H, R, W, C, A> protocol::Protocol for Descending<B, H, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
    R: Send,
    W: Send,
    C: Send,
    A: Acceptor,
    H: Height,
    S<H>: Height,
{
    type Height = H;
    type Error = Error<B::Error>;
    type Output = (R, W);
}

impl<B, R, W, C, A> protocol::Protocol for Completing<B, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
    R: Send,
    W: Send,
    C: Send,
    A: Acceptor,
{
    type Height = Z;
    type Error = Error<B::Error>;
    type Output = (R, W);
}

impl<B, R, W, C, A> protocol::CompleteEqual<B> for Connected<B, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
    R: Send,
    W: Send,
    C: Connector,
    A: Acceptor,
{
    /// Return the untouched control halves; equal versions used no streams.
    async fn complete_equal(self) -> Result<(R, W), Self::Error> {
        match self.state {
            ConnectedState::Equal(read, write) => Ok((read, write)),
            ConnectedState::Diverged(..) => unreachable!("equal completion for divergent versions"),
        }
    }
}

impl<B, R, W, C, A> protocol::Initiator<B> for Connected<B, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
    R: Send,
    W: Send,
    C: Connector,
    A: Acceptor,
{
    type Next = Descending<B, UnderRoot, R, W, C, A>;

    /// Replay the remote initiator's opening question from its greeting.
    ///
    /// The opening question's content already crossed inside the greeting's
    /// listing, so no frame is read here. The initiator-direction opening
    /// stream carries the remote's early supplies instead: its receiver is
    /// bound now and handed to the next stage, which reads (and thereby
    /// claims) it only when a root-level request needs an opening supply.
    fn initiator(self) -> (BoxResponses<B, UnderRoot, Self::Error>, Self::Next) {
        let mut session = self.diverged();
        debug_assert_eq!(session.remote, Speaker::Initiator);
        let early = session.incoming::<UnderRoot>();
        let (responses, scopes) = session.work.initiator();
        let next = Descending {
            session,
            scopes,
            early: Some(early),
            height: PhantomData,
        };
        (responses, next)
    }
}

impl<B, R, W, C, A> protocol::Responder<B> for Connected<B, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
    R: Send,
    W: Send,
    C: Connector,
    A: Acceptor,
{
    type Next = Descending<B, UnderUnderRoot, R, W, C, A>;

    /// Proxy the opening: consume the local question, write the early
    /// supplies, decode the remote's top-level reply.
    ///
    /// Binds both opening streams: the incoming (responder-spoken)
    /// opening-reply stream and the outgoing (initiator-spoken)
    /// opening-supply stream. The local opening question sends no frame of
    /// its own — its content rode the greeting — but its trailing supplies
    /// open the outgoing stream when the local initiator holds exclusive
    /// root children.
    fn responder(
        self,
        requests: impl Requests<B, UnderRoot>,
    ) -> (BoxResponses<B, UnderRoot, Self::Error>, Self::Next) {
        let mut session = self.diverged();
        debug_assert_eq!(session.remote, Speaker::Responder);
        let incoming = session.incoming::<UnderRoot>();
        let outgoing = session.outgoing::<UnderRoot>();
        let (responses, next_scopes) = session.work.opening_responder(requests, incoming, outgoing);
        let next = Descending {
            session,
            scopes: next_scopes,
            early: None,
            height: PhantomData,
        };
        (responses, next)
    }
}

impl<B, H, R, W, C, A> protocol::Reply<B> for Descending<B, S<S<H>>, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
    R: Send,
    W: Send,
    C: Connector,
    A: Acceptor,
    H: Height,
    S<H>: Height,
    S<S<H>>: Height,
    S<S<S<H>>>: Height,
{
    type Next = Descending<B, H, R, W, C, A>;

    /// Proxy one ordinary two-height descent transition.
    fn reply(
        mut self,
        requests: impl Requests<B, S<S<H>>>,
    ) -> (BoxResponses<B, S<H>, Self::Error>, Self::Next) {
        let incoming = self.session.incoming::<S<H>>();
        let outgoing = self.session.outgoing::<S<S<H>>>();
        let early = self.early.take();
        let (responses, next_scopes) =
            self.session
                .work
                .internal_replies(requests, self.scopes, incoming, outgoing, early);
        let next = Descending {
            session: self.session,
            scopes: next_scopes,
            early: None,
            height: PhantomData,
        };
        (responses, next)
    }
}

impl<B, R, W, C, A> protocol::Reply<B> for Descending<B, S<Z>, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
    R: Send,
    W: Send,
    C: Connector,
    A: Acceptor,
{
    type Next = Completing<B, R, W, C, A>;

    /// Proxy the leaf-parent transition into the role-specific terminal.
    fn reply(
        mut self,
        requests: impl Requests<B, S<Z>>,
    ) -> (BoxResponses<B, Z, Self::Error>, Self::Next) {
        debug_assert!(
            self.early.is_none(),
            "the opening-supply stream is consumed by the first descending stage"
        );
        let incoming = self.session.incoming::<Z>();
        let outgoing = self.session.outgoing::<S<Z>>();
        let (responses, next_scopes) =
            self.session
                .work
                .leaf_replies(requests, self.scopes, incoming, outgoing);
        let next = Completing {
            session: self.session,
            scopes: next_scopes,
        };
        (responses, next)
    }
}

impl<B, R, W, C, A> protocol::CompleteInitiator<B> for Completing<B, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
    R: Send,
    W: Send,
    C: Connector,
    A: Acceptor,
{
    /// Encode the local responder's final leaf answers and close its stream.
    async fn complete_initiator(
        mut self,
        requests: impl Requests<B, Z>,
    ) -> Result<(R, W), Self::Error> {
        debug_assert_eq!(self.session.remote, Speaker::Initiator);
        let outgoing = self.session.outgoing::<Z>();
        self.session
            .work
            .complete_initiator(requests, self.scopes, outgoing)
            .await
    }
}

impl<B, R, W, C, A> protocol::CompleteResponder<B> for Descending<B, Z, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
    R: Send,
    W: Send,
    C: Connector,
    A: Acceptor,
{
    /// Proxy the final bidirectional leaf exchange to clean completion.
    fn complete_responder(
        mut self,
        requests: impl Requests<B, Z>,
    ) -> (
        BoxResponses<B, Z, Self::Error>,
        impl Future<Output = Result<(R, W), Self::Error>> + Send,
    ) {
        debug_assert_eq!(self.session.remote, Speaker::Responder);
        let incoming = self.session.incoming::<Z>();
        let outgoing = self.session.outgoing::<Z>();
        self.session
            .work
            .complete_responder(requests, self.scopes, incoming, outgoing)
    }
}
