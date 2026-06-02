use std::io;

use bitvec::domain::Domain;
use bitvec::prelude::*;

use crate::DecodeError;

/// The packed storage form: a most-significant-bit-first bit stream over bytes.
pub(crate) type Bits = BitVec<u8, Msb0>;

/// A borrowed view of the packed storage form.
pub(crate) type BitsSlice = BitSlice<u8, Msb0>;

/// Borrow bytes as an MSB-first bit stream without first copying them into a
/// [`Bits`].
pub(crate) fn bytes_as_bits(bytes: &[u8]) -> &BitsSlice {
    bytes.view_bits::<Msb0>()
}

/// Streams a bit-concatenation of canonical bit slices to a writer, packing
/// MSB-first into bytes and zero-padding the final partial byte — with no
/// intermediate buffer. `Clock::encode_to` writes the id stream then the event
/// stream through one of these, so the cross-stream byte (the partial id tail
/// merged with the leading event bits) is produced on the fly rather than via a
/// combined `BitVec`; single-stream `Party`/`Version` go through
/// [`pack_to_writer`].
pub(crate) struct BitWriter<'w, W: io::Write> {
    w: &'w mut W,
    /// The byte under construction: `filled` valid bits in its high positions
    /// (MSB first), the low `8 - filled` bits zero.
    cur: u8,
    /// Number of valid high bits in `cur` (`0..8`).
    filled: u32,
}

impl<'w, W: io::Write> BitWriter<'w, W> {
    pub(crate) fn new(w: &'w mut W) -> Self {
        BitWriter {
            w,
            cur: 0,
            filled: 0,
        }
    }

    /// Append `k` bits (`1..=8`) taken from the high `k` positions of `src` (its
    /// low `8 - k` bits must be zero), MSB-first.
    fn push(&mut self, src: u8, k: u32) -> io::Result<()> {
        debug_assert!(
            (1..=8).contains(&k) && u32::from(src).trailing_zeros() >= 8 - k,
            "push expects {k} live bits in the high positions of {src:#010b}",
        );
        if self.filled + k < 8 {
            self.cur |= src >> self.filled;
            self.filled += k;
            Ok(())
        } else {
            let out = self.cur | (src >> self.filled);
            self.w.write_all(&[out])?;
            // The bits of `src` that did not fit become the next partial byte,
            // shifted to the high positions; the `u16` cast keeps `<< 8` (when
            // `filled == 0`, i.e. a whole byte) from overflowing and clears `cur`.
            self.cur = ((u16::from(src) << (8 - self.filled)) & 0xFF) as u8;
            self.filled = self.filled + k - 8;
            Ok(())
        }
    }

    /// Append a canonical bit slice (MSB-first). The slice starts on a byte
    /// boundary in its own backing store (every stored `Party`/`Version` does):
    /// when the writer is itself byte-aligned the whole-byte body is emitted in
    /// one `write_all`; otherwise it is merged byte-by-byte across the boundary.
    pub(crate) fn write(&mut self, bits: &BitsSlice) -> io::Result<()> {
        if bits.is_empty() {
            return Ok(());
        }
        match bits.domain() {
            Domain::Enclave(elem) if elem.head().into_inner() == 0 => {
                self.push(elem.load_value(), bits.len() as u32)
            }
            Domain::Region {
                head: None,
                body,
                tail,
            } => {
                if self.filled == 0 {
                    self.w.write_all(body)?;
                } else {
                    for &b in body {
                        self.push(b, 8)?;
                    }
                }
                if let Some(elem) = tail {
                    self.push(elem.load_value(), (bits.len() % 8) as u32)?;
                }
                Ok(())
            }
            _ => {
                // A source that does not start on a byte boundary — not produced
                // by the stored forms; per-bit fallback keeps the writer correct
                // for any slice without an intermediate buffer.
                for bit in bits.iter().by_vals() {
                    self.push(if bit { 0x80 } else { 0 }, 1)?;
                }
                Ok(())
            }
        }
    }

    /// Flush the final partial byte (zero-padded) if any bits are pending.
    pub(crate) fn finish(self) -> io::Result<()> {
        if self.filled > 0 {
            self.w.write_all(&[self.cur])?;
        }
        Ok(())
    }
}

/// Pack a single canonical bit stream into bytes written to `w`, zero-padding
/// the final partial byte. The single-stream entry to [`BitWriter`] used by
/// `Party`/`Version`'s `encode_to`.
pub(crate) fn pack_to_writer<W: io::Write>(bits: &BitsSlice, w: &mut W) -> io::Result<()> {
    let mut writer = BitWriter::new(w);
    writer.write(bits)?;
    writer.finish()
}

/// Require that the bits from `pos` onward are exactly the canonical padding: a
/// run of zeros shorter than a byte. [`pack_to_writer`] only pads the final
/// partial byte, so a canonical stream has at most 7 trailing zero bits; both a
/// nonzero padding bit AND a whole spurious zero byte (`>= 8` trailing bits,
/// even if all zero) are non-canonical. Bounding the length is what makes
/// `decode` injective on bytes — without it, `decode([.., 0x00])` would accept
/// the same value under infinitely many byte strings, re-encoding to a shorter
/// stream than its own input.
pub(crate) fn require_zero_padding(bits: &BitsSlice, pos: usize) -> Result<(), DecodeError> {
    if bits.len() - pos >= 8 || bits[pos..].any() {
        Err(DecodeError::TrailingBits)
    } else {
        Ok(())
    }
}
