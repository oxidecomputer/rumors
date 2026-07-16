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
//! Every gossip session begins with the shared fixed-size
//! [`crate::tree::mirror::handshake`] preamble before this adapter
//! is constructed. The framed [`message::Handshake`] greeting that follows
//! carries the causal [`Version`](crate::Version) alone.
//!
//! # Direction
//!
//! When the local responder calls `b.exchange(m)` on its remote-initiator proxy
//! `b`, the `request` `m` is *our* outgoing message, written to the wire, and
//! the return is the remote initiator's response, read back.
//!
//! # Framing
//!
//! Each borsh-encoded message is shipped as a single length-delimited frame
//! (4-byte big-endian length prefix) through
//! [`crate::tree::mirror::framing`]'s exact-read
//! [`FrameRead`]/[`FrameWrite`]: the protocol's height schedule names the
//! type each side expects next, and the frame boundary tells the reader
//! exactly how many bytes belong to that next message. Frame lengths are
//! uncapped — arbitrarily large subtrees travel in one frame — because by
//! the time any length is trusted, the preamble has already vetted the
//! counterparty. The reader never consumes a byte past the frame it was
//! asked for; [`crate::tree::mirror::framing`]'s docs explain how that
//! guarantee is what lets one
//! connection host back-to-back sessions.
//!
//! # In-band termination
//!
//! The protocol's own emptiness predicate drives session termination: a side
//! has converged when its outgoing message has `requested.is_empty() &&
//! uncertain.is_empty()`. Each protocol method reads its response, inspects the
//! appropriate predicate (per the table in [`super::super::protocol`]), and yields
//! [`Step::Continue`] or [`Step::Done`] accordingly. The stream is never closed
//! by the protocol itself: a `(reader, writer)` pair can host multiple
//! back-to-back sync sessions.

use std::convert::Infallible;
use std::marker::PhantomData;

use tokio::io::{AsyncRead, AsyncWrite};

use borsh::{BorshDeserialize, BorshSerialize};

use crate::Error;
use crate::tree::mirror::framing::{FrameRead, FrameWrite};
use crate::tree::typed::{
    Node,
    height::{Height, Root, S, UnderRoot, UnderUnderRoot, Z},
};

use super::super::{
    message,
    protocol::{self, Step},
};

/// The version state for an [`Exchange`] which has just been initialized but
/// has not yet connected.
pub struct Start;

/// The version state for an [`Exchange`] which has received and sent versions
/// with its peer, and so can proceed to the rest of the protocol.
pub struct Connected;

/// A wire-bound proxy of the counterparty at protocol height `H`.
///
/// Holds the underlying reader/writer (each wrapped for exact-read framing) and
/// a phantom tag pinning the height; the counterparty's actual zipper lives on
/// the far side of the wire.
pub struct Exchange<T, R, W, V, H: Height> {
    reader: FrameRead<R>,
    writer: FrameWrite<W>,
    #[allow(clippy::type_complexity)]
    _phantom: PhantomData<fn() -> (T, V, H)>,
}

impl<T, R, W> Exchange<T, R, W, Start, Root> {
    /// Begin an [`Exchange`] on transport halves wrapped after the shared raw
    /// preamble has completed.
    pub fn start(reader: FrameRead<R>, writer: FrameWrite<W>) -> Self {
        Self {
            reader,
            writer,
            _phantom: PhantomData,
        }
    }
}

impl<T, R, W, H: Height> Exchange<T, R, W, Connected, H> {
    /// Construct a [`Connected`]-state [`Exchange`] from already-framed
    /// reader/writer halves, threading them through from a predecessor stage.
    fn connected(reader: FrameRead<R>, writer: FrameWrite<W>) -> Self {
        Self {
            reader,
            writer,
            _phantom: PhantomData,
        }
    }
}

impl<T, R: Send, W: Send, V, H: Height> protocol::Stage for Exchange<T, R, W, V, H> {
    type Height = H;
    /// The reconciled tree lives on the local side; the proxy yields its
    /// framed reader/writer halves back to the caller, which stays the
    /// stream's single owner.
    ///
    /// A session that needs a trailing frame after
    /// the descent (the party hand-off when serving a
    /// [bootstrapping](crate::Peer::bootstrap) peer or absorbing a
    /// [retiring](crate::Peer::retire) one) reads it from the same
    /// [`FrameRead`] the descent used.
    type Output = (FrameRead<R>, FrameWrite<W>);
    type Error = Error;
}

/// Borsh-encode `msg` into a single length-delimited frame and ship it.
///
/// [`FrameWrite::frame`] flushes, so on a clean return the bytes have
/// reached the underlying writer's flush boundary (typically the OS write
/// buffer).
pub(super) async fn send_msg<M, W>(writer: &mut FrameWrite<W>, msg: &M) -> Result<(), Error>
where
    W: AsyncWrite + Unpin + Send,
    M: BorshSerialize,
{
    let mut buf = Vec::new();
    msg.serialize(&mut buf).map_err(Error::Io)?;
    writer.frame(&buf).await.map_err(Error::Io)?;
    Ok(())
}

/// Pull one length-delimited frame off the wire and borsh-decode it as `M`.
///
/// A peer that closes the stream instead of sending the message — cleanly
/// or mid-frame — surfaces as an
/// [`UnexpectedEof`](borsh::io::ErrorKind::UnexpectedEof) borsh I/O error.
pub(super) async fn recv_msg<M, R>(reader: &mut FrameRead<R>) -> Result<M, Error>
where
    R: AsyncRead + Unpin + Send,
    M: BorshDeserialize,
{
    let frame = reader
        .frame()
        .await
        .map_err(|e| match e.kind() {
            borsh::io::ErrorKind::UnexpectedEof => borsh::io::Error::new(
                borsh::io::ErrorKind::UnexpectedEof,
                "peer closed before sending expected message",
            ),
            _ => e,
        })
        .map_err(Error::Io)?;
    M::try_from_slice(&frame).map_err(Error::Io)
}

// One protocol-trait impl block per trait, each at the specific height it
// pertains to. Together with the [`protocol::AfterExchange`] blanket impls,
// they discharge every transition in the protocol's height schedule.

impl<T, R, W> protocol::Accept<T> for Exchange<T, R, W, Start, Root>
where
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
    T: BorshSerialize + BorshDeserialize + Send + Sync,
{
    type Next = Exchange<T, R, W, Connected, Root>;

    async fn accept(
        mut self,
        request: message::Handshake,
    ) -> Result<protocol::Step<message::Handshake, Self::Next, Self::Output>, Self::Error> {
        // `request` is our local caller's handshake; ship it across to the
        // peer, then read the peer's handshake reply. (If our caller is
        // retiring, `request.intent` only announces the hand-off; the party
        // itself travels as a trailing frame after reconciliation, via
        // `send_party`.)
        send_msg(&mut self.writer, &request).await?;
        let peer: message::Handshake = recv_msg(&mut self.reader).await?;

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
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
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
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
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
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
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
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
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

impl<T, R, W> protocol::CloseResponder<T> for Exchange<T, R, W, Connected, S<Z>>
where
    T: BorshDeserialize + Send + Sync,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    type Next = Exchange<T, R, W, Connected, Z>;

    async fn close_responder(
        mut self,
        request: message::Exchange<T, Z>,
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

        if response.requested.is_empty() {
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

impl<T, R, W> protocol::CompleteInitiator<T> for Exchange<T, R, W, Connected, Z>
where
    T: BorshDeserialize + Send + Sync,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    async fn complete_initiator(
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

        // `CompleteInitiator` is statically `Done`: the `Next` slot is
        // `Infallible`, so `Continue` is uninhabitable here.
        Ok(Step::Done {
            msg: response,
            output: (self.reader, self.writer),
        })
    }
}

impl<T, R, W> protocol::CompleteResponder<T> for Exchange<T, R, W, Connected, Z>
where
    T: Send + Sync,
    R: AsyncRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    async fn complete_responder(
        mut self,
        request: message::Complete<T>,
    ) -> Result<Step<(), Infallible, Self::Output>, Error> {
        // Final write; the real responder absorbs this and is done.
        send_msg(&mut self.writer, &request).await?;
        Ok(Step::Done {
            msg: (),
            output: (self.reader, self.writer),
        })
    }
}
