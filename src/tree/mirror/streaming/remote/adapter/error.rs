use crate::tree::mirror::streaming::materialized::Overdraft;

use super::super::codec::{DecodeLeafError, LengthOverflow};

/// A prefix-free reaction could not be paired with the question it answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ScopeError {
    /// A positional query followed all children named by its question.
    #[error("a query has no remaining child in its question")]
    UnpositionedQuery,
    /// A match followed all children named by its question.
    ///
    /// Detected at the offending frame, so a reply cannot grow its decoded
    /// skeleton past the question's fan before the overrun surfaces.
    #[error("a match has no remaining child in its question")]
    UnpositionedMatch,
    /// A nonempty query cannot descend below leaf height.
    #[error("a leaf-height reply contains a nonempty query")]
    NonemptyLeafQuery,
    /// A reply carried a second match: its first match covers every later
    /// listed child that also matches, so no later match takes a frame.
    #[error("a reply carries a match after its first")]
    RepeatedMatch,
    /// A match covers a radix that is not a position after its own: one the
    /// question did not list, or one the reply has already passed.
    #[error("a match covers radix {radix:#04x}, which is not a later listed child")]
    UnlistedCover {
        /// The radix covered.
        radix: u8,
    },
    /// A reply ended with a listed child it neither took a reaction for nor
    /// covered by its match.
    #[error("a reply ends without answering or covering listed radix {radix:#04x}")]
    UncoveredEnd {
        /// The first listed radix left.
        radix: u8,
    },
    /// A reply supplied a node at a radix its match covers, which both sides
    /// hold.
    #[error("a reply supplies radix {radix:#04x}, which its match covers")]
    CoveredSupply {
        /// The radix supplied.
        radix: u8,
    },
    /// A reaction sits at or below the radix of the reaction before it: a
    /// reply's reactions ascend by radix, its supplies among its positional
    /// reactions.
    #[error("a reaction at radix {radix:#04x} does not follow one at radix {after:#04x}")]
    Unordered {
        /// The radix of the reaction before it.
        after: u8,
        /// The radix of the reaction out of order.
        radix: u8,
    },
}

/// The initiator's distinguished opening reply violated its canonical shape:
/// exactly one query.
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
    /// The opening reply carries nothing after its query.
    #[error("the opening reply continues after its query")]
    Trailing,
}

/// A protocol reply could not be rendered faithfully as wire frames.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
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
#[non_exhaustive]
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
    /// A leaf was supplied to a side that holds no network secret: a
    /// bootstrapper meeting another bootstrapper, which can be supplied
    /// nothing, and cannot place a leaf without its network's key.
    #[error("a leaf was supplied to a bootstrapper holding no network secret")]
    UnkeyedSupply,
    /// A supplied leaf's keyed path lies outside the scope its run was supplied
    /// under.
    ///
    /// The message prints keyed paths, which reveal no key (see
    /// [`NetworkSecret`](crate::network::NetworkSecret)).
    #[error("supplied leaf {actual:02x?} is outside reply scope {expected:02x?}")]
    LeafOutsideScope {
        /// The path prefix of the scope the leaf was supplied under.
        expected: Vec<u8>,
        /// The leaf's keyed path, computed from its version.
        actual: [u8; 32],
    },
    /// Supplied leaves were not strictly ascending by keyed path.
    ///
    /// The message prints keyed paths, which reveal no key (see
    /// [`NetworkSecret`](crate::network::NetworkSecret)).
    #[error("supplied leaf {current:02x?} does not follow {previous:02x?}")]
    LeafOrder {
        /// The keyed path of the leaf supplied before it.
        previous: [u8; 32],
        /// The keyed path of the leaf out of order.
        current: [u8; 32],
    },
    /// A supplied version's encoding exceeds the peer's declared
    /// `max_version_bytes`.
    ///
    /// Every leaf version a peer supplies is one its own tree materializes, so
    /// the aggregate its greeting declared must cover it. A version arriving
    /// over it shows the declaration false, so the session fails fast rather
    /// than run on it. The greeting's own causal version is *not* held to the
    /// declaration — an empty tree honestly declares a zero aggregate while its
    /// redaction-advanced version is nonempty — so only supplied leaf records
    /// are checked.
    #[error(
        "supplied version encodes {actual} bytes, over the peer's declared {declared}-byte bound"
    )]
    OversizedVersion { declared: u64, actual: usize },
    /// A supplied leaf's version is not contained in the peer's own greeting
    /// version, which joins every version the peer's tree holds.
    ///
    /// A conforming peer supplies only leaves its tree holds, so it never
    /// supplies such a leaf. Absorbed, the leaf would lie outside the ceiling
    /// this side adopts for the session, where no later redaction this side
    /// makes can cover it, so the session fails at the offending record, before
    /// its payload takes backend custody. Containment is a partial order: a
    /// version incomparable with the peer's is as uncontained as one above it.
    ///
    /// The message prints the leaf's keyed path, which reveals no key (see
    /// [`NetworkSecret`](crate::network::NetworkSecret)).
    #[error("supplied leaf {path:02x?} lies outside the peer's declared version")]
    UncontainedSupply {
        /// The leaf's keyed path, computed from its version.
        path: [u8; 32],
    },
    /// A supplied leaf's version is contained in this side's own greeting
    /// version: the leaf is one this side holds or has redacted.
    ///
    /// A peer filters its supplies against that version and ships only leaves
    /// it does not contain, so a conforming peer never supplies such a leaf.
    /// Absorbing one would bring back a message this side redacted, so the
    /// session fails at the offending record, before its payload takes backend
    /// custody.
    ///
    /// The message prints the leaf's keyed path, which reveals no key (see
    /// [`NetworkSecret`](crate::network::NetworkSecret)).
    #[error("supplied leaf {path:02x?} is one this side has already seen")]
    SeenSupply {
        /// The leaf's keyed path, computed from its version.
        path: [u8; 32],
    },
    /// A supplied leaf record past the peer's declared `set_len`.
    ///
    /// An honest peer supplies each leaf at most once and only leaves its own
    /// set holds, so its greeting-declared set length bounds the session's
    /// total supplied records. The charge lands at ingress, before the record's
    /// payload takes backend custody, so a peer supplying past its declaration
    /// fails the session at the offending record while its reply is still open,
    /// never after the reply materializes. The in-process walk enforces the
    /// same premise at absorption as
    /// [`Violation::OverdrawnSupply`](crate::tree::mirror::streaming::materialized::Violation::OverdrawnSupply).
    #[error("supplied leaf overruns the peer's declared set length of {declared}")]
    OverdrawnSupply { declared: u64 },
    /// A reply holds more reactions, or carries more listing entries, than the
    /// grandchild count the replier gave the node it is about allows
    /// ([`ReplyBound`](crate::tree::mirror::streaming::ledger::ReplyBound)).
    ///
    /// The asker reserved memory for the reply from that count before it asked,
    /// so a reply over it fails the session at the frame that overruns.
    #[error("a reply exceeds its bound of {bound} {part}")]
    ExceedsBound { part: BoundedPart, bound: usize },
    /// A positional wire reaction cannot be scoped without another child.
    #[error(transparent)]
    Scope(#[from] ScopeError),
    /// A leaf record inside a supply run failed canonical decoding.
    #[error("a supplied leaf record is not canonical")]
    Record(#[source] DecodeLeafError),
}

/// The error a reply's frame reveals by asking its quota for more than it can
/// give: the bound it would exceed, or the scope rule it would break.
impl<E> From<Overdraft> for DecodeError<E> {
    fn from(overdraft: Overdraft) -> Self {
        match overdraft {
            Overdraft::Reactions { bound } => Self::ExceedsBound {
                part: BoundedPart::Reactions,
                bound,
            },
            Overdraft::Listing { bound } => Self::ExceedsBound {
                part: BoundedPart::Listing,
                bound,
            },
            Overdraft::Unordered { after, radix } => ScopeError::Unordered { after, radix }.into(),
            Overdraft::CoveredSupply { radix } => ScopeError::CoveredSupply { radix }.into(),
            Overdraft::UnpositionedQuery => ScopeError::UnpositionedQuery.into(),
            Overdraft::UnpositionedMatch => ScopeError::UnpositionedMatch.into(),
            Overdraft::RepeatedMatch => ScopeError::RepeatedMatch.into(),
        }
    }
}

/// What a reply's bound limits ([`DecodeError::ExceedsBound`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BoundedPart {
    /// The reply's reactions.
    #[error("reactions")]
    Reactions,
    /// The listing entries its queries carry together.
    #[error("listing entries")]
    Listing,
}
