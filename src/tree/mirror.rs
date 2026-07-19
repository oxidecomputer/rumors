//! Mirror-sync between two replicas of the typed tree.
//!
//! [`streaming`] is the default protocol. `alternating` serves as streaming's
//! behavioral oracle in this crate's tests, and remains selectable on the
//! wire behind the `protocol-v1` cargo feature: its state machines are a
//! large monomorphization surface, so binaries that never speak V1 should
//! not spend compile time on it.
//!
//! # Wire ingress inventory
//!
//! Every point where peer-controlled bytes enter a parser, in both
//! protocols. The tripwire contract is uniform across the table: malformed
//! bytes at any ingress surface a typed session error — never a panic,
//! never a hang, never a silent misparse. Under the trusted-counterparty
//! model (see the crate docs) this validation is a bug tripwire, not a
//! security boundary; in particular, peer-declared frame lengths are
//! trusted for allocation once the preamble has vetted the counterparty
//! (see [`framing`]). Each ingress names the suite that feeds it malformed
//! bytes; a new ingress belongs in this table, with a suite of its own,
//! before it ships. Suites live in `tests.rs` siblings of the parser they
//! exercise (test modules do not render in rustdoc, so they are named by
//! path here rather than linked).
//!
//! Shared by both protocols, in session order:
//!
//! 1. **Session preamble** — the fixed 25-byte magic/version/network/intent
//!    frame every session leads with. Parser: [`handshake`]. Suite:
//!    `handshake/tests.rs` (truncation at every boundary, magic/version
//!    mismatch, intent byte space, bootstrap-retire conflict, whole-frame
//!    fuzz against a decode oracle).
//! 2. **Trailing party-donation frame** — the framed identity hand-off after
//!    a bootstrap or retire descent. Parser: [`party`]. Suite:
//!    `party/tests.rs` (frame truncation and length lies, body byte space,
//!    trailing garbage, frame-boundary preservation, canonical-round-trip
//!    fuzz).
//!
//! The streaming (V2) protocol:
//!
//! 3. **Greeting frames** — the causal-version frame and the root-fan
//!    listing frame on the control stream. Parser: `streaming/remote/proxy/
//!    start.rs` (`receive`). Suite: `start/tests.rs` (both frames:
//!    truncation, length lies, malformed and trailing bytes, listing order
//!    violations, arbitrary-body fuzz).
//! 4. **Data-stream label** — the two epoch/index bytes ahead of a data
//!    stream's first frame. Parser: `streaming/remote/streams.rs`
//!    (`AcceptDriver`). Suite: `streams/tests.rs` (wrong epoch, unknown
//!    index, duplicate and unasked streams, label/frame disagreement).
//! 5. **Data-stream frame codec** — signal byte, query listing, supply-run
//!    framing. Parser: `streaming/remote/codec/`. Suites:
//!    `codec/decode/tests.rs`, the error atlas and bounded-exhaustive frame
//!    corpus in `codec/tests/`, and full-stack injection in
//!    `proxy/tests/malformed.rs`. No arbitrary-byte fuzz here by decision:
//!    a supply frame's peer-declared `u32` length is trusted for allocation
//!    (the documented post-preamble trust), so unconstrained header fuzz
//!    would exercise the allocator, not the parser; the bounded-exhaustive
//!    corpus covers the byte space the codec actually interprets.
//! 6. **Leaf records inside supply runs** — per-record version/message
//!    decode plus cross-record order and scope. Parser:
//!    `streaming/remote/adapter/decode.rs`. Suite:
//!    `adapter/tests/malformed.rs`.
//! 7. **Session epilogue marker** — the one completion byte each side reads
//!    after all session work. Parser: the gossip driver in
//!    `src/peer/gossip.rs`. Suite: `src/peer/gossip/tests.rs` (byte-space
//!    exhaustion, honest-cut/violation distinction, boundary preservation);
//!    end-to-end pins in `tests/lifecycle.rs`.
//!
//! The alternating (V1) protocol, behind `protocol-v1`:
//!
//! 8. **Framed borsh messages** — the greeting and every descent message,
//!    each one length-delimited frame. Framing parser:
//!    `alternating/backend/remote.rs` (`recv_msg`); body parsers:
//!    `alternating/message.rs` and the typed node decoder in
//!    `src/tree/typed/node.rs`. Suites: `alternating/backend/remote/
//!    tests.rs` (frame truncation, length lies, trailing bytes, boundary
//!    preservation, per-message-type body fuzz),
//!    `alternating/message/tests.rs` (canonical-order rejection and node
//!    structural lies), `alternating/wire_snapshot.rs` (byte-for-byte
//!    pinning).
//!
//! The [`framing`] substrate (the shared length-delimited frame reader)
//! carries ingresses 2, 3, and 8; its truncation and length-lie behavior is
//! pinned through each consumer's suite above rather than in isolation, so
//! every pin holds at the error type the session actually surfaces.

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
