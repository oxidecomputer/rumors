/// A session-fatal failure: a backend error or a counterparty [`Violation`].
#[derive(Debug, thiserror::Error)]
pub enum Error<E> {
    #[error(transparent)]
    Backend(#[from] E),
    #[error(transparent)]
    Violation(Violation),
}

/// The ways a counterparty can misbehave: exactly the semantic faults
/// only this side can detect, because they depend on what we hold (our
/// questions, our tree, and the greeting the peer declared to us).
///
/// Non-exhaustive: enforcement of further session invariants adds
/// variants without breaking downstream matches.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Violation {
    /// A reply arrived with no query outstanding.
    #[error("reply received for unknown query")]
    UnaskedReply,
    /// The reply stream ended while questions were outstanding.
    #[error("no reply to outstanding query")]
    UnansweredQuery,
    /// The reply ended before reacting to every listed child.
    #[error("reply failed to cover every listed radix")]
    UnfinishedReply,
    /// A positional `Match` after every held child has been answered.
    #[error("reply attempted to match unknown child")]
    UnexpectedMatch,
    /// A positional `Query` after every held child has been answered.
    #[error("reply attempted to query unknown child")]
    UnexpectedQuery,
    /// A `Supply` whose radix lands on an already-held child.
    #[error("reply attempted to supply a child that is already known")]
    UnexpectedSupply,
    /// A `Supply` whose radix violates the implicit ordering of children.
    #[error("reply attempted to supply a child out of order")]
    InvalidSupply,
    /// A supplied subtree carrying a version outside the sender's declared
    /// version.
    ///
    /// An honest replica transmits only versions causally contained in
    /// its greeting's declared version, so an escaped version marks a
    /// nonconforming sender. Rejected at ingestion so the escape cannot
    /// outlive the session: absorbed, the leaf would sit above every
    /// replica's session ceiling (the version ceiling the greeting
    /// fixed), beyond the reach of redaction and the deletion filter
    /// alike.
    #[error("reply supplied a subtree with a version outside the sender's declared version")]
    UncontainedSupply,
}
