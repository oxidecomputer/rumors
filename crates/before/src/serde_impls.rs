//! `serde` support (feature-gated).
//!
//! Each type serializes as its canonical byte encoding
//! ([`encode`](crate::Clock::encode)) and deserializes back through the strict
//! validator ([`decode`](crate::Clock::decode)), so the serialized form is
//! exactly the wire form and a deserialized value is guaranteed to be in
//! canonical normal form.
//!
//! Deserializing a [`Party`] or [`Clock`] duplicates identity exactly as
//! [`Party::decode`]/[`Clock::decode`] do — nothing ties serialized bytes to
//! their source, so their linearity notes apply verbatim at this door ([Safety
//! rules](crate#safety-rules)).

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::span::Span;
use crate::{Clock, Party, Rank, Ranked, Version};

impl Serialize for Party {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(&self.encode())
    }
}

impl<'de> Deserialize<'de> for Party {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let bytes = <Vec<u8>>::deserialize(d)?;
        Party::decode(&bytes[..]).map_err(D::Error::custom)
    }
}

impl Serialize for Version {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(&self.encode())
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let bytes = <Vec<u8>>::deserialize(d)?;
        Version::decode(&bytes[..]).map_err(D::Error::custom)
    }
}

impl Serialize for Clock {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(&self.encode())
    }
}

impl<'de> Deserialize<'de> for Clock {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let bytes = <Vec<u8>>::deserialize(d)?;
        Clock::decode(&bytes[..]).map_err(D::Error::custom)
    }
}

/// The canonical lexicographic bytes of [`Rank::encode`].
///
/// Byte-wise order on the payload is still [`Ord`] on ranks, and the
/// numerator–exponent pair stays off the wire (the decompression-bomb hazard
/// [`Rank::encode`] documents).
impl Serialize for Rank {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(&self.encode())
    }
}

impl<'de> Deserialize<'de> for Rank {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let bytes = <Vec<u8>>::deserialize(d)?;
        Rank::decode(&bytes[..]).map_err(D::Error::custom)
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
        let bytes = <Vec<u8>>::deserialize(d)?;
        Ranked::decode(&bytes[..]).map_err(D::Error::custom)
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
        let bytes = <Vec<u8>>::deserialize(d)?;
        Span::decode(&bytes[..]).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests;
