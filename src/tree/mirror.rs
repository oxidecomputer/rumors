//! Mirror-sync between two replicas of the typed tree.
//!
//! [`streaming`] is the default protocol. `alternating` serves as streaming's
//! behavioral oracle in this crate's tests, and remains selectable on the
//! wire behind the `protocol-v1` cargo feature: its state machines are a
//! large monomorphization surface, so binaries that never speak V1 should
//! not spend compile time on it.

#[cfg(any(test, feature = "protocol-v1"))]
pub(crate) mod alternating;
pub mod streaming;

pub(crate) mod framing;
pub(crate) mod handshake;
pub(crate) mod party;

/// An error which can occur during mirroring: either a client error or a server one.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error<C, S> {
    /// The protocol participant supplied in the client position failed.
    #[error("mirror client failed")]
    Client(#[source] C),
    /// The protocol participant supplied in the server position failed.
    #[error("mirror server failed")]
    Server(#[source] S),
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
