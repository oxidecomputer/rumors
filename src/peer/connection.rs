//! Report whether a joining or departing session left its connection reusable.

use std::fmt;

use crate::Error;
use crate::bookmark::{Bookmark, NoBookmark};

/// Whether a connection can carry another session, once we've finished using
/// it.
///
/// A session that completes successfully leaves its connection exactly at the
/// boundary between sessions, and both sides are aware of this. For this
/// reason, it is safe to reuse such a connection. By contrast, a session that
/// fails leaves the connection at an unknown position, so neither side can
/// reuse it.
///
/// To keep the connection for another session, take its halves with
/// [`into_halves`](Self::into_halves); if the connection is reusable, you'll
/// get it back, and if it's not, `?` passes the error on.
#[must_use = "a reusable connection returns its halves; a lost one carries the session's error"]
pub enum Connection<R, W, B: Bookmark = NoBookmark> {
    /// The session completed, and both sides are aware of this.
    ///
    /// The connection halves rest exactly at the boundary between sessions,
    /// ready for another session.
    Reusable {
        /// The read half of the connection.
        read: R,
        /// The write half of the connection.
        write: W,
    },
    /// The session failed part-way through, and cannot be safely
    /// recovered.
    ///
    /// Halves passed by value were dropped with the session; halves passed as
    /// `&mut` are still technically usable, but must not carry another session.
    /// Discard them, and reconnect for further sessions.
    Lost {
        /// The failure which caused the connection to break.
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

/// Take back a reusable connection's halves, throwing an error if this can't be
/// done.
impl<R, W, B: Bookmark> Connection<R, W, B> {
    /// Return a reusable connection's halves, or the error that broke the
    /// connection.
    ///
    /// # Errors
    ///
    /// Returns the underlying error if the connection is [`Lost`](Self::Lost).
    ///
    /// # Examples
    ///
    /// A caller who wants the connection for another session can take its
    /// halves with `?`, and propagate the error:
    ///
    /// ```
    /// use rumors::{Connection, Error};
    ///
    /// fn reuse<R, W>(connection: Connection<R, W>) -> Result<(R, W), Error> {
    ///     let (read, write) = connection.into_halves()?;
    ///     Ok((read, write))
    /// }
    ///
    /// let reusable: Connection<&[u8], Vec<u8>> = Connection::Reusable {
    ///     read: &[],
    ///     write: Vec::new(),
    /// };
    /// assert!(reuse(reusable).is_ok());
    /// ```
    pub fn into_halves(self) -> Result<(R, W), Error<B>> {
        match self {
            Self::Reusable { read, write } => Ok((read, write)),
            Self::Lost { error } => Err(error),
        }
    }
}

/// Format a connection result without requiring debuggable transport types.
impl<R, W, B: Bookmark> fmt::Debug for Connection<R, W, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Reusable { .. } => f.debug_struct("Reusable").finish_non_exhaustive(),
            Self::Lost { error } => f.debug_struct("Lost").field("error", error).finish(),
        }
    }
}
