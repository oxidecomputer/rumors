//! Session diagnostics independent of the protocol's implementation layers.

use std::{fmt, io};

use super::Error;
use crate::observe::Role;
use crate::tree::mirror::streaming::remote::codec;

/// The part of the `rumors` protocol in which a failure was detected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Phase {
    ///  Specifying the protocol, network, and session intent.
    Preamble,
    /// Exchanging initial versions and reconciliation settings.
    Greeting,
    /// Comparing divergences and transferring new messages.
    Reconciliation,
    /// Exchanging state to complete a network join or retirement.
    IdentityTransfer,
    /// Exchanging final confirmation that the session completed successfully.
    Completion,
}

impl fmt::Display for Phase {
    /// Describe the phase succinctly.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Preamble => "the preamble",
            Self::Greeting => "the greeting",
            Self::Reconciliation => "reconciliation",
            Self::IdentityTransfer => "identity transfer",
            Self::Completion => "completion confirmation",
        })
    }
}

/// An operation which depends on the underlying wire transport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TransportOperation {
    /// Receiving bytes from the remote peer.
    Read,
    /// Sending bytes to the remote peer.
    Write,
    /// Ensuring buffered writes are visible to the peer.
    Flush,
}

impl fmt::Display for TransportOperation {
    /// Name the failed transport operation succinctly.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Flush => "flush",
        })
    }
}

/// A logical data-stream position, when a protocol violation can be attributed
/// as such.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DataStream {
    /// The protocol role sending this stream's bytes.
    pub sender: Role,
    /// The logical stream index, if known.
    pub index: Option<u8>,
}

/// Where a session detected a failure in the logical stream structure of the
/// protocol.
///
/// `data_stream` is absent for the preamble, greeting, and closing items, and
/// for failures detected without a specified data-stream.
///
/// The error context does not definitively establish which peer or transport
/// implementation is at fault, but it may provide a hint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Context {
    /// The sequential protocol phase currently in progress.
    pub phase: Phase,
    /// The data-stream directionality and index, when known.
    pub data_stream: Option<DataStream>,
}

impl Context {
    /// Locate a failure without assigning it a data stream.
    pub(crate) fn new(phase: Phase) -> Self {
        Self {
            phase,
            data_stream: None,
        }
    }

    /// Preserve the codec's known direction and optional stream index.
    pub(crate) fn at(origin: codec::Origin) -> Self {
        let (sender, index) = match origin {
            codec::Origin::Direction(speaker) => (speaker.role(), None),
            codec::Origin::Stream { speaker, stream } => (speaker.role(), Some(stream.index())),
        };
        Self {
            phase: Phase::Reconciliation,
            data_stream: Some(DataStream { sender, index }),
        }
    }
}

impl fmt::Display for Context {
    /// Include any known stream in the diagnostic's location.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.phase)?;
        if let Some(DataStream { sender, index }) = self.data_stream {
            write!(f, ", {sender:?}")?;
            if let Some(index) = index {
                write!(f, " stream {index}")?;
            }
        }
        Ok(())
    }
}

/// An operation on the underlying wire transport failed.
///
/// Look to `source.kind()` for the I/O category, noting that a clean channel
/// closure before required bytes arrive is represented by `UnexpectedEof`.
#[derive(Debug, thiserror::Error)]
#[error("{operation} failed during {context}")]
#[non_exhaustive]
pub struct TransportError {
    /// Where the failure was detected in the protocol.
    pub context: Context,
    /// The type of operation which could not complete.
    pub operation: TransportOperation,
    /// The transport's error, or `UnexpectedEof` for a premature channel
    /// closure.
    #[source]
    pub source: io::Error,
}

/// A violation of the `rumors` protocol or one of its implementation
/// invariants.
///
/// **If this error occurs, it is *always* a bug in `rumors`**. [**Please report
/// it**](https://github.com/oxidecomputer/rumors/issues/new)!
///
/// The concrete type of the underlying error is not part of the public API;
/// applications should handle this as one failure category.
#[derive(Debug, thiserror::Error)]
#[error("protocol violation during {context}")]
pub struct ProtocolViolation {
    /// Where/when the violation was detected in the protocol.
    pub context: Context,
    /// Details to include in your bug report, including the original failure's
    /// source chain.
    #[source]
    pub source: Box<dyn std::error::Error + Send + Sync>,
}

impl Error {
    /// Locate an I/O failure at an operation outside the data frames.
    pub(crate) fn transport(
        phase: Phase,
        operation: TransportOperation,
        source: io::Error,
    ) -> Self {
        Self::Transport(TransportError {
            context: Context::new(phase),
            operation,
            source,
        })
    }

    /// Retain an implementation diagnostic as one public violation category.
    pub(crate) fn violation(
        phase: Phase,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Protocol(ProtocolViolation {
            context: Context::new(phase),
            source: Box::new(source),
        })
    }
}
