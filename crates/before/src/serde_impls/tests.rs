//! Checks of serde representations, strict decoding, and composition.

use core::fmt::Debug;

use proptest::prelude::*;
use serde::de::value::{Error as ValueError, MapDeserializer, SeqDeserializer, StrDeserializer};
use serde::de::{DeserializeOwned, IntoDeserializer};
use serde_json::Value;
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

/// Deserializes `T` from a human-readable map holding `entries`, in order.
///
/// `serde`'s `MapDeserializer` reports itself as human-readable, so it stands
/// in for any human-readable format, and it spells maps that JSON cannot:
/// keys given as bytes or as integers.
fn read_map<'a, T, K>(entries: impl IntoIterator<Item = (K, &'a str)>) -> Result<T, ValueError>
where
    T: DeserializeOwned,
    K: IntoDeserializer<'a, ValueError>,
{
    T::deserialize(MapDeserializer::new(entries.into_iter()))
}

/// Deserializes `T` from a human-readable sequence holding `texts`, in order.
fn read_seq<'a, T>(texts: impl IntoIterator<Item = &'a str>) -> Result<T, ValueError>
where
    T: DeserializeOwned,
{
    T::deserialize(SeqDeserializer::new(texts.into_iter()))
}

/// Checks that a human-readable deserializer reads `value` from every complete
/// form in which a format may hand over its fields, and rejects incomplete or
/// overfull ones.
///
/// `fields` holds each field's name and text, in the order the named record
/// lists them.
///
/// The accepted forms are:
///
/// - the named record keyed by strings, as JSON writes it, and with its
///   entries reversed, as formats that sort their keys (bencode, for one) may
///   write it;
/// - the named record keyed by bytes, as csv hands over its header keys;
/// - the field texts in order, as a sequence, which is how csv without
///   headers and `rmp-serde`'s human-readable tuple mode write a struct, and
///   also as a JSON array;
/// - the field texts in a map keyed by field position.
///
/// The rejected forms are the string-keyed record with a field omitted, with
/// a field repeated, or with an unknown field added, and the sequence without
/// its last field text.
fn assert_readable_reads_every_complete_field_form<T>(
    value: &T,
    fields: &[(&'static str, String)],
) -> Result<(), TestCaseError>
where
    T: DeserializeOwned + PartialEq + Debug,
{
    let named: Vec<(&str, &str)> = fields
        .iter()
        .map(|(name, text)| (*name, text.as_str()))
        .collect();
    let texts = named.iter().map(|&(_, text)| text);
    let json_array = Value::Array(texts.clone().map(Value::from).collect());

    let accepted: [(&str, Result<T, String>); 6] = [
        (
            "string keys",
            read_map(named.iter().copied()).map_err(|e| e.to_string()),
        ),
        (
            "string keys in reverse order",
            read_map(named.iter().rev().copied()).map_err(|e| e.to_string()),
        ),
        (
            "byte keys",
            read_map(named.iter().map(|&(name, text)| (name.as_bytes(), text)))
                .map_err(|e| e.to_string()),
        ),
        (
            "a sequence",
            read_seq(texts.clone()).map_err(|e| e.to_string()),
        ),
        (
            "a JSON array",
            serde_json::from_value(json_array).map_err(|e| e.to_string()),
        ),
        (
            "position keys",
            read_map((0u64..).zip(texts.clone())).map_err(|e| e.to_string()),
        ),
    ];
    for (form, result) in accepted {
        prop_assert_eq!(
            result.as_ref(),
            Ok(value),
            "the fields as {} decoded otherwise",
            form
        );
    }

    let mut rejected: Vec<(String, Result<T, ValueError>)> = Vec::new();
    for (index, &(name, text)) in named.iter().enumerate() {
        let mut omitted = named.clone();
        omitted.remove(index);
        rejected.push((format!("the record without {name:?}"), read_map(omitted)));

        let mut repeated = named.clone();
        repeated.push((name, text));
        rejected.push((format!("the record repeating {name:?}"), read_map(repeated)));
    }
    let mut extended = named.clone();
    extended.push(("unknown", named[0].1));
    rejected.push((
        "the record with an unknown field".to_owned(),
        read_map(extended),
    ));
    let short = texts.take(named.len() - 1);
    rejected.push((
        "the sequence without its last field".to_owned(),
        read_seq(short),
    ));
    for (form, result) in rejected {
        prop_assert!(result.is_err(), "{} decoded as {:?}", form, result);
    }
    Ok(())
}

proptest! {
    /// A human-readable deserializer reads a clock, span, or ranked view from
    /// its complete named record or from its field texts in order, and
    /// rejects incomplete or overfull forms.
    ///
    /// The named record's keys may be strings or bytes, in any order; the
    /// field texts come in the order the record lists them, as a sequence or
    /// keyed by position. A record that omits, repeats, or adds a field is
    /// rejected, and so is a sequence that is short a field. The module doc
    /// says why fields in order are accepted.
    #[test]
    fn readable_serde_reads_complete_records_by_name_or_in_order(
        op in arb_oracle_party_nonempty(),
        oa in arb_oracle_version(),
        ob in arb_oracle_version(),
    ) {
        let a = from_oracle_version(&oa);
        let b = from_oracle_version(&ob);

        let clock = Clock::from_parts(from_oracle_party(&op), a.clone());
        assert_readable_reads_every_complete_field_form(
            &clock,
            &[
                ("party", clock.party().to_string()),
                ("version", clock.version().to_string()),
            ],
        )?;

        let span = a.span(&b);
        assert_readable_reads_every_complete_field_form(
            &span,
            &[("lo", span.lo().to_string()), ("hi", span.hi().to_string())],
        )?;

        let ranked = Ranked::from(a);
        assert_readable_reads_every_complete_field_form(
            &ranked,
            &[("version", ranked.version().to_string())],
        )?;
    }
}

/// Deserializes `T` from `text` through `T`'s own human-readable
/// deserializer, returning `None` when it rejects the text.
fn read_text<T: DeserializeOwned>(text: &str) -> Option<T> {
    let text: StrDeserializer<'_, ValueError> = text.into_deserializer();
    T::deserialize(text).ok()
}

/// A text for one record field: arbitrary, or the field's valid text under
/// one small edit that a lenient parser might forgive.
#[derive(Clone, Debug)]
enum FieldText {
    /// Arbitrary text, almost always invalid.
    Arbitrary(String),
    /// The valid text, unchanged.
    Valid,
    /// The valid text after a space.
    LeadingSpace,
    /// The valid text before a space.
    TrailingSpace,
    /// The valid text in upper case.
    Uppercase,
    /// The valid text after a zero.
    LeadingZero,
    /// The valid text without its last character.
    Truncated,
}

impl FieldText {
    /// Returns this field text, given the field's valid text.
    fn apply(&self, valid: &str) -> String {
        match self {
            Self::Arbitrary(text) => text.clone(),
            Self::Valid => valid.to_owned(),
            Self::LeadingSpace => format!(" {valid}"),
            Self::TrailingSpace => format!("{valid} "),
            Self::Uppercase => valid.to_uppercase(),
            Self::LeadingZero => format!("0{valid}"),
            Self::Truncated => {
                let mut text = valid.to_owned();
                text.pop();
                text
            }
        }
    }
}

/// Generates every [`FieldText`] variant, so valid, nearly valid, and
/// arbitrary texts all occur.
fn arb_field_text() -> impl Strategy<Value = FieldText> {
    prop_oneof![
        any::<String>().prop_map(FieldText::Arbitrary),
        Just(FieldText::Valid),
        Just(FieldText::LeadingSpace),
        Just(FieldText::TrailingSpace),
        Just(FieldText::Uppercase),
        Just(FieldText::LeadingZero),
        Just(FieldText::Truncated),
    ]
}

/// Checks that `fields`, each a field name and that field's text, decode to
/// `expected` (`None` for a rejection) both as a string-keyed record and as a
/// sequence of their texts in order.
///
/// Reading both forms catches a deserializer that parses field text one way in
/// its map path and another way in its sequence path.
fn assert_fields_decode_to<T>(
    fields: &[(&str, &str)],
    expected: &Option<T>,
    case: &str,
) -> Result<(), TestCaseError>
where
    T: DeserializeOwned + PartialEq + Debug,
{
    let from_record = read_map::<T, _>(fields.iter().copied()).ok();
    prop_assert_eq!(&from_record, expected, "{} as a record", case);
    let from_sequence = read_seq::<T>(fields.iter().map(|&(_, text)| text)).ok();
    prop_assert_eq!(&from_sequence, expected, "{} as a sequence", case);
    Ok(())
}

proptest! {
    /// A human-readable record reads each field's text exactly as the field's
    /// own type reads it, whether the fields come named or in order.
    ///
    /// When `Party` or `Version` accepts a text, the record holding it decodes
    /// to the value built from that field; when the type rejects the text, so
    /// does the record.
    ///
    /// Each field (a clock's party and version, a ranked view's version, and a
    /// span's endpoints) takes the edited text in turn, with every other field
    /// valid.
    #[test]
    fn readable_records_read_field_text_as_the_field_types_do(
        op in arb_oracle_party_nonempty(),
        oa in arb_oracle_version(),
        ob in arb_oracle_version(),
        edit in arb_field_text(),
    ) {
        let a = from_oracle_version(&oa);
        let span = a.span(&from_oracle_version(&ob));
        let party_text = from_oracle_party(&op).to_string();
        let a_text = a.to_string();
        let (lo_text, hi_text) = (span.lo().to_string(), span.hi().to_string());

        let party = edit.apply(&party_text);
        let expected = read_text::<Party>(&party).map(|party| Clock::from_parts(party, a.clone()));
        assert_fields_decode_to(
            &[("party", &party), ("version", &a_text)],
            &expected,
            &format!("clock party {party:?}"),
        )?;

        let version = edit.apply(&a_text);
        let expected = read_text::<Version>(&version)
            .map(|version| Clock::from_parts(from_oracle_party(&op), version));
        assert_fields_decode_to(
            &[("party", &party_text), ("version", &version)],
            &expected,
            &format!("clock version {version:?}"),
        )?;

        let expected = read_text::<Version>(&version).map(Ranked::from);
        assert_fields_decode_to(
            &[("version", &version)],
            &expected,
            &format!("ranked version {version:?}"),
        )?;

        let lo = edit.apply(&lo_text);
        let expected =
            read_text::<Version>(&lo).and_then(|lo| Span::new(lo, span.hi().clone()).ok());
        assert_fields_decode_to(
            &[("lo", &lo), ("hi", &hi_text)],
            &expected,
            &format!("span lo {lo:?}"),
        )?;

        let hi = edit.apply(&hi_text);
        let expected =
            read_text::<Version>(&hi).and_then(|hi| Span::new(span.lo().clone(), hi).ok());
        assert_fields_decode_to(
            &[("lo", &lo_text), ("hi", &hi)],
            &expected,
            &format!("span hi {hi:?}"),
        )?;
    }
}
