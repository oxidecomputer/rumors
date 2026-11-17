//! What could possibly go wrong?
//!
//! Failures during gossip return [`Error`]. Choose how to respond based on the
//! cause of the error:
//!
//! - [`Transport`](Error::Transport): reconnect to the peer over a fresh
//!   connection (or don't!).
//! - [`Mismatch`](Error::Mismatch): examine the mismatch to deploy a compatible
//!   application, join the intended network, or align configuration.
//! - [`Bookmark`](Error::Bookmark): repair or replace the underlying bookmark
//!   storage.
//! - [`GossipTimeout`](Error::GossipTimeout): the caller-specified gossip
//!   timeout expired. Reconnect to the peer, adjusting the deadline if needed.
//! - [`Protocol`](Error::Protocol): [please report a bug in this
//!   crate](https://github.com/oxidecomputer/rumors/issues/new) by providing
//!   the error's context and diagnostic source.

use std::fmt;

use crate::{
    Count, Network, Protocol,
    bookmark::{Bookmark, NoBookmark},
    observe::SessionErrorKind,
    tree::mirror::preamble,
};

mod session;
pub use crate::{
    bookmark::{BookmarkIo, FormatError},
    message::EncodeError,
};
pub use session::{
    Context, DataStream, Phase, ProtocolViolation, TransportError, TransportOperation,
};

/// An incompatibility between peers which prevents them from reconciling.
///
/// Each variant carries the values needed to choose the right recovery action.
/// This category of error cannot be resolved by retrying without making changes
/// to the [`Peer`](crate::Peer) in some way. After resolving the underlying
/// issue, reconnect over a fresh connection.
///
/// When such an error occurs, no reconciliation has yet taken place.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Mismatch {
    /// The remote peer uses an incompatible wire protocol.
    ///
    /// Deploy compatible
    /// `rumors` versions at both ends.
    #[error(
        "peer speaks rumors protocol version {remote_version}, while this build speaks {local_protocol:?}"
    )]
    #[non_exhaustive]
    Protocol {
        /// The protocol version spoken by this build.
        local_protocol: Protocol,
        /// The protocol version advertised by the remote peer.
        remote_version: u64,
    },

    /// The remote peer belongs to a different gossip network.
    ///
    /// Optionally, use these event bounds to apply the dominance rule described
    /// by [`Peer`](crate::Peer)'s "Bootstrapping without consensus" section.
    #[error("peer belongs to network {remote_network:?}, ours is {local_network:?}")]
    #[non_exhaustive]
    Network {
        /// The local peer's network identifier.
        local_network: Network,
        /// A lower bound on the total number of events in the local peer's
        /// gossip network.
        local_min_events: Count,
        /// The remote peer's network identifier.
        remote_network: Network,
        /// A lower bound on the total number of events in the remote peer's
        /// gossip network.
        remote_min_events: Count,
    },

    /// The peers have different configured payload depth limits, and are thus
    /// incompatible.
    ///
    /// Align [`Peer::message_depth_limit`](crate::Peer::message_depth_limit)
    /// across all peers in the gossip network, then reconnect.
    #[error("peer's payload depth limit ({remote} steps) differs from ours ({local} steps)")]
    #[non_exhaustive]
    MessageDepth {
        /// The local peer's configured payload depth limit.
        local: u64,
        /// The remote peer's advertised payload depth limit.
        remote: u64,
    },
}

/// A failure returned during reconciliation.
#[non_exhaustive]
#[derive(thiserror::Error)]
pub enum Error<B: Bookmark = NoBookmark> {
    /// A transport operation failed or the peer closed the connection before it
    /// completed.
    ///
    /// Reconnect to the peer using a fresh connection.
    #[error(transparent)]
    Transport(TransportError),

    /// A protocol invariant failed to be upheld by the implementation.
    ///
    /// [Please report its context and diagnostic source as a
    /// bug.](https://github.com/oxidecomputer/rumors/issues/new)
    #[error(transparent)]
    Protocol(ProtocolViolation),

    /// The two peers have incompatible protocols, networks, or settings.
    ///
    /// Resolve the mismatch by changing parameters before retrying on a fresh
    /// connection.
    #[error(transparent)]
    Mismatch(Mismatch),

    /// The application's gossip timeout interrupted an active session.
    ///
    /// Discard the connection and reconnect to the peer.
    #[error("gossip timeout exceeded; discard the connection and reconnect")]
    GossipTimeout,

    /// The application's [`Bookmark`] failed to load, persist, or parse.
    ///
    /// In case of non-transient errors, repair or replace the underlying
    /// storage before retrying.
    #[error(transparent)]
    Bookmark(BookmarkIo<B::Error>),
}

impl<B: Bookmark> Error<B> {
    /// Reduce this error to the category exposed to session observers.
    pub(crate) fn session_error_kind(&self) -> SessionErrorKind {
        match self {
            Self::Transport(_) => SessionErrorKind::Transport,
            Self::Protocol(_) => SessionErrorKind::Protocol,
            Self::Mismatch(_) => SessionErrorKind::Mismatch,
            Self::GossipTimeout => SessionErrorKind::GossipTimeout,
            Self::Bookmark(_) => SessionErrorKind::Bookmark,
        }
    }
}

/// Format failures without requiring the bookmark handle itself to be
/// debuggable.
impl<B: Bookmark> fmt::Debug for Error<B> {
    /// Formats the error and its actionable diagnostic fields.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(error) => f.debug_tuple("Transport").field(error).finish(),
            Self::Protocol(error) => f.debug_tuple("Protocol").field(error).finish(),
            Self::Mismatch(error) => f.debug_tuple("Mismatch").field(error).finish(),
            Self::GossipTimeout => f.write_str("GossipTimeout"),
            Self::Bookmark(error) => f.debug_tuple("Bookmark").field(error).finish(),
        }
    }
}

/// Map preamble failures to public session error categories.
impl From<preamble::Error> for Error {
    /// Classify preamble failures without exposing its wire grammar.
    fn from(error: preamble::Error) -> Self {
        match error {
            preamble::Error::Io { operation, source } => {
                Self::transport(Phase::Preamble, operation, source)
            }
            preamble::Error::VersionMismatch {
                local_protocol,
                remote_version,
            } => Self::Mismatch(Mismatch::Protocol {
                local_protocol,
                remote_version,
            }),
            error @ preamble::Error::Truncated { .. } => Self::transport(
                Phase::Preamble,
                TransportOperation::Read,
                std::io::Error::new(std::io::ErrorKind::UnexpectedEof, error),
            ),
            error => Self::violation(Phase::Preamble, error),
        }
    }
}

/// Adapt errors from sessions that have no bookmark-specific storage failure.
impl Error {
    /// Retag an error under a bookmark type without losing its cause.
    pub(crate) fn widen<B: Bookmark>(self) -> Error<B> {
        match self {
            Self::Transport(error) => Error::Transport(error),
            Self::Protocol(error) => Error::Protocol(error),
            Self::Mismatch(error) => Error::Mismatch(error),
            Self::GossipTimeout => Error::GossipTimeout,
            Self::Bookmark(error) => match error {
                BookmarkIo::Io(never) => match never {},
                BookmarkIo::Format(error) => Error::Bookmark(BookmarkIo::Format(error)),
            },
        }
    }
}
