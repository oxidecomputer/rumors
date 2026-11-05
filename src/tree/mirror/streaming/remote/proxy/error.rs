//! Failures surfaced by the remote protocol participant.

use crate::tree::mirror::streaming::remote::{adapter, codec, pipe};

/// A failure while proxying one remote participant.
///
/// Transport failures surface through the pipe ([`Pipe`](Self::Pipe)) or the
/// greeting exchange. Backend failures and violations of the protocol or
/// wire format retain their own identity.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error<E> {
    /// The pipe failed, or the peer broke its framing, during
    /// reconciliation.
    #[error(transparent)]
    Pipe(#[from] pipe::PipeError),
    /// Reading the peer's greeting item failed.
    #[error("failed to read the peer's greeting")]
    GreetingRead(#[source] std::io::Error),
    /// The peer's greeting arrived but is not canonical rumors CBOR.
    #[error("failed to decode the peer's greeting")]
    GreetingDecode(#[source] codec::GreetingError),
    /// Sending the local greeting failed before reconciliation began.
    #[error("{operation} failed during the greeting: {source}")]
    GreetingWrite {
        /// Whether writing or flushing failed.
        operation: crate::error::TransportOperation,
        /// The transport's original error.
        #[source]
        source: std::io::Error,
    },
    /// The peer's greeting listing violated canonical ascending radix order.
    #[error("peer greeting carried a non-canonical root-fan listing")]
    GreetingListing(#[source] codec::QueryOrderError),
    /// The peer's configured payload depth limit differs from ours.
    ///
    /// Detected symmetrically, after the greetings and before anything
    /// else, so a mixed fleet is caught even on a converged session.
    #[error("peer's payload depth limit ({remote} steps) differs from ours ({local} steps)")]
    PayloadDepthMismatch {
        /// This side's configured limit.
        local: u64,
        /// The limit the peer's greeting declared.
        remote: u64,
    },
    /// The local opening reply could not be encoded.
    #[error("local opening reply is invalid")]
    OpeningEncode(#[source] adapter::OpeningError),
    /// A normal local reply could not be converted to wire frames.
    #[error(transparent)]
    Encode(#[from] adapter::EncodeError<E>),
    /// Normal remote wire frames could not be reconstructed as a reply.
    #[error(transparent)]
    Decode(#[from] adapter::DecodeError<E>),
    /// The local opening stream omitted its opening question.
    #[error("opening stream ended before its opening question")]
    MissingOpening,
    /// The local opening stream contained more than its opening question.
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
    /// The local protocol asked another question in its terminal reply.
    #[error("local terminal reply contained another query")]
    TerminalQuery,
    /// A decoded reply found its level's parking full: a local invariant
    /// failure.
    ///
    /// Parking holds every reply the local side can have invited at this
    /// height, and a reply is decoded only against the record of the local
    /// question it answers, so no peer can fill it; a local premise of that
    /// bound failed.
    #[error(
        "parking at height {height} was full at its capacity of {capacity}: a local premise \
         of the in-flight bound failed"
    )]
    ParkingOverflow {
        /// The height of the parked replies.
        height: usize,
        /// The derived capacity of the level's parking.
        capacity: usize,
    },
    /// A taken reply's questions could not be derived again, although its
    /// decoding validated the same positions: a local invariant failure.
    #[error("a taken reply's questions could not be derived again")]
    TakeDerivation(#[source] adapter::ScopeError),
}
