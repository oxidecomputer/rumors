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
