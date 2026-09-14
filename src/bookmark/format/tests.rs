//! Integrity protects the version and payload, independent of envelope spelling.
//! Round trips, malformed fields, truncation, and byte pins check both CBOR layers.

use crate::tags::{PARTY_TAG, VERSION_TAG};
use crate::tree::mirror::cbor::{MAJOR_ARRAY, MAJOR_BSTR, MAJOR_TAG, MAJOR_UINT};
use before::{Clock, Party, Version};
use ciborium::value::Value;
use proptest::prelude::*;

use super::*;
use crate::Network;

/// A deterministic network with several identities and a nested write frontier.
/// Concurrent writes exercise real version topology in the format snapshot.
fn sample_record() -> Record {
    let network = Network::from_bytes([0x5a; 16]);
    let mut clock = Clock::seed();
    let mut first = clock.fork();
    let second = clock.fork();
    clock.tick();
    first.tick();
    first.tick();
    let mut entry = NetworkRecord::default();
    for clock in [clock, first, second] {
        let (party, version) = clock.into_parts();
        entry.record(&party, &version);
    }
    let mut record = Record::default();
    assert!(record.networks.insert(network, entry));
    record
}

/// Compare decoded values and both recency orders independently of the encoder.
fn record_eq(a: &Record, b: &Record) -> bool {
    a.networks.len() == b.networks.len()
        && a.networks
            .iter()
            .zip(b.networks.iter())
            .all(|((ka, a), (kb, b))| {
                ka == kb && a.written == b.written && a.identities == b.identities
            })
}

/// Render a CBOR header with either its shortest or its eight-byte argument.
fn alternate_head(out: &mut Vec<u8>, major: u8, value: u64, wide: bool) {
    if wide {
        out.push((major << 5) | 27);
        out.extend_from_slice(&value.to_be_bytes());
    } else {
        cbor::write_head(out, major, value);
    }
}

/// Render test values using independent choices of legal CBOR spellings.
fn alternate_cbor(value: &Value, out: &mut Vec<u8>, indefinite: bool, wide: bool, chunk: usize) {
    match value {
        Value::Array(items) => {
            if indefinite {
                out.push(0x9f);
            } else {
                alternate_head(out, MAJOR_ARRAY, items.len() as u64, wide);
            }
            for item in items {
                alternate_cbor(item, out, indefinite, wide, chunk);
            }
            if indefinite {
                out.push(0xff);
            }
        }
        Value::Bytes(bytes) => {
            if indefinite {
                out.push(0x5f);
                for bytes in bytes.chunks(chunk) {
                    alternate_head(out, MAJOR_BSTR, bytes.len() as u64, wide);
                    out.extend_from_slice(bytes);
                }
                out.push(0xff);
            } else {
                alternate_head(out, MAJOR_BSTR, bytes.len() as u64, wide);
                out.extend_from_slice(bytes);
            }
        }
        Value::Tag(tag, inner) => {
            alternate_head(out, MAJOR_TAG, *tag, wide);
            alternate_cbor(inner, out, indefinite, wide, chunk);
        }
        Value::Integer(value) => {
            alternate_head(out, MAJOR_UINT, (*value).try_into().unwrap(), wide);
        }
        _ => {
            unreachable!("bookmark test values contain arrays, tags, bytes, and positive integers")
        }
    }
}

/// Access the envelope's fields when constructing a malformed test value.
fn envelope_fields(value: &mut Value) -> &mut Vec<Value> {
    let Value::Tag(_, inner) = value else {
        unreachable!()
    };
    let Value::Array(fields) = inner.as_mut() else {
        unreachable!()
    };
    fields
}

proptest! {
    /// Each network consumes exactly three fields and its own closing boundary.
    #[test]
    fn network_fields_have_exact_arity(indefinite: bool, fields in 0usize..7) {
        let encoded = encode(&sample_record());
        let mut value: Value = from_slice(&unframe(&encoded).unwrap()).unwrap();
        let Value::Array(networks) = &mut value else { unreachable!() };
        let Value::Array(entry) = &mut networks[0] else { unreachable!() };
        entry.resize(fields, Value::Null);
        let mut payload = Vec::new();
        // Extra values are deliberately CBOR arrays so they could be mistaken
        // for another network if the tuple's boundary were not checked.
        for field in entry.iter_mut().skip(3) { *field = Value::Array(Vec::new()); }
        alternate_cbor(&value, &mut payload, indefinite, false, 3);
        prop_assert_eq!(decode(&frame(&payload)).is_ok(), fields == 3);
    }

    /// Legal CBOR spellings preserve identities, frontiers, and recency; every
    /// truncated spelling is rejected even when its frame has a valid hash.
    #[test]
    fn alternate_cbors_decode_without_weakening_validation(
        indefinite in any::<bool>(), wide in any::<bool>(), chunk in 1usize..20,
        cut in any::<usize>(),
    ) {
        let expected = sample_record();
        let file = encode(&expected);
        let value: Value = from_slice(&unframe(&file).unwrap()).unwrap();
        let mut payload = Vec::new();
        alternate_cbor(&value, &mut payload, indefinite, wide, chunk);
        // The generic reader independently confirms the alternate spelling.
        let parsed: Value = ciborium::de::from_reader(payload.as_slice()).unwrap();
        prop_assert_eq!(parsed, value);
        prop_assert!(record_eq(&decode(&frame(&payload)).unwrap(), &expected));
        let cut = cut % payload.len();
        prop_assert!(decode(&frame(&payload[..cut])).is_err());
    }

    /// Trailing bytes are rejected at the end of definite and indefinite payloads.
    #[test]
    fn trailing_payload_bytes_are_rejected(indefinite in any::<bool>(), suffix in proptest::collection::vec(any::<u8>(), 1..30)) {
        let value: Value = from_slice(&unframe(&encode(&sample_record())).unwrap()).unwrap();
        let mut payload = Vec::new();
        alternate_cbor(&value, &mut payload, indefinite, true, 2);
        payload.extend_from_slice(&suffix);
        prop_assert!(matches!(decode(&frame(&payload)), Err(FormatError::Record(_))), "trailing payload bytes must be rejected");
    }

    /// Every container, tag, and byte-string boundary rejects the wrong CBOR type.
    #[test]
    fn record_structure_is_checked_at_every_boundary(boundary in 0usize..8) {
        let paths: &[&[usize]] = &[
            &[], &[0], &[0, 0], &[0, 1], &[0, 1, 0],
            &[0, 2], &[0, 2, 0], &[0, 2, 0, 0],
        ];
        let file = encode(&sample_record());
        let mut payload: Value = from_slice(&unframe(&file).unwrap()).unwrap();
        let mut node = &mut payload;
        for &index in paths[boundary] {
            node = match node {
                Value::Array(items) => &mut items[index],
                Value::Tag(_, inner) => inner,
                _ => unreachable!("each path names a record field"),
            };
        }
        *node = Value::Bool(false);
        let mut bytes = Vec::new();
        ciborium::ser::into_writer(&payload, &mut bytes).unwrap();
        prop_assert!(matches!(decode(&frame(&bytes)), Err(FormatError::Record(_))),
            "a boolean cannot replace a record field");
    }

    /// Framing is invertible: `unframe` recovers exactly the bytes `frame`
    /// wrapped, for any payload.
    #[test]
    fn framing_round_trips(payload: Vec<u8>) {
        let framed = frame(&payload);
        prop_assert_eq!(unframe(&framed).unwrap(), payload.as_slice());
    }

    /// Equivalent outer CBOR spellings preserve the same checksum and payload.
    #[test]
    fn envelope_spellings_preserve_integrity(
        payload: Vec<u8>, indefinite: bool, wide: bool, chunk in 1usize..32,
        cut in any::<usize>(),
    ) {
        let value: Value = from_slice(&frame(&payload)).unwrap();
        let mut encoded = Vec::new();
        alternate_cbor(&value, &mut encoded, indefinite, wide, chunk);
        prop_assert_eq!(unframe(&encoded).unwrap(), payload);
        prop_assert!(unframe(&encoded[..cut % encoded.len()]).is_err());
    }

    /// Changing any payload or digest byte is rejected even with alternate envelope headers.
    #[test]
    fn changed_contents_fail_integrity(
        payload in prop::collection::vec(any::<u8>(), 1..128),
        digest: bool, index in any::<usize>(), mask in 1u8..=255,
        indefinite: bool, wide: bool,
    ) {
        let mut value: Value = from_slice(&frame(&payload)).unwrap();
        let fields = envelope_fields(&mut value);
        let bytes = if digest { &mut fields[1] } else {
            let Value::Tag(_, inner) = &mut fields[2] else { unreachable!() };
            inner
        };
        let Value::Bytes(bytes) = bytes else { unreachable!() };
        let index = index % bytes.len();
        bytes[index] ^= mask;
        let mut encoded = Vec::new();
        alternate_cbor(&value, &mut encoded, indefinite, wide, 3);
        prop_assert!(matches!(unframe(&encoded), Err(FormatError::HashMismatch)));
    }

    /// The envelope consumes exactly its three fields, including an indefinite array's end.
    #[test]
    fn envelope_fields_have_exact_arity(indefinite: bool, fields in 0usize..7) {
        let mut value: Value = from_slice(&frame(b"payload")).unwrap();
        envelope_fields(&mut value).resize(fields, Value::Array(Vec::new()));
        let mut encoded = Vec::new();
        alternate_cbor(&value, &mut encoded, indefinite, false, 3);
        prop_assert_eq!(unframe(&encoded).is_ok(), fields == 3);
    }

    /// The self-described and embedded-item tags are required, regardless of their encoding.
    #[test]
    fn envelope_tags_are_required(outer: bool, missing: bool, wrong in any::<u64>()) {
        let mut value: Value = from_slice(&frame(b"payload")).unwrap();
        let tagged = if outer { &mut value } else { &mut envelope_fields(&mut value)[2] };
        let Value::Tag(tag, inner) = tagged else { unreachable!() };
        if missing { *tagged = *inner.clone(); }
        else { *tag = if wrong == *tag { wrong ^ 1 } else { wrong }; }
        prop_assert!(matches!(unframe(&to_vec(&value)), Err(FormatError::NotABookmark { .. })),
            "both envelope tags are required");
    }

    /// Cutting a frame anywhere before its end fails to validate — a partial
    /// write is caught as [`FormatError::Truncated`], never misread.
    #[test]
    fn truncation_at_every_prefix_is_rejected(
        payload in prop::collection::vec(any::<u8>(), 0..64),
        index: prop::sample::Index,
    ) {
        let framed = frame(&payload);
        let cut = index.index(framed.len());
        let truncated = matches!(
            unframe(&framed[..cut]),
            Err(FormatError::Truncated { len }) if len == cut,
        );
        prop_assert!(truncated);
    }

    /// Bytes appended after the frame array fail to validate: the frame is
    /// exactly one CBOR item, so a follower is a shape defect.
    #[test]
    fn trailing_bytes_are_rejected(payload: Vec<u8>, extra: u8) {
        let mut framed = frame(&payload);
        framed.push(extra);
        let rejected = matches!(
            unframe(&framed),
            Err(FormatError::NotABookmark { .. }),
        );
        prop_assert!(rejected);
    }

    /// A record survives a serialize/validate/deserialize round trip unchanged,
    /// for an arbitrary number of identities under an arbitrary network id.
    #[test]
    fn record_round_trips(network: [u8; 16], extra_forks in 0usize..12) {
        let (mut party, mut version) = Clock::seed().into_parts();
        let mut entry = NetworkRecord::default();
        for _ in 0..extra_forks {
            let fork = party.fork();
            version.tick(&fork);
            entry.record(&fork, &version);
        }
        entry.record(&party, &version);
        let mut record = Record::default();
        assert!(record.networks.insert(Network::from_bytes(network), entry));

        let decoded = decode(&encode(&record)).expect("a freshly encoded record decodes");
        prop_assert!(record_eq(&decoded, &record));
    }
}

/// An empty record round-trips to an empty record, distinct from "absent".
#[test]
fn empty_record_round_trips() {
    let empty = Record::default();
    let decoded = decode(&encode(&empty)).expect("the empty record decodes");
    assert!(decoded.networks.is_empty());
}

/// Foreign leading bytes are rejected as [`FormatError::NotABookmark`], not
/// misread — including a file that opens with plain ASCII where the
/// self-described tag belongs.
#[test]
fn foreign_magic_is_rejected() {
    let mut framed = encode(&sample_record());
    framed[0] ^= 0xff;
    assert!(matches!(
        unframe(&framed),
        Err(FormatError::NotABookmark { .. })
    ));

    let ascii = b"RUMORSBOOKMARKISH TEXT, NOT CBOR";
    assert!(matches!(
        unframe(ascii),
        Err(FormatError::NotABookmark { .. })
    ));
}

/// A frame declaring an unknown format version is rejected on the version
/// alone — its hash is valid, so the rejection is
/// [`FormatError::VersionMismatch`], never decoded under this build's
/// assumptions.
#[test]
fn unknown_version_is_rejected() {
    let framed = frame_as(0xbeef, b"payload");
    assert!(matches!(
        unframe(&framed),
        Err(FormatError::VersionMismatch { found: 0xbeef }),
    ));
}

/// Every earlier format version is strictly rejected: the earlier frame
/// shapes share no decoder with this one, and there is deliberately no
/// migration path.
#[test]
fn prior_versions_are_rejected() {
    for prior in 0..BOOKMARK_FORMAT_VERSION {
        let framed = frame_as(prior, b"payload");
        assert!(matches!(
            unframe(&framed),
            Err(FormatError::VersionMismatch { found }) if found == prior,
        ));
    }
}

/// A frame whose payload no longer matches its stored hash is rejected as
/// corrupt.
#[test]
fn payload_corruption_is_rejected() {
    let mut framed = encode(&sample_record());
    let last = framed.len() - 1;
    framed[last] ^= 0xff;
    assert!(matches!(unframe(&framed), Err(FormatError::HashMismatch)));
}

/// The empty input is [`FormatError::Truncated`], never mistaken for an
/// absent bookmark.
#[test]
fn short_input_is_truncated() {
    assert!(matches!(
        unframe(&[]),
        Err(FormatError::Truncated { len: 0 })
    ));
}

/// A missing atom tag is a record defect even when the integrity hash is valid.
#[test]
fn untagged_identity_is_a_record_defect() {
    let record = sample_record();
    let file = encode(&record);
    let mut payload: Value = from_slice(&unframe(&file).unwrap()).unwrap();
    let Value::Array(networks) = &mut payload else {
        unreachable!()
    };
    let Value::Array(fields) = &mut networks[0] else {
        unreachable!()
    };
    let Value::Array(identities) = &mut fields[2] else {
        unreachable!()
    };
    let Value::Tag(_, inner) = &identities[0] else {
        unreachable!()
    };
    identities[0] = *inner.clone();
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(&payload, &mut bytes).unwrap();
    assert!(matches!(
        decode(&frame(&bytes)),
        Err(FormatError::Record(_))
    ));
}

/// Duplicate networks cannot introduce independently stale recovery requirements.
#[test]
fn duplicate_networks_are_rejected() {
    let record = sample_record();
    let file = encode(&record);
    let mut payload: Value = from_slice(&unframe(&file).unwrap()).unwrap();
    let Value::Array(networks) = &mut payload else {
        unreachable!()
    };
    networks.push(networks[0].clone());
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(&payload, &mut bytes).unwrap();
    assert!(matches!(
        decode(&frame(&bytes)),
        Err(FormatError::Record(_))
    ));
}

/// Large identity encodings survive the Serde round trip.
#[test]
fn large_identity_round_trips() {
    let mut identity = Party::seed();
    for _ in 0..8192 {
        drop(identity.fork());
    }
    assert!(identity.encode().len() > 1024);
    let mut network = NetworkRecord::default();
    network.record(&identity, &Version::new());
    let mut expected = Record::default();
    assert!(
        expected
            .networks
            .insert(Network::from_bytes([0x42; 16]), network)
    );
    let file = encode(&expected);
    assert!(record_eq(&decode(&file).unwrap(), &expected));
}

/// Malformed identity and frontier encodings are rejected inside an intact frame.
#[test]
fn invalid_atoms_are_rejected() {
    let file = encode(&sample_record());
    let payload: Value = from_slice(&unframe(&file).unwrap()).unwrap();
    for frontier in [false, true] {
        let mut malformed = payload.clone();
        let Value::Array(networks) = &mut malformed else {
            unreachable!()
        };
        let Value::Array(fields) = &mut networks[0] else {
            unreachable!()
        };
        let atom = if frontier {
            &mut fields[1]
        } else {
            let Value::Array(identities) = &mut fields[2] else {
                unreachable!()
            };
            &mut identities[0]
        };
        let Value::Tag(_, bytes) = atom else {
            unreachable!()
        };
        **bytes = Value::Bytes(Vec::new());
        let mut encoded = Vec::new();
        ciborium::ser::into_writer(&malformed, &mut encoded).unwrap();
        let Err(error) = decode(&frame(&encoded)) else {
            panic!("an empty atom is invalid");
        };
        assert!(matches!(error, FormatError::Record(_)));
    }
}

/// The whole file parses as exactly one CBOR item under a reader that knows
/// nothing of rumors.
///
/// Unwrapping the standard tags (55799, then 24) and the identity and version
/// tags exposes the record's full structure, with no bytes outside CBOR items
/// at either level. This is the tamper-evident form of the "fully
/// CBOR-parseable on disk" promise.
#[test]
fn file_is_rumors_blind_cbor() {
    use ciborium::value::Value;

    let file = encode(&sample_record());
    let mut input = file.as_slice();
    let item: Value = ciborium::de::from_reader(&mut input).expect("the file parses as CBOR");
    assert!(input.is_empty(), "no bytes outside the one CBOR item");

    let Value::Tag(55799, frame) = item else {
        panic!("the file is not self-described CBOR");
    };
    let Value::Array(items) = *frame else {
        panic!("the frame is not an array");
    };
    let [version, integrity, payload]: [Value; 3] =
        items.try_into().expect("the frame array has three items");
    assert_eq!(version, Value::from(BOOKMARK_FORMAT_VERSION));
    let Value::Bytes(integrity) = integrity else {
        panic!("the integrity item is not a byte string");
    };
    assert_eq!(integrity.len(), HASH_LEN);

    let Value::Tag(24, embedded) = payload else {
        panic!("the payload is not an embedded CBOR item");
    };
    let Value::Bytes(embedded) = *embedded else {
        panic!("the embedded item is not a byte string");
    };
    let mut inner = embedded.as_slice();
    let record: Value = ciborium::de::from_reader(&mut inner).expect("the payload parses as CBOR");
    assert!(inner.is_empty(), "no bytes outside the record item");

    let Value::Array(entries) = record else {
        panic!("the record is not an array");
    };
    for entry in entries {
        let Value::Array(fields) = entry else {
            panic!("the network is not an array")
        };
        let [key, written, identities]: [Value; 3] = fields.try_into().unwrap();
        assert!(matches!(key, Value::Bytes(bytes) if bytes.len() == 16));
        assert!(
            matches!(written, Value::Tag(VERSION_TAG, inner) if matches!(*inner, Value::Bytes(_)))
        );
        let Value::Array(identities) = identities else {
            panic!("identities are not an array")
        };
        for identity in identities {
            assert!(
                matches!(identity, Value::Tag(PARTY_TAG, inner) if matches!(*inner, Value::Bytes(_)))
            );
        }
    }
}

/// Render a bookmark frame for the byte pins: exact hex, then annotation.
///
/// The frame's exact hex rides the first line — the pin itself, which an
/// annotation change leaves byte-identical — followed by a decoded,
/// annotated reading of the same bytes in the wire captures' idiom.
///
/// The digest is decoded from the frame rather than recomputed, so the
/// annotation shows what storage holds. `unframe` verifies it first.
fn annotated(frame: &[u8]) -> String {
    use crate::tree::mirror::cbor::{TAG_EMBEDDED_ITEM, TAG_SELF_DESCRIBED};
    use std::fmt::Write;
    let record = decode(frame).expect("the pinned frame decodes");
    let payload = unframe(frame).expect("the pinned frame unframes");
    let Required(Complete((_, integrity, _))) = from_slice::<Envelope>(frame).unwrap();
    let mut out = format!("{}\n\n", hex::encode(frame));
    writeln!(out, "{TAG_SELF_DESCRIBED}( / self-described CBOR /").unwrap();
    writeln!(out, "  [").unwrap();
    writeln!(
        out,
        "    {BOOKMARK_FORMAT_VERSION} / bookmark format version /"
    )
    .unwrap();
    writeln!(
        out,
        "    h'{}' / integrity: SHA3-256 of the version and payload items /",
        hex::encode(integrity)
    )
    .unwrap();
    writeln!(
        out,
        "    {TAG_EMBEDDED_ITEM}(<< / embedded record, {} byte(s) /",
        payload.len()
    )
    .unwrap();
    writeln!(
        out,
        "      [ / {} network(s), newest first /",
        record.networks.len()
    )
    .unwrap();
    for (network, entry) in record.networks.iter() {
        writeln!(out, "        [").unwrap();
        writeln!(
            out,
            "          h'{}' / network /",
            hex::encode(network.to_bytes())
        )
        .unwrap();
        writeln!(
            out,
            "          {VERSION_TAG}(h'{}') / shared write frontier /",
            hex::encode(entry.written.encode())
        )
        .unwrap();
        writeln!(
            out,
            "          [ / {} identity(s), oldest first /",
            entry.identities.len()
        )
        .unwrap();
        for identity in &entry.identities {
            writeln!(
                out,
                "            {PARTY_TAG}(h'{}') / identity /",
                hex::encode(identity.encode())
            )
            .unwrap();
        }
        writeln!(out, "          ]").unwrap();
        writeln!(out, "        ]").unwrap();
    }
    writeln!(out, "      ]").unwrap();
    writeln!(out, "    >>)").unwrap();
    writeln!(out, "  ]").unwrap();
    write!(out, ")").unwrap();
    out
}

/// The empty frame pins the format, including the empty network array.
#[test]
fn pins_the_empty_frame() {
    insta::assert_snapshot!("frame_empty", annotated(&encode(&Record::default())));
}

/// A populated frame also pins nested version and identity encodings.
#[test]
fn pins_a_non_trivial_frame() {
    insta::assert_snapshot!("frame_non_trivial", annotated(&encode(&sample_record())));
}

/// The payload accepts indefinite arrays; its integrity hash still binds every
/// byte.
#[test]
fn indefinite_length_payload_array_is_accepted() {
    let record = decode(&frame(&[0x9f, 0xff])).expect("a valid empty array");
    assert!(record.networks.is_empty());
}
