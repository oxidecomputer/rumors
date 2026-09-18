//! The wire participant's states for exchanging greetings.

use tokio::io::{AsyncRead, AsyncWrite};

use crate::{
    link::{Acceptor, Connector, Link},
    message::PayloadCodec,
    observe::{CaptureRead, SessionHandle},
    tree::{
        mirror::streaming::{
            Backend, Leaf,
            message::Greeting,
            protocol::{self, Accept, CompleteConnect, Connect},
            remote::{
                codec::{RunBudget, greeting as greeting_codec},
                proxy::{
                    Connected, Error,
                    work::{ControlRead, Ingress},
                },
            },
            stats::Recorder,
            window::{ReplicaSize, WindowConfig},
        },
        typed::height::{Root, Z},
    },
};

/// A wire-bound protocol participant ready for the greeting exchange.
///
/// Consumes a [`Link`] carrier for one session: the control halves host the
/// greeting exchange (and are the session's output), the connector
/// and acceptor supply the descent's data streams, and the carrier's epoch
/// labels every stream this session opens.
pub struct Handshaking<B, R, W, C, A, V = Start>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// The store used to reconstruct supplied nodes.
    backend: B,
    /// The transport carrier reserved for this session.
    link: Link<R, W, C, A>,
    /// State retained by the current greeting phase.
    versions: V,
    /// The session's window choice, resolved against the greeting's
    /// exchanged set sizes; see
    /// [`window`](crate::tree::mirror::streaming::window).
    window: WindowConfig,
    /// The session's stats recorder: every stream this session binds
    /// counts its codec bytes through it.
    stats: Recorder,
    /// This replica's payload codec for supplied messages.
    codec: PayloadCodec,
    /// The session's observation handle: every wire item this session
    /// moves is delivered through it (inert unless a handler attached).
    observe: SessionHandle,
}

impl<B, R, W, C, A> Handshaking<B, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// Bind one session's link carrier before exchanging causal versions.
    ///
    /// `codec` reconstructs every supplied message received in this session.
    pub fn start(backend: B, link: Link<R, W, C, A>, codec: PayloadCodec) -> Self {
        Self {
            backend,
            link,
            versions: Start,
            window: WindowConfig::default(),
            stats: Recorder::default(),
            codec,
            observe: SessionHandle::default(),
        }
    }

    /// Select this session's window choice; see
    /// [`window`](crate::tree::mirror::streaming::window).
    pub fn window(mut self, window: WindowConfig) -> Self {
        self.window = window;
        self
    }

    /// Share the session's stats recorder, so a driver holding its clone
    /// can read the codec boundary's byte counts after the session completes.
    ///
    /// Without this call the session still counts, into a recorder nobody
    /// reads.
    pub fn stats(mut self, stats: Recorder) -> Self {
        self.stats = stats;
        self
    }

    /// Share the session's observation handle, so the greeting exchange
    /// and every stream this session binds deliver their wire items.
    ///
    /// Without this call the session runs with the inert default.
    pub fn observe(mut self, observe: SessionHandle) -> Self {
        self.observe = observe;
        self
    }
}

/// Handshake state before this participant has sent its version.
pub struct Start;

/// The peer greeting received before the local server produces its response.
///
/// Reached only through the [`Connect`] impl below, which the test
/// harness's wire-path arrangement runs and production never does.
pub struct Connecting {
    /// The remote greeting retained until the local greeting is ready.
    remote: Greeting,
}

/// A wire participant remains at root height through the greeting exchange.
impl<B, R, W, C, A, V> protocol::Phase for Handshaking<B, R, W, C, A, V>
where
    B: Backend<Node<Z>: Leaf>,
    R: Send,
    W: Send,
    C: Send,
    A: Send,
    V: Send,
{
    type Height = Root;
    type Error = Error<B::Error>;
    type Output = (ControlRead<R>, W);
}

impl<B, R, W, C, A, V> Handshaking<B, R, W, C, A, V>
where
    B: Backend<Node<Z>: Leaf>,
    C: Connector,
    A: Acceptor,
{
    /// Validate the exchanged premises and retain them for driver dispatch.
    fn connected(
        self,
        local: Greeting,
        remote: Greeting,
    ) -> Result<Connected<B, R, W, C, A>, Error<B::Error>> {
        // Check configuration before the driver can take the equal-version
        // shortcut, so agreement on content cannot hide incompatible peers.
        payload_depth_limits_match::<B::Error>(&self.codec, &remote)?;
        let window = self.window.resolve(
            [
                ReplicaSize::new(local.set_len, local.max_version_bytes),
                ReplicaSize::new(remote.set_len, remote.max_version_bytes),
            ],
            B::node_bytes,
        );
        let budget = run_budget(&local, &remote);
        let Greeting {
            max_version_bytes: remote_version_bytes,
            set_len: remote_set_len,
            listing: remote_listing,
            ..
        } = remote;
        Ok(Connected {
            ingress: Ingress::new(
                self.backend,
                remote_version_bytes,
                remote_set_len,
                self.codec,
            ),
            link: self.link,
            remote_listing,
            window,
            budget,
            stats: self.stats,
            observe: self.observe,
        })
    }
}

/// The wire participant in the protocol's client position: this impl and
/// its [`CompleteConnect`] continuation exist for the test harness's
/// wire-path arrangement and have no production caller.
impl<B, R, W, C, A> Connect<B> for Handshaking<B, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
    C: Connector,
    A: Acceptor,
{
    type Next = Handshaking<B, R, W, C, A, Connecting>;

    /// Receive the remote greeting before asking the local server to answer it.
    async fn connect(mut self) -> Result<(Greeting, Self::Next), Self::Error> {
        let remote = receive::<B::Error, _>(&mut self.link.control_read, &self.observe).await?;
        let greeting = remote.clone();
        let next = Handshaking {
            backend: self.backend,
            link: self.link,
            versions: Connecting { remote },
            window: self.window,
            stats: self.stats,
            codec: self.codec,
            observe: self.observe,
        };
        Ok((greeting, next))
    }
}

/// The client-position continuation of [`Connect`]: for the test
/// harness's wire-path arrangement, with no production caller.
impl<B, R, W, C, A> CompleteConnect<B> for Handshaking<B, R, W, C, A, Connecting>
where
    B: Backend<Node<Z>: Leaf>,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
    C: Connector,
    A: Acceptor,
{
    type Next = Connected<B, R, W, C, A>;

    /// Send the local participant's greeting and retain the exchanged premises.
    async fn complete_connect(mut self, mut local: Greeting) -> Result<Self::Next, Self::Error> {
        // The wire value of the local limit is the codec's: the one
        // configuration every parse of this session already runs under.
        local.payload_depth_limit = self.codec.limit().get();
        send::<B::Error, _>(&local, &mut self.link.control_write, &self.observe).await?;
        let remote = self.versions.remote.clone();
        self.connected(local, remote)
    }
}

impl<B, R, W, C, A> Accept<B> for Handshaking<B, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
    C: Connector,
    A: Acceptor,
{
    type Next = Connected<B, R, W, C, A>;

    /// Exchange greetings concurrently and retain the exchanged premises.
    async fn accept(mut self, mut local: Greeting) -> Result<(Greeting, Self::Next), Self::Error> {
        // The wire value of the local limit is the codec's: the one
        // configuration every parse of this session already runs under.
        local.payload_depth_limit = self.codec.limit().get();
        let send = send::<B::Error, _>(&local, &mut self.link.control_write, &self.observe);
        let receive = receive::<B::Error, _>(&mut self.link.control_read, &self.observe);
        let (_, remote) = futures_util::future::try_join(send, receive).await?;
        let greeting = remote.clone();
        let next = self.connected(local, remote)?;
        Ok((greeting, next))
    }
}

/// Require the peer's declared payload depth limit to equal ours.
///
/// The limit is a property of the shared set — every replica must be
/// able to hold and forward all content — so it is exchanged for
/// equality, never negotiated: negotiating down is unsound (a peer may
/// already hold messages deeper than a negotiated bound, which it would
/// then not be allowed to gossip), so any negotiation scheme merely
/// relocates the failure to mid-session, conditional on which leaves
/// differ. Both sides detect the mismatch symmetrically, like a network
/// mismatch.
fn payload_depth_limits_match<E>(codec: &PayloadCodec, remote: &Greeting) -> Result<(), Error<E>> {
    let local = codec.limit().get();
    let declared = remote.payload_depth_limit;
    if declared != local {
        return Err(Error::PayloadDepthMismatch {
            local,
            remote: declared,
        });
    }
    Ok(())
}

/// Compute the session's supply-run budget.
///
/// The budget is the smaller of the two greetings' targets, so each
/// side's setting bounds both what it builds and what is built for it,
/// and the more memory-constrained end sets the pace.
fn run_budget(ours: &Greeting, theirs: &Greeting) -> RunBudget {
    let bytes = ours.target_message_size.min(theirs.target_message_size);
    RunBudget::from_bytes(usize::try_from(bytes).unwrap_or(usize::MAX))
}

/// Send one greeting: a single self-delimiting control-stream item,
/// flushed in one hop.
///
/// The spelling lives in
/// [`codec::greeting`](crate::tree::mirror::streaming::remote::codec::greeting).
/// The listing rides inside the item — the wire carriage of the opening
/// question's content (see [`Greeting`] for the always-carry trade).
async fn send<E, W>(
    greeting: &Greeting,
    write: &mut W,
    observe: &SessionHandle,
) -> Result<(), Error<E>>
where
    W: AsyncWrite + Unpin,
{
    use tokio::io::AsyncWriteExt as _;
    let item = greeting_codec::encode_greeting(greeting);
    write
        .write_all(&item)
        .await
        .map_err(|source| Error::GreetingWrite {
            operation: crate::error::TransportOperation::Write,
            source,
        })?;
    write.flush().await.map_err(|source| Error::GreetingWrite {
        operation: crate::error::TransportOperation::Flush,
        source,
    })?;
    observe.control_sent(&item);
    Ok(())
}

/// Receive and canonically decode one greeting item.
///
/// The greeting is peer-controlled, so its whole spelling is enforced
/// on ingress — deterministic heads, the exact key roster, and the
/// listing's canonical strictly-ascending radix order, the same rule the
/// frame codec applies to a wire query — before any scope is built
/// from it.
async fn receive<E, R>(read: &mut R, observe: &SessionHandle) -> Result<Greeting, Error<E>>
where
    R: AsyncRead + Unpin,
{
    let route = |e| match e {
        greeting_codec::ReadGreetingError::Io(io) => Error::GreetingRead(io),
        greeting_codec::ReadGreetingError::Decode(defect) => Error::GreetingDecode(defect),
        greeting_codec::ReadGreetingError::Listing(order) => Error::GreetingListing(order),
    };
    if observe.attached() {
        let mut capture = CaptureRead::new(read);
        let greeting = greeting_codec::read_greeting(&mut capture)
            .await
            .map_err(route)?;
        observe.control_received(capture.bytes());
        Ok(greeting)
    } else {
        greeting_codec::read_greeting(read).await.map_err(route)
    }
}

#[cfg(test)]
mod tests;
