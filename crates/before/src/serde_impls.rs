//! `serde` support (feature-gated).
//!
//! Binary formats use each type's canonical byte encoding and strict decoder.
//! Human-readable formats use the public text forms for scalar values and
//! named records for composite values.
//!
//! A composite also deserializes from its fields in order, as a sequence or
//! keyed by position, because headerless csv and `rmp-serde`'s tuple mode
//! write structs that way and a visitor cannot tell which format called it.
//! Named keys may come in any order and as bytes. A record that omits,
//! repeats, or adds a field is rejected. The derived `Deserialize` of the
//! records below does all of this; a hand-written one must too.
//!
//! Deserializing a [`Party`] or [`Clock`] duplicates identity exactly as
//! [`Party::decode`]/[`Clock::decode`] do — nothing ties serialized bytes to
//! their source, so their linearity notes apply verbatim at this entry point
//! ([Safety rules](crate#safety-rules)).

use core::fmt;

use serde::de::{Error as _, SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::count::CanonicalLimbs;
use crate::error::Decode;
use crate::span::Span;
use crate::{Clock, Count, Party, Rank, Ranked, Version};

/// Human-readable fields of a clock.
#[derive(Serialize)]
#[serde(rename = "Clock")]
struct ClockRef<'a> {
    /// Identity share.
    party: &'a Party,
    /// Causal history.
    version: &'a Version,
}

/// Owned human-readable fields of a clock.
#[derive(Deserialize)]
#[serde(rename = "Clock", deny_unknown_fields)]
struct ClockOwned {
    /// Identity share.
    party: Party,
    /// Causal history.
    version: Version,
}

/// Human-readable field of a ranked view.
#[derive(Serialize)]
#[serde(rename = "Ranked")]
struct RankedRef<'a> {
    /// The viewed version.
    version: &'a Version,
}

/// Owned human-readable field of a ranked view.
#[derive(Deserialize)]
#[serde(rename = "Ranked", deny_unknown_fields)]
struct RankedOwned {
    /// The viewed version.
    version: Version,
}

/// Human-readable fields of a span.
#[derive(Serialize)]
#[serde(rename = "Span")]
struct SpanRef<'a> {
    /// Lower endpoint.
    lo: &'a Version,
    /// Upper endpoint.
    hi: &'a Version,
}

/// Owned human-readable fields of a span.
#[derive(Deserialize)]
#[serde(rename = "Span", deny_unknown_fields)]
struct SpanOwned {
    /// Lower endpoint.
    lo: Version,
    /// Upper endpoint.
    hi: Version,
}

/// Reads either a typed byte buffer or a byte sequence and passes ownership to
/// the strict decoder.
fn deserialize_bytes<'de, D, T>(
    d: D,
    decode: fn(bytes::Bytes) -> Result<T, Decode>,
) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
{
    let bytes = serde_bytes::ByteBuf::deserialize(d)?;
    decode(bytes.into_vec().into()).map_err(D::Error::custom)
}

/// Human-readable formats use canonical hexadecimal; binary formats use the
/// canonical bytes.
impl Serialize for Party {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if s.is_human_readable() {
            s.collect_str(self)
        } else {
            s.serialize_bytes(self.as_bytes())
        }
    }
}

/// Parses hexadecimal text or strictly decodes binary bytes.
impl<'de> Deserialize<'de> for Party {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        if d.is_human_readable() {
            String::deserialize(d)?.parse().map_err(D::Error::custom)
        } else {
            deserialize_bytes(d, Party::decode_bytes)
        }
    }
}

/// Human-readable formats use canonical hexadecimal; binary formats use the
/// canonical bytes.
impl Serialize for Version {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if s.is_human_readable() {
            s.collect_str(self)
        } else {
            s.serialize_bytes(self.as_bytes())
        }
    }
}

/// Parses hexadecimal text or strictly decodes binary bytes.
impl<'de> Deserialize<'de> for Version {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        if d.is_human_readable() {
            String::deserialize(d)?.parse().map_err(D::Error::custom)
        } else {
            deserialize_bytes(d, Version::decode_bytes)
        }
    }
}

/// Human-readable formats use a party/version record; binary formats preserve
/// the canonical clock bytes.
impl Serialize for Clock {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if s.is_human_readable() {
            ClockRef {
                party: self.party(),
                version: self.version(),
            }
            .serialize(s)
        } else {
            s.serialize_bytes(&self.encode())
        }
    }
}

/// Reads a human-readable party/version record or strictly decodes binary
/// bytes.
impl<'de> Deserialize<'de> for Clock {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        if d.is_human_readable() {
            let ClockOwned { party, version } = ClockOwned::deserialize(d)?;
            Ok(Clock::from_parts(party, version))
        } else {
            deserialize_bytes(d, Clock::decode_bytes)
        }
    }
}

/// Human-readable formats use [`Rank`]'s text form; binary formats use its
/// canonical encoded bytes.
impl Serialize for Rank {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if s.is_human_readable() {
            s.collect_str(self)
        } else {
            s.serialize_bytes(&self.encode())
        }
    }
}

/// Parses human-readable text or strictly decodes binary bytes, according to
/// the format.
impl<'de> Deserialize<'de> for Rank {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        if d.is_human_readable() {
            <String>::deserialize(d)?.parse().map_err(D::Error::custom)
        } else {
            let bytes = serde_bytes::ByteBuf::deserialize(d)?;
            Rank::decode_bytes(&bytes).map_err(D::Error::custom)
        }
    }
}

/// Human-readable formats use a version record. Binary formats preserve the
/// canonical composite key of [`Ranked::encode`].
impl Serialize for Ranked<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if s.is_human_readable() {
            RankedRef {
                version: self.version(),
            }
            .serialize(s)
        } else {
            s.serialize_bytes(&self.encode())
        }
    }
}

/// Reads a human-readable version record. Binary formats deserialize through
/// [`Ranked::decode`], including its rank/version consistency check.
impl<'de> Deserialize<'de> for Ranked<'static> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        if d.is_human_readable() {
            let RankedOwned { version } = RankedOwned::deserialize(d)?;
            Ok(Ranked::from(version))
        } else {
            deserialize_bytes(d, Ranked::decode_bytes)
        }
    }
}

/// Human-readable formats use a lower/upper endpoint record. Binary formats
/// preserve the canonical composite of [`Span::encode`].
impl Serialize for Span<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if s.is_human_readable() {
            SpanRef {
                lo: self.lo(),
                hi: self.hi(),
            }
            .serialize(s)
        } else {
            s.serialize_bytes(&self.encode())
        }
    }
}

/// Deserializes through [`Span::new`] in human-readable formats and
/// [`Span::decode`] in binary ones.
///
/// Both reject crossed and concurrent pairs, so a deserialized span is valid.
impl<'de> Deserialize<'de> for Span<'static> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        if d.is_human_readable() {
            let SpanOwned { lo, hi } = SpanOwned::deserialize(d)?;
            Span::new(lo, hi).map_err(D::Error::custom)
        } else {
            deserialize_bytes(d, Span::decode_bytes)
        }
    }
}

/// Human-readable formats use decimal text. Binary formats encode the count as
/// least-significant-first `u64` limbs; zero has no limbs.
impl Serialize for Count {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if s.is_human_readable() {
            s.collect_str(self)
        } else {
            let mut sequence = s.serialize_seq(Some(self.limbs().len()))?;
            for limb in self.limbs() {
                sequence.serialize_element(&limb)?;
            }
            sequence.end()
        }
    }
}

/// Parses decimal text or decodes canonical least-significant-first `u64`
/// limbs. A nonempty sequence ending in zero is rejected.
impl<'de> Deserialize<'de> for Count {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        if d.is_human_readable() {
            return String::deserialize(d)?.parse().map_err(D::Error::custom);
        }

        d.deserialize_seq(CountVisitor)
    }
}

/// Reads Count's binary limb sequence directly into its integer storage.
struct CountVisitor;

impl<'de> Visitor<'de> for CountVisitor {
    type Value = Count;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("canonical least-significant-first u64 limbs")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Count, A::Error> {
        let mut limbs = CanonicalLimbs::new();
        while let Some(limb) = sequence.next_element()? {
            limbs.push(limb);
        }
        limbs
            .finish()
            .ok_or_else(|| A::Error::custom("count has a trailing zero limb"))
    }
}

#[cfg(test)]
mod tests;
