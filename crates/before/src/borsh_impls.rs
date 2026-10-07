//! `borsh` support (feature-gated).
//!
//! The tree-based types retain exactly their canonical byte encoding:
//! [`Party::as_bytes`], [`Version::as_bytes`], [`Clock::encode`],
//! [`Rank::encode`], [`Ranked::encode`], or [`Span::encode`]. The encodings are
//! self-delimiting, so a decoder finds their ends from the encoding itself; no
//! borsh length prefix is needed. This also lets values compose inside a larger
//! borsh stream while preserving their in-memory wire form.
//!
//! [`Count`] has no tree encoding. It uses a Borsh sequence of canonical
//! least-significant-first `u64` limbs; the sequence is empty for zero.
//!
//! Deserializing a [`Party`] or [`Clock`] duplicates identity exactly as
//! [`Party::decode`]/[`Clock::decode`] do.

use borsh::io::{Error, ErrorKind, Read, Write};
use borsh::{BorshDeserialize, BorshSerialize};
use num_bigint::BigUint;

use crate::{
    bits::{BitRead, Bits, BitsReader},
    count::CanonicalLimbs,
    error::Decode,
    span::Span,
    testing::instrument::scan,
    Clock, Count, Party, Rank, Ranked, Version,
};

/// A bit reader which consumes only one canonical tree.
///
/// The encodings are prefix-free and the bytes after a tree belong to the
/// next borsh field, so this reader pulls one byte at a time,
/// each strictly on demand — only when the parse asks for a bit past what has
/// already been read. Those bytes accumulate in `bytes`, which serves twice
/// over: it is the decode window ([`read_bit`](BitRead::read_bit) is an
/// index + mask into it; [`read_gamma`](BitRead::read_gamma) proves whole gamma
/// codes from it through the word decoder), and at [`finish`] it becomes the
/// value's stored bits without a copy.
///
/// [`finish`]: StreamBitsReader::finish
struct StreamBitsReader<'a, R> {
    reader: &'a mut R,
    /// Every byte read from `reader`, in order.
    bytes: Vec<u8>,
    /// The parse's bit position within `bytes`.
    ///
    /// Invariant: `position <= 8 * bytes.len()`, with equality exactly when
    /// the buffered bits are exhausted (the next [`read_bit`] refills). The
    /// bits between `position` and the buffer's end were read from the reader
    /// but not yet consumed by the parse; they are the only bits the
    /// [`read_gamma`] window may prove a code from.
    ///
    /// `u64`, not `usize`: a field's byte count is bounded only by what the
    /// reader yields, and `8 · bytes.len()` outgrows a 32-bit `usize` from
    /// 512 MiB of field — exactly representable here.
    ///
    /// [`read_bit`]: BitRead::read_bit
    /// [`read_gamma`]: BitRead::read_gamma
    position: u64,
}

impl<'a, R: Read> StreamBitsReader<'a, R> {
    /// Begin decoding a field without reading ahead into the next one.
    fn new(reader: &'a mut R) -> Self {
        StreamBitsReader {
            reader,
            bytes: Vec::new(),
            position: 0,
        }
    }

    /// Consume the tree's padding and hand back the field's canonical bytes.
    ///
    /// The padding — one `1` marker, then zeros to the byte boundary — is
    /// consumed through the same on-demand reads as the parse: when the live
    /// bits end flush against a byte boundary, the padding is a whole
    /// `1000_0000` byte the parse never pulled, and leaving it unread would
    /// hand its bits to the next borsh field. After it, the buffered bytes
    /// are exactly the tree's marker-padded canonical spelling, which the
    /// decoded value adopts without a copy.
    fn finish(mut self) -> Result<Vec<u8>, Decode> {
        if !self.read_bit()? {
            return Err(Decode::TrailingBits);
        }
        while !self.position.is_multiple_of(8) {
            if self.read_bit()? {
                return Err(Decode::TrailingBits);
            }
        }
        Ok(self.bytes)
    }
}

impl<R: Read> BitRead for StreamBitsReader<'_, R> {
    // The rich error type, not the in-memory reader's `Truncated`: this is the boundary
    // where `Decode::Io` enters, and it is constructed only when a read
    // actually fails — never on the per-bit success path.
    type Error = Decode;

    fn read_bit(&mut self) -> Result<bool, Decode> {
        if self.position == self.bytes.len() as u64 * 8 {
            let mut byte = [0];
            self.reader.read_exact(&mut byte).map_err(Decode::Io)?;
            self.bytes.push(byte[0]);
        }
        // An in-range byte index fits `usize`: it indexes the buffer.
        let bit = self.bytes[(self.position / 8) as usize] & (0x80 >> (self.position % 8)) != 0;
        // Count the logical bit consumed; the meter observes the decoder's
        // work rather than how reads are batched.
        scan::record_bits(1);
        self.position += 1;
        Ok(bit)
    }

    fn position(&self) -> u64 {
        self.position
    }

    fn read_gamma(&mut self) -> Result<BigUint, Decode> {
        // Word fast path over the bytes already read, exactly as
        // the in-memory decoder: the window's proven bits end at the
        // buffer's end, so it can never consume — or even inspect — a byte
        // the reader has not yielded, and speculative reads (which would
        // steal bytes from the next borsh field) are impossible by
        // construction. It fires when earlier refills left enough unconsumed
        // bits buffered; everything else, every reject included, is decided
        // by the per-bit loop below, refilling byte by byte on demand.
        if let Some((n, next)) =
            BitsReader::gamma_from_window(&self.bytes, self.bytes.len() as u64 * 8, self.position)
        {
            scan::record_bits(next - self.position);
            self.position = next;
            return Ok(BigUint::from(n));
        }
        self.read_gamma_slow()
    }
}

impl Decode {
    /// Preserve an underlying I/O failure; classify malformed bytes as invalid data.
    fn into_borsh_error(self) -> Error {
        match self {
            Decode::Io(source) => source,
            error => Error::new(ErrorKind::InvalidData, error),
        }
    }
}

impl BorshSerialize for Party {
    fn serialize<W: Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        writer.write_all(self.as_bytes())
    }
}

impl BorshDeserialize for Party {
    fn deserialize_reader<R: Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let mut bits = StreamBitsReader::new(reader);
        crate::party::io::validate::from_reader(&mut bits).map_err(Decode::into_borsh_error)?;
        let bytes = bits.finish().map_err(Decode::into_borsh_error)?;
        // Every stored terminal owns its region, and every stored branch has a
        // child. A complete parsed tree therefore satisfies Party's nonempty
        // ownership invariant.
        Ok(Party::from_canonical(Bits::from_canonical(bytes.into())))
    }
}

impl BorshSerialize for Version {
    fn serialize<W: Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        writer.write_all(self.as_bytes())
    }
}

impl BorshDeserialize for Version {
    fn deserialize_reader<R: Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let mut bits = StreamBitsReader::new(reader);
        crate::version::io::validate::from_reader(&mut bits).map_err(Decode::into_borsh_error)?;
        let bytes = bits.finish().map_err(Decode::into_borsh_error)?;
        Ok(Version::from_canonical(Bits::from_canonical(bytes.into())))
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
        Rank::decode_stream(|| {
            let mut byte = [0];
            reader.read_exact(&mut byte).map_err(Decode::Io)?;
            Ok(byte[0])
        })
        .map_err(Decode::into_borsh_error)
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
        let mut rank_bytes = Vec::new();
        Rank::decode_stream(|| {
            let mut byte = [0];
            reader.read_exact(&mut byte).map_err(Decode::Io)?;
            rank_bytes.push(byte[0]);
            Ok(byte[0])
        })
        .map_err(Decode::into_borsh_error)?;
        let version = Version::deserialize_reader(reader)?;
        if !version.rank().encoding_matches(&rank_bytes) {
            return Err(Decode::NotCanonical.into_borsh_error());
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
        use crate::version::io::validate::Admission;
        let lo = Version::deserialize_reader(reader)?;
        let mut cursor = StreamBitsReader::new(reader);
        let admission = crate::version::io::validate::dominating_from(&lo, &mut cursor)
            .map_err(Decode::into_borsh_error)?;
        // The final byte's padding check outranks the pair verdict,
        // exactly as the byte-slice decode orders them.
        let bytes = cursor.finish().map_err(Decode::into_borsh_error)?;
        let hi = match admission {
            Admission::Refuted => return Err(Decode::NotCanonical.into_borsh_error()),
            // The coincident span stores one buffer twice: the admission
            // walk proved the second stream byte-equal to the first, so
            // the join is the meet's clone — an `O(1)` refcount bump the
            // ptr_eq fast paths then recognize — and the parsed bytes are
            // dropped unstored.
            Admission::Equal => lo.clone(),
            Admission::Dominates => Version::from_canonical(Bits::from_canonical(bytes.into())),
        };
        Ok(Span::owned(lo, hi))
    }
}

/// Encodes a count as its least-significant-first `u64` limbs. Borsh's sequence
/// length prefixes the limbs; zero is the empty sequence.
impl BorshSerialize for Count {
    fn serialize<W: Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        let len = u32::try_from(self.limbs().len())
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "count has too many limbs"))?;
        len.serialize(writer)?;
        for limb in self.limbs() {
            limb.serialize(writer)?;
        }
        Ok(())
    }
}

/// Decodes canonical least-significant-first `u64` limbs. A nonempty sequence
/// ending in zero is rejected as redundant.
impl BorshDeserialize for Count {
    fn deserialize_reader<R: Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let len = u32::deserialize_reader(reader)? as usize;
        let mut limbs = CanonicalLimbs::new();
        for _ in 0..len {
            limbs.push(u64::deserialize_reader(reader)?);
        }
        limbs
            .finish()
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "count has a trailing zero limb"))
    }
}

#[cfg(test)]
mod tests;
