//! `serde` support (feature-gated).
//!
//! Each type serializes as its canonical byte encoding
//! ([`encode`](crate::Clock::encode)) and deserializes back through the strict
//! validator ([`decode`](crate::Clock::decode)), so the serialized form is
//! exactly the wire form and a deserialized value is guaranteed to be in
//! canonical normal form.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{Clock, Party, Version};

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
