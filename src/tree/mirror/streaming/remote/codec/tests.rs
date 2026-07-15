use std::collections::BTreeMap;
use std::io::Cursor;

use proptest::{
    collection::{btree_map, vec},
    prelude::*,
};

use super::frame::MAX_QUERY_CHILDREN;
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
        encode(speaker, &frame, &mut encoded).unwrap();
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

/// Every frame with at most two zero-hash children round-trips for both speakers.
#[test]
fn bounded_frames_round_trip_exhaustively() {
    let mut frames = 0;
    for index in 0_u8..Stream::COUNT {
        let stream = Stream::new(index).unwrap();
        for flow in [
            Flow::Continue,
            Flow::End(End::Reply),
            Flow::End(End::Stream),
        ] {
            round_trip_both((stream, Frame::Reaction(Reaction::Match, flow)));
            frames += 1;

            round_trip_both((
                stream,
                Frame::Reaction(Reaction::Supply(Version::new(), Message::new(())), flow),
            ));
            frames += 1;

            enumerate_queries(0, &mut Vec::new(), &mut |children| {
                round_trip_both((
                    stream,
                    Frame::Reaction(Reaction::Query(children.to_vec()), flow),
                ));
                frames += 1;
            });
        }

        for end in [End::Reply, End::Stream] {
            round_trip_both((stream, Frame::End(end)));
            frames += 1;
        }
    }
    assert_eq!(frames, EXHAUSTIVE_FRAME_CASES);
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

fn round_trip_both(frame: WireFrame<()>) {
    for speaker in [Speaker::Initiator, Speaker::Responder] {
        let mut encoded = Vec::new();
        encode(speaker, &frame, &mut encoded).unwrap();
        assert_eq!(decode_exact::<()>(speaker, &encoded).unwrap(), frame);
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
