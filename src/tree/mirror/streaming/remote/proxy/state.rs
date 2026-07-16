//! Typed protocol states over an open multiplexed session.
//!
//! Each proxy stage owns the one [`Scope`] queue needed to interpret the local
//! reply it will receive at that height. Its outgoing response stream pumps
//! those local replies to the wire while decoding the remote replies which
//! answer locally-created questions. The active [`Work`] response pump lets a
//! stage yield its reply before publishing the lower scopes derived from it,
//! so one-slot backpressure cannot withhold the reply which releases it.

use tokio::io::{AsyncRead, AsyncWrite};

use crate::tree::{
    mirror::streaming::{
        Backend, Leaf, Node,
        channel::Receiver,
        convert::Convert,
        protocol::{self, BoxResponses, Requests, Responses},
        remote::{
            adapter::Scope,
            codec::{Frame, Speaker, Stream},
            proxy::{Error, work::Work},
            session::{Drivers, FrameSender, Incoming, Outgoing},
        },
    },
    typed::height::{Height, Root, S, UnderRoot, UnderUnderRoot, Z},
};

/// Session endpoints and backend shared by every state in one proxy chain.
struct Session<B, T, R, W>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    remote: Speaker,
    incoming: Incoming<T>,
    outgoing: Outgoing<T>,
    work: Work<B, T, R, W>,
}

impl<B, T, R, W> Session<B, T, R, W>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    /// Take the incoming logical stream spoken by the remote at `height`.
    fn incoming<H: Height>(&mut self) -> tokio_stream::wrappers::ReceiverStream<Frame<T>> {
        self.incoming.take(stream_at::<H>(self.remote))
    }

    /// Take the outgoing logical stream spoken locally at `height`.
    fn outgoing<H: Height>(&mut self) -> FrameSender<T> {
        self.outgoing.take(stream_at::<H>(self.remote.other()))
    }
}

/// Find the logical stream assigned to one speaker and reply height.
fn stream_at<H: Height>(speaker: Speaker) -> Stream {
    Stream::at_height(speaker, H::HEIGHT).expect("every protocol reply height has one stream")
}

/// A proxy after the version exchange but before its elected role is known.
pub struct Connected<B, T, R, W>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    session: Session<B, T, R, W>,
}

impl<B, T, R, W> Connected<B, T, R, W>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    /// Bind already-created session endpoints to the remote elected speaker.
    pub fn new(
        backend: B,
        remote: Speaker,
        incoming: Incoming<T>,
        outgoing: Outgoing<T>,
        drivers: Drivers<R, W, T>,
    ) -> Self {
        Self {
            session: Session {
                remote,
                incoming,
                outgoing,
                work: Work::new(backend, drivers),
            },
        }
    }
}

impl<B, T, R, W> protocol::Protocol for Connected<B, T, R, W>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    R: Send,
    W: Send,
{
    type Height = Root;
    type Error = Error<B::Error>;
    type Output = (R, W);
}

/// A proxy inside the descent with scopes for the next local reply stream.
pub struct Descending<B, T, H, R, W>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    H: Height,
    S<H>: Height,
{
    session: Session<B, T, R, W>,
    scopes: Receiver<Scope<H>>,
}

/// The initiator proxy's leaf terminal and accumulated transport work.
pub struct Completing<B, T, R, W>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
{
    session: Session<B, T, R, W>,
    scopes: Receiver<Scope<Z>>,
}

impl<B, T, H, R, W> protocol::Protocol for Descending<B, T, H, R, W>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    R: Send,
    W: Send,
    H: Height,
    S<H>: Height,
{
    type Height = H;
    type Error = Error<B::Error>;
    type Output = (R, W);
}

impl<B, T, R, W> protocol::Protocol for Completing<B, T, R, W>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: Send + Sync + 'static,
    R: Send,
    W: Send,
{
    type Height = Z;
    type Error = Error<B::Error>;
    type Output = (R, W);
}

impl<B, T, R, W> protocol::CompleteEqual<B, T> for Connected<B, T, R, W>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    /// Close every unused logical stream and wait for the peer to do likewise.
    async fn complete_equal(self) -> Result<(R, W), Self::Error> {
        let Session {
            incoming,
            outgoing,
            work,
            ..
        } = self.session;
        work.complete_equal(incoming, outgoing).await
    }
}

impl<B, T, R, W> protocol::Initiator<B, T> for Connected<B, T, R, W>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    type Next = Descending<B, T, UnderRoot, R, W>;

    /// Decode the remote initiator's distinguished opening question.
    fn initiator(mut self) -> (impl Responses<B, T, UnderRoot, Self::Error>, Self::Next) {
        debug_assert_eq!(self.session.remote, Speaker::Initiator);
        let incoming = self.session.incoming::<UnderRoot>();
        let (responses, scopes) = self.session.work.initiator(incoming);
        let next = Descending {
            session: self.session,
            scopes,
        };
        (responses, next)
    }
}

impl<B, T, R, W> protocol::Responder<B, T> for Connected<B, T, R, W>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
    UnderRoot: crate::tree::mirror::streaming::convert::Convert,
{
    type Next = Descending<B, T, UnderUnderRoot, R, W>;

    /// Proxy the distinguished opening in both physical directions.
    fn responder(
        mut self,
        requests: impl Requests<B, T, UnderRoot>,
    ) -> (BoxResponses<B, T, UnderRoot, Self::Error>, Self::Next) {
        debug_assert_eq!(self.session.remote, Speaker::Responder);
        let incoming = self.session.incoming::<UnderRoot>();
        let outgoing = self.session.outgoing::<UnderRoot>();
        let (responses, next_scopes) = self
            .session
            .work
            .opening_responder(requests, incoming, outgoing);
        let next = Descending {
            session: self.session,
            scopes: next_scopes,
        };
        (responses, next)
    }
}

impl<B, T, H, R, W> protocol::Reply<B, T> for Descending<B, T, S<S<H>>, R, W>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
    H: Height,
    S<H>: Convert,
    S<S<H>>: Convert,
    S<S<S<H>>>: Height,
{
    type Next = Descending<B, T, H, R, W>;

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

impl<B, T, R, W> protocol::Reply<B, T> for Descending<B, T, S<Z>, R, W>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    type Next = Completing<B, T, R, W>;

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

impl<B, T, R, W> protocol::CompleteInitiator<B, T> for Completing<B, T, R, W>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
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

impl<B, T, R, W> protocol::CompleteResponder<B, T> for Descending<B, T, Z, R, W>
where
    B: Backend<T, Node<Z>: Leaf<T>>,
    T: borsh::BorshDeserialize + Send + Sync + 'static,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
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
