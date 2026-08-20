use crate::message::{PayloadCodec, PayloadDepthLimit};
use proptest::prelude::*;

use super::*;
use crate::Version;
use crate::message::Message;
use crate::tree::arb::arb_version;
use crate::tree::mirror::cbor::{MAJOR_BSTR, MAJOR_MAP, MAJOR_TAG, TAG_CBOR_SEQUENCE};
use crate::tree::typed::{Hash, hash::MERKLE_HASH_LEN};

use super::super::{
    error::{DecodeLeafError, Origin, QueryOrderError},
    frame::{LeafRunError, MAX_QUERY_CHILDREN, RECORD_TAG_LEN},
    signal::{DecodeSignalError, End, Flow, Speaker, Stream, StreamError},
};

const SPEAKERS: [Speaker; 2] = [Speaker::Initiator, Speaker::Responder];

fn stream(index: u8) -> Stream {
    Stream::new(index).unwrap()
}

fn signal(stream: Stream, signal: Signal) -> u8 {
    WireSignal::new(Speaker::Initiator, stream, signal)
        .unwrap()
        .to_byte()
}

/// The frame head of a `arity`-item frame carrying `code`: the array head
/// then the signal's unsigned-int head.
fn frame_head(arity: u64, code: u8) -> Vec<u8> {
    let mut head = Vec::new();
    cbor::write_head(&mut head, cbor::MAJOR_ARRAY, arity);
    cbor::write_head(&mut head, MAJOR_UINT, u64::from(code));
    head
}

/// A whole body-free frame.
fn bare_frame(stream: Stream, s: Signal) -> Vec<u8> {
    frame_head(1, signal(stream, s))
}

/// A whole supply frame declaring `body.len()` run bytes and carrying
/// `body`.
fn supply(stream: Stream, flow: Flow, body: &[u8]) -> Vec<u8> {
    supply_declaring(stream, flow, body.len(), body)
}

/// A supply frame declaring `declared` run bytes while carrying `body`.
fn supply_declaring(stream: Stream, flow: Flow, declared: usize, body: &[u8]) -> Vec<u8> {
    let mut encoded = frame_head(2, signal(stream, Signal::Supply(flow)));
    cbor::write_head(&mut encoded, MAJOR_TAG, TAG_CBOR_SEQUENCE);
    cbor::write_head(&mut encoded, MAJOR_BSTR, declared as u64);
    encoded.extend_from_slice(body);
    encoded
}

/// A whole query frame carrying `children` as its listing map, written
/// raw (no canonical-order validation) so tests can synthesize
/// violations.
fn query(stream: Stream, flow: Flow, children: &[(u8, Hash)]) -> Vec<u8> {
    let mut encoded = frame_head(2, signal(stream, Signal::Query(flow)));
    super::super::frame::write_listing(&mut encoded, children);
    encoded
}

/// One leaf record as it appears inside a run body: the embedded-sequence
/// tag and byte-string head, then the tagged version atom, then the
/// payload's CBOR bytes bare.
fn record(version: &Version, message: &Message) -> Vec<u8> {
    let mut content = Vec::new();
    cbor::write_head(&mut content, MAJOR_TAG, crate::tags::VERSION_TAG);
    ciborium::ser::into_writer(version, &mut content).unwrap();
    content.extend_from_slice(message.as_slice());
    let mut record = Vec::new();
    cbor::write_head(&mut record, MAJOR_TAG, TAG_CBOR_SEQUENCE);
    cbor::write_head(&mut record, MAJOR_BSTR, content.len() as u64);
    record.extend_from_slice(&content);
    record
}

/// A record item wrapping raw content bytes, for malformed-content cases.
fn raw_record(content: &[u8]) -> Vec<u8> {
    let mut record = Vec::new();
    cbor::write_head(&mut record, MAJOR_TAG, TAG_CBOR_SEQUENCE);
    cbor::write_head(&mut record, MAJOR_BSTR, content.len() as u64);
    record.extend_from_slice(content);
    record
}

/// A record's content for `version` and `message`, without its item heads.
fn record_content(version: &Version, message: &Message) -> Vec<u8> {
    let mut content = Vec::new();
    cbor::write_head(&mut content, MAJOR_TAG, crate::tags::VERSION_TAG);
    ciborium::ser::into_writer(version, &mut content).unwrap();
    content.extend_from_slice(message.as_slice());
    content
}

fn arb_speaker() -> impl Strategy<Value = Speaker> {
    prop_oneof![Just(Speaker::Initiator), Just(Speaker::Responder)]
}

fn arb_flow() -> impl Strategy<Value = Flow> {
    prop_oneof![Just(Flow::Continue), Just(Flow::End)]
}

/// The stream constructor rejects an index past the stream range with a
/// typed error naming the index.
#[test]
fn out_of_range_stream_index_is_rejected() {
    assert_eq!(
        Stream::new(Stream::COUNT),
        Err(StreamError::Invalid {
            index: Stream::COUNT
        })
    );
}

/// Reserved signal codes within the byte range retain the stream encoded
/// alongside them; codes past the byte range and non-int signal items are
/// malformed signals.
#[test]
fn invalid_signals_are_rejected() {
    for byte in WireSignal::BYTE_COUNT..=u8::MAX {
        for speaker in SPEAKERS {
            let invalid = WireSignal::from_byte(speaker, byte).unwrap_err();
            let DecodeSignalError::Reserved(reserved) = invalid else {
                panic!("unexpected signal error")
            };
            let encoded = frame_head(1, byte);
            let error = decode_exact(speaker, RunBudget::default(), &encoded).unwrap_err();
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
    // Past the byte range, and a non-int item where the signal belongs.
    for speaker in SPEAKERS {
        let mut encoded = Vec::new();
        cbor::write_head(&mut encoded, cbor::MAJOR_ARRAY, 1);
        cbor::write_head(&mut encoded, MAJOR_UINT, 256);
        let error = decode_exact(speaker, RunBudget::default(), &encoded).unwrap_err();
        assert_eq!(error.origin, Origin::direction(speaker));
        assert!(matches!(
            error.kind,
            DecodeErrorKind::Malformed {
                part: FramePart::Signal,
                ..
            }
        ));

        let mut encoded = Vec::new();
        cbor::write_head(&mut encoded, cbor::MAJOR_ARRAY, 1);
        cbor::write_head(&mut encoded, MAJOR_BSTR, 0);
        let error = decode_exact(speaker, RunBudget::default(), &encoded).unwrap_err();
        assert!(matches!(
            error.kind,
            DecodeErrorKind::Malformed {
                part: FramePart::Signal,
                ..
            }
        ));
    }
}

/// A frame item that is not a one- or two-element array, or whose array
/// length contradicts its signal's body arity, is rejected typed.
#[test]
fn frame_shape_is_enforced() {
    let stream = stream(4);
    for speaker in SPEAKERS {
        // Not an array at all.
        let error = decode_exact(speaker, RunBudget::default(), &[0x00]).unwrap_err();
        assert!(matches!(error.kind, DecodeErrorKind::FrameShape { .. }));
        // A three-item array.
        let error = decode_exact(speaker, RunBudget::default(), &[0x83]).unwrap_err();
        assert!(matches!(error.kind, DecodeErrorKind::FrameShape { .. }));
        // A body-free signal inside a two-item array.
        let encoded = frame_head(2, signal(stream, Signal::Match(Flow::Continue)));
        let error = decode_exact(speaker, RunBudget::default(), &encoded).unwrap_err();
        assert_eq!(error.origin, Origin::stream(speaker, stream));
        assert!(matches!(
            error.kind,
            DecodeErrorKind::FrameArity {
                expected: 1,
                found: 2
            }
        ));
        // A body-bearing signal inside a one-item array.
        let encoded = frame_head(1, signal(stream, Signal::Query(Flow::Continue)));
        let error = decode_exact(speaker, RunBudget::default(), &encoded).unwrap_err();
        assert!(matches!(
            error.kind,
            DecodeErrorKind::FrameArity {
                expected: 2,
                found: 1
            }
        ));
    }
}

/// A widened (non-shortest-form) signal head is rejected: the wire admits
/// one spelling per value.
#[test]
fn widened_signal_heads_are_rejected() {
    let stream = stream(3);
    let code = signal(stream, Signal::Match(Flow::Continue));
    for speaker in SPEAKERS {
        // The code spelled with a needlessly wide argument.
        let encoded = [0x81, 0x19, 0x00, code];
        let error = decode_exact(speaker, RunBudget::default(), &encoded).unwrap_err();
        assert!(matches!(
            error.kind,
            DecodeErrorKind::Malformed {
                part: FramePart::Signal,
                ..
            }
        ));
    }
}

/// Truncation identifies both the absent component and its known origin.
#[test]
fn truncated_bodies_are_rejected() {
    let stream = stream(4);
    for speaker in SPEAKERS {
        let query_head = frame_head(2, signal(stream, Signal::Query(Flow::Continue)));
        let mut half_listing = query_head.clone();
        cbor::write_head(&mut half_listing, MAJOR_MAP, 1);
        let supply_head = frame_head(2, signal(stream, Signal::Supply(Flow::Continue)));
        let cases = [
            (Vec::new(), FramePart::FrameHead, Origin::direction(speaker)),
            (vec![0x81], FramePart::Signal, Origin::direction(speaker)),
            (
                query_head,
                FramePart::QueryChildren,
                Origin::stream(speaker, stream),
            ),
            (
                half_listing,
                FramePart::QueryChildren,
                Origin::stream(speaker, stream),
            ),
            (
                supply_head.clone(),
                FramePart::SupplyLength,
                Origin::stream(speaker, stream),
            ),
            (
                {
                    let mut frame = supply_head;
                    cbor::write_head(&mut frame, MAJOR_TAG, TAG_CBOR_SEQUENCE);
                    cbor::write_head(&mut frame, MAJOR_BSTR, 4);
                    frame
                },
                FramePart::SupplyRun,
                Origin::stream(speaker, stream),
            ),
        ];
        for (encoded, missing, origin) in cases {
            let error = decode_exact(speaker, RunBudget::default(), &encoded).unwrap_err();
            assert_eq!(error.origin, origin, "case {missing:?}");
            let DecodeErrorKind::Truncated {
                missing: actual,
                source,
            } = error.kind
            else {
                panic!("unexpected error kind for {missing:?}: {:?}", error.kind);
            };
            assert_eq!(actual, missing);
            assert_eq!(source.kind(), std::io::ErrorKind::UnexpectedEof);
        }
    }
}

proptest! {
    /// An arbitrary run of supplied records decodes into a frame carrying the
    /// exact run body, byte for byte.
    ///
    /// Deferral of record decoding is what
    /// `a_zero_length_record_is_structurally_valid` pins: its record only a
    /// non-eager decoder can accept; this body's records are all well-formed,
    /// so byte equality alone cannot tell eager from lazy.
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

/// Structurally invalid runs are rejected at the wire with their exact
/// cause: an empty run, bytes that are no record item, or a record's
/// content past the run's end.
#[test]
fn malformed_run_structure_is_typed() {
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

        // Bytes where a record item belongs that are not one.
        let not_a_record = decode_exact(
            speaker,
            RunBudget::default(),
            &supply(stream, Flow::Continue, &[0x00, 0x00]),
        )
        .unwrap_err();
        assert_eq!(not_a_record.origin, Origin::stream(speaker, stream));
        assert!(matches!(
            not_a_record.kind,
            DecodeErrorKind::InvalidRun(LeafRunError::NotARecord { remaining: 2, .. })
        ));

        // A record declaring more content than the run holds.
        let overrun = raw_record(&[0, 0])[..RECORD_TAG_LEN + 2].to_vec();
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

/// An empty-content record inside a run body is structurally valid.
///
/// From raw wire bytes, a run body of one record whose byte string is
/// empty chains exactly, so the codec accepts the frame and defers the
/// record's failure to its record iterator: the empty content cannot hold
/// a tagged version, and the iterator reports the version decode failure.
#[test]
fn a_zero_length_record_is_structurally_valid() {
    let stream = stream(8);
    let encoded = supply(stream, Flow::End, &raw_record(&[]));
    for speaker in SPEAKERS {
        let (decoded_stream, frame) =
            decode_exact(speaker, RunBudget::default(), &encoded).unwrap();
        assert_eq!(decoded_stream, stream);
        let Frame::Reaction(Reaction::Supply(run), Flow::End) = frame else {
            panic!("a structurally valid run decodes as a supply reaction");
        };
        assert_eq!(run.record_count(), 1);
        let error = run
            .records(PayloadCodec::new::<u64>(PayloadDepthLimit::default()))
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
    // A version byte string promising two bytes, cut short after one.
    let mut content = Vec::new();
    cbor::write_head(&mut content, MAJOR_TAG, crate::tags::VERSION_TAG);
    content.extend_from_slice(&[0x42, 0x01]);
    let run = LeafRun::from_encoded(raw_record(&content)).unwrap();
    let error = run
        .records(PayloadCodec::new::<u64>(PayloadDepthLimit::default()))
        .next()
        .unwrap()
        .unwrap_err();
    let DecodeLeafError::Version(source) = error else {
        panic!("unexpected record error");
    };
    assert_eq!(source.kind(), std::io::ErrorKind::UnexpectedEof);

    // An untagged version where the tagged atom belongs.
    let mut content = Vec::new();
    ciborium::ser::into_writer(&Version::new(), &mut content).unwrap();
    let run = LeafRun::from_encoded(raw_record(&content)).unwrap();
    let error = run
        .records(PayloadCodec::new::<u64>(PayloadDepthLimit::default()))
        .next()
        .unwrap()
        .unwrap_err();
    let DecodeLeafError::Version(source) = error else {
        panic!("unexpected record error");
    };
    assert_eq!(source.kind(), std::io::ErrorKind::InvalidData);

    // A tagged version with no message behind it.
    let content = record_content(&Version::new(), &Message::new(0u64));
    let missing_message = &content[..content.len() - Message::new(0u64).as_slice().len()];
    let run = LeafRun::from_encoded(raw_record(missing_message)).unwrap();
    let error = run
        .records(PayloadCodec::new::<u64>(PayloadDepthLimit::default()))
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
    let mut content = record_content(&Version::new(), &Message::new(0u64));
    content.push(u8::MIN);
    let run = LeafRun::from_encoded(raw_record(&content)).unwrap();
    let error = run
        .records(PayloadCodec::new::<u64>(PayloadDepthLimit::default()))
        .next()
        .unwrap()
        .unwrap_err();
    let DecodeLeafError::Message(source) = error else {
        panic!("unexpected record error");
    };
    assert_eq!(source.kind(), std::io::ErrorKind::InvalidData);
}

/// Pins the stated ingress boundary: the version atom's CBOR head is not
/// spelling-judged (the atom's content is, by `Version::decode`).
///
/// A record whose version byte string wears a widened two-byte-length
/// head still decodes. Flipping this to rejection is a deliberate
/// contract change, not drift.
#[test]
fn widened_version_atom_head_is_not_spelling_judged() {
    // The canonical atom bytes: ciborium serializes a version as a byte
    // string whose one-byte head's low bits carry the length; strip that
    // head to get the content alone.
    let mut atom = Vec::new();
    ciborium::ser::into_writer(&Version::new(), &mut atom).unwrap();
    let content_bytes = &atom[1..];

    let mut content = Vec::new();
    cbor::write_head(&mut content, MAJOR_TAG, crate::tags::VERSION_TAG);
    // The same byte string, its length spelled in the widened two-byte
    // form (major 2, additional info 25) instead of the shortest head.
    content.push(0x59);
    content.extend_from_slice(&u16::try_from(content_bytes.len()).unwrap().to_be_bytes());
    content.extend_from_slice(content_bytes);
    content.extend_from_slice(Message::new(0u64).as_slice());

    let run = LeafRun::from_encoded(raw_record(&content)).unwrap();
    let (version, _message) = run
        .records(PayloadCodec::new::<u64>(PayloadDepthLimit::default()))
        .next()
        .unwrap()
        .expect("a widened version-atom head decodes: spelling is not re-judged here");
    assert_eq!(version, Version::new());
}

/// Pins the stated ingress boundary: the version atom's CBOR head is not
/// spelling-judged, indefinite lengths included.
///
/// A record whose version byte string is spelled indefinite-length (one
/// definite segment of the canonical content, then the break) still
/// decodes. Flipping this to rejection is a deliberate contract change,
/// not drift.
#[test]
fn indefinite_version_atom_head_is_not_spelling_judged() {
    // The canonical atom bytes: ciborium serializes a version as a byte
    // string whose one-byte head's low bits carry the length; strip that
    // head to get the content alone.
    let mut atom = Vec::new();
    ciborium::ser::into_writer(&Version::new(), &mut atom).unwrap();
    let content_bytes = &atom[1..];

    let mut content = Vec::new();
    cbor::write_head(&mut content, MAJOR_TAG, crate::tags::VERSION_TAG);
    // The same bytes as an indefinite-length byte string: the start
    // marker (major 2, additional info 31), one definite segment holding
    // the canonical content, and the break.
    content.push(0x5f);
    cbor::write_head(&mut content, MAJOR_BSTR, content_bytes.len() as u64);
    content.extend_from_slice(content_bytes);
    content.push(0xff);
    content.extend_from_slice(Message::new(0u64).as_slice());

    let run = LeafRun::from_encoded(raw_record(&content)).unwrap();
    let (version, _message) = run
        .records(PayloadCodec::new::<u64>(PayloadDepthLimit::default()))
        .next()
        .unwrap()
        .expect("an indefinite version-atom head decodes: spelling is not re-judged here");
    assert_eq!(version, Version::new());
}

/// Pins the stated ingress boundary: the application payload is decoded
/// by a general CBOR reader that does not judge spelling.
///
/// A record whose payload is the indefinite-length empty map (a spelling
/// the emitter never writes) still decodes. Flipping this to rejection
/// is a deliberate contract change, not drift.
#[test]
fn indefinite_payload_spelling_is_not_spelling_judged() {
    let mut content = Vec::new();
    cbor::write_head(&mut content, MAJOR_TAG, crate::tags::VERSION_TAG);
    ciborium::ser::into_writer(&Version::new(), &mut content).unwrap();
    // The indefinite-length empty map: start marker, then the break.
    content.extend_from_slice(&[0xbf, 0xff]);

    let run = LeafRun::from_encoded(raw_record(&content)).unwrap();
    let (version, message) = run
        .records(PayloadCodec::new::<std::collections::BTreeMap<u8, u8>>(
            PayloadDepthLimit::default(),
        ))
        .next()
        .unwrap()
        .expect("an indefinite-length payload decodes: spelling is not judged here");
    assert_eq!(version, Version::new());
    assert_eq!(
        *message.arc::<std::collections::BTreeMap<u8, u8>>(),
        std::collections::BTreeMap::new()
    );
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
        let encoded = query(stream, Flow::Continue, &children);
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

    /// An arbitrary canonical query round-trips through the decoder.
    #[test]
    fn canonical_queries_decode(
        index in 1_u8..Stream::MAX,
        speaker in arb_speaker(),
        flow in arb_flow(),
        radixes in proptest::collection::btree_set(any::<u8>(), 1..=32),
    ) {
        let stream = stream(index);
        let children: Vec<(u8, Hash)> = radixes
            .iter()
            .map(|&radix| (radix, Hash([radix; MERKLE_HASH_LEN])))
            .collect();
        let encoded = query(stream, flow, &children);
        prop_assert_eq!(
            decode_exact(speaker, RunBudget::default(), &encoded).unwrap(),
            (stream, Frame::Reaction(Reaction::Query(children), flow))
        );
    }
}

/// A query body whose listing map is empty is rejected: an empty query
/// travels as its own signal, so the map spelling requires at least one
/// child (the upper bound is pinned by
/// `oversized_query_listing_is_rejected`).
#[test]
fn empty_query_listing_is_rejected() {
    let stream = stream(5);
    let encoded = query(stream, Flow::Continue, &[]);
    for speaker in SPEAKERS {
        let error = decode_exact(speaker, RunBudget::default(), &encoded).unwrap_err();
        assert!(matches!(
            error.kind,
            DecodeErrorKind::Malformed {
                part: FramePart::QueryChildren,
                ..
            }
        ));
    }
}

/// A query listing declaring more children than the radix space holds is
/// rejected at its map head, before any entry is read: the map spelling
/// admits at most one child per radix value.
#[test]
fn oversized_query_listing_is_rejected() {
    let stream = stream(5);
    let mut encoded = frame_head(2, signal(stream, Signal::Query(Flow::Continue)));
    // A map head declaring one entry past the radix space, with no
    // entries behind it: the rejection is decided on the head alone, in
    // both decoders.
    cbor::write_head(&mut encoded, MAJOR_MAP, MAX_QUERY_CHILDREN as u64 + 1);
    for speaker in SPEAKERS {
        let error = decode_both(speaker, RunBudget::default(), &encoded)
            .expect_err("a listing past the radix space cannot decode");
        assert_eq!(error.origin, Origin::stream(speaker, stream));
        assert!(matches!(
            error.kind,
            DecodeErrorKind::Malformed {
                part: FramePart::QueryChildren,
                detail: "listing exceeds the radix space",
            }
        ));
    }
}

/// Exact decoding rejects a trailing frame while incremental decoding preserves it.
#[test]
fn exact_decode_rejects_trailing_frame() {
    let stream = stream(10);
    let first = bare_frame(stream, Signal::Match(Flow::Continue));
    let second = bare_frame(stream, Signal::End(End::Reply));
    let mut encoded = first.clone();
    encoded.extend_from_slice(&second);
    for speaker in SPEAKERS {
        let error = decode_exact(speaker, RunBudget::default(), &encoded).unwrap_err();
        assert_eq!(error.origin, Origin::stream(speaker, stream));
        assert!(matches!(
            error.kind,
            DecodeErrorKind::TrailingBytes { count } if count == second.len()
        ));

        let mut rest = encoded.as_slice();
        let frame = decode(speaker, RunBudget::default(), &mut rest).unwrap();
        assert_eq!(
            frame,
            (stream, Frame::Reaction(Reaction::Match, Flow::Continue))
        );
        assert_eq!(rest, second.as_slice());
    }
}

/// Async EOF is clean only before a frame head; every partial body reports
/// the same missing part and stream context in both decoders.
///
/// The clean close is checked async-only: the sync oracle's callers always
/// expect a frame, so it deliberately treats a clean close as a truncation.
#[test]
fn async_eof_distinguishes_close_from_truncation() {
    let stream = stream(4);
    for speaker in SPEAKERS {
        let mut closed = FrameRead::new(speaker, RunBudget::default(), &[][..]);
        assert_eq!(pollster::block_on(closed.frame()).unwrap(), None);

        let supply_head = frame_head(2, signal(stream, Signal::Supply(Flow::Continue)));
        let cases = [
            (
                frame_head(2, signal(stream, Signal::Query(Flow::Continue))),
                FramePart::QueryChildren,
            ),
            (
                {
                    let mut frame = frame_head(2, signal(stream, Signal::Query(Flow::Continue)));
                    cbor::write_head(&mut frame, MAJOR_MAP, 1);
                    frame
                },
                FramePart::QueryChildren,
            ),
            (supply_head.clone(), FramePart::SupplyLength),
            (
                {
                    let mut frame = supply_head;
                    cbor::write_head(&mut frame, MAJOR_TAG, TAG_CBOR_SEQUENCE);
                    cbor::write_head(&mut frame, MAJOR_BSTR, 4);
                    frame
                },
                FramePart::SupplyRun,
            ),
        ];
        for (encoded, missing) in cases {
            let error = decode_both(speaker, RunBudget::default(), &encoded)
                .expect_err("a truncated frame cannot decode");
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

/// An invalid async signal consumes only its own frame, leaving the
/// following valid frame at the next exact boundary.
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
        let mut bytes = frame_head(1, invalid);
        bytes.extend_from_slice(&frame_head(1, valid));
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
                part: FramePart::FrameHead,
                source,
            } if source.kind() == std::io::ErrorKind::Other
        ));
    }
}

/// Supply-body truncation cuts at every seeded offset all classify as a
/// truncated `SupplyRun` with an `UnexpectedEof` source.
///
/// The seeded offsets are one byte short of, exactly on, and one byte
/// past each payload chunk boundary, plus the zero-byte, one-byte, and
/// one-short-of-total cuts. The chunked body read preserves the typed
/// truncation contract at every seam, the zero- and one-byte cuts
/// exercising the earliest possible ones — where a record's leading
/// heads would sit.
#[test]
fn supply_truncation_at_chunk_boundaries_is_typed() {
    use crate::tree::mirror::framing::{PAYLOAD_CHUNK_LEN, chunk_boundary_cuts};

    let declared = 2 * PAYLOAD_CHUNK_LEN + 5;
    let stream = stream(6);
    for speaker in SPEAKERS {
        for delivered in chunk_boundary_cuts(declared) {
            let body = vec![0xA5; delivered];
            let encoded = supply_declaring(stream, Flow::Continue, declared, &body);
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

/// The charged wire size of a supply frame carrying `body` run bytes: the
/// budget envelope constant plus the body, the exact quantity `covers`
/// prices and `OverbatchedRun` reports.
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
    /// Ingress enforces the run budget, deciding from at most the first
    /// record's heads.
    ///
    /// A multi-record supply frame decodes when its charged wire size is
    /// within the budget and fails typed as `OverbatchedRun` — carrying
    /// that wire size and the budget — when it is past it. The rejection
    /// is decided ahead of the rest of the body: a stream ending right
    /// after the first record's heads still classifies as the budget
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
        let mut first_record_heads = 0;
        for (at, (version, value)) in records.iter().enumerate() {
            let record = record(version, &Message::new(*value));
            if at == 0 {
                let content = {
                    let mut input = record.as_slice();
                    super::super::frame::record_head(&mut input)
                        .expect("a built record has record heads");
                    record.len() - input.len()
                };
                first_record_heads = content;
            }
            body.extend_from_slice(&record);
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

        // From at most the first record's heads: the same rejection when
        // the stream ends right after them (the heads are the leading
        // body bytes the check reads) — a decoder that buffered the whole
        // body first would classify this as a truncation instead.
        let prefix = &encoded[..encoded.len() - body.len() + first_record_heads];
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
/// An over-budget body too short to hold a record's heads is the
/// violation, decided on the declared length alone (no body byte follows,
/// yet the error is not a truncation); a first record that falls short of
/// the body or overruns it is the violation; a stream ending inside the
/// first record's heads, or inside an admitted lone record's body, is a
/// truncated supply run.
#[test]
fn overbatched_corners_classify_exactly() {
    let stream = stream(9);
    let zero = RunBudget::from_bytes(0);
    for speaker in SPEAKERS {
        // Declared bodies too short for a record's heads, none delivered.
        for declared in 0..RECORD_TAG_LEN + 1 {
            let encoded = supply_declaring(stream, Flow::End, declared, &[]);
            let error = decode_both(speaker, zero, &encoded)
                .expect_err("a headless over-budget body cannot decode");
            assert!(
                matches!(error.kind, DecodeErrorKind::OverbatchedRun { .. }),
                "declared {declared}: {:?}",
                error.kind
            );
        }

        // A first record falling short of the body (two records' shapes)
        // and one overrunning it: both are the violation.
        let two_records = record(&Version::new(), &Message::new(1)).repeat(2);
        let overrun = {
            let mut record = Vec::new();
            cbor::write_head(&mut record, MAJOR_TAG, TAG_CBOR_SEQUENCE);
            cbor::write_head(&mut record, MAJOR_BSTR, 200);
            record.extend_from_slice(&[0; 8]);
            record
        };
        for body in [two_records, overrun] {
            let error = decode_both(speaker, zero, &supply(stream, Flow::End, &body))
                .expect_err("a non-spanning first record cannot decode over budget");
            assert!(
                matches!(error.kind, DecodeErrorKind::OverbatchedRun { .. }),
                "{:?}",
                error.kind
            );
        }

        // Ends inside the first record's heads, and inside an admitted lone
        // record's body: truncations of the supply run, not violations.
        let lone = record(&Version::new(), &Message::new(1));
        let encoded = supply(stream, Flow::End, &lone);
        let heads_end = encoded.len() - lone.len() + 1;
        for cut in [heads_end, encoded.len() - 1] {
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

/// A hand-crafted record whose payload nests one scope past the peer's
/// depth limit dies typed at wire ingress, while the same shape at
/// exactly the limit decodes clean, pinning the boundary.
///
/// Send-side admission binds only this crate's own senders, so a
/// nonconforming implementation's over-deep supply must still surface as
/// `DecodeLeafError::Message` (invalid data), never as a panic or an
/// untyped abort.
#[test]
fn an_over_deep_supplied_payload_dies_typed_at_ingress() {
    /// The receiving payload type: pure array nesting, the innermost
    /// array empty, matching the hand-crafted bytes below.
    #[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    struct Arr(Vec<Arr>);
    let limit = PayloadDepthLimit::default();
    let deep_payload = |depth: usize| -> Vec<u8> {
        // `depth - 1` single-element array heads around one empty array:
        // nesting depth is exactly `depth` scopes.
        let mut bytes = vec![0x81; depth - 1];
        bytes.push(0x80);
        bytes
    };
    let record_with_payload = |payload: &[u8]| -> Vec<u8> {
        let mut content = Vec::new();
        cbor::write_head(&mut content, MAJOR_TAG, crate::tags::VERSION_TAG);
        ciborium::ser::into_writer(&Version::new(), &mut content).unwrap();
        content.extend_from_slice(payload);
        content
    };
    let codec = PayloadCodec::new::<Arr>(limit);

    // One scope past the limit: typed rejection at the record iterator.
    let over = record_with_payload(&deep_payload(limit.get() as usize + 1));
    let run = LeafRun::from_encoded(raw_record(&over)).unwrap();
    let error = run.records(codec).next().unwrap().unwrap_err();
    let DecodeLeafError::Message(source) = error else {
        panic!("an over-deep payload must fail as a message decode error");
    };
    assert_eq!(source.kind(), std::io::ErrorKind::InvalidData);

    // Exactly at the limit: the same shape decodes clean.
    let at = record_with_payload(&deep_payload(limit.get() as usize));
    let run = LeafRun::from_encoded(raw_record(&at)).unwrap();
    run.records(codec)
        .next()
        .unwrap()
        .expect("a payload at exactly the limit decodes");
}
