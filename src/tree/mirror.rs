//! Mirror-sync between two replicas of the typed tree: the [`alternating`]
//! and [`streaming`] protocol implementations, and the vocabulary they share.

pub mod alternating;
pub mod streaming;

/// An error which can occur during mirroring: either a client error or a server one.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error<C, S> {
    Client(C),
    Server(S),
}

impl<C, S> Error<C, S> {
    /// The same fault, seen from the counterparty's frame.
    ///
    /// The drivers run the descent in initiator/responder order regardless of
    /// which side is the local client; when the version tiebreak swaps the
    /// roles, the error's sides swap back with it.
    pub(crate) fn flip(self) -> Error<S, C> {
        match self {
            Error::Client(client) => Error::Server(client),
            Error::Server(server) => Error::Client(server),
        }
    }
}

/// A first-position error lifts into the sum.
///
/// Only the first position can have this impl: its second-position mirror
/// would overlap with it when `C = S`, and coherence permits one. This
/// asymmetry shapes how the streaming driver uses the sum — each party runs
/// its session at the *frame-relative* instantiation with its own error
/// first, so `?` lifts either party's backend errors through this one impl,
/// and the party boundary [flips](Error::flip) errors between frames as they
/// cross (the same flip the drivers apply when the version tiebreak swaps
/// the roles).
impl<C, S> From<C> for Error<C, S> {
    fn from(client: C) -> Self {
        Error::Client(client)
    }
}
