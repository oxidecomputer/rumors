//! The self-delimiting frame grammar shared by every logical wire stream.
//!
//! A signal byte densely encodes `(frame state, stream)` rather than imposing a
//! bit-field boundary. There are ten frame states — four reaction forms, each
//! continuing or ending its reply, plus a bare empty-reply end and a bare
//! stream-end control — and 17 streams. `state * 17 + stream` occupies values 0
//! through 169; the other 86 byte values are reserved. Speaker and stream then
//! select a phase-specific subset: the initiator admits 162 placements and the
//! responder 163, rejecting the rest before their frame body is read.
//!
//! Reply and stream lifetimes are deliberately orthogonal. Every nonempty
//! reply ends on its final reaction; an empty reply is one bare reply-end
//! frame. After its final reply, a producer sends a separate bare stream-end
//! control. The session demultiplexer consumes that control and closes the
//! logical receiver, so the protocol adapter sees only complete replies. This
//! lets a lazy reply stream flush each item immediately without looking ahead
//! to discover whether that item is also the stream's last.
//!
//! An empty query is wholly represented by its signal. A nonempty query carries
//! `count - 1` in one byte, covering 1 through 256. A supply body is a
//! [`LeafRun`] behind an exact `u32` body length: one or more
//! backend-neutral `(Version, Message)` leaf records, each behind its own
//! exact `u32` record length. The codec validates the run's record framing
//! once its whole body arrives but leaves the records encoded; the adapter
//! decodes them one at a time, constructs its backend-specific leaves, and
//! validates their version-derived paths. How many records share one run
//! is the sender's choice within the session's [`RunBudget`], and the
//! decoder holds arriving frames to that same budget: any within-budget
//! batching decodes, a single record of any size decodes (the encoder's
//! minimum-one-record overhang), and a frame batching multiple records past
//! the budget is rejected typed ([`DecodeErrorKind::OverbatchedRun`])
//! before its body is buffered.
//!
//! Encoding trusts the protocol and adapter to produce phase-correct,
//! canonically ordered frames; it performs no redundant semantic validation.
//! Decoding is the trust boundary and validates every peer-controlled signal,
//! query, and supply-run structure before returning a frame. [`FrameRead`]
//! and [`FrameWrite`] apply that same grammar directly to Tokio byte streams
//! without buffering a complete outgoing frame.

mod budget;
#[cfg(any(test, feature = "test-internals"))]
mod capture;
mod decode;
mod encode;
mod error;
mod frame;
mod signal;

#[cfg(test)]
pub use budget::SUPPLY_FRAME_OVERHEAD;
pub use budget::{DEFAULT_TARGET_MESSAGE_SIZE, RunBudget};

#[cfg(any(test, feature = "test-internals"))]
pub use capture::{LinkCapture, render_v2_capture};
pub use decode::FrameRead;
#[cfg(test)]
pub use decode::{decode, decode_exact};
pub use encode::FrameWrite;
#[cfg(test)]
pub use encode::encode;
pub use error::{
    DecodeError, DecodeErrorKind, DecodeLeafError, EncodeError, EncodeErrorKind, FramePart, Origin,
    QueryOrderError,
};
#[cfg(test)]
pub use frame::WireFrame;
pub use frame::{Frame, LeafRun, LeafRunError, Reaction, validate_children};
pub use signal::{
    DecodeSignalError, End, Flow, InvalidSignalPlacement, InvalidWireSignal, Speaker, Stream,
    StreamClass,
};

/// The signal byte opening one initiator-spoken, reply-ending supply frame.
///
/// The allocator meter (`tests/decode_alloc.rs`) prepends it to a hand-built
/// supply body so the codec's supply read path is drivable from outside the
/// crate.
#[cfg(any(test, feature = "test-internals"))]
pub(crate) fn supply_signal_byte() -> u8 {
    signal::WireSignal::encode(
        Stream::new(0).expect("stream 0 is within the stream range"),
        signal::Signal::Supply(Flow::End),
    )
}

/// Decode one initiator-spoken frame from `read` under `budget`, dropping
/// the decoded value.
///
/// The allocator meter (`tests/decode_alloc.rs`) drives the supply read path
/// through this to price a supply body in bytes requested from the
/// allocator; the decoded value is noise to that meter, but the typed error
/// passes through so the meter can also assert how a failure classified.
/// The budget is the meter's to choose: the framing ceiling keeps every
/// well-framed declaration on the body-read path being priced, while a
/// small budget prices the ingress gate itself.
#[cfg(any(test, feature = "test-internals"))]
pub(crate) async fn decode_frame_discarded(
    read: impl tokio::io::AsyncRead + Unpin,
    budget: RunBudget,
) -> Result<(), DecodeError> {
    let mut read = FrameRead::new(Speaker::Initiator, budget, read);
    read.frame::<u64>().await.map(|_| ())
}

#[cfg(test)]
mod tests;
