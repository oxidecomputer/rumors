//! Checks of serde representations, strict decoding, and composition.

use proptest::prelude::*;
use serde_test::{assert_tokens, Configure, Token};

use crate::span::Span;
use crate::testing::bridge::{from_oracle_party, from_oracle_version};
use crate::testing::generators::{arb_magnitude, arb_oracle_party_nonempty, arb_oracle_version};
use crate::{Clock, Count, Party, Rank, Ranked, Version};

/// Two strictly ordered versions (`older < newer`) from one history.
fn ordered_pair() -> (Version, Version) {
    let mut clock = Clock::seed();
    let older = clock.tick().clone();
    let newer = clock.tick().clone();
    (older, newer)
}

/// Convert an owned encoding into a static `serde_test` byte token.
fn byte_token(bytes: Vec<u8>) -> Token {
    Token::Bytes(bytes.leak())
}

/// Convert owned text into a static serde-test token.
fn text_token(text: String) -> Token {
    Token::Str(text.leak())
}

/// Binary formats preserve the byte encodings; readable formats use the public
/// strings and named records.
#[test]
fn serde_data_model_matches_each_format_class() {
    let party = Party::seed();
    let bytes = party.encode();
    let text = party.to_string();
    assert_tokens(&party.compact(), &[byte_token(bytes)]);
    assert_tokens(&Party::seed().readable(), &[text_token(text)]);

    let version = Version::new();
    let bytes = version.encode();
    let text = version.to_string();
    assert_tokens(&version.clone().compact(), &[byte_token(bytes)]);
    assert_tokens(&version.readable(), &[text_token(text)]);

    let clock = Clock::seed();
    let bytes = clock.encode();
    let party_text = clock.party().to_string();
    let version_text = clock.version().to_string();
    assert_tokens(&clock.compact(), &[byte_token(bytes)]);
    let clock = Clock::seed();
    assert_tokens(
        &clock.readable(),
        &[
            Token::Struct {
                name: "Clock",
                len: 2,
            },
            Token::Str("party"),
            text_token(party_text),
            Token::Str("version"),
            text_token(version_text),
            Token::StructEnd,
        ],
    );

    let rank: Rank = "101.01".parse().unwrap();
    let bytes = rank.encode();
    assert_tokens(&rank.clone().compact(), &[byte_token(bytes)]);
    assert_tokens(&rank.readable(), &[Token::Str("101.01")]);

    let ranked = Ranked::from(Version::new());
    let bytes = ranked.encode();
    let version_text = ranked.version().to_string();
    assert_tokens(&ranked.clone().compact(), &[byte_token(bytes)]);
    assert_tokens(
        &ranked.readable(),
        &[
            Token::Struct {
                name: "Ranked",
                len: 1,
            },
            Token::Str("version"),
            text_token(version_text),
            Token::StructEnd,
        ],
    );

    let (older, newer) = ordered_pair();
    let span = Span::new(&older, &newer).unwrap().into_owned();
    let bytes = span.encode();
    let lo_text = span.lo().to_string();
    let hi_text = span.hi().to_string();
    assert_tokens(&span.clone().compact(), &[byte_token(bytes)]);
    assert_tokens(
        &span.readable(),
        &[
            Token::Struct {
                name: "Span",
                len: 2,
            },
            Token::Str("lo"),
            text_token(lo_text),
            Token::Str("hi"),
            text_token(hi_text),
            Token::StructEnd,
        ],
    );

    let count = Count::from(u128::MAX) + Count::from(1u8);
    let text = count.to_string();
    assert_tokens(
        &count.clone().compact(),
        &[
            Token::Seq { len: Some(3) },
            Token::U64(0),
            Token::U64(0),
            Token::U64(1),
            Token::SeqEnd,
        ],
    );
    assert_tokens(&count.clone().readable(), &[text_token(text)]);
}

proptest! {
    /// Every serde-enabled value round-trips through representative readable
    /// and binary formats.
    ///
    /// The property also checks that JSON exposes a rank as its canonical text
    /// and that CBOR receives byte strings from binary serialization.
    #[test]
    fn serde_roundtrips_every_supported_type(
        op in arb_oracle_party_nonempty(),
        oa in arb_oracle_version(),
        ob in arb_oracle_version(),
        magnitude in arb_magnitude(),
    ) {
        let party = from_oracle_party(&op);
        let a = from_oracle_version(&oa);
        let b = from_oracle_version(&ob);
        let clock = Clock::from_parts(party, a.clone());
        let rank = a.rank();
        let ranked = Ranked::from(a.clone());
        let span = a.span(&b);
        let count = Count(magnitude);

        let c2: Clock = serde_json::from_slice(&serde_json::to_vec(&clock).unwrap()).unwrap();
        let r2: Rank = serde_json::from_slice(&serde_json::to_vec(&rank).unwrap()).unwrap();
        let k2: Ranked = serde_json::from_slice(&serde_json::to_vec(&ranked).unwrap()).unwrap();
        let s2: Span = serde_json::from_slice(&serde_json::to_vec(&span).unwrap()).unwrap();
        let n2: Count = serde_json::from_slice(&serde_json::to_vec(&count).unwrap()).unwrap();
        prop_assert_eq!(&c2, &clock);
        prop_assert_eq!(&r2, &rank);
        prop_assert_eq!(&k2, &ranked);
        prop_assert_eq!(&s2, &span);
        prop_assert_eq!(&n2, &count);
        prop_assert_eq!(serde_json::to_value(clock.party()).unwrap(), clock.party().to_string());
        prop_assert_eq!(serde_json::to_value(clock.version()).unwrap(), clock.version().to_string());
        prop_assert_eq!(
            serde_json::to_value(&rank).unwrap(),
            serde_json::Value::String(rank.to_string()),
        );
        prop_assert_eq!(serde_json::to_value(&count).unwrap(), count.to_string());

        let c2: Clock = postcard::from_bytes(&postcard::to_allocvec(&clock).unwrap()).unwrap();
        let r2: Rank = postcard::from_bytes(&postcard::to_allocvec(&rank).unwrap()).unwrap();
        let k2: Ranked = postcard::from_bytes(&postcard::to_allocvec(&ranked).unwrap()).unwrap();
        let s2: Span = postcard::from_bytes(&postcard::to_allocvec(&span).unwrap()).unwrap();
        let n2: Count = postcard::from_bytes(&postcard::to_allocvec(&count).unwrap()).unwrap();
        prop_assert_eq!(&c2, &clock);
        prop_assert_eq!(&r2, &rank);
        prop_assert_eq!(&k2, &ranked);
        prop_assert_eq!(&s2, &span);
        prop_assert_eq!(&n2, &count);

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

    /// Postcard receives exactly each value's canonical byte encoding.
    ///
    /// Serializing the value and serializing its encoded bytes must therefore
    /// produce identical postcard messages.
    #[test]
    fn binary_serde_preserves_existing_canonical_encodings(
        op in arb_oracle_party_nonempty(),
        oa in arb_oracle_version(),
        ob in arb_oracle_version(),
    ) {
        let party = from_oracle_party(&op);
        let a = from_oracle_version(&oa);
        let b = from_oracle_version(&ob);
        let clock = Clock::from_parts(party, a.clone());
        let rank = a.rank();
        let ranked = Ranked::from(a.clone());
        let span = a.span(&b);

        prop_assert_eq!(
            postcard::to_allocvec(clock.party()).unwrap(),
            postcard::to_allocvec(&clock.party().as_bytes()).unwrap(),
        );
        prop_assert_eq!(
            postcard::to_allocvec(clock.version()).unwrap(),
            postcard::to_allocvec(&clock.version().as_bytes()).unwrap(),
        );
        prop_assert_eq!(
            postcard::to_allocvec(&clock).unwrap(),
            postcard::to_allocvec(&clock.encode()).unwrap(),
        );
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

    /// Count's binary serde form is exactly its canonical limb sequence, and
    /// a redundant high zero limb is rejected.
    #[test]
    fn count_binary_serde_is_canonical(value in arb_magnitude()) {
        let count = Count(value);
        let mut limbs = count.limbs().collect::<Vec<_>>();
        prop_assert_eq!(
            postcard::to_allocvec(&count).unwrap(),
            postcard::to_allocvec(&limbs).unwrap(),
        );

        limbs.push(0);
        let redundant = postcard::to_allocvec(&limbs).unwrap();
        prop_assert!(postcard::from_bytes::<Count>(&redundant).is_err());
    }
}

/// Deserialization rejects malformed bytes and noncanonical readable values.
///
/// The malformed binary cases cover trailing input, a rank/version mismatch,
/// and reversed span endpoints. JSON additionally rejects noncanonical rank
/// text, crossed readable span endpoints, and byte-array substitutions for
/// textual ranks.
#[test]
fn serde_rejects_defective_payloads() {
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
    assert!(serde_json::from_value::<Span>(serde_json::json!({
        "lo": newer.to_string(),
        "hi": older.to_string(),
    }))
    .is_err());
    assert!(serde_json::from_str::<Count>("\"01\"").is_err());
    for body in [&ranked_trailing, &mismatched] {
        assert!(postcard::from_bytes::<Ranked>(&postcard_frame(body)).is_err());
        assert!(serde_json::from_slice::<Ranked>(&json_frame(body)).is_err());
    }
    for body in [&span_trailing, &crossed] {
        assert!(postcard::from_bytes::<Span>(&postcard_frame(body)).is_err());
        assert!(serde_json::from_slice::<Span>(&json_frame(body)).is_err());
    }
}

/// The serde implementations preserve tuple framing in postcard.
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
