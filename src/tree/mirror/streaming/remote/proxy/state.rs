//! Typed protocol states over an open per-stream session.
//!
//! Each proxy stage owns the one [`Scope`] queue needed to interpret the local
//! reply it will receive at that height. Its outgoing response stream pumps
//! those local replies to the wire while decoding the remote replies which
//! answer locally-created questions. The active [`Work`] response pump lets a
//! stage yield its reply before publishing the lower scopes derived from it,
//! so one-slot backpressure cannot withhold the reply which releases it.

use std::io::Cursor;
use std::marker::PhantomData;
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::link::{Acceptor, Connector, Link};
use crate::observe::{Role, SessionHandle};
use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        channel::Receiver,
        protocol::{self, BoxResponses, Requests},
        remote::{
            adapter::Scope,
            codec::{RunBudget, Speaker, Stream},
            proxy::{
                Error,
                work::{ControlRead, Physical, Work},
            },
            streams::{
                AcceptDriver, Claims, ErrorRoute, StreamReceiver, StreamSender, claims, error_route,
            },
        },
        stats::Recorder,
        window::Window,
    },
    typed::{
        Hash,
        height::{Height, Root, S, UnderRoot, UnderUnderRoot, Z},
    },
};

/// Session endpoints and backend shared by every state in one proxy chain.
struct Session<B, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
    A: Acceptor,
{
    /// Which role the remote endpoint plays in the wire schedule.
    remote: Speaker,
    /// The link epoch carried by every data-stream label.
    epoch: u8,
    /// Opens data streams spoken by the local endpoint.
    connector: C,
    /// Supplies data streams accepted from the remote endpoint.
    claims: Claims<A::Rx>,
    /// Reports transport failures to the session driver.
    route: ErrorRoute,
    /// The session's negotiated run budget, handed to every incoming
    /// stream this session binds so its codec enforces the budget at
    /// ingress (the outgoing side's copy lives in [`Work`]).
    budget: RunBudget,
    /// The session's stats recorder, handed to every stream this session
    /// binds so the codec seam's byte counts accumulate in one place.
    stats: Recorder,
    /// The session's observation handle, handed to every stream this
    /// session binds so each can create its own observer when it opens.
    observe: SessionHandle,
    /// Encodes local work and decodes remote work across the session.
    work: Work<B, R, W, A>,
}

impl<B, R, W, C, A> Session<B, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
    C: Connector,
    A: Acceptor,
{
    /// Bind the incoming logical stream spoken by the remote at height `H`.
    fn incoming<H: Height>(&mut self) -> StreamReceiver {
        let stream = stream_at::<H>(self.remote);
        StreamReceiver::new(
            self.claims.take(stream),
            self.remote,
            stream,
            self.budget,
            self.route.clone(),
            self.stats.clone(),
            self.observe.clone(),
        )
    }

    /// Bind the outgoing logical stream spoken locally at height `H`.
    fn outgoing<H: Height>(&self) -> StreamSender<C> {
        let local = self.remote.other();
        StreamSender::new(
            self.connector.clone(),
            self.epoch,
            local,
            stream_at::<H>(local),
            self.stats.clone(),
            self.observe.clone(),
        )
    }
}

/// Find the logical stream assigned to one speaker and reply height.
fn stream_at<H: Height>(speaker: Speaker) -> Stream {
    Stream::at_height(speaker, H::HEIGHT).expect("every protocol reply height has one stream")
}

/// A proxy holding exchanged premises until the driver selects its path.
pub struct Connected<B, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// The backend which materializes received nodes.
    pub(super) backend: B,
    /// The transport carrier retained until data-stream role dispatch.
    pub(super) link: Link<R, W, C, A>,
    /// Maximum encoded version size declared by the remote endpoint.
    pub(super) remote_version_bytes: u64,
    /// Message count declared by the remote endpoint.
    pub(super) remote_set_len: u64,
    /// Root-fan listing sent by the remote endpoint.
    pub(super) remote_listing: Vec<(u8, Hash)>,
    /// Queue capacities resolved from both greetings.
    pub(super) window: Window,
    /// Supply-run size accepted by both peers.
    pub(super) budget: RunBudget,
    /// Counts bytes crossing the codec boundary.
    pub(super) stats: Recorder,
    /// Decodes payloads supplied by the remote endpoint.
    pub(super) codec: crate::message::PayloadCodec,
    /// Observes the elected role and wire traffic.
    pub(super) observe: SessionHandle,
}

impl<B, R, W, C, A> Connected<B, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
    C: Connector,
    A: Acceptor,
{
    /// Open the per-stream session after the driver assigns the remote role.
    fn open(self, remote: Speaker) -> Session<B, R, W, C, A> {
        let Connected {
            backend,
            link,
            remote_version_bytes,
            remote_set_len,
            remote_listing,
            window,
            budget,
            stats,
            codec,
            observe,
        } = self;
        observe.elected(match remote.other() {
            Speaker::Initiator => Role::Initiator,
            Speaker::Responder => Role::Responder,
        });
        let Link {
            control_read,
            control_write,
            connector,
            acceptor,
            session,
        } = link;
        let epoch = session.epoch();
        let (slots, claims) = claims();
        let (route, errors) = error_route();
        let accept = AcceptDriver::new(acceptor, epoch, remote, slots, route.clone());
        let work = Work::new(
            backend,
            window,
            budget,
            remote_version_bytes,
            remote_set_len,
            remote_listing,
            Physical {
                control_read,
                control_write,
                remote,
                accept,
                errors,
            },
            codec,
        );
        Session {
            remote,
            epoch,
            connector,
            claims,
            route,
            budget,
            stats,
            observe,
            work,
        }
    }
}

/// A connected wire participant remains at root height until driver dispatch.
impl<B, R, W, C, A> protocol::Phase for Connected<B, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
    R: Send,
    W: Send,
    C: Send,
    A: Acceptor,
{
    type Height = Root;
    type Error = Error<B::Error>;
    type Output = (ControlRead<R>, W);
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
    early: Option<StreamReceiver>,
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

/// A wire descent phase processes the height carried by its state.
impl<B, H, R, W, C, A> protocol::Phase for Descending<B, H, R, W, C, A>
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
    type Output = (ControlRead<R>, W);
}

/// A completing wire participant processes the final leaf-height replies.
impl<B, R, W, C, A> protocol::Phase for Completing<B, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
    R: Send,
    W: Send,
    C: Send,
    A: Acceptor,
{
    type Height = Z;
    type Error = Error<B::Error>;
    type Output = (ControlRead<R>, W);
}

impl<B, R, W, C, A> protocol::CompleteEqual<B> for Connected<B, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
    R: AsyncRead + Unpin + Send,
    W: Send,
    C: Connector,
    A: Acceptor,
{
    /// Return the control halves without opening data streams.
    async fn complete_equal(self) -> Result<(ControlRead<R>, W), Self::Error> {
        let Link {
            control_read,
            control_write,
            ..
        } = self.link;
        Ok((Cursor::new(Vec::new()).chain(control_read), control_write))
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
        let mut session = self.open(Speaker::Initiator);
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
        let mut session = self.open(Speaker::Responder);
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

    /// Proxy the leaf-parent transition into the initiator's terminal.
    fn reply(
        mut self,
        requests: impl Requests<B, S<Z>>,
    ) -> (BoxResponses<B, Z, Self::Error>, Self::Next) {
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
    R: AsyncRead + Unpin + Send,
    W: Send,
    C: Connector,
    A: Acceptor,
{
    /// Encode the local responder's final leaf answers and close its stream.
    async fn complete_initiator(
        self,
        requests: impl Requests<B, Z>,
    ) -> Result<(ControlRead<R>, W), Self::Error> {
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
    R: AsyncRead + Unpin + Send,
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
        impl Future<Output = Result<(ControlRead<R>, W), Self::Error>> + Send,
    ) {
        let incoming = self.session.incoming::<Z>();
        let outgoing = self.session.outgoing::<Z>();
        self.session
            .work
            .complete_responder(requests, self.scopes, incoming, outgoing)
    }
}
