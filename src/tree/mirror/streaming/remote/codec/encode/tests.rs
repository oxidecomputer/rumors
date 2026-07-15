use borsh::BorshSerialize;
use proptest::prelude::*;

use super::*;
use crate::{
    message::Message,
    tree::{
        arb::arb_version,
        typed::{Hash, hash::MERKLE_HASH_LEN},
    },
};

use super::super::{
    error::{Origin, QueryOrderError},
    frame::{MAX_QUERY_CHILDREN, QUERY_CHILD_LEN, QUERY_COUNT_BIAS, QUERY_COUNT_LEN},
    signal::{End, Flow, Speaker, Stream},
};

const SPEAKERS: [Speaker; 2] = [Speaker::Initiator, Speaker::Responder];
const FLOWS: [Flow; 3] = [
    Flow::Continue,
    Flow::End(End::Reply),
    Flow::End(End::Stream),
];

/// Smallest amount by which a generated query exceeds the protocol fan.
const MIN_OVERSIZED_QUERY_EXTRA: usize = 1;

/// Exclusive upper bound for generated children beyond the protocol fan.
const MAX_OVERSIZED_QUERY_EXTRA: usize = 64;

fn stream(index: u8) -> Stream {
    Stream::new(index).unwrap()
}

fn signal(stream: Stream, signal: Signal) -> u8 {
    WireSignal::new(stream, signal).to_byte()
}

fn arb_speaker() -> impl Strategy<Value = Speaker> {
    prop_oneof![Just(Speaker::Initiator), Just(Speaker::Responder)]
}

fn arb_flow() -> impl Strategy<Value = Flow> {
    prop_oneof![
        Just(Flow::Continue),
        Just(Flow::End(End::Reply)),
        Just(Flow::End(End::Stream)),
    ]
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
            (
                stream,
                Frame::Reaction(Reaction::Match, Flow::End(End::Reply)),
            ),
            signal(stream, Signal::Match(Flow::End(End::Reply))),
        ),
        (
            (
                stream,
                Frame::Reaction(Reaction::Match, Flow::End(End::Stream)),
            ),
            signal(stream, Signal::Match(Flow::End(End::Stream))),
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
        index in 0_u8..Stream::COUNT,
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

    /// Every query wider than the radix fan reports its exact origin and count.
    #[test]
    fn oversized_query_count_is_rejected(
        index in 0_u8..Stream::COUNT,
        speaker in arb_speaker(),
        extra in MIN_OVERSIZED_QUERY_EXTRA..MAX_OVERSIZED_QUERY_EXTRA,
    ) {
        let stream = stream(index);
        let count = MAX_QUERY_CHILDREN + extra;
        let children = (0..count)
            .map(|index| (index as u8, Hash::default()))
            .collect();
        let error = encode(
            speaker,
            &(
                stream,
                Frame::<u64>::Reaction(Reaction::Query(children), Flow::Continue),
            ),
            &mut Vec::new(),
        )
        .unwrap_err();
        prop_assert_eq!(error.origin, Origin::stream(speaker, stream));
        let correct = matches!(
            error.kind,
            EncodeErrorKind::QueryTooWide { count: actual } if actual == count
        );
        prop_assert!(correct);
    }

    /// Every adjacent non-ascending pair reports its values and origin.
    #[test]
    fn unordered_query_is_rejected(
        index in 0_u8..Stream::COUNT,
        speaker in arb_speaker(),
        previous in any::<u8>(),
        radix in any::<u8>(),
    ) {
        prop_assume!(previous >= radix);
        let stream = stream(index);
        let children = vec![(previous, Hash::default()), (radix, Hash::default())];
        let error = encode(
            speaker,
            &(
                stream,
                Frame::<u64>::Reaction(Reaction::Query(children), Flow::Continue),
            ),
            &mut Vec::new(),
        )
        .unwrap_err();
        prop_assert_eq!(error.origin, Origin::stream(speaker, stream));
        let correct = matches!(
            error.kind,
            EncodeErrorKind::QueryOutOfOrder(QueryOrderError {
                previous: actual_previous,
                radix: actual_radix,
            }) if actual_previous == previous && actual_radix == radix
        );
        prop_assert!(correct);
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
