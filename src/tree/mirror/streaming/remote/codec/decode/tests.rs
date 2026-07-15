use borsh::BorshSerialize;
use proptest::prelude::*;

use super::*;
use crate::tree::arb::arb_version;

use super::super::{
    error::{Origin, QueryOrderError},
    frame::{QUERY_COUNT_BIAS, QUERY_COUNT_LEN},
    signal::{DecodeSignalError, End, Flow, Speaker, Stream, StreamError},
};

const SPEAKERS: [Speaker; 2] = [Speaker::Initiator, Speaker::Responder];

/// A one-byte prefix of a Version whose gamma integer is incomplete.
const TRUNCATED_VERSION: &[u8] = &[1];

fn stream(index: u8) -> Stream {
    Stream::new(index).unwrap()
}

fn signal(stream: Stream, signal: Signal) -> u8 {
    WireSignal::new(Speaker::Initiator, stream, signal)
        .unwrap()
        .to_byte()
}

fn supply(stream: Stream, flow: Flow, body: &[u8]) -> Vec<u8> {
    let mut encoded = vec![signal(stream, Signal::Supply(flow))];
    encoded.extend_from_slice(&(body.len() as u32).to_be_bytes());
    encoded.extend_from_slice(body);
    encoded
}

fn arb_speaker() -> impl Strategy<Value = Speaker> {
    prop_oneof![Just(Speaker::Initiator), Just(Speaker::Responder)]
}

fn arb_flow() -> impl Strategy<Value = Flow> {
    prop_oneof![Just(Flow::Continue), Just(Flow::End)]
}

/// Reserved signal states retain the stream encoded alongside them.
#[test]
fn invalid_signals_are_rejected() {
    assert_eq!(
        Stream::new(Stream::COUNT),
        Err(StreamError::Invalid {
            index: Stream::COUNT
        })
    );
    for byte in WireSignal::BYTE_COUNT..=u8::MAX {
        for speaker in SPEAKERS {
            let invalid = WireSignal::from_byte(speaker, byte).unwrap_err();
            let DecodeSignalError::Reserved(reserved) = invalid else {
                panic!("unexpected signal error")
            };
            let error = decode_exact::<u64>(speaker, &[byte]).unwrap_err();
            assert_eq!(error.origin, Origin::stream(speaker, reserved.stream()));
            let DecodeErrorKind::InvalidSignal(DecodeSignalError::Reserved(source)) = error.kind
            else {
                panic!("unexpected error kind");
            };
            assert_eq!(source, reserved);
            assert_eq!(source.byte(), byte);
            assert_eq!(source.state(), byte / Stream::COUNT);
            assert!(std::error::Error::source(&source).is_some());
        }
    }
}

/// Truncation identifies both the absent component and its known origin.
#[test]
fn truncated_bodies_are_rejected() {
    let stream = stream(4);
    for speaker in SPEAKERS {
        let cases = [
            (Vec::new(), FramePart::Signal, Origin::direction(speaker)),
            (
                vec![signal(stream, Signal::Query(Flow::Continue))],
                FramePart::QueryCount,
                Origin::stream(speaker, stream),
            ),
            (
                vec![signal(stream, Signal::Query(Flow::Continue)), u8::MIN],
                FramePart::QueryChildren,
                Origin::stream(speaker, stream),
            ),
            (
                vec![signal(stream, Signal::Supply(Flow::Continue))],
                FramePart::SupplyLength,
                Origin::stream(speaker, stream),
            ),
            (
                {
                    let mut frame = vec![signal(stream, Signal::Supply(Flow::Continue))];
                    frame.extend_from_slice(&1_u32.to_be_bytes());
                    frame
                },
                FramePart::SupplyLeaf,
                Origin::stream(speaker, stream),
            ),
        ];
        for (encoded, missing, origin) in cases {
            let error = decode_exact::<u64>(speaker, &encoded).unwrap_err();
            assert_eq!(error.origin, origin);
            let DecodeErrorKind::Truncated {
                missing: actual,
                source,
            } = error.kind
            else {
                panic!("unexpected error kind");
            };
            assert_eq!(actual, missing);
            assert_eq!(source.kind(), borsh::io::ErrorKind::UnexpectedEof);
        }
    }
}

proptest! {
    /// Arbitrary supplied leaves decode once into their backend-neutral pair.
    #[test]
    fn supplied_leaf_is_decoded_immediately(
        index in 1_u8..Stream::MAX,
        speaker in arb_speaker(),
        flow in arb_flow(),
        version in arb_version(),
        value in any::<u64>(),
    ) {
        let stream = stream(index);
        let message = Message::new(value);
        let mut body = Vec::new();
        version.serialize(&mut body).unwrap();
        message.serialize(&mut body).unwrap();
        let encoded = supply(stream, flow, &body);

        prop_assert_eq!(
            decode_exact::<u64>(speaker, &encoded).unwrap(),
            (
                stream,
                Frame::Reaction(Reaction::Supply(version, message), flow)
            )
        );
    }
}

/// Leaf failures identify their component and retain the Borsh source error.
#[test]
fn supplied_leaf_errors_are_typed() {
    let stream = stream(8);
    for speaker in SPEAKERS {
        let invalid_version =
            decode_exact::<u64>(speaker, &supply(stream, Flow::Continue, TRUNCATED_VERSION))
                .unwrap_err();
        assert_eq!(invalid_version.origin, Origin::stream(speaker, stream));
        let DecodeErrorKind::InvalidLeaf(DecodeLeafError::Version(source)) = invalid_version.kind
        else {
            panic!("unexpected error kind");
        };
        assert_eq!(source.kind(), borsh::io::ErrorKind::UnexpectedEof);

        let mut version = Vec::new();
        Version::new().serialize(&mut version).unwrap();
        let invalid_message =
            decode_exact::<u64>(speaker, &supply(stream, Flow::Continue, &version)).unwrap_err();
        assert_eq!(invalid_message.origin, Origin::stream(speaker, stream));
        let DecodeErrorKind::InvalidLeaf(DecodeLeafError::Message(source)) = invalid_message.kind
        else {
            panic!("unexpected error kind");
        };
        assert_eq!(source.kind(), borsh::io::ErrorKind::InvalidData);

        0_u64.serialize(&mut version).unwrap();
        version.push(u8::MIN);
        let trailing =
            decode_exact::<u64>(speaker, &supply(stream, Flow::Continue, &version)).unwrap_err();
        assert_eq!(trailing.origin, Origin::stream(speaker, stream));
        assert!(matches!(
            trailing.kind,
            DecodeErrorKind::InvalidLeaf(DecodeLeafError::TrailingBytes {
                count: WireSignal::ENCODED_LEN
            })
        ));
    }
}

proptest! {
    /// Every adjacent non-ascending pair reports its values and origin.
    #[test]
    fn unordered_query_is_rejected(
        index in 1_u8..Stream::MAX,
        speaker in arb_speaker(),
        previous in any::<u8>(),
        radix in any::<u8>(),
    ) {
        prop_assume!(previous >= radix);
        let stream = stream(index);
        let children = vec![(previous, Hash::default()), (radix, Hash::default())];
        let encoded_count = u8::try_from(children.len() - QUERY_COUNT_BIAS).unwrap();
        let mut encoded = Vec::with_capacity(WireSignal::ENCODED_LEN + QUERY_COUNT_LEN);
        encoded.extend_from_slice(&[
            signal(stream, Signal::Query(Flow::Continue)),
            encoded_count,
        ]);
        for (radix, hash) in &children {
            encoded.push(*radix);
            encoded.extend_from_slice(hash.as_bytes());
        }
        let error = decode_exact::<u64>(speaker, &encoded).unwrap_err();
        prop_assert_eq!(error.origin, Origin::stream(speaker, stream));
        let correct = matches!(
            error.kind,
            DecodeErrorKind::QueryOutOfOrder(QueryOrderError {
                previous: actual_previous,
                radix: actual_radix,
            }) if actual_previous == previous && actual_radix == radix
        );
        prop_assert!(correct);
    }
}

/// Exact decoding rejects a trailing frame while incremental decoding preserves it.
#[test]
fn exact_decode_rejects_trailing_frame() {
    let stream = stream(10);
    let first = signal(stream, Signal::Match(Flow::Continue));
    let second = signal(stream, Signal::End(End::Reply));
    let encoded = [first, second];
    for speaker in SPEAKERS {
        let error = decode_exact::<u64>(speaker, &encoded).unwrap_err();
        assert_eq!(error.origin, Origin::stream(speaker, stream));
        assert!(matches!(
            error.kind,
            DecodeErrorKind::TrailingBytes {
                count: WireSignal::ENCODED_LEN
            }
        ));

        let mut rest = encoded.as_slice();
        let frame = decode::<u64>(speaker, &mut rest).unwrap();
        assert_eq!(
            frame,
            (stream, Frame::Reaction(Reaction::Match, Flow::Continue))
        );
        assert_eq!(rest, &[second]);
    }
}

/// Async EOF is clean only before a signal; every partial body reports the
/// same missing part and stream context as synchronous decoding.
#[test]
fn async_eof_distinguishes_close_from_truncation() {
    let stream = stream(4);
    for speaker in SPEAKERS {
        let mut closed = FrameRead::new(speaker, &[][..]);
        assert_eq!(pollster::block_on(closed.frame::<u64>()).unwrap(), None);

        let cases = [
            (
                vec![signal(stream, Signal::Query(Flow::Continue))],
                FramePart::QueryCount,
            ),
            (
                vec![signal(stream, Signal::Query(Flow::Continue)), u8::MIN],
                FramePart::QueryChildren,
            ),
            (
                vec![signal(stream, Signal::Supply(Flow::Continue))],
                FramePart::SupplyLength,
            ),
            (
                {
                    let mut frame = vec![signal(stream, Signal::Supply(Flow::Continue))];
                    frame.extend_from_slice(&1_u32.to_be_bytes());
                    frame
                },
                FramePart::SupplyLeaf,
            ),
        ];
        for (encoded, missing) in cases {
            let mut reader = FrameRead::new(speaker, encoded.as_slice());
            let error = pollster::block_on(reader.frame::<u64>()).unwrap_err();
            assert_eq!(error.origin, Origin::stream(speaker, stream));
            assert!(matches!(
                error.kind,
                DecodeErrorKind::Truncated {
                    missing: actual,
                    source,
                } if actual == missing && source.kind() == borsh::io::ErrorKind::UnexpectedEof
            ));
        }
    }
}

/// An invalid async signal consumes only itself, leaving the following valid
/// frame at the next exact boundary.
#[test]
fn async_invalid_signal_does_not_consume_a_body() {
    for speaker in SPEAKERS {
        let (stream, other, invalid_signal, valid_signal, valid_frame) = match speaker {
            Speaker::Initiator => (
                stream(0),
                Speaker::Responder,
                Signal::Match(Flow::Continue),
                Signal::QueryEmpty(Flow::End),
                Frame::Reaction(Reaction::Query(Vec::new()), Flow::End),
            ),
            Speaker::Responder => (
                stream(Stream::MAX),
                Speaker::Initiator,
                Signal::Match(Flow::Continue),
                Signal::End(End::Reply),
                Frame::End(End::Reply),
            ),
        };
        let invalid = WireSignal::new(other, stream, invalid_signal)
            .unwrap()
            .to_byte();
        let valid = WireSignal::new(speaker, stream, valid_signal)
            .unwrap()
            .to_byte();
        let bytes = [invalid, valid];
        let mut reader = FrameRead::new(speaker, bytes.as_slice());

        let error = pollster::block_on(reader.frame::<u64>()).unwrap_err();
        assert_eq!(error.origin, Origin::stream(speaker, stream));
        assert!(matches!(
            error.kind,
            DecodeErrorKind::InvalidSignal(DecodeSignalError::Placement(_))
        ));
        assert_eq!(
            pollster::block_on(reader.frame::<u64>()).unwrap(),
            Some((stream, valid_frame)),
        );
    }
}

struct FailingReader;

impl borsh::io::Read for FailingReader {
    fn read(&mut self, _buf: &mut [u8]) -> borsh::io::Result<usize> {
        Err(borsh::io::ErrorKind::Other.into())
    }
}

/// Reader failures before the signal retain their frame part and speaker.
#[test]
fn reader_errors_are_contextual() {
    for speaker in SPEAKERS {
        let error = decode::<()>(speaker, &mut FailingReader).unwrap_err();
        assert_eq!(error.origin, Origin::direction(speaker));
        assert!(matches!(
            error.kind,
            DecodeErrorKind::Read {
                part: FramePart::Signal,
                source,
            } if source.kind() == borsh::io::ErrorKind::Other
        ));
    }
}
