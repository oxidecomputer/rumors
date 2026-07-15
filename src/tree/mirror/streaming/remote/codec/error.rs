//! Typed codec failures with their protocol origin.

use std::fmt;

use crate::tree::mirror::framing::LengthOverflow;

use super::signal::{DecodeSignalError, InvalidSignalPlacement, Speaker, Stream};

/// The speaker and, when known, logical stream which produced an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// The direction is known, but no signal byte supplied a stream yet.
    Direction(Speaker),
    /// Both the direction and logical stream are known.
    Stream { speaker: Speaker, stream: Stream },
}

impl Origin {
    pub fn direction(speaker: Speaker) -> Self {
        Origin::Direction(speaker)
    }

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

/// The absent component of a truncated frame.
#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq)]
pub enum FramePart {
    #[error("signal byte")]
    Signal,
    #[error("query count")]
    QueryCount,
    #[error("query child listing")]
    QueryChildren,
    #[error("supply length")]
    SupplyLength,
    #[error("supply leaf")]
    SupplyLeaf,
}

/// A query listing that is not in canonical radix order.
#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq)]
#[error("query child radix {radix} does not follow {previous} in ascending order")]
pub struct QueryOrderError {
    pub previous: u8,
    pub radix: u8,
}

/// A Borsh failure while encoding a supplied leaf.
#[derive(Debug, thiserror::Error)]
pub enum EncodeLeafError {
    #[error("supplied version could not be encoded")]
    Version(#[source] borsh::io::Error),
    #[error("supplied message could not be encoded")]
    Message(#[source] borsh::io::Error),
}

/// Why an outgoing frame could not be encoded.
#[derive(Debug, thiserror::Error)]
pub enum EncodeErrorKind {
    #[error(transparent)]
    InvalidSignal(#[from] InvalidSignalPlacement),
    #[error("could not write the frame's {part}")]
    Write {
        part: FramePart,
        #[source]
        source: borsh::io::Error,
    },
    #[error("query contains {count} children, exceeding the one-byte radix fan")]
    QueryTooWide { count: usize },
    #[error(transparent)]
    QueryOutOfOrder(#[from] QueryOrderError),
    #[error(transparent)]
    InvalidLeaf(#[from] EncodeLeafError),
    #[error(
        "supplied Version ({version_len} bytes) and Message ({message_len} bytes) overflow usize"
    )]
    SupplyLengthOverflow {
        version_len: usize,
        message_len: usize,
    },
    #[error(transparent)]
    SupplyTooLarge(#[from] LengthOverflow),
}

/// An outgoing codec failure with its speaker and stream.
#[derive(Debug, thiserror::Error)]
#[error("{origin}: {kind}")]
pub struct EncodeError {
    pub origin: Origin,
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

/// A Borsh or canonicality failure while decoding a supplied leaf.
#[derive(Debug, thiserror::Error)]
pub enum DecodeLeafError {
    #[error("supplied Version could not be decoded")]
    Version(#[source] borsh::io::Error),
    #[error("supplied Message could not be decoded")]
    Message(#[source] borsh::io::Error),
    #[error("{count} trailing bytes follow the supplied Version and Message")]
    TrailingBytes { count: usize },
}

/// Why an incoming frame could not be decoded.
#[derive(Debug, thiserror::Error)]
pub enum DecodeErrorKind {
    #[error("could not read the frame's {part}")]
    Read {
        part: FramePart,
        #[source]
        source: borsh::io::Error,
    },
    #[error(transparent)]
    InvalidSignal(#[from] DecodeSignalError),
    #[error("frame ended before its {missing}")]
    Truncated {
        missing: FramePart,
        #[source]
        source: borsh::io::Error,
    },
    #[error(transparent)]
    QueryOutOfOrder(#[from] QueryOrderError),
    #[error(transparent)]
    InvalidLeaf(#[from] DecodeLeafError),
    #[error("{count} trailing bytes follow the frame")]
    TrailingBytes { count: usize },
}

/// An incoming codec failure with its known protocol origin.
#[derive(Debug, thiserror::Error)]
#[error("{origin}: {kind}")]
pub struct DecodeError {
    pub origin: Origin,
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
