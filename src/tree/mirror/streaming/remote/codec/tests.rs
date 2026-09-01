use std::collections::BTreeMap;
use std::fmt::Write;
use std::io::Cursor;
use std::pin::Pin;
use std::task::{Context, Poll};

use proptest::{
    collection::{btree_map, vec},
    prelude::*,
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use super::frame::{LeafRun, MAX_QUERY_CHILDREN};
use super::signal::{Signal, WireSignal};
use super::*;
use crate::{
    Version,
    message::Message,
    tree::{
        arb::arb_version,
        mirror::{cbor, framing::PAYLOAD_CHUNK_LEN},
        typed::{Hash, hash::MERKLE_HASH_LEN},
    },
};

use serde::Serialize;
mod error_atlas;

/// Largest query fan in the exhaustive small-scope enumeration.
const MAX_EXHAUSTIVE_BRANCHING: usize = 2;

/// Frames produced by the bounded exhaustive enumeration.
const EXHAUSTIVE_FRAME_CASES: usize = 1_118_600;

/// Bounded exhaustive frames admitted in the initiator direction.
const INITIATOR_EXHAUSTIVE_FRAME_CASES: usize = 987_012;

/// Bounded exhaustive frames admitted in the responder direction.
const RESPONDER_EXHAUSTIVE_FRAME_CASES: usize = 1_052_803;

/// Elected speaker directions represented by the codec.
const SPEAKER_COUNT: usize = 2;

/// Semantic signal states represented by the dense grammar.
const SIGNAL_COUNT: usize = 10;

/// Speaker, stream, and signal buckets in the exhaustive corpus manifest.
const CORPUS_BUCKET_COUNT: usize = SPEAKER_COUNT * Stream::COUNT as usize * SIGNAL_COUNT;

/// Every semantic signal state, independent of its stream placement.
const SIGNALS: [Signal; SIGNAL_COUNT] = [
    Signal::Match(Flow::Continue),
    Signal::Match(Flow::End),
    Signal::QueryEmpty(Flow::Continue),
    Signal::QueryEmpty(Flow::End),
    Signal::Query(Flow::Continue),
    Signal::Query(Flow::End),
    Signal::Supply(Flow::Continue),
    Signal::Supply(Flow::End),
    Signal::End(End::Reply),
    Signal::End(End::Stream),
];

/// Exclusive upper bound for arbitrary bytes following a decoded frame.
const MAX_ARBITRARY_SUFFIX_LEN: usize = 32;

/// Inclusive upper bound on records in an arbitrary supply run.
const MAX_ARBITRARY_RUN_RECORDS: usize = 4;

/// Build a supply run from decoded leaf records.
fn leaf_run<T: Serialize + Clone + Send + Sync + 'static>(records: &[(Version, T)]) -> LeafRun {
    let mut run = LeafRun::new();
    for (version, value) in records {
        run.push(version, &Message::new(value.clone()))
            .expect("a test record fits the run framing");
    }
    run
}

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
    prop_oneof![Just(Flow::Continue), Just(Flow::End)]
}

fn arb_frame() -> impl Strategy<Value = WireFrame> {
    prop_oneof![
        (arb_stream(), arb_flow())
            .prop_map(|(stream, flow)| (stream, Frame::Reaction(Reaction::Match, flow))),
        (arb_stream(), arb_query(), arb_flow()).prop_map(|(stream, children, flow)| (
            stream,
            Frame::Reaction(Reaction::Query(children), flow)
        )),
        (
            arb_stream(),
            vec((arb_version(), any::<u64>()), 1..=MAX_ARBITRARY_RUN_RECORDS),
            arb_flow(),
        )
            .prop_map(|(stream, records, flow)| (
                stream,
                Frame::Reaction(Reaction::Supply(leaf_run(&records)), flow)
            )),
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
        let decoded = decode(speaker, RunBudget::default(), &mut rest).unwrap();
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

        let mut reader = FrameRead::new(speaker, RunBudget::default(), written.bytes.as_slice());
        let decoded = pollster::block_on(reader.frame()).unwrap();
        prop_assert_eq!(decoded, Some(frame));
        prop_assert_eq!(pollster::block_on(reader.frame()).unwrap(), None);
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
#[pollster::test]
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
                Reaction::Supply(leaf_run(&[(Version::new(), 42_u64)])),
                Flow::End,
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
            let mut reader = FrameRead::new(speaker, RunBudget::default(), receive);
            assert_eq!(reader.frame().await.unwrap(), Some(first));
            assert_eq!(reader.frame().await.unwrap(), Some(second));
            assert_eq!(reader.frame().await.unwrap(), None);
        };
        futures::join!(sending, receiving);
    }
}

/// All 340 placements pin either their canonical frame bytes or typed rejection.
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
                        // The frame head carries the dense code as a uint
                        // item right behind the array head.
                        let mut expected_signal = Vec::new();
                        crate::tree::mirror::cbor::write_head(
                            &mut expected_signal,
                            crate::tree::mirror::cbor::MAJOR_UINT,
                            u64::from(wire.to_byte()),
                        );
                        assert_eq!(&encoded[1..1 + expected_signal.len()], expected_signal);
                        assert_eq!(
                            decode_exact(speaker, RunBudget::default(), &encoded).unwrap(),
                            (stream, frame)
                        );
                        write!(atlas, "    {signal:?}: accepted len {} hex ", encoded.len())
                            .unwrap();
                        write_hex(&mut atlas, &encoded);
                        atlas.push('\n');
                    }
                    Err(invalid) => {
                        let mut rejected = Vec::new();
                        crate::tree::mirror::cbor::write_head(
                            &mut rejected,
                            crate::tree::mirror::cbor::MAJOR_ARRAY,
                            1,
                        );
                        crate::tree::mirror::cbor::write_head(
                            &mut rejected,
                            crate::tree::mirror::cbor::MAJOR_UINT,
                            u64::from(invalid.byte()),
                        );
                        let error =
                            decode_exact(speaker, RunBudget::default(), &rejected).unwrap_err();
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

fn representative_frame(signal: Signal) -> Frame {
    match signal {
        Signal::Match(flow) => Frame::Reaction(Reaction::Match, flow),
        Signal::QueryEmpty(flow) => Frame::Reaction(Reaction::Query(Vec::new()), flow),
        Signal::Query(flow) => Frame::Reaction(Reaction::Query(vec![(0, Hash::default())]), flow),
        Signal::Supply(flow) => {
            Frame::Reaction(Reaction::Supply(leaf_run(&[(Version::new(), ())])), flow)
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
        for flow in [Flow::Continue, Flow::End] {
            check_both(
                (stream, Frame::Reaction(Reaction::Match, flow)),
                &mut accepted,
                &mut buckets,
            );
            frames += 1;

            check_both(
                (
                    stream,
                    Frame::Reaction(Reaction::Supply(leaf_run(&[(Version::new(), ())])), flow),
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

fn check_both(frame: WireFrame, accepted: &mut [usize; 2], buckets: &mut [CorpusBucket]) {
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
                assert_eq!(
                    decode_exact(speaker, RunBudget::default(), &encoded).unwrap(),
                    frame
                );
                bucket.accept(&encoded);
            }
            Err(invalid) => bucket.reject(invalid),
        }
    }
}

fn frame_signal(frame: &Frame) -> Signal {
    match frame {
        Frame::Reaction(Reaction::Match, flow) => Signal::Match(*flow),
        Frame::Reaction(Reaction::Query(children), flow) if children.is_empty() => {
            Signal::QueryEmpty(*flow)
        }
        Frame::Reaction(Reaction::Query(_), flow) => Signal::Query(*flow),
        Frame::Reaction(Reaction::Supply(_), flow) => Signal::Supply(*flow),
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
            Reaction::Supply(leaf_run(&[(Version::new(), ())])),
            Flow::End,
        ),
    );
    let mut writer = Cursor::new(Vec::new());
    encode(Speaker::Initiator, &frame, &mut writer).unwrap();
    let frame_len = writer.position();
    writer.get_mut().push(0xaa);

    let mut reader = Cursor::new(writer.into_inner());
    assert_eq!(
        decode(Speaker::Initiator, RunBudget::default(), &mut reader).unwrap(),
        frame
    );
    assert_eq!(reader.position(), frame_len);
}

/// A slice-backed async reader that counts transport reads and serves
/// each in full, so a decode's count is the reader's own read plan and
/// not the transport's chunking.
struct CountingRead<'a> {
    bytes: &'a [u8],
    reads: usize,
}

impl AsyncRead for CountingRead<'_> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        self.reads += 1;
        let served = self.bytes.len().min(buf.remaining());
        let (head, rest) = self.bytes.split_at(served);
        buf.put_slice(head);
        self.bytes = rest;
        Poll::Ready(Ok(()))
    }
}

/// Transport reads the async decoder spends on one head: one for its
/// initial byte, one more when the head carries an argument extension
/// (a value of 24 or more).
fn head_reads(value: u64) -> usize {
    1 + usize::from(cbor::head_len(value) > 1)
}

/// Transport reads the async decoder spends on one canonical frame whose
/// transport serves every read in full: the reader's read plan, stated
/// over the frame's wire shape.
///
/// The reader fetches one CBOR head at a time ([`head_reads`]), then one
/// read per fixed-width field: a listed digest, or a run body within one
/// payload chunk. A supply frame's run must fit one chunk for the plan
/// to hold; `async_decode_spends_its_read_plan` assumes that bound.
fn read_plan(frame: &WireFrame) -> usize {
    let (stream, frame) = frame;
    let signal = frame_signal(frame);
    let arity = match frame {
        Frame::Reaction(Reaction::Query(children), _) if !children.is_empty() => 2,
        Frame::Reaction(Reaction::Supply(_), _) => 2,
        _ => 1,
    };
    let mut reads = head_reads(arity) + head_reads(u64::from(WireSignal::encode(*stream, signal)));
    match frame {
        Frame::Reaction(Reaction::Query(children), _) if !children.is_empty() => {
            reads += head_reads(children.len() as u64);
            for (radix, _) in children {
                reads += head_reads(u64::from(*radix)) + head_reads(MERKLE_HASH_LEN as u64) + 1;
            }
        }
        Frame::Reaction(Reaction::Supply(run), _) => {
            reads += head_reads(cbor::TAG_CBOR_SEQUENCE) + head_reads(run.encoded_len() as u64) + 1;
        }
        _ => {}
    }
    reads
}

/// Decode `encoded` as one frame through the async reader over a
/// transport that serves every read in full, returning the frame and the
/// reads it cost.
fn decode_counting(speaker: Speaker, encoded: &[u8]) -> (WireFrame, usize) {
    let mut reader = FrameRead::new(
        speaker,
        RunBudget::default(),
        CountingRead {
            bytes: encoded,
            reads: 0,
        },
    );
    let frame = pollster::block_on(reader.frame())
        .expect("a canonical frame decodes")
        .expect("a nonempty stream is not a clean close");
    (frame, reader.into_inner().reads)
}

proptest! {
    /// The async decoder spends exactly its read plan on every canonical
    /// frame.
    ///
    /// No head or field costs a transport read the plan does not name,
    /// and none is skipped: the plan is positive for every frame, so a
    /// reader that stopped reading could not satisfy it.
    #[test]
    fn async_decode_spends_its_read_plan(
        frame in arb_frame(),
        initiator in any::<bool>(),
    ) {
        let speaker = if initiator {
            Speaker::Initiator
        } else {
            Speaker::Responder
        };
        prop_assume!(WireSignal::new(speaker, frame.0, frame_signal(&frame.1)).is_ok());
        if let Frame::Reaction(Reaction::Supply(run), _) = &frame.1 {
            prop_assume!(run.encoded_len() <= PAYLOAD_CHUNK_LEN);
        }
        let mut encoded = Vec::new();
        encode(speaker, &frame, &mut encoded).unwrap();
        let (decoded, reads) = decode_counting(speaker, &encoded);
        prop_assert_eq!(&decoded, &frame);
        prop_assert_eq!(reads, read_plan(&frame));
    }
}

/// The read plan at the wire's reference shapes, pinned as numbers so a
/// change to the reader's batching shows in this diff.
///
/// The match reaction is the frame the reader meets most often, pinned
/// on either side of the signal head's width boundary (codes below 24
/// take a one-byte head); the full-fan query is the widest listing; the
/// lone-record supply is the smallest run. Each case checks the decoder's
/// actual reads against the pinned number and the pinned number against
/// the plan formula, so the formula and the reader are held to each
/// other.
#[test]
fn read_plan_at_reference_shapes() {
    let stream = Stream::new(4).unwrap();
    let last_stream = Stream::new(Stream::MAX).unwrap();
    let cases: [(&str, WireFrame, usize); 4] = [
        (
            "match ending its reply, one-byte signal",
            (stream, Frame::Reaction(Reaction::Match, Flow::End)),
            2,
        ),
        (
            "match ending its reply, two-byte signal",
            (last_stream, Frame::Reaction(Reaction::Match, Flow::End)),
            3,
        ),
        (
            "full-fan query",
            (
                stream,
                Frame::Reaction(
                    Reaction::Query(
                        (0..=u8::MAX)
                            .map(|radix| (radix, Hash([radix; MERKLE_HASH_LEN])))
                            .collect(),
                    ),
                    Flow::Continue,
                ),
            ),
            1261,
        ),
        (
            "lone-record supply",
            (
                stream,
                Frame::Reaction(
                    Reaction::Supply(leaf_run(&[(Version::new(), 42_u64)])),
                    Flow::End,
                ),
            ),
            7,
        ),
    ];
    for (name, frame, expected) in cases {
        let mut encoded = Vec::new();
        encode(Speaker::Initiator, &frame, &mut encoded).unwrap();
        let (decoded, reads) = decode_counting(Speaker::Initiator, &encoded);
        assert_eq!(decoded, frame, "{name}");
        assert_eq!(reads, expected, "{name}: transport reads");
        assert_eq!(read_plan(&frame), expected, "{name}: read plan");
    }
}
