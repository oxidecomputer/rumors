//! Failures surfaced by the remote protocol participant.

use crate::message::PayloadDepthLimit;
use crate::tree::mirror::streaming::remote::{adapter, codec, streams};

/// A protocol or adapter failure while proxying one remote counterparty.
///
/// When the incoming stream supply fails, transport errors may be reported
/// as [`SupplyClosed`](streams::StreamError::SupplyClosed) with the supply's
/// I/O error as their source. Backend failures and violations of the protocol
/// or wire format retain their own identity.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error<E> {
    /// Reading one of the peer's greeting frames failed.
    #[error("failed to read streaming handshake")]
    HandshakeRead(#[source] std::io::Error),
    /// The peer's greeting arrived but is not canonical rumors CBOR.
    #[error("failed to decode streaming handshake")]
    HandshakeDecode(#[source] codec::GreetingError),
    /// Writing and flushing the local greeting frames failed.
    #[error("failed to write streaming handshake")]
    HandshakeWrite(#[source] std::io::Error),
    /// The peer's greeting listing violated canonical ascending radix order.
    #[error("peer greeting carried a non-canonical root-fan listing")]
    HandshakeListing(#[source] codec::QueryOrderError),
    /// The peer's configured payload depth limit differs from ours.
    ///
    /// Detected symmetrically, after the greetings and before anything
    /// else, so a mixed fleet is caught even on a converged session.
    #[error("peer's payload depth limit ({remote}) differs from ours ({local})")]
    PayloadDepthMismatch {
        /// This side's configured limit.
        local: PayloadDepthLimit,
        /// The limit the peer's greeting declared.
        remote: PayloadDepthLimit,
    },
    /// The locally-produced distinguished opening could not be encoded.
    #[error("local opening reply is invalid")]
    OpeningEncode(#[source] adapter::OpeningError),
    /// A normal local reply could not be converted to wire frames.
    #[error(transparent)]
    Encode(#[from] adapter::EncodeError<E>),
    /// Normal remote wire frames could not be reconstructed as a reply.
    #[error(transparent)]
    Decode(#[from] adapter::DecodeError<E>),
    /// A frame constructed by the adapter violated the reply-only boundary.
    #[error(transparent)]
    ReplyFrame(#[from] streams::ReplyFrameError),
    /// An outgoing logical stream could not be opened, labeled, or written.
    #[error(transparent)]
    Send(#[from] streams::SendError),
    /// An incoming logical stream failed to decode or ended prematurely.
    #[error(transparent)]
    Stream(#[from] streams::StreamError),
    /// An incoming transport stream could not be accepted or routed.
    #[error(transparent)]
    Accept(#[from] streams::AcceptError),
    /// The local opening stream omitted its distinguished question.
    #[error("opening stream ended before its distinguished question")]
    MissingOpening,
    /// The local opening stream contained more than its distinguished question.
    #[error("local opening stream contained an additional reply")]
    ExtraOpening,
    /// A remote logical stream supplied a reply which answered no local query.
    #[error("remote logical stream contained an unasked reply")]
    UnaskedReply,
    /// The local protocol produced a reply which answered no remote query.
    #[error("local protocol produced an unasked reply")]
    UnaskedLocalReply,
    /// The local protocol ended a reply stream while a remote query remained.
    #[error("local protocol left a remote query unanswered")]
    UnansweredRemoteQuery,
    /// The terminal responder attempted to ask another leaf question.
    #[error("terminal responder reply contained another query")]
    TerminalQuery,
}

impl<E> Error<E> {
    /// Whether loss of the stream supply could explain this failure.
    ///
    /// Only transport reads, writes, opens, and truncated streams qualify.
    /// Decoding a complete but invalid record is a violation, even when its
    /// decoder uses an I/O error type to describe the invalid content.
    pub(super) fn is_transport_failure(&self) -> bool {
        match self {
            Self::Send(
                streams::SendError::Connect { .. }
                | streams::SendError::Label { .. }
                | streams::SendError::Frame(codec::EncodeError {
                    kind: codec::EncodeErrorKind::Write { .. } | codec::EncodeErrorKind::Flush(_),
                    ..
                }),
            ) => true,
            Self::Stream(error) => error.is_transport_failure(),
            _ => false,
        }
    }

    /// Attach a known supply failure without replacing an independent error.
    ///
    /// A queued violation can explain a selected transport error. Otherwise,
    /// prefer a report naming the stream that needed the failed supply; use
    /// the direction alone when no such report was observed.
    pub(super) fn attribute(
        self,
        errors: &mut streams::FirstStreamError,
        remote: codec::Speaker,
    ) -> Self {
        if !self.is_transport_failure() {
            return self;
        }
        let queued = match errors.take_report() {
            Some(error) if !error.is_transport_failure() => return Self::Stream(error),
            queued => queued,
        };
        let (origin, source) = match (self, queued) {
            (Self::Stream(streams::StreamError::SupplyClosed { origin, source }), _)
            | (_, Some(streams::StreamError::SupplyClosed { origin, source })) => (origin, source),
            (error, _) => {
                return match errors.take_supply_failure() {
                    Some(source) => Self::Stream(streams::StreamError::SupplyClosed {
                        origin: codec::Origin::direction(remote),
                        source: Some(source),
                    }),
                    None => error,
                };
            }
        };
        Self::Stream(streams::StreamError::SupplyClosed {
            origin,
            source: source.or_else(|| errors.take_supply_failure()),
        })
    }
}
