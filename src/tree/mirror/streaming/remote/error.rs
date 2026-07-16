//! Typed failures surfaced by a wire-bound streaming participant.
//!
//! [`Error`] is the protocol-facing sum. Its variants retain the typed adapter,
//! session, and codec failures below, all of which are re-exported here so a
//! caller can match a failure down to its precise cause without depending on
//! the private implementation modules.

pub use super::adapter::{DecodeError, EncodeError, OpeningError, ScopeError};
pub use super::codec::{
    DecodeError as CodecDecodeError, DecodeErrorKind as CodecDecodeErrorKind, DecodeLeafError,
    DecodeSignalError, EncodeError as CodecEncodeError, EncodeErrorKind as CodecEncodeErrorKind,
    EncodeLeafError, FramePart, InvalidSignalPlacement, InvalidWireSignal, Origin, QueryOrderError,
    Speaker, Stream, StreamClass,
};
pub use super::proxy::Error;
pub use super::session::{DemuxError, MuxError, ReplyFrameError, SendError};
pub use crate::tree::mirror::framing::LengthOverflow;
