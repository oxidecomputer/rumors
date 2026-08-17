//! Differential decode fuzzing across the serialization seams.
//!
//! Every fused or transport decode in `before` has a composed counterpart
//! spelled from smaller public pieces. Feed both the same arbitrary bytes
//! and hold them to agreement on three axes:
//!
//!   1. **Accept**: the two paths accept exactly the same inputs.
//!   2. **Value and re-encode**: an accept yields the same value on both
//!      paths, and the value re-encodes to the bytes consumed.
//!   3. **Rejection genre**: a rejected input rejects with the same
//!      `Decode` variant through both paths, up to the documented
//!      divergences spelled at each arm.
//!
//! The genre axis is what plain round-trip fuzzing is blind to: two paths
//! can both reject an input yet disagree on *which* error, silently
//! breaking the documented precedence (structural genres outrank the pair
//! verdict — `Span::decode`'s Errors contract).
//!
//! The arms:
//!
//!   - `Span::decode` (the fused parse-and-validate) vs the composed
//!     spelling: carve the lower bound as a self-delimiting prefix, decode
//!     the upper bound from the rest, then validate the pair with
//!     `Span::new`.
//!   - `Ranked::decode` (the fused key decode) vs the composed spelling:
//!     carve the rank stream, decode the version from the rest, then
//!     cross-check the rank against the version's own fold.
//!   - borsh `deserialize_reader` (the self-delimiting prefix read of the
//!     same wire form) vs the whole-slice raw `decode`, for all six types.
//!   - postcard, the byte-carrying serde format of record, vs the composed
//!     spelling (postcard `Vec<u8>` framing, then the raw decode), for all
//!     six types.
//!
//! Every accepted value additionally re-encodes byte-identically through
//! both the raw encode and the transport serializer, and the whole body
//! runs under the harness heap cap (`before_fuzz::under_heap_cap`).

#![no_main]

use std::io::ErrorKind;

use borsh::{BorshDeserialize, BorshSerialize};
use libfuzzer_sys::fuzz_target;

use before::{error::Decode, Clock, Party, Rank, Ranked, Span, Version};

fuzz_target!(|data: &[u8]| {
    before_fuzz::under_heap_cap(|| run(data));
});

/// One input's body: run every differential arm on the same bytes.
fn run(data: &[u8]) {
    span_differential(data);
    ranked_differential(data);

    // `composite` is true exactly for the two types whose decode ends in a
    // prefix-scoped composite check (the ranked cross-check, the span pair
    // verdict); the scalar types get the exact-genre agreement.
    borsh_vs_raw(data, |b| Party::decode(b), Party::encode, false);
    borsh_vs_raw(data, |b| Version::decode(b), Version::encode, false);
    borsh_vs_raw(data, |b| Clock::decode(b), Clock::encode, false);
    borsh_vs_raw(data, |b| Rank::decode(b), Rank::encode, false);
    borsh_vs_raw(data, |b| Ranked::decode(b), Ranked::encode, true);
    borsh_vs_raw(data, |b| Span::decode(b), Span::encode, true);

    postcard_vs_composed(data, |b| Party::decode(b), Party::encode);
    postcard_vs_composed(data, |b| Version::decode(b), Version::encode);
    postcard_vs_composed(data, |b| Clock::decode(b), Clock::encode);
    postcard_vs_composed(data, |b| Rank::decode(b), Rank::encode);
    postcard_vs_composed(data, |b| Ranked::decode(b), Ranked::encode);
    postcard_vs_composed(data, |b| Span::decode(b), Span::encode);
}

/// A rejection genre: `Decode`'s variants with the payload stripped, so
/// genres compare across paths (slice reads never produce `Decode::Io`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Genre {
    Truncated,
    TrailingBits,
    NotCanonical,
}

/// The genre of a raw `Decode` rejection.
fn genre(error: &Decode) -> Genre {
    match error {
        Decode::Truncated => Genre::Truncated,
        Decode::TrailingBits => Genre::TrailingBits,
        Decode::NotCanonical => Genre::NotCanonical,
        Decode::Io(source) => unreachable!("slice reads never fail: {source}"),
    }
}

/// The genre of a borsh rejection over a slice reader.
///
/// The borsh impls map `Decode::Io` to the bare io error (over a slice,
/// only `UnexpectedEof` — the truncation genre) and wrap every other
/// `Decode` as the payload of an `InvalidData` error, recovered here by
/// downcast.
fn borsh_genre(error: &std::io::Error) -> Genre {
    if error.kind() == ErrorKind::UnexpectedEof {
        return Genre::Truncated;
    }
    let inner = error
        .get_ref()
        .and_then(|inner| inner.downcast_ref::<Decode>())
        .unwrap_or_else(|| {
            panic!("a non-EOF borsh decode error carries the Decode genre: {error}")
        });
    genre(inner)
}

/// Which stage of a composed two-component decode rejected: the genre
/// agreement rules differ per stage (see `span_differential`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    /// The first component's self-delimiting prefix parse.
    First,
    /// The second component's whole-remainder parse.
    Second,
    /// The composite validation over two well-formed components.
    Pair,
}

/// The composed counterpart of `Span::decode`.
///
/// Carve the lower bound as a self-delimiting prefix (the borsh reader
/// path, the public spelling of "parse one version and stop"), decode the
/// upper bound from the remainder (`Version::decode`'s whole-input
/// contract), then validate the pair with `Span::new`.
fn span_composed(data: &[u8]) -> Result<(Version, Version), (Stage, Genre)> {
    let mut reader = data;
    let lo = <Version as BorshDeserialize>::deserialize_reader(&mut reader)
        .map_err(|e| (Stage::First, borsh_genre(&e)))?;
    let hi = Version::decode(reader).map_err(|e| (Stage::Second, genre(&e)))?;
    match Span::new(&lo, &hi) {
        Ok(_) => Ok((lo, hi)),
        Err(before::error::Crossed) => Err((Stage::Pair, Genre::NotCanonical)),
    }
}

/// `Span::decode` against the composed spelling, on all three axes.
///
/// Genre agreement is exact except for the one documented divergence
/// (`Span::decode`'s Errors contract): the fused admission walk carries
/// no height accumulator — the dominance verdict subsumes it — so an
/// upper bound whose running height dips negative *and* which is
/// structurally defective later in the stream reports the structural
/// genre, where component-wise decoding reports the dip (`NotCanonical`)
/// it meets first. A `Second`-stage `NotCanonical` therefore admits a fused
/// `Truncated`/`TrailingBits`; every other stage-genre pair must match
/// exactly, and in particular a `Pair`-stage rejection (both components
/// well-formed, the pair crossed or concurrent) must be fused
/// `NotCanonical` — pronounced only after the padding check, so a
/// structural defect can never hide behind the pair verdict.
fn span_differential(data: &[u8]) {
    let fused = Span::decode(data);
    match span_composed(data) {
        Ok((lo, hi)) => {
            let span = fused.expect("composed span accepts: fused decode must accept");
            assert_eq!(span.lo(), &lo, "fused and composed lower bounds disagree");
            assert_eq!(span.hi(), &hi, "fused and composed upper bounds disagree");
            assert_eq!(span.encode(), data, "accepted span re-encodes to its input");
        }
        Err((stage, composed)) => {
            let error = fused.expect_err("composed span rejects: fused decode must reject");
            let fused = genre(&error);
            let agreed = match (stage, composed, fused) {
                (_, c, f) if c == f => true,
                // The documented height-dip subsumption divergence.
                (Stage::Second, Genre::NotCanonical, Genre::Truncated | Genre::TrailingBits) => {
                    true
                }
                _ => false,
            };
            assert!(
                agreed,
                "span reject-genre divergence: composed {composed:?} at {stage:?}, fused {fused:?}"
            );
        }
    }
}

/// The composed counterpart of `Ranked::decode`.
///
/// Carve the rank as a self-delimiting prefix (the borsh reader path),
/// decode the version from the remainder, then cross-check the rank
/// against the version's own fold — a mismatched pair is the canonical
/// spelling of no key.
fn ranked_composed(data: &[u8]) -> Result<Ranked<'static>, (Stage, Genre)> {
    let mut reader = data;
    let rank = <Rank as BorshDeserialize>::deserialize_reader(&mut reader)
        .map_err(|e| (Stage::First, borsh_genre(&e)))?;
    let version = Version::decode(reader).map_err(|e| (Stage::Second, genre(&e)))?;
    if version.rank() != rank {
        return Err((Stage::Pair, Genre::NotCanonical));
    }
    Ok(Ranked::from(version))
}

/// `Ranked::decode` against the composed spelling, on all three axes.
///
/// Genre agreement is exact at every stage: the fused decode is the same
/// carve-decode-crosscheck order, with no subsumption seam.
fn ranked_differential(data: &[u8]) {
    let fused = Ranked::decode(data);
    match ranked_composed(data) {
        Ok(key) => {
            let decoded = fused.expect("composed ranked key accepts: fused decode must accept");
            assert_eq!(decoded, key, "fused and composed ranked keys disagree");
            assert_eq!(
                decoded.encode(),
                data,
                "accepted ranked key re-encodes to its input"
            );
        }
        Err((stage, composed)) => {
            let error = fused.expect_err("composed ranked key rejects: fused decode must reject");
            let fused = genre(&error);
            assert_eq!(
                fused, composed,
                "ranked reject-genre divergence: composed {composed:?} at {stage:?}, fused {fused:?}"
            );
        }
    }
}

/// The borsh transport against the raw decode of the same wire form.
///
/// borsh reads one self-delimiting value from a prefix and leaves the
/// remainder to the next field; the raw decode consumes the whole slice.
/// The correspondence:
///
///   - borsh accepts a prefix ⟹ the raw decode accepts exactly those
///     bytes as the same value, the value re-encodes to them (raw and
///     borsh serializers alike), and — when a remainder exists — the raw
///     decode of the *whole* slice rejects it as `TrailingBits`.
///   - borsh rejects ⟹ the raw decode of the whole slice rejects, with
///     the genre mapped: `UnexpectedEof` is exactly raw `Truncated`; an
///     embedded `Decode` genre matches exactly, except on the `composite`
///     types, where the raw whole-slice parse may report `TrailingBits`
///     where borsh's prefix-scoped composite checks (the ranked
///     cross-check, the span pair verdict) reject first — borsh cannot
///     see bytes it has not been asked to read. The scalar types run no
///     check after the structural parse, so for them that genre pair is
///     impossible and any occurrence is a divergence.
fn borsh_vs_raw<T>(
    data: &[u8],
    decode: impl Fn(&[u8]) -> Result<T, Decode>,
    encode: fn(&T) -> Vec<u8>,
    composite: bool,
) where
    T: BorshDeserialize + BorshSerialize,
{
    let mut reader = data;
    match T::deserialize_reader(&mut reader) {
        Ok(value) => {
            let consumed = &data[..data.len() - reader.len()];
            let raw = decode(consumed)
                .expect("borsh accepts a prefix: the raw decode must accept those bytes");
            assert_eq!(
                encode(&raw),
                encode(&value),
                "borsh and raw decodes disagree on the accepted value"
            );
            assert_eq!(
                encode(&value),
                consumed,
                "accepted value re-encodes to the consumed prefix"
            );
            assert_eq!(
                borsh::to_vec(&value).expect("borsh serialization to a Vec is infallible"),
                consumed,
                "borsh re-serialization is not the consumed prefix"
            );
            if !reader.is_empty() {
                let whole = genre(&decode(data).err().expect(
                    "bytes remain past the borsh value: the whole-slice decode must reject",
                ));
                assert_eq!(
                    whole,
                    Genre::TrailingBits,
                    "input past a complete value is the trailing genre"
                );
            }
        }
        Err(error) => {
            let raw = genre(
                &decode(data)
                    .err()
                    .expect("borsh rejects: the whole-slice decode must reject"),
            );
            let borsh = borsh_genre(&error);
            let agreed = match (borsh, raw) {
                (b, r) if b == r => true,
                // A prefix-scoped composite check rejects where the
                // whole-slice parse still sees unconsumed input — possible
                // only on the types that run one.
                (Genre::NotCanonical, Genre::TrailingBits) => composite,
                _ => false,
            };
            assert!(
                agreed,
                "borsh reject-genre divergence: borsh {borsh:?}, raw {raw:?}"
            );
        }
    }
}

/// The postcard transport against its composed spelling.
///
/// The serde impls deserialize a byte payload from the format, then run
/// the strict raw decode over it. postcard is the byte-carrying format of
/// record (the committed serde tests pin its payload to `encode()`), so
/// the composed counterpart is public: take a `Vec<u8>` from the same
/// bytes, then raw-decode the payload. Agreement: the framing stage
/// rejects identically (same postcard error discriminant); a framed
/// payload the raw decode rejects must reject as postcard's custom serde
/// error; a framed payload the raw decode accepts must yield the same
/// value, the same remainder, and a payload that is exactly the value's
/// canonical encoding.
fn postcard_vs_composed<T>(
    data: &[u8],
    decode: impl Fn(&[u8]) -> Result<T, Decode>,
    encode: fn(&T) -> Vec<u8>,
) where
    T: serde::de::DeserializeOwned,
{
    let fused = postcard::take_from_bytes::<T>(data);
    match postcard::take_from_bytes::<Vec<u8>>(data) {
        Ok((payload, rest)) => match decode(&payload) {
            Ok(value) => {
                let (decoded, fused_rest) =
                    fused.expect("the framed payload raw-decodes: postcard must accept");
                assert_eq!(
                    encode(&decoded),
                    encode(&value),
                    "postcard and composed decodes disagree on the accepted value"
                );
                assert_eq!(fused_rest, rest, "postcard consumed a different frame");
                assert_eq!(
                    encode(&value),
                    payload,
                    "accepted payload is not the canonical encoding"
                );
            }
            Err(_) => {
                let error = fused
                    .err()
                    .expect("the framed payload fails the raw decode: postcard must reject");
                assert!(
                    matches!(error, postcard::Error::SerdeDeCustom),
                    "a payload-stage rejection surfaces as postcard's custom serde error: {error}"
                );
            }
        },
        Err(framing) => {
            let error = fused
                .err()
                .expect("no byte payload frames from the input: postcard must reject");
            assert_eq!(
                core::mem::discriminant(&error),
                core::mem::discriminant(&framing),
                "framing-stage rejections must agree: fused {error}, composed {framing}"
            );
        }
    }
}
