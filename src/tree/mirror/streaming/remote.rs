//! Wire-bound proxy for the streaming mirror.
//!
//! The submodule roles: [`codec`] owns the frame grammar; [`proxy`] the
//! session state machine and its work loops; [`adapter`] scope retention
//! and leaf reconstruction; [`pipe`] the multiplexing of every logical
//! stream onto the session's one duplex byte stream.
//!
//! The transport is one reliable, ordered duplex byte stream. Each
//! direction carries the speaker's logical streams, one per level it
//! answers at, up to [`STREAM_COUNT`] of them, their frames interleaved;
//! the preamble, greeting, and closing items share the same stream before
//! and after the frames.
//!
//! [`codec`] defines the frame grammar. Every frame names its logical
//! stream, which is how [`pipe`] routes it. The decoder also rejects signals
//! that do not belong to the speaker and phase before reading their bodies.
//!
//! Reply and stream ends are separate events. A reaction or bare `ReplyEnd`
//! completes a reply; a later bare `StreamEnd` closes the logical stream.
//! The pipe consumes that control instead of exposing it to the protocol
//! adapter as an empty reply.
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
//! content — the initiator's root-fan listing — rides the greeting (see
//! [`message::Greeting`](super::message::Greeting) for the trade), so the
//! elected responder answers one hop earlier. The initiator's opening
//! logical stream therefore carries nothing; root children only the
//! initiator holds cross as ordinary supplies, answering the responder's
//! empty queries for them.
//!
//! [`adapter`] retains the question scope omitted from protocol replies. It
//! attaches each newly asked scope to the exact outgoing frame which makes the
//! question publishable, derives supplied radices from leaf content, and uses
//! the backend's existing conversion fold to reconstruct one node per ascending
//! leaf run.
//!
//! [`pipe`] states why one byte stream suffices: the receive path never
//! waits on the walk, so no level's blocked consumer can stop another
//! level's delivery.

mod adapter;
pub(crate) mod codec;
mod error;
mod pipe;
mod proxy;

#[cfg(any(test, feature = "test-internals"))]
pub use codec::{FrameShape, PreparedFrame};
#[cfg(any(test, feature = "test-internals"))]
pub use codec::{HookCapture, HookStream, PipeCapture};
#[cfg(any(test, feature = "test-internals"))]
pub(crate) use codec::{assert_items_account_for, render_hook_capture};
#[cfg(any(test, feature = "test-internals"))]
pub(crate) use codec::{decode_frame_discarded, lone_record_run, supply_frame_head};
#[cfg(any(test, feature = "test-internals"))]
pub(crate) use codec::{prepare_frame, write_prepared_frame};

pub(crate) use adapter::Scope;
pub(crate) use codec::STREAM_COUNT;
pub use codec::{DEFAULT_TARGET_MESSAGE_SIZE, MAX_RUN_BUDGET_BYTES, RunBudget};
pub(crate) use error::streaming_error;
pub use pipe::PipeRead;
#[cfg(test)]
pub use proxy::Error;
pub use proxy::Handshaking;
pub(crate) use proxy::Parked;
