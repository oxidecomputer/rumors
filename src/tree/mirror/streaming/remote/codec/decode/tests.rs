use proptest::prelude::*;

use super::*;
use crate::Version;
use crate::message::Message;
use crate::tree::arb::arb_version;

use super::super::{
    error::{DecodeLeafError, Origin, QueryOrderError},
    frame::{QUERY_COUNT_BIAS, QUERY_COUNT_LEN},
    signal::{DecodeSignalError, End, Flow, Speaker, Stream, StreamError},
};

const SPEAKERS: [Speaker; 2] = [Speaker::Initiator, Speaker::Responder];

/// A CBOR byte-string header promising two version bytes, cut short after
/// one: the version field ends inside its own framing.
const TRUNCATED_VERSION: &[u8] = &[0x42, 0x01];

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

/// One length-prefixed leaf record as it appears inside a run body: the
/// version as one CBOR value, then the payload's CBOR bytes bare.
fn record(version: &Version, message: &Message) -> Vec<u8> {
    let mut body = Vec::new();
    ciborium::ser::into_writer(version, &mut body).unwrap();
    body.extend_from_slice(message.as_slice());
    let mut record = (body.len() as u32).to_be_bytes().to_vec();
    record.extend_from_slice(&body);
    record
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
            let error = decode_exact(speaker, RunBudget::default(), &[byte]).unwrap_err();
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
                FramePart::SupplyRun,
                Origin::stream(speaker, stream),
            ),
        ];
        for (encoded, missing, origin) in cases {
            let error = decode_exact(speaker, RunBudget::default(), &encoded).unwrap_err();
            assert_eq!(error.origin, origin);
            let DecodeErrorKind::Truncated {
                missing: actual,
                source,
            } = error.kind
            else {
                panic!("unexpected error kind");
            };
            assert_eq!(actual, missing);
            assert_eq!(source.kind(), std::io::ErrorKind::UnexpectedEof);
        }
    }
}

proptest! {
    /// An arbitrary run of supplied records decodes into a frame carrying the
    /// exact run body, without decoding any record eagerly.
    #[test]
    fn supplied_run_is_decoded_structurally(
        index in 1_u8..Stream::MAX,
        speaker in arb_speaker(),
        flow in arb_flow(),
        records in proptest::collection::vec((arb_version(), any::<u64>()), 1..=4),
    ) {
        let stream = stream(index);
        let mut body = Vec::new();
        for (version, value) in &records {
            body.extend_from_slice(&record(version, &Message::new(*value)));
        }
        let encoded = supply(stream, flow, &body);

        let expected = LeafRun::from_encoded(body).unwrap();
        prop_assert_eq!(
            decode_exact(speaker, RunBudget::default(), &encoded).unwrap(),
            (stream, Frame::Reaction(Reaction::Supply(expected), flow))
        );
    }
}

/// Structurally invalid runs are rejected at the wire with their exact cause:
/// an empty run, a record header past the run's end, or a record body past
/// the run's end.
#[test]
fn malformed_run_structure_is_typed() {
    use super::super::frame::LeafRunError;

    let stream = stream(8);
    for speaker in SPEAKERS {
        let empty = decode_exact(
            speaker,
            RunBudget::default(),
            &supply(stream, Flow::Continue, &[]),
        )
        .unwrap_err();
        assert_eq!(empty.origin, Origin::stream(speaker, stream));
        assert!(matches!(
            empty.kind,
            DecodeErrorKind::InvalidRun(LeafRunError::Empty)
        ));

        let short_header = decode_exact(
            speaker,
            RunBudget::default(),
            &supply(stream, Flow::Continue, &[0, 0]),
        )
        .unwrap_err();
        assert_eq!(short_header.origin, Origin::stream(speaker, stream));
        assert!(matches!(
            short_header.kind,
            DecodeErrorKind::InvalidRun(LeafRunError::TruncatedHeader { remaining: 2 })
        ));

        let mut overrun = 2_u32.to_be_bytes().to_vec();
        overrun.push(0);
        let short_record = decode_exact(
            speaker,
            RunBudget::default(),
            &supply(stream, Flow::Continue, &overrun),
        )
        .unwrap_err();
        assert_eq!(short_record.origin, Origin::stream(speaker, stream));
        assert!(matches!(
            short_record.kind,
            DecodeErrorKind::InvalidRun(LeafRunError::TruncatedRecord {
                len: 2,
                remaining: 1
            })
        ));
    }
}

/// A zero-length record header inside a run body is structurally valid.
///
/// From raw wire bytes, a run body of one bare `00000000` header chains
/// exactly, so the codec accepts the frame and defers the record's failure
/// to its record iterator: the empty body cannot hold a version, and the
/// iterator reports the version decoder's `UnexpectedEof`.
#[test]
fn a_zero_length_record_is_structurally_valid() {
    let stream = stream(8);
    let encoded = supply(stream, Flow::End, &[0, 0, 0, 0]);
    for speaker in SPEAKERS {
        let (decoded_stream, frame) =
            decode_exact(speaker, RunBudget::default(), &encoded).unwrap();
        assert_eq!(decoded_stream, stream);
        let Frame::Reaction(Reaction::Supply(run), Flow::End) = frame else {
            panic!("a structurally valid run decodes as a supply reaction");
        };
        assert_eq!(run.record_count(), 1);
        let error = run
            .records(Message::deserializer::<u64>())
            .next()
            .unwrap()
            .unwrap_err();
        let DecodeLeafError::Version(source) = error else {
            panic!("unexpected record error");
        };
        assert_eq!(source.kind(), std::io::ErrorKind::UnexpectedEof);
    }
}

/// A record's canonical decoding is deferred to the run's record iterator,
/// which types each failure and retains the source error.
#[test]
fn supplied_record_errors_are_typed() {
    let mut truncated_version = (TRUNCATED_VERSION.len() as u32).to_be_bytes().to_vec();
    truncated_version.extend_from_slice(TRUNCATED_VERSION);
    let run = LeafRun::from_encoded(truncated_version).unwrap();
    let error = run
        .records(Message::deserializer::<u64>())
        .next()
        .unwrap()
        .unwrap_err();
    let DecodeLeafError::Version(source) = error else {
        panic!("unexpected record error");
    };
    assert_eq!(source.kind(), std::io::ErrorKind::UnexpectedEof);

    let mut version = Vec::new();
    ciborium::ser::into_writer(&Version::new(), &mut version).unwrap();
    let mut missing_message = (version.len() as u32).to_be_bytes().to_vec();
    missing_message.extend_from_slice(&version);
    let run = LeafRun::from_encoded(missing_message).unwrap();
    let error = run
        .records(Message::deserializer::<u64>())
        .next()
        .unwrap()
        .unwrap_err();
    let DecodeLeafError::Message(source) = error else {
        panic!("unexpected record error");
    };
    assert_eq!(source.kind(), std::io::ErrorKind::UnexpectedEof);

    // Bytes past the canonical pair make the payload malformed: the
    // payload runs to the record's end, so the deserializer's
    // exactly-one-value check is what rejects the excess.
    ciborium::ser::into_writer(&0_u64, &mut version).unwrap();
    version.push(u8::MIN);
    let mut trailing = (version.len() as u32).to_be_bytes().to_vec();
    trailing.extend_from_slice(&version);
    let run = LeafRun::from_encoded(trailing).unwrap();
    let error = run
        .records(Message::deserializer::<u64>())
        .next()
        .unwrap()
        .unwrap_err();
    let DecodeLeafError::Message(source) = error else {
        panic!("unexpected record error");
    };
    assert_eq!(source.kind(), std::io::ErrorKind::InvalidData);
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
        let error = decode_exact(speaker, RunBudget::default(), &encoded).unwrap_err();
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
        let error = decode_exact(speaker, RunBudget::default(), &encoded).unwrap_err();
        assert_eq!(error.origin, Origin::stream(speaker, stream));
        assert!(matches!(
            error.kind,
            DecodeErrorKind::TrailingBytes {
                count: WireSignal::ENCODED_LEN
            }
        ));

        let mut rest = encoded.as_slice();
        let frame = decode(speaker, RunBudget::default(), &mut rest).unwrap();
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
        let mut closed = FrameRead::new(speaker, RunBudget::default(), &[][..]);
        assert_eq!(pollster::block_on(closed.frame()).unwrap(), None);

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
                FramePart::SupplyRun,
            ),
        ];
        for (encoded, missing) in cases {
            let mut reader = FrameRead::new(speaker, RunBudget::default(), encoded.as_slice());
            let error = pollster::block_on(reader.frame()).unwrap_err();
            assert_eq!(error.origin, Origin::stream(speaker, stream));
            assert!(matches!(
                error.kind,
                DecodeErrorKind::Truncated {
                    missing: actual,
                    source,
                } if actual == missing && source.kind() == std::io::ErrorKind::UnexpectedEof
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
                Signal::End(End::Stream),
                Frame::End(End::Stream),
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
        let mut reader = FrameRead::new(speaker, RunBudget::default(), bytes.as_slice());

        let error = pollster::block_on(reader.frame()).unwrap_err();
        assert_eq!(error.origin, Origin::stream(speaker, stream));
        assert!(matches!(
            error.kind,
            DecodeErrorKind::InvalidSignal(DecodeSignalError::Placement(_))
        ));
        assert_eq!(
            pollster::block_on(reader.frame()).unwrap(),
            Some((stream, valid_frame)),
        );
    }
}

struct FailingReader;

impl std::io::Read for FailingReader {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::ErrorKind::Other.into())
    }
}

/// Reader failures before the signal retain their frame part and speaker.
#[test]
fn reader_errors_are_contextual() {
    for speaker in SPEAKERS {
        let error = decode(speaker, RunBudget::default(), &mut FailingReader).unwrap_err();
        assert_eq!(error.origin, Origin::direction(speaker));
        assert!(matches!(
            error.kind,
            DecodeErrorKind::Read {
                part: FramePart::Signal,
                source,
            } if source.kind() == std::io::ErrorKind::Other
        ));
    }
}

/// Supply-body truncation cuts landing one byte short of, exactly on, and
/// one byte past each payload chunk boundary all classify as a truncated
/// `SupplyRun` with an `UnexpectedEof` source.
///
/// The chunked body read preserves the typed truncation contract at every
/// seam.
#[test]
fn supply_truncation_at_chunk_boundaries_is_typed() {
    use crate::tree::mirror::framing::{PAYLOAD_CHUNK_LEN, chunk_boundary_cuts};

    let declared = 2 * PAYLOAD_CHUNK_LEN + 5;
    let stream = stream(6);
    for speaker in SPEAKERS {
        for delivered in chunk_boundary_cuts(declared) {
            let mut encoded = vec![signal(stream, Signal::Supply(Flow::Continue))];
            encoded.extend_from_slice(&u32::try_from(declared).unwrap().to_be_bytes());
            encoded.extend(vec![0xA5; delivered]);
            let mut reader = FrameRead::new(speaker, RunBudget::default(), encoded.as_slice());
            let error = pollster::block_on(reader.frame()).unwrap_err();
            assert_eq!(error.origin, Origin::stream(speaker, stream));
            assert!(
                matches!(
                    error.kind,
                    DecodeErrorKind::Truncated {
                        missing: FramePart::SupplyRun,
                        ref source,
                    } if source.kind() == std::io::ErrorKind::UnexpectedEof
                ),
                "cut after {delivered} delivered body bytes"
            );
        }
    }
}

/// The full wire size of a supply frame carrying `body` run bytes.
fn frame_wire_size(body: &[u8]) -> usize {
    super::super::SUPPLY_FRAME_OVERHEAD + body.len()
}

/// A decode failure's classification with its I/O source elided: the two
/// decoders share every typed field but wrap different reader libraries,
/// whose error texts legitimately differ.
fn kind_signature(kind: &DecodeErrorKind) -> String {
    match kind {
        DecodeErrorKind::Read { part, .. } => format!("Read({part:?})"),
        DecodeErrorKind::Truncated { missing, .. } => format!("Truncated({missing:?})"),
        other => format!("{other:?}"),
    }
}

/// Decode one frame from `bytes` through both decoders — the async reader
/// and the sync oracle — requiring them to classify identically, and
/// return the shared outcome.
fn decode_both(
    speaker: Speaker,
    budget: RunBudget,
    bytes: &[u8],
) -> Result<WireFrame, DecodeError> {
    let mut reader = FrameRead::new(speaker, budget, bytes);
    let from_async = pollster::block_on(reader.frame())
        .map(|frame| frame.expect("a nonempty byte stream is not a clean close"));
    let mut rest = bytes;
    let from_sync = decode(speaker, budget, &mut rest);
    match (from_async, from_sync) {
        (Ok(a), Ok(s)) => {
            assert_eq!(a, s, "the two decoders accept different frames");
            Ok(a)
        }
        (Err(a), Err(s)) => {
            assert_eq!(
                kind_signature(&a.kind),
                kind_signature(&s.kind),
                "the two decoders classify the failure differently"
            );
            assert_eq!(a.origin, s.origin);
            Err(a)
        }
        (a, s) => panic!("the two decoders disagree: async {a:?}, sync {s:?}"),
    }
}

proptest! {
    /// Ingress enforces the run budget as the exact complement of the
    /// encoder's flush rule, deciding before any body byte is read.
    ///
    /// A multi-record supply frame decodes when its full wire size is
    /// within the budget and fails typed as `OverbatchedRun` — carrying
    /// that wire size and the budget — when it is past it. The rejection
    /// is decided ahead of the body: a stream ending right after the
    /// first record's length header still classifies as the budget
    /// violation, never as a truncation. Both decoders (the async reader
    /// and the sync oracle) agree throughout.
    #[test]
    fn multi_record_frames_are_held_to_the_run_budget(
        index in 1_u8..Stream::MAX,
        speaker in arb_speaker(),
        flow in arb_flow(),
        records in proptest::collection::vec((arb_version(), any::<u64>()), 2..=4),
        surplus in 0_usize..64,
        deficit in 1_usize..64,
    ) {
        let stream = stream(index);
        let mut body = Vec::new();
        for (version, value) in &records {
            body.extend_from_slice(&record(version, &Message::new(*value)));
        }
        let encoded = supply(stream, flow, &body);
        let wire_size = frame_wire_size(&body);

        // Within budget (boundary included): the batching decodes.
        let within = RunBudget::from_bytes(wire_size + surplus);
        let expected = LeafRun::from_encoded(body.clone()).unwrap();
        prop_assert_eq!(
            decode_both(speaker, within, &encoded).expect("a within-budget batching decodes"),
            (stream, Frame::Reaction(Reaction::Supply(expected), flow))
        );

        // Past the budget: typed rejection naming the frame and the budget.
        let over = RunBudget::from_bytes(wire_size.saturating_sub(deficit));
        let error = decode_both(speaker, over, &encoded).expect_err(
            "undetected over-budget batching: a multi-record frame past the \
             budget must fail typed",
        );
        prop_assert_eq!(error.origin, Origin::stream(speaker, stream));
        let typed = matches!(
            error.kind,
            DecodeErrorKind::OverbatchedRun { declared, budget }
                if declared == wire_size && budget == over.bytes()
        );
        prop_assert!(typed, "mistyped over-budget batching: {:?}", error.kind);

        // Before the body read: the same rejection from only the signal,
        // the run length header, and the first record's length header — no
        // body byte exists to read, so a decoder that buffered the body
        // first would classify this as a truncation instead.
        let prefix = &encoded[..1 + LENGTH_HEADER_LEN + LENGTH_HEADER_LEN];
        let error = decode_both(speaker, over, prefix).expect_err(
            "undetected over-budget batching: the violation must be decided \
             ahead of the body",
        );
        let early = matches!(
            error.kind,
            DecodeErrorKind::OverbatchedRun { declared, budget }
                if declared == wire_size && budget == over.bytes()
        );
        prop_assert!(
            early,
            "over-budget batching was not rejected before the body read: {:?}",
            error.kind
        );
    }

    /// A single record larger than the run budget still decodes.
    ///
    /// The encoder's minimum-one-record rule ships such a record alone, so
    /// the ingress check admits the lone-record overhang at any budget —
    /// the no-false-positive half of the enforcement, in both decoders.
    #[test]
    fn oversized_lone_record_still_decodes(
        index in 1_u8..Stream::MAX,
        speaker in arb_speaker(),
        flow in arb_flow(),
        (version, value) in (arb_version(), any::<u64>()),
        budget_bytes in 0_usize..64,
    ) {
        let stream = stream(index);
        let body = record(&version, &Message::new(value));
        let encoded = supply(stream, flow, &body);
        // Clamp under the frame's wire size so the frame always overhangs.
        let over = RunBudget::from_bytes(budget_bytes.min(frame_wire_size(&body) - 1));
        let expected = LeafRun::from_encoded(body.clone()).unwrap();
        prop_assert_eq!(
            decode_both(speaker, over, &encoded)
                .expect("a lone record past the budget is the legal overhang"),
            (stream, Frame::Reaction(Reaction::Supply(expected), flow))
        );
    }
}

/// Corner classifications of the run-budget ingress check, under a zero
/// budget so every frame overhangs.
///
/// An over-budget body too short to hold a record header is the
/// violation, decided on the declared length alone (no body byte follows,
/// yet the error is not a truncation); a first record header that falls
/// short of the body or overruns it is the violation; a stream ending
/// inside the first record header, or inside an admitted lone record's
/// body, is a truncated supply run.
#[test]
fn overbatched_corners_classify_exactly() {
    let stream = stream(9);
    let zero = RunBudget::from_bytes(0);
    for speaker in SPEAKERS {
        // Declared bodies too short for a record header, none delivered.
        for declared in 0..LENGTH_HEADER_LEN {
            let mut encoded = vec![signal(stream, Signal::Supply(Flow::End))];
            encoded.extend_from_slice(&(declared as u32).to_be_bytes());
            let error = decode_both(speaker, zero, &encoded)
                .expect_err("a headerless over-budget body cannot decode");
            assert!(
                matches!(error.kind, DecodeErrorKind::OverbatchedRun { .. }),
                "declared {declared}: {:?}",
                error.kind
            );
        }

        // A first record header falling short of the body (two records'
        // shapes) and one overrunning it: both are the violation.
        let two_records = [record(&Version::new(), &Message::new(1))]
            .concat()
            .repeat(2);
        let mut overrun = (200_u32).to_be_bytes().to_vec();
        overrun.extend_from_slice(&[0; 8]);
        for body in [two_records, overrun] {
            let error = decode_both(speaker, zero, &supply(stream, Flow::End, &body))
                .expect_err("a non-spanning first record cannot decode over budget");
            assert!(
                matches!(error.kind, DecodeErrorKind::OverbatchedRun { .. }),
                "{:?}",
                error.kind
            );
        }

        // Ends inside the first record header, and inside an admitted lone
        // record's body: truncations of the supply run, not violations.
        let lone = record(&Version::new(), &Message::new(1));
        let encoded = supply(stream, Flow::End, &lone);
        let header_end = 1 + LENGTH_HEADER_LEN;
        for cut in [header_end + 2, encoded.len() - 1] {
            let error = decode_both(speaker, zero, &encoded[..cut])
                .expect_err("a truncated frame cannot decode");
            assert!(
                matches!(
                    error.kind,
                    DecodeErrorKind::Truncated {
                        missing: FramePart::SupplyRun,
                        ..
                    }
                ),
                "cut at {cut}: {:?}",
                error.kind
            );
        }
    }
}
