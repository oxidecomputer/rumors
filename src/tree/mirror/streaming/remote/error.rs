//! Typed failures surfaced by a wire-bound streaming participant.
//!
//! [`RemoteError`] is the protocol-facing sum. Its variants retain the typed
//! adapter, stream-layer, and codec failures below, all of which are
//! re-exported here so a caller can match a failure down to its precise cause
//! without depending on the private implementation modules.

pub use super::adapter::{
    DecodeError as ReplyDecodeError, EncodeError as ReplyEncodeError, OpeningError, ScopeError,
};
pub use super::codec::{
    DecodeError as CodecDecodeError, DecodeErrorKind as CodecDecodeErrorKind, DecodeLeafError,
    DecodeSignalError, EncodeError as CodecEncodeError, EncodeErrorKind as CodecEncodeErrorKind,
    FramePart, GreetingError, HeadError, InvalidSignalPlacement, InvalidWireSignal, LeafRunError,
    ListingIssue, Origin, QueryOrderError, Speaker, Stream, StreamClass,
};
pub use super::proxy::Error as RemoteError;
pub use super::streams::{AcceptError, ReplyFrameError, SendError, StreamError};
pub use crate::tree::mirror::framing::LengthOverflow;
