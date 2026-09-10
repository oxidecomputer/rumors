//! Session diagnostics independent of the protocol's implementation layers.

use std::{fmt, io};

use super::Error;
use crate::observe::Role;
use crate::tree::mirror::streaming::remote::codec;

/// The part of a session in which a failure was detected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Phase {
    /// Identifying the protocol, network, and session intent.
    Preamble,
    /// Exchanging versions and reconciliation settings.
    Greeting,
    /// Comparing trees and transferring their differences.
    Reconciliation,
    /// Transferring an identity during bootstrap or retirement.
    IdentityTransfer,
    /// Exchanging confirmation that the session completed.
    ///
    /// On an existing replica, all local session work has committed before
    /// this phase. A failure means the peer's commit is unconfirmed; it does
    /// not undo local changes. A donated identity can be lost if its recipient
    /// fails before accepting it.
    ///
    /// Bootstrap is the exception: the newcomer confirms completion before
    /// constructing its peer. On failure it discards the received identity
    /// and applies nothing, even if the provider has committed.
    Completion,
}

impl fmt::Display for Phase {
    /// Describe the phase without requiring knowledge of internal modules.
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

/// An operation whose progress depends on the transport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TransportOperation {
    /// Receiving bytes from the peer.
    Read,
    /// Sending bytes to the peer.
    Write,
    /// Making buffered writes visible to the peer.
    Flush,
    /// Opening an outgoing data stream.
    Open,
    /// Waiting for an incoming data stream.
    Accept,
}

impl fmt::Display for TransportOperation {
    /// Name the failed transport operation.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Flush => "flush",
            Self::Open => "open",
            Self::Accept => "accept",
        })
    }
}

/// A data-stream position, when a failure can be attributed to one direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DataStream {
    /// The elected role sending this stream's bytes.
    pub sender: Role,
    /// The logical stream index, if its label or frame identified one.
    pub index: Option<u8>,
}

/// Where a session detected a failure.
///
/// `data_stream` is absent for control traffic and for failures detected
/// without a particular data-stream direction. Context describes detection;
/// it does not establish which peer or transport implementation is at fault.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Context {
    /// The session phase in progress.
    pub phase: Phase,
    /// The data-stream direction and index, when known.
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
    /// Include a known stream in the diagnostic's location.
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

/// A transport operation failed or the peer closed before it could complete.
///
/// Inspect `source.kind()` for the I/O category. A clean close before required
/// bytes or streams arrive is represented by `UnexpectedEof`. The context and
/// original source remain available for logging; no message parsing is needed.
#[derive(Debug, thiserror::Error)]
#[error("{operation} failed during {context}: {source}")]
#[non_exhaustive]
pub struct TransportError {
    /// Where the failure was detected.
    pub context: Context,
    /// The operation which could not complete.
    pub operation: TransportOperation,
    /// The transport's error, or `UnexpectedEof` for a premature clean close.
    #[source]
    pub source: io::Error,
}

/// A violation of the protocol or one of its implementation invariants.
///
/// This indicates a bug or incompatible implementation, not an ordinary peer
/// departure. Report the context and diagnostic source. The diagnostic's
/// concrete type is not part of the public API; applications handle this as
/// one failure category rather than depending on individual grammar checks.
#[derive(Debug, thiserror::Error)]
#[error("protocol violation during {context}: {source}")]
pub struct ProtocolViolation {
    /// Where the violation was detected, with stream information when known.
    pub context: Context,
    /// Details for a bug report, including the original failure's source chain.
    #[source]
    pub source: Box<dyn std::error::Error + Send + Sync>,
}

impl Error {
    /// Locate an I/O failure at a control-stream operation.
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
