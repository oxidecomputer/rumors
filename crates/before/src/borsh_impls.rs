//! `borsh` support (feature-gated).
//!
//! Each type's borsh representation is exactly its canonical byte encoding:
//! [`Party::as_bytes`], [`Version::as_bytes`], or [`Clock::encode`]. The tree
//! encodings are prefix-free, so a decoder finds their ends from the encoding
//! itself; no borsh length prefix is needed. This also lets values compose
//! inside a larger borsh stream while preserving their in-memory wire form.

use borsh::io::{Error, ErrorKind, Read, Write};
use borsh::{BorshDeserialize, BorshSerialize};

use crate::{
    codec::{self, BitCursor, Bits},
    error::Decode,
    Clock, Party, Version,
};

/// A bit cursor which reads only as far as one canonical tree requires.
struct ReaderCursor<'a, R> {
    reader: &'a mut R,
    bits: Bits,
    position: usize,
}

impl<'a, R> ReaderCursor<'a, R> {
    fn new(reader: &'a mut R) -> Self {
        ReaderCursor {
            reader,
            bits: Bits::new(),
            position: 0,
        }
    }

    fn finish(mut self) -> Result<Bits, Decode> {
        codec::require_zero_padding(&self.bits, self.position)?;
        self.bits.truncate(self.position);
        Ok(self.bits)
    }
}

impl<R: Read> BitCursor for ReaderCursor<'_, R> {
    fn read_bit(&mut self) -> Result<bool, Decode> {
        if self.position == self.bits.len() {
            let mut byte = [0];
            self.reader.read_exact(&mut byte).map_err(Decode::Io)?;
            self.bits.extend_from_bitslice(codec::bytes_as_bits(&byte));
        }
        let bit = self.bits[self.position];
        self.position += 1;
        Ok(bit)
    }

    fn position(&self) -> usize {
        self.position
    }
}

/// Read and validate one byte-aligned canonical id tree.
fn deserialize_id<R: Read>(reader: &mut R) -> borsh::io::Result<Bits> {
    let mut cursor = ReaderCursor::new(reader);
    codec::parse_id_from(&mut cursor).map_err(decode_error)?;
    cursor.finish().map_err(decode_error)
}

/// Read and validate one byte-aligned canonical event tree.
fn deserialize_event<R: Read>(reader: &mut R) -> borsh::io::Result<Bits> {
    let mut cursor = ReaderCursor::new(reader);
    codec::parse_ev_from(&mut cursor).map_err(decode_error)?;
    cursor.finish().map_err(decode_error)
}

fn decode_error(error: Decode) -> Error {
    match error {
        Decode::Io(source) => source,
        error => Error::new(ErrorKind::InvalidData, error),
    }
}

impl BorshSerialize for Party {
    fn serialize<W: Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        writer.write_all(self.as_bytes())
    }
}

impl BorshDeserialize for Party {
    fn deserialize_reader<R: Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let bits = deserialize_id(reader)?;
        if codec::id_is_empty(&bits) {
            return Err(decode_error(Decode::Anonymous));
        }
        Ok(Party::from_bits(bits))
    }
}

impl BorshSerialize for Version {
    fn serialize<W: Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        writer.write_all(self.as_bytes())
    }
}

impl BorshDeserialize for Version {
    fn deserialize_reader<R: Read>(reader: &mut R) -> borsh::io::Result<Self> {
        deserialize_event(reader).map(Version::from_bits)
    }
}

impl BorshSerialize for Clock {
    fn serialize<W: Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        self.encode_to(writer)
    }
}

impl BorshDeserialize for Clock {
    fn deserialize_reader<R: Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let party = Party::deserialize_reader(reader)?;
        let version = Version::deserialize_reader(reader)?;
        Ok(Clock::from_parts(party, version))
    }
}

#[cfg(test)]
mod tests;
