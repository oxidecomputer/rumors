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
    AcceptError, CodecDecodeError, CodecDecodeErrorKind, CodecEncodeError, CodecEncodeErrorKind,
    DecodeError, DecodeLeafError, DecodeSignalError, EncodeError, EncodeLeafError, FramePart,
    InvalidSignalPlacement, InvalidWireSignal, LengthOverflow, OpeningError, Origin,
    QueryOrderError, RemoteError, ReplyFrameError, ScopeError, SendError, Speaker, Stream,
    StreamClass, StreamError,
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
        /// A lower bound on events recorded in the local universe, as this
        /// side declared it in the session's handshake.
        ///
        /// Together with `remote_min_events` this lets both sides of a
        /// mismatch apply one deterministic dominance rule from the error
        /// alone (see [`Peer`](crate::Peer)'s "Bootstrapping without
        /// consensus").
        local_min_events: u64,
    },

    /// A retiring peer offered an identity overlapping one already held here.
    #[error("retiring peer's party overlaps ours")]
    PartyOverlap,

    /// The session's closing epilogue failed *after* the local replica
    /// committed.
    ///
    /// Under [`Protocol::V2`] each side ends a session by
    /// exchanging a one-byte completion marker on the control stream, so `Ok`
    /// certifies the peer completed and committed too. This error is the
    /// residue that exchange cannot eliminate (the two-generals problem): the
    /// local replica **is** committed — every message and identity the session
    /// moved is applied here — and only the confirmation of the *peer's*
    /// completion failed. The source is the I/O failure that cut the exchange
    /// short, or an invalid-data error if the peer wrote something other than
    /// the marker where it belonged.
    #[error("session epilogue failed after local commit: {0}")]
    Epilogue(#[source] std::io::Error),

    /// A session was started on a link whose previous session was
    /// interrupted.
    ///
    /// A session that fails or is cancelled leaves the link's control
    /// stream mid-frame, where a next session would misread its leftover
    /// bytes as a preamble. The link records the interruption
    /// ([`SessionState`](crate::link::SessionState)) and every subsequent
    /// session fails here, before any wire traffic. Discard the link and
    /// reconnect; the replica itself is unharmed.
    #[error(
        "link is poisoned: an earlier session on it was interrupted before completing; discard the link and reconnect"
    )]
    LinkPoisoned,

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
                local_min_events,
            } => Error::NetworkMismatch {
                remote_network,
                remote_min_events,
                local_min_events,
            },
            Error::PartyOverlap => Error::PartyOverlap,
            Error::Epilogue(error) => Error::Epilogue(error),
            Error::LinkPoisoned => Error::LinkPoisoned,
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
