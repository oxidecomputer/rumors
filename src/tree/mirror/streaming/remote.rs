//! Wire-bound proxy for the streaming mirror.
//!
//! The submodule roles: [`codec`] owns the frame grammar; [`proxy`] the
//! session state machine and its work loops; [`adapter`] scope retention
//! and leaf reconstruction; [`streams`] the binding of logical streams to
//! the link's transport streams.
//!
//! The transport is a [`Link`](crate::link). Each direction can open up to
//! [`STREAM_COUNT`](crate::link::STREAM_COUNT) independently flow-controlled
//! data streams, established lazily as the descent needs them.
//!
//! [`codec`] defines the frame grammar. Every frame names its logical stream;
//! [`streams`] checks that name against the transport label, so a miswired
//! link fails at its first frame. The decoder also rejects signals that do not
//! belong to the speaker and phase before reading their bodies.
//!
//! Reply and stream ends are separate events. A reaction or bare `ReplyEnd`
//! completes a reply; a later bare `StreamEnd` closes the logical stream
//! ahead of the transport-level half-close. The stream layer consumes that
//! control instead of exposing it to the protocol adapter as an empty reply.
//!
//! An empty query occupies its signal alone; a nonempty query encodes its
//! child count minus one, covering every possible nonempty radix fan.
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
//! matching empty queries are answered by empty replies: pairing
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

pub(crate) use codec::STREAM_COUNT;
pub use codec::{DEFAULT_TARGET_MESSAGE_SIZE, MAX_RUN_BUDGET_BYTES, RunBudget};
pub(crate) use error::streaming_error;
pub use proxy::ControlRead;
#[cfg(test)]
pub use proxy::Error;
pub use proxy::Handshaking;
