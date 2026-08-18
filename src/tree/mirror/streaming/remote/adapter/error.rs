use crate::tree::mirror::framing::LengthOverflow;

use super::super::codec::DecodeLeafError;

/// A prefix-free reaction could not be paired with the question it answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ScopeError {
    /// A positional query followed all children named by its question.
    #[error("a query has no remaining child in its question")]
    UnpositionedQuery,
    /// A match followed all children named by its question.
    ///
    /// Detected at the offending frame, so a reply cannot grow its
    /// decoded skeleton past the question's fan before the overrun
    /// surfaces.
    #[error("a match has no remaining child in its question")]
    UnpositionedMatch,
    /// A nonempty query cannot descend below leaf height.
    #[error("a leaf-height reply contains a nonempty query")]
    NonemptyLeafQuery,
}

/// The initiator's distinguished opening reply violated its canonical
/// shape: one leading query, then only whole-subtree supplies.
///
/// Local-only by construction: the peer's opening question arrives as the
/// greeting's listing, validated at greeting decode, so only the locally
/// produced opening reply can still be malformed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OpeningError {
    /// The opening reply must lead with the implicit root question.
    #[error("the opening reply is empty")]
    Empty,
    /// The opening's first reaction must ask the implicit root question.
    #[error("the opening reply does not lead with a query")]
    NotQuery,
    /// Everything after the opening question must be an early supply.
    #[error("opening reaction {index} is not a whole-subtree supply")]
    NotSupply { index: usize },
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
    /// A supplied leaf's encoding overflows the run's record framing.
    #[error("a supplied leaf record overflows the run framing")]
    Record(#[source] LengthOverflow),
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
    /// Transport control leaked through the demultiplexer into reply decoding.
    #[error("a stream-end control reached the protocol reply decoder")]
    UnexpectedStreamEnd,
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
    /// A supplied version's encoding exceeds the peer's declared `max_version_bytes`.
    ///
    /// Every leaf version a peer supplies is one its own tree
    /// materializes, so the aggregate its greeting declared must cover
    /// it. The local window solve priced node residency from that
    /// declaration; a version arriving over it voids the pricing
    /// premise, so the session fails fast instead of running outside
    /// its memory envelope. The greeting's own causal version is *not*
    /// held to the declaration — an empty tree honestly declares a
    /// zero aggregate while its redaction-advanced version is nonempty
    /// — so only supplied leaf records are checked.
    #[error(
        "supplied version encodes {actual} bytes, over the peer's declared {declared}-byte bound"
    )]
    OversizedVersion { declared: u64, actual: usize },
    /// A supplied leaf record past the peer's declared `set_len`.
    ///
    /// An honest peer supplies each leaf at most once and only leaves its
    /// own set holds, so its greeting-declared set length bounds the
    /// session's total supplied records; the local window solve priced
    /// absorbed-supply volume from that declaration. The charge lands at
    /// ingress, before the record's payload takes backend custody, so a
    /// peer supplying past its declaration fails the session at the
    /// offending record while its reply is still open, never after the
    /// reply materializes. The in-process walk enforces the same premise
    /// at absorption as
    /// [`Violation::OverdrawnSupply`](crate::error::MaterializedViolation::OverdrawnSupply).
    #[error("supplied leaf overruns the peer's declared set length of {declared}")]
    OverdrawnSupply { declared: u64 },
    /// A positional wire reaction cannot be scoped without another child.
    #[error(transparent)]
    Scope(#[from] ScopeError),
    /// A leaf record inside a supply run failed canonical decoding.
    #[error("a supplied leaf record is not canonical")]
    Record(#[source] DecodeLeafError),
    /// The opening-supply stream carried frames after its one reply.
    #[error("the opening-supply stream carried a second reply")]
    ExtraOpeningReply,
}
