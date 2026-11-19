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
    Count, Network, Participant, Protocol,
    bookmark::{MaybeBookmark, NoBookmark},
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
    /// Optionally, use the [`NetworkMismatch`]'s event bounds to apply the
    /// dominance rule described
    /// by [`Peer`](crate::Peer)'s "Bootstrapping a network without consensus"
    /// section.
    #[error(
        "peer belongs to network {:?}, ours is {:?}",
        .0.remote.network,
        .0.local.network
    )]
    Network(NetworkMismatch),

    /// The peers have different configured message depth limits, and are thus
    /// incompatible.
    ///
    /// Align [`Peer::message_depth_limit`](crate::Peer::message_depth_limit)
    /// across all peers in the gossip network, then reconnect.
    #[error("peer's message depth limit ({remote} steps) differs from ours ({local} steps)")]
    #[non_exhaustive]
    MessageDepth {
        /// The local peer's configured message depth limit.
        local: u64,
        /// The remote peer's advertised message depth limit.
        remote: u64,
    },
}

/// The two differing networks of a gossip session which failed because these
/// networks differed.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct NetworkMismatch {
    /// The local peer's network.
    pub local: NetworkPrecedence,
    /// The remote peer's network.
    pub remote: NetworkPrecedence,
}

/// A ranking to help determine a network's precedence when two networks
/// collide.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NetworkPrecedence {
    /// A lower bound on the total number of messages ever sent in the network.
    min_events: Count,
    /// The network's identifier.
    network: Network,
}

/// Policy for whether a peer may yield to another network when doing so could
/// discard data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DataLoss {
    /// Yield only without losing data.
    #[default]
    Forbid,
    /// Yield even if that loses data.
    Allow,
}

/// Examine a network's precedence.
impl NetworkPrecedence {
    /// Pair a network with its event bound.
    pub(crate) fn new(network: Network, min_events: Count) -> Self {
        Self {
            min_events,
            network,
        }
    }

    /// A lower bound on the total number of messages ever sent in the network.
    pub fn min_events(&self) -> &Count {
        &self.min_events
    }

    /// The network's identifier.
    pub fn network(&self) -> Network {
        self.network
    }
}

/// Decide which peer should yield to the other, under the specified data loss
/// policy.
impl NetworkMismatch {
    /// Which peer should yield to the other's network, if either.
    pub fn yielding_peer(&self, data_loss: DataLoss) -> Option<Participant> {
        let (lesser, yielding) = if self.local < self.remote {
            (&self.local, Participant::Local)
        } else {
            (&self.remote, Participant::Remote)
        };
        match data_loss {
            DataLoss::Allow => Some(yielding),
            DataLoss::Forbid => (lesser.min_events == Count::ZERO).then_some(yielding),
        }
    }
}

/// A failure returned during reconciliation.
#[non_exhaustive]
#[derive(thiserror::Error)]
pub enum Error<B: MaybeBookmark = NoBookmark> {
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

    /// The application's [`Bookmark`](crate::Bookmark) failed to load, persist,
    /// or parse.
    ///
    /// In case of non-transient errors, repair or replace the underlying
    /// storage before retrying.
    #[error(transparent)]
    Bookmark(B::Failure),
}

impl<B: MaybeBookmark> Error<B> {
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
impl<B: MaybeBookmark> fmt::Debug for Error<B> {
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
    pub(crate) fn widen<B: MaybeBookmark>(self) -> Error<B> {
        match self {
            Self::Transport(error) => Error::Transport(error),
            Self::Protocol(error) => Error::Protocol(error),
            Self::Mismatch(error) => Error::Mismatch(error),
            Self::GossipTimeout => Error::GossipTimeout,
            Self::Bookmark(never) => match never {},
        }
    }
}

#[cfg(test)]
mod tests;
