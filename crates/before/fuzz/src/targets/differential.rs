//! Agreement between fused decoders and equivalent public compositions.
//!
//! Canonical round trips cannot detect two decoders that reject the same bytes
//! for different reasons. This target compares acceptance, decoded values,
//! bytes consumed, and rejection classes across raw, borsh, and postcard
//! paths. It also checks the fused `Span` and `Ranked` decoders against their
//! component-wise spellings.

use std::io::{Error, ErrorKind};

use before::{error::Decode, Clock, Count, Party, Rank, Ranked, Span, Version};
use borsh::{BorshDeserialize, BorshSerialize};

use super::for_each_wire_type;

/// A raw decoder's rejection class, without an I/O payload.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Rejection {
    /// The input ended before the value did.
    Truncated,
    /// Bits remained after a complete value or its padding was malformed.
    TrailingBits,
    /// The bytes describe no canonical value.
    NotCanonical,
}

/// Where a composed two-component decoder rejected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Stage {
    /// The first component rejected.
    First,
    /// The second component rejected.
    Second,
    /// Both components decoded, but their relationship was invalid.
    Pair,
}

/// Whether a type performs a relationship check after reading its components.
#[derive(Clone, Copy)]
enum Composite {
    /// Decoding ends when the encoded structure ends.
    Scalar,
    /// A ranked key verifies that its rank matches its version.
    Ranked,
    /// A span verifies that its endpoints are ordered.
    Span,
}

impl Composite {
    /// Whether the decoder can reject a complete prefix after reading its trees.
    fn checks_components(self) -> bool {
        matches!(self, Self::Ranked | Self::Span)
    }
}

/// Run every differential decoder on one byte string.
pub fn run(data: &[u8]) {
    span(data);
    ranked(data);

    /// Check both transports for one entry in the shared wire-type list.
    macro_rules! check {
        ($type:ty, $name:literal, $kind:ident) => {
            borsh_vs_raw::<$type>(
                data,
                $name,
                |bytes| <$type>::decode(bytes),
                <$type>::encode,
                Composite::$kind,
            );
            postcard_vs_raw::<$type>(data, |bytes| <$type>::decode(bytes), <$type>::encode);
        };
    }
    for_each_wire_type!(check);
    count_borsh(data);
    count_postcard(data);
}

/// Classify a raw rejection.
fn rejection(error: &Decode) -> Rejection {
    match error {
        Decode::Truncated => Rejection::Truncated,
        Decode::TrailingBits => Rejection::TrailingBits,
        Decode::NotCanonical => Rejection::NotCanonical,
        Decode::Io(source) => unreachable!("slice reads cannot fail: {source}"),
        _ => unreachable!("a new Decode variant needs a fuzz classification"),
    }
}

/// Classify a borsh rejection produced by a raw wire decoder.
fn borsh_rejection(error: &Error) -> Rejection {
    if error.kind() == ErrorKind::UnexpectedEof {
        return Rejection::Truncated;
    }
    error
        .get_ref()
        .and_then(|source| source.downcast_ref::<Decode>())
        .map(rejection)
        .unwrap_or_else(|| panic!("a borsh wire rejection must carry Decode: {error}"))
}

/// Decode a span from its two public components.
fn composed_span(data: &[u8]) -> Result<(Version, Version), (Stage, Rejection)> {
    let mut reader = data;
    let lo = Version::deserialize_reader(&mut reader)
        .map_err(|error| (Stage::First, borsh_rejection(&error)))?;
    let hi = Version::decode(reader).map_err(|error| (Stage::Second, rejection(&error)))?;
    if Span::new(&lo, &hi).is_ok() {
        Ok((lo, hi))
    } else {
        Err((Stage::Pair, Rejection::NotCanonical))
    }
}

/// Check the fused span decoder against component decoding and validation.
fn span(data: &[u8]) {
    let fused = Span::decode(data);
    match composed_span(data) {
        Ok((lo, hi)) => {
            let decoded = fused.expect("component decoding accepts a valid span");
            assert_eq!(decoded.lo(), &lo, "span lower bounds disagree");
            assert_eq!(decoded.hi(), &hi, "span upper bounds disagree");
            assert_eq!(
                decoded.encode(),
                data,
                "accepted span bytes are not canonical"
            );
        }
        Err((stage, composed)) => {
            let fused = rejection(&fused.expect_err("component decoding rejects the span"));
            let height_dip_is_subsumed = matches!(
                (stage, composed, fused),
                (
                    Stage::Second,
                    Rejection::NotCanonical,
                    Rejection::Truncated | Rejection::TrailingBits
                )
            );
            assert!(
                composed == fused || height_dip_is_subsumed,
                "span rejection differs: composed {composed:?} at {stage:?}, fused {fused:?}"
            );
        }
    }
}

/// Decode a ranked key from its rank and version components.
fn composed_ranked(data: &[u8]) -> Result<Ranked<'static>, (Stage, Rejection)> {
    let mut reader = data;
    let rank = Rank::deserialize_reader(&mut reader)
        .map_err(|error| (Stage::First, borsh_rejection(&error)))?;
    let version = Version::decode(reader).map_err(|error| (Stage::Second, rejection(&error)))?;
    if version.rank() == rank {
        Ok(Ranked::from(version))
    } else {
        Err((Stage::Pair, Rejection::NotCanonical))
    }
}

/// Check the fused ranked-key decoder against component decoding and validation.
fn ranked(data: &[u8]) {
    let fused = Ranked::decode(data);
    match composed_ranked(data) {
        Ok(composed) => {
            let decoded = fused.expect("component decoding accepts a valid ranked key");
            assert_eq!(decoded, composed, "ranked-key values disagree");
            assert_eq!(
                decoded.encode(),
                data,
                "accepted ranked-key bytes are not canonical"
            );
        }
        Err((stage, composed)) => {
            let fused = rejection(&fused.expect_err("component decoding rejects the ranked key"));
            assert_eq!(
                fused, composed,
                "ranked-key rejection differs: composed {composed:?} at {stage:?}, fused {fused:?}"
            );
        }
    }
}

/// Check a self-delimiting borsh decoder against the whole-slice raw decoder.
fn borsh_vs_raw<T>(
    data: &[u8],
    name: &str,
    decode: impl Fn(&[u8]) -> Result<T, Decode>,
    encode: fn(&T) -> Vec<u8>,
    composite: Composite,
) where
    T: BorshDeserialize + BorshSerialize + core::fmt::Debug,
{
    let mut reader = data;
    match T::deserialize_reader(&mut reader) {
        Ok(value) => {
            let consumed = &data[..data.len() - reader.len()];
            let raw = decode(consumed).expect("borsh-accepted bytes must raw-decode");
            assert_eq!(encode(&raw), encode(&value), "borsh and raw values differ");
            assert_eq!(
                encode(&value),
                consumed,
                "borsh accepted non-canonical bytes"
            );
            assert_eq!(
                borsh::to_vec(&value).expect("serialization to Vec cannot fail"),
                consumed,
                "borsh re-encoding changed the consumed bytes"
            );
            if !reader.is_empty() {
                assert_eq!(
                    rejection(
                        &decode(data).expect_err("whole-slice decoding must reject a remainder")
                    ),
                    Rejection::TrailingBits,
                    "bytes after a complete value must be trailing input"
                );
            }
        }
        Err(error) => {
            let raw = rejection(&decode(data).expect_err("borsh rejection must raw-reject"));
            let borsh = borsh_rejection(&error);
            let consumed = &data[..data.len() - reader.len()];
            let prefix_rejection = borsh == Rejection::NotCanonical
                && raw == Rejection::TrailingBits
                && !reader.is_empty()
                && composite.checks_components()
                && decode(consumed)
                    .is_err_and(|error| rejection(&error) == Rejection::NotCanonical);
            assert!(
                borsh == raw || prefix_rejection,
                "{name} borsh rejection {borsh:?} differs from raw rejection {raw:?}"
            );
        }
    }
}

/// Check postcard's byte-carrying serde decoder against framing plus raw decode.
fn postcard_vs_raw<T>(
    data: &[u8],
    decode: impl Fn(&[u8]) -> Result<T, Decode>,
    encode: fn(&T) -> Vec<u8>,
) where
    T: serde::de::DeserializeOwned + core::fmt::Debug,
{
    let fused = postcard::take_from_bytes::<T>(data);
    match postcard::take_from_bytes::<Vec<u8>>(data) {
        Ok((payload, rest)) => match decode(&payload) {
            Ok(composed) => {
                let (decoded, fused_rest) = fused.expect("a canonical payload must deserialize");
                assert_eq!(
                    encode(&decoded),
                    encode(&composed),
                    "postcard values differ"
                );
                assert_eq!(fused_rest, rest, "postcard consumed a different frame");
                assert_eq!(
                    encode(&composed),
                    payload,
                    "postcard accepted non-canonical bytes"
                );
            }
            Err(_) => assert!(
                matches!(fused, Err(postcard::Error::SerdeDeCustom)),
                "a raw payload rejection must be postcard's custom serde error"
            ),
        },
        Err(framing) => {
            let fused = fused.expect_err("invalid byte framing must reject");
            assert_eq!(
                core::mem::discriminant(&fused),
                core::mem::discriminant(&framing),
                "postcard framing rejections differ"
            );
        }
    }
}

/// Check Count's borsh decoder against its public limb sequence.
fn count_borsh(data: &[u8]) {
    let mut fused_reader = data;
    let fused = Count::deserialize_reader(&mut fused_reader);
    let mut composed_reader = data;
    match Vec::<u64>::deserialize_reader(&mut composed_reader) {
        Ok(limbs) if limbs.last() != Some(&0) => {
            let count = fused.expect("a canonical limb sequence must decode as Count");
            assert_eq!(
                fused_reader, composed_reader,
                "Count consumed a different prefix"
            );
            assert_eq!(
                count.limbs().collect::<Vec<_>>(),
                limbs,
                "Count changed its limbs"
            );
            assert_eq!(
                borsh::to_vec(&count).expect("serialization to Vec cannot fail"),
                &data[..data.len() - fused_reader.len()],
                "Count borsh re-encoding changed the consumed bytes"
            );
        }
        Ok(_) => assert!(
            fused.is_err(),
            "a limb sequence ending in zero is not canonical"
        ),
        Err(composed) => {
            let fused = fused.expect_err("invalid limb framing must reject");
            assert_eq!(
                fused.kind(),
                composed.kind(),
                "Count borsh framing rejections differ"
            );
        }
    }
}

/// Check Count's binary serde decoder against its public limb sequence.
fn count_postcard(data: &[u8]) {
    let fused = postcard::take_from_bytes::<Count>(data);
    match postcard::take_from_bytes::<Vec<u64>>(data) {
        Ok((limbs, rest)) if limbs.last() != Some(&0) => {
            let (count, fused_rest) =
                fused.expect("a canonical limb sequence must decode as Count");
            assert_eq!(
                fused_rest, rest,
                "Count consumed a different postcard frame"
            );
            assert_eq!(
                count.limbs().collect::<Vec<_>>(),
                limbs,
                "Count changed its limbs"
            );
        }
        Ok(_) => assert!(
            matches!(fused, Err(postcard::Error::SerdeDeCustom)),
            "a trailing zero limb must be postcard's custom serde error"
        ),
        Err(composed) => {
            let fused = fused.expect_err("invalid limb framing must reject");
            assert_eq!(
                core::mem::discriminant(&fused),
                core::mem::discriminant(&composed),
                "Count postcard framing rejections differ"
            );
        }
    }
}
