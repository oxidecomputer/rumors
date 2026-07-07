//! The streaming protocol spoken over a real wire: a typed proxy of the
//! remote counterparty.
//!
//! Where [`materialized`](super::materialized) realizes the protocol traits
//! by walking a local tree, the implementors here realize them as a proxy of
//! the counterparty across an asynchronous read/write pair: each stage
//! serializes the messages the local party feeds it onto the wire, and
//! decodes the counterparty's replies off of it. The proxy defines no backend
//! of its own — it is parameterized by the *local party's* backend, through
//! which it explodes outgoing subtrees into leaves for the wire and
//! reassembles incoming leaves into that party's node types ([`codec`]).

mod codec;
mod level;

/// The stream tag of the node-carrying level keyed at `height`.
///
/// The schedule sends exactly one stream per height — `exchanged` at 31
/// down to 2, `closing` at 1, `complete` at 0 — so a level's position in
/// the schedule is a pure function of height, independent of which side
/// the proxy plays: tags 2 through 33, with [`INITIATE`] and [`OPENING`]
/// ahead of them and [`HANDSHAKE`] out of schedule entirely.
pub(super) fn level(height: usize) -> u8 {
    debug_assert!(height <= 31, "node-carrying levels sit under the root");
    (33 - height) as u8
}

/// The initiator's root-hash level.
pub(super) const INITIATE: u8 = 0;

/// The responder's root-listing level.
pub(super) const OPENING: u8 = 1;

/// The out-of-schedule stream carrying the pre-session version exchange.
pub(super) const HANDSHAKE: u8 = 255;

/// A protocol violation observed on the wire.
///
/// Every variant names a byte sequence or message ordering that no honest
/// counterparty produces: encountering one means the peer is buggy or
/// malicious (or the transport corrupted something), and the session aborts
/// with the local tree untouched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Violation {
    /// A level's stream ended before the grammar it was carrying completed.
    #[error("wire stream truncated mid-message")]
    Truncated,

    /// A leaf's derived path fell outside the subtree the run's first leaf
    /// named.
    #[error("leaf outside its subtree")]
    Misplaced,

    /// A subtree's derived leaf paths were not strictly ascending.
    #[error("leaf paths not strictly ascending")]
    LeafOrder,

    /// A subtree carried no leaves at all.
    #[error("empty subtree")]
    EmptySubtree,

    /// An item arrived that its level's message vocabulary cannot express
    /// (or that interrupted a leaf run).
    #[error("item foreign to its level")]
    UnexpectedItem,

    /// A level's message prefixes were not strictly ascending.
    #[error("message prefixes not strictly ascending")]
    MessageOrder,
}

/// The error of a remote session, generic over the local backend's error `E`.
///
/// `Io` and `Violation` originate on the proxy's side of the party boundary;
/// `Backend` carries the local backend's own faults raised while exploding
/// outgoing subtrees for the wire. (Faults raised *reassembling* incoming
/// subtrees travel separately, in the second position of the outgoing
/// streams' [`OutputError`](super::protocol::OutputError), exactly as the
/// materialized implementation's conversion faults do.)
#[derive(Debug, thiserror::Error)]
pub enum Error<E> {
    /// An underlying transport error, or a borsh error encountered while
    /// parsing items off a level's stream.
    #[error(transparent)]
    Io(std::io::Error),

    /// The counterparty broke the wire protocol.
    #[error(transparent)]
    Violation(Violation),

    /// The local backend failed while exploding a subtree for the wire.
    #[error("backend error while encoding for the wire")]
    Backend(#[source] E),
}

/// A backend fault lifts into the session error.
///
/// This is the impl behind the drivers' `I::Error: From<BI::Error>` bound:
/// the party boundary's [`divert`](super::divert) lifts the backend's own
/// errors through it.
impl<E> From<E> for Error<E> {
    fn from(backend: E) -> Self {
        Error::Backend(backend)
    }
}

/// A wire-layer fault with no backend involvement: what the transport and
/// the grammar alone can produce.
pub(super) type WireError = Error<std::convert::Infallible>;

impl WireError {
    /// Re-tag a backend-free fault under any backend error type: a total,
    /// lossless re-tag, since the backend arm is uninhabited here.
    pub(super) fn widen<E>(self) -> Error<E> {
        match self {
            Error::Io(error) => Error::Io(error),
            Error::Violation(violation) => Error::Violation(violation),
            Error::Backend(never) => match never {},
        }
    }
}
