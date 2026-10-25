//! Totality and specified verdicts of every decoder, on near-valid encodings
//! and on short arbitrary bytes.
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
//! memory, is the reference the other production paths are compared against,
//! and the specification ([`verdict`]) is the reference for all of them. The
//! near-valid strategies ([`near_valid`](crate::testing::generators::near_valid))
//! reach deep trees, wide heights, and damaged encodings far past the
//! committed rejection tests' small trees, and two properties drive every
//! entry point with them:
//!
//! - **Totality.** Every entry point of every type decodes every near-valid
//!   encoding of every type without panicking. A version decoder meets
//!   parties, ranks, and composites as well as versions: inputs that no
//!   type's own strategy draws for it.
//! - **Agreement.** For each type, [`check_agreement`] requires every
//!   whole-input entry point and borsh to follow the specification's verdict
//!   on every input ([`follows`]): to accept exactly the canonical encodings,
//!   each as the value the specification reads, and to reject with an error
//!   that names a class the input carries. Postcard alone is exempt from
//!   naming the class, because its error drops the decoder's message, but its
//!   rejection must still come from the decoder. The span's human-readable
//!   record decodes through `Span::new` rather than the binary decoder, so it
//!   is judged by agreement with the slice decode instead. The property also
//!   requires the entry points to agree with the slice decode, and the
//!   specification to agree with what construction settles.
//!
//! Two more tests per type hold the slice decode and borsh to the
//! specification, and the record to the slice decode, on short arbitrary
//! bytes, which the near-valid strategies reach only by chance
//! ([`check_short_input`]). One decodes every input of up to
//! [`EXHAUSTED_LEN`] bytes; the other is a property that draws fresh inputs
//! of the next few lengths, up to [`SAMPLED_LEN`] bytes, on every run.
//!
//! No decoder promises which error variant reports an input with several
//! defects, so a rejection may name any class the input carries. The one
//! documented precedence, a span's structural errors before its endpoint
//! ordering, narrows the classes the specification permits for a span.

use std::fmt::Debug;

use borsh::BorshDeserialize;
use num_bigint::BigUint;
use proptest::prelude::*;
use serde::de::DeserializeOwned;

use crate::error::{Decode, ParseValue};
use crate::testing::bridge::{to_oracle_party, to_oracle_version};
use crate::testing::canonical::Canonical;
use crate::testing::generators::near_valid::{
    arb_clock_encoding, arb_party_encoding, arb_rank_encoding, arb_ranked_encoding,
    arb_span_encoding, arb_version_encoding, Expect, NearValid,
};
use crate::testing::oracles::codec::verdict::{self, Class, Ending, Verdict};
use crate::testing::oracles::{codec, flat};
use crate::testing::scripted_io::{arb_io_schedule, IoSchedule, ScriptedReader};
use crate::{Clock, Party, Rank, Ranked, Span, Version};

/// A type with a canonical encoding and the entry points that decode it,
/// its reader `decode` included through [`Canonical`].
trait Wire: Canonical + DeserializeOwned + BorshDeserialize {
    /// A value of the type as the specification reads it.
    type Spec: PartialEq + Debug;

    /// Parses the type's hexadecimal text form, if it has one.
    fn parse_hex(_text: &str) -> Option<Result<Self, ParseValue>> {
        None
    }

    /// Decodes the type's human-readable record of `bytes`, if it has one
    /// and the specification frames its fields in `bytes`.
    fn record(_bytes: &[u8]) -> Option<Result<Self, String>> {
        None
    }

    /// The specification's verdict on `bytes`.
    fn verdict(bytes: &[u8], ending: Ending) -> Verdict<Self::Spec>;

    /// The decoded value in the specification's representation.
    fn read_back(&self) -> Self::Spec;

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

/// A version reads back as its normalized flat form, and spells as that
/// form's encoding.
impl Wire for Version {
    type Spec = codec::Version;

    fn parse_hex(text: &str) -> Option<Result<Self, ParseValue>> {
        Some(text.parse())
    }

    fn verdict(bytes: &[u8], ending: Ending) -> Verdict<codec::Version> {
        verdict::version(bytes, ending)
    }

    fn read_back(&self) -> codec::Version {
        flat_version(self)
    }

    fn spelling(&self) -> Option<Vec<u8>> {
        Some(flat_version(self).encode())
    }
}

/// A party reads back as its normalized flat form, and spells as that
/// form's encoding.
impl Wire for Party {
    type Spec = codec::Party;

    fn parse_hex(text: &str) -> Option<Result<Self, ParseValue>> {
        Some(text.parse())
    }

    fn verdict(bytes: &[u8], ending: Ending) -> Verdict<codec::Party> {
        verdict::party(bytes, ending)
    }

    fn read_back(&self) -> codec::Party {
        flat_party(self)
    }

    fn spelling(&self) -> Option<Vec<u8>> {
        Some(flat_party(self).encode())
    }
}

/// A rank reads back as its stored numerator and exponent, which its
/// invariant keeps normalized, and spells as the encoding of their
/// normalized form.
impl Wire for Rank {
    type Spec = (BigUint, u64);

    fn verdict(bytes: &[u8], ending: Ending) -> Verdict<(BigUint, u64)> {
        verdict::rank(bytes, ending)
    }

    fn read_back(&self) -> (BigUint, u64) {
        let (num, exp) = self.raw_parts();
        (num.clone(), exp)
    }

    fn spelling(&self) -> Option<Vec<u8>> {
        let (num, exp) = self.raw_parts();
        let (num, exp) = codec::normalize_rank(num.clone(), exp);
        Some(codec::encode_rank(&num, exp))
    }
}

/// A clock reads back as its party's and its version's flat forms, and
/// spells as its party's spelling followed by its version's.
impl Wire for Clock {
    type Spec = (codec::Party, codec::Version);

    fn verdict(bytes: &[u8], ending: Ending) -> Verdict<(codec::Party, codec::Version)> {
        verdict::clock(bytes, ending)
    }

    fn read_back(&self) -> (codec::Party, codec::Version) {
        (flat_party(self.party()), flat_version(self.version()))
    }

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

/// A span reads back as its endpoints' flat forms, and spells as its lower
/// endpoint's spelling followed by its upper endpoint's.
///
/// The spelling exists only when the lower endpoint lies at or below the
/// upper, which the flat model judges independently of production's
/// comparison.
impl Wire for Span<'static> {
    type Spec = (codec::Version, codec::Version);

    /// Splits `bytes` where the specification frames the lower endpoint, and
    /// decodes the record of the two parts.
    fn record(bytes: &[u8]) -> Option<Result<Self, String>> {
        let (lo, hi) = bytes.split_at(verdict::span_lower_len(bytes)?);
        let record = format!(
            r#"{{"lo":"{}","hi":"{}"}}"#,
            hex::encode(lo),
            hex::encode(hi)
        );
        Some(serde_json::from_str(&record).map_err(|error| error.to_string()))
    }

    fn verdict(bytes: &[u8], ending: Ending) -> Verdict<(codec::Version, codec::Version)> {
        verdict::span(bytes, ending)
    }

    fn read_back(&self) -> (codec::Version, codec::Version) {
        (flat_version(self.lo()), flat_version(self.hi()))
    }

    fn spelling(&self) -> Option<Vec<u8>> {
        let (lo, hi) = (flat_version(self.lo()), flat_version(self.hi()));
        flat::Version::from_spelling(&lo)
            .le(&flat::Version::from_spelling(&hi))
            .then(|| [lo.encode(), hi.encode()].concat())
    }
}

/// A ranked key reads back as its version's flat form, and spells as the
/// encoding of that version's rank followed by the version's spelling.
///
/// The rank is the flat model's, computed independently of production's
/// rank fold.
impl Wire for Ranked<'static> {
    type Spec = codec::Version;

    fn verdict(bytes: &[u8], ending: Ending) -> Verdict<codec::Version> {
        verdict::ranked(bytes, ending)
    }

    fn read_back(&self) -> codec::Version {
        flat_version(self.version())
    }

    fn spelling(&self) -> Option<Vec<u8>> {
        let version = flat_version(self.version());
        let (num, exp) = flat::Version::from_spelling(&version).rank();
        Some([codec::encode_rank(&num, exp), version.encode()].concat())
    }
}

/// The defect class a decode error names, or `None` for a reader failure or
/// an input too large to measure, which no test input can reach.
fn class_of(error: &Decode) -> Option<Class> {
    match error {
        Decode::Truncated => Some(Class::Truncated),
        Decode::TrailingBits => Some(Class::TrailingBits),
        Decode::NotCanonical => Some(Class::NotCanonical),
        _ => None,
    }
}

/// The defect class whose `Decode` message an error message carries, if
/// any.
fn class_in_message(message: &str) -> Option<Class> {
    [
        (Decode::Truncated, Class::Truncated),
        (Decode::TrailingBits, Class::TrailingBits),
        (Decode::NotCanonical, Class::NotCanonical),
    ]
    .into_iter()
    .find_map(|(error, class)| message.contains(&error.to_string()).then_some(class))
}

/// What a rejection says about its cause.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cause {
    /// The error names this defect class.
    Class(Class),
    /// The error says only that the type's decoder raised it: postcard's,
    /// which keeps a deserializer's custom error but drops its message.
    Decoder,
    /// The error names no defect class: a reader failure, a syntax error, or
    /// a framing error raised before the type's decoder ran.
    Unexplained,
}

impl Cause {
    /// The cause an error naming `class`, or no class, gives.
    fn named(class: Option<Class>) -> Cause {
        class.map_or(Cause::Unexplained, Cause::Class)
    }
}

/// How an entry point rejected an input.
#[derive(Debug)]
struct Rejection {
    /// What the error says about its cause.
    cause: Cause,
    /// The error, for failure messages.
    message: String,
}

impl Rejection {
    /// The rejection `error` reports, whose cause is `cause`.
    fn new(cause: Cause, error: &impl Debug) -> Self {
        Rejection {
            cause,
            message: format!("{error:?}"),
        }
    }
}

/// One entry point's verdict on an input.
struct Outcome<T> {
    /// The entry point, for failure messages.
    entry: &'static str,
    /// The decoded value, or the rejection.
    result: Result<T, Rejection>,
}

/// The outcome of an entry point that reports a [`Decode`] error.
fn decoded<T>(entry: &'static str, result: Result<T, Decode>) -> Outcome<T> {
    Outcome {
        entry,
        result: result.map_err(|error| Rejection::new(Cause::named(class_of(&error)), &error)),
    }
}

/// Every entry point of `T` that reads the whole of `bytes`, the slice
/// decode first.
///
/// The chunked reader serves the bytes on `schedule`. Postcard and CBOR read
/// the bytes framed as one byte string. Postcard's error drops the decoder's
/// message, so a postcard rejection says only that the decoder raised it;
/// CBOR's keeps the message, and with it the defect class.
fn entry_points<T: Wire>(bytes: &[u8], schedule: &IoSchedule) -> Vec<Outcome<T>> {
    let mut outcomes = vec![
        decoded("the slice decode", T::decode_reader(bytes)),
        decoded(
            "the chunked reader decode",
            T::decode_reader(ScriptedReader::new(bytes, schedule, None)),
        ),
    ];
    if let Some(parsed) = T::parse_hex(&hex::encode(bytes)) {
        outcomes.push(Outcome {
            entry: "hexadecimal text",
            result: parsed.map_err(|error| {
                let class = match &error {
                    ParseValue::InvalidEncoding(error) => class_of(error),
                    _ => None,
                };
                Rejection::new(Cause::named(class), &error)
            }),
        });
    }
    let field = serde_bytes::Bytes::new(bytes);
    let frame = postcard::to_allocvec(field).expect("postcard frames any byte string");
    outcomes.push(Outcome {
        entry: "postcard",
        result: postcard::from_bytes(&frame).map_err(|error| {
            let cause = match error {
                postcard::Error::SerdeDeCustom => Cause::Decoder,
                _ => Cause::Unexplained,
            };
            Rejection::new(cause, &error)
        }),
    });
    let mut frame = Vec::new();
    ciborium::ser::into_writer(field, &mut frame).expect("CBOR frames any byte string");
    outcomes.push(Outcome {
        entry: "CBOR",
        result: ciborium::de::from_reader::<T, _>(&frame[..]).map_err(|error| {
            let message = format!("{error:?}");
            Rejection::new(Cause::named(class_in_message(&message)), &error)
        }),
    });
    outcomes
}

/// The defect class a borsh error names: the reader running out is
/// truncation, and the decoder's own errors carry their `Decode`.
fn borsh_class(error: &borsh::io::Error) -> Option<Class> {
    if error.kind() == borsh::io::ErrorKind::UnexpectedEof {
        return Some(Class::Truncated);
    }
    error
        .get_ref()
        .and_then(|inner| inner.downcast_ref::<Decode>())
        .and_then(class_of)
}

/// Requires an entry point to follow the specification's verdict: to accept
/// exactly when the verdict does, decoding the value it reads, and to reject
/// with an error that names a class the verdict permits.
///
/// Postcard alone is exempt from naming a class, because its error drops the
/// decoder's message; its rejection must still come from the decoder
/// ([`Cause::Decoder`]), not from the framing.
fn follows<T: Wire>(
    entry: &str,
    decoded: Result<&T, &Rejection>,
    verdict: &Verdict<T::Spec>,
) -> Result<(), TestCaseError> {
    match (verdict, decoded) {
        (Verdict::Accept { value, .. }, Ok(decoded)) => prop_assert_eq!(
            &decoded.read_back(),
            value,
            "{} decoded a value other than the one the specification reads",
            entry
        ),
        (Verdict::Accept { .. }, Err(rejection)) => {
            return Err(TestCaseError::fail(format!(
                "{entry} rejected ({rejection:?}) an encoding the specification accepts"
            )));
        }
        (Verdict::Reject(classes), Ok(decoded)) => {
            return Err(TestCaseError::fail(format!(
                "{entry} accepted {decoded:?}, but the specification rejects with {classes:?}"
            )));
        }
        (Verdict::Reject(classes), Err(rejection)) => match rejection.cause {
            Cause::Class(class) => prop_assert!(
                classes.contains(&class),
                "{} reported {:?}, but the specification permits only {:?}",
                entry,
                class,
                classes
            ),
            Cause::Decoder => {}
            Cause::Unexplained => {
                return Err(TestCaseError::fail(format!(
                    "{entry} rejected without naming a defect class: {}",
                    rejection.message
                )));
            }
        },
    }
    Ok(())
}

/// Reads one `T` from `stream` through borsh, and requires it to follow the
/// specification's verdict on the stream ([`follows`]), consuming exactly
/// the bytes an accepted value spans.
///
/// Returns borsh's result and the bytes it consumed.
fn check_borsh<T: Wire>(
    stream: &[u8],
) -> Result<(Result<T, borsh::io::Error>, usize), TestCaseError> {
    let mut unread = stream;
    let streamed = T::deserialize_reader(&mut unread);
    let consumed = stream.len() - unread.len();
    let verdict = T::verdict(stream, Ending::Stream);
    let borsh = streamed
        .as_ref()
        .map_err(|error| Rejection::new(Cause::named(borsh_class(error)), error));
    follows("borsh", borsh.as_ref().copied(), &verdict)?;
    if let (Verdict::Accept { len, .. }, Ok(_)) = (&verdict, &streamed) {
        prop_assert_eq!(
            consumed,
            *len,
            "borsh consumed a byte count other than the specified value's length"
        );
    }
    Ok((streamed, consumed))
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

/// Checks every entry point of `T` on one near-valid input against the
/// specification's verdict and the slice decode, `T::decode(&input.bytes[..])`.
///
/// 1. The specification agrees with what construction settles: it accepts a
///    canonical encoding, rejects a defective one, and rejects a span with
///    both a structural defect and out-of-order endpoints with structural
///    classes alone, the precedence `Span::decode` documents. This
///    calibrates the specification on the inputs whose verdict construction
///    knows.
/// 2. Every whole-input entry point ([`entry_points`]) follows the
///    specification's verdict on the bytes ([`follows`]), on every input,
///    those construction leaves open included.
/// 3. An accepted encoding is canonical: it equals the independent encoder's
///    spelling of the decoded value ([`Wire::spelling`]).
/// 4. Every other whole-input entry point accepts exactly when the slice
///    decode does, with a value equal under `T`'s own equality.
/// 5. Where the specification frames a span's lower endpoint, the
///    human-readable record of the two endpoints decodes exactly when the
///    slice decode accepts, to an equal span.
/// 6. Borsh reads one value from the bytes followed by `junk`, and follows
///    the specification's verdict on that stream, consuming exactly the
///    bytes an accepted value spans. When the slice decode accepts, borsh
///    decodes an equal value and leaves the junk unread. When borsh accepts,
///    the bytes it consumed are a canonical encoding of its value by the
///    slice decode. Where construction puts a span's structural defect
///    inside the encoding, borsh on the bytes alone reports a structural
///    error.
fn check_agreement<T: Wire>(
    input: &NearValid,
    schedule: &IoSchedule,
    junk: &[u8],
) -> Result<(), TestCaseError> {
    let bytes = &input.bytes[..];
    let verdict = T::verdict(bytes, Ending::Whole);
    match input.expect {
        Expect::Canonical => prop_assert!(
            verdict.accepts(),
            "the specification rejects a canonical encoding: {:?}",
            verdict
        ),
        Expect::Defective => prop_assert!(
            !verdict.accepts(),
            "the specification accepts a defective encoding"
        ),
        Expect::StructuralOverOrdering { .. } => prop_assert!(
            matches!(&verdict, Verdict::Reject(classes)
                if classes.iter().all(|class| class.is_structural())),
            "the specification permits more than structural classes for a span with a \
             structural defect and out-of-order endpoints: {:?}",
            verdict
        ),
        Expect::Open => {}
    }

    let outcomes = entry_points::<T>(bytes, schedule);
    for outcome in &outcomes {
        follows(outcome.entry, outcome.result.as_ref(), &verdict)?;
    }

    let accepted = outcomes[0].result.as_ref().ok();
    if let Some(value) = accepted {
        let spelling = value.spelling();
        prop_assert_eq!(
            spelling.as_deref(),
            Some(bytes),
            "an accepted encoding must be the canonical spelling of its value"
        );
    }
    for outcome in &outcomes[1..] {
        agree(outcome.entry, accepted, outcome.result.as_ref().ok())?;
    }

    if let Some(record) = T::record(bytes) {
        agree("the human-readable record", accepted, record.as_ref().ok())?;
    }

    let stream = [bytes, junk].concat();
    let (streamed, consumed) = check_borsh::<T>(&stream)?;
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
            streamed
                .as_ref()
                .err()
                .and_then(borsh_class)
                .is_some_and(Class::is_structural),
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
    drop(T::record(bytes));
    let _ = T::deserialize_reader(&mut &bytes[..]);
}

/// The longest input length every byte string of which the exhaustive
/// short-input tests decode.
const EXHAUSTED_LEN: usize = 2;

/// The longest input length the sampled short-input properties draw.
const SAMPLED_LEN: usize = 8;

/// Checks `T`'s slice decode, borsh on the bytes alone, and the
/// human-readable record against the specification on one short input.
///
/// The slice decode and borsh must each follow the specification's verdict
/// ([`follows`], [`check_borsh`]); the record, where the type has one and the
/// specification frames it, must agree with the slice decode.
///
/// Returns whether the specification accepts `bytes`.
fn check_short_input<T: Wire>(bytes: &[u8]) -> Result<bool, TestCaseError> {
    let verdict = T::verdict(bytes, Ending::Whole);
    let slice = decoded("the slice decode", T::decode_reader(bytes));
    follows(slice.entry, slice.result.as_ref(), &verdict)?;
    if let Some(record) = T::record(bytes) {
        agree(
            "the human-readable record",
            slice.result.as_ref().ok(),
            record.as_ref().ok(),
        )?;
    }
    let (_streamed, _consumed) = check_borsh::<T>(bytes)?;
    Ok(verdict.accepts())
}

/// Every byte string of up to [`EXHAUSTED_LEN`] bytes, shortest first.
fn exhausted_short_inputs() -> impl Iterator<Item = Vec<u8>> {
    (0..=EXHAUSTED_LEN).flat_map(|len| {
        (0..1usize << (8 * len)).map(move |n| n.to_be_bytes()[size_of::<usize>() - len..].to_vec())
    })
}

/// Runs [`check_short_input`] for `T` on every byte string of up to
/// [`EXHAUSTED_LEN`] bytes, and panics with the first input that fails.
///
/// Every type has a canonical encoding of at most [`EXHAUSTED_LEN`] bytes
/// (an empty version, the seed party, a zero rank, and their composites), so
/// the inputs must include both a verdict that accepts and one that rejects;
/// otherwise the check judged only one side.
fn check_exhausted_short_inputs<T: Wire>() {
    let (mut accepted, mut rejected) = (0usize, 0usize);
    for bytes in exhausted_short_inputs() {
        match check_short_input::<T>(&bytes) {
            Ok(true) => accepted += 1,
            Ok(false) => rejected += 1,
            Err(failure) => panic!("{failure} (input {bytes:02x?})"),
        }
    }
    assert!(
        accepted > 0 && rejected > 0,
        "the short inputs must meet both verdicts: {accepted} accepted, {rejected} rejected"
    );
}

/// A byte string longer than [`EXHAUSTED_LEN`] and at most [`SAMPLED_LEN`]
/// bytes, every length equally likely and every byte uniform.
///
/// The exhaustive tests cover every shorter input. A failing case shrinks by
/// deleting bytes, down to `EXHAUSTED_LEN + 1` of them, and then by lowering
/// each remaining byte toward zero.
fn arb_sampled_short_input() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), EXHAUSTED_LEN + 1..=SAMPLED_LEN)
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
    /// reader, hexadecimal text where the type has it, postcard, CBOR, a
    /// span's human-readable record where the specification frames its
    /// endpoints, and borsh. Proptest reports a panic as a failing case, so
    /// the property needs no assertion of its own.
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

    /// `Version`'s decoders follow the specification's verdict and agree
    /// with one another on near-valid version encodings, as
    /// [`check_agreement`] states.
    #[test]
    fn version_decoders_agree_on_near_valid_encodings(
        input in arb_version_encoding(),
        schedule in arb_io_schedule(),
        junk in arb_junk(),
    ) {
        check_agreement::<Version>(&input, &schedule, &junk)?;
    }

    /// `Party`'s decoders follow the specification's verdict and agree with
    /// one another on near-valid party encodings, as [`check_agreement`]
    /// states.
    #[test]
    fn party_decoders_agree_on_near_valid_encodings(
        input in arb_party_encoding(),
        schedule in arb_io_schedule(),
        junk in arb_junk(),
    ) {
        check_agreement::<Party>(&input, &schedule, &junk)?;
    }

    /// `Rank`'s decoders follow the specification's verdict and agree with
    /// one another on near-valid rank encodings, as [`check_agreement`]
    /// states.
    #[test]
    fn rank_decoders_agree_on_near_valid_encodings(
        input in arb_rank_encoding(),
        schedule in arb_io_schedule(),
        junk in arb_junk(),
    ) {
        check_agreement::<Rank>(&input, &schedule, &junk)?;
    }

    /// `Clock`'s decoders follow the specification's verdict and agree with
    /// one another on near-valid clock encodings, as [`check_agreement`]
    /// states.
    #[test]
    fn clock_decoders_agree_on_near_valid_encodings(
        input in arb_clock_encoding(),
        schedule in arb_io_schedule(),
        junk in arb_junk(),
    ) {
        check_agreement::<Clock>(&input, &schedule, &junk)?;
    }

    /// `Span`'s decoders follow the specification's verdict and agree with
    /// one another and with the human-readable record on near-valid span
    /// encodings, as [`check_agreement`] states.
    ///
    /// The verdict carries the precedence of structural errors over endpoint
    /// ordering.
    #[test]
    fn span_decoders_agree_on_near_valid_encodings(
        input in arb_span_encoding(),
        schedule in arb_io_schedule(),
        junk in arb_junk(),
    ) {
        check_agreement::<Span<'static>>(&input, &schedule, &junk)?;
    }

    /// `Ranked`'s decoders follow the specification's verdict and agree
    /// with one another on near-valid ranked-key encodings, as
    /// [`check_agreement`] states.
    ///
    /// The verdict requires a key's rank to be its version's own.
    #[test]
    fn ranked_decoders_agree_on_near_valid_encodings(
        input in arb_ranked_encoding(),
        schedule in arb_io_schedule(),
        junk in arb_junk(),
    ) {
        check_agreement::<Ranked<'static>>(&input, &schedule, &junk)?;
    }
}

/// `Version`'s slice decode, borsh, and the specification agree on every
/// input of up to [`EXHAUSTED_LEN`] bytes, as [`check_short_input`] states.
///
/// The near-valid strategies draw arbitrary bytes for versions, parties, and
/// ranks one input in thirteen, and never for composites; these tests
/// exhaust the shortest lengths for every type, and the sampled short-input
/// properties draw the next few.
#[test]
fn version_decoders_follow_the_specification_on_exhausted_short_inputs() {
    check_exhausted_short_inputs::<Version>();
}

/// `Party`'s slice decode, borsh, and the specification agree on every input
/// of up to [`EXHAUSTED_LEN`] bytes, as [`check_short_input`] states.
#[test]
fn party_decoders_follow_the_specification_on_exhausted_short_inputs() {
    check_exhausted_short_inputs::<Party>();
}

/// `Rank`'s slice decode, borsh, and the specification agree on every input
/// of up to [`EXHAUSTED_LEN`] bytes, as [`check_short_input`] states.
#[test]
fn rank_decoders_follow_the_specification_on_exhausted_short_inputs() {
    check_exhausted_short_inputs::<Rank>();
}

/// `Clock`'s slice decode, borsh, and the specification agree on every input
/// of up to [`EXHAUSTED_LEN`] bytes, as [`check_short_input`] states.
#[test]
fn clock_decoders_follow_the_specification_on_exhausted_short_inputs() {
    check_exhausted_short_inputs::<Clock>();
}

/// `Span`'s slice decode, borsh, human-readable record, and the
/// specification agree on every input of up to [`EXHAUSTED_LEN`] bytes, as
/// [`check_short_input`] states.
#[test]
fn span_decoders_follow_the_specification_on_exhausted_short_inputs() {
    check_exhausted_short_inputs::<Span<'static>>();
}

/// `Ranked`'s slice decode, borsh, and the specification agree on every
/// input of up to [`EXHAUSTED_LEN`] bytes, as [`check_short_input`] states.
#[test]
fn ranked_decoders_follow_the_specification_on_exhausted_short_inputs() {
    check_exhausted_short_inputs::<Ranked<'static>>();
}

proptest! {
    /// `Version`'s slice decode, borsh, and the specification agree on
    /// arbitrary bytes longer than [`EXHAUSTED_LEN`] and at most
    /// [`SAMPLED_LEN`], as [`check_short_input`] states.
    #[test]
    fn version_decoders_follow_the_specification_on_sampled_short_inputs(
        bytes in arb_sampled_short_input(),
    ) {
        check_short_input::<Version>(&bytes)?;
    }

    /// `Party`'s slice decode, borsh, and the specification agree on
    /// arbitrary bytes longer than [`EXHAUSTED_LEN`] and at most
    /// [`SAMPLED_LEN`], as [`check_short_input`] states.
    #[test]
    fn party_decoders_follow_the_specification_on_sampled_short_inputs(
        bytes in arb_sampled_short_input(),
    ) {
        check_short_input::<Party>(&bytes)?;
    }

    /// `Rank`'s slice decode, borsh, and the specification agree on
    /// arbitrary bytes longer than [`EXHAUSTED_LEN`] and at most
    /// [`SAMPLED_LEN`], as [`check_short_input`] states.
    #[test]
    fn rank_decoders_follow_the_specification_on_sampled_short_inputs(
        bytes in arb_sampled_short_input(),
    ) {
        check_short_input::<Rank>(&bytes)?;
    }

    /// `Clock`'s slice decode, borsh, and the specification agree on
    /// arbitrary bytes longer than [`EXHAUSTED_LEN`] and at most
    /// [`SAMPLED_LEN`], as [`check_short_input`] states.
    #[test]
    fn clock_decoders_follow_the_specification_on_sampled_short_inputs(
        bytes in arb_sampled_short_input(),
    ) {
        check_short_input::<Clock>(&bytes)?;
    }

    /// `Span`'s slice decode, borsh, human-readable record, and the
    /// specification agree on arbitrary bytes longer than [`EXHAUSTED_LEN`]
    /// and at most [`SAMPLED_LEN`], as [`check_short_input`] states.
    #[test]
    fn span_decoders_follow_the_specification_on_sampled_short_inputs(
        bytes in arb_sampled_short_input(),
    ) {
        check_short_input::<Span<'static>>(&bytes)?;
    }

    /// `Ranked`'s slice decode, borsh, and the specification agree on
    /// arbitrary bytes longer than [`EXHAUSTED_LEN`] and at most
    /// [`SAMPLED_LEN`], as [`check_short_input`] states.
    #[test]
    fn ranked_decoders_follow_the_specification_on_sampled_short_inputs(
        bytes in arb_sampled_short_input(),
    ) {
        check_short_input::<Ranked<'static>>(&bytes)?;
    }
}
