//! Report whether a joining or departing session left its connection reusable.

use std::fmt;

use crate::Error;
use crate::bookmark::{Bookmark, NoBookmark};

/// Whether a connection can carry another session, once this side has
/// joined or left the network over it.
///
/// A session that completes leaves its connection exactly at the session
/// boundary, ready for another session; a session that fails leaves it at
/// an unknown position. Bootstrap and retirement decide whether this side
/// joined or left before that is known, so their outcomes carry the
/// connection's fate as a separate value: [`Joined`](crate::Joined) beside
/// the new peer, and [`Retire`](crate::Retire) when this peer has left. See
/// [what a session promises](crate#what-a-session-promises).
///
/// `B` is the bookmark type of the session's [`Error`]. Retirement can fail
/// in its peer's bookmark storage; bootstrap's session error is always
/// `Error<NoBookmark>`.
#[must_use = "a reusable connection returns its halves; a lost one carries the session's error"]
pub enum Connection<R, W, B: Bookmark = NoBookmark> {
    /// The session completed, and both sides confirmed it. The halves rest
    /// exactly at the session boundary, ready for another session.
    Reusable {
        /// The connection's read half.
        read: R,
        /// The connection's write half.
        write: W,
    },
    /// The session failed after this side had joined or left the network,
    /// for example by a cut connection or an expired session deadline.
    ///
    /// The connection is at an unknown position: discard it, and reconnect
    /// for further sessions. Halves passed by value were dropped with the
    /// session; halves passed as `&mut` are still the caller's, and must not
    /// carry another session.
    Lost {
        /// The session failure.
        error: Error<B>,
    },
}

/// Settle a connection's fate from its session's result.
impl<R, W, B: Bookmark> Connection<R, W, B> {
    /// Return the halves if the session completed, or else its error.
    pub(crate) fn from_session(completed: Result<(), Error<B>>, read: R, write: W) -> Self {
        match completed {
            Ok(()) => Self::Reusable { read, write },
            Err(error) => Self::Lost { error },
        }
    }
}

/// Format a connection's fate without requiring debuggable transport types.
impl<R, W, B: Bookmark> fmt::Debug for Connection<R, W, B> {
    /// Formats the variant and, for a lost connection, its error.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Reusable { .. } => f.debug_struct("Reusable").finish_non_exhaustive(),
            Self::Lost { error } => f.debug_struct("Lost").field("error", error).finish(),
        }
    }
}
