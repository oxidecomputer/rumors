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
    frame::{MAX_QUERY_CHILDREN, listing_len},
    signal::{End, Flow, Speaker, Stream},
};
use crate::tree::mirror::cbor::{MAJOR_TAG, MAJOR_UINT, TAG_CBOR_SEQUENCE};

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

/// The frame head of a `arity`-item frame carrying `code`.
fn frame_head(arity: u64, code: u8) -> Vec<u8> {
    let mut head = Vec::new();
    cbor::write_head(&mut head, MAJOR_ARRAY, arity);
    cbor::write_head(&mut head, MAJOR_UINT, u64::from(code));
    head
}

fn arb_speaker() -> impl Strategy<Value = Speaker> {
    prop_oneof![Just(Speaker::Initiator), Just(Speaker::Responder)]
}

fn arb_flow() -> impl Strategy<Value = Flow> {
    prop_oneof![Just(Flow::Continue), Just(Flow::End)]
}

/// Every query fan and flow state has one canonical map representation,
/// its length priced exactly by the listing closed form.
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
            let frame: WireFrame = (
                stream,
                Frame::Reaction(Reaction::Query(children.clone()), flow),
            );
            for speaker in SPEAKERS {
                let mut encoded = Vec::new();
                encode(speaker, &frame, &mut encoded).unwrap();
                if count == 0 {
                    assert_eq!(
                        encoded,
                        frame_head(1, signal(stream, Signal::QueryEmpty(flow)))
                    );
                } else {
                    let head = frame_head(2, signal(stream, Signal::Query(flow)));
                    assert_eq!(&encoded[..head.len()], head.as_slice());
                    assert_eq!(encoded.len(), head.len() + listing_len(&children));
                }
            }
        }
    }
}

/// Match flow and both bare ends exhaust the body-free representations:
/// each is exactly its one-item array head and signal.
#[test]
fn body_free_frames_are_exhaustive() {
    let stream = stream(4);
    let cases: Vec<(WireFrame, u8)> = vec![
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
            assert_eq!(encoded, frame_head(1, *expected));
        }
    }
}

proptest! {
    /// Supply framing is exact for an arbitrary run of backend-neutral
    /// leaf records.
    ///
    /// The layout: the run's embedded-sequence heads, then one record item
    /// per leaf, in push order — each record the tagged version atom and
    /// bare payload behind its own embedded-sequence heads.
    #[test]
    fn supplied_run_is_framed_exactly(
        index in 1_u8..Stream::MAX,
        speaker in arb_speaker(),
        flow in arb_flow(),
        records in proptest::collection::vec((arb_version(), any::<u64>()), 1..=4),
    ) {
        let stream = stream(index);
        let mut run = super::LeafRun::new();
        let mut body = Vec::new();
        for (version, value) in &records {
            let message = Message::new(*value);
            run.push(version, &message).unwrap();
            let mut content = Vec::new();
            cbor::write_head(&mut content, MAJOR_TAG, crate::tags::VERSION_TAG);
            ciborium::ser::into_writer(version, &mut content).unwrap();
            content.extend_from_slice(message.as_slice());
            cbor::write_head(&mut body, MAJOR_TAG, TAG_CBOR_SEQUENCE);
            cbor::write_head(&mut body, MAJOR_BSTR, content.len() as u64);
            body.extend_from_slice(&content);
        }
        let frame = (stream, Frame::Reaction(Reaction::Supply(run), flow));

        let mut encoded = Vec::new();
        encode(speaker, &frame, &mut encoded).unwrap();
        let mut expected = frame_head(2, signal(stream, Signal::Supply(flow)));
        cbor::write_head(&mut expected, MAJOR_TAG, TAG_CBOR_SEQUENCE);
        cbor::write_head(&mut expected, MAJOR_BSTR, body.len() as u64);
        expected.extend_from_slice(&body);
        prop_assert_eq!(encoded, expected);
    }

}

struct FailingWriter;

impl std::io::Write for FailingWriter {
    fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
        Err(std::io::ErrorKind::Other.into())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Writer failures retain their frame part, stream, and speaker.
#[test]
fn writer_errors_are_contextual() {
    let stream = stream(12);
    let frame: WireFrame = (stream, Frame::End(End::Reply));
    for speaker in SPEAKERS {
        let error = encode(speaker, &frame, &mut FailingWriter).unwrap_err();
        assert_eq!(error.origin, Origin::stream(speaker, stream));
        assert!(matches!(
            error.kind,
            EncodeErrorKind::Write {
                part: FramePart::FrameHead,
                source,
            } if source.kind() == std::io::ErrorKind::Other
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
    let frame: WireFrame = (stream, Frame::End(End::Reply));
    for speaker in SPEAKERS {
        let mut writer = FrameWrite::new(speaker, FailingAsyncWriter(AsyncFailure::Write));
        let error = pollster::block_on(writer.frame(&frame)).unwrap_err();
        assert_eq!(error.origin, Origin::stream(speaker, stream));
        assert!(matches!(
            error.kind,
            EncodeErrorKind::Write {
                part: FramePart::FrameHead,
                source,
            } if source.kind() == std::io::ErrorKind::Other
        ));

        let mut writer = FrameWrite::new(speaker, FailingAsyncWriter(AsyncFailure::Flush));
        let error = pollster::block_on(writer.frame(&frame)).unwrap_err();
        assert_eq!(error.origin, Origin::stream(speaker, stream));
        assert!(matches!(
            error.kind,
            EncodeErrorKind::Flush(source)
                if source.kind() == std::io::ErrorKind::Other
        ));
    }
}
