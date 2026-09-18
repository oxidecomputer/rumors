use std::pin::Pin;
use std::task::{Context, Poll};

use crate::message::{PayloadCodec, PayloadDepthLimit};
use proptest::prelude::*;
use tokio::io::{AsyncRead, ReadBuf};

use super::*;
use crate::Version;
use crate::message::Message;
use crate::tree::arb::{arb_radixes, arb_version};
use crate::tree::mirror::cbor::{HeadError, MAJOR_BSTR, MAJOR_MAP, MAJOR_TAG, TAG_CBOR_SEQUENCE};
use crate::tree::typed::{Hash, hash::MERKLE_HASH_LEN};

use super::super::{
    error::{DecodeLeafError, OpenerItem, Origin, QueryOrderError, VersionDecodeError},
    frame::{
        LeafRunError, ListingIssue, ListingStructureError, MAX_QUERY_CHILDREN,
        MIN_RECORD_HEADS_LEN, RECORD_TAG_LEN,
    },
    signal::{DecodeSignalError, End, Flow, Speaker, Stream},
};

/// Both possible senders of a frame.
const SPEAKERS: [Speaker; 2] = [Speaker::Initiator, Speaker::Responder];

/// A checked logical stream for a test fixture.
fn stream(index: u8) -> Stream {
    Stream::new(index).unwrap()
}

/// The opener of an `arity`-item frame on `stream` carrying `signal`: the
/// array head, then the stream and state items.
fn frame_head(arity: u64, stream: Stream, signal: Signal) -> Vec<u8> {
    let mut head = Vec::new();
    cbor::write_head(&mut head, cbor::MAJOR_ARRAY, arity);
    cbor::write_head(&mut head, MAJOR_UINT, u64::from(stream.index()));
    cbor::write_head(&mut head, MAJOR_UINT, u64::from(signal.state()));
    head
}

/// A whole body-free frame.
fn bare_frame(stream: Stream, s: Signal) -> Vec<u8> {
    frame_head(2, stream, s)
}

/// A whole supply frame declaring `body.len()` run bytes and carrying
/// `body`.
fn supply(stream: Stream, flow: Flow, body: &[u8]) -> Vec<u8> {
    supply_declaring(stream, flow, body.len(), body)
}

/// A supply frame declaring `declared` run bytes while carrying `body`.
fn supply_declaring(stream: Stream, flow: Flow, declared: usize, body: &[u8]) -> Vec<u8> {
    let mut encoded = frame_head(3, stream, Signal::Supply(flow));
    cbor::write_head(&mut encoded, MAJOR_TAG, TAG_CBOR_SEQUENCE);
    cbor::write_head(&mut encoded, MAJOR_BSTR, declared as u64);
    encoded.extend_from_slice(body);
    encoded
}

/// A whole query frame carrying `children` as its listing map, written
/// raw (no canonical-order validation) so tests can synthesize
/// violations.
fn query(stream: Stream, flow: Flow, children: &[(u8, Hash)]) -> Vec<u8> {
    let mut encoded = frame_head(3, stream, Signal::Query(flow));
    super::super::frame::write_listing(&mut encoded, children);
    encoded
}

/// One leaf record as it appears inside a run body: the embedded-sequence
/// tag and byte-string head, then the tagged version atom, then the
/// payload's CBOR bytes bare.
fn record(version: &Version, message: &Message) -> Vec<u8> {
    raw_record(&record_content(version, message))
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

/// Either direction of a session.
fn arb_speaker() -> impl Strategy<Value = Speaker> {
    prop_oneof![Just(Speaker::Initiator), Just(Speaker::Responder)]
}

/// A reaction that continues or finishes its reply.
fn arb_flow() -> impl Strategy<Value = Flow> {
    prop_oneof![Just(Flow::Continue), Just(Flow::End)]
}

/// The stream constructor rejects an index past the stream range with an
/// error naming the index.
#[test]
fn out_of_range_stream_index_is_rejected() {
    assert!(matches!(
        Stream::new(Stream::COUNT),
        Err(error) if error.index() == Stream::COUNT
    ));
}

/// A reserved state code on a known stream is rejected naming that
/// stream; a reserved stream index is rejected against the direction;
/// a non-int item in either position is a malformed signal.
#[test]
fn invalid_openers_are_rejected() {
    let stream = stream(4);
    for speaker in SPEAKERS {
        for state in [u64::from(Signal::STATE_COUNT), u64::from(u8::MAX), 256] {
            let mut encoded = Vec::new();
            cbor::write_head(&mut encoded, cbor::MAJOR_ARRAY, 2);
            cbor::write_head(&mut encoded, MAJOR_UINT, u64::from(stream.index()));
            cbor::write_head(&mut encoded, MAJOR_UINT, state);
            let error = decode_exact(speaker, RunBudget::default(), &encoded).unwrap_err();
            assert_eq!(error.origin, Origin::stream(speaker, stream));
            let DecodeErrorKind::InvalidSignal(DecodeSignalError::State {
                stream: framed,
                state: rejected,
            }) = error.kind
            else {
                panic!("unexpected error kind: {:?}", error.kind);
            };
            assert_eq!((framed, rejected), (stream, state));
        }
        for index in [u64::from(Stream::COUNT), u64::from(u8::MAX), 256] {
            let mut encoded = Vec::new();
            cbor::write_head(&mut encoded, cbor::MAJOR_ARRAY, 2);
            cbor::write_head(&mut encoded, MAJOR_UINT, index);
            cbor::write_head(&mut encoded, MAJOR_UINT, 0);
            let error = decode_exact(speaker, RunBudget::default(), &encoded).unwrap_err();
            assert_eq!(error.origin, Origin::direction(speaker));
            assert!(matches!(
                error.kind,
                DecodeErrorKind::InvalidSignal(DecodeSignalError::Stream { index: rejected })
                    if rejected == index
            ));
        }
        // A non-int item where the stream belongs, and where the state
        // belongs.
        let mut encoded = Vec::new();
        cbor::write_head(&mut encoded, cbor::MAJOR_ARRAY, 2);
        cbor::write_head(&mut encoded, MAJOR_BSTR, 0);
        let error = decode_exact(speaker, RunBudget::default(), &encoded).unwrap_err();
        assert_eq!(error.origin, Origin::direction(speaker));
        assert!(matches!(
            error.kind,
            DecodeErrorKind::OpenerType {
                item: OpenerItem::Stream,
                ..
            }
        ));
        let mut encoded = Vec::new();
        cbor::write_head(&mut encoded, cbor::MAJOR_ARRAY, 2);
        cbor::write_head(&mut encoded, MAJOR_UINT, u64::from(stream.index()));
        cbor::write_head(&mut encoded, MAJOR_BSTR, 0);
        let error = decode_exact(speaker, RunBudget::default(), &encoded).unwrap_err();
        assert!(matches!(
            error.kind,
            DecodeErrorKind::OpenerType {
                item: OpenerItem::State,
                ..
            }
        ));
    }
}

/// A frame item that is not a two- or three-element array, or whose array
/// length contradicts its signal's body arity, is rejected typed.
#[test]
fn frame_shape_is_enforced() {
    let stream = stream(4);
    for speaker in SPEAKERS {
        // Not an array at all.
        let error = decode_exact(speaker, RunBudget::default(), &[0x00]).unwrap_err();
        assert!(matches!(error.kind, DecodeErrorKind::FrameType { .. }));
        // A one-item array, and a four-item array.
        for head in [0x81, 0x84] {
            let error = decode_exact(speaker, RunBudget::default(), &[head]).unwrap_err();
            assert!(matches!(error.kind, DecodeErrorKind::FrameLength { .. }));
        }
        // A body-free signal inside a three-item array.
        let encoded = frame_head(3, stream, Signal::Match(Flow::Continue));
        let error = decode_exact(speaker, RunBudget::default(), &encoded).unwrap_err();
        assert_eq!(error.origin, Origin::stream(speaker, stream));
        assert!(matches!(
            error.kind,
            DecodeErrorKind::FrameArity {
                expected: 2,
                found: 3
            }
        ));
        // A body-bearing signal inside a two-item array.
        let encoded = frame_head(2, stream, Signal::Query(Flow::Continue));
        let error = decode_exact(speaker, RunBudget::default(), &encoded).unwrap_err();
        assert!(matches!(
            error.kind,
            DecodeErrorKind::FrameArity {
                expected: 3,
                found: 2
            }
        ));
    }
}

/// A widened (non-shortest-form) state item is rejected: the wire admits
/// one spelling per value.
#[test]
fn widened_signal_heads_are_rejected() {
    let stream = stream(3);
    let state = Signal::Match(Flow::Continue).state();
    for speaker in SPEAKERS {
        // The state code spelled with a needlessly wide argument.
        let encoded = [0x82, stream.index(), 0x19, 0x00, state];
        let error = decode_exact(speaker, RunBudget::default(), &encoded).unwrap_err();
        assert!(matches!(
            error.kind,
            DecodeErrorKind::Head {
                part: FramePart::Signal,
                source: HeadError::NotShortest,
            }
        ));
    }
}

/// Truncation identifies both the absent component and its known origin.
#[test]
fn truncated_bodies_are_rejected() {
    let stream = stream(4);
    for speaker in SPEAKERS {
        let query_head = frame_head(3, stream, Signal::Query(Flow::Continue));
        let mut half_listing = query_head.clone();
        cbor::write_head(&mut half_listing, MAJOR_MAP, 1);
        let supply_head = frame_head(3, stream, Signal::Supply(Flow::Continue));
        let cases = [
            (Vec::new(), FramePart::FrameHead, Origin::direction(speaker)),
            (vec![0x82], FramePart::Signal, Origin::direction(speaker)),
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
        assert!(matches!(
            error,
            DecodeLeafError::Version(VersionDecodeError::TagHead(HeadError::Truncated))
        ));
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
    assert!(matches!(
        error,
        DecodeLeafError::Version(VersionDecodeError::Truncated {
            declared: 2,
            available: 1,
        })
    ));

    // An untagged version where the tagged atom belongs.
    let mut content = Vec::new();
    ciborium::ser::into_writer(&Version::new(), &mut content).unwrap();
    let run = LeafRun::from_encoded(raw_record(&content)).unwrap();
    let error = run
        .records(PayloadCodec::new::<u64>(PayloadDepthLimit::default()))
        .next()
        .unwrap()
        .unwrap_err();
    assert!(matches!(
        error,
        DecodeLeafError::Version(VersionDecodeError::Tag { .. })
    ));

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

/// Assert that `content` fails at the version head with `expected`.
fn assert_version_head_error(content: &[u8], expected: HeadError) {
    let run = LeafRun::from_encoded(raw_record(content)).unwrap();
    let error = run
        .records(PayloadCodec::new::<u64>(PayloadDepthLimit::default()))
        .next()
        .unwrap()
        .unwrap_err();
    assert!(matches!(
        error,
        DecodeLeafError::Version(VersionDecodeError::BytesHead(actual))
            if actual == expected
    ));
}

/// A widened version byte-string head is rejected with its typed
/// shortest-form defect.
#[test]
fn widened_version_atom_head_is_rejected() {
    let version = Version::new();
    let atom = version.as_bytes();

    let mut content = Vec::new();
    cbor::write_head(&mut content, MAJOR_TAG, crate::tags::VERSION_TAG);
    // Additional information 25 encodes the small atom length two bytes
    // wider than its canonical one-byte head.
    content.push(0x59);
    content.extend_from_slice(&u16::try_from(atom.len()).unwrap().to_be_bytes());
    content.extend_from_slice(atom);
    content.extend_from_slice(Message::new(0u64).as_slice());

    assert_version_head_error(&content, HeadError::NotShortest);
}

/// An indefinite version byte string is rejected with its typed
/// definite-length defect.
#[test]
fn indefinite_version_atom_head_is_rejected() {
    let version = Version::new();
    let atom = version.as_bytes();

    let mut content = Vec::new();
    cbor::write_head(&mut content, MAJOR_TAG, crate::tags::VERSION_TAG);
    // An indefinite byte string: its start marker, one definite segment,
    // and the break. The decoder rejects the start before reading either.
    content.push(0x5f);
    cbor::write_head(&mut content, MAJOR_BSTR, atom.len() as u64);
    content.extend_from_slice(atom);
    content.push(0xff);
    content.extend_from_slice(Message::new(0u64).as_slice());

    assert_version_head_error(&content, HeadError::Indefinite);
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
        (previous, radix) in (any::<u8>(), any::<u8>())
            .prop_map(|(a, b)| (a.max(b), a.min(b))),
    ) {
        let stream = stream(index);
        let children = vec![(previous, Hash::default()), (radix, Hash::default())];
        let encoded = query(stream, Flow::Continue, &children);
        let error = decode_both(speaker, RunBudget::default(), &encoded)
            .expect_err("a non-ascending listing cannot decode");
        prop_assert_eq!(error.origin, Origin::stream(speaker, stream));
        let correct = matches!(
            error.kind,
            DecodeErrorKind::InvalidListing(ListingIssue::Order(QueryOrderError {
                previous: actual_previous,
                radix: actual_radix,
            })) if actual_previous == previous && actual_radix == radix
        );
        prop_assert!(correct);
    }

    /// Every non-canonical spelling of a listing entry head is rejected by
    /// both decoders, naming the head's defect and the query listing it
    /// sits in.
    #[test]
    fn non_canonical_listing_heads_are_rejected(
        index in 1_u8..Stream::MAX,
        speaker in arb_speaker(),
        (entry, expected) in arb_listing_head_defect(),
    ) {
        let stream = stream(index);
        let mut encoded = frame_head(3, stream, Signal::Query(Flow::Continue));
        cbor::write_head(&mut encoded, MAJOR_MAP, 1);
        encoded.extend_from_slice(&entry);
        let error = decode_both(speaker, RunBudget::default(), &encoded)
            .expect_err("a listing with a non-canonical head cannot decode");
        prop_assert_eq!(error.origin, Origin::stream(speaker, stream));
        let named = matches!(
            error.kind,
            DecodeErrorKind::InvalidListing(ListingIssue::Head(actual)) if actual == expected
        );
        prop_assert!(named, "expected {expected:?}, got {:?}", error.kind);
    }

    /// An arbitrary canonical query round-trips through both decoders, and
    /// the asynchronous decoder stops exactly at the next frame boundary.
    #[test]
    fn canonical_queries_decode(
        index in 1_u8..Stream::MAX,
        speaker in arb_speaker(),
        flow in arb_flow(),
        radixes in arb_radixes(1..=32),
    ) {
        let stream = stream(index);
        let children: Vec<(u8, Hash)> = radixes
            .iter()
            .map(|&radix| (radix, Hash::from([radix; MERKLE_HASH_LEN])))
            .collect();
        let encoded = query(stream, flow, &children);
        let expected = (stream, Frame::Reaction(Reaction::Query(children), flow));
        prop_assert_eq!(
            decode_exact(speaker, RunBudget::default(), &encoded).unwrap(),
            expected.clone()
        );

        let mut repeated = encoded.clone();
        repeated.extend_from_slice(&encoded);
        let mut reader = FrameRead::new(speaker, RunBudget::default(), repeated.as_slice());
        prop_assert_eq!(pollster::block_on(reader.frame()).unwrap(), Some(expected.clone()));
        prop_assert_eq!(pollster::block_on(reader.frame()).unwrap(), Some(expected));
        prop_assert_eq!(pollster::block_on(reader.frame()).unwrap(), None);
    }
}

/// One listing entry with a non-canonical head, and the defect both
/// decoders must name.
///
/// The spellings: a widened key (a radix below 24 spelled with a one-byte
/// argument), a widened value head (the digest's length spelled with a
/// two-byte argument), an indefinite-length value head, or a reserved key
/// head. Each entry carries a full digest behind the defect, so nothing
/// but the head is wrong.
fn arb_listing_head_defect() -> impl Strategy<Value = (Vec<u8>, HeadError)> {
    let digest = [0u8; MERKLE_HASH_LEN];
    let canonical_value = move |entry: &mut Vec<u8>| {
        cbor::write_head(entry, MAJOR_BSTR, MERKLE_HASH_LEN as u64);
        entry.extend_from_slice(&digest);
    };
    prop_oneof![
        (0_u8..24).prop_map(move |radix| {
            let mut entry = vec![0x18, radix];
            canonical_value(&mut entry);
            (entry, HeadError::NotShortest)
        }),
        (0_u8..24).prop_map(move |radix| {
            let mut entry = vec![radix, 0x59, 0x00, MERKLE_HASH_LEN as u8];
            entry.extend_from_slice(&digest);
            (entry, HeadError::NotShortest)
        }),
        (0_u8..24).prop_map(move |radix| {
            let mut entry = vec![radix, 0x5f];
            entry.extend_from_slice(&digest);
            (entry, HeadError::Indefinite)
        }),
        Just({
            let mut entry = vec![0x1c];
            canonical_value(&mut entry);
            (entry, HeadError::Reserved)
        }),
    ]
}

/// A query body whose listing map is empty is rejected by both decoders.
///
/// An empty query travels as its own signal, so the map spelling requires
/// at least one child (the upper bound is pinned by
/// `oversized_query_listing_is_rejected`).
#[test]
fn empty_query_listing_is_rejected() {
    let stream = stream(5);
    let encoded = query(stream, Flow::Continue, &[]);
    for speaker in SPEAKERS {
        let error = decode_both(speaker, RunBudget::default(), &encoded)
            .expect_err("an empty listing cannot decode");
        assert!(matches!(error.kind, DecodeErrorKind::EmptyQuery));
    }
}

/// A query listing declaring more children than the radix space holds is
/// rejected at its map head, before any entry is read: the map spelling
/// admits at most one child per radix value.
#[test]
fn oversized_query_listing_is_rejected() {
    let stream = stream(5);
    let mut encoded = frame_head(3, stream, Signal::Query(Flow::Continue));
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
            DecodeErrorKind::InvalidListing(ListingIssue::Structure(
                ListingStructureError::TooMany {
                    declared,
                    maximum: MAX_QUERY_CHILDREN,
                }
            )) if declared == MAX_QUERY_CHILDREN as u64 + 1
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

        let supply_head = frame_head(3, stream, Signal::Supply(Flow::Continue));
        let cases = [
            (
                frame_head(3, stream, Signal::Query(Flow::Continue)),
                FramePart::QueryChildren,
            ),
            (
                {
                    let mut frame = frame_head(3, stream, Signal::Query(Flow::Continue));
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
        WireSignal::new(other, stream, invalid_signal).expect("valid for the other speaker");
        WireSignal::new(speaker, stream, valid_signal).expect("valid for this speaker");
        let mut bytes = frame_head(2, stream, invalid_signal);
        bytes.extend_from_slice(&frame_head(2, stream, valid_signal));
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

/// What a `TestRead` transport does once it has failed: keep failing,
/// close cleanly, or resume delivering the rest of its bytes.
#[derive(Debug, Clone, Copy)]
enum AfterFailure {
    /// Keep reporting the same error.
    Fail,
    /// Return EOF on subsequent reads.
    Close,
    /// Deliver the remaining bytes on subsequent reads.
    Resume,
}

/// Bytes delivered with a repeating chunk schedule and an optional read failure.
///
/// Both I/O traits use the same byte source. Async reads also yield once before
/// each operation, so the decoder must retain its progress across polls.
struct TestRead {
    bytes: Vec<u8>,
    position: usize,
    /// Bytes left before the injected error; `usize::MAX` disables it.
    remaining: usize,
    after: AfterFailure,
    failed: bool,
    reads_after_failure: usize,
    kind: std::io::ErrorKind,
    chunks: Vec<usize>,
    step: usize,
    /// Whether the next poll should yield before serving bytes.
    pending: bool,
}

/// Configure a finite input and inspect how much the decoder consumed.
impl TestRead {
    /// Deliver all bytes without injecting an error.
    fn new(bytes: &[u8]) -> Self {
        Self {
            bytes: bytes.to_vec(),
            position: 0,
            remaining: usize::MAX,
            after: AfterFailure::Fail,
            failed: false,
            reads_after_failure: 0,
            kind: std::io::ErrorKind::Other,
            chunks: vec![usize::MAX],
            step: 0,
            pending: true,
        }
    }

    /// Fail the first read after `remaining` bytes, then follow `after`.
    fn fail_after(
        mut self,
        remaining: usize,
        after: AfterFailure,
        kind: std::io::ErrorKind,
    ) -> Self {
        self.remaining = remaining;
        self.after = after;
        self.kind = kind;
        self
    }

    /// Repeat these positive read sizes until the input ends.
    fn chunked(mut self, chunks: Vec<usize>) -> Self {
        assert!(!chunks.is_empty() && chunks.iter().all(|&n| n > 0));
        self.chunks = chunks;
        self
    }

    /// Serve one read, retaining the exact position and any injected failure.
    fn serve(&mut self, want: usize) -> std::io::Result<&[u8]> {
        if want == 0 {
            return Ok(&[]);
        }
        if self.failed {
            self.reads_after_failure += 1;
            match self.after {
                AfterFailure::Fail => return Err(self.kind.into()),
                AfterFailure::Close => return Ok(&[]),
                AfterFailure::Resume => {}
            }
        } else if self.remaining == 0 {
            self.failed = true;
            return Err(self.kind.into());
        }
        let chunk = self.chunks[self.step % self.chunks.len()];
        self.step += 1;
        let available = self.bytes.len() - self.position;
        let served = if self.failed {
            available
        } else {
            self.remaining.min(available)
        }
        .min(want)
        .min(chunk);
        let start = self.position;
        self.position += served;
        if !self.failed {
            self.remaining -= served;
        }
        Ok(&self.bytes[start..self.position])
    }

    /// Bytes no read has taken, including any trailing frame.
    fn unread(&self) -> &[u8] {
        &self.bytes[self.position..]
    }
}

/// Feed the synchronous reference decoder without asynchronous scheduling.
impl std::io::Read for TestRead {
    /// Copy the next scheduled chunk into the caller's buffer.
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        let served = self.serve(out.len())?;
        out[..served.len()].copy_from_slice(served);
        Ok(served.len())
    }
}

/// Yield before each read, waking the driver so finite input can progress.
impl AsyncRead for TestRead {
    /// Alternate a self-waking yield with a scheduled read.
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if self.pending {
            self.pending = false;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        self.pending = true;
        let served = self.serve(buf.remaining())?;
        buf.put_slice(served);
        Poll::Ready(Ok(()))
    }
}

/// A transport failure inside a frame's opener is reported at the item
/// it interrupted, identically by both decoders and without another read
/// of the transport, whatever the transport would do next.
///
/// Each row delivers a prefix of an opener and then fails. The canonical
/// `End(Stream)` opener on stream 9 is three one-byte items: a failure
/// before the first byte is a read error at the frame head; after one or
/// two bytes, at the signal; after all three, unseen, and the frame
/// decodes. Two non-canonical openers fail inside a head's extension
/// bytes: a stream item `0x18` owed its one-byte extension, and an array
/// head `0x9b` owed eight. Every row runs against a transport that then
/// keeps failing, closes, or resumes delivering; in every case no read
/// follows the failure, so a resuming transport still holds the bytes
/// it would have delivered.
#[test]
fn opener_read_failures_are_reported_in_wire_order() {
    let stream = stream(9);
    let canonical = bare_frame(stream, Signal::End(End::Stream));
    assert_eq!(canonical, [0x82, 0x09, 0x09]);
    let rows: Vec<(&[u8], usize, Option<FramePart>)> = vec![
        (&canonical, 0, Some(FramePart::FrameHead)),
        (&canonical, 1, Some(FramePart::Signal)),
        (&canonical, 2, Some(FramePart::Signal)),
        (&canonical, 3, None),
        (&[0x82, 0x18, 0x09, 0x09], 2, Some(FramePart::Signal)),
        (
            &[0x9b, 0, 0, 0, 0, 0, 0, 0, 2, 0x09, 0x09],
            1,
            Some(FramePart::FrameHead),
        ),
    ];
    for speaker in SPEAKERS {
        for after in [
            AfterFailure::Fail,
            AfterFailure::Close,
            AfterFailure::Resume,
        ] {
            for &(bytes, remaining, interrupted) in &rows {
                let case =
                    format!("{speaker:?}, {bytes:02x?} cut after {remaining}, then {after:?}");
                let budget = RunBudget::default();
                let mut sync =
                    TestRead::new(bytes).fail_after(remaining, after, std::io::ErrorKind::Other);
                let from_sync = decode(speaker, budget, &mut sync);
                let mut reader = FrameRead::new(
                    speaker,
                    budget,
                    TestRead::new(bytes).fail_after(remaining, after, std::io::ErrorKind::Other),
                );
                let from_async = pollster::block_on(reader.frame());
                let r#async = reader.into_inner();
                for transport in [&sync, &r#async] {
                    assert_eq!(
                        transport.reads_after_failure, 0,
                        "{case}: the transport was read after it failed"
                    );
                    assert_eq!(
                        transport.unread(),
                        &bytes[remaining..],
                        "{case}: the transport does not rest where the failure struck"
                    );
                }
                let Some(interrupted) = interrupted else {
                    let frame = (stream, Frame::End(End::Stream));
                    assert_eq!(from_sync.expect(&case), frame, "{case}");
                    assert_eq!(from_async.expect(&case), Some(frame), "{case}");
                    continue;
                };
                let from_sync = from_sync.expect_err(&case);
                let from_async = from_async.expect_err(&case);
                assert_eq!(from_async.origin, from_sync.origin, "{case}");
                for error in [&from_sync, &from_async] {
                    assert!(
                        matches!(
                            &error.kind,
                            DecodeErrorKind::Read { part, source }
                                if *part == interrupted && source.kind() == std::io::ErrorKind::Other
                        ),
                        "{case}: {:?}",
                        error.kind
                    );
                }
            }
        }
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

/// Compare error categories, context, and I/O kinds without depending on
/// reader-specific error text.
fn kind_signature(kind: &DecodeErrorKind) -> String {
    match kind {
        DecodeErrorKind::Read { part, source } => format!("Read({part:?}, {:?})", source.kind()),
        DecodeErrorKind::Truncated { missing, source } => {
            format!("Truncated({missing:?}, {:?})", source.kind())
        }
        other => format!("{other:?}"),
    }
}

/// Compare the synchronous reference with bulk and fragmented async reads.
/// Successful reads must also leave exactly the same suffix untouched.
fn decode_both(
    speaker: Speaker,
    budget: RunBudget,
    bytes: &[u8],
) -> Result<WireFrame, DecodeError> {
    let mut rest = bytes;
    let from_sync = decode(speaker, budget, &mut rest);
    for chunks in [vec![usize::MAX], vec![1, 3, 2]] {
        let mut reader = FrameRead::new(speaker, budget, TestRead::new(bytes).chunked(chunks));
        let from_async = crate::testing::run_to_quiescence(reader.frame())
            .expect("finite input makes progress")
            .map(|frame| frame.expect("these cases expect a frame"));
        match (&from_async, &from_sync) {
            (Ok(a), Ok(s)) => {
                assert_eq!(a, s, "the two decoders accept different frames");
                assert_eq!(reader.into_inner().unread(), rest);
            }
            (Err(a), Err(s)) => {
                assert_eq!(kind_signature(&a.kind), kind_signature(&s.kind));
                assert_eq!(a.origin, s.origin);
            }
            (a, s) => panic!("the two decoders disagree: async {a:?}, sync {s:?}"),
        }
    }
    from_sync
}

proptest! {
    /// Ingress enforces the run budget, deciding from at most the first
    /// record's heads.
    ///
    /// A multi-record supply frame decodes when its charged wire size is
    /// within the budget and returns `OverbatchedRun`, carrying
    /// that wire size and the budget, when it is past it. The rejection
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

        // Past the budget: a rejection naming the frame and the budget.
        let over = RunBudget::from_bytes(wire_size.saturating_sub(deficit));
        let error = decode_both(speaker, over, &encoded).expect_err(
            "undetected over-budget batching: a multi-record frame past the \
             budget must return an error",
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
        for declared in 0..MIN_RECORD_HEADS_LEN {
            let encoded = supply_declaring(stream, Flow::End, declared, &[]);
            let error = decode_both(speaker, zero, &encoded)
                .expect_err("a headless over-budget body cannot decode");
            assert!(
                matches!(error.kind, DecodeErrorKind::OverbatchedRun { .. }),
                "declared {declared}: {:?}",
                error.kind
            );
        }

        // At the boundary, an empty-content record is structurally valid.
        // Its missing version is diagnosed later by the record iterator.
        let empty = raw_record(&[]);
        assert_eq!(empty.len(), MIN_RECORD_HEADS_LEN);
        let (_, frame) = decode_both(speaker, zero, &supply(stream, Flow::End, &empty))
            .expect("the smallest lone record passes the framing check");
        let Frame::Reaction(Reaction::Supply(run), _) = frame else {
            panic!("expected a supply frame");
        };
        assert_eq!(run.record_count(), 1);
        assert_eq!(run.as_bytes(), empty);

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

/// The over-budget lone-record body read consumes no byte beyond the
/// declared run: the frame after it stays intact in the transport and
/// decodes next, whatever the delivery chunking.
///
/// Stream 9, `Supply(End)` declaring a four-byte run that is exactly one
/// record of one content byte, then `End(Stream)` on the same stream,
/// under a zero budget so the lone-record path is taken. Both decoders
/// accept the record; the async reader then decodes the second frame and
/// reports the clean close after it.
#[test]
fn over_budget_lone_record_read_stays_within_its_frame() {
    let stream = stream(9);
    let zero = RunBudget::from_bytes(0);
    let lone = raw_record(&[0x00]);
    assert_eq!(lone, [0xd8, 0x3f, 0x41, 0x00]);
    let mut encoded = supply(stream, Flow::End, &lone);
    assert_eq!(&encoded[..6], [0x83, 0x09, 0x07, 0xd8, 0x3f, 0x44]);
    let trailing = bare_frame(stream, Signal::End(End::Stream));
    assert_eq!(trailing, [0x82, 0x09, 0x09]);
    encoded.extend_from_slice(&trailing);
    let run = LeafRun::from_encoded(lone).unwrap();

    for speaker in SPEAKERS {
        let first = (
            stream,
            Frame::Reaction(Reaction::Supply(run.clone()), Flow::End),
        );
        assert_eq!(
            decode_both(speaker, zero, &encoded).expect("both decoders accept the lone record"),
            first
        );
        for chunk in 1..=encoded.len() {
            let read = TestRead::new(&encoded).chunked(vec![chunk]);
            let mut reader = FrameRead::new(speaker, zero, read);
            let mut next = || pollster::block_on(reader.frame()).unwrap();
            assert_eq!(next(), Some(first.clone()), "chunk {chunk}");
            assert_eq!(
                next(),
                Some((stream, Frame::End(End::Stream))),
                "chunk {chunk}"
            );
            assert_eq!(next(), None, "chunk {chunk}");
        }
    }
}

/// Record iteration enforces the payload depth limit: exactly the limit
/// decodes, while one level deeper returns a payload error. Frame decoding
/// accepts the record's structure before its content is examined.
#[test]
fn record_iteration_enforces_payload_depth() {
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

    // One scope past the limit: a rejection at the record iterator.
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

mod transport;
