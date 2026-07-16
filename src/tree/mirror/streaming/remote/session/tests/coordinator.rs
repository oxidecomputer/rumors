//! Three-branch completion and causal first-error properties.

use std::{
    future,
    pin::Pin,
    task::{Context, Poll},
};

use futures::StreamExt;
use tokio::io::AsyncWrite;

use super::super::{DriveError, Drivers, MuxError, STREAM_COUNT, stream_at};
use super::{RecordingWriter, encoded, reply, reply_sequence, stream_end};
use crate::tree::mirror::streaming::remote::codec::{
    EncodeErrorKind, Flow, Frame, Origin, Reaction, Speaker, decode,
};

/// Protocol-side marker used to distinguish the coordinator's error branch.
#[derive(Debug, thiserror::Error)]
#[error("protocol marker")]
struct ProtocolError;

/// Successful coordination preserves every reply and returns all three outputs.
///
/// A representative interior stream carries several complete replies before
/// its separate stream end in both directions; the other phases carry one.
#[pollster::test]
async fn returns_all_three_outputs_after_every_branch_completes() {
    const SUFFIX: &[u8] = &[0xfa, 0xfb];

    for local in [Speaker::Initiator, Speaker::Responder] {
        let remote = local.other();
        let frames = (0..STREAM_COUNT).flat_map(|index| {
            let stream = stream_at(index);
            reply_sequence::<()>(remote, stream)
                .into_iter()
                .chain([stream_end()])
                .map(move |frame| (stream, frame))
        });
        let mut bytes = encoded(remote, frames);
        bytes.extend_from_slice(SUFFIX);
        let (drivers, mut incoming, mut outgoing) =
            Drivers::<_, _, ()>::new(local, bytes.as_slice(), RecordingWriter::default());

        let protocol = async move {
            for index in 0..STREAM_COUNT {
                let stream = stream_at(index);
                let mut frames = incoming.take(stream);
                for expected in reply_sequence(remote, stream) {
                    assert_eq!(frames.next().await, Some(expected));
                }
                assert_eq!(frames.next().await, None);
            }
            for index in 0..STREAM_COUNT {
                let stream = stream_at(index);
                let mut sender = outgoing.take(stream);
                for frame in reply_sequence(local, stream) {
                    sender
                        .frame(reply(frame))
                        .await
                        .map_err(|_| ProtocolError)?;
                }
                sender.finish().await.map_err(|_| ProtocolError)?;
            }
            Ok::<_, ProtocolError>(42)
        };

        let (output, rest, written) = drivers.run(protocol).await.unwrap();
        assert_eq!(output, 42);
        assert_eq!(rest, SUFFIX);

        let mut bytes = written.bytes.as_slice();
        for index in 0..STREAM_COUNT {
            let stream = stream_at(index);
            for frame in reply_sequence(local, stream) {
                assert_eq!(decode::<()>(local, &mut bytes).unwrap(), (stream, frame));
            }
            assert_eq!(
                decode::<()>(local, &mut bytes).unwrap(),
                (stream, stream_end())
            );
        }
        assert!(bytes.is_empty());
    }
}

/// An immediate semantic failure wins over the sender and receiver closures
/// caused by dropping the protocol's logical-stream endpoints.
#[pollster::test]
async fn protocol_error_precedes_its_channel_drop_symptoms() {
    let (drivers, incoming, outgoing) =
        Drivers::<_, _, ()>::new(Speaker::Initiator, &[][..], RecordingWriter::default());
    let protocol = async move {
        drop((incoming, outgoing));
        Err::<(), _>(ProtocolError)
    };

    assert!(matches!(
        drivers.run(protocol).await,
        Err(DriveError::Protocol(ProtocolError))
    ));
}

/// A physical input failure is returned while the protocol still owns every
/// logical endpoint, rather than being translated into a local channel close.
#[pollster::test]
async fn incoming_error_precedes_protocol_cancellation() {
    let (drivers, incoming, outgoing) =
        Drivers::<_, _, ()>::new(Speaker::Initiator, &[][..], RecordingWriter::default());
    let protocol = async move {
        let _endpoints = (incoming, outgoing);
        future::pending::<Result<(), ProtocolError>>().await
    };

    assert!(matches!(
        drivers.run(protocol).await,
        Err(DriveError::Incoming(super::super::DemuxError::PrematureEof {
            origin,
            ..
        })) if origin == Origin::direction(Speaker::Responder)
    ));
}

/// Writer which accepts bytes and fails the flush completing their frame.
#[derive(Debug)]
struct FlushFailure;

impl AsyncWrite for FlushFailure {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Ok(bytes.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Err(std::io::ErrorKind::Other.into()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

/// A physical output failure is returned in the poll which causes the paired
/// receipt to close, before the protocol can reduce it to `SendError`.
#[pollster::test]
async fn outgoing_error_precedes_receipt_close_symptom() {
    let (keep_open, read) = tokio::io::duplex(1);
    let (drivers, incoming, mut outgoing) =
        Drivers::<_, _, ()>::new(Speaker::Initiator, read, FlushFailure);
    let protocol = async move {
        let _incoming = incoming;
        let stream = stream_at(8);
        let mut sender = outgoing.take(stream);
        sender
            .frame(reply(Frame::Reaction(Reaction::Match, Flow::Continue)))
            .await
            .map_err(|_| ProtocolError)
    };

    let error = drivers.run(protocol).await.unwrap_err();
    drop(keep_open);
    assert!(matches!(
        error,
        DriveError::Outgoing(MuxError::Codec(error))
            if matches!(error.kind, EncodeErrorKind::Flush(_))
    ));
}
