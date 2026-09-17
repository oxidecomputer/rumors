//! Translate reconciliation failures at the public session boundary.

use std::{convert::Infallible, io};

use super::{codec, proxy, streams};
use crate::{
    Error,
    error::{Context, Mismatch, Phase, ProtocolViolation, TransportError, TransportOperation},
    tree::mirror::{self, streaming::materialized},
};

/// A production reconciliation failure after removing impossible backend cases.
pub(crate) enum MirrorError {
    /// The local tree rejected the protocol exchange.
    Local(materialized::Violation),
    /// The wire-bound participant failed.
    Remote(proxy::Error<Infallible>),
}

/// Remove the production mirror's uninhabited backend error.
impl From<mirror::Error<materialized::Error<Infallible>, proxy::Error<Infallible>>>
    for MirrorError
{
    /// Preserve the only two failures a production reconciliation can produce.
    fn from(
        error: mirror::Error<materialized::Error<Infallible>, proxy::Error<Infallible>>,
    ) -> Self {
        match error {
            mirror::Error::Client(materialized::Error::Backend(never)) => match never {},
            mirror::Error::Client(materialized::Error::Violation(error)) => Self::Local(error),
            mirror::Error::Server(error) => Self::Remote(error),
        }
    }
}

/// Translate the raw production mirror result at the peer boundary.
pub(crate) fn streaming_error(
    error: mirror::Error<materialized::Error<Infallible>, proxy::Error<Infallible>>,
) -> Error {
    MirrorError::from(error).into()
}

impl From<MirrorError> for Error {
    /// Translate an internal reconciliation failure into its public cause.
    fn from(error: MirrorError) -> Self {
        use TransportOperation as Op;
        use codec::{DecodeErrorKind, EncodeErrorKind};
        use streams::{SendError, StreamError};
        let remote = match error {
            MirrorError::Local(error) => {
                return Self::violation(Phase::Reconciliation, error);
            }
            MirrorError::Remote(error) => error,
        };
        let (context, operation, source) = match remote {
            proxy::Error::PeerDeparted(source) => {
                (Context::new(Phase::Reconciliation), Op::Read, source)
            }
            proxy::Error::PayloadDepthMismatch { local, remote } => {
                return Self::Mismatch(Mismatch::PayloadDepth { local, remote });
            }
            proxy::Error::HandshakeRead(source) => {
                (Context::new(Phase::Greeting), Op::Read, source)
            }
            proxy::Error::HandshakeWrite { operation, source } => {
                (Context::new(Phase::Greeting), operation, source)
            }
            proxy::Error::Stream(StreamError::Truncated { origin }) => (
                Context::at(origin),
                Op::Read,
                io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "data stream ended before its completion marker",
                ),
            ),
            proxy::Error::Stream(StreamError::SupplyClosed { origin, source }) => (
                Context::at(origin),
                Op::Accept,
                source.unwrap_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "incoming stream supply closed",
                    )
                }),
            ),
            proxy::Error::Stream(StreamError::Decode(codec::DecodeError {
                origin,
                kind:
                    DecodeErrorKind::Read { source, .. } | DecodeErrorKind::Truncated { source, .. },
            })) => (Context::at(origin), Op::Read, source),
            proxy::Error::Send(SendError::Connect { origin, source }) => {
                (Context::at(origin), Op::Open, source)
            }
            proxy::Error::Send(SendError::Label { origin, source }) => {
                (Context::at(origin), Op::Write, source)
            }
            proxy::Error::Send(SendError::Frame(codec::EncodeError {
                origin,
                kind: EncodeErrorKind::Write { source, .. },
            })) => (Context::at(origin), Op::Write, source),
            proxy::Error::Send(SendError::Frame(codec::EncodeError {
                origin,
                kind: EncodeErrorKind::Flush(source),
            })) => (Context::at(origin), Op::Flush, source),
            error => {
                let context = violation_context(&error);
                return Self::Protocol(ProtocolViolation {
                    context,
                    source: Box::new(error),
                });
            }
        };
        Self::Transport(TransportError {
            context,
            operation,
            source,
        })
    }
}

/// Locate a violation without exposing the proxy's error hierarchy.
fn violation_context(error: &proxy::Error<std::convert::Infallible>) -> Context {
    use streams::{AcceptError, SendError, StreamError};
    match error {
        proxy::Error::HandshakeDecode(_) | proxy::Error::HandshakeListing(_) => {
            Context::new(Phase::Greeting)
        }
        proxy::Error::Stream(StreamError::Mislabeled { origin, .. }) => Context::at(*origin),
        proxy::Error::Stream(StreamError::Decode(error)) => Context::at(error.origin),
        proxy::Error::Send(SendError::Frame(error)) => Context::at(error.origin),
        proxy::Error::Accept(
            AcceptError::Epoch { origin, .. }
            | AcceptError::UnknownStream { origin, .. }
            | AcceptError::Label { origin, .. }
            | AcceptError::Duplicate { origin }
            | AcceptError::Unexpected { origin },
        ) => Context::at(*origin),
        _ => Context::new(Phase::Reconciliation),
    }
}

#[cfg(test)]
mod tests;
