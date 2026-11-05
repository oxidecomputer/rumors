//! Typed protocol states over an open session pipe.
//!
//! Each proxy stage owns the one [`Scope`] queue needed to interpret the local
//! reply it will receive at that height. Background tasks write local replies
//! while decoding the remote replies which answer local questions. Decoded
//! replies park until their stage takes them, so decoding never waits on the
//! walk; each take publishes the scopes its reply's questions create before
//! the stage receives the reply.

use crate::tree::mirror::streaming::ledger::Ledger;
use crate::tree::mirror::streaming::message::Listed;
use std::marker::PhantomData;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::observe::{Role, SessionHandle};
use crate::tree::{
    mirror::streaming::{
        Backend, Leaf,
        channel::Receiver,
        protocol::{self, BoxResponses, Requests},
        remote::{
            adapter::Scope,
            codec::{RunBudget, Speaker},
            pipe::{Demultiplexer, Multiplexer, PipeRead},
            proxy::{
                Error,
                work::{Ingress, Physical, Work},
            },
        },
        stats::Recorder,
        window::Window,
    },
    typed::height::{Height, Root, S, UnderRoot, UnderUnderRoot, Z},
};

/// A proxy holding exchanged premises until the driver selects its path.
pub struct Connected<B, R, W>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// Backend and validation state for incoming replies.
    pub(super) ingress: Ingress<B>,
    /// The pipe's read half, retained until role dispatch.
    pub(super) read: R,
    /// The pipe's write half, retained until role dispatch.
    pub(super) write: W,
    /// Root-fan listing sent by the remote endpoint.
    pub(super) remote_listing: Vec<Listed>,
    /// The session window, adopted from the local walk that resolved it
    /// from both greetings ([`ShareWindow`](protocol::ShareWindow)); `None`
    /// until the driver hands it over.
    pub(super) window: Option<Window>,
    /// Supply-run size accepted by both peers.
    pub(super) budget: RunBudget,
    /// The endpoint's ledger, adopted from the local walk.
    pub(super) ledger: Ledger,
    /// Counts bytes crossing the codec boundary.
    pub(super) stats: Recorder,
    /// Observes the elected role and wire traffic.
    pub(super) observe: SessionHandle,
}

impl<B, R, W> Connected<B, R, W>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// Open the session's pipe after the driver assigns the remote role.
    ///
    /// # Panics
    ///
    /// Panics if no window was adopted, which only a driver pairing the
    /// proxy with a participant that resolves none can cause: a programmer
    /// error, since every endpoint pairs its proxy with its walk.
    fn open(self, remote: Speaker) -> Work<B, R, W> {
        let Connected {
            ingress,
            read,
            write,
            remote_listing,
            window,
            budget,
            ledger,
            stats,
            observe,
        } = self;
        let window = window
            .expect("the driver hands the proxy its walk's window before either opens a role");
        observe.elected(match remote.other() {
            Speaker::Initiator => Role::Initiator,
            Speaker::Responder => Role::Responder,
        });
        Work::new(
            ingress,
            window,
            budget,
            remote_listing,
            ledger,
            Physical {
                read,
                write,
                remote,
                multiplexer: Multiplexer::default(),
                demultiplexer: Demultiplexer::default(),
            },
            stats,
            observe,
        )
    }
}

/// The proxy holds no window configuration of its own: it adopts the window
/// its local walk resolved, which parking's capacity rests on.
impl<B, R, W> protocol::ShareWindow for Connected<B, R, W>
where
    B: Backend<Node<Z>: Leaf>,
    R: Send,
    W: Send,
{
    /// Resolve nothing: the proxy's window is its walk's.
    fn resolve_window(&mut self) -> Option<Window> {
        None
    }

    /// Adopt the walk's window for every queue and parking lane.
    fn adopt_window(&mut self, window: Window) {
        self.window = Some(window);
    }

    /// Adopt the walk's ledger, which the proxy's records are charged to.
    fn adopt_ledger(&mut self, ledger: Ledger) {
        self.ledger = ledger;
    }
}

/// A connected wire participant remains at root height until driver dispatch.
impl<B, R, W> protocol::Phase for Connected<B, R, W>
where
    B: Backend<Node<Z>: Leaf>,
    R: Send,
    W: Send,
{
    type Height = Root;
    type Error = Error<B::Error>;
    type Output = (PipeRead<R>, W);
}

/// A proxy inside the descent with scopes for the next local reply stream.
pub struct Descending<B, H, R, W>
where
    B: Backend<Node<Z>: Leaf>,
    H: Height,
    S<H>: Height,
{
    /// The session's pipe and the work accumulated so far.
    work: Work<B, R, W>,
    /// The next local reply's scopes, erased: the typestate's `H` is what
    /// pins this queue to the stage that consumes it at the right height,
    /// and every scope's parent prefix carries the runtime witness.
    scopes: Receiver<Scope>,
    /// The stage's height, phantom (`fn() -> H` for the auto-trait
    /// shortcut; see [`typed::Node`](crate::tree::typed::Node)).
    height: PhantomData<fn() -> H>,
}

/// The initiator proxy's leaf terminal and accumulated transport work.
pub struct Completing<B, R, W>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// The session's pipe and the work accumulated through the descent.
    work: Work<B, R, W>,
    /// The leaf scopes awaiting the local terminal replies.
    scopes: Receiver<Scope>,
}

/// A wire descent phase processes the height carried by its state.
impl<B, H, R, W> protocol::Phase for Descending<B, H, R, W>
where
    B: Backend<Node<Z>: Leaf>,
    R: Send,
    W: Send,
    H: Height,
    S<H>: Height,
{
    type Height = H;
    type Error = Error<B::Error>;
    type Output = (PipeRead<R>, W);
}

/// A completing wire participant processes the final leaf-height replies.
impl<B, R, W> protocol::Phase for Completing<B, R, W>
where
    B: Backend<Node<Z>: Leaf>,
    R: Send,
    W: Send,
{
    type Height = Z;
    type Error = Error<B::Error>;
    type Output = (PipeRead<R>, W);
}

impl<B, R, W> protocol::CompleteEqual<B> for Connected<B, R, W>
where
    B: Backend<Node<Z>: Leaf>,
    R: AsyncRead + Unpin + Send,
    W: Send,
{
    /// Return the pipe's halves without sending a frame.
    async fn complete_equal(self) -> Result<(PipeRead<R>, W), Self::Error> {
        Ok((PipeRead::new(self.read), self.write))
    }
}

impl<B, R, W> protocol::Initiator<B> for Connected<B, R, W>
where
    B: Backend<Node<Z>: Leaf>,
    R: Send,
    W: Send,
{
    type Next = Descending<B, UnderRoot, R, W>;

    /// Replay the remote initiator's opening.
    ///
    /// [`Work::initiator`] reconstructs the opening question from the
    /// greeting; no frame carries it.
    fn initiator(self) -> (BoxResponses<B, UnderRoot, Self::Error>, Self::Next) {
        let mut work = self.open(Speaker::Initiator);
        let (responses, scopes) = work.initiator();
        let next = Descending {
            work,
            scopes,
            height: PhantomData,
        };
        (responses, next)
    }
}

impl<B, R, W> protocol::Responder<B> for Connected<B, R, W>
where
    B: Backend<Node<Z>: Leaf>,
    R: Send,
    W: Send,
{
    type Next = Descending<B, UnderUnderRoot, R, W>;

    /// Bind the remote responder's opening stream and proxy the local
    /// initiator's opening.
    ///
    /// [`Work::opening_responder`] owns the encoding and decoding mechanics.
    fn responder(
        self,
        requests: impl Requests<B, UnderRoot>,
    ) -> (BoxResponses<B, UnderRoot, Self::Error>, Self::Next) {
        let mut work = self.open(Speaker::Responder);
        let (responses, scopes) = work.opening_responder(requests);
        let next = Descending {
            work,
            scopes,
            height: PhantomData,
        };
        (responses, next)
    }
}

impl<B, H, R, W> protocol::Reply<B> for Descending<B, S<S<H>>, R, W>
where
    B: Backend<Node<Z>: Leaf>,
    R: Send,
    W: Send,
    H: Height,
    S<H>: Height,
    S<S<H>>: Height,
    S<S<S<H>>>: Height,
{
    type Next = Descending<B, H, R, W>;

    /// Proxy one ordinary two-height descent transition.
    fn reply(
        mut self,
        requests: impl Requests<B, S<S<H>>>,
    ) -> (BoxResponses<B, S<H>, Self::Error>, Self::Next) {
        let (responses, scopes) = self.work.internal_replies(requests, self.scopes);
        let next = Descending {
            work: self.work,
            scopes,
            height: PhantomData,
        };
        (responses, next)
    }
}

impl<B, R, W> protocol::Reply<B> for Descending<B, S<Z>, R, W>
where
    B: Backend<Node<Z>: Leaf>,
    R: Send,
    W: Send,
{
    type Next = Completing<B, R, W>;

    /// Proxy the leaf-parent transition into the initiator's terminal.
    fn reply(
        mut self,
        requests: impl Requests<B, S<Z>>,
    ) -> (BoxResponses<B, Z, Self::Error>, Self::Next) {
        let (responses, scopes) = self.work.leaf_replies(requests, self.scopes);
        let next = Completing {
            work: self.work,
            scopes,
        };
        (responses, next)
    }
}

impl<B, R, W> protocol::CompleteInitiator<B> for Completing<B, R, W>
where
    B: Backend<Node<Z>: Leaf>,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    /// Encode the local responder's final leaf answers and close its stream.
    async fn complete_initiator(
        self,
        requests: impl Requests<B, Z>,
    ) -> Result<(PipeRead<R>, W), Self::Error> {
        self.work.complete_initiator(requests, self.scopes).await
    }
}

impl<B, R, W> protocol::CompleteResponder<B> for Descending<B, Z, R, W>
where
    B: Backend<Node<Z>: Leaf>,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    /// Proxy the final bidirectional leaf exchange to clean completion.
    fn complete_responder(
        self,
        requests: impl Requests<B, Z>,
    ) -> (
        BoxResponses<B, Z, Self::Error>,
        impl Future<Output = Result<(PipeRead<R>, W), Self::Error>> + Send,
    ) {
        self.work.complete_responder(requests, self.scopes)
    }
}
