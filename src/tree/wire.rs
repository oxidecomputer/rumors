//! The alternating protocol's byte codec: explicit structural framing over
//! `std::io`, with every variable-width atom one CBOR value.
//!
//! Lives beside the tree because the node serializer is one of its
//! encoders; the streaming protocol has its own codec and uses only the
//! CBOR atoms.
//!
//! Container shapes (counts, radix records, fixed-width prefixes and
//! hashes) are protocol framing, written and validated by hand exactly
//! like the streaming codec's signal and length headers. The two atoms a
//! frame cannot delimit itself — a [`Version`] and a [`Message`] — ride
//! as single CBOR values, self-delimiting by CBOR's own length headers:
//! the version as a byte string wrapping its canonical encoding (the
//! `before` serde form), the message as a byte string wrapping its cached
//! CBOR payload. Decoding pulls exactly one value off the stream, so the
//! bytes after an atom belong to the next field.

use std::io::{Read, Write};

use crate::Version;
use crate::message::Message;
use crate::tree::typed::Hash;

/// Encode `self` onto a byte stream.
///
/// The method is `write_wire`, not `encode_to`: `before`'s types carry
/// inherent `encode_to` methods (the bare canonical codec), and an
/// inherent method silently shadows a trait method at call sites — a
/// shadowed call here would write unframed bytes where the decoder
/// expects a CBOR value.
pub(crate) trait Encode {
    fn write_wire<W: Write>(&self, writer: &mut W) -> std::io::Result<()>;
}

/// Decode one `Self` off a byte stream, consuming exactly its bytes.
///
/// Named `read_wire` for the same shadowing reason as
/// [`Encode::write_wire`].
pub(crate) trait Decode: Sized {
    fn read_wire<R: Read>(reader: &mut R) -> std::io::Result<Self>;
}

/// Encode a value into a fresh buffer.
pub(crate) fn to_vec<T: Encode>(value: &T) -> std::io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    value.write_wire(&mut buf)?;
    Ok(buf)
}

/// Decode a value from an exact slice, rejecting trailing bytes.
pub(crate) fn from_slice<T: Decode>(mut bytes: &[u8]) -> std::io::Result<T> {
    let value = T::read_wire(&mut bytes)?;
    if !bytes.is_empty() {
        return Err(invalid(format!(
            "{} trailing bytes after the decoded value",
            bytes.len()
        )));
    }
    Ok(value)
}

/// An `InvalidData` error with the given message.
pub(crate) fn invalid(message: impl Into<String>) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message.into())
}

/// Map a ciborium serialization failure into the stream's error type.
///
/// Value errors are unreachable for the types this codec writes (their
/// `Serialize` impls emit exactly one byte string), so every failure here
/// is the writer's.
fn ser_error(error: ciborium::ser::Error<std::io::Error>) -> std::io::Error {
    match error {
        ciborium::ser::Error::Io(error) => error,
        ciborium::ser::Error::Value(message) => invalid(message),
    }
}

/// Map a ciborium deserialization failure into the stream's error type,
/// preserving the truncation/corruption split the callers classify by.
fn de_error(error: ciborium::de::Error<std::io::Error>) -> std::io::Error {
    match error {
        ciborium::de::Error::Io(error) => error,
        error => invalid(error.to_string()),
    }
}

impl Encode for u8 {
    fn write_wire<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&[*self])
    }
}

impl Decode for u8 {
    fn read_wire<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut byte = [0u8; 1];
        reader.read_exact(&mut byte)?;
        Ok(byte[0])
    }
}

/// A container's length: a little-endian `u32` count.
impl Encode for u32 {
    fn write_wire<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&self.to_le_bytes())
    }
}

impl Decode for u32 {
    fn read_wire<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut bytes = [0u8; 4];
        reader.read_exact(&mut bytes)?;
        Ok(u32::from_le_bytes(bytes))
    }
}

/// A length-counted sequence: `u32` LE count, then each element.
impl<A: Encode> Encode for Vec<A> {
    fn write_wire<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let count = u32::try_from(self.len())
            .map_err(|_| invalid("container length does not fit in a u32 count"))?;
        count.write_wire(writer)?;
        for item in self {
            item.write_wire(writer)?;
        }
        Ok(())
    }
}

impl<A: Decode> Decode for Vec<A> {
    fn read_wire<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let count = u32::read_wire(reader)? as usize;
        // Grow as elements arrive rather than trusting the declared count
        // for the allocation (the same discipline as the framing reader).
        let mut items = Vec::new();
        for _ in 0..count {
            items.push(A::read_wire(reader)?);
        }
        Ok(items)
    }
}

/// An optional value: a presence byte (0 or 1), then the value.
impl<A: Encode> Encode for Option<A> {
    fn write_wire<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        match self {
            None => 0u8.write_wire(writer),
            Some(value) => {
                1u8.write_wire(writer)?;
                value.write_wire(writer)
            }
        }
    }
}

impl<A: Decode> Decode for Option<A> {
    fn read_wire<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        match u8::read_wire(reader)? {
            0 => Ok(None),
            1 => Ok(Some(A::read_wire(reader)?)),
            tag => Err(invalid(format!("invalid Option tag byte {tag}"))),
        }
    }
}

/// A pair: the two fields back to back.
impl<A: Encode, B: Encode> Encode for (A, B) {
    fn write_wire<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.write_wire(writer)?;
        self.1.write_wire(writer)
    }
}

impl<A: Decode, B: Decode> Decode for (A, B) {
    fn read_wire<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        Ok((A::read_wire(reader)?, B::read_wire(reader)?))
    }
}

/// A Merkle hash: its raw fixed-width bytes (the width is pinned by the
/// type, so no length travels).
impl Encode for Hash {
    fn write_wire<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(self.as_bytes())
    }
}

impl Decode for Hash {
    fn read_wire<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut bytes = [0u8; crate::tree::typed::hash::MERKLE_HASH_LEN];
        reader.read_exact(&mut bytes)?;
        Ok(Hash(bytes))
    }
}

/// One CBOR value: a byte string wrapping the canonical version encoding.
impl Encode for Version {
    fn write_wire<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        ciborium::ser::into_writer(self, writer).map_err(ser_error)
    }
}

impl Decode for Version {
    fn read_wire<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        ciborium::de::from_reader(reader).map_err(de_error)
    }
}

/// One CBOR value: a byte string wrapping the cached CBOR payload. The
/// decode direction is typed ([`Message::from_reader`]): the payload's
/// type is erased in storage, so decoding names it explicitly.
impl Encode for Message {
    fn write_wire<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        ciborium::ser::into_writer(self, writer).map_err(ser_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{Message, PayloadCodec, PayloadDepthLimit};
    use crate::tree::arb::nth_party;
    use crate::tree::typed::Node;
    use crate::tree::typed::height::Z;
    use crate::tree::typed::node::DecodeNode;

    /// Every wire atom round-trips through `to_vec`/`from_slice`, alone
    /// and composed: the version and message as CBOR values, the node's
    /// structural framing around them, and a `Vec` of prefix-node pairs.
    ///
    /// The leaf case is the shadowing tripwire: `Version::encode_to` (the
    /// bare canonical codec) must never be what the node serializer calls,
    /// or the version travels unframed and the decode misaligns.
    #[test]
    fn atoms_round_trip() {
        let party = nth_party(0);
        let mut version = crate::Version::new();
        version.tick(&party);

        let v = to_vec(&version).unwrap();
        let back: crate::Version = from_slice(&v).unwrap();
        assert_eq!(back, version);

        let m = Message::new(());
        let enc = to_vec(&m).unwrap();
        let back = Message::from_reader(
            &mut enc.as_slice(),
            PayloadCodec::mint::<()>(PayloadDepthLimit::default()),
        )
        .unwrap();
        assert_eq!(back.as_slice(), m.as_slice());

        let leaf: Node<Z> = Node::leaf(version.clone(), Message::new(()));
        let enc = to_vec(&leaf).unwrap();
        let mut input = enc.as_slice();
        let back = Z::read_node(
            &mut input,
            PayloadCodec::mint::<()>(PayloadDepthLimit::default()),
        )
        .unwrap();
        assert!(
            input.is_empty(),
            "the node decode consumes exactly its bytes"
        );
        assert_eq!(back.hash(), leaf.hash());
    }
}
