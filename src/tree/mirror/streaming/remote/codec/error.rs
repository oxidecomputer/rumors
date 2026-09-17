//! Typed codec failures with their protocol origin.

use std::fmt;

use crate::tree::mirror::cbor::{Head, HeadError};

use super::frame::{LeafRunError, ListingIssue};
use super::signal::{DecodeSignalError, Speaker, Stream};

/// A supplied-run length which cannot be represented by its wire head.
#[derive(Debug, thiserror::Error)]
#[error("payload length {len} exceeds the u32 framing limit")]
pub(crate) struct LengthOverflow {
    /// The unrepresentable payload length.
    pub(crate) len: usize,
    /// The failed integer conversion.
    #[source]
    pub(crate) source: std::num::TryFromIntError,
}

/// The speaker and, when known, logical stream which produced an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// The direction is known, but no signal supplied a stream yet.
    Direction(Speaker),
    /// Both the direction and logical stream are known.
    Stream {
        /// The peer direction that produced the error.
        speaker: Speaker,
        /// The logical stream that produced the error.
        stream: Stream,
    },
}

impl Origin {
    /// Record an error before its logical stream is known.
    pub(crate) fn direction(speaker: Speaker) -> Self {
        Origin::Direction(speaker)
    }

    /// Record an error from a known logical stream.
    pub(crate) fn stream(speaker: Speaker, stream: Stream) -> Self {
        Origin::Stream { speaker, stream }
    }
}

impl fmt::Display for Origin {
    /// Name the direction and, when known, its logical stream.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Origin::Direction(speaker) => write!(f, "{speaker} direction"),
            Origin::Stream { speaker, stream } => {
                write!(f, "{speaker} stream {}", stream.index())
            }
        }
    }
}

/// The absent or malformed component of a frame.
#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq)]
pub enum FramePart {
    /// The array head that begins a frame.
    #[error("frame head")]
    FrameHead,
    /// The signal that identifies the frame's operation.
    #[error("signal")]
    Signal,
    /// The child map carried by a query.
    #[error("query child listing")]
    QueryChildren,
    /// The length that begins a supplied run.
    #[error("supply run head")]
    SupplyLength,
    /// The records carried by a supplied run.
    #[error("supply run")]
    SupplyRun,
}

/// One unsigned integer in the two-item frame opener.
#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq)]
pub enum OpenerItem {
    /// The logical stream index.
    #[error("stream index")]
    Stream,
    /// The reaction state code.
    #[error("state code")]
    State,
}

/// A query listing that is not in canonical radix order.
#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq)]
#[error("query child radix {radix} does not follow {previous} in ascending order")]
pub struct QueryOrderError {
    /// The preceding radix in the listing.
    pub previous: u8,
    /// The radix that failed to increase.
    pub radix: u8,
}

/// Why an outgoing frame could not be encoded.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum EncodeErrorKind {
    /// Writing one frame component failed.
    #[error("could not write the frame's {part}")]
    Write {
        /// The component being written.
        part: FramePart,
        /// The transport error.
        #[source]
        source: std::io::Error,
    },
    /// Flushing a complete frame failed.
    #[error("could not flush the completed frame")]
    Flush(#[source] std::io::Error),
    /// A supplied run cannot be represented by the framing format.
    #[error(transparent)]
    SupplyTooLarge(#[from] LengthOverflow),
}

/// An outgoing codec failure with its speaker and stream.
#[derive(Debug, thiserror::Error)]
#[error("{origin}: {kind}")]
pub struct EncodeError {
    /// The direction and logical stream being written.
    pub origin: Origin,
    /// The encoding failure.
    #[source]
    pub kind: EncodeErrorKind,
}

impl EncodeError {
    /// Attach the frame's protocol origin to an encoding failure.
    pub(crate) fn new(speaker: Speaker, stream: Stream, kind: EncodeErrorKind) -> Self {
        Self {
            origin: Origin::stream(speaker, stream),
            kind,
        }
    }
}

/// A decode failure in one supplied record.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DecodeLeafError {
    /// The record's version is invalid.
    #[error("supplied Version could not be decoded: {0}")]
    Version(#[source] VersionDecodeError),
    /// The record's payload is invalid.
    #[error("supplied Message could not be decoded")]
    Message(#[source] std::io::Error),
}

/// Why a supplied record's version atom could not be decoded.
#[derive(Debug, thiserror::Error)]
pub enum VersionDecodeError {
    /// The version tag's CBOR head is malformed.
    #[error("version tag head is invalid: {0}")]
    TagHead(#[source] HeadError),
    /// The version atom does not begin with the version tag.
    #[error("version tag is {actual:?}; expected the version-atom tag")]
    Tag {
        /// Head found where the version tag belongs.
        actual: Head,
    },
    /// The version byte string's CBOR head is malformed.
    #[error("version byte-string head is invalid: {0}")]
    BytesHead(#[source] HeadError),
    /// The version tag does not wrap a byte string.
    #[error("version body has head {actual:?}; expected a byte string")]
    Bytes {
        /// Head found where the version byte string belongs.
        actual: Head,
    },
    /// The version's declared body cannot be addressed on this platform.
    #[error("version declares {declared} bytes, which do not fit in memory")]
    TooLarge {
        /// Byte length declared by the version atom.
        declared: u64,
    },
    /// The record ends inside the version byte string.
    #[error("version declares {declared} bytes, but only {available} remain in the record")]
    Truncated {
        /// Byte length declared by the version atom.
        declared: usize,
        /// Bytes remaining after its head.
        available: usize,
    },
    /// The complete bytes are not one valid version encoding.
    #[error("version bytes are invalid: {0}")]
    Value(#[source] before::error::Decode),
}

/// Why an incoming frame could not be decoded.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DecodeErrorKind {
    /// Reading one frame component failed.
    #[error("could not read the frame's {part}")]
    Read {
        /// The component being read.
        part: FramePart,
        /// The transport error.
        #[source]
        source: std::io::Error,
    },
    /// The frame's signal is invalid in its stream or phase.
    #[error(transparent)]
    InvalidSignal(#[from] DecodeSignalError),
    /// The frame ended before a required component.
    #[error("frame ended before its {missing}")]
    Truncated {
        /// The component that was not fully received.
        missing: FramePart,
        /// The transport error reporting the short read.
        #[source]
        source: std::io::Error,
    },
    /// A CBOR head was truncated, reserved, indefinite, or not shortest-form.
    #[error("frame's {part} head is invalid: {source}")]
    Head {
        /// The frame component whose head was invalid.
        part: FramePart,
        /// The head defect.
        #[source]
        source: HeadError,
    },
    /// A query's child listing violated its shared structural grammar.
    #[error("query child listing is invalid: {0}")]
    InvalidListing(#[source] ListingIssue),
    /// The frame item does not begin with a CBOR array.
    #[error("frame head is {actual:?}; expected an array")]
    FrameType {
        /// Head found where the frame array belongs.
        actual: Head,
    },
    /// The frame array has no valid protocol arity.
    #[error("frame array declares {declared} items; expected two or three")]
    FrameLength {
        /// Item count declared by the array.
        declared: u64,
    },
    /// The frame array's length contradicts its signal's body arity.
    #[error("frame array carries {found} item(s) where its signal takes {expected}")]
    FrameArity {
        /// The number of items required by the signal.
        expected: u64,
        /// The number of items declared by the frame.
        found: u64,
    },
    /// One opener item is not an unsigned integer.
    #[error("frame's {item} has head {actual:?}; expected an unsigned integer")]
    OpenerType {
        /// Opener item being decoded.
        item: OpenerItem,
        /// Head found where its unsigned integer belongs.
        actual: Head,
    },
    /// A query body does not begin with a child-listing map.
    #[error("query body has head {actual:?}; expected a child-listing map")]
    QueryType {
        /// Head found where the child-listing map belongs.
        actual: Head,
    },
    /// A query signal carries an empty child listing.
    #[error("a query frame carries an empty child listing; use the empty-query signal")]
    EmptyQuery,
    /// A supply body does not begin with the embedded-sequence tag.
    #[error("supply tag is {actual:?}; expected the embedded-sequence tag")]
    SupplyTag {
        /// Head found where the supply tag belongs.
        actual: Head,
    },
    /// The supply tag does not wrap a byte string.
    #[error("supply body has head {actual:?}; expected a byte string")]
    SupplyType {
        /// Head found where the supply byte string belongs.
        actual: Head,
    },
    /// The supply body exceeds the wire format's length range.
    #[error("supply run declares {declared} bytes; the wire permits at most {maximum}")]
    SupplyTooLarge {
        /// Body length declared by the byte string.
        declared: u64,
        /// Largest body length admitted by the wire.
        maximum: u32,
    },
    /// A supplied run has invalid record framing.
    #[error(transparent)]
    InvalidRun(#[from] LeafRunError),
    /// A supplied run exceeds the session's byte budget.
    #[error(
        "supply frame charges {declared} wire bytes, batching records past the {budget}-byte run budget"
    )]
    OverbatchedRun {
        /// Bytes charged to the run, including its framing allowance.
        declared: usize,
        /// Maximum charged bytes accepted for one run.
        budget: usize,
    },
    /// Bytes remain after a complete frame.
    #[cfg(test)]
    #[error("{count} trailing bytes follow the frame")]
    TrailingBytes {
        /// The unconsumed byte count.
        count: usize,
    },
}

/// An incoming codec failure with its known protocol origin.
#[derive(Debug, thiserror::Error)]
#[error("{origin}: {kind}")]
pub struct DecodeError {
    /// The known direction and logical stream.
    pub origin: Origin,
    /// The decoding failure.
    #[source]
    pub kind: DecodeErrorKind,
}

impl DecodeError {
    /// Attach a speaker when decoding failed before the stream was known.
    pub(crate) fn direction(speaker: Speaker, kind: DecodeErrorKind) -> Self {
        Self {
            origin: Origin::direction(speaker),
            kind,
        }
    }

    /// Attach the decoded speaker and stream to a frame-body failure.
    pub(crate) fn stream(speaker: Speaker, stream: Stream, kind: DecodeErrorKind) -> Self {
        Self {
            origin: Origin::stream(speaker, stream),
            kind,
        }
    }
}
