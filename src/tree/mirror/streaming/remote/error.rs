//! Translate reconciliation failures at the public session boundary.

use std::convert::Infallible;

use super::{codec, pipe, proxy};
use crate::{
    Error,
    error::{Context, Mismatch, Phase, ProtocolViolation, TransportError, TransportOperation},
    tree::mirror::{
        self,
        streaming::{materialized, message::CountError},
    },
};

/// A production reconciliation failure after removing impossible backend cases.
pub(crate) enum MirrorError {
    /// The local tree rejected the protocol exchange during the phase.
    Local(Phase, materialized::Violation),
    /// The local backend reported a count no node can have during the phase.
    Count(Phase, CountError),
    /// The wire-bound participant failed.
    Remote(proxy::Error<Infallible>),
}

impl MirrorError {
    /// Classify a raw production mirror failure, locating failures of the local
    /// walk, which carry no location of their own, in `phase`.
    fn new(
        error: mirror::Error<materialized::Error<Infallible>, proxy::Error<Infallible>>,
        phase: Phase,
    ) -> Self {
        match error {
            mirror::Error::Client(materialized::Error::Backend(never)) => match never {},
            mirror::Error::Client(materialized::Error::Violation(error)) => {
                Self::Local(phase, error)
            }
            mirror::Error::Client(materialized::Error::Count(error)) => Self::Count(phase, error),
            mirror::Error::Server(error) => Self::Remote(error),
        }
    }
}

/// Translate raw production mirror failures at the peer boundary, during
/// `phase`.
///
/// The phase locates the local walk's failures; the wire-bound participant's
/// carry their own location.
pub(crate) fn streaming_error(
    phase: Phase,
) -> impl FnOnce(mirror::Error<materialized::Error<Infallible>, proxy::Error<Infallible>>) -> Error
{
    move |error| MirrorError::new(error, phase).into()
}

impl From<MirrorError> for Error {
    /// Translate an internal reconciliation failure into its public cause.
    fn from(error: MirrorError) -> Self {
        use TransportOperation as Op;
        use codec::{DecodeErrorKind, EncodeErrorKind};
        use pipe::PipeError;
        let remote = match error {
            MirrorError::Local(phase, error) => return Self::violation(phase, error),
            // The only production backend, the in-memory replica, keeps
            // every count in range by construction, so a count out of range is an implementation
            // defect: a protocol violation in the public classification.
            MirrorError::Count(phase, error) => return Self::violation(phase, error),
            MirrorError::Remote(error) => error,
        };
        let (context, operation, source) = match remote {
            proxy::Error::Pipe(PipeError::Departed(source)) => {
                (Context::new(Phase::Reconciliation), Op::Read, source)
            }
            proxy::Error::PayloadDepthMismatch { local, remote } => {
                return Self::Mismatch(Mismatch::PayloadDepth { local, remote });
            }
            proxy::Error::GreetingRead(source) => (Context::new(Phase::Greeting), Op::Read, source),
            proxy::Error::GreetingWrite { operation, source } => {
                (Context::new(Phase::Greeting), operation, source)
            }
            proxy::Error::Pipe(PipeError::Decode(codec::DecodeError {
                origin,
                kind:
                    DecodeErrorKind::Read { source, .. } | DecodeErrorKind::Truncated { source, .. },
            })) => (Context::at(origin), Op::Read, source),
            proxy::Error::Pipe(PipeError::Encode(codec::EncodeError {
                origin,
                kind: EncodeErrorKind::Write { source, .. },
            })) => (Context::at(origin), Op::Write, source),
            proxy::Error::Pipe(PipeError::Encode(codec::EncodeError {
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
    use pipe::PipeError;
    match error {
        proxy::Error::GreetingDecode(_) | proxy::Error::GreetingListing(_) => {
            Context::new(Phase::Greeting)
        }
        proxy::Error::Pipe(PipeError::Decode(error)) => Context::at(error.origin),
        proxy::Error::Pipe(PipeError::Encode(error)) => Context::at(error.origin),
        proxy::Error::Pipe(PipeError::Unasked { origin } | PipeError::Unfinished { origin }) => {
            Context::at(*origin)
        }
        _ => Context::new(Phase::Reconciliation),
    }
}

#[cfg(test)]
mod tests;
