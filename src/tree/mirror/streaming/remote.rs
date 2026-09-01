//! Wire-bound proxy for the streaming mirror.
//!
//! The submodule roles: [`codec`] owns the frame grammar; [`proxy`] the
//! session state machine and its work loops; [`adapter`] scope retention
//! and leaf reconstruction; [`streams`] the binding of logical streams to
//! the link's transport streams.
//!
//! The transport is a [`Link`](crate::link): 17 logical streams in each
//! direction, each carried by its own independently flow-controlled
//! transport stream, lazily established as the descent needs it.
//!
//! [`codec`] defines the common frame grammar: the signal densely encodes
//! the product of ten frame states and 17 stream ids as
//! `state * 17 + stream`. The states are each of the four reaction forms
//! (`Match`, empty/nonempty `Query`, and `Supply`) either continuing or
//! ending its reply, plus bare `ReplyEnd` and `StreamEnd`. Values 170
//! through 255 are reserved. The phase schedule narrows that syntactic
//! product further: the initiator admits 162 placements and the responder
//! 163, rejecting the rest immediately after the signal byte, before any
//! frame body is read. Every frame's signal names the stream it rides, so
//! the signal's stream component is redundant with the stream's label —
//! deliberately: [`streams`] holds every frame to exact agreement with the
//! label, so a miswired link surfaces at the first frame.
//!
//! Reply and stream ends are separate events. A reaction or bare `ReplyEnd`
//! completes a reply; a later bare `StreamEnd` closes the logical stream
//! ahead of the transport-level half-close. The stream layer consumes that
//! control instead of exposing it to the protocol adapter as an empty reply.
//!
//! An empty query occupies its signal alone; a nonempty query's one-byte
//! count-minus-one admits every fan from 1 through 256.
//!
//! Supplied leaves ship in *runs*: one exact-length-delimited body carrying
//! one or more leaf records, each itself exact-length-delimited — a CBOR
//! byte string wrapping the [`Version`](crate::Version)'s canonical bytes,
//! then the [`Message`](crate::message::Message)'s CBOR payload. The
//! encoder chunks a supplied subtree's leaves into runs by a byte budget
//! ([`RunBudget`]); once a run's whole body arrives, the frame codec
//! validates its record framing and the incoming adapter decodes each
//! backend-neutral pair exactly once, constructing a backend leaf and
//! validating its version-derived path.
//!
//! The initiator's distinguished opening question needs no wire frame: its
//! content — the initiator's root-fan listing — rides the greeting on the
//! control stream (see [`message::Greeting`](super::message::Greeting)
//! for the trade), so the elected responder answers one hop earlier. The
//! initiator-direction opening stream instead carries the *early
//! supplies*: the initiator's exclusive root children, shipped whole in
//! one supplies-only reply the moment the greetings cross — the answers
//! to root-level requests the responder is provably about to make, one
//! hop and one decomposition level ahead of being asked. The stream opens
//! exactly when that exclusive set is nonempty, and the responder's
//! matching empty queries are answered by bare empty replies: pairing
//! intact, content relocated.
//!
//! [`adapter`] retains the question scope omitted from protocol replies. It
//! attaches each newly asked scope to the exact outgoing frame which makes the
//! question publishable, derives supplied radices from leaf content, and uses
//! the backend's existing conversion fold to reconstruct one node per ascending
//! leaf run.
//!
//! [`streams`] binds logical streams to the link's transport streams —
//! lazy opening and claiming, session-epoch labels, and the accept driver
//! that routes anonymous arrivals — and states why no scheduling layer sits
//! between a producer and its stream: each write is flushed before the
//! producer's attached question is published, and backpressure on one
//! stream is invisible to every other by the link contract.

mod adapter;
pub(crate) mod codec;
mod error;
mod proxy;
mod streams;

#[cfg(any(test, feature = "test-internals"))]
pub use codec::{FrameShape, PreparedFrame};
#[cfg(any(test, feature = "test-internals"))]
pub use codec::{HookCapture, HookStream, LinkCapture};
#[cfg(any(test, feature = "test-internals"))]
pub(crate) use codec::{assert_items_account_for, render_hook_capture, stream_label};
#[cfg(any(test, feature = "test-internals"))]
pub(crate) use codec::{decode_frame_discarded, lone_record_run, supply_frame_head};
#[cfg(any(test, feature = "test-internals"))]
pub(crate) use codec::{prepare_frame, write_prepared_frame};
pub use error::*;

/// The codec's logical stream count, for cross-layer constant assertions.
#[cfg(test)]
pub(crate) fn codec_stream_count() -> u8 {
    codec::Stream::COUNT
}
pub use codec::{DEFAULT_TARGET_MESSAGE_SIZE, RunBudget};
pub use proxy::Error;
pub use proxy::Handshaking;
