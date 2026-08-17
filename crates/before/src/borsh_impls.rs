//! `borsh` support (feature-gated).
//!
//! Each type's borsh representation is exactly its canonical byte encoding:
//! [`Party::as_bytes`], [`Version::as_bytes`], [`Clock::encode`],
//! [`Rank::encode`], [`Ranked::encode`], or [`Span::encode`]. The encodings are
//! self-delimiting, so a decoder finds their ends from the encoding itself; no
//! borsh length prefix is needed. This also lets values compose inside a larger
//! borsh stream while preserving their in-memory wire form.
//!
//! Deserializing a [`Party`] or [`Clock`] duplicates identity exactly as
//! [`Party::decode`]/[`Clock::decode`] do.

use borsh::io::{Error, ErrorKind, Read, Write};
use borsh::{BorshDeserialize, BorshSerialize};

use crate::{
    codec::{self, BitCursor, BitsMut},
    error::Decode,
    span::Span,
    version::decode_rank_stream,
    Clock, Party, Rank, Ranked, Version,
};

/// A bit cursor which reads only as far as one canonical tree requires.
///
/// The encodings are prefix-free and the bytes after a tree belong to the
/// next borsh field, so the cursor pulls from the reader one byte at a time,
/// each strictly on demand — only when the parse asks for a bit past what has
/// already been read. Those bytes accumulate in `bytes`, which serves twice
/// over: it is the decode window ([`read_bit`](BitCursor::read_bit) is an
/// index + mask into it; [`read_int`](BitCursor::read_int) proves whole gamma
/// codes from it through the word decoder), and at [`finish`] it becomes the
/// value's stored bits without a copy.
///
/// [`finish`]: ReaderCursor::finish
struct ReaderCursor<'a, R> {
    reader: &'a mut R,
    /// Every byte read from `reader`, in order.
    bytes: Vec<u8>,
    /// The parse's bit position within `bytes`.
    ///
    /// Invariant: `position <= 8 * bytes.len()`, with equality exactly when
    /// the buffered bits are exhausted (the next [`read_bit`] refills). The
    /// bits between `position` and the buffer's end were read from the reader
    /// but not yet consumed by the parse; they are the only bits the
    /// [`read_int`] window may prove a code from.
    ///
    /// [`read_bit`]: BitCursor::read_bit
    /// [`read_int`]: BitCursor::read_int
    position: usize,
}

impl<'a, R: Read> ReaderCursor<'a, R> {
    fn new(reader: &'a mut R) -> Self {
        ReaderCursor {
            reader,
            bytes: Vec::new(),
            position: 0,
        }
    }

    fn finish(mut self) -> Result<BitsMut, Decode> {
        // Consume the tree's padding — one `1` marker, then zeros to the
        // byte boundary — through the same on-demand reads as the parse:
        // when the live bits end flush against a byte boundary, the
        // padding is a whole `1000_0000` byte the parse never pulled, and
        // leaving it unread would hand its bits to the next borsh field.
        let end = self.position;
        if !self.read_bit()? {
            return Err(Decode::TrailingBits);
        }
        while !self.position.is_multiple_of(8) {
            if self.read_bit()? {
                return Err(Decode::TrailingBits);
            }
        }
        let mut bits = BitsMut::from_vec(self.bytes);
        bits.truncate(end);
        Ok(bits)
    }
}

impl<R: Read> BitCursor for ReaderCursor<'_, R> {
    // The rich error type, not `cursor::Truncated`: this is the boundary
    // where `Decode::Io` enters, and it is constructed only when a read
    // actually fails — never on the per-bit success path.
    type Error = Decode;

    fn read_bit(&mut self) -> Result<bool, Decode> {
        if self.position == 8 * self.bytes.len() {
            let mut byte = [0];
            self.reader.read_exact(&mut byte).map_err(Decode::Io)?;
            self.bytes.push(byte[0]);
        }
        let bit = self.bytes[self.position / 8] & (0x80 >> (self.position % 8)) != 0;
        self.position += 1;
        Ok(bit)
    }

    fn position(&self) -> usize {
        self.position
    }

    fn read_int(&mut self) -> Result<codec::Int, Decode> {
        // Word fast path over the bytes already read, exactly as
        // `SliceCursor::read_int`: the window's proven bits end at the
        // buffer's end, so it can never consume — or even inspect — a byte
        // the reader has not yielded, and speculative reads (which would
        // steal bytes from the next borsh field) are impossible by
        // construction. It fires when earlier refills left enough unconsumed
        // bits buffered; everything else, every reject included, is decided
        // by the per-bit loop below, refilling byte by byte on demand.
        if let Some((n, next)) =
            codec::decode_int_window(codec::bytes_as_bits(&self.bytes), self.position)
        {
            self.position = next;
            return Ok(codec::Int::Small(n));
        }
        codec::decode_int_from(self).map(codec::Int::from_base)
    }
}

/// Read and validate one byte-aligned canonical id tree.
fn deserialize_id<R: Read>(reader: &mut R) -> borsh::io::Result<BitsMut> {
    let mut cursor = ReaderCursor::new(reader);
    codec::parse_id_from(&mut cursor).map_err(decode_error)?;
    cursor.finish().map_err(decode_error)
}

/// Read and validate one byte-aligned canonical skyline event stream.
fn deserialize_event<R: Read>(reader: &mut R) -> borsh::io::Result<BitsMut> {
    let mut cursor = ReaderCursor::new(reader);
    crate::version::skyline::validate_from(&mut cursor).map_err(decode_error)?;
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
        // The id grammar has no empty production (a starved reader rejects
        // inside the parse), so the parsed id is a nonzero share — the
        // standalone-party invariant (paper §3: `i ≠ 0`) holds structurally.
        let bits = deserialize_id(reader)?;
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

/// The canonical lexicographic bytes of [`Rank::encode`], unframed.
///
/// Borsh is a transport for the one wire form, never a second format,
/// so byte-wise order on the serialized bytes is still [`Ord`] on
/// ranks, and the numerator–exponent pair stays off the wire (the
/// decompression-bomb hazard [`Rank::encode`] documents).
impl BorshSerialize for Rank {
    fn serialize<W: Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        self.encode_to(writer)
    }
}

/// Reads exactly one canonical rank stream: self-delimiting, so the
/// bytes after its closing bit belong to the next borsh field.
impl BorshDeserialize for Rank {
    fn deserialize_reader<R: Read>(reader: &mut R) -> borsh::io::Result<Self> {
        decode_rank_stream(|| {
            let mut byte = [0];
            reader.read_exact(&mut byte).map_err(Decode::Io)?;
            Ok(byte[0])
        })
        .map_err(decode_error)
    }
}

/// The canonical composite key of [`Ranked::encode`], unframed: the
/// rank's self-delimiting stream, then the version's canonical bytes.
///
/// Borsh is a transport for the one wire form, never a second format,
/// so byte-wise order on the serialized bytes is still [`Ord`] on the
/// views, ties included — the causal ordering survives the transport.
impl BorshSerialize for Ranked<'_> {
    fn serialize<W: Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        self.encode_to(writer)
    }
}

/// Reads exactly one composite key: the self-delimiting rank stream,
/// then one canonical version stream.
///
/// The parsed rank is verified against the version's own rank fold
/// ([`Ranked::decode`]'s contract), and the bytes after the version
/// belong to the next borsh field.
impl BorshDeserialize for Ranked<'static> {
    fn deserialize_reader<R: Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let rank = decode_rank_stream(|| {
            let mut byte = [0];
            reader.read_exact(&mut byte).map_err(Decode::Io)?;
            Ok(byte[0])
        })
        .map_err(decode_error)?;
        let version = Version::deserialize_reader(reader)?;
        if version.rank() != rank {
            return Err(decode_error(Decode::NotCanonical));
        }
        Ok(Ranked::from(version))
    }
}

/// The canonical composite of [`Span::encode`], unframed: the meet's
/// canonical bytes, then the join's.
///
/// Borsh is a transport for the one wire form, never a second format;
/// both components are byte-aligned and self-delimiting, so the
/// composite needs no length prefix inside a larger stream.
impl BorshSerialize for Span<'_> {
    fn serialize<W: Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        self.encode_to(writer)
    }
}

/// Reads exactly one composite span: two byte-aligned canonical version
/// streams, the second parsed and validated against the first in one
/// fused pass.
///
/// [`Span::decode`]'s contract exactly — crossed and concurrent pairs
/// rejected — with the bytes after the join belonging to the next
/// borsh field.
impl BorshDeserialize for Span<'static> {
    fn deserialize_reader<R: Read>(reader: &mut R) -> borsh::io::Result<Self> {
        use crate::version::skyline::Admission;
        let lo = Version::deserialize_reader(reader)?;
        let mut cursor = ReaderCursor::new(reader);
        let admission = crate::version::skyline::validate_dominating_from(lo.view(), &mut cursor)
            .map_err(decode_error)?;
        // The final byte's padding check outranks the pair verdict,
        // exactly as the byte-slice decode orders them.
        let bits = cursor.finish().map_err(decode_error)?;
        let hi = match admission {
            Admission::Refuted => return Err(decode_error(Decode::NotCanonical)),
            // The coincident span stores one buffer twice: the admission
            // walk proved the second stream byte-equal to the first, so
            // the join is the meet's clone — an `O(1)` refcount bump the
            // ptr_eq fast paths then recognize — and the parsed bits are
            // dropped unstored.
            Admission::Equal => lo.clone(),
            Admission::Dominates => Version::from_bits(bits),
        };
        Ok(Span::owned(lo, hi))
    }
}

#[cfg(test)]
mod tests;
