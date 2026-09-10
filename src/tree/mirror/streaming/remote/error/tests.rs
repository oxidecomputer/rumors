//! Public classification preserves a failure's provenance, not just its I/O kind.

use super::*;
use crate::error::DataStream;
use crate::tree::mirror::streaming::remote::adapter;
use codec::{
    DecodeError, DecodeErrorKind, EncodeError, EncodeErrorKind, FramePart, Origin, Speaker, Stream,
};
use proptest::prelude::*;
use std::{convert::Infallible, io};
use streams::{SendError, StreamError};

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
            (proxy::Error::Send(SendError::Connect { origin, source: source() }), TransportOperation::Open),
            (proxy::Error::Send(SendError::Label { origin, source: source() }), TransportOperation::Write),
            (proxy::Error::Send(SendError::Frame(EncodeError { origin, kind: EncodeErrorKind::Write { part: FramePart::Signal, source: source() } })), TransportOperation::Write),
            (proxy::Error::Send(SendError::Frame(EncodeError { origin, kind: EncodeErrorKind::Flush(source()) })), TransportOperation::Flush),
            (proxy::Error::Stream(StreamError::Decode(DecodeError { origin, kind: DecodeErrorKind::Read { part: FramePart::SupplyRun, source: source() } })), TransportOperation::Read),
            (proxy::Error::Stream(StreamError::Decode(DecodeError { origin, kind: DecodeErrorKind::Truncated { missing: FramePart::SupplyRun, source: source() } })), TransportOperation::Read),
            (proxy::Error::Stream(StreamError::SupplyClosed { origin, source: Some(source()) }), TransportOperation::Accept),
        ] {
            let public = Error::from(MirrorError::Server(error));
            let Error::Transport(error) = public else { prop_assert!(false, "transport was misclassified: {public:?}"); return Ok(()); };
            prop_assert_eq!(error.context, Context { phase: Phase::Reconciliation, data_stream: Some(DataStream { sender: speaker.role(), index: Some(index) }) });
            prop_assert_eq!(error.operation, operation);
            prop_assert_eq!(error.source.kind(), kind);
            prop_assert_eq!(error.source.get_ref().unwrap().downcast_ref::<Injected>().unwrap().0, marker);
        }
    }

    /// An I/O-shaped error from a complete record remains a protocol violation;
    /// diagnostics retain its concrete cause for debugging.
    #[test]
    fn complete_record_errors_are_violations(
        marker in any::<u64>(),
        kind in prop::sample::select(vec![io::ErrorKind::UnexpectedEof, io::ErrorKind::InvalidData]),
    ) {
        let remote = proxy::Error::Decode(adapter::DecodeError::Record(codec::DecodeLeafError::Version(io::Error::new(kind, Injected(marker)))));
        let public = Error::from(MirrorError::Server(remote));
        let Error::Protocol(error) = public else { prop_assert!(false, "complete record blamed on transport: {public:?}"); return Ok(()); };
        prop_assert_eq!(error.context.phase, Phase::Reconciliation);
        let remote = error.source.downcast_ref::<proxy::Error<Infallible>>().unwrap();
        let proxy::Error::Decode(adapter::DecodeError::Record(codec::DecodeLeafError::Version(source))) = remote else { unreachable!() };
        prop_assert_eq!(source.kind(), kind);
        prop_assert_eq!(source.get_ref().unwrap().downcast_ref::<Injected>().unwrap().0, marker);
    }

    /// A frame violation retains the logical stream which detected it.
    #[test]
    fn frame_violations_keep_their_location(index in 0..Stream::COUNT, initiator in any::<bool>()) {
        let speaker = if initiator { Speaker::Initiator } else { Speaker::Responder };
        let origin = Origin::stream(speaker, Stream::new(index).unwrap());
        let remote = proxy::Error::Stream(StreamError::Decode(DecodeError { origin, kind: DecodeErrorKind::FrameShape { detail: "wrong frame shape" } }));
        let Error::Protocol(error) = Error::from(MirrorError::Server(remote)) else { prop_assert!(false, "invalid frame was not a violation"); return Ok(()); };
        prop_assert_eq!(error.context.data_stream, Some(DataStream { sender: speaker.role(), index: Some(index) }));
        prop_assert!(error.source.downcast_ref::<proxy::Error<Infallible>>().is_some());
    }
}
