//! Typed protocol states over an open per-stream session.
//!
//! Each proxy stage owns the one [`Scope`] queue needed to interpret the local
//! reply it will receive at that height. Its outgoing response stream pumps
//! those local replies to the wire while decoding the remote replies which
//! answer locally-created questions. The active [`Work`] response pump lets a
//! stage yield its reply before publishing the lower scopes derived from it,
//! so one-slot backpressure cannot withhold the reply which releases it.

use crate::link::{Acceptor, Connector};
use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        channel::Receiver,
        convert::Convert,
        protocol::{self, BoxResponses, Requests, Responses},
        remote::{
            adapter::Scope,
            codec::{Speaker, Stream},
            proxy::{Error, work::Work},
            streams::{Claims, ErrorRoute, StreamReceiver, StreamSender},
        },
    },
    typed::height::{Height, Root, S, UnderRoot, UnderUnderRoot, Z},
};

/// Session endpoints and backend shared by every state in one proxy chain.
struct Session<B, T, R, W, C, A>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    A: Acceptor,
{
    remote: Speaker,
    epoch: u8,
    connector: C,
    claims: Claims<A::Rx>,
    route: ErrorRoute,
    work: Work<B, T, R, W, A>,
}

impl<B, T, R, W, C, A> Session<B, T, R, W, C, A>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    C: Connector,
    A: Acceptor,
{
    /// Bind the incoming logical stream spoken by the remote at `height`.
    fn incoming<H: Height>(&mut self) -> StreamReceiver<A::Rx, T> {
        let stream = stream_at::<H>(self.remote);
        StreamReceiver::new(
            self.claims.take(stream),
            self.remote,
            stream,
            self.route.clone(),
        )
    }

    /// Bind the outgoing logical stream spoken locally at `height`.
    fn outgoing<H: Height>(&mut self) -> StreamSender<C, T> {
        let local = self.remote.other();
        StreamSender::new(
            self.connector.clone(),
            self.epoch,
            local,
            stream_at::<H>(local),
        )
    }
}

/// Find the logical stream assigned to one speaker and reply height.
fn stream_at<H: Height>(speaker: Speaker) -> Stream {
    Stream::at_height(speaker, H::HEIGHT).expect("every protocol reply height has one stream")
}

/// A proxy after the version exchange but before its elected role is known.
pub struct Connected<B, T, R, W, C, A>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    A: Acceptor,
{
    state: ConnectedState<B, T, R, W, C, A>,
}

/// Equal versions need no data streams; divergent versions own a session.
enum ConnectedState<B, T, R, W, C, A>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    A: Acceptor,
{
    Equal(R, W),
    Diverged(Box<Session<B, T, R, W, C, A>>),
}

impl<B, T, R, W, C, A> Connected<B, T, R, W, C, A>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    C: Connector,
    A: Acceptor,
{
    /// Bind an elected session's parts to the remote elected speaker.
    pub fn new(
        remote: Speaker,
        epoch: u8,
        connector: C,
        claims: Claims<A::Rx>,
        route: ErrorRoute,
        work: Work<B, T, R, W, A>,
    ) -> Self {
        Self {
            state: ConnectedState::Diverged(Box::new(Session {
                remote,
                epoch,
                connector,
                claims,
                route,
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
    fn diverged(self) -> Session<B, T, R, W, C, A> {
        match self.state {
            ConnectedState::Diverged(session) => *session,
            ConnectedState::Equal(..) => unreachable!("descent opened for equal versions"),
        }
    }
}

impl<B, T, R, W, C, A> protocol::Protocol for Connected<B, T, R, W, C, A>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
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
pub struct Descending<B, T, H, R, W, C, A>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
    A: Acceptor,
{
    session: Session<B, T, R, W, C, A>,
    scopes: Receiver<Scope<H>>,
}

/// The initiator proxy's leaf terminal and accumulated transport work.
pub struct Completing<B, T, R, W, C, A>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    A: Acceptor,
{
    session: Session<B, T, R, W, C, A>,
    scopes: Receiver<Scope<Z>>,
}

impl<B, T, H, R, W, C, A> protocol::Protocol for Descending<B, T, H, R, W, C, A>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
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

impl<B, T, R, W, C, A> protocol::Protocol for Completing<B, T, R, W, C, A>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    R: Send,
    W: Send,
    C: Send,
    A: Acceptor,
{
    type Height = Z;
    type Error = Error<B::Error>;
    type Output = (R, W);
}

impl<B, T, R, W, C, A> protocol::CompleteEqual<B, T> for Connected<B, T, R, W, C, A>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
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

impl<B, T, R, W, C, A> protocol::Initiator<B, T> for Connected<B, T, R, W, C, A>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    R: Send,
    W: Send,
    C: Connector,
    A: Acceptor,
{
    type Next = Descending<B, T, UnderRoot, R, W, C, A>;

    /// Replay the remote initiator's opening question from its greeting.
    ///
    /// No stream is claimed here: the opening's content already crossed
    /// inside the greeting's listing, and the initiator-direction opening
    /// stream never exists on the wire.
    fn initiator(self) -> (impl Responses<B, T, UnderRoot, Self::Error>, Self::Next) {
        let mut session = self.diverged();
        debug_assert_eq!(session.remote, Speaker::Initiator);
        let (responses, scopes) = session.work.initiator();
        let next = Descending { session, scopes };
        (responses, next)
    }
}

impl<B, T, R, W, C, A> protocol::Responder<B, T> for Connected<B, T, R, W, C, A>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    R: Send,
    W: Send,
    C: Connector,
    A: Acceptor,
    UnderRoot: crate::tree::mirror::streaming::convert::Convert,
{
    type Next = Descending<B, T, UnderUnderRoot, R, W, C, A>;

    /// Proxy the opening: consume the local question, decode the remote's
    /// top-level reply.
    ///
    /// Only the incoming (responder-spoken) opening-reply stream is bound;
    /// the local opening question sends no frame of its own, its content
    /// having ridden the greeting.
    fn responder(
        self,
        requests: impl Requests<B, T, UnderRoot>,
    ) -> (BoxResponses<B, T, UnderRoot, Self::Error>, Self::Next) {
        let mut session = self.diverged();
        debug_assert_eq!(session.remote, Speaker::Responder);
        let incoming = session.incoming::<UnderRoot>();
        let (responses, next_scopes) = session.work.opening_responder(requests, incoming);
        let next = Descending {
            session,
            scopes: next_scopes,
        };
        (responses, next)
    }
}

impl<B, T, H, R, W, C, A> protocol::Reply<B, T> for Descending<B, T, S<S<H>>, R, W, C, A>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    R: Send,
    W: Send,
    C: Connector,
    A: Acceptor,
    H: Height,
    S<H>: Convert,
    S<S<H>>: Convert,
    S<S<S<H>>>: Height,
{
    type Next = Descending<B, T, H, R, W, C, A>;

    /// Proxy one ordinary two-height descent transition.
    fn reply(
        mut self,
        requests: impl Requests<B, T, S<S<H>>>,
    ) -> (BoxResponses<B, T, S<H>, Self::Error>, Self::Next) {
        let incoming = self.session.incoming::<S<H>>();
        let outgoing = self.session.outgoing::<S<S<H>>>();
        let (responses, next_scopes) =
            self.session
                .work
                .internal_replies(requests, self.scopes, incoming, outgoing);
        let next = Descending {
            session: self.session,
            scopes: next_scopes,
        };
        (responses, next)
    }
}

impl<B, T, R, W, C, A> protocol::Reply<B, T> for Descending<B, T, S<Z>, R, W, C, A>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    R: Send,
    W: Send,
    C: Connector,
    A: Acceptor,
{
    type Next = Completing<B, T, R, W, C, A>;

    /// Proxy the leaf-parent transition into the role-specific terminal.
    fn reply(
        mut self,
        requests: impl Requests<B, T, S<Z>>,
    ) -> (BoxResponses<B, T, Z, Self::Error>, Self::Next) {
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

impl<B, T, R, W, C, A> protocol::CompleteInitiator<B, T> for Completing<B, T, R, W, C, A>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    R: Send,
    W: Send,
    C: Connector,
    A: Acceptor,
{
    /// Encode the local responder's final leaf answers and close its stream.
    async fn complete_initiator(
        mut self,
        requests: impl Requests<B, T, Z>,
    ) -> Result<(R, W), Self::Error> {
        debug_assert_eq!(self.session.remote, Speaker::Initiator);
        let outgoing = self.session.outgoing::<Z>();
        self.session
            .work
            .complete_initiator(requests, self.scopes, outgoing)
            .await
    }
}

impl<B, T, R, W, C, A> protocol::CompleteResponder<B, T> for Descending<B, T, Z, R, W, C, A>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    R: Send,
    W: Send,
    C: Connector,
    A: Acceptor,
{
    /// Proxy the final bidirectional leaf exchange to clean completion.
    fn complete_responder(
        mut self,
        requests: impl Requests<B, T, Z>,
    ) -> (
        BoxResponses<B, T, Z, Self::Error>,
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
