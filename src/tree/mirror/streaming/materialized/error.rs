//! Failures detected by the materialized reconciliation walk.
//!
//! Backend failures identify storage work that could not complete. Count
//! errors identify a node count the backend reported outside every node's
//! range, a breach of the backend contract such as corrupted storage would
//! cause. Violations identify the protocol invariant that the peer's reply
//! broke; they are diagnostics for finding an implementation defect in one
//! of the peers.

use crate::tree::mirror::streaming::message::CountError;

/// A failure that ends the materialized walk.
#[derive(Debug, thiserror::Error)]
pub enum Error<E> {
    /// The backend could not read or construct a node.
    #[error(transparent)]
    Backend(#[from] E),
    /// The backend reported a count no node can have, for a node the walk
    /// was listing, or listed a different number of children than it
    /// reported.
    #[error(transparent)]
    Count(CountError),
    /// The peer's reply broke a reconciliation invariant.
    #[error(transparent)]
    Violation(Violation),
}

/// A reconciliation invariant broken by the peer's reply.
///
/// These checks depend on local context that the wire decoder does not have:
/// outstanding questions, locally held nodes, and the peer's greeting.
/// A reply must account for held children in radix order. It may confirm a
/// child, ask to reconcile it more deeply, or supply a child absent locally.
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
    /// The reply confirmed a child that the question did not list.
    #[error("reply attempted to match unknown child")]
    UnexpectedMatch,
    /// The reply carried a match after its first, whose covered set names
    /// every later matching child.
    #[error("reply matched a child after its first match")]
    RepeatedMatch,
    /// The reply's match covered a child that the question did not list
    /// after the match's own.
    #[error("reply's match covered a child that is not a later listed child")]
    UnlistedCover,
    /// The peer asked about a child that the question did not list.
    #[error("reply attempted to query unknown child")]
    UnexpectedQuery,
    /// The peer supplied a child that the receiving replica already holds.
    #[error("reply attempted to supply a child that is already known")]
    UnexpectedSupply,
    /// A supplied child did not fit the question's radix order.
    #[error("reply attempted to supply a child out of order")]
    InvalidSupply,
    /// A supplied subtree carrying a version not in the causal past or present
    /// of the sender's declared version.
    #[error("reply supplied a subtree with a version outside the sender's declared version")]
    UncontainedSupply,
    /// The total number of leaves supplied was more than the sender's declared
    /// set length, counted across the whole session.
    #[error("reply supplied more leaves than the sender's declared set length")]
    OverdrawnSupply,
}
