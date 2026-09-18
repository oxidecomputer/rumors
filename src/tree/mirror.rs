//! Mirror-sync between two replicas of the typed tree.
//!
//! [`streaming`] is the wire protocol; its behavioral oracle in this
//! crate's tests is the in-memory merge (`Tree::join`). The two walks have
//! separate deletion filters whose agreement is checked differentially.
//!
//! # Malformed input
//!
//! Once an ingress can determine that received bytes violate the protocol, it
//! returns a typed session error rather than panicking or accepting them. A
//! non-conforming peer can instead leave an item or exchange incomplete—for
//! example, by announcing a body and withholding some of it—and leave the
//! session waiting; applications impose deadlines on that wait. Under the
//! trusted-counterparty model (see the crate docs), validation detects
//! implementation defects rather than enforcing a security boundary. In
//! particular, frame lengths are trusted for allocation after the preamble
//! validates the counterparty (see [`framing`]). Every ingress has
//! malformed-input tests beside its parser.

pub mod streaming;

#[cfg(test)]
mod tests;

pub(crate) mod cbor;
pub(crate) mod framing;
pub(crate) mod party;
pub(crate) mod preamble;

/// Whether `bound` is causally contained in `declared`: the version-
/// containment predicate the protocol enforces on supplied subtrees at
/// ingestion.
///
/// Named because version order is partial and the callers branch on the
/// *negation*: a bound that is incomparable with the declared version is
/// just as uncontained as one strictly above it, which a bare `!(a <= b)`
/// reads as easily getting wrong.
pub(crate) fn contained(bound: &crate::Version, declared: &crate::Version) -> bool {
    bound <= declared
}

/// An error during mirroring, from either the client or the server position.
#[derive(Debug, thiserror::Error)]
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
    /// which side is the local client; when the role election swaps the
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
/// cross (the same flip the drivers apply when the role election swaps the
/// roles).
impl<C, S> From<C> for Error<C, S> {
    fn from(client: C) -> Self {
        Error::Client(client)
    }
}
