//! Public classification preserves a failure's provenance, not just its I/O kind.

use super::*;
use crate::error::DataStream;
use crate::tree::mirror::streaming::remote::adapter;
use codec::{
    DecodeError, DecodeErrorKind, EncodeError, EncodeErrorKind, FramePart, Origin, Speaker, Stream,
};
use pipe::PipeError;
use proptest::prelude::*;
use std::{convert::Infallible, io};

/// A distinguishable transport source retained through public conversion.
#[derive(Debug, thiserror::Error)]
#[error("injected failure {0}")]
struct Injected(u64);

proptest! {
    /// Every stream and I/O category keeps its direction, operation, and original
    /// source when a transport failure crosses the public boundary.
    #[test]
    fn transport_provenance_survives(
        index in 0..Stream::COUNT, initiator in any::<bool>(), marker in any::<u64>(),
        kind in prop::sample::select(vec![io::ErrorKind::BrokenPipe, io::ErrorKind::UnexpectedEof, io::ErrorKind::InvalidData]),
    ) {
        let speaker = if initiator { Speaker::Initiator } else { Speaker::Responder };
        let origin = Origin::stream(speaker, Stream::new(index).unwrap());
        let source = || io::Error::new(kind, Injected(marker));
        for (error, operation) in [
            (proxy::Error::Pipe(PipeError::Encode(EncodeError { origin, kind: EncodeErrorKind::Write { part: FramePart::Signal, source: source() } })), TransportOperation::Write),
            (proxy::Error::Pipe(PipeError::Encode(EncodeError { origin, kind: EncodeErrorKind::Flush(source()) })), TransportOperation::Flush),
            (proxy::Error::Pipe(PipeError::Decode(DecodeError { origin, kind: DecodeErrorKind::Read { part: FramePart::SupplyRun, source: source() } })), TransportOperation::Read),
            (proxy::Error::Pipe(PipeError::Decode(DecodeError { origin, kind: DecodeErrorKind::Truncated { missing: FramePart::SupplyRun, source: source() } })), TransportOperation::Read),
        ] {
            let public = Error::from(MirrorError::Remote(error));
            let Error::Transport(error) = public else { prop_assert!(false, "transport was misclassified: {public:?}"); return Ok(()); };
            prop_assert_eq!(error.context, Context { phase: Phase::Reconciliation, data_stream: Some(DataStream { sender: speaker.role(), index: Some(index) }) });
            prop_assert_eq!(error.operation, operation);
            prop_assert_eq!(error.source.kind(), kind);
            prop_assert_eq!(error.source.get_ref().unwrap().downcast_ref::<Injected>().unwrap().0, marker);
        }
    }

    /// A pipe that ends or fails between frames keeps its source and is
    /// attributed to reconciliation, without inventing a data-stream
    /// position.
    #[test]
    fn departure_keeps_its_transport_source(
        marker in any::<u64>(),
        kind in prop::sample::select(vec![io::ErrorKind::UnexpectedEof, io::ErrorKind::ConnectionReset]),
    ) {
        let public = Error::from(MirrorError::Remote(proxy::Error::Pipe(PipeError::Departed(io::Error::new(kind, Injected(marker))))));
        let Error::Transport(error) = public else { panic!("departure must be a transport failure"); };
        prop_assert_eq!(error.context.phase, Phase::Reconciliation);
        prop_assert_eq!(error.context.data_stream, None);
        prop_assert_eq!(error.operation, TransportOperation::Read);
        prop_assert_eq!(error.source.kind(), kind);
        prop_assert_eq!(error.source.get_ref().unwrap().downcast_ref::<Injected>().unwrap().0, marker);
    }

    /// An I/O-shaped error from a complete record remains a protocol violation;
    /// diagnostics retain its concrete cause for debugging.
    #[test]
    fn complete_record_errors_are_violations(
        marker in any::<u64>(),
        kind in prop::sample::select(vec![io::ErrorKind::UnexpectedEof, io::ErrorKind::InvalidData]),
    ) {
        let remote = proxy::Error::Decode(adapter::DecodeError::Record(codec::DecodeLeafError::Message(io::Error::new(kind, Injected(marker)))));
        let public = Error::from(MirrorError::Remote(remote));
        let Error::Protocol(error) = public else { prop_assert!(false, "complete record blamed on transport: {public:?}"); return Ok(()); };
        prop_assert_eq!(error.context.phase, Phase::Reconciliation);
        let remote = error.source.downcast_ref::<proxy::Error<Infallible>>().unwrap();
        let proxy::Error::Decode(adapter::DecodeError::Record(codec::DecodeLeafError::Message(source))) = remote else { unreachable!() };
        prop_assert_eq!(source.kind(), kind);
        prop_assert_eq!(source.get_ref().unwrap().downcast_ref::<Injected>().unwrap().0, marker);
    }

    /// A frame violation retains the logical stream which detected it.
    #[test]
    fn frame_violations_keep_their_location(index in 0..Stream::COUNT, initiator in any::<bool>()) {
        let speaker = if initiator { Speaker::Initiator } else { Speaker::Responder };
        let origin = Origin::stream(speaker, Stream::new(index).unwrap());
        let remote = proxy::Error::Pipe(PipeError::Decode(DecodeError {
            origin,
            kind: DecodeErrorKind::FrameType {
                actual: crate::tree::mirror::cbor::Head { major: 0, value: 0 },
            },
        }));
        let Error::Protocol(error) = Error::from(MirrorError::Remote(remote)) else { prop_assert!(false, "invalid frame was not a violation"); return Ok(()); };
        prop_assert_eq!(error.context.data_stream, Some(DataStream { sender: speaker.role(), index: Some(index) }));
        prop_assert!(error.source.downcast_ref::<proxy::Error<Infallible>>().is_some());
    }
}

/// Generates a phase a local walk failure can arise in: the greeting, whose
/// listing the walk builds, or reconciliation.
fn arb_walk_phase() -> impl Strategy<Value = Phase> {
    prop::sample::select(vec![Phase::Greeting, Phase::Reconciliation])
}

/// Generates a count no node can have, of either kind.
fn arb_count_error() -> impl Strategy<Value = CountError> {
    prop_oneof![
        (257..=usize::MAX).prop_map(CountError::Children),
        (65_537..=usize::MAX).prop_map(CountError::Grandchildren),
    ]
}

proptest! {
    /// A count the local backend reports out of range crosses the public
    /// boundary as a protocol violation with the count as its source.
    ///
    /// It is located in the phase its call site names, and a violation the
    /// local walk detects is located the same way.
    #[test]
    fn local_walk_failures_keep_their_phase_and_source(
        phase in arb_walk_phase(),
        count in arb_count_error(),
    ) {
        let public = streaming_error(phase)(mirror::Error::Client(materialized::Error::Count(count)));
        let Error::Protocol(error) = public else { prop_assert!(false, "a count error was misclassified: {public:?}"); return Ok(()); };
        prop_assert_eq!(error.context, Context::new(phase));
        prop_assert_eq!(error.source.downcast_ref::<CountError>(), Some(&count));

        let violation = materialized::Violation::UnexpectedQuery;
        let public = streaming_error(phase)(mirror::Error::Client(materialized::Error::Violation(violation)));
        let Error::Protocol(error) = public else { prop_assert!(false, "a local violation was misclassified: {public:?}"); return Ok(()); };
        prop_assert_eq!(error.context, Context::new(phase));
        prop_assert_eq!(error.source.downcast_ref::<materialized::Violation>(), Some(&violation));
    }
}
