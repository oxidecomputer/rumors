//! Public failures from transport sessions and durable identity handling.

use std::convert::Infallible;

use crate::{
    Network, Protocol,
    bookmark::{BookmarkError, BookmarkIo, NoBookmark},
    tree::mirror::{self, handshake},
};

pub use crate::tree::mirror::streaming::materialized::{
    Error as MaterializedError, Violation as MaterializedViolation,
};
pub use crate::tree::mirror::streaming::remote::{
    CodecDecodeError, CodecDecodeErrorKind, CodecEncodeError, CodecEncodeErrorKind, DecodeError,
    DecodeLeafError, DecodeSignalError, DemuxError, EncodeError, EncodeLeafError, FramePart,
    InvalidSignalPlacement, InvalidWireSignal, LengthOverflow, MuxError, OpeningError, Origin,
    QueryOrderError, RemoteError, ReplyFrameError, ScopeError, SendError, Speaker, Stream,
    StreamClass,
};

/// The concrete production mirror failure, retaining its detecting side.
pub type MirrorError = mirror::Error<MaterializedError<Infallible>, RemoteError<Infallible>>;

/// An error returned by bootstrap, gossip, or retirement.
///
/// Generic over the bookmark `B` in play only to retain its backend error in
/// [`Bookmark`](Self::Bookmark). Every wire and protocol variant is otherwise
/// bookmark-independent. The default bookmark type has an uninhabited backend
/// error.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error<B: BookmarkError = NoBookmark> {
    /// An underlying reader/writer error, or a Borsh framing failure outside
    /// the streaming mirror itself.
    #[error(transparent)]
    Io(#[from] borsh::io::Error),

    /// The peer's preamble did not begin with [`PROTOCOL_MAGIC`](crate::PROTOCOL_MAGIC).
    #[error("peer is not a rumors stream (remote magic: {remote_magic:x?})")]
    MagicMismatch { remote_magic: [u8; 6] },

    /// The peer speaks a different wire dialect.
    #[error("peer speaks rumors protocol version {remote_version}, we selected {local_protocol:?}")]
    VersionMismatch {
        local_protocol: Protocol,
        remote_version: u16,
    },

    /// Both peers were gossiping but belong to unrelated causal universes.
    #[error("peer belongs to a different network ({remote_network:?})")]
    NetworkMismatch {
        /// The network identifier advertised by the remote peer.
        remote_network: Network,
        /// A lower bound on events recorded in the remote universe.
        remote_min_events: u64,
    },

    /// A retiring peer offered an identity overlapping one already held here.
    #[error("retiring peer's party overlaps ours")]
    PartyOverlap,

    /// The peer's intent byte had no defined meaning.
    #[error("peer sent an invalid intent byte ({byte:#04x})")]
    IntentInvalid { byte: u8 },

    /// A peer cannot simultaneously receive and donate an identity.
    #[error("peer claimed to bootstrap and retire in the same session")]
    BootstrapRetireConflict,

    /// The application's bookmark failed to load, persist, or decode.
    #[error(transparent)]
    Bookmark(BookmarkIo<B::Error>),

    /// Reconciliation failed in either the materialized participant or its
    /// wire-bound counterparty proxy.
    ///
    /// The nested source retains the detecting side and remains matchable
    /// through backend, adapter, session, codec, and transport errors.
    #[error(transparent)]
    Mirror(#[from] MirrorError),
}

impl From<handshake::Error> for Error<NoBookmark> {
    fn from(error: handshake::Error) -> Self {
        match error {
            handshake::Error::Io(error) => Error::Io(error),
            handshake::Error::MagicMismatch { remote_magic } => {
                Error::MagicMismatch { remote_magic }
            }
            handshake::Error::VersionMismatch {
                local_protocol,
                remote_version,
            } => Error::VersionMismatch {
                local_protocol,
                remote_version,
            },
            handshake::Error::IntentInvalid { byte } => Error::IntentInvalid { byte },
            handshake::Error::BootstrapRetireConflict => Error::BootstrapRetireConflict,
        }
    }
}

impl Error<NoBookmark> {
    /// Re-tag a bookmark-free session error under any bookmark `B`.
    ///
    /// Wire and protocol machinery produces `Error<NoBookmark>`; peer-level
    /// drivers return `Error<B>`. The only bookmark backend error here is
    /// uninhabited, making the conversion total and lossless.
    pub(crate) fn widen<B: BookmarkError>(self) -> Error<B> {
        match self {
            Error::Io(error) => Error::Io(error),
            Error::MagicMismatch { remote_magic } => Error::MagicMismatch { remote_magic },
            Error::VersionMismatch {
                local_protocol,
                remote_version,
            } => Error::VersionMismatch {
                local_protocol,
                remote_version,
            },
            Error::NetworkMismatch {
                remote_network,
                remote_min_events,
            } => Error::NetworkMismatch {
                remote_network,
                remote_min_events,
            },
            Error::PartyOverlap => Error::PartyOverlap,
            Error::IntentInvalid { byte } => Error::IntentInvalid { byte },
            Error::BootstrapRetireConflict => Error::BootstrapRetireConflict,
            Error::Mirror(error) => Error::Mirror(error),
            Error::Bookmark(error) => match error {
                BookmarkIo::Io(never) => match never {},
                BookmarkIo::Format(error) => Error::Bookmark(BookmarkIo::Format(error)),
            },
        }
    }
}
