#[derive(Debug, thiserror::Error)]
pub enum Error<E> {
    #[error(transparent)]
    Backend(#[from] E),
    #[error(transparent)]
    Violation(Violation),
}

/// The ways a counterparty can misbehave: exactly the semantic faults
/// only this side can detect, because they depend on our questions and
/// our tree.
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
}
