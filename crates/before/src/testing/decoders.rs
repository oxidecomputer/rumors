//! Totality and entry-point agreement of every decoder on near-valid
//! encodings.
//!
//! `Eq` and `Hash` on versions, parties, clocks, spans, and ranked keys
//! compare canonical bytes, and on ranks they compare the normalized
//! numerator and exponent, which the canonical rank encoding spells one to
//! one. A decoder that accepts a non-canonical spelling therefore builds a
//! value whose equality disagrees with its bytes. Every operation is also
//! promised total, so a decoder must return an error, never panic, on any
//! bytes.
//!
//! Each type decodes through several entry points ([`entry_points`]): its
//! reader `decode` on a whole and on a chunked, interrupting reader,
//! hexadecimal text (parties and versions only), and the serde impls under a
//! binary format (postcard and CBOR here). Borsh reads a value from a
//! stream, and a span also decodes from its human-readable record through
//! `Span::new`. Each is documented to accept exactly the canonical
//! encodings. The *slice decode*, the reader `decode` over the bytes in
//! memory, is the reference the others are compared against. The near-valid
//! strategies ([`near_valid`](crate::testing::generators::near_valid)) reach
//! deep trees, wide heights, and damaged encodings far past the committed
//! rejection tests' small trees, and two properties drive every entry point
//! with them:
//!
//! - **Totality.** Every whole-input entry point of every type, and borsh,
//!   decodes every near-valid encoding of every type without panicking. The
//!   span's human-readable record needs an endpoint split that arbitrary
//!   inputs lack, so only the agreement property drives it. A version decoder meets
//!   parties, ranks, and composites as well as versions: inputs that no
//!   type's own strategy draws for it.
//! - **Agreement.** For each type, [`check_agreement`] requires the slice
//!   decode to accept what construction settles as canonical, to reject what
//!   it settles as defective, and to accept nothing that is not canonical,
//!   and requires every other entry point to agree with the slice decode.
//!   Construction leaves some damaged inputs open. For those the property
//!   checks only that an acceptance is canonical and that the entry points
//!   agree, so a wrong rejection goes undetected; settling them against a
//!   specification is a separate instrument's job.
//!
//! No decoder promises which error variant reports an input with several
//! defects, so the entry points are compared on acceptance and decoded value
//! alone. The one documented precedence, a span's structural errors before
//! its endpoint ordering, is checked on inputs built to have both.

use borsh::BorshDeserialize;
use proptest::prelude::*;
use serde::de::DeserializeOwned;

use crate::error::{Decode, ParseValue};
use crate::testing::bridge::{to_oracle_party, to_oracle_version};
use crate::testing::canonical::Canonical;
use crate::testing::generators::near_valid::{
    arb_clock_encoding, arb_party_encoding, arb_rank_encoding, arb_ranked_encoding,
    arb_span_encoding, arb_version_encoding, Expect, NearValid,
};
use crate::testing::oracles::codec;
use crate::testing::scripted_io::{arb_io_schedule, IoSchedule, ScriptedReader};
use crate::{Clock, Party, Rank, Ranked, Span, Version};

/// A type with a canonical encoding and the entry points that decode it,
/// its reader `decode` included through [`Canonical`].
trait Wire: Canonical + DeserializeOwned + BorshDeserialize {
    /// Parses the type's hexadecimal text form, if it has one.
    fn parse_hex(_text: &str) -> Option<Result<Self, ParseValue>> {
        None
    }

    /// Decodes the type's human-readable record of two endpoints, if it has
    /// one, from the endpoints' bytes.
    fn from_record(_lo: &[u8], _hi: &[u8]) -> Option<Result<Self, String>> {
        None
    }

    /// The independent encoder's spelling of this value, or `None` when no
    /// canonical encoding spells it.
    ///
    /// The value is read back through the recursive oracle, normalized, and
    /// spelled by [`codec`], which shares no code with production's writers.
    /// Normalizing is what makes the comparison a canonicity check: a
    /// decoder that accepted a non-canonical spelling yields a value whose
    /// normalized spelling differs from the bytes it accepted.
    fn spelling(&self) -> Option<Vec<u8>>;
}

/// The normalized flat form of a version, read back through the recursive
/// oracle.
fn flat_version(version: &Version) -> codec::Version {
    codec::Version::from_oracle(&to_oracle_version(version)).normalized()
}

/// The normalized flat form of a party, read back through the recursive
/// oracle.
fn flat_party(party: &Party) -> codec::Party {
    codec::Party::from_oracle(&to_oracle_party(party)).normalized()
}

/// A version's spelling is its normalized flat form's encoding.
impl Wire for Version {
    fn parse_hex(text: &str) -> Option<Result<Self, ParseValue>> {
        Some(text.parse())
    }

    fn spelling(&self) -> Option<Vec<u8>> {
        Some(flat_version(self).encode())
    }
}

/// A party's spelling is its normalized flat form's encoding.
impl Wire for Party {
    fn parse_hex(text: &str) -> Option<Result<Self, ParseValue>> {
        Some(text.parse())
    }

    fn spelling(&self) -> Option<Vec<u8>> {
        Some(flat_party(self).encode())
    }
}

/// A rank's spelling is the encoding of its normalized numerator and
/// exponent.
impl Wire for Rank {
    fn spelling(&self) -> Option<Vec<u8>> {
        let (num, exp) = self.raw_parts();
        let (num, exp) = codec::normalize_rank(num.clone(), exp);
        Some(codec::encode_rank(&num, exp))
    }
}

/// A clock's spelling is its party's spelling followed by its version's.
impl Wire for Clock {
    fn spelling(&self) -> Option<Vec<u8>> {
        Some(
            [
                flat_party(self.party()).encode(),
                flat_version(self.version()).encode(),
            ]
            .concat(),
        )
    }
}

/// A span's spelling is its lower endpoint's spelling followed by its upper
/// endpoint's.
///
/// It exists only when the lower endpoint lies at or below the upper, which
/// the flat forms judge independently of production's comparison.
impl Wire for Span<'static> {
    fn from_record(lo: &[u8], hi: &[u8]) -> Option<Result<Self, String>> {
        let record = format!(
            r#"{{"lo":"{}","hi":"{}"}}"#,
            hex::encode(lo),
            hex::encode(hi)
        );
        Some(serde_json::from_str(&record).map_err(|error| error.to_string()))
    }

    fn spelling(&self) -> Option<Vec<u8>> {
        let (lo, hi) = (flat_version(self.lo()), flat_version(self.hi()));
        lo.le(&hi).then(|| [lo.encode(), hi.encode()].concat())
    }
}

/// A ranked key's spelling is the encoding of its version's rank followed by
/// the version's spelling.
///
/// The rank is the one the version's normalized flat form computes,
/// independently of production's rank fold.
impl Wire for Ranked<'static> {
    fn spelling(&self) -> Option<Vec<u8>> {
        let version = flat_version(self.version());
        let (num, exp) = version.rank();
        Some([codec::encode_rank(&num, exp), version.encode()].concat())
    }
}

/// One entry point's verdict on a whole input.
struct Outcome<T> {
    /// The entry point, for failure messages.
    entry: &'static str,
    /// The decoded value, or a description of the rejection.
    result: Result<T, String>,
    /// Whether a rejection reports a structural error, where the entry
    /// point's error says.
    structural: Option<bool>,
}

/// Whether a decode error is structural: the input ended early, or its
/// padding or trailing bytes are malformed.
fn is_structural(error: &Decode) -> bool {
    matches!(error, Decode::Truncated | Decode::TrailingBits)
}

/// The outcome of an entry point that reports a [`Decode`] error.
fn decoded<T>(entry: &'static str, result: Result<T, Decode>) -> Outcome<T> {
    let structural = result.as_ref().err().map(is_structural);
    Outcome {
        entry,
        result: result.map_err(|error| format!("{error:?}")),
        structural,
    }
}

/// Every entry point of `T` that reads the whole of `bytes`, the slice
/// decode first.
///
/// The chunked reader serves the bytes on `schedule`. Postcard and CBOR read
/// the bytes framed as one byte string; postcard's error drops the decoder's
/// message, while CBOR's keeps it.
fn entry_points<T: Wire>(bytes: &[u8], schedule: &IoSchedule) -> Vec<Outcome<T>> {
    let mut outcomes = vec![
        decoded("the slice decode", T::decode_reader(bytes)),
        decoded(
            "the chunked reader decode",
            T::decode_reader(ScriptedReader::new(bytes, schedule, None)),
        ),
    ];
    if let Some(parsed) = T::parse_hex(&hex::encode(bytes)) {
        let structural = match &parsed {
            Err(ParseValue::InvalidEncoding(error)) => Some(is_structural(error)),
            _ => None,
        };
        outcomes.push(Outcome {
            entry: "hexadecimal text",
            result: parsed.map_err(|error| format!("{error:?}")),
            structural,
        });
    }
    let field = serde_bytes::Bytes::new(bytes);
    let frame = postcard::to_allocvec(field).expect("postcard frames any byte string");
    outcomes.push(Outcome {
        entry: "postcard",
        result: postcard::from_bytes(&frame).map_err(|error| format!("{error:?}")),
        structural: None,
    });
    let mut frame = Vec::new();
    ciborium::ser::into_writer(field, &mut frame).expect("CBOR frames any byte string");
    let cbor = ciborium::de::from_reader::<T, _>(&frame[..]).map_err(|error| format!("{error:?}"));
    let structural = cbor.as_ref().err().map(|message| {
        [Decode::Truncated, Decode::TrailingBits]
            .iter()
            .any(|error| message.contains(&error.to_string()))
    });
    outcomes.push(Outcome {
        entry: "CBOR",
        result: cbor,
        structural,
    });
    outcomes
}

/// Whether a borsh error reports a structural defect: the reader ran out, or
/// the decoder classified the bytes as truncated or malformed padding.
fn is_borsh_structural(error: &borsh::io::Error) -> bool {
    error.kind() == borsh::io::ErrorKind::UnexpectedEof
        || error
            .get_ref()
            .and_then(|inner| inner.downcast_ref::<Decode>())
            .is_some_and(is_structural)
}

/// Requires an entry point's decoded value to equal the slice decode's, or
/// both to reject.
fn agree<T: Wire>(entry: &str, slice: Option<&T>, other: Option<&T>) -> Result<(), TestCaseError> {
    prop_assert_eq!(
        slice,
        other,
        "{} disagrees with the slice decode (left: slice, right: {})",
        entry,
        entry
    );
    Ok(())
}

/// Checks every entry point of `T` on one near-valid input against the slice
/// decode, `T::decode(&input.bytes[..])`.
///
/// 1. What construction settles holds: the slice decode accepts a canonical
///    encoding, rejects a defective one, and rejects a span with both a
///    structural defect and out-of-order endpoints with `Truncated` or
///    `TrailingBits`, the precedence `Span::decode` documents.
/// 2. An accepted encoding is canonical: it equals the independent encoder's
///    spelling of the decoded value ([`Wire::spelling`]). This judges every
///    accepted input, including those construction leaves open; an open
///    input the slice decode wrongly rejects is not detected here.
/// 3. Every other whole-input entry point ([`entry_points`]) accepts exactly
///    when the slice decode does, with an equal value. Where the span
///    precedence applies, every entry point whose error says reports a
///    structural one.
/// 4. Where construction knows a span's endpoint split, the human-readable
///    record of the two endpoints decodes exactly when the slice decode
///    accepts, to an equal span.
/// 5. Borsh reads one value from the bytes followed by `junk`. When the
///    slice decode accepts, borsh decodes an equal value and consumes
///    exactly the bytes, leaving the junk unread. When borsh accepts, the
///    bytes it consumed are a canonical encoding of its value by the slice
///    decode. Where the span precedence applies to a defect inside the
///    encoding, borsh on the bytes alone reports a structural error.
fn check_agreement<T: Wire>(
    input: &NearValid,
    schedule: &IoSchedule,
    junk: &[u8],
) -> Result<(), TestCaseError> {
    let bytes = &input.bytes[..];
    let outcomes = entry_points::<T>(bytes, schedule);
    let slice = &outcomes[0];
    let precedence = matches!(input.expect, Expect::StructuralOverOrdering { .. });
    match input.expect {
        Expect::Canonical => prop_assert!(
            slice.result.is_ok(),
            "the slice decode rejected a canonical encoding: {:?}",
            slice.result.as_ref().err()
        ),
        Expect::Defective => prop_assert!(
            slice.result.is_err(),
            "the slice decode accepted a defective encoding"
        ),
        Expect::StructuralOverOrdering { .. } | Expect::Open => {}
    }
    let accepted = slice.result.as_ref().ok();
    if let Some(value) = accepted {
        let spelling = value.spelling();
        prop_assert_eq!(
            spelling.as_deref(),
            Some(bytes),
            "an accepted encoding must be the canonical spelling of its value"
        );
    }

    for outcome in &outcomes {
        agree(outcome.entry, accepted, outcome.result.as_ref().ok())?;
        prop_assert!(
            !precedence || outcome.structural != Some(false),
            "{} reported endpoint order before a structural error, breaking the \
             span precedence: {:?}",
            outcome.entry,
            outcome.result.as_ref().err()
        );
    }
    if precedence {
        prop_assert_eq!(
            slice.structural,
            Some(true),
            "a span with a structural defect and out-of-order endpoints must report \
             the structural error"
        );
    }

    if let Some(split) = input.split {
        let (lo, hi) = bytes.split_at(split);
        if let Some(record) = T::from_record(lo, hi) {
            agree("the human-readable record", accepted, record.as_ref().ok())?;
        }
    }

    let stream = [bytes, junk].concat();
    let mut unread = &stream[..];
    let streamed = T::deserialize_reader(&mut unread);
    let consumed = stream.len() - unread.len();
    match (accepted, &streamed) {
        (Some(value), _) => {
            prop_assert_eq!(
                streamed.as_ref().ok(),
                Some(value),
                "borsh rejected or changed a value"
            );
            prop_assert_eq!(
                consumed,
                bytes.len(),
                "borsh consumed a different byte count"
            );
        }
        (None, Ok(value)) => {
            let prefix = T::decode_reader(&stream[..consumed]);
            prop_assert_eq!(
                prefix.as_ref().ok(),
                Some(value),
                "borsh accepted {} bytes the slice decode does not accept as its value",
                consumed
            );
        }
        (None, Err(_)) => {}
    }
    if input.expect == (Expect::StructuralOverOrdering { in_stream: true }) {
        let mut unread = bytes;
        let streamed = T::deserialize_reader(&mut unread);
        prop_assert!(
            streamed.as_ref().err().is_some_and(is_borsh_structural),
            "borsh broke the span precedence: {:?}",
            streamed
        );
    }
    Ok(())
}

/// Decodes `bytes` through every entry point of `T`, discarding the
/// verdicts.
fn decode_everywhere<T: Wire>(bytes: &[u8], schedule: &IoSchedule) {
    drop(entry_points::<T>(bytes, schedule));
    let _ = T::deserialize_reader(&mut &bytes[..]);
}

/// A near-valid encoding of any of the six types, for decoding as every
/// type.
fn arb_any_encoding() -> impl Strategy<Value = NearValid> {
    prop_oneof![
        arb_version_encoding(),
        arb_party_encoding(),
        arb_rank_encoding(),
        arb_clock_encoding(),
        arb_span_encoding(),
        arb_ranked_encoding(),
    ]
}

/// Up to five bytes for borsh to leave unread after a value.
fn arb_junk() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..6)
}

proptest! {
    /// No decoder panics on any near-valid encoding of any type.
    ///
    /// Each input, whatever type it was built for, goes through every entry
    /// point of all six types: the reader decode on a whole and a chunked
    /// reader, hexadecimal text where the type has it, postcard, CBOR, and
    /// borsh. Proptest reports a panic as a failing case, so the property
    /// needs no assertion of its own.
    #[test]
    fn every_decoder_is_total_on_near_valid_encodings(
        input in arb_any_encoding(),
        schedule in arb_io_schedule(),
    ) {
        decode_everywhere::<Version>(&input.bytes, &schedule);
        decode_everywhere::<Party>(&input.bytes, &schedule);
        decode_everywhere::<Rank>(&input.bytes, &schedule);
        decode_everywhere::<Clock>(&input.bytes, &schedule);
        decode_everywhere::<Span<'static>>(&input.bytes, &schedule);
        decode_everywhere::<Ranked<'static>>(&input.bytes, &schedule);
    }

    /// `Version`'s decoders accept nothing that is not canonical and agree
    /// with one another on near-valid version encodings.
    ///
    /// As [`check_agreement`] states, the slice decode also accepts what
    /// construction settles as canonical and rejects what it settles as
    /// defective.
    #[test]
    fn version_decoders_agree_on_near_valid_encodings(
        input in arb_version_encoding(),
        schedule in arb_io_schedule(),
        junk in arb_junk(),
    ) {
        check_agreement::<Version>(&input, &schedule, &junk)?;
    }

    /// `Party`'s decoders accept nothing that is not canonical and agree
    /// with one another on near-valid party encodings.
    ///
    /// As [`check_agreement`] states, the slice decode also accepts what
    /// construction settles as canonical and rejects what it settles as
    /// defective.
    #[test]
    fn party_decoders_agree_on_near_valid_encodings(
        input in arb_party_encoding(),
        schedule in arb_io_schedule(),
        junk in arb_junk(),
    ) {
        check_agreement::<Party>(&input, &schedule, &junk)?;
    }

    /// `Rank`'s decoders accept nothing that is not canonical and agree
    /// with one another on near-valid rank encodings.
    ///
    /// As [`check_agreement`] states, the slice decode also accepts what
    /// construction settles as canonical and rejects what it settles as
    /// defective.
    #[test]
    fn rank_decoders_agree_on_near_valid_encodings(
        input in arb_rank_encoding(),
        schedule in arb_io_schedule(),
        junk in arb_junk(),
    ) {
        check_agreement::<Rank>(&input, &schedule, &junk)?;
    }

    /// `Clock`'s decoders accept nothing that is not canonical and agree
    /// with one another on near-valid clock encodings.
    ///
    /// As [`check_agreement`] states, the slice decode also accepts what
    /// construction settles as canonical and rejects what it settles as
    /// defective.
    #[test]
    fn clock_decoders_agree_on_near_valid_encodings(
        input in arb_clock_encoding(),
        schedule in arb_io_schedule(),
        junk in arb_junk(),
    ) {
        check_agreement::<Clock>(&input, &schedule, &junk)?;
    }

    /// `Span`'s decoders accept nothing that is not canonical and ordered and
    /// agree with one another on near-valid span encodings.
    ///
    /// As [`check_agreement`] states, the slice decode also accepts what
    /// construction settles as canonical and rejects what it settles as
    /// defective, the human-readable record agrees where construction knows
    /// the endpoint split, and every entry point reports structural errors
    /// before endpoint ordering.
    #[test]
    fn span_decoders_agree_on_near_valid_encodings(
        input in arb_span_encoding(),
        schedule in arb_io_schedule(),
        junk in arb_junk(),
    ) {
        check_agreement::<Span<'static>>(&input, &schedule, &junk)?;
    }

    /// `Ranked`'s decoders accept nothing that is not canonical with its
    /// version's own rank and agree with one another on near-valid ranked-key
    /// encodings.
    ///
    /// As [`check_agreement`] states, the slice decode also accepts what
    /// construction settles as canonical and rejects what it settles as
    /// defective.
    #[test]
    fn ranked_decoders_agree_on_near_valid_encodings(
        input in arb_ranked_encoding(),
        schedule in arb_io_schedule(),
        junk in arb_junk(),
    ) {
        check_agreement::<Ranked<'static>>(&input, &schedule, &junk)?;
    }
}
