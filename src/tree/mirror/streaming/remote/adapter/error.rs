/// A prefix-free reaction could not be paired with the question it answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ScopeError {
    /// A positional query followed all children named by its question.
    #[error("a query has no remaining child in its question")]
    UnpositionedQuery,
    /// Queries cannot descend below leaf height.
    #[error("a leaf-height reply contains a query")]
    LeafQuery,
}

/// The initiator's distinguished opening reply did not contain its one query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OpeningError {
    /// The opening reply must contain exactly one reaction.
    #[error("the opening reply contains {count} reactions instead of one")]
    ReactionCount { count: usize },
    /// The opening reaction must ask the implicit root question.
    #[error("the opening reply does not contain a query")]
    NotQuery,
    /// The opening wire frame must close its stream on that query.
    #[error("the opening frame is not a stream-ending query")]
    InvalidFrame,
}

/// A protocol reply could not be rendered faithfully as wire frames.
#[derive(Debug, thiserror::Error)]
pub enum EncodeError<E> {
    /// The local backend failed while exploding a supplied node.
    #[error("backend failed while enumerating a supplied node")]
    Backend(#[source] E),
    /// A positional reaction could not be scoped safely.
    #[error(transparent)]
    Scope(#[from] ScopeError),
}

/// Wire frames could not be reconstructed into one scoped protocol reply.
#[derive(Debug, thiserror::Error)]
pub enum DecodeError<E> {
    /// The local backend failed while assembling supplied leaves.
    #[error("backend failed while assembling supplied leaves")]
    Backend(#[source] E),
    /// The frame stream ended before its reply boundary.
    #[error("wire stream ended before the current reply")]
    TruncatedReply,
    /// A bare end followed one or more reaction frames.
    #[error("a nonempty reply uses a bare end frame")]
    BareEndAfterReaction,
    /// A supplied leaf's content-derived path is outside the expected scope.
    #[error("supplied leaf {actual:02x?} is outside reply scope {expected:02x?}")]
    LeafOutsideScope { expected: Vec<u8>, actual: [u8; 32] },
    /// Supplied leaves were not strictly ascending by content-derived path.
    #[error("supplied leaf {current:02x?} does not follow {previous:02x?}")]
    LeafOrder {
        previous: [u8; 32],
        current: [u8; 32],
    },
    /// A later supplied run reused or preceded an earlier run's radix.
    #[error("supplied radix {radix:#04x} does not follow {previous:#04x}")]
    SupplyOrder { previous: u8, radix: u8 },
    /// A positional wire reaction cannot be scoped without another child.
    #[error(transparent)]
    Scope(#[from] ScopeError),
}
