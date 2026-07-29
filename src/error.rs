//! Public failures from transport sessions and durable identity handling.
//!
//! You handle [`Error`]; everything else on this page is the diagnostic
//! taxonomy reachable through [`Error::Mirror`], for matching and bug
//! reports. Every session `Err` poisons its link (discard it and
//! reconnect, [`Error::LinkPoisoned`]), so the table below states what
//! each variant means *beyond* that:
//!
//! | Variant | Replica | Beyond reconnecting |
//! |---|---|---|
//! | [`Error::Io`] | unchanged | transport failure (retry over a fresh link), or a Borsh framing fault outside the streaming mirror (counterparty bug: report it) |
//! | [`Error::MagicMismatch`] | unchanged | the counterparty is not speaking rumors: fix the dial target |
//! | [`Error::VersionMismatch`] | unchanged | select the same [`Protocol`] at both ends; if both already do, the selected protocol's wire version differs across the two releases: align crate versions |
//! | [`Error::ProtocolUnsupported`] | unchanged | the selected protocol cannot run on this peer's storage backend: select [`Protocol::V2`], or keep the peer in memory |
//! | [`Error::NetworkMismatch`] | unchanged | unrelated universes: apply the dominance rule ([`Peer`](crate::Peer)'s "Bootstrapping without consensus") |
//! | [`Error::PartyOverlap`] | unchanged | nothing was absorbed: the retiring peer's identity overlaps ours |
//! | [`Error::Epilogue`] | **committed** (a bootstrapping side instead applies nothing) | none locally: what was certainly lost is the peer's confirmation (a donor's identity may be lost with it: see the variant) |
//! | [`Error::LinkPoisoned`] | unchanged | handle the first non-poisoned error; repeats mean the reconnect is not producing a fresh link |
//! | [`Error::IntentInvalid`] | unchanged | counterparty bug: report it |
//! | [`Error::BootstrapRetireConflict`] | unchanged | counterparty bug: report it |
//! | [`Error::BootstrapHistoryConflict`] | unchanged | counterparty bug: report it |
//! | [`Error::Bookmark`] | unchanged (committed if raised after absorbing a retirement) | fix or replace the bookmark storage, then retry |
//! | [`Error::Mirror`] | unchanged | reconciliation failed: the nested source names the detecting side and the fault |
//! | [`Error::Storage`] | unchanged | the storage backend failed on a local operation: fix or replace the storage, then retry |

use std::convert::Infallible;

use crate::{
    Network, Protocol, Ticks,
    bookmark::{BookmarkError, BookmarkIo, NoBookmark},
    tree::mirror::{self, handshake},
};

pub use crate::tree::mirror::streaming::materialized::{
    Error as MaterializedError, Violation as MaterializedViolation,
};
pub use crate::tree::mirror::streaming::remote::{
    AcceptError, CodecDecodeError, CodecDecodeErrorKind, CodecEncodeError, CodecEncodeErrorKind,
    DecodeError, DecodeLeafError, DecodeSignalError, EncodeError, FramePart,
    InvalidSignalPlacement, InvalidWireSignal, LeafRunError, LengthOverflow, OpeningError, Origin,
    QueryOrderError, RemoteError, ReplyFrameError, ScopeError, SendError, Speaker, Stream,
    StreamClass, StreamError,
};

/// The production mirror failure, retaining its detecting side.
///
/// Generic over the storage backend's error `E`; the default in-memory
/// backend is infallible.
pub type MirrorError<E = Infallible> = mirror::Error<MaterializedError<E>, RemoteError<E>>;

/// A storage-backend failure on a local operation.
///
/// Wraps the backend's own error wherever an operation touches stored
/// tree state outside a session: sending, redacting, committing a batch,
/// or reading through a snapshot or observer. The default in-memory
/// backend is infallible, so for it this error cannot be constructed.
#[derive(Debug, thiserror::Error)]
#[error("storage backend failed: {0}")]
pub struct StorageError<E>(#[source] pub E);

/// An error returned by bootstrap, gossip, or retirement.
///
/// Generic over the bookmark `B` only to retain its backend error in
/// [`Bookmark`](Self::Bookmark), and over the storage backend's error `E`
/// in [`Mirror`](Self::Mirror) and [`Storage`](Self::Storage). Every wire
/// and protocol variant is otherwise independent of both. The default
/// bookmark type and the default in-memory storage backend both have
/// uninhabited errors.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error<B: BookmarkError = NoBookmark, E = Infallible> {
    /// An underlying reader/writer error, or a Borsh framing failure outside
    /// the streaming mirror itself.
    #[error(transparent)]
    Io(#[from] borsh::io::Error),

    /// The peer's preamble did not begin with [`PROTOCOL_MAGIC`](crate::PROTOCOL_MAGIC).
    #[error("peer is not a rumors stream (remote magic: {remote_magic:x?})")]
    MagicMismatch { remote_magic: [u8; 6] },

    /// The peer speaks a different wire dialect.
    #[error("peer speaks rumors protocol version {remote_version}, we selected {local_protocol:?}")]
    VersionMismatch {
        local_protocol: Protocol,
        remote_version: u16,
    },

    /// The selected protocol cannot run against this peer's storage
    /// backend.
    ///
    /// Raised before any wire traffic, so nothing moved and the link is
    /// poisoned only by the usual session-interruption rule. Today this
    /// names exactly one combination: [`Protocol::V1`] sessions require
    /// the in-memory backend (the frozen alternating protocol works on
    /// resident nodes). Select [`Protocol::V2`], or keep V1 peers on the
    /// in-memory backend.
    #[error("the selected protocol ({protocol:?}) requires the in-memory storage backend")]
    ProtocolUnsupported {
        /// The protocol the peer had selected.
        protocol: Protocol,
    },

    /// Both peers were gossiping but belong to unrelated causal universes.
    #[error("peer belongs to a different network ({remote_network:?})")]
    NetworkMismatch {
        /// The network identifier advertised by the remote peer.
        remote_network: Network,
        /// A lower bound on events recorded in the remote universe.
        remote_min_events: Ticks,
        /// A lower bound on events recorded in the local universe, as this
        /// side declared it in the session's handshake.
        ///
        /// Together with `remote_min_events` this lets both sides of a
        /// mismatch apply one deterministic dominance rule from the error
        /// alone (see [`Peer`](crate::Peer)'s "Bootstrapping without
        /// consensus"): [`Ticks`] is totally ordered at any magnitude, so
        /// the comparison never saturates or ties spuriously, however deep
        /// the two universes' histories run.
        local_min_events: Ticks,
    },

    /// A retiring peer offered an identity overlapping one already held here.
    #[error("retiring peer's party overlaps ours")]
    PartyOverlap,

    /// The session's closing epilogue failed *after* the session's local
    /// work committed.
    ///
    /// A [`Protocol::V2`] session ends with a completion exchange on the
    /// control stream: each side commits all of its session work, then
    /// writes one marker byte and reads the peer's. Returning `Ok` requires
    /// having read the peer's marker, so `Ok` certifies that the peer
    /// committed too. This error means the exchange itself failed: the
    /// local side committed, but the peer's confirmation never arrived, so
    /// the peer may have committed or may have failed. That uncertainty is
    /// irreducible (the two-generals problem); the exchange pins it to this
    /// one distinguished error instead of letting it hide behind `Ok`.
    ///
    /// Replica state never needs the exchange: content converges by CRDT
    /// join whatever either side believes, and a donated identity is
    /// committed out of the donor before it crosses the wire, so a failed
    /// session can leave an identity held by no one, never by both. What
    /// the exchange protects is the *success report*. A donor completing
    /// an identity hand-off (retire, or serving a bootstrap) loses the
    /// donated identity irreparably whenever its counterparty fails
    /// before committing; that loss happens with or without a
    /// confirmation exchange, but without one, the donor would report
    /// success anyway. With the exchange in place, identity can be lost
    /// only inside this one window, and the window always announces
    /// itself as this error, never as an `Ok`.
    ///
    /// For a session on an existing replica (gossip, retire, or the side
    /// serving a bootstrap), the local replica **is** committed on this
    /// error: every message and identity the session moved is applied here.
    /// The bootstrapping side is the exception: its epilogue runs before
    /// any [`Peer`](crate::Peer) exists, so the received identity is
    /// dropped and nothing is applied locally, while the provider may have
    /// committed; the forked identity is then lost (see
    /// [`Bootstrap::join`](crate::Bootstrap::join)). The source is the I/O
    /// failure that cut the exchange short, or an invalid-data error if the
    /// peer wrote something other than the marker where it belonged.
    #[error("session epilogue failed after local commit: {0}")]
    Epilogue(#[source] std::io::Error),

    /// A session was started on a link whose previous session was
    /// interrupted.
    ///
    /// A session that fails or is cancelled leaves the link's control
    /// stream mid-frame, where a next session would misread its leftover
    /// bytes as a preamble. The link records the interruption
    /// ([`SessionState`](crate::link::SessionState)) and every subsequent
    /// session fails here, before any wire traffic. Discard the link and
    /// reconnect; the replica itself is unharmed. Seeing this repeatedly
    /// means the reconnect path is not actually producing a fresh link;
    /// the root cause is the first non-poisoned error.
    #[error(
        "link is poisoned: an earlier session on it was interrupted before completing; discard the link and reconnect"
    )]
    LinkPoisoned,

    /// The peer's intent byte had no defined meaning.
    #[error("peer sent an invalid intent byte ({byte:#04x})")]
    IntentInvalid { byte: u8 },

    /// A peer cannot simultaneously receive and donate an identity.
    #[error("peer claimed to bootstrap and retire in the same session")]
    BootstrapRetireConflict,

    /// A bootstrap claimant declared a non-empty causal version.
    ///
    /// A bootstrap claimant is definitionally a newborn replica with no
    /// causal history, so its greeting version must be empty. The declared
    /// version feeds the deletion-honoring filter as the claimant's causal
    /// frontier, and a mis-declared frontier would make established content
    /// read as deleted-there; the conflict between the two claims
    /// (newborn, yet with history) is rejected here, after the greeting
    /// and before reconciliation moves anything.
    ///
    /// Detected by whichever side faces the claimant: a provider serving
    /// the bootstrap, or a bootstrapping peer whose counterparty is itself
    /// a claimant (the mutual-bootstrap encounter). The detecting side's
    /// replica is unchanged (no content, identity, or bookmark state
    /// moved) and its link is poisoned like any failed session's. The
    /// recovery is the claimant's: rejoin with a genuinely newborn
    /// replica, whose version is empty by construction.
    #[error(
        "peer claimed to bootstrap while declaring causal history (at least {claimed_min_events} events): a bootstrap claimant is a newborn replica whose version is empty"
    )]
    BootstrapHistoryConflict {
        /// A lower bound on events recorded in the claimant's declared
        /// version, as [`NetworkMismatch`](Self::NetworkMismatch) counts
        /// them.
        claimed_min_events: Ticks,
    },

    /// The application's bookmark failed to load, persist, or decode.
    ///
    /// The replica's content is never affected; fix or replace the storage
    /// and retry. In the common case the error arrives *before* the
    /// session transmits anything, and nothing has changed at all. The one
    /// post-commit case: absorbing a retiring peer commits the reconciled
    /// content and the absorbed identity first, then persists, so this
    /// error can arrive with the absorption live but not yet crash-safe. A
    /// crash before some later session persists successfully strands that
    /// identity, held by no live peer and recorded in no bookmark;
    /// retrying [`gossip`](crate::Rumors::gossip) on a fresh link re-runs
    /// the persist. Independently, identity a failed update had already
    /// reclaimed from the record stays live in memory, and the next
    /// successful persist records it.
    #[error(transparent)]
    Bookmark(BookmarkIo<B::Error>),

    /// Reconciliation failed in either the materialized participant or its
    /// wire-bound counterparty proxy.
    ///
    /// The nested source retains the detecting side and remains matchable
    /// through backend, adapter, session, codec, and transport errors.
    #[error(transparent)]
    Mirror(#[from] MirrorError<E>),

    /// The storage backend failed on a local operation inside the session
    /// driver, outside reconciliation itself.
    ///
    /// The replica is unchanged: the session's result was not committed.
    /// Fix or replace the storage, then retry over a fresh link.
    #[error(transparent)]
    Storage(#[from] StorageError<E>),
}

impl<E> From<handshake::Error> for Error<NoBookmark, E> {
    fn from(error: handshake::Error) -> Self {
        match error {
            handshake::Error::Io(error) => Error::Io(error),
            handshake::Error::MagicMismatch { remote_magic } => {
                Error::MagicMismatch { remote_magic }
            }
            handshake::Error::VersionMismatch {
                local_protocol,
                remote_version,
            } => Error::VersionMismatch {
                local_protocol,
                remote_version,
            },
            handshake::Error::IntentInvalid { byte } => Error::IntentInvalid { byte },
            handshake::Error::BootstrapRetireConflict => Error::BootstrapRetireConflict,
        }
    }
}

impl<E> Error<NoBookmark, E> {
    /// Re-tags a bookmark-free session error under any bookmark `B`.
    ///
    /// Wire and protocol machinery produces `Error<NoBookmark, E>`;
    /// peer-level drivers return `Error<B, E>`. The only bookmark backend
    /// error here is uninhabited, making the conversion total and lossless.
    pub(crate) fn widen<B: BookmarkError>(self) -> Error<B, E> {
        match self {
            Error::Io(error) => Error::Io(error),
            Error::MagicMismatch { remote_magic } => Error::MagicMismatch { remote_magic },
            Error::VersionMismatch {
                local_protocol,
                remote_version,
            } => Error::VersionMismatch {
                local_protocol,
                remote_version,
            },
            Error::NetworkMismatch {
                remote_network,
                remote_min_events,
                local_min_events,
            } => Error::NetworkMismatch {
                remote_network,
                remote_min_events,
                local_min_events,
            },
            Error::PartyOverlap => Error::PartyOverlap,
            Error::ProtocolUnsupported { protocol } => Error::ProtocolUnsupported { protocol },
            Error::Epilogue(error) => Error::Epilogue(error),
            Error::LinkPoisoned => Error::LinkPoisoned,
            Error::IntentInvalid { byte } => Error::IntentInvalid { byte },
            Error::BootstrapRetireConflict => Error::BootstrapRetireConflict,
            Error::BootstrapHistoryConflict { claimed_min_events } => {
                Error::BootstrapHistoryConflict { claimed_min_events }
            }
            Error::Mirror(error) => Error::Mirror(error),
            Error::Storage(error) => Error::Storage(error),
            Error::Bookmark(error) => match error {
                BookmarkIo::Io(never) => match never {},
                BookmarkIo::Format(error) => Error::Bookmark(BookmarkIo::Format(error)),
            },
        }
    }
}
