use std::collections::BTreeMap;
use std::fmt::Write;
use std::io::Cursor;
use std::pin::Pin;
use std::task::{Context, Poll};

use proptest::{
    collection::{btree_map, vec},
    prelude::*,
};
use tokio::io::AsyncWrite;

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

mod error_atlas;

/// Largest query fan in the exhaustive small-scope enumeration.
const MAX_EXHAUSTIVE_BRANCHING: usize = 2;

/// Frames produced by the bounded exhaustive enumeration.
const EXHAUSTIVE_FRAME_CASES: usize = 1_677_883;

/// Bounded exhaustive frames admitted in the initiator direction.
const INITIATOR_EXHAUSTIVE_FRAME_CASES: usize = 1_513_393;

/// Bounded exhaustive frames admitted in the responder direction.
const RESPONDER_EXHAUSTIVE_FRAME_CASES: usize = 1_546_288;

/// Elected speaker directions represented by the codec.
const SPEAKER_COUNT: usize = 2;

/// Semantic signal states represented by the dense grammar.
const SIGNAL_COUNT: usize = 14;

/// Speaker, stream, and signal buckets in the exhaustive corpus manifest.
const CORPUS_BUCKET_COUNT: usize = SPEAKER_COUNT * Stream::COUNT as usize * SIGNAL_COUNT;

/// Every semantic signal state, independent of its stream placement.
const SIGNALS: [Signal; SIGNAL_COUNT] = [
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
        prop_assume!(WireSignal::new(speaker, frame.0, frame_signal(&frame.1)).is_ok());
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

    /// The async bridge emits the synchronous codec's exact bytes and decodes
    /// them back without retaining state at EOF, for both speaker directions.
    #[test]
    fn async_frame_round_trips_canonically(
        frame in arb_frame(),
        initiator in any::<bool>(),
    ) {
        let speaker = if initiator {
            Speaker::Initiator
        } else {
            Speaker::Responder
        };
        prop_assume!(WireSignal::new(speaker, frame.0, frame_signal(&frame.1)).is_ok());
        let mut canonical = Vec::new();
        encode(speaker, &frame, &mut canonical).unwrap();
        let mut writer = FrameWrite::new(speaker, RecordingWrite::default());
        pollster::block_on(writer.frame(&frame)).unwrap();
        let written = writer.into_inner();
        prop_assert_eq!(written.flushes, 1);
        prop_assert_eq!(&written.bytes, &canonical);

        let mut reader = FrameRead::new(speaker, written.bytes.as_slice());
        let decoded = pollster::block_on(reader.frame::<u64>()).unwrap();
        prop_assert_eq!(decoded, Some(frame));
        prop_assert_eq!(pollster::block_on(reader.frame::<u64>()).unwrap(), None);
    }
}

#[derive(Default)]
struct RecordingWrite {
    bytes: Vec<u8>,
    flushes: usize,
}

impl AsyncWrite for RecordingWrite {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        // Force `write_all` to preserve every frame part across partial writes.
        let written = bytes.len().min(3);
        self.bytes.extend_from_slice(&bytes[..written]);
        Poll::Ready(Ok(written))
    }

    fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.flushes += 1;
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

/// A one-byte duplex forces async frame pieces through backpressure while two
/// adjacent variable bodies retain their exact boundary.
#[tokio::test]
async fn async_duplex_preserves_adjacent_frame_boundaries() {
    let stream = Stream::new(7).unwrap();
    for speaker in [Speaker::Initiator, Speaker::Responder] {
        let first = (
            stream,
            Frame::Reaction(
                Reaction::Query(vec![
                    (1, Hash([1; MERKLE_HASH_LEN])),
                    (2, Hash([2; MERKLE_HASH_LEN])),
                ]),
                Flow::Continue,
            ),
        );
        let second = (
            stream,
            Frame::Reaction(
                Reaction::Supply(Version::new(), Message::new(42_u64)),
                Flow::End(End::Reply),
            ),
        );
        let (send, receive) = tokio::io::duplex(1);
        let sent_first = first.clone();
        let sent_second = second.clone();
        let sending = async {
            let mut writer = FrameWrite::new(speaker, send);
            writer.frame(&sent_first).await.unwrap();
            writer.frame(&sent_second).await.unwrap();
        };
        let receiving = async {
            let mut reader = FrameRead::new(speaker, receive);
            assert_eq!(reader.frame::<u64>().await.unwrap(), Some(first));
            assert_eq!(reader.frame::<u64>().await.unwrap(), Some(second));
            assert_eq!(reader.frame::<u64>().await.unwrap(), None);
        };
        futures::join!(sending, receiving);
    }
}

/// All 476 placements pin either their canonical frame bytes or typed rejection.
#[test]
fn canonical_frame_atlas_snapshot() {
    let mut atlas = String::new();
    for speaker in [Speaker::Initiator, Speaker::Responder] {
        writeln!(atlas, "{speaker:?}").unwrap();
        for index in 0..Stream::COUNT {
            let stream = Stream::new(index).unwrap();
            writeln!(atlas, "  stream {index:02}").unwrap();
            for signal in SIGNALS {
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
                        write!(atlas, "    {signal:?}: accepted len {} hex ", encoded.len())
                            .unwrap();
                        write_hex(&mut atlas, &encoded);
                        atlas.push('\n');
                    }
                    Err(invalid) => {
                        let error = decode_exact::<()>(speaker, &[invalid.byte()]).unwrap_err();
                        assert_eq!(error.origin, Origin::stream(speaker, stream));
                        assert!(matches!(
                            error.kind,
                            DecodeErrorKind::InvalidSignal(DecodeSignalError::Placement(source))
                                if source == invalid
                        ));
                        writeln!(
                            atlas,
                            "    {signal:?}: rejected byte {:02x} class {:?}",
                            invalid.byte(),
                            invalid.class(),
                        )
                        .unwrap();
                    }
                }
            }
        }
    }
    insta::assert_snapshot!(atlas);
}

fn write_hex(out: &mut impl Write, bytes: &[u8]) {
    for byte in bytes {
        write!(out, "{byte:02x}").unwrap();
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

/// Every bounded frame's exact codec outcome is pinned by speaker, stream, and signal.
#[test]
fn bounded_corpus_manifest_snapshot() {
    let mut frames = 0;
    let mut accepted = [0; 2];
    let mut buckets = (0..CORPUS_BUCKET_COUNT)
        .map(|_| CorpusBucket::default())
        .collect::<Vec<_>>();
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
                &mut buckets,
            );
            frames += 1;

            check_both(
                (
                    stream,
                    Frame::Reaction(Reaction::Supply(Version::new(), Message::new(())), flow),
                ),
                &mut accepted,
                &mut buckets,
            );
            frames += 1;

            enumerate_queries(0, &mut Vec::new(), &mut |children| {
                check_both(
                    (
                        stream,
                        Frame::Reaction(Reaction::Query(children.to_vec()), flow),
                    ),
                    &mut accepted,
                    &mut buckets,
                );
                frames += 1;
            });
        }

        for end in [End::Reply, End::Stream] {
            check_both((stream, Frame::End(end)), &mut accepted, &mut buckets);
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

    let mut manifest = String::new();
    for (direction, speaker) in [Speaker::Initiator, Speaker::Responder]
        .into_iter()
        .enumerate()
    {
        writeln!(manifest, "{speaker:?}").unwrap();
        for index in 0..Stream::COUNT {
            writeln!(manifest, "  stream {index:02}").unwrap();
            for (signal_index, signal) in SIGNALS.into_iter().enumerate() {
                let bucket = &buckets[corpus_bucket(direction, index, signal_index)];
                writeln!(
                    manifest,
                    "    {signal:?}: cases {} accepted {} rejected {} rejection {:?} digest {}",
                    bucket.cases,
                    bucket.accepted,
                    bucket.rejected,
                    bucket.rejection,
                    bucket.hasher.clone().finalize().to_hex(),
                )
                .unwrap();
            }
        }
    }
    insta::assert_snapshot!(manifest);
}

#[derive(Default)]
struct CorpusBucket {
    cases: usize,
    accepted: usize,
    rejected: usize,
    rejection: Option<StreamClass>,
    hasher: blake3::Hasher,
}

impl CorpusBucket {
    fn accept(&mut self, encoded: &[u8]) {
        const ACCEPTED: u8 = 1;

        self.cases += 1;
        self.accepted += 1;
        self.hasher.update(&[ACCEPTED]);
        self.hasher.update(&(encoded.len() as u64).to_be_bytes());
        self.hasher.update(encoded);
    }

    fn reject(&mut self, invalid: InvalidSignalPlacement) {
        const REJECTED: u8 = 0;

        let class = invalid.class();
        assert!(self.rejection.is_none_or(|previous| previous == class));
        self.cases += 1;
        self.rejected += 1;
        self.rejection = Some(class);
        self.hasher.update(&[REJECTED, invalid.byte()]);
    }
}

fn corpus_bucket(direction: usize, stream: u8, signal: usize) -> usize {
    (direction * usize::from(Stream::COUNT) + usize::from(stream)) * SIGNAL_COUNT + signal
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

fn check_both(frame: WireFrame<()>, accepted: &mut [usize; 2], buckets: &mut [CorpusBucket]) {
    let signal = frame_signal(&frame.1);
    let signal_index = SIGNALS
        .iter()
        .position(|candidate| *candidate == signal)
        .expect("every frame maps to a semantic signal state");
    for (direction, speaker) in [Speaker::Initiator, Speaker::Responder]
        .into_iter()
        .enumerate()
    {
        let bucket = &mut buckets[corpus_bucket(direction, frame.0.index(), signal_index)];
        match WireSignal::new(speaker, frame.0, signal) {
            Ok(_) => {
                let mut encoded = Vec::new();
                encode(speaker, &frame, &mut encoded).unwrap();
                accepted[direction] += 1;
                assert_eq!(decode_exact::<()>(speaker, &encoded).unwrap(), frame);
                bucket.accept(&encoded);
            }
            Err(invalid) => bucket.reject(invalid),
        }
    }
}

fn frame_signal<T>(frame: &Frame<T>) -> Signal {
    match frame {
        Frame::Reaction(Reaction::Match, flow) => Signal::Match(*flow),
        Frame::Reaction(Reaction::Query(children), flow) if children.is_empty() => {
            Signal::QueryEmpty(*flow)
        }
        Frame::Reaction(Reaction::Query(_), flow) => Signal::Query(*flow),
        Frame::Reaction(Reaction::Supply(_, _), flow) => Signal::Supply(*flow),
        Frame::End(end) => Signal::End(*end),
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
