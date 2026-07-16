//! The self-delimiting frame grammar shared by every logical wire stream.
//!
//! A signal byte densely encodes `(frame state, stream)` rather than imposing a
//! bit-field boundary. There are ten frame states — four reaction forms, each
//! continuing or ending its reply, plus a bare empty-reply end and a bare
//! stream-end control — and 17 streams. `state * 17 + stream` occupies values 0
//! through 169; the other 86 byte values are reserved. Speaker and stream then
//! select a phase-specific subset: the initiator admits 161 placements and the
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
//! `count - 1` in one byte, covering 1 through 256. A supply body is the
//! backend-neutral `(Version, Message<T>)` pair behind an exact `u32` body
//! length. The codec decodes that leaf once after its whole body arrives; the
//! adapter constructs its backend-specific leaf and validates its
//! content-addressed path.
//!
//! Encoding trusts the protocol and adapter to produce phase-correct,
//! canonically ordered frames; it performs no redundant semantic validation.
//! Decoding is the trust boundary and validates every peer-controlled signal,
//! query, and supplied leaf before returning a frame. [`FrameRead`] and
//! [`FrameWrite`] apply that same grammar directly to Tokio byte streams
//! without buffering a complete outgoing frame.

mod decode;
mod encode;
mod error;
mod frame;
mod signal;

pub use decode::{FrameRead, decode, decode_exact};
pub use encode::{FrameWrite, encode};
pub use error::{
    DecodeError, DecodeErrorKind, DecodeLeafError, EncodeError, EncodeErrorKind, EncodeLeafError,
    FramePart, Origin, QueryOrderError,
};
pub use frame::{Frame, Reaction, WireFrame};
pub use signal::{
    DecodeSignalError, End, Flow, InvalidSignalPlacement, InvalidWireSignal, Speaker, Stream,
    StreamClass, StreamError,
};

#[cfg(test)]
mod tests;
