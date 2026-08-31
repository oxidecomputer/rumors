//! The frame is self-inverse and self-checking: it round-trips any payload,
//! rejects every single-byte corruption and every truncation, parses whole
//! under a rumors-blind CBOR reader, and pins byte-for-byte.

use std::collections::BTreeMap;

use before::Clock;
use proptest::prelude::*;

use super::*;
use crate::Network;

/// A fixed, non-trivial record: one network mapped to a seed clock and two of
/// its forks, with concurrent ticks synced so the clocks carry nested,
/// non-degenerate versions.
///
/// The ticks are load-bearing for the format pin: an all-empty record's
/// version payloads are the two-bit empty coding, which pins nothing of the
/// version-2 skyline payload bytes — the nested versions here put real
/// topology and delta codes into the pinned frame. Deterministic —
/// `Network::from_bytes` and `Clock::seed`/`fork`/`tick`/`sync` draw no
/// randomness — so anything derived from it (a snapshot, a hash) is stable
/// across runs.
fn sample_record() -> BTreeMap<Network, Vec<Clock>> {
    let network = Network::from_bytes([0x5a; 16]);
    let mut clock = Clock::seed();
    let mut first = clock.fork();
    let second = clock.fork();
    clock.tick();
    first.tick();
    first.tick();
    clock.sync(&mut first).expect("forked clocks are disjoint");
    BTreeMap::from([(network, vec![clock, first, second])])
}

/// Two records are equal when their CBOR encodings are: a [`Clock`]
/// is `!Clone` and exposes no value equality, so the bytes are the oracle.
fn record_eq(a: &BTreeMap<Network, Vec<Clock>>, b: &BTreeMap<Network, Vec<Clock>>) -> bool {
    let encode = |record: &BTreeMap<Network, Vec<Clock>>| {
        let mut buf = Vec::new();
        ciborium::ser::into_writer(record, &mut buf).unwrap();
        buf
    };
    encode(a) == encode(b)
}

proptest! {
    /// Framing is invertible: `unframe` recovers exactly the bytes `frame`
    /// wrapped, for any payload.
    #[test]
    fn framing_round_trips(payload: Vec<u8>) {
        let framed = frame(&payload);
        prop_assert_eq!(unframe(&framed).unwrap(), payload.as_slice());
    }

    /// A frame always opens with the self-described CBOR tag, the three-item
    /// frame array, and the format-version item, whatever the payload.
    #[test]
    fn frame_carries_the_tag(payload: Vec<u8>) {
        let framed = frame(&payload);
        let mut opening = SELF_DESCRIBED_HEAD.to_vec();
        opening.push(FRAME_ARRAY);
        cbor::write_head(&mut opening, MAJOR_UINT, BOOKMARK_FORMAT_VERSION);
        prop_assert!(framed.starts_with(&opening));
    }

    /// Flipping any one byte of the frame (opening, version, hash, payload
    /// headers, or payload) makes it fail to validate: nothing corrupt is
    /// ever accepted.
    #[test]
    fn any_single_byte_corruption_is_rejected(
        payload in prop::collection::vec(any::<u8>(), 1..64),
        index: prop::sample::Index,
    ) {
        let mut framed = frame(&payload);
        let i = index.index(framed.len());
        framed[i] ^= 0xff;
        prop_assert!(unframe(&framed).is_err());
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
            Err(FormatError::NotABookmark { defect: FrameDefect::TrailingBytes }),
        );
        prop_assert!(rejected);
    }

    /// A record survives a serialize/validate/deserialize round trip unchanged,
    /// for an arbitrary number of forked clocks under an arbitrary network id.
    #[test]
    fn record_round_trips(network: [u8; 16], extra_forks in 0usize..12) {
        let mut clock = Clock::seed();
        let mut clocks: Vec<Clock> = Vec::new();
        for _ in 0..extra_forks {
            clocks.push(clock.fork());
        }
        clocks.push(clock);
        let record = BTreeMap::from([(Network::from_bytes(network), clocks)]);

        let decoded = decode(&encode(&record)).expect("a freshly encoded record decodes");
        prop_assert!(record_eq(&decoded, &record));
    }
}

/// An empty record round-trips to an empty record, distinct from "absent".
#[test]
fn empty_record_round_trips() {
    let empty = BTreeMap::new();
    let decoded = decode(&encode(&empty)).expect("the empty record decodes");
    assert!(decoded.is_empty());
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
        Err(FormatError::NotABookmark {
            defect: FrameDefect::SelfDescribedTag
        })
    ));

    let ascii = b"RUMORSBOOKMARKISH TEXT, NOT CBOR";
    assert!(matches!(
        unframe(ascii),
        Err(FormatError::NotABookmark {
            defect: FrameDefect::SelfDescribedTag
        })
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

/// A non-shortest-form spelling of the format version is rejected as a shape
/// defect even though its value matches: the frame is deterministic-encoding
/// CBOR, and a wide header is a spelling this codec never writes.
#[test]
fn non_canonical_version_spelling_is_rejected() {
    // Rebuild a frame exactly as `frame_as` would, but spell version 4 as
    // the two-byte header 0x18 0x04, hashing over that spelling so only the
    // spelling check can reject it.
    let payload = b"payload";
    let mut covered = vec![0x18, u8::try_from(BOOKMARK_FORMAT_VERSION).unwrap()];
    let version_item_len = covered.len();
    covered.extend_from_slice(&EMBEDDED_CBOR);
    cbor::write_head(&mut covered, MAJOR_BSTR, payload.len() as u64);
    covered.extend_from_slice(payload);
    let hash = blake3::hash(&covered);

    let mut framed = SELF_DESCRIBED_HEAD.to_vec();
    framed.push(FRAME_ARRAY);
    framed.extend_from_slice(&covered[..version_item_len]);
    framed.extend_from_slice(&INTEGRITY_HEAD);
    framed.extend_from_slice(hash.as_bytes());
    framed.extend_from_slice(&covered[version_item_len..]);

    assert!(matches!(
        unframe(&framed),
        Err(FormatError::NotABookmark {
            defect: FrameDefect::FormatVersion
        }),
    ));
}

/// The encoded length of the format-version item in a frame this codec
/// writes: the offset arithmetic below computes it rather than
/// hardcoding it, so a version bump cannot silently skew the flips.
fn version_item_len() -> usize {
    let mut version_item = Vec::new();
    cbor::write_head(&mut version_item, MAJOR_UINT, BOOKMARK_FORMAT_VERSION);
    version_item.len()
}

/// Corrupting the integrity item's header is rejected as the typed
/// [`FrameDefect::Integrity`] shape defect, distinct from a hash
/// mismatch: the header bytes are part of the frame's fixed spelling.
#[test]
fn corrupt_integrity_head_is_an_integrity_defect() {
    let mut framed = frame(b"payload");
    let integrity_at = SELF_DESCRIBED_HEAD.len() + 1 + version_item_len();
    assert_eq!(
        framed[integrity_at], INTEGRITY_HEAD[0],
        "the computed offset lands on the integrity head"
    );
    framed[integrity_at] ^= 0xff;
    assert!(matches!(
        unframe(&framed),
        Err(FormatError::NotABookmark {
            defect: FrameDefect::Integrity
        }),
    ));
}

/// Corrupting the payload item's tag byte is rejected as the typed
/// [`FrameDefect::PayloadTag`] shape defect: the embedded-CBOR tag is
/// part of the frame's fixed spelling, checked before the hash.
#[test]
fn corrupt_payload_tag_is_a_payload_tag_defect() {
    let mut framed = frame(b"payload");
    let payload_tag_at =
        SELF_DESCRIBED_HEAD.len() + 1 + version_item_len() + INTEGRITY_HEAD.len() + HASH_LEN;
    assert_eq!(
        framed[payload_tag_at], EMBEDDED_CBOR[0],
        "the computed offset lands on the payload tag"
    );
    framed[payload_tag_at] ^= 0xff;
    assert!(matches!(
        unframe(&framed),
        Err(FormatError::NotABookmark {
            defect: FrameDefect::PayloadTag
        }),
    ));
}

/// A non-shortest-form spelling of the payload byte-string length is
/// rejected as the typed [`FrameDefect::PayloadByteString`] defect.
///
/// The value matches; only the spelling is wrong: the frame is
/// deterministic-encoding CBOR, and a wide header is a spelling this
/// codec never writes.
#[test]
fn non_canonical_payload_spelling_is_rejected() {
    // Rebuild a frame exactly as `frame_as` would, but spell the 7-byte
    // payload's byte-string head as the widened two-byte form 0x58 0x07,
    // hashing over that spelling so only the spelling check can reject
    // it.
    let payload = b"payload";
    let mut covered = Vec::new();
    cbor::write_head(&mut covered, MAJOR_UINT, BOOKMARK_FORMAT_VERSION);
    let version_item_len = covered.len();
    covered.extend_from_slice(&EMBEDDED_CBOR);
    covered.extend_from_slice(&[0x58, u8::try_from(payload.len()).unwrap()]);
    covered.extend_from_slice(payload);
    let hash = blake3::hash(&covered);

    let mut framed = SELF_DESCRIBED_HEAD.to_vec();
    framed.push(FRAME_ARRAY);
    framed.extend_from_slice(&covered[..version_item_len]);
    framed.extend_from_slice(&INTEGRITY_HEAD);
    framed.extend_from_slice(hash.as_bytes());
    framed.extend_from_slice(&covered[version_item_len..]);

    assert!(matches!(
        unframe(&framed),
        Err(FormatError::NotABookmark {
            defect: FrameDefect::PayloadByteString
        }),
    ));
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

/// An intact frame whose payload item holds an untagged clock is a
/// [`RecordDefect`], not corruption: the hash passed, so the defect class is
/// [`FormatError::Record`].
#[test]
fn untagged_clock_is_a_record_defect() {
    let clock = Clock::seed();
    // The map `encode` writes, minus the clock's tag.
    let map = ciborium::value::Value::Map(vec![(
        ciborium::value::Value::Bytes(vec![0x5a; 16]),
        ciborium::value::Value::Array(vec![ciborium::value::Value::Bytes(clock.encode())]),
    )]);
    let mut payload = Vec::new();
    ciborium::ser::into_writer(&map, &mut payload).unwrap();
    assert!(matches!(
        decode(&frame(&payload)),
        Err(FormatError::Record(RecordDefect::ClockUntagged)),
    ));
}

/// The whole file parses as exactly one CBOR item under a reader that knows
/// nothing of rumors.
///
/// Unwrapping the standard tags (55799, then 24) and the clock tag exposes
/// the record's full structure, with no bytes outside CBOR items at either
/// level. This is the tamper-evident form of the "fully CBOR-parseable on
/// disk" promise.
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

    let Value::Map(entries) = record else {
        panic!("the record is not a map");
    };
    for (key, clocks) in entries {
        assert!(matches!(key, Value::Bytes(bytes) if bytes.len() == 16));
        let Value::Array(clocks) = clocks else {
            panic!("a record entry is not an array");
        };
        for clock in clocks {
            assert!(
                matches!(clock, Value::Tag(tag, inner)
                    if tag == crate::tags::CLOCK_TAG && matches!(*inner, Value::Bytes(_))),
                "every stored clock is a clock-tagged byte string",
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
/// The integrity digest is sliced from the frame's own bytes, never
/// recomputed, so the annotation reads what the pin holds; `unframe` has
/// already verified it against the payload.
fn annotated(frame: &[u8]) -> String {
    use crate::tree::mirror::cbor::{TAG_EMBEDDED_ITEM, TAG_SELF_DESCRIBED};
    use std::fmt::Write;
    let record = decode(frame).expect("the pinned frame decodes");
    let payload = unframe(frame).expect("the pinned frame unframes");
    // Fixed offsets: the version head is a single byte for any version
    // below 24, and everything before the digest is a pinned spelling.
    const { assert!(BOOKMARK_FORMAT_VERSION < 24, "the version head is one byte") };
    let hash_at = SELF_DESCRIBED_HEAD.len() + 1 + 1 + INTEGRITY_HEAD.len();
    let integrity = &frame[hash_at..hash_at + HASH_LEN];
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
        "    h'{}' / integrity: BLAKE3 of the embedded record /",
        hex::encode(integrity)
    )
    .unwrap();
    writeln!(
        out,
        "    {TAG_EMBEDDED_ITEM}(<< / embedded record, {} byte(s) /",
        payload.len()
    )
    .unwrap();
    writeln!(out, "      {{ / {} network(s) /", record.len()).unwrap();
    for (network, clocks) in &record {
        writeln!(
            out,
            "        h'{}' / network / =>",
            hex::encode(network.to_bytes())
        )
        .unwrap();
        writeln!(out, "          [ / {} clock(s) /", clocks.len()).unwrap();
        for clock in clocks {
            writeln!(
                out,
                "            {}(h'{}') / clock /",
                crate::tags::CLOCK_TAG,
                hex::encode(clock.encode())
            )
            .unwrap();
        }
        writeln!(out, "          ]").unwrap();
    }
    writeln!(out, "      }}").unwrap();
    writeln!(out, "    >>)").unwrap();
    writeln!(out, "  ]").unwrap();
    write!(out, ")").unwrap();
    out
}

/// The encoded empty record pins byte-for-byte.
///
/// The snapshot's first line is the frame's exact hex — the
/// self-described frame over the embedded CBOR encoding of an empty
/// map — followed by [`annotated`]'s decoded reading of the same bytes.
///
/// A change to the hex line is a deliberate on-disk format change, like
/// the wire-format snapshots; an annotation-only change must preserve it
/// byte-identically.
#[test]
fn pins_the_empty_frame() {
    insta::assert_snapshot!("frame_empty", annotated(&encode(&BTreeMap::new())));
}

/// The encoded non-trivial record pins byte-for-byte, so format drift cannot
/// hide in a populated payload (multiple clocks under a network id) the way it
/// could in an empty one.
///
/// The snapshot's first line is the frame's exact hex (the pin); the rest
/// is [`annotated`]'s decoded reading of the same bytes.
///
/// The pinned bytes are fixture-derived: a re-accept whose only cause is a
/// deliberate [`sample_record`] change — the format attested unchanged by the
/// untouched `frame_empty` pin and the round-trip/corruption suite in the
/// same commit — is a sanctioned *fixture re-pin*, not a format change. A
/// snapshot tamper sweep attributes this pin's history to the fixture, never
/// to a protocol or format revision (the sanctioned exception is also on the
/// snapshot roster in `AGENTS.md`).
#[test]
fn pins_a_non_trivial_frame() {
    insta::assert_snapshot!("frame_non_trivial", annotated(&encode(&sample_record())));
}

/// Pins the stated ingress boundary: the embedded payload's spelling is
/// not ingress-judged — the hash binds bytes, the frame binds shape.
///
/// A payload spelled as an indefinite-length map (a spelling this codec
/// never writes) decodes to the empty record. Flipping this to rejection
/// is a deliberate contract change, not drift.
#[test]
fn indefinite_length_payload_map_is_not_spelling_judged() {
    let record =
        decode(&frame(&[0xbf, 0xff])).expect("the payload's spelling is not ingress-judged");
    assert!(record.is_empty());
}
