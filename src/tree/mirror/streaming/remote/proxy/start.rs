//! The wire participant's states for exchanging greetings.

use crate::tree::mirror::streaming::message::Grandchildren;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::{
    message::PayloadCodec,
    observe::{CaptureRead, SessionHandle},
    tree::{
        mirror::streaming::{
            Backend, Leaf,
            message::Greeting,
            protocol::{self, Accept, CompleteConnect, Connect},
            remote::{
                codec::{RunBudget, greeting as greeting_codec},
                pipe::PipeRead,
                proxy::{Connected, Error, work::Ingress},
            },
            stats::Recorder,
        },
        typed::height::{Root, Z},
    },
};

/// A wire-bound protocol participant ready for the greeting exchange.
///
/// Consumes the two halves of one duplex byte stream for one session: the
/// greetings, every logical stream's frames, and the session's closing
/// items all share it, and the halves are the session's output.
pub struct Handshaking<B, R, W, V = Start>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// The store used to reconstruct supplied nodes.
    backend: B,
    /// The pipe's read half.
    read: R,
    /// The pipe's write half.
    write: W,
    /// State retained by the current greeting phase.
    versions: V,
    /// The session's stats recorder: the pipe counts its codec bytes
    /// through it.
    stats: Recorder,
    /// This replica's payload codec for supplied messages.
    codec: PayloadCodec,
    /// The session's observation handle: every wire item this session
    /// moves is delivered through it (inert unless a handler attached).
    observe: SessionHandle,
}

impl<B, R, W> Handshaking<B, R, W>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// Bind one session's pipe halves before exchanging causal versions.
    ///
    /// `codec` reconstructs every supplied message received in this session.
    pub fn start(backend: B, read: R, write: W, codec: PayloadCodec) -> Self {
        Self {
            backend,
            read,
            write,
            versions: Start,
            stats: Recorder::default(),
            codec,
            observe: SessionHandle::default(),
        }
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
    /// and every logical stream this session binds deliver their wire
    /// items.
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
impl<B, R, W, V> protocol::Phase for Handshaking<B, R, W, V>
where
    B: Backend<Node<Z>: Leaf>,
    R: Send,
    W: Send,
    V: Send,
{
    type Height = Root;
    type Error = Error<B::Error>;
    type Output = (PipeRead<R>, W);
}

impl<B, R, W, V> Handshaking<B, R, W, V>
where
    B: Backend<Node<Z>: Leaf>,
{
    /// Validate the exchanged premises and retain them for driver dispatch.
    fn connected(
        self,
        local: Greeting,
        remote: Greeting,
    ) -> Result<Connected<B, R, W>, Error<B::Error>> {
        // Check configuration before the driver can take the equal-version
        // shortcut, so agreement on content cannot hide incompatible peers.
        payload_depth_limits_match::<B::Error>(&self.codec, &remote)?;
        let budget = run_budget(&local, &remote);
        let Greeting {
            max_version_bytes: remote_version_bytes,
            set_len: remote_set_len,
            listing: remote_listing,
            grandchildren: remote_grandchildren,
            ..
        } = remote;
        Ok(Connected {
            ingress: Ingress::new(
                self.backend,
                remote_version_bytes,
                remote_set_len,
                // An empty tree counts none; a bound of one reaction above
                // that costs nothing a conforming reply can exceed.
                usize::try_from(remote_grandchildren)
                    .ok()
                    .and_then(Grandchildren::new)
                    .unwrap_or(Grandchildren::ONE),
                self.codec,
            ),
            read: self.read,
            write: self.write,
            remote_listing,
            budget,
            stats: self.stats,
            observe: self.observe,
        })
    }
}

/// The wire participant in the protocol's client position: this impl and
/// its [`CompleteConnect`] continuation exist for the test harness's
/// wire-path arrangement and have no production caller.
impl<B, R, W> Connect<B> for Handshaking<B, R, W>
where
    B: Backend<Node<Z>: Leaf>,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    type Next = Handshaking<B, R, W, Connecting>;

    /// Receive the remote greeting before asking the local server to answer it.
    async fn connect(mut self) -> Result<(Greeting, Self::Next), Self::Error> {
        let remote = receive::<B::Error, _>(&mut self.read, &self.observe).await?;
        let greeting = remote.clone();
        let next = Handshaking {
            backend: self.backend,
            read: self.read,
            write: self.write,
            versions: Connecting { remote },
            stats: self.stats,
            codec: self.codec,
            observe: self.observe,
        };
        Ok((greeting, next))
    }
}

/// The client-position continuation of [`Connect`]: for the test
/// harness's wire-path arrangement, with no production caller.
impl<B, R, W> CompleteConnect<B> for Handshaking<B, R, W, Connecting>
where
    B: Backend<Node<Z>: Leaf>,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    type Next = Connected<B, R, W>;

    /// Send the local participant's greeting and retain the exchanged premises.
    async fn complete_connect(mut self, mut local: Greeting) -> Result<Self::Next, Self::Error> {
        // The wire value of the local limit is the codec's: the one
        // configuration every parse of this session already runs under.
        local.payload_depth_limit = self.codec.limit().get();
        send::<B::Error, _>(&local, &mut self.write, &self.observe).await?;
        let remote = self.versions.remote.clone();
        self.connected(local, remote)
    }
}

impl<B, R, W> Accept<B> for Handshaking<B, R, W>
where
    B: Backend<Node<Z>: Leaf>,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    type Next = Connected<B, R, W>;

    /// Exchange greetings concurrently and retain the exchanged premises.
    async fn accept(mut self, mut local: Greeting) -> Result<(Greeting, Self::Next), Self::Error> {
        // The wire value of the local limit is the codec's: the one
        // configuration every parse of this session already runs under.
        local.payload_depth_limit = self.codec.limit().get();
        let send = send::<B::Error, _>(&local, &mut self.write, &self.observe);
        let receive = receive::<B::Error, _>(&mut self.read, &self.observe);
        let (_, remote) = futures::future::try_join(send, receive).await?;
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

/// Send one greeting: a single self-delimiting control item on the pipe,
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
