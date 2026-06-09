//! Wire-bound counterpart to [`super::local`].
//!
//! Where `local::Exchange` realizes the protocol trait family by traversing an
//! in-memory zipper, `remote::Exchange<T, R, W, H>` realizes it as a proxy of
//! the *counterparty*: each protocol method serializes its incoming request
//! into the writer and deserializes the counterparty's response from the
//! reader. The struct carries only a paired `(reader, writer)` plus a phantom
//! tag pinning the protocol height: all of the actual state lives on the
//! counterparty's side of the wire.
//!
//! # Handshake
//!
//! Every gossip session begins with a 24-byte preamble, exchanged
//! concurrently by both sides before any framed traffic:
//!
//! ```text
//! [ magic = b"RUMORS": 6B | version: 2B (big-endian) | network: 16B ]
//! ```
//!
//! - **Magic** is [`crate::PROTOCOL_MAGIC`] (`b"RUMORS"`). A peer that opens
//!   the connection with anything else is rejected as [`Error::MagicMismatch`];
//!   it isn't speaking the `rumors` protocol at all.
//!
//! - **Version** is [`crate::PROTOCOL_VERSION`], a monotonic `u16`. Patch
//!   versions of `rumors` never change it; minor versions are forward-
//!   compatible (additive wire changes, both sides downgrade to
//!   `min(local, remote)`); major versions may bump it incompatibly and
//!   surface as [`Error::VersionMismatch`].
//!
//! - **Network** is the 128-bit [`Network`] identifier of this side's universe,
//!   and doubles as the session-intent signal. A real (non-[`ZERO`]) value
//!   means an ordinary peer; the all-zero [`ZERO`] placeholder means this side
//!   is [bootstrapping](super::bootstrap) and holds no universe yet. When
//!   *both* sides carry a real network and the two differ, the session is
//!   rejected as [`Error::NetworkMismatch`]: the peers descend from different
//!   [`seed`](crate::Known::seed)s and must not combine, even if their parties
//!   happen to look disjoint. A bootstrapping side's placeholder suppresses
//!   that check, and the provider's network becomes the value the bootstrapper
//!   adopts.
//!
//! Both sides drive the write and the read concurrently via
//! [`futures_util::future::try_join`]; a peer that reads before writing would
//! deadlock against another peer doing the same on a connection with a tiny
//! write buffer.
//!
//! # Direction
//!
//! When the local responder calls `b.exchange(m)` on its remote-initiator proxy
//! `b`, the `request` `m` is *our* outgoing message, written to the wire, and
//! the return is the remote initiator's response, read back.
//!
//! # Framing
//!
//! After the handshake, each borsh-encoded message is shipped as a single
//! length-delimited frame via [`tokio_util::codec::LengthDelimitedCodec`]
//! (4-byte big-endian length prefix). The codec's `max_frame_length` is raised
//! to `usize::MAX` so that arbitrarily large subtrees can travel in one frame;
//! the protocol's height schedule still names the type each side expects next,
//! and the frame boundary now lets the async reader know exactly how many bytes
//! belong to that next message.
//!
//! # In-band termination
//!
//! The protocol's own emptiness predicate drives session termination: a side
//! has converged when its outgoing message has `requested.is_empty() &&
//! uncertain.is_empty()`. Each protocol method reads its response, inspects the
//! appropriate predicate (per the table in [`super::protocol`]), and yields
//! [`Step::Continue`] or [`Step::Done`] accordingly. The stream is never closed
//! by the protocol itself: a `(reader, writer)` pair can host multiple
//! back-to-back sync sessions.

use std::convert::Infallible;
use std::marker::PhantomData;

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

use borsh::{BorshDeserialize, BorshSerialize};

use crate::network::Network;
use crate::tree::typed::{
    Node,
    height::{Height, Root, S, Z},
};

use super::message::{self, UnderRoot, UnderUnderRoot};
use super::protocol::{self, Step};

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An underlying reader/writer error, or a borsh framing error encountered
    /// while parsing a message off the wire.
    #[error(transparent)]
    Io(borsh::io::Error),

    /// The peer's handshake preamble did not begin with [`PROTOCOL_MAGIC`]:
    /// the connection is not speaking the `rumors` protocol at all.
    ///
    /// [`PROTOCOL_MAGIC`]: crate::PROTOCOL_MAGIC
    #[error("peer is not a rumors stream (remote magic: {remote_magic:x?})")]
    MagicMismatch { remote_magic: [u8; 6] },

    /// The peer's handshake magic matched but its protocol version is
    /// incompatible with ours. See [`PROTOCOL_VERSION`].
    ///
    /// [`PROTOCOL_VERSION`]: crate::PROTOCOL_VERSION
    #[error(
        "peer speaks rumors protocol version {remote_version}, we speak {}",
        crate::PROTOCOL_VERSION
    )]
    VersionMismatch { remote_version: u16 },

    /// Both peers were gossiping but belong to different [`Network`]s: they
    /// descend from unrelated [`seed`](crate::Known::seed)s and must not
    /// combine, regardless of whether their parties appear disjoint. (A
    /// bootstrapping peer sends the placeholder [`Network`], so a session where
    /// either side is bootstrapping never raises this.)
    #[error("peer belongs to a different network ({remote_network:?})")]
    NetworkMismatch {
        /// The network identifier for the remote network.
        remote_network: Network,
        /// A lower-bound for the number of events which have ever been recorded
        /// in the remote network.
        ///
        /// This can be useful as a tie-break heuristic to resolve in favor of
        /// an older network.
        remote_min_events: u64,
    },

    /// A retiring peer offered a [`Party`](before::Party) whose id-region
    /// overlaps ours, so it cannot be [`join`](before::Party::join)ed. In a
    /// well-formed universe every live party is disjoint, so this only arises
    /// from a buggy or malicious peer; we leave our own party untouched and
    /// abort the session.
    #[error("retiring peer's party overlaps ours")]
    PartyOverlap,
}

impl From<borsh::io::Error> for Error {
    fn from(e: borsh::io::Error) -> Self {
        Error::Io(e)
    }
}

/// Exchange and validate the raw protocol preamble `[magic(6) | proto_version(2
/// BE)]` with a peer, before any framed traffic.
///
/// This is the *only* raw (non-length-delimited) exchange in a session; it runs
/// before the [`message::Handshake`] body and the rest of the protocol, so a
/// non-`rumors` peer (wrong magic) or an incompatible one (wrong version) is
/// rejected *before* the length-delimited codec ever trusts a peer-supplied
/// frame length, so that a garbage peer cannot induce a huge-frame allocation.
///
/// The [`Network`] is no longer part of the preamble: it now rides the framed
/// [`message::Handshake`] the `connect`/`accept` steps exchange, where the
/// network-match check is applied.
///
/// Both sides write and read concurrently via [`futures_util::future::try_join`]:
/// a peer that reads before writing would deadlock against another doing the
/// same on a transport whose write buffer is smaller than the preamble.
///
/// Returns [`Error::MagicMismatch`] when the peer's first six bytes are not
/// [`crate::PROTOCOL_MAGIC`], or [`Error::VersionMismatch`] when the magic
/// matches but the version does not.
pub async fn preamble<R, W>(read: &mut R, write: &mut W) -> Result<(), Error>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut local = [0u8; 8];
    local[..6].copy_from_slice(&crate::PROTOCOL_MAGIC);
    local[6..8].copy_from_slice(&crate::PROTOCOL_VERSION.to_be_bytes());

    let mut remote = [0u8; 8];
    // Flush after writing: `write_all` alone only reaches the writer's buffer,
    // and a buffering transport (a compression layer, a `BufWriter`, a TLS
    // record buffer) may hold all 8 bytes back. Since the peer concurrently
    // `read_exact`s 8 bytes before sending anything further, an unflushed
    // preamble deadlocks both sides. A raw socket forwards immediately and so
    // never exposed this, but the `AsyncWrite` contract does not promise it.
    let write_fut = async {
        write.write_all(&local).await.map_err(Error::Io)?;
        write.flush().await.map_err(Error::Io)
    };
    let read_fut = async {
        read.read_exact(&mut remote)
            .await
            .map(|_| ())
            .map_err(Error::Io)
    };
    futures_util::future::try_join(write_fut, read_fut).await?;

    let remote_magic: [u8; 6] = remote[..6].try_into().expect("6 bytes");
    if remote_magic != crate::PROTOCOL_MAGIC {
        return Err(Error::MagicMismatch { remote_magic });
    }
    let remote_version = u16::from_be_bytes([remote[6], remote[7]]);
    if remote_version != crate::PROTOCOL_VERSION {
        return Err(Error::VersionMismatch { remote_version });
    }
    Ok(())
}

/// The version state for an [`Exchange`] which has just been initialized but
/// has not yet connected.
pub struct Start;

/// The version state for an [`Exchange`] which has received and sent versions
/// with its peer, and so can proceed to the rest of the protocol.
pub struct Connected;

/// A wire-bound proxy of the counterparty at protocol height `H`. Holds the
/// underlying reader/writer (each wrapped in a length-delimited codec) and a
/// phantom tag pinning the height; the counterparty's actual zipper lives on
/// the far side of the wire.
pub struct Exchange<T, R, W, V, H: Height> {
    reader: FramedRead<R, LengthDelimitedCodec>,
    writer: FramedWrite<W, LengthDelimitedCodec>,
    #[allow(clippy::type_complexity)]
    _phantom: PhantomData<fn() -> (T, V, H)>,
}

/// Construct a length-delimited codec with the frame-length cap raised to
/// `usize::MAX`. The protocol can ship whole subtrees in a single frame, and
/// we don't want the default 8 MiB cap to fail those legitimately.
pub(super) fn make_codec() -> LengthDelimitedCodec {
    let mut codec = LengthDelimitedCodec::new();
    codec.set_max_frame_length(usize::MAX);
    codec
}

impl<T, R, W> Exchange<T, R, W, Start, Root> {
    /// Wrap a `(reader, writer)` pair as an [`Exchange`], ready to start
    /// the protocol.
    pub fn start(reader: R, writer: W) -> Self {
        Self {
            reader: FramedRead::new(reader, make_codec()),
            writer: FramedWrite::new(writer, make_codec()),
            _phantom: PhantomData,
        }
    }
}

impl<T, R, W, H: Height> Exchange<T, R, W, Connected, H> {
    /// Construct a [`Connected`]-state [`Exchange`] from already-framed
    /// reader/writer halves, threading them through from a predecessor stage.
    fn connected(
        reader: FramedRead<R, LengthDelimitedCodec>,
        writer: FramedWrite<W, LengthDelimitedCodec>,
    ) -> Self {
        Self {
            reader,
            writer,
            _phantom: PhantomData,
        }
    }
}

impl<T, R, W, V, H: Height> protocol::Stage for Exchange<T, R, W, V, H> {
    type Height = H;
    /// The reconciled tree lives on the local side; the proxy yields its framed
    /// reader/writer halves back to the caller. This lets a session that needs a
    /// trailing frame after the descent — the fork-last party hand-off when
    /// serving a [bootstrapping](super::bootstrap) peer — read it from the *same*
    /// [`FramedRead`] the descent used, whose buffer may already hold the
    /// trailing frame's leading bytes (a fresh reader would lose them).
    type Output = (
        FramedRead<R, LengthDelimitedCodec>,
        FramedWrite<W, LengthDelimitedCodec>,
    );
    type Error = Error;
}

/// Borsh-encode `msg` into a single length-delimited frame and ship it.
///
/// `SinkExt::send` calls `poll_ready`, `start_send`, and `poll_flush` in
/// sequence, so on a clean return the bytes have reached the underlying
/// writer's flush boundary (typically the OS write buffer).
pub(super) async fn send_msg<M, W>(
    writer: &mut FramedWrite<W, LengthDelimitedCodec>,
    msg: &M,
) -> Result<(), Error>
where
    W: AsyncWrite + Unpin,
    M: BorshSerialize,
{
    let mut buf = Vec::new();
    msg.serialize(&mut buf).map_err(Error::Io)?;
    writer.send(Bytes::from(buf)).await.map_err(Error::Io)?;
    Ok(())
}

/// Pull one length-delimited frame off the wire and borsh-decode it as `M`.
///
/// A clean end-of-stream (peer closed before sending the expected message) is
/// surfaced as an [`UnexpectedEof`](borsh::io::ErrorKind::UnexpectedEof)
/// borsh I/O error, matching what the synchronous predecessor would have
/// raised mid-`deserialize_reader`.
pub(super) async fn recv_msg<M, R>(
    reader: &mut FramedRead<R, LengthDelimitedCodec>,
) -> Result<M, Error>
where
    R: AsyncRead + Unpin,
    M: BorshDeserialize,
{
    let frame = reader
        .next()
        .await
        .ok_or_else(|| {
            borsh::io::Error::new(
                borsh::io::ErrorKind::UnexpectedEof,
                "peer closed before sending expected message",
            )
        })
        .map_err(Error::Io)?
        .map_err(Error::Io)?;
    M::try_from_slice(&frame).map_err(Error::Io)
}

/// Provider side of the party hand-off that completes bootstrapping a brand-new
/// peer (and is reused by [`Known::retire`](crate::Known::retire)): fork `party`
/// and ship the give-half as one frame, *after* the mirror descent has
/// transferred all content.
///
/// Bootstrapping is not a separate bulk transfer: a peer holding nothing greets
/// with the placeholder [`Network::ZERO`](crate::Network) and an empty tree,
/// then runs the ordinary [mirror descent](super::local) — the empty side pulls
/// all of the provider's content through the usual `providing` channel. The
/// descent moves *content* but not *parties*, so one thing remains: the provider
/// must hand the newcomer a [`Party`](before::Party). That is this single frame.
///
/// # Ordering is load-bearing
///
/// Forking last means a failure during the (large) descent never costs a party
/// region. If the party frame itself is lost, the provider must assume it could
/// *still* have been received: it is not safe to reclaim the forked party, and
/// if it was in fact not received, the party permanently leaks out of the
/// system. No acknowledgement could shrink that residual window to zero — a lost
/// final message leaves the provider unable to tell "peer got the party" from
/// "peer did not" (the two-generals problem) — so forking last is the structural
/// minimum, and it costs no extra round-trip. Because
/// [`Party::fork`](before::Party::fork) splits the identifier space without
/// ticking the clock, a party frame lost in that window costs only a slice of
/// the id space, never causal correctness: the provider's retained half stays a
/// valid, disjoint party.
///
/// The frame travels on the *same* [`FramedWrite`] the descent used (surfaced
/// back to the caller as the remote exchange's output), because the descent's
/// reader on the far side may already have buffered this frame's leading bytes.
pub(crate) async fn send_party_fork<W>(
    party: &mut before::Party,
    writer: &mut FramedWrite<W, LengthDelimitedCodec>,
) -> Result<(), Error>
where
    W: AsyncWrite + Unpin,
{
    let give = party.fork();
    send_msg(writer, &give).await
}

/// Bootstrapper side of the hand-off: read the forked party the provider ships
/// after the descent, off the same reader the descent used. See
/// [`send_party_fork`] for why the fork sits last.
pub(crate) async fn recv_party<R>(
    reader: &mut FramedRead<R, LengthDelimitedCodec>,
) -> Result<before::Party, Error>
where
    R: AsyncRead + Unpin,
{
    recv_msg(reader).await
}

// One protocol-trait impl block per trait, each at the specific height it
// pertains to. Together with the [`protocol::AfterExchange`] blanket impls,
// they discharge every transition in the protocol's height schedule.

impl<T, R, W> protocol::Accept<T> for Exchange<T, R, W, Start, Root>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
    T: BorshSerialize + BorshDeserialize + Send + Sync,
{
    type Next = Exchange<T, R, W, Connected, Root>;

    async fn accept(
        mut self,
        request: message::Handshake,
    ) -> Result<protocol::Step<message::Handshake, Self::Next, Self::Output>, Self::Error> {
        // `request` is our local caller's handshake; ship it across to the peer,
        // then read the peer's handshake reply. (If our caller is retiring,
        // `request.party` is the aliased party we hand over here; dropping the
        // local copy afterward is exactly the ownership transfer retire wants.)
        send_msg(&mut self.writer, &request).await?;
        let peer: message::Handshake = recv_msg(&mut self.reader).await?;

        // Greeting validation, sibling to the magic/version preamble: two real
        // (non-`ZERO`) networks that differ descend from unrelated seeds and
        // must never combine. A `ZERO` on either side (a bootstrapping peer)
        // suppresses the check. Uniform across gossip/retire/bootstrap, so it
        // lives here in the connect phase rather than in each caller's dispatch.
        if !request.network.is_bootstrap()
            && !peer.network.is_bootstrap()
            && request.network != peer.network
        {
            return Err(Error::NetworkMismatch {
                remote_network: peer.network,
                remote_min_events: peer.version.min_ticks(),
            });
        }

        // If the two versions are the same, both sides are immediately done.
        if request.version == peer.version {
            return Ok(protocol::Step::Done {
                msg: peer,
                output: (self.reader, self.writer),
            });
        }

        Ok(protocol::Step::Continue {
            msg: peer,
            next: Exchange::connected(self.reader, self.writer),
        })
    }
}

impl<T, R, W> protocol::Initiator<T> for Exchange<T, R, W, Connected, Root>
where
    T: BorshDeserialize + Send + Sync,
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
    Node<T, UnderRoot>: BorshDeserialize,
{
    type Next = Exchange<T, R, W, Connected, Root>;

    async fn initiator(mut self) -> Result<Step<message::Initiate, Self::Next, Infallible>, Error> {
        // No write: the real initiator (on the far side of the wire) has
        // already shipped its `Initiate` and we are reading it now.
        let msg: message::Initiate = recv_msg(&mut self.reader).await?;
        // `Initiator::initiator` is statically `Continue`: the `Output` slot
        // is `Infallible`, so `Done` is uninhabitable here.
        Ok(Step::Continue {
            msg,
            next: Exchange::connected(self.reader, self.writer),
        })
    }
}

impl<T, R, W> protocol::Responder<T> for Exchange<T, R, W, Connected, Root>
where
    T: BorshDeserialize + Send + Sync,
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
    Node<T, UnderRoot>: BorshDeserialize,
{
    type Next = Exchange<T, R, W, Connected, UnderRoot>;

    async fn responder(
        mut self,
        request: message::Initiate,
    ) -> Result<Step<message::Opening, Self::Next, Self::Output>, Error> {
        send_msg(&mut self.writer, &request).await?;

        // The responder always emits an `Opening`, possibly empty. We can no
        // longer infer termination from an empty `Opening` alone: it can mean
        // either "the trees are equal" or "the responder has no children but
        // we (the initiator) might still have data to provide." Always
        // `Continue` and let the next stage's `open_initiator` decide.
        let response: message::Opening = recv_msg(&mut self.reader).await?;
        Ok(Step::Continue {
            msg: response,
            next: Exchange::connected(self.reader, self.writer),
        })
    }
}

impl<T, R, W> protocol::OpenInitiator<T> for Exchange<T, R, W, Connected, Root>
where
    T: BorshDeserialize + Send + Sync,
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
    Node<T, UnderRoot>: BorshDeserialize,
{
    type Next = Exchange<T, R, W, Connected, UnderUnderRoot>;

    async fn open_initiator(
        mut self,
        request: message::Opening,
    ) -> Result<Step<message::Exchange<T, UnderUnderRoot>, Self::Next, Self::Output>, Error> {
        send_msg(&mut self.writer, &request).await?;

        // We always await a response: even an empty `Opening` can prompt the
        // counterparty to send back a non-trivial `providing` (the "we have,
        // they lack" Left case when we are the empty side).
        let response: message::Exchange<T, UnderUnderRoot> = recv_msg(&mut self.reader).await?;

        if response.requested.is_empty() && response.uncertain.is_empty() {
            Ok(Step::Done {
                msg: response,
                output: (self.reader, self.writer),
            })
        } else {
            Ok(Step::Continue {
                msg: response,
                next: Exchange::connected(self.reader, self.writer),
            })
        }
    }
}

impl<T, R, W, H> protocol::Exchange<T> for Exchange<T, R, W, Connected, S<S<H>>>
where
    T: BorshDeserialize + Send + Sync,
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
    H: Height,
    S<H>: Height,
    S<S<H>>: Height,
    Node<T, S<H>>: BorshDeserialize,
    // Assumed at impl-validation time so we don't have to case-analyze `H`
    // here: at use sites `H` is concrete and one of the three blanket impls
    // in `super::protocol` discharges it.
    Exchange<T, R, W, Connected, H>: protocol::AfterExchange<T, H>,
{
    type Next = Exchange<T, R, W, Connected, H>;

    async fn exchange(
        mut self,
        request: message::Exchange<T, S<H>>,
    ) -> Result<Step<message::Exchange<T, H>, Self::Next, Self::Output>, Error> {
        // If the message we just sent will cause the other party to be done,
        // they won't ever respond, so don't await their response.
        let counterparty_finished = request.requested.is_empty() && request.uncertain.is_empty();

        send_msg(&mut self.writer, &request).await?;

        if counterparty_finished {
            return Ok(Step::Done {
                msg: message::Exchange::default(),
                output: (self.reader, self.writer),
            });
        }

        let response: message::Exchange<T, H> = recv_msg(&mut self.reader).await?;

        if response.requested.is_empty() && response.uncertain.is_empty() {
            Ok(Step::Done {
                msg: response,
                output: (self.reader, self.writer),
            })
        } else {
            Ok(Step::Continue {
                msg: response,
                next: Exchange::connected(self.reader, self.writer),
            })
        }
    }
}

impl<T, R, W> protocol::CloseInitiator<T> for Exchange<T, R, W, Connected, S<S<Z>>>
where
    T: BorshDeserialize + Send + Sync,
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    type Next = Exchange<T, R, W, Connected, Z>;

    async fn close_initiator(
        mut self,
        request: message::Exchange<T, S<Z>>,
    ) -> Result<Step<message::Closing<T>, Self::Next, Self::Output>, Error> {
        // If the message we just sent will cause the other party to be done,
        // they won't ever respond, so don't await their response.
        let counterparty_finished = request.requested.is_empty() && request.uncertain.is_empty();

        send_msg(&mut self.writer, &request).await?;

        if counterparty_finished {
            return Ok(Step::Done {
                msg: message::Closing::default(),
                output: (self.reader, self.writer),
            });
        }

        let response: message::Closing<T> = recv_msg(&mut self.reader).await?;

        // `CloseInitiator` is the protocol's natural endgame: always `Done`.
        Ok(Step::Done {
            msg: response,
            output: (self.reader, self.writer),
        })
    }
}

impl<T, R, W> protocol::CompleteResponder<T> for Exchange<T, R, W, Connected, S<Z>>
where
    T: BorshDeserialize + Send + Sync,
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    async fn complete_responder(
        mut self,
        request: message::Closing<T>,
    ) -> Result<Step<message::Complete<T>, Infallible, Self::Output>, Error> {
        // If the message we just sent will cause the other party to be done,
        // they won't ever respond, so don't await their response.
        let counterparty_finished = request.requested.is_empty();

        send_msg(&mut self.writer, &request).await?;

        if counterparty_finished {
            return Ok(Step::Done {
                msg: message::Complete::default(),
                output: (self.reader, self.writer),
            });
        }

        let response: message::Complete<T> = recv_msg(&mut self.reader).await?;

        // `CompleteResponder` is statically `Done`: the `Next` slot is
        // `Infallible`, so `Continue` is uninhabitable here.
        Ok(Step::Done {
            msg: response,
            output: (self.reader, self.writer),
        })
    }
}

impl<T, R, W> protocol::CompleteInitiator<T> for Exchange<T, R, W, Connected, Z>
where
    T: Send + Sync,
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    async fn complete_initiator(
        mut self,
        request: message::Complete<T>,
    ) -> Result<Step<(), Infallible, Self::Output>, Error> {
        // Final write; the real initiator absorbs this and is done.
        send_msg(&mut self.writer, &request).await?;
        Ok(Step::Done {
            msg: (),
            output: (self.reader, self.writer),
        })
    }
}
