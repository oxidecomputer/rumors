//! `borsh` support (feature-gated). Each type serializes as its canonical byte
//! encoding ([`encode`](crate::Clock::encode)) wrapped in borsh's length-prefixed
//! byte-sequence framing, and deserializes back through the strict validator
//! ([`decode`](crate::Clock::decode)). So the payload is exactly the wire form, a
//! deserialized value is guaranteed canonical, and — because the framing is the
//! same `u32`-length-prefixed shape borsh gives `Vec<u8>` — these values are
//! self-delimiting and compose inside a larger borsh stream (the rumors mirror
//! protocol relies on this to ship a [`Version`] frame mid-message).

use borsh::io::{Error, ErrorKind, Read, Write};
use borsh::{BorshDeserialize, BorshSerialize};

use crate::{Clock, Party, Version};

/// Write `bytes` with borsh's `Vec<u8>` framing (a `u32` little-endian length
/// prefix followed by the raw bytes), without first copying into a `Vec`.
fn serialize_bytes<W: Write>(bytes: &[u8], writer: &mut W) -> borsh::io::Result<()> {
    let len = u32::try_from(bytes.len())
        .map_err(|_| Error::new(ErrorKind::InvalidData, "encoded length exceeds u32"))?;
    len.serialize(writer)?;
    writer.write_all(bytes)
}

/// Read the length-prefixed bytes written by [`serialize_bytes`]. Symmetric with
/// borsh's own `Vec<u8>` decoding, so either side accepts the other's framing.
fn deserialize_bytes<R: Read>(reader: &mut R) -> borsh::io::Result<Vec<u8>> {
    <Vec<u8>>::deserialize_reader(reader)
}

impl BorshSerialize for Party {
    fn serialize<W: Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        serialize_bytes(self.as_bytes(), writer)
    }
}

impl BorshDeserialize for Party {
    fn deserialize_reader<R: Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let bytes = deserialize_bytes(reader)?;
        Party::decode(&bytes[..]).map_err(|e| Error::new(ErrorKind::InvalidData, e))
    }
}

impl BorshSerialize for Version {
    fn serialize<W: Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        serialize_bytes(self.as_bytes(), writer)
    }
}

impl BorshDeserialize for Version {
    fn deserialize_reader<R: Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let bytes = deserialize_bytes(reader)?;
        Version::decode(&bytes[..]).map_err(|e| Error::new(ErrorKind::InvalidData, e))
    }
}

impl BorshSerialize for Clock {
    fn serialize<W: Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        serialize_bytes(&self.encode(), writer)
    }
}

impl BorshDeserialize for Clock {
    fn deserialize_reader<R: Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let bytes = deserialize_bytes(reader)?;
        Clock::decode(&bytes[..]).map_err(|e| Error::new(ErrorKind::InvalidData, e))
    }
}
