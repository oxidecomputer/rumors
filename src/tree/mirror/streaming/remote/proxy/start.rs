//! The wire participant's protocol handshake states.

use crate::message::{PayloadCodec, PayloadDepthLimit};

use tokio::io::{AsyncRead, AsyncWrite};

use crate::{
    link::{Acceptor, Connector, Link},
    observe::{CaptureRead, Role, SessionHandle},
    tree::{
        mirror::streaming::{
            Backend, Leaf,
            message::{Greeting, initiates},
            protocol::{self, Accept, CompleteConnect, Connect},
            remote::{
                codec::{RunBudget, Speaker, greeting as greeting_codec},
                proxy::{
                    Connected, Error,
                    work::{Physical, Work},
                },
                streams::{AcceptDriver, claims, error_route},
            },
            stats::Recorder,
            window::{Window, WindowConfig},
        },
        typed::{
            Hash,
            height::{Root, Z},
        },
    },
};

/// A wire-bound protocol participant ready for the version handshake.
///
/// Consumes a [`Link`] carrier for one session: the control halves host the
/// causal-version handshake (and are the session's output), the connector
/// and acceptor supply the descent's data streams, and the carrier's epoch
/// labels every stream this session opens.
pub struct Handshaking<B, R, W, C, A, V = Start>
where
    B: Backend<Node<Z>: Leaf>,
{
    backend: B,
    link: Link<R, W, C, A>,
    versions: V,
    /// The session's window choice, resolved against the greeting's
    /// exchanged set sizes; see
    /// [`window`](crate::tree::mirror::streaming::window).
    window: WindowConfig,
    /// The session's stats recorder: every stream this session binds
    /// counts its codec bytes through it.
    stats: Recorder,
    /// The peer's payload codec: the typed ingress every supplied
    /// leaf record decodes through (see [`PayloadCodec`]).
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
    /// `codec` is the peer's payload codec: every leaf
    /// record this session decodes builds its payload through it.
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
    /// can read the codec seam's byte counts after the session completes.
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
pub struct Connecting {
    remote: Greeting,
}

impl<B, R, W, C, A, V> protocol::Protocol for Handshaking<B, R, W, C, A, V>
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
    type Output = (R, W);
}

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

impl<B, R, W, C, A> CompleteConnect<B> for Handshaking<B, R, W, C, A, Connecting>
where
    B: Backend<Node<Z>: Leaf>,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
    C: Connector,
    A: Acceptor,
{
    type Next = Connected<B, R, W, C, A>;

    /// Send the local server's greeting, then open only if versions differ.
    async fn complete_connect(mut self, mut theirs: Greeting) -> Result<Self::Next, Self::Error> {
        // The wire value of the local limit is the codec's: the one
        // configuration every parse of this session already runs under.
        theirs.payload_depth_limit = self.codec.limit().get();
        send::<B::Error, _>(&theirs, &mut self.link.control_write, &self.observe).await?;
        // Payload depth limits must be equal — checked after both
        // greetings are in hand and before the equal-versions resolution,
        // so mixed configurations surface even on converged sessions.
        payload_depth_limits_match::<B::Error>(&self.codec, &self.versions.remote)?;
        let window = self.window.resolve(
            theirs.set_len,
            self.versions.remote.set_len,
            theirs.max_version_bytes,
            self.versions.remote.max_version_bytes,
            B::node_bytes,
        );
        let budget = run_budget(&theirs, &self.versions.remote);
        Ok(connected(
            self.backend,
            window,
            budget,
            theirs,
            self.versions.remote,
            self.link,
            self.stats,
            self.codec,
            self.observe,
        ))
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

    /// Exchange greetings concurrently, then open only if versions differ.
    async fn accept(
        mut self,
        mut request: Greeting,
    ) -> Result<(Greeting, Self::Next), Self::Error> {
        // The wire value of the local limit is the codec's: the one
        // configuration every parse of this session already runs under.
        request.payload_depth_limit = self.codec.limit().get();
        let send = send::<B::Error, _>(&request, &mut self.link.control_write, &self.observe);
        let receive = receive::<B::Error, _>(&mut self.link.control_read, &self.observe);
        let (_, remote) = futures_util::future::try_join(send, receive).await?;
        // Payload depth limits must be equal — checked after both
        // greetings are in hand and before the equal-versions resolution,
        // so mixed configurations surface even on converged sessions.
        payload_depth_limits_match::<B::Error>(&self.codec, &remote)?;
        let greeting = remote.clone();
        let window = self.window.resolve(
            request.set_len,
            remote.set_len,
            request.max_version_bytes,
            remote.max_version_bytes,
            B::node_bytes,
        );
        let budget = run_budget(&request, &remote);
        let next = connected(
            self.backend,
            window,
            budget,
            request,
            remote,
            self.link,
            self.stats,
            self.codec,
            self.observe,
        );
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
    let local = codec.limit();
    let declared = PayloadDepthLimit::new(remote.payload_depth_limit);
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
        .map_err(Error::HandshakeWrite)?;
    write.flush().await.map_err(Error::HandshakeWrite)?;
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
        greeting_codec::ReadGreetingError::Io(io) => Error::HandshakeRead(io),
        greeting_codec::ReadGreetingError::Decode(defect) => Error::HandshakeDecode(defect),
        greeting_codec::ReadGreetingError::Listing(order) => Error::HandshakeListing(order),
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

/// Return untouched control halves on equality, otherwise open the session.
///
/// On equality both carried listings are dropped unused — the documented
/// price of carrying them unconditionally.
#[allow(clippy::too_many_arguments)] // The argument list is the handshake's
// dataflow into the elected session, one premise per argument.
fn connected<B, R, W, C, A>(
    backend: B,
    window: Window,
    budget: RunBudget,
    local: Greeting,
    remote: Greeting,
    link: Link<R, W, C, A>,
    stats: Recorder,
    codec: PayloadCodec,
    observe: SessionHandle,
) -> Connected<B, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
    C: Connector,
    A: Acceptor,
{
    if local.version == remote.version {
        return Connected::equal(link.control_read, link.control_write);
    }
    // The role election of record: the smaller exchanged set initiates,
    // canonical version bytes break ties (`message::initiates`).
    let local = if initiates(
        local.set_len,
        &local.version,
        remote.set_len,
        &remote.version,
    ) {
        Speaker::Initiator
    } else {
        Speaker::Responder
    };
    // The election is decided exactly here; observers learn it before
    // any data stream can open.
    observe.elected(match local {
        Speaker::Initiator => Role::Initiator,
        Speaker::Responder => Role::Responder,
    });
    open(
        backend,
        window,
        budget,
        local,
        remote.max_version_bytes,
        remote.set_len,
        remote.listing,
        link,
        stats,
        codec,
        observe,
    )
}

/// Allocate one session's claim table, error route, and accept driver.
///
/// `peer_listing` is the remote greeting's root-fan listing: replayed as
/// the remote's opening question when the remote wins the initiator
/// election, and merged against the local opening's listing to gate the
/// early-supply stream when it loses. `peer_version_bytes` is the remote
/// greeting's `max_version_bytes`, which the session enforces on every
/// version the remote supplies; `peer_set_len` is its declared set
/// length, which the session charges per supplied record at ingress.
#[allow(clippy::too_many_arguments)]
fn open<B, R, W, C, A>(
    backend: B,
    window: Window,
    budget: RunBudget,
    local: Speaker,
    peer_version_bytes: u64,
    peer_set_len: u64,
    peer_listing: Vec<(u8, Hash)>,
    link: Link<R, W, C, A>,
    stats: Recorder,
    codec: PayloadCodec,
    observe: SessionHandle,
) -> Connected<B, R, W, C, A>
where
    B: Backend<Node<Z>: Leaf>,
    C: Connector,
    A: Acceptor,
{
    let Link {
        control_read,
        control_write,
        connector,
        acceptor,
        session,
    } = link;
    let epoch = session.epoch();
    let remote = local.other();
    let (slots, claims) = claims();
    let (route, errors) = error_route();
    let accept = AcceptDriver::new(acceptor, epoch, remote, slots, route.clone());
    let work = Work::new(
        backend,
        window,
        budget,
        peer_version_bytes,
        peer_set_len,
        peer_listing,
        Physical {
            control_read,
            control_write,
            remote,
            accept,
            errors,
        },
        codec,
    );
    Connected::new(
        remote, epoch, connector, claims, route, budget, stats, observe, work,
    )
}

#[cfg(test)]
mod tests;
