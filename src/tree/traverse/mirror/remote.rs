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
//! Every gossip session begins with the fixed-size preamble frame, exchanged
//! concurrently by both sides before any variable-length traffic; see
//! [`mod@preamble`] for the byte layout, the rejection rules, and the
//! exchange itself. The framed [`message::Handshake`] greeting that follows
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
//! (4-byte big-endian length prefix) through [`framing`]'s exact-read
//! [`FrameRead`]/[`FrameWrite`]: the protocol's height schedule names the
//! type each side expects next, and the frame boundary tells the reader
//! exactly how many bytes belong to that next message. Frame lengths are
//! uncapped — arbitrarily large subtrees travel in one frame — because by
//! the time any length is trusted, the preamble has already vetted the
//! counterparty. The reader never consumes a byte past the frame it was
//! asked for; [`framing`]'s docs explain how that guarantee is what lets one
//! connection host back-to-back sessions.
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

use tokio::io::{AsyncRead, AsyncWrite};

use borsh::{BorshDeserialize, BorshSerialize};

use crate::bookmark::{Bookmark, NoBookmark};
use crate::network::Network;
use crate::tree::typed::{
    Node,
    height::{Height, Root, S, Z},
};

use super::message::{self, UnderRoot, UnderUnderRoot};
use super::protocol::{self, Step};

pub mod framing;
mod preamble;

pub use framing::{Fill, FrameRead, FrameWrite};
pub use preamble::{Staged, preamble};
pub(crate) use preamble::{recv_party, send_party};

/// The error type returned by the gossip protocol.
///
/// Generic over the [`Bookmark`] in play only to carry its
/// [`Error`](Bookmark::Error) in the [`Bookmark`](Self::Bookmark) variant;
/// every other variant is bookmark-independent. The default `B = NoBookmark`
/// has an [uninhabited](std::convert::Infallible) bookmark error.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error<B: Bookmark = NoBookmark> {
    /// An underlying reader/writer error, or a borsh framing error encountered
    /// while parsing a message off the wire.
    #[error(transparent)]
    Io(borsh::io::Error),

    /// The peer's preamble did not begin with [`PROTOCOL_MAGIC`]: the
    /// connection is not speaking the `rumors` protocol at all.
    ///
    /// [`PROTOCOL_MAGIC`]: crate::PROTOCOL_MAGIC
    #[error("peer is not a rumors stream (remote magic: {remote_magic:x?})")]
    MagicMismatch { remote_magic: [u8; 6] },

    /// The peer's magic matched but its protocol version differs from ours.
    /// See [`PROTOCOL_VERSION`].
    ///
    /// [`PROTOCOL_VERSION`]: crate::PROTOCOL_VERSION
    #[error(
        "peer speaks rumors protocol version {remote_version}, we speak {}",
        crate::PROTOCOL_VERSION
    )]
    VersionMismatch { remote_version: u16 },

    /// Both peers were gossiping but belong to different [`Network`]s: they
    /// descend from unrelated [`seed`](crate::Peer::seed)s and must not
    /// combine, even if their causal state happens to look compatible.
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

    /// A retiring peer offered an identity that overlaps one already held here.
    /// Identities in a well-formed universe are disjoint by construction, so
    /// this only arises from a buggy or malicious peer; the session aborts with
    /// our own state untouched.
    #[error("retiring peer's party overlaps ours")]
    PartyOverlap,

    /// The peer's preamble intent byte was neither 0 (remain) nor 1 (retire).
    #[error("peer sent an invalid intent byte ({byte:#04x})")]
    IntentInvalid { byte: u8 },

    /// The peer's preamble frame declared a length other than the preamble's
    /// fixed size, despite carrying our magic and protocol version. No honest
    /// peer produces this, so it indicates a buggy or malicious counterparty.
    #[error("peer's preamble frame declared {declared} bytes")]
    PreambleLengthInvalid { declared: u32 },

    /// The peer declared the bootstrap placeholder [`Network`] together with a
    /// retiring intent. Retiring donates a party and bootstrapping receives
    /// one, so no honest peer combines them; rejecting the combination in the
    /// preamble exchange keeps a buggy or malicious peer from maneuvering us
    /// into both forking it a party and absorbing its donation, which would
    /// orphan the fork's id-region.
    #[error("peer claimed to bootstrap and retire in the same session")]
    BootstrapRetireConflict,

    /// The application's [`Bookmark`] failed to persist or load the local
    /// identity during the session. The wire is unaffected, but the session
    /// aborts: proceeding past an unpersisted identity is exactly the leak the
    /// bookmark exists to prevent.
    #[error(transparent)]
    Bookmark(B::Error),
}

impl<B: Bookmark> From<borsh::io::Error> for Error<B> {
    fn from(e: borsh::io::Error) -> Self {
        Error::Io(e)
    }
}

impl Error<NoBookmark> {
    /// Re-tag a bookmark-free error under any [`Bookmark`].
    ///
    /// The wire-level session machinery is generic-free and produces
    /// `Error<NoBookmark>`; the peer-level drivers return `Error<B>`. Every
    /// variant but [`Bookmark`](Self::Bookmark) is bookmark-independent, and
    /// that one is uninhabited here, so this is a total, lossless re-tag.
    pub(crate) fn widen<B: Bookmark>(self) -> Error<B> {
        match self {
            Error::Io(e) => Error::Io(e),
            Error::MagicMismatch { remote_magic } => Error::MagicMismatch { remote_magic },
            Error::VersionMismatch { remote_version } => Error::VersionMismatch { remote_version },
            Error::NetworkMismatch {
                remote_network,
                remote_min_events,
            } => Error::NetworkMismatch {
                remote_network,
                remote_min_events,
            },
            Error::PartyOverlap => Error::PartyOverlap,
            Error::IntentInvalid { byte } => Error::IntentInvalid { byte },
            Error::PreambleLengthInvalid { declared } => Error::PreambleLengthInvalid { declared },
            Error::BootstrapRetireConflict => Error::BootstrapRetireConflict,
            // Uninhabited at `NoBookmark`: `Infallible` has no values.
            Error::Bookmark(never) => match never {},
        }
    }
}

/// The version state for an [`Exchange`] which has just been initialized but
/// has not yet connected.
pub struct Start;

/// The version state for an [`Exchange`] which has received and sent versions
/// with its peer, and so can proceed to the rest of the protocol.
pub struct Connected;

/// A wire-bound proxy of the counterparty at protocol height `H`. Holds the
/// underlying reader/writer (each wrapped for exact-read framing) and a
/// phantom tag pinning the height; the counterparty's actual zipper lives on
/// the far side of the wire.
pub struct Exchange<T, R, W, V, H: Height> {
    reader: FrameRead<R>,
    writer: FrameWrite<W>,
    #[allow(clippy::type_complexity)]
    _phantom: PhantomData<fn() -> (T, V, H)>,
}

impl<T, R, W> Exchange<T, R, W, Start, Root> {
    /// Wrap the framed halves the [`preamble()`] exchange already used as an
    /// [`Exchange`], ready to start the protocol.
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

impl<T, R, W, V, H: Height> protocol::Stage for Exchange<T, R, W, V, H> {
    type Height = H;
    /// The reconciled tree lives on the local side; the proxy yields its
    /// framed reader/writer halves back to the caller, which stays the
    /// stream's single owner. A session that needs a trailing frame after
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
    W: AsyncWrite + Unpin,
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
    R: AsyncRead + Unpin,
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
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
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
