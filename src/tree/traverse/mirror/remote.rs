//! Wire-bound counterpart to [`super::local`].
//!
//! Where `local::Exchange` realizes the protocol trait family by traversing an
//! in-memory zipper, `remote::Exchange<P, T, R, W, H>` realizes it as a proxy
//! of the *counterparty*: each protocol method serializes its incoming request
//! into the writer and deserializes the counterparty's response from the
//! reader. The struct carries only a paired `(reader, writer)` plus a phantom
//! tag pinning the protocol height: all of the actual state lives on the
//! counterparty's side of the wire.
//!
//! # Direction
//!
//! When the local responder calls `b.exchange(m)` on its remote-initiator
//! proxy `b`, the `request` `m` is *our* outgoing message --- written to the
//! wire --- and the return is the remote initiator's response, read back.
//!
//! # Framing
//!
//! Each borsh-encoded message is shipped as a single length-delimited frame
//! via [`tokio_util::codec::LengthDelimitedCodec`] (4-byte big-endian length
//! prefix). The codec's `max_frame_length` is raised to `usize::MAX` so that
//! arbitrarily large subtrees can travel in one frame; the protocol's height
//! schedule still names the type each side expects next, and the frame
//! boundary now lets the async reader know exactly how many bytes belong to
//! that next message.
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
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

use borsh::{BorshDeserialize, BorshSerialize};

use crate::tree::typed::{
    Node,
    height::{Height, Root, S, Z},
};
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
}

impl From<borsh::io::Error> for Error {
    fn from(e: borsh::io::Error) -> Self {
        Error::Io(e)
    }
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
pub struct Exchange<P, T, R, W, V, H: Height> {
    reader: FramedRead<R, LengthDelimitedCodec>,
    writer: FramedWrite<W, LengthDelimitedCodec>,
    #[allow(clippy::type_complexity)]
    _phantom: PhantomData<fn() -> (P, T, V, H)>,
}

/// Construct a length-delimited codec with the frame-length cap raised to
/// `usize::MAX`. The protocol can ship whole subtrees in a single frame, and
/// we don't want the default 8 MiB cap to fail those legitimately.
fn make_codec() -> LengthDelimitedCodec {
    let mut codec = LengthDelimitedCodec::new();
    codec.set_max_frame_length(usize::MAX);
    codec
}

impl<P, T, R, W> Exchange<P, T, R, W, Start, Root> {
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

impl<P, T, R, W, H: Height> Exchange<P, T, R, W, Connected, H> {
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

impl<P, T, R, W, V, H: Height> protocol::Stage for Exchange<P, T, R, W, V, H> {
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

impl<P, T, R, W> protocol::Accept<P, T> for Exchange<P, T, R, W, Start, Root>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
    T: BorshSerialize + BorshDeserialize,
    P: BorshSerialize + BorshDeserialize + Clone + Ord + AsRef<[u8]>,
{
    type Next = Exchange<P, T, R, W, Connected, Root>;

    async fn accept(
        mut self,
        their_version: Version<P>,
    ) -> Result<protocol::Step<Version<P>, Self::Next, Self::Output>, Self::Error> {
        // Ship the version we just received from our local caller across to
        // the remote accepter, then read the remote's reply.
        send_msg(&mut self.writer, &their_version).await?;
        let our_version: Version<P> = recv_msg(&mut self.reader).await?;

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

impl<P, T, R, W> protocol::Initiator<P, T> for Exchange<P, T, R, W, Connected, Root>
where
    P: Clone + Ord + AsRef<[u8]> + BorshSerialize + BorshDeserialize,
    T: BorshDeserialize,
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
    Node<P, T, UnderRoot>: BorshDeserialize,
{
    type Next = Exchange<P, T, R, W, Connected, Root>;

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

impl<P, T, R, W> protocol::Responder<P, T> for Exchange<P, T, R, W, Connected, Root>
where
    P: Clone + Ord + AsRef<[u8]> + BorshSerialize + BorshDeserialize,
    T: BorshDeserialize,
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
    Node<P, T, UnderRoot>: BorshDeserialize,
{
    type Next = Exchange<P, T, R, W, Connected, UnderRoot>;

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

impl<P, T, R, W> protocol::OpenInitiator<P, T> for Exchange<P, T, R, W, Connected, Root>
where
    P: Clone + Ord + AsRef<[u8]> + BorshSerialize + BorshDeserialize,
    T: BorshDeserialize,
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
    Node<P, T, UnderRoot>: BorshDeserialize,
{
    type Next = Exchange<P, T, R, W, Connected, UnderUnderRoot>;

    async fn open_initiator(
        mut self,
        request: message::Opening,
    ) -> Result<Step<message::Exchange<P, T, UnderUnderRoot>, Self::Next, ()>, Error> {
        send_msg(&mut self.writer, &request).await?;

        // We always await a response: even an empty `Opening` can prompt the
        // counterparty to send back a non-trivial `providing` (the "we have,
        // they lack" Left case when we are the empty side).
        let response: message::Exchange<P, T, UnderUnderRoot> = recv_msg(&mut self.reader).await?;

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

impl<P, T, R, W, H> protocol::Exchange<P, T> for Exchange<P, T, R, W, Connected, S<S<H>>>
where
    P: Clone + Ord + AsRef<[u8]> + BorshSerialize + BorshDeserialize,
    T: BorshDeserialize,
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
    H: Height,
    S<H>: Height,
    S<S<H>>: Height,
    Node<P, T, S<H>>: BorshDeserialize,
    // Assumed at impl-validation time so we don't have to case-analyze `H`
    // here: at use sites `H` is concrete and one of the three blanket impls
    // in `super::protocol` discharges it.
    Exchange<P, T, R, W, Connected, H>: protocol::AfterExchange<P, T, H>,
{
    type Next = Exchange<P, T, R, W, Connected, H>;

    async fn exchange(
        mut self,
        request: message::Exchange<P, T, S<H>>,
    ) -> Result<Step<message::Exchange<P, T, H>, Self::Next, ()>, Error> {
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

        let response: message::Exchange<P, T, H> = recv_msg(&mut self.reader).await?;

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

impl<P, T, R, W> protocol::CloseInitiator<P, T> for Exchange<P, T, R, W, Connected, S<S<Z>>>
where
    P: Clone + Ord + AsRef<[u8]> + BorshSerialize + BorshDeserialize,
    T: BorshDeserialize,
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    type Next = Exchange<P, T, R, W, Connected, Z>;

    async fn close_initiator(
        mut self,
        request: message::Exchange<P, T, S<Z>>,
    ) -> Result<Step<message::Closing<P, T>, Self::Next, ()>, Error> {
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

        let response: message::Closing<P, T> = recv_msg(&mut self.reader).await?;

        // `CloseInitiator` is the protocol's natural endgame: always `Done`.
        Ok(Step::Done {
            msg: response,
            output: (),
        })
    }
}

impl<P, T, R, W> protocol::CompleteResponder<P, T> for Exchange<P, T, R, W, Connected, S<Z>>
where
    P: Clone + Ord + AsRef<[u8]> + BorshSerialize + BorshDeserialize,
    T: BorshDeserialize,
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    async fn complete_responder(
        mut self,
        request: message::Closing<P, T>,
    ) -> Result<Step<message::Complete<P, T>, Infallible, ()>, Error> {
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

        let response: message::Complete<P, T> = recv_msg(&mut self.reader).await?;

        // `CompleteResponder` is statically `Done`: the `Next` slot is
        // `Infallible`, so `Continue` is uninhabitable here.
        Ok(Step::Done {
            msg: response,
            output: (),
        })
    }
}

impl<P, T, R, W> protocol::CompleteInitiator<P, T> for Exchange<P, T, R, W, Connected, Z>
where
    P: Clone + Ord + AsRef<[u8]> + BorshSerialize,
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    async fn complete_initiator(
        mut self,
        request: message::Complete<P, T>,
    ) -> Result<Step<(), Infallible, Self::Output>, Error> {
        // Final write; the real initiator absorbs this and is done.
        send_msg(&mut self.writer, &request).await?;
        Ok(Step::Done {
            msg: (),
            output: (),
        })
    }
}
