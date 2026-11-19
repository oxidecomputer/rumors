//! Report the outcome of joining a gossip network.

use std::fmt;

use crate::Error;
use crate::Peer;

use super::builder::PeerBuilder;
use super::connection::Connection;
use crate::bookmark::{MaybeBookmark, NoBookmark};

/// The outcome of [`Peer::join`] and [`PeerBuilder::join`].
#[must_use = "a `Join` contains a peer or a builder needed for retry"]
pub enum Join<T: Send + Sync + 'static, R, W, B: MaybeBookmark = NoBookmark> {
    /// The peer joined the network, and any selected bookmark was attached and
    /// persisted.
    Connected {
        /// The newly joined peer, carrying the selected configuration and
        /// storage.
        peer: Peer<T, B>,
        /// Whether the connection can be reused to carry another session.
        connection: Connection<R, W>,
    },
    /// The peer joined the network, but attaching the selected bookmark failed.
    ///
    /// The returned peer is connected, but unbookmarked; retry its bookmark
    /// attachment.
    Unbookmarked {
        /// The joined peer, paired with the bookmark failure.
        unbookmarked: B::Unbookmarked<T>,
        /// Whether the connection can be reused to carry another session.
        connection: Connection<R, W>,
    },
    /// The selected bookmark could not be read or decoded, so the network was
    /// not joined and nothing was sent on the connection.
    BookmarkFailed {
        /// The complete builder, ready to retry joining.
        builder: PeerBuilder<T, B>,
        /// The failure to read or decode the stored bookmark.
        error: B::BookmarkFailed,
        /// The connection's read half, unused.
        read: R,
        /// The connection's write half, unused.
        write: W,
    },
    /// Both endpoints were attempting to join each other, so neither could
    /// provide a gossip network to the other.
    Bounced {
        /// The complete builder, ready to retry joining (probably from a
        /// different peer).
        builder: PeerBuilder<T, B>,
        /// The connection's read half, which could be used for another session.
        read: R,
        /// The connection's write half, which could be used for another
        /// session.
        write: W,
    },
    /// The session failed.
    ///
    /// Discard the connection and try again.
    Failed {
        /// The error which failed the session.
        error: Error,
        /// The complete builder, ready to retry joining on a fresh connection.
        builder: PeerBuilder<T, B>,
    },
}

/// Format the outcome of attempting to join a network without requiring
/// debuggable message, bookmark, or transport types.
impl<T: Send + Sync + 'static, R, W, B: MaybeBookmark> fmt::Debug for Join<T, R, W, B> {
    /// Formats the outcome and the peer/state it carries.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connected { peer, connection } => f
                .debug_struct("Connected")
                .field("peer", peer)
                .field("connection", connection)
                .finish(),
            Self::BookmarkFailed { builder, error, .. } => f
                .debug_struct("BookmarkFailed")
                .field("builder", builder)
                .field("error", error)
                .finish_non_exhaustive(),
            Self::Bounced { builder, .. } => f
                .debug_struct("Bounced")
                .field("builder", builder)
                .finish_non_exhaustive(),
            Self::Unbookmarked {
                unbookmarked,
                connection,
            } => f
                .debug_struct("Unbookmarked")
                .field("unbookmarked", unbookmarked)
                .field("connection", connection)
                .finish(),
            Self::Failed { error, builder } => f
                .debug_struct("Failed")
                .field("error", error)
                .field("builder", builder)
                .finish(),
        }
    }
}
