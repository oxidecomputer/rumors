//! Wire-bound counterpart to [`super::local`].
//!
//! Where `local::Exchange` realizes the protocol trait family by traversing an
//! in-memory zipper, `remote::Exchange<T, R, W, H>` realizes it as a proxy
//! of the *counterparty*: each protocol method serializes its incoming request
//! into the writer and deserializes the counterparty's response from the
//! reader. The struct carries only a paired `(reader, writer)` plus a phantom
//! tag pinning the protocol height: all of the actual state lives on the
//! counterparty's side of the wire.
//!
//! # Handshake
//!
//! Every gossip session begins with an 8-byte preamble, exchanged
//! concurrently by both sides before any framed traffic:
//!
//! ```text
//! [ R U M O R S | version: u16 (big-endian) ]
//!     magic                   2B
//!      6B
//! ```
//!
//! - **Magic** is [`crate::PROTOCOL_MAGIC`] (`b"RUMORS"`). A peer that opens
//!   the connection with anything else is rejected as
//!   [`Error::MagicMismatch`] — it isn't speaking the `rumors` protocol
//!   at all.
//! - **Version** is [`crate::PROTOCOL_VERSION`], a monotonic `u16`. Patch
//!   versions of `rumors` never change it; minor versions are forward-
//!   compatible (additive wire changes, both sides downgrade to
//!   `min(local, remote)`); major versions may bump it incompatibly and
//!   surface as [`Error::VersionMismatch`].
//!
//! Both sides drive the write and the read concurrently via
//! [`futures_util::future::try_join`]; a peer that reads before writing
//! would deadlock against another peer doing the same on a connection
//! with a tiny kernel write buffer.
//!
//! # Direction
//!
//! When the local responder calls `b.exchange(m)` on its remote-initiator
//! proxy `b`, the `request` `m` is *our* outgoing message --- written to the
//! wire --- and the return is the remote initiator's response, read back.
//!
//! # Framing
//!
//! After the handshake, each borsh-encoded message is shipped as a single
//! length-delimited frame via [`tokio_util::codec::LengthDelimitedCodec`]
//! (4-byte big-endian length prefix). The codec's `max_frame_length` is
//! raised to `usize::MAX` so that arbitrarily large subtrees can travel
//! in one frame; the protocol's height schedule still names the type
//! each side expects next, and the frame boundary now lets the async
//! reader know exactly how many bytes belong to that next message.
//!
//! # In-band termination
//!
//! The protocol's own emptiness predicate drives session termination: a side
//! has converged when its outgoing message has `requested.is_empty() &&
//! uncertain.is_empty()`. Each protocol method reads its response, inspects
//! the appropriate predicate (per the table in [`super::protocol`]), and
//! yields [`Step::Continue`] or [`Step::Done`] accordingly. The stream is
//! never closed by the protocol itself: a `(reader, writer)` pair can host
//! multiple back-to-back sync sessions.

use std::convert::Infallible;
use std::marker::PhantomData;

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

use borsh::{BorshDeserialize, BorshSerialize};

use crate::tree::typed::height::{Height, Root, S, Z};
use crate::version::Version;

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
}

impl From<borsh::io::Error> for Error {
    fn from(e: borsh::io::Error) -> Self {
        Error::Io(e)
    }
}

/// Exchange the 8-byte protocol handshake with a peer.
///
/// The local side writes `[magic(6) | version(u16 BE)]` while concurrently
/// reading the same shape from the peer. The concurrent drive is required: a
/// peer that reads before writing deadlocks against a peer that does the same
/// when the kernel write buffer is smaller than 8 bytes (rare but possible on
/// heavily-tuned sockets).
///
/// Returns [`Error::MagicMismatch`] when the peer's first six bytes are not
/// [`crate::PROTOCOL_MAGIC`], or [`Error::VersionMismatch`] when the magic
/// matches but the version does not.
pub async fn handshake<R, W>(read: &mut R, write: &mut W) -> Result<(), Error>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let v = crate::PROTOCOL_VERSION.to_be_bytes();
    let m = crate::PROTOCOL_MAGIC;
    let local: [u8; 8] = [m[0], m[1], m[2], m[3], m[4], m[5], v[0], v[1]];

    let mut remote = [0u8; 8];
    // Flush after writing the preamble: `write_all` alone only reaches the
    // writer's buffer, and a buffering transport (a compression layer, a
    // `BufWriter`, a TLS record buffer) may hold all 8 bytes back. Since the
    // peer concurrently `read_exact`s 8 bytes before it will send anything
    // further, an unflushed preamble deadlocks both sides. A raw socket
    // forwards immediately and so never exposed this, but the contract on
    // `AsyncWrite` does not promise it.
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
fn make_codec() -> LengthDelimitedCodec {
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
    /// The reconciled tree lives on the local side; the proxy yields no value.
    type Output = ();
    type Error = Error;
}

/// Borsh-encode `msg` into a single length-delimited frame and ship it.
///
/// `SinkExt::send` calls `poll_ready`, `start_send`, and `poll_flush` in
/// sequence, so on a clean return the bytes have reached the underlying
/// writer's flush boundary (typically the OS write buffer).
async fn send_msg<M, W>(
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
async fn recv_msg<M, R>(reader: &mut FramedRead<R, LengthDelimitedCodec>) -> Result<M, Error>
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
        their_version: Version,
    ) -> Result<protocol::Step<Version, Self::Next, Self::Output>, Self::Error> {
        // Ship the version we just received from our local caller across to
        // the remote accepter, then read the remote's reply.
        send_msg(&mut self.writer, &their_version).await?;
        let our_version: Version = recv_msg(&mut self.reader).await?;

        // If the two versions are the same, both sides are immediately done.
        if our_version == their_version {
            return Ok(protocol::Step::Done {
                msg: our_version,
                output: (),
            });
        }

        Ok(protocol::Step::Continue {
            msg: our_version,
            next: Exchange::connected(self.reader, self.writer),
        })
    }
}

impl<T, R, W> protocol::Initiator<T> for Exchange<T, R, W, Connected, Root>
where
    T: BorshDeserialize + Send + Sync,
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
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
{
    type Next = Exchange<T, R, W, Connected, UnderRoot>;

    async fn responder(
        mut self,
        request: message::Initiate,
    ) -> Result<Step<message::Opening, Self::Next, ()>, Error> {
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
{
    type Next = Exchange<T, R, W, Connected, UnderUnderRoot>;

    async fn open_initiator(
        mut self,
        request: message::Opening,
    ) -> Result<Step<message::Exchange<T, UnderUnderRoot>, Self::Next, ()>, Error> {
        send_msg(&mut self.writer, &request).await?;

        // We always await a response: even an empty `Opening` can prompt the
        // counterparty to send back a non-trivial `providing` (the "we have,
        // they lack" Left case when we are the empty side).
        let response: message::Exchange<T, UnderUnderRoot> = recv_msg(&mut self.reader).await?;

        if response.requested.is_empty() && response.uncertain.is_empty() {
            Ok(Step::Done {
                msg: response,
                output: (),
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
    // Assumed at impl-validation time so we don't have to case-analyze `H`
    // here: at use sites `H` is concrete and one of the three blanket impls
    // in `super::protocol` discharges it.
    Exchange<T, R, W, Connected, H>: protocol::AfterExchange<T, H>,
{
    type Next = Exchange<T, R, W, Connected, H>;

    async fn exchange(
        mut self,
        request: message::Exchange<T, S<H>>,
    ) -> Result<Step<message::Exchange<T, H>, Self::Next, ()>, Error> {
        // If the message we just sent will cause the other party to be done,
        // they won't ever respond, so don't await their response.
        let counterparty_finished = request.requested.is_empty() && request.uncertain.is_empty();

        send_msg(&mut self.writer, &request).await?;

        if counterparty_finished {
            return Ok(Step::Done {
                msg: message::Exchange::default(),
                output: (),
            });
        }

        let response: message::Exchange<T, H> = recv_msg(&mut self.reader).await?;

        if response.requested.is_empty() && response.uncertain.is_empty() {
            Ok(Step::Done {
                msg: response,
                output: (),
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
    ) -> Result<Step<message::Closing<T>, Self::Next, ()>, Error> {
        // If the message we just sent will cause the other party to be done,
        // they won't ever respond, so don't await their response.
        let counterparty_finished = request.requested.is_empty() && request.uncertain.is_empty();

        send_msg(&mut self.writer, &request).await?;

        if counterparty_finished {
            return Ok(Step::Done {
                msg: message::Closing::default(),
                output: (),
            });
        }

        let response: message::Closing<T> = recv_msg(&mut self.reader).await?;

        // `CloseInitiator` is the protocol's natural endgame: always `Done`.
        Ok(Step::Done {
            msg: response,
            output: (),
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
    ) -> Result<Step<message::Complete<T>, Infallible, ()>, Error> {
        // If the message we just sent will cause the other party to be done,
        // they won't ever respond, so don't await their response.
        let counterparty_finished = request.requested.is_empty();

        send_msg(&mut self.writer, &request).await?;

        if counterparty_finished {
            return Ok(Step::Done {
                msg: message::Complete::default(),
                output: (),
            });
        }

        let response: message::Complete<T> = recv_msg(&mut self.reader).await?;

        // `CompleteResponder` is statically `Done`: the `Next` slot is
        // `Infallible`, so `Continue` is uninhabitable here.
        Ok(Step::Done {
            msg: response,
            output: (),
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
            output: (),
        })
    }
}
