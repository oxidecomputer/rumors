use borsh::BorshSerialize;
use proptest::prelude::*;
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::AsyncWrite;

use super::*;
use crate::{
    message::Message,
    tree::{
        arb::arb_version,
        typed::{Hash, hash::MERKLE_HASH_LEN},
    },
};

use super::super::{
    error::Origin,
    frame::{MAX_QUERY_CHILDREN, QUERY_CHILD_LEN, QUERY_COUNT_BIAS, QUERY_COUNT_LEN},
    signal::{End, Flow, Speaker, Stream},
};

const SPEAKERS: [Speaker; 2] = [Speaker::Initiator, Speaker::Responder];
const FLOWS: [Flow; 2] = [Flow::Continue, Flow::End];

fn stream(index: u8) -> Stream {
    Stream::new(index).unwrap()
}

fn signal(stream: Stream, signal: Signal) -> u8 {
    WireSignal::new(Speaker::Initiator, stream, signal)
        .unwrap()
        .to_byte()
}

fn arb_speaker() -> impl Strategy<Value = Speaker> {
    prop_oneof![Just(Speaker::Initiator), Just(Speaker::Responder)]
}

fn arb_flow() -> impl Strategy<Value = Flow> {
    prop_oneof![Just(Flow::Continue), Just(Flow::End)]
}

/// Every query fan and flow state has one canonical count representation.
#[test]
fn query_count_covers_every_fan_and_flow() {
    let stream = stream(7);
    for count in 0..=MAX_QUERY_CHILDREN {
        let children = (0..count)
            .map(|radix| {
                let radix = radix as u8;
                (radix, Hash([radix; MERKLE_HASH_LEN]))
            })
            .collect::<Vec<_>>();
        for flow in FLOWS {
            let frame: WireFrame<u64> = (
                stream,
                Frame::Reaction(Reaction::Query(children.clone()), flow),
            );
            for speaker in SPEAKERS {
                let mut encoded = Vec::new();
                encode(speaker, &frame, &mut encoded).unwrap();
                if count == 0 {
                    assert_eq!(encoded, [signal(stream, Signal::QueryEmpty(flow))]);
                } else {
                    assert_eq!(encoded[0], signal(stream, Signal::Query(flow)));
                    assert_eq!(encoded[1], (count - QUERY_COUNT_BIAS) as u8);
                    assert_eq!(
                        encoded.len(),
                        WireSignal::ENCODED_LEN + QUERY_COUNT_LEN + count * QUERY_CHILD_LEN
                    );
                }
            }
        }
    }
}

/// Match flow and both bare ends exhaust their one-byte representations.
#[test]
fn one_byte_frames_are_exhaustive() {
    let stream = stream(4);
    let cases: Vec<(WireFrame<u64>, u8)> = vec![
        (
            (stream, Frame::Reaction(Reaction::Match, Flow::Continue)),
            signal(stream, Signal::Match(Flow::Continue)),
        ),
        (
            (stream, Frame::Reaction(Reaction::Match, Flow::End)),
            signal(stream, Signal::Match(Flow::End)),
        ),
        (
            (stream, Frame::End(End::Reply)),
            signal(stream, Signal::End(End::Reply)),
        ),
        (
            (stream, Frame::End(End::Stream)),
            signal(stream, Signal::End(End::Stream)),
        ),
    ];
    for speaker in SPEAKERS {
        for (frame, expected) in &cases {
            let mut encoded = Vec::new();
            encode(speaker, frame, &mut encoded).unwrap();
            assert_eq!(encoded, [*expected]);
        }
    }
}

proptest! {
    /// Supply framing is exact for arbitrary backend-neutral leaves.
    #[test]
    fn supplied_leaf_is_framed_exactly(
        index in 1_u8..Stream::MAX,
        speaker in arb_speaker(),
        flow in arb_flow(),
        version in arb_version(),
        value in any::<u64>(),
    ) {
        let stream = stream(index);
        let message = Message::new(value);
        let frame = (
            stream,
            Frame::Reaction(
                Reaction::Supply(version.clone(), message.clone()),
                flow,
            ),
        );

        let mut encoded = Vec::new();
        encode(speaker, &frame, &mut encoded).unwrap();
        let mut expected = vec![signal(stream, Signal::Supply(flow))];
        let mut body = Vec::new();
        version.serialize(&mut body).unwrap();
        message.serialize(&mut body).unwrap();
        expected.extend_from_slice(&(body.len() as u32).to_be_bytes());
        expected.extend_from_slice(&body);
        prop_assert_eq!(encoded, expected);
    }

}

struct FailingWriter;

impl borsh::io::Write for FailingWriter {
    fn write(&mut self, _buf: &[u8]) -> borsh::io::Result<usize> {
        Err(borsh::io::ErrorKind::Other.into())
    }

    fn flush(&mut self) -> borsh::io::Result<()> {
        Ok(())
    }
}

/// Writer failures retain their frame part, stream, and speaker.
#[test]
fn writer_errors_are_contextual() {
    let stream = stream(12);
    let frame: WireFrame<()> = (stream, Frame::End(End::Reply));
    for speaker in SPEAKERS {
        let error = encode(speaker, &frame, &mut FailingWriter).unwrap_err();
        assert_eq!(error.origin, Origin::stream(speaker, stream));
        assert!(matches!(
            error.kind,
            EncodeErrorKind::Write {
                part: FramePart::Signal,
                source,
            } if source.kind() == borsh::io::ErrorKind::Other
        ));
    }
}

#[derive(Clone, Copy)]
enum AsyncFailure {
    Write,
    Flush,
}

struct FailingAsyncWriter(AsyncFailure);

impl AsyncWrite for FailingAsyncWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.0 {
            AsyncFailure::Write => Poll::Ready(Err(std::io::ErrorKind::Other.into())),
            AsyncFailure::Flush => Poll::Ready(Ok(bytes.len())),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.0 {
            AsyncFailure::Write => Poll::Ready(Ok(())),
            AsyncFailure::Flush => Poll::Ready(Err(std::io::ErrorKind::Other.into())),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

/// Async write and flush failures retain their exact operation, stream, and speaker.
#[test]
fn async_writer_errors_are_contextual() {
    let stream = stream(12);
    let frame: WireFrame<()> = (stream, Frame::End(End::Reply));
    for speaker in SPEAKERS {
        let mut writer = FrameWrite::new(speaker, FailingAsyncWriter(AsyncFailure::Write));
        let error = pollster::block_on(writer.frame(&frame)).unwrap_err();
        assert_eq!(error.origin, Origin::stream(speaker, stream));
        assert!(matches!(
            error.kind,
            EncodeErrorKind::Write {
                part: FramePart::Signal,
                source,
            } if source.kind() == borsh::io::ErrorKind::Other
        ));

        let mut writer = FrameWrite::new(speaker, FailingAsyncWriter(AsyncFailure::Flush));
        let error = pollster::block_on(writer.frame(&frame)).unwrap_err();
        assert_eq!(error.origin, Origin::stream(speaker, stream));
        assert!(matches!(
            error.kind,
            EncodeErrorKind::Flush(source)
                if source.kind() == borsh::io::ErrorKind::Other
        ));
    }
}
