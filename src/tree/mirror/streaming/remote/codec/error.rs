//! Typed codec failures with their protocol origin.

use std::fmt;

use crate::tree::mirror::framing::LengthOverflow;

use super::frame::LeafRunError;
use super::signal::{DecodeSignalError, Speaker, Stream};

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
    pub fn direction(speaker: Speaker) -> Self {
        Origin::Direction(speaker)
    }

    /// Record an error from a known logical stream.
    pub fn stream(speaker: Speaker, stream: Stream) -> Self {
        Origin::Stream { speaker, stream }
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Origin::Direction(speaker) => write!(f, "{speaker:?} direction"),
            Origin::Stream { speaker, stream } => {
                write!(f, "{speaker:?} stream {}", stream.index())
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
    pub fn new(speaker: Speaker, stream: Stream, kind: EncodeErrorKind) -> Self {
        Self {
            origin: Origin::stream(speaker, stream),
            kind,
        }
    }
}

/// A decode failure in one supplied record.
#[derive(Debug, thiserror::Error)]
pub enum DecodeLeafError {
    /// The record's version is invalid.
    #[error("supplied Version could not be decoded")]
    Version(#[source] std::io::Error),
    /// The record's payload is invalid.
    #[error("supplied Message could not be decoded")]
    Message(#[source] std::io::Error),
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
    /// A query's child radixes are not strictly increasing.
    #[error(transparent)]
    QueryOutOfOrder(#[from] QueryOrderError),
    /// The frame item is not a two- or three-element CBOR array.
    #[error("frame is not a CBOR reaction array: {detail}")]
    FrameShape {
        /// A concise description of the invalid shape.
        detail: &'static str,
    },
    /// The frame array's length contradicts its signal's body arity.
    #[error("frame array carries {found} item(s) where its signal takes {expected}")]
    FrameArity {
        /// The number of items required by the signal.
        expected: u64,
        /// The number of items declared by the frame.
        found: u64,
    },
    /// A frame component was present but not canonical CBOR of the
    /// expected shape.
    #[error("frame's {part} is malformed: {detail}")]
    Malformed {
        /// The malformed component.
        part: FramePart,
        /// A concise description of the defect.
        detail: &'static str,
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
    pub fn direction(speaker: Speaker, kind: DecodeErrorKind) -> Self {
        Self {
            origin: Origin::direction(speaker),
            kind,
        }
    }

    /// Attach the decoded speaker and stream to a frame-body failure.
    pub fn stream(speaker: Speaker, stream: Stream, kind: DecodeErrorKind) -> Self {
        Self {
            origin: Origin::stream(speaker, stream),
            kind,
        }
    }
}
