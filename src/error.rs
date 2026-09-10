//! Failures from sessions and durable identity handling.
//!
//! Session methods return [`Error`]. Choose a response from its cause:
//!
//! - [`Transport`](Error::Transport): reconnect over a fresh link.
//! - [`Mismatch`](Error::Mismatch): use the mismatch kind and its values to
//!   resolve incompatible protocols, networks, or settings.
//! - [`Bookmark`](Error::Bookmark): repair or replace the bookmark storage.
//! - [`Protocol`](Error::Protocol): report a bug with its context and
//!   diagnostic source. Individual protocol checks stay private.
//!
//! A failed or cancelled session leaves its link unusable. Discard it before
//! retrying; reusing it returns [`Error::LinkPoisoned`].
//!
//! Failure does not guarantee unchanged state. On an existing replica, a
//! failure in [`Phase::Completion`] leaves local work committed but the peer's
//! commit unconfirmed; bootstrap instead discards the received identity.
//! [`Error::Bookmark`] can also leave identity changes live but not persisted.

use std::convert::Infallible;

use crate::{
    Network, PayloadDepthLimit, Protocol, Ticks,
    bookmark::{BookmarkError, BookmarkIo, NoBookmark},
    tree::mirror::{
        self, handshake,
        streaming::{materialized, remote},
    },
};

mod session;
pub use crate::message::EncodeError;
pub use session::{
    Context, DataStream, Phase, ProtocolViolation, TransportError, TransportOperation,
};

/// The production mirror's internal failure type; neither backend can fail.
pub(crate) type MirrorError =
    mirror::Error<materialized::Error<Infallible>, remote::Error<Infallible>>;

/// An incompatibility that prevents the peers from reconciling.
///
/// Each variant carries the values needed to choose a recovery action.
/// Retrying unchanged peers cannot resolve a mismatch; after resolving it,
/// reconnect over a fresh link.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Mismatch {
    /// The peer speaks a different wire version. Select the same protocol at
    /// both ends; if it is already the same, align the crate versions.
    #[error("peer speaks rumors protocol version {remote_version}, we selected {local_protocol:?}")]
    #[non_exhaustive]
    Protocol {
        /// The protocol selected locally.
        local_protocol: Protocol,
        /// The wire version advertised by the peer.
        remote_version: u64,
    },

    /// Both peers were gossiping but belong to different causal universes.
    /// Use these event bounds to apply the dominance rule described by
    /// [`Peer`](crate::Peer)'s "Bootstrapping without consensus" section.
    #[error("peer belongs to network {remote_network:?}, ours is {local_network:?}")]
    #[non_exhaustive]
    Network {
        /// The network identifier this side advertised.
        local_network: Network,
        /// The event bound this side advertised for its own universe.
        local_min_events: Ticks,
        /// The network identifier advertised by the peer.
        remote_network: Network,
        /// A lower bound on events recorded in the remote universe.
        remote_min_events: Ticks,
    },

    /// The peers have different payload depth limits. Align
    /// [`Peer::payload_depth_limit`](crate::Peer::payload_depth_limit)
    /// across the fleet, then reconnect. No reconciliation has taken place.
    #[error("peer's payload depth limit ({remote}) differs from ours ({local})")]
    #[non_exhaustive]
    PayloadDepth {
        /// This side's configured limit.
        local: PayloadDepthLimit,
        /// The limit the peer advertised.
        remote: PayloadDepthLimit,
    },
}

/// A failure returned by bootstrap, gossip, or retirement.
///
/// `B` retains the application's bookmark error type. Session diagnostics are
/// independent of bookmark storage. For failures with a [`Context`], inspect
/// its phase as well as its cause: completion can fail after local commit.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error<B: BookmarkError = NoBookmark> {
    /// A transport operation failed or the peer closed before it completed.
    /// Reconnect using a fresh link.
    #[error(transparent)]
    Transport(TransportError),

    /// A protocol or implementation invariant failed. Report its context and
    /// diagnostic source as a bug; ordinary peer departure is a transport failure.
    #[error(transparent)]
    Protocol(ProtocolViolation),

    /// The peers have incompatible protocols, networks, or settings.
    /// Resolve the mismatch before retrying on a fresh link.
    #[error(transparent)]
    Mismatch(Mismatch),

    /// An earlier session failed or was cancelled on this link. Discard it and
    /// reconnect; its stream positions no longer mark a session boundary.
    #[error("link is poisoned by an interrupted session; discard it and reconnect")]
    LinkPoisoned,

    /// The application's bookmark failed to load, persist, or decode.
    /// Repair or replace the storage before retrying.
    ///
    /// Usually this happens before any traffic. Absorbing a retirement instead
    /// commits content and identity before persisting, so that absorption is
    /// live but not yet crash-safe on this error. A later successful gossip
    /// persists it. Identity reclaimed during an unsuccessful update also
    /// remains live in memory until a successful persist records it.
    #[error(transparent)]
    Bookmark(BookmarkIo<B::Error>),
}

impl From<handshake::Error> for Error {
    /// Classify preamble failures without exposing its wire grammar.
    fn from(error: handshake::Error) -> Self {
        match error {
            handshake::Error::Io { operation, source } => {
                Self::transport(Phase::Preamble, operation, source)
            }
            handshake::Error::VersionMismatch {
                local_protocol,
                remote_version,
            } => Self::Mismatch(Mismatch::Protocol {
                local_protocol,
                remote_version,
            }),
            error @ handshake::Error::Truncated { .. } => Self::transport(
                Phase::Preamble,
                TransportOperation::Read,
                std::io::Error::new(std::io::ErrorKind::UnexpectedEof, error),
            ),
            error => Self::violation(Phase::Preamble, error),
        }
    }
}

impl Error {
    /// Retag an error under a bookmark type without losing its cause.
    pub(crate) fn widen<B: BookmarkError>(self) -> Error<B> {
        match self {
            Self::Transport(error) => Error::Transport(error),
            Self::Protocol(error) => Error::Protocol(error),
            Self::Mismatch(error) => Error::Mismatch(error),
            Self::LinkPoisoned => Error::LinkPoisoned,
            Self::Bookmark(error) => match error {
                BookmarkIo::Io(never) => match never {},
                BookmarkIo::Format(error) => Error::Bookmark(BookmarkIo::Format(error)),
            },
        }
    }
}
