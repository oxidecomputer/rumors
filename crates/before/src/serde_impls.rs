//! `serde` support (feature-gated).
//!
//! Binary formats use each type's canonical byte encoding and strict decoder.
//! Human-readable formats render [`Rank`] as its canonical binary text and
//! parse that same form; the other types continue to use bytes.
//!
//! Deserializing a [`Party`] or [`Clock`] duplicates identity exactly as
//! [`Party::decode`]/[`Clock::decode`] do — nothing ties serialized bytes to
//! their source, so their linearity notes apply verbatim at this entry point
//! ([Safety rules](crate#safety-rules)).

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::Decode;
use crate::span::Span;
use crate::{Clock, Party, Rank, Ranked, Version};

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

/// Serializes a party as its canonical bytes.
impl Serialize for Party {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(self.as_bytes())
    }
}

/// Strictly decodes a party from typed bytes or a byte sequence.
impl<'de> Deserialize<'de> for Party {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        deserialize_bytes(d, Party::decode_bytes)
    }
}

/// Serializes a version as its canonical bytes.
impl Serialize for Version {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(self.as_bytes())
    }
}

/// Strictly decodes a version from typed bytes or a byte sequence.
impl<'de> Deserialize<'de> for Version {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        deserialize_bytes(d, Version::decode_bytes)
    }
}

/// Serializes a clock as its canonical bytes.
impl Serialize for Clock {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(&self.encode())
    }
}

/// Strictly decodes a clock from typed bytes or a byte sequence.
impl<'de> Deserialize<'de> for Clock {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        deserialize_bytes(d, Clock::decode_bytes)
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

/// The canonical composite key of [`Ranked::encode`]: the rank's
/// self-delimiting stream, then the version's canonical bytes, so byte-wise
/// order on the payload is still [`Ord`] on the views.
impl Serialize for Ranked<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(&self.encode())
    }
}

/// Deserializes through [`Ranked::decode`]: the parsed rank is verified against
/// the version's own rank fold, so a mismatched pair is rejected as
/// non-canonical.
impl<'de> Deserialize<'de> for Ranked<'static> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        deserialize_bytes(d, Ranked::decode_bytes)
    }
}

/// The canonical composite of [`Span::encode`]: the meet's canonical bytes,
/// then the join's.
impl Serialize for Span<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(&self.encode())
    }
}

/// Deserializes through [`Span::decode`].
///
/// The second component is parsed while its dominance over the first is
/// validated in the same fused pass, so crossed and concurrent pairs are
/// rejected and a deserialized span is valid by construction.
impl<'de> Deserialize<'de> for Span<'static> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        deserialize_bytes(d, Span::decode_bytes)
    }
}

#[cfg(test)]
mod tests;
