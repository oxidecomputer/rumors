//! The self-delimiting frame grammar shared by every logical wire stream.
//!
//! A signal byte densely encodes `(frame state, stream)` rather than imposing a
//! bit-field boundary. There are fourteen frame states — four reaction forms,
//! each continuing, ending its reply, or ending its stream and reply, plus bare
//! reply and stream ends — and 17 streams. `state * 17 + stream` occupies values
//! 0 through 237; the other 18 byte values are invalid.
//!
//! An empty query is wholly represented by its signal. A nonempty query carries
//! `count - 1` in one byte, covering 1 through 256. A supply body is the
//! backend-neutral `(Version, Message<T>)` pair behind an exact `u32` body
//! length. The codec decodes that leaf once after its whole body arrives; the
//! adapter constructs its backend-specific leaf and validates its
//! content-addressed path.

mod decode;
mod encode;
mod error;
mod frame;
mod signal;

pub use decode::{decode, decode_exact};
pub use encode::encode;
pub use error::{
    DecodeError, DecodeErrorKind, DecodeLeafError, EncodeError, EncodeErrorKind, EncodeLeafError,
    FramePart, Origin, QueryOrderError,
};
pub use frame::{Frame, Reaction, WireFrame};
pub use signal::{End, Flow, Speaker, Stream, StreamError};

#[cfg(test)]
mod tests;
