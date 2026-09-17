//! Serde representation, strictness, round-trip, and composition tests.
//!
//! Round-trips through human-readable and binary formats, canonical payload
//! pins, strict rejection, and composition inside a larger serde value.
//! The party/version/clock legs live beside the world fixture in
//! `clock/tests.rs`.

use proptest::prelude::*;
use serde_test::{assert_tokens, Configure, Token};

use crate::span::Span;
use crate::testing::bridge::from_oracle_version;
use crate::testing::generators::arb_oracle_version;
use crate::{Clock, Rank, Ranked, Version};

/// Two strictly ordered versions (`older < newer`) from one history.
fn ordered_pair() -> (Version, Version) {
    let mut clock = Clock::seed();
    let older = clock.tick().clone();
    let newer = clock.tick().clone();
    (older, newer)
}

/// Makes serde_test's static byte token from an owned encoding.
fn byte_token(bytes: Vec<u8>) -> Token {
    Token::Bytes(bytes.leak())
}

/// Compact serde uses typed canonical bytes for every value, while readable
/// serde uses canonical text for ranks.
#[test]
fn serde_data_model_matches_each_format_class() {
    let party = crate::Party::seed();
    let bytes = party.encode();
    assert_tokens(&party.compact(), &[byte_token(bytes)]);

    let version = Version::new();
    let bytes = version.encode();
    assert_tokens(&version.compact(), &[byte_token(bytes)]);

    let clock = Clock::seed();
    let bytes = clock.encode();
    assert_tokens(&clock.compact(), &[byte_token(bytes)]);

    let rank: Rank = "101.01".parse().unwrap();
    let bytes = rank.encode();
    assert_tokens(&rank.clone().compact(), &[byte_token(bytes)]);
    assert_tokens(&rank.readable(), &[Token::Str("101.01")]);

    let ranked = Ranked::from(Version::new());
    let bytes = ranked.encode();
    assert_tokens(&ranked.compact(), &[byte_token(bytes)]);

    let (older, newer) = ordered_pair();
    let span = Span::new(&older, &newer).unwrap().into_owned();
    let bytes = span.encode();
    assert_tokens(&span.compact(), &[byte_token(bytes)]);
}

proptest! {
    /// [`Rank`], [`Ranked`], and [`Span`] round-trip through serde.
    ///
    /// JSON uses `Rank`'s canonical text and bytes for the composite types.
    /// Postcard and CBOR use canonical bytes for every type.
    #[test]
    fn serde_roundtrip_rank_and_span(
        oa in arb_oracle_version(),
        ob in arb_oracle_version(),
    ) {
        let a = from_oracle_version(&oa);
        let b = from_oracle_version(&ob);
        let rank = a.rank();
        let ranked = Ranked::from(a.clone());
        let span = a.span(&b);

        let r2: Rank = serde_json::from_slice(&serde_json::to_vec(&rank).unwrap()).unwrap();
        let k2: Ranked = serde_json::from_slice(&serde_json::to_vec(&ranked).unwrap()).unwrap();
        let s2: Span = serde_json::from_slice(&serde_json::to_vec(&span).unwrap()).unwrap();
        prop_assert_eq!(&r2, &rank);
        prop_assert_eq!(&k2, &ranked);
        prop_assert_eq!(&s2, &span);
        prop_assert_eq!(
            serde_json::to_value(&rank).unwrap(),
            serde_json::Value::String(rank.to_string()),
        );

        let r2: Rank = postcard::from_bytes(&postcard::to_allocvec(&rank).unwrap()).unwrap();
        let k2: Ranked = postcard::from_bytes(&postcard::to_allocvec(&ranked).unwrap()).unwrap();
        let s2: Span = postcard::from_bytes(&postcard::to_allocvec(&span).unwrap()).unwrap();
        prop_assert_eq!(&r2, &rank);
        prop_assert_eq!(&k2, &ranked);
        prop_assert_eq!(&s2, &span);

        // CBOR reports itself as binary, so every payload is a byte string.
        let cbor = |bytes: &[u8]| -> u8 { bytes[0] >> 5 };
        let mut buf = Vec::new();
        ciborium::ser::into_writer(&rank, &mut buf).unwrap();
        prop_assert_eq!(cbor(&buf), 2, "Rank did not serialize as a CBOR byte string");
        let r3: Rank = ciborium::de::from_reader(&buf[..]).unwrap();
        prop_assert_eq!(&r3, &rank);
        let mut buf = Vec::new();
        ciborium::ser::into_writer(&ranked, &mut buf).unwrap();
        prop_assert_eq!(cbor(&buf), 2, "Ranked did not serialize as a CBOR byte string");
        let k3: Ranked = ciborium::de::from_reader(&buf[..]).unwrap();
        prop_assert_eq!(&k3, &ranked);
        let mut buf = Vec::new();
        ciborium::ser::into_writer(&span, &mut buf).unwrap();
        prop_assert_eq!(cbor(&buf), 2, "Span did not serialize as a CBOR byte string");
        let s3: Span = ciborium::de::from_reader(&buf[..]).unwrap();
        prop_assert_eq!(&s3, &span);
    }

    /// The serde byte payload is exactly the canonical encoding.
    ///
    /// Each type serializes to the same stream as its own `encode()` bytes
    /// handed to the format as a plain byte sequence — the wire form is
    /// `encode()` with nothing added, reordered, or wrapped.
    #[test]
    fn serde_bytes_pin_the_canonical_encoding_rank_and_span(
        oa in arb_oracle_version(),
        ob in arb_oracle_version(),
    ) {
        let a = from_oracle_version(&oa);
        let b = from_oracle_version(&ob);
        let rank = a.rank();
        let ranked = Ranked::from(a.clone());
        let span = a.span(&b);

        prop_assert_eq!(
            postcard::to_allocvec(&rank).unwrap(),
            postcard::to_allocvec(&rank.encode()).unwrap(),
        );
        prop_assert_eq!(
            postcard::to_allocvec(&ranked).unwrap(),
            postcard::to_allocvec(&ranked.encode()).unwrap(),
        );
        prop_assert_eq!(
            postcard::to_allocvec(&span).unwrap(),
            postcard::to_allocvec(&span.encode()).unwrap(),
        );
    }
}

/// Serde deserialization accepts only the representation selected by the
/// format and validates it strictly.
///
/// The binary path rejects trailing bytes, a rank/version mismatch, and crossed
/// span endpoints. The human-readable rank path rejects noncanonical text and
/// byte arrays.
#[test]
fn serde_rejects_defective_rank_and_span_payloads() {
    let (older, newer) = ordered_pair();
    let span = Span::new(&older, &newer).unwrap();

    let mut rank_trailing = older.rank().encode();
    rank_trailing.push(0x00);
    assert!(Rank::decode(&rank_trailing[..]).is_err());

    let mut ranked_trailing = Ranked::from(older.clone()).encode();
    ranked_trailing.push(0x00);
    assert!(Ranked::decode(&ranked_trailing[..]).is_err());

    let mut span_trailing = span.encode();
    span_trailing.push(0x00);
    assert!(Span::decode(&span_trailing[..]).is_err());

    // A Ranked prefix must equal the version's rank, and span endpoints must
    // remain ordered.
    let mismatched = [newer.rank().encode(), older.encode()].concat();
    assert!(Ranked::decode(&mismatched[..]).is_err());
    let crossed = [newer.encode(), older.encode()].concat();
    assert!(Span::decode(&crossed[..]).is_err());

    let postcard_frame = |body: &[u8]| postcard::to_allocvec(&body.to_vec()).unwrap();
    let json_frame = |body: &[u8]| serde_json::to_vec(&body.to_vec()).unwrap();
    for body in [&rank_trailing, &mismatched] {
        assert!(postcard::from_bytes::<Rank>(&postcard_frame(body)).is_err());
        assert!(serde_json::from_slice::<Rank>(&json_frame(body)).is_err());
    }
    for text in ["01", "1.0", " 1", "1 "] {
        assert!(serde_json::from_str::<Rank>(&format!("\"{text}\"")).is_err());
    }
    for body in [&ranked_trailing, &mismatched] {
        assert!(postcard::from_bytes::<Ranked>(&postcard_frame(body)).is_err());
        assert!(serde_json::from_slice::<Ranked>(&json_frame(body)).is_err());
    }
    for body in [&span_trailing, &crossed] {
        assert!(postcard::from_bytes::<Span>(&postcard_frame(body)).is_err());
        assert!(serde_json::from_slice::<Span>(&json_frame(body)).is_err());
    }
}

/// The serde implementations compose in a larger binary value.
#[test]
fn serde_composes_rank_and_span_in_larger_values() {
    let (older, newer) = ordered_pair();
    let tuple = (
        Span::new(&older, &newer).unwrap(),
        older.rank(),
        Ranked::from(newer.clone()),
    );
    let bytes = postcard::to_allocvec(&tuple).unwrap();
    let back: (Span, Rank, Ranked) = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(back, tuple);
}
