//! Failures surfaced by the remote protocol participant.

use crate::tree::mirror::streaming::remote::{adapter, codec, streams};

/// A protocol or adapter failure while proxying one remote counterparty.
#[derive(Debug, thiserror::Error)]
pub enum Error<E> {
    /// Reading one of the peer's greeting frames failed.
    #[error("failed to read streaming handshake")]
    HandshakeRead(#[source] std::io::Error),
    /// A greeting body was not a canonical causal version or listing.
    #[error("failed to decode streaming handshake")]
    HandshakeDecode(#[source] std::io::Error),
    /// Writing and flushing the local greeting frames failed.
    #[error("failed to write streaming handshake")]
    HandshakeWrite(#[source] std::io::Error),
    /// The peer's greeting listing violated canonical ascending radix order.
    #[error("peer greeting carried a non-canonical root-fan listing")]
    HandshakeListing(#[source] codec::QueryOrderError),
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
