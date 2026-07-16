//! Incoming routing, closure, and backpressure properties.

use futures::{StreamExt, poll};

use super::super::{DemuxError, STREAM_COUNT, incoming as build_incoming, stream_at};
use super::{SPEAKERS, encoded, ending_reply, stream_end};
use crate::tree::mirror::streaming::remote::codec::{Flow, Frame, Origin, Reaction};

/// Every interleaved incoming frame reaches only its named logical stream, and
/// the byte following the final stream end remains untouched for the caller.
#[pollster::test]
async fn routes_all_streams_and_preserves_the_trailing_boundary() {
    const SUFFIX: &[u8] = &[0xfe, 0xff];

    for speaker in SPEAKERS {
        let frames = (0..STREAM_COUNT).rev().flat_map(|index| {
            let stream = stream_at(index);
            [
                (stream, ending_reply::<()>(speaker, stream)),
                (stream, stream_end()),
            ]
        });
        let mut bytes = encoded(speaker, frames);
        bytes.extend_from_slice(SUFFIX);

        let (demux, mut incoming) = build_incoming::<_, ()>(speaker, bytes.as_slice());
        let rest = demux.run().await.unwrap();
        assert_eq!(rest, SUFFIX);

        for index in 0..STREAM_COUNT {
            let stream = stream_at(index);
            let mut frames = incoming.take(stream);
            assert_eq!(frames.next().await, Some(ending_reply(speaker, stream)));
            assert_eq!(frames.next().await, None);
        }
    }
}

/// EOF reports precisely the logical streams whose stream-end frame has not
/// arrived, without treating a transport close as normal phase completion.
#[pollster::test]
async fn reports_every_stream_left_open_at_eof() {
    for speaker in SPEAKERS {
        let closed = stream_at(STREAM_COUNT / 2);
        let bytes = encoded(speaker, [(closed, stream_end::<()>())]);
        let (demux, _incoming) = build_incoming::<_, ()>(speaker, bytes.as_slice());
        let DemuxError::PrematureEof { origin, open } = demux.run().await.unwrap_err() else {
            panic!("EOF before all stream ends must be premature");
        };
        assert_eq!(origin, Origin::direction(speaker));
        let expected = (0..STREAM_COUNT)
            .map(stream_at)
            .filter(|stream| *stream != closed)
            .collect::<Vec<_>>();
        assert_eq!(open, expected);
    }
}

/// A peer cannot reopen a logical stream after its stream-end frame.
#[pollster::test]
async fn rejects_a_frame_after_stream_end() {
    for speaker in SPEAKERS {
        let stream = stream_at(8);
        let bytes = encoded(
            speaker,
            [
                (stream, stream_end::<()>()),
                (stream, ending_reply(speaker, stream)),
            ],
        );
        let (demux, _incoming) = build_incoming::<_, ()>(speaker, bytes.as_slice());
        assert!(matches!(
            demux.run().await,
            Err(DemuxError::FrameAfterEnd { origin })
                if origin == Origin::stream(speaker, stream)
        ));
    }
}

/// Losing a protocol consumer is distinguished from peer or codec failure and
/// names the logical stream whose bounded queue can no longer make progress.
#[pollster::test]
async fn reports_a_dropped_receiver() {
    for speaker in SPEAKERS {
        let stream = stream_at(8);
        let frame = Frame::Reaction(Reaction::<()>::Match, Flow::Continue);
        let bytes = encoded(speaker, [(stream, frame)]);
        let (demux, mut incoming) = build_incoming::<_, ()>(speaker, bytes.as_slice());
        drop(incoming.take(stream));
        assert!(matches!(
            demux.run().await,
            Err(DemuxError::ReceiverDropped { origin })
                if origin == Origin::stream(speaker, stream)
        ));
    }
}

/// Stream-end control preserves dropped-consumer detection.
///
/// The peer's control must not disguise a protocol consumer which disappeared
/// before the logical stream closed.
#[pollster::test]
async fn reports_a_dropped_receiver_at_stream_end() {
    for speaker in SPEAKERS {
        let stream = stream_at(8);
        let bytes = encoded(speaker, [(stream, stream_end::<()>())]);
        let (demux, mut incoming) = build_incoming::<_, ()>(speaker, bytes.as_slice());
        drop(incoming.take(stream));
        assert!(matches!(
            demux.run().await,
            Err(DemuxError::ReceiverDropped { origin })
                if origin == Origin::stream(speaker, stream)
        ));
    }
}

/// A full logical-stream slot stops the sole demultiplexer before it can read
/// another frame, so consumer pressure propagates directly to the transport.
#[pollster::test]
async fn has_exactly_one_pending_frame_per_stream() {
    for speaker in SPEAKERS {
        let stream = stream_at(8);
        let frame = Frame::Reaction(Reaction::<()>::Match, Flow::Continue);
        let bytes = encoded(speaker, [(stream, frame.clone()), (stream, frame.clone())]);
        let (demux, mut incoming) = build_incoming::<_, ()>(speaker, bytes.as_slice());
        let mut frames = incoming.take(stream);
        let mut running = Box::pin(demux.run());

        assert!(poll!(&mut running).is_pending());
        assert_eq!(frames.next().await, Some(frame.clone()));
        let DemuxError::PrematureEof { .. } = running.await.unwrap_err() else {
            panic!("two non-ending frames followed by EOF must be premature");
        };
        assert_eq!(frames.next().await, Some(frame));
    }
}
