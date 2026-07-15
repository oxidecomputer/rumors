use std::collections::BTreeMap;
use std::io::Cursor;

use proptest::{
    collection::{btree_map, vec},
    prelude::*,
};

use super::frame::MAX_QUERY_CHILDREN;
use super::signal::{Signal, WireSignal};
use super::*;
use crate::{
    Version,
    message::Message,
    tree::{
        arb::arb_version,
        typed::{Hash, hash::MERKLE_HASH_LEN},
    },
};

/// Largest query fan in the exhaustive small-scope enumeration.
const MAX_EXHAUSTIVE_BRANCHING: usize = 2;

/// Frames produced by the bounded exhaustive enumeration.
const EXHAUSTIVE_FRAME_CASES: usize = 1_677_883;

/// Bounded exhaustive frames admitted in the initiator direction.
const INITIATOR_EXHAUSTIVE_FRAME_CASES: usize = 1_513_393;

/// Bounded exhaustive frames admitted in the responder direction.
const RESPONDER_EXHAUSTIVE_FRAME_CASES: usize = 1_546_288;

/// Every semantic signal state, independent of its stream placement.
const SIGNALS: &[Signal] = &[
    Signal::Match(Flow::Continue),
    Signal::Match(Flow::End(End::Reply)),
    Signal::Match(Flow::End(End::Stream)),
    Signal::QueryEmpty(Flow::Continue),
    Signal::QueryEmpty(Flow::End(End::Reply)),
    Signal::QueryEmpty(Flow::End(End::Stream)),
    Signal::Query(Flow::Continue),
    Signal::Query(Flow::End(End::Reply)),
    Signal::Query(Flow::End(End::Stream)),
    Signal::Supply(Flow::Continue),
    Signal::Supply(Flow::End(End::Reply)),
    Signal::Supply(Flow::End(End::Stream)),
    Signal::End(End::Reply),
    Signal::End(End::Stream),
];

/// Exclusive upper bound for arbitrary bytes following a decoded frame.
const MAX_ARBITRARY_SUFFIX_LEN: usize = 32;

fn arb_stream() -> impl Strategy<Value = Stream> {
    (0_u8..Stream::COUNT).prop_map(|index| Stream::new(index).unwrap())
}

fn arb_hash() -> impl Strategy<Value = Hash> {
    any::<[u8; MERKLE_HASH_LEN]>().prop_map(Hash)
}

fn arb_query() -> impl Strategy<Value = Vec<(u8, Hash)>> {
    btree_map(any::<u8>(), arb_hash(), 0..=MAX_QUERY_CHILDREN)
        .prop_map(|children: BTreeMap<_, _>| children.into_iter().collect())
}

fn arb_flow() -> impl Strategy<Value = Flow> {
    prop_oneof![
        Just(Flow::Continue),
        Just(Flow::End(End::Reply)),
        Just(Flow::End(End::Stream)),
    ]
}

fn arb_frame() -> impl Strategy<Value = WireFrame<u64>> {
    prop_oneof![
        (arb_stream(), arb_flow())
            .prop_map(|(stream, flow)| (stream, Frame::Reaction(Reaction::Match, flow))),
        (arb_stream(), arb_query(), arb_flow()).prop_map(|(stream, children, flow)| (
            stream,
            Frame::Reaction(Reaction::Query(children), flow)
        )),
        (arb_stream(), arb_version(), any::<u64>(), arb_flow()).prop_map(
            |(stream, version, value, flow)| (
                stream,
                Frame::Reaction(Reaction::Supply(version, Message::new(value)), flow)
            )
        ),
        arb_stream().prop_map(|stream| (stream, Frame::End(End::Reply))),
        arb_stream().prop_map(|stream| (stream, Frame::End(End::Stream))),
    ]
}

proptest! {
    /// Every valid frame is self-delimiting and round-trips canonically.
    #[test]
    fn frame_round_trips(
        frame in arb_frame(),
        suffix in vec(any::<u8>(), 0..MAX_ARBITRARY_SUFFIX_LEN),
        initiator in any::<bool>(),
    ) {
        let speaker = if initiator {
            Speaker::Initiator
        } else {
            Speaker::Responder
        };
        let mut encoded = Vec::new();
        if let Err(error) = encode(speaker, &frame, &mut encoded) {
            prop_assert!(matches!(error.kind, EncodeErrorKind::InvalidSignal(_)));
            prop_assert!(encoded.is_empty());
            return Ok(());
        }
        let frame_len = encoded.len();
        encoded.extend_from_slice(&suffix);

        let mut rest = encoded.as_slice();
        let decoded = decode::<u64>(speaker, &mut rest).unwrap();
        prop_assert_eq!(&decoded, &frame);
        prop_assert_eq!(rest, suffix.as_slice());

        let mut canonical = Vec::new();
        encode(speaker, &decoded, &mut canonical).unwrap();
        prop_assert_eq!(canonical, encoded[..frame_len].to_vec());
    }
}

/// All 476 placements either round-trip or fail before a frame body is touched.
#[test]
fn signal_placements_are_enforced_exhaustively() {
    for speaker in [Speaker::Initiator, Speaker::Responder] {
        for index in 0..Stream::COUNT {
            let stream = Stream::new(index).unwrap();
            for &signal in SIGNALS {
                let frame = representative_frame(signal);
                match WireSignal::new(speaker, stream, signal) {
                    Ok(wire) => {
                        let mut encoded = Vec::new();
                        encode(speaker, &(stream, frame.clone()), &mut encoded).unwrap();
                        assert_eq!(encoded.first(), Some(&wire.to_byte()));
                        assert_eq!(
                            decode_exact::<()>(speaker, &encoded).unwrap(),
                            (stream, frame)
                        );
                    }
                    Err(invalid) => {
                        let mut encoded = Vec::new();
                        let error = encode(speaker, &(stream, frame), &mut encoded).unwrap_err();
                        assert_eq!(error.origin, Origin::stream(speaker, stream));
                        assert!(encoded.is_empty());
                        assert!(matches!(
                            error.kind,
                            EncodeErrorKind::InvalidSignal(source) if source == invalid
                        ));

                        let error = decode_exact::<()>(speaker, &[invalid.byte()]).unwrap_err();
                        assert_eq!(error.origin, Origin::stream(speaker, stream));
                        assert!(matches!(
                            error.kind,
                            DecodeErrorKind::InvalidSignal(DecodeSignalError::Placement(source))
                                if source == invalid
                        ));
                    }
                }
            }
        }
    }
}

fn representative_frame(signal: Signal) -> Frame<()> {
    match signal {
        Signal::Match(flow) => Frame::Reaction(Reaction::Match, flow),
        Signal::QueryEmpty(flow) => Frame::Reaction(Reaction::Query(Vec::new()), flow),
        Signal::Query(flow) => Frame::Reaction(Reaction::Query(vec![(0, Hash::default())]), flow),
        Signal::Supply(flow) => {
            Frame::Reaction(Reaction::Supply(Version::new(), Message::new(())), flow)
        }
        Signal::End(end) => Frame::End(end),
    }
}

/// Every bounded frame round-trips exactly where its speaker admits its signal.
#[test]
fn bounded_frames_round_trip_exhaustively() {
    let mut frames = 0;
    let mut accepted = [0; 2];
    for index in 0_u8..Stream::COUNT {
        let stream = Stream::new(index).unwrap();
        for flow in [
            Flow::Continue,
            Flow::End(End::Reply),
            Flow::End(End::Stream),
        ] {
            check_both(
                (stream, Frame::Reaction(Reaction::Match, flow)),
                &mut accepted,
            );
            frames += 1;

            check_both(
                (
                    stream,
                    Frame::Reaction(Reaction::Supply(Version::new(), Message::new(())), flow),
                ),
                &mut accepted,
            );
            frames += 1;

            enumerate_queries(0, &mut Vec::new(), &mut |children| {
                check_both(
                    (
                        stream,
                        Frame::Reaction(Reaction::Query(children.to_vec()), flow),
                    ),
                    &mut accepted,
                );
                frames += 1;
            });
        }

        for end in [End::Reply, End::Stream] {
            check_both((stream, Frame::End(end)), &mut accepted);
            frames += 1;
        }
    }
    assert_eq!(frames, EXHAUSTIVE_FRAME_CASES);
    assert_eq!(
        accepted,
        [
            INITIATOR_EXHAUSTIVE_FRAME_CASES,
            RESPONDER_EXHAUSTIVE_FRAME_CASES,
        ]
    );
}

fn enumerate_queries(
    next: u16,
    children: &mut Vec<(u8, Hash)>,
    visit: &mut impl FnMut(&[(u8, Hash)]),
) {
    visit(children);
    if children.len() == MAX_EXHAUSTIVE_BRANCHING {
        return;
    }
    for radix in next..=u16::from(u8::MAX) {
        children.push((radix as u8, Hash::default()));
        enumerate_queries(radix + 1, children, visit);
        children.pop();
    }
}

fn check_both(frame: WireFrame<()>, accepted: &mut [usize; 2]) {
    for (direction, speaker) in [Speaker::Initiator, Speaker::Responder]
        .into_iter()
        .enumerate()
    {
        let mut encoded = Vec::new();
        match encode(speaker, &frame, &mut encoded) {
            Ok(()) => {
                accepted[direction] += 1;
                assert_eq!(decode_exact::<()>(speaker, &encoded).unwrap(), frame);
            }
            Err(error) => {
                assert!(encoded.is_empty());
                assert!(matches!(error.kind, EncodeErrorKind::InvalidSignal(_)));
            }
        }
    }
}

/// Generic readers and writers consume exactly one frame.
#[test]
fn generic_io_preserves_frame_boundaries() {
    let stream = Stream::new(11).unwrap();
    let frame = (
        stream,
        Frame::Reaction(
            Reaction::Supply(Version::new(), Message::new(())),
            Flow::End(End::Reply),
        ),
    );
    let mut writer = Cursor::new(Vec::new());
    encode(Speaker::Initiator, &frame, &mut writer).unwrap();
    let frame_len = writer.position();
    writer.get_mut().push(0xaa);

    let mut reader = Cursor::new(writer.into_inner());
    assert_eq!(
        decode::<()>(Speaker::Initiator, &mut reader).unwrap(),
        frame
    );
    assert_eq!(reader.position(), frame_len);
}
