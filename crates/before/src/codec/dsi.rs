//! The word-parallel stream cursor: sequential bit, unary, and Elias-gamma
//! reads over one packed stream, on `dsi-bitstream`'s buffered big-endian
//! reader.
//!
//! [`DsiCursor`] is the skyline walks' production reader. It reads the same
//! bits and returns the same values as the per-bit [`BitCursor`] loop over a
//! [`SliceCursor`](super::SliceCursor), while taking whole topology runs in one
//! [`read_unary`](DsiCursor::read_unary) (a `leading_zeros` over a buffered
//! window) and whole payload codes in `O(1)` word operations. The accept/reject
//! boundary lives in this wrapper, not the library: `position`/`len` bound
//! every read against the view's live bit length, so the reader's zero padding
//! past the live bits is never surfaced as data.
//!
//! Values are read through the in-house wide arm, never `dsi-bitstream`'s own
//! `read_gamma`: that entry supports only values below `2^64` (guarded by a
//! `debug_assert`, a silent mis-decode in release), while this crate's coding
//! has no value cap, which is required because joins must be allowed to
//! propagate arbitrary-width heights.
//!
//! [`read_int`](DsiCursor::read_int) therefore composes the unary prefix and
//! mantissa itself: a 9-bit table tier, a machine-word arm for `k < 64`, and a
//! `UBig` wide arm for `k >= 64`, bit-identical to the per-bit loop's
//! ([`decode_int_from`](super::decode_int_from)) wide fallback. The witnesses
//! in `tests` pin the value and the consumed width at and across the word seam
//! (`k = 63, 64, 65, ~100`).
//!
//! The writers stay in-house ([`PackedBuilder`](super::PackedBuilder) and the
//! byte-backed append paths): `dsi-bitstream`'s writer wants a word sink, and
//! adapting the byte-backed stores to one is a separate trade this module does
//! not make.

use core::fmt::Display;

use dashu_int::UBig;
use dsi_bitstream::codes::gamma_tables;
use dsi_bitstream::impls::BufBitReader;
use dsi_bitstream::traits::{BitRead, WordRead, BE};

use crate::error::Decode;

use super::cursor::Truncated;
use super::{Base, BitCursor, BitsView, Int};

/// A word-parallel sequential cursor over an existing packed bit view.
///
/// The skyline walks' reader: [`read_bit`](BitCursor::read_bit) for interleaved
/// single flags, [`read_unary`](BitCursor::read_unary) for topology runs,
/// [`read_int`](BitCursor::read_int) for payload codes, and
/// [`skip_int`](DsiCursor::skip_int) for codes whose value is not needed. Every
/// read records the same scan-meter bits as the per-bit loop it replaces: the
/// meter prices bits examined, not how the examining path batches them.
pub(crate) struct DsiCursor<'a> {
    reader: BufBitReader<BE, ByteWords<'a>>,
    /// The position immediately after the last live bit read.
    ///
    /// `u64`, not `usize`: the byte decode doors walk a whole input buffer
    /// as bits, and `8 · bytes.len()` can exceed a 32-bit `usize` (a 600
    /// MiB buffer holds 2^32+ bit positions) while remaining exactly
    /// representable here. Stored-stream walks never leave `usize` range —
    /// the storage doors bound a stream's live length below it.
    position: u64,
    /// The stream's live bit length, in the same `u64` denomination.
    len: u64,
}

impl<'a> DsiCursor<'a> {
    /// Open a cursor at bit 0 of a stream's view.
    ///
    /// A stored stream's live view and a byte decode door's whole padded
    /// buffer enter identically: padding bits are data to a door's walk,
    /// and the door's marker check afterwards judges the remainder.
    pub(crate) fn new(bits: BitsView<'a>) -> Self {
        DsiCursor::new_at(bits, 0)
    }

    /// Open a cursor at bit `pos` of a stream's view.
    ///
    /// `O(1)`: the word source starts at `pos`'s byte and the cursor discards
    /// the at most 7 leading bits before `pos` unrecorded (the walk never
    /// examines them).
    ///
    /// # Panics
    ///
    /// Panics if `pos` lies past the view's live length.
    pub(crate) fn new_at(bits: BitsView<'a>, pos: u64) -> Self {
        assert!(pos <= bits.len(), "cursor opened past the stream's end");
        let (body, tail) = bits.body_tail();
        let mut reader = BufBitReader::new(ByteWords::new(body, tail, (pos / 8) as usize));
        let skip = (pos % 8) as usize;
        if skip != 0 {
            reader
                .skip_bits(skip)
                .expect("a mid-byte start has at least its own byte to skip within");
        }
        DsiCursor {
            reader,
            position: pos,
            len: bits.len(),
        }
    }

    /// The position immediately after the last bit read, in the cursor's own
    /// `u64` denomination: the byte decode doors' end-position read, exact
    /// even where the walked buffer holds more bit positions than `usize`.
    pub(crate) fn position_u64(&self) -> u64 {
        self.position
    }

    /// Read the unary prefix at the cursor without recording a successful run:
    /// the count of `0` bits before (and consuming) the terminating `1`.
    ///
    /// `Truncated` when the live bits end before a `1`: the phantom
    /// zeros past the live length (the word source masks the tail
    /// byte's dead bits and zero-fills past the stream) can only
    /// lengthen an apparent prefix past `len`, never terminate one
    /// early.
    fn unary_raw(&mut self) -> Result<u64, Truncated> {
        match self.reader.read_unary() {
            Err(_) => Err(self.truncated()),
            Ok(k) => {
                // No overflow: position and len are at most 8 · a buffer's
                // byte count and k is bounded by the word source's total
                // bits, all far below 2^64.
                if self.position + k + 1 > self.len {
                    return Err(self.truncated());
                }
                Ok(k)
            }
        }
    }

    /// Reject at the live length, recording the examined tail.
    ///
    /// A rejecting read still examined every remaining live bit — a
    /// self-delimiting stream's truncation is only discoverable by parsing to
    /// its end, which is exactly what the truncation-reject scan floors demand
    /// the meter see — so the tail records before the reject surfaces, and the
    /// cursor parks at the live length, where the per-bit loop's failing read
    /// leaves its own cursor.
    fn truncated(&mut self) -> Truncated {
        super::scan::record_bits_u64(self.len - self.position);
        self.position = self.len;
        Truncated
    }

    /// Skip one Elias-gamma-coded integer at the cursor without materializing
    /// its value; `Truncated` exactly where [`read_int`](BitCursor::read_int)
    /// would be.
    ///
    /// The prefix length alone determines the code's width, so the skip is one
    /// unary read plus a bit discard; it records the same scan-meter bits a
    /// read would — the skip is the topology walks' stand-in for reading the
    /// code.
    pub(crate) fn skip_int(&mut self) -> Result<(), Truncated> {
        let k = self.unary_raw()?;
        let code_len = 2 * k + 1;
        if self.position + code_len > self.len {
            return Err(self.truncated());
        }
        let mut remaining = k;
        while remaining > 0 {
            let chunk = remaining.min(u64::from(u64::BITS));
            self.reader
                .skip_bits(chunk as usize)
                .expect("the mantissa was proven to fit the live length");
            remaining -= chunk;
        }
        super::scan::record_bits_u64(code_len);
        self.position += code_len;
        Ok(())
    }
}

impl BitCursor for DsiCursor<'_> {
    type Error = Truncated;

    fn read_bit(&mut self) -> Result<bool, Truncated> {
        if self.position >= self.len {
            return Err(Truncated);
        }
        let bit = self
            .reader
            .read_bits(1)
            .expect("the word source zero-fills to the live length")
            != 0;
        // Same scan-meter record as `SliceCursor::read_bit`: the meter
        // prices bits examined, not how the examining path batches them.
        super::scan::record_bits(1);
        self.position += 1;
        Ok(bit)
    }

    fn position(&self) -> usize {
        // Exact for every walk that reads positions through the trait: a
        // stored stream's live length fits `usize` on every target (the
        // storage doors bound it), and the byte decode doors read their
        // end positions through `position_u64` instead. A byte-door
        // walk parking past usize (its truncation reject on a buffer past
        // the storable bound) is never followed by a trait position read.
        usize::try_from(self.position).expect("a walked position fits usize")
    }

    fn read_unary(&mut self) -> Result<usize, Truncated> {
        let k = self.unary_raw()?;
        super::scan::record_bits_u64(k + 1);
        self.position += k + 1;
        // A run at or past usize bits (possible only on a 32-bit target,
        // inside a multi-part door buffer past 512 MiB) can only belong to
        // a stream past the storable bound — its part could never freeze —
        // so the checked conversion fails loudly there instead of handing
        // the walk a truncated count.
        Ok(usize::try_from(k).expect("a unary run fits usize"))
    }

    /// Read one Elias-gamma-coded integer: accepting and rejecting on exactly
    /// the same inputs as [`decode_int_from`](super::decode_int_from), per-bit
    /// over this cursor.
    ///
    /// Three tiers, all word-parallel through the buffered reader:
    ///
    /// - a 9-bit table hit (`gamma_tables::read_table_be`), taken only
    ///   when at least 9 live bits remain, so a table-matched code can
    ///   never extend past the live length;
    /// - the composed unary-prefix + mantissa read for machine-word
    ///   codes (`k < 64`) — `dsi-bitstream`'s own `read_gamma` is
    ///   unusable here because its supported range caps at `u64` while
    ///   this coding has no value cap;
    /// - the wide arm (`k >= 64`), bit-identical to
    ///   [`decode_int_from`](super::decode_int_from)'s wide
    ///   fallback: mantissa top bit at `k`, then `k` stream bits filled
    ///   from word chunks.
    ///
    /// A `1` bit can only live inside the live length, so a returned unary
    /// prefix always ends inside the stream; the explicit length checks bound
    /// the mantissa, and a stream ending mid-code reads `Truncated` exactly as
    /// the per-bit loop does.
    fn read_int(&mut self) -> Result<Int, Decode> {
        // Table tier: only when the peeked 9 bits are all live.
        if self.len - self.position >= gamma_tables::READ_BITS as u64 {
            if let Some((value, used)) = gamma_tables::read_table_be(&mut self.reader) {
                super::scan::record_bits(used);
                self.position += used as u64;
                return Ok(Int::Small(value));
            }
        }
        let k = self.unary_raw().map_err(|_| Decode::Truncated)?;
        // The unary prefix's terminating 1 is a live bit, so the prefix fits;
        // the whole `2k + 1`-bit code must too. Rejecting here — before either
        // mantissa arm runs — is where this reader parts from the per-bit loop
        // on cost: a truncated wide code allocates nothing, where the loop
        // sizes the wide value before its mantissa read can fail.
        let code_len = 2 * k + 1;
        if self.position + code_len > self.len {
            self.truncated();
            return Err(Decode::Truncated);
        }
        if k < u64::from(u64::BITS) {
            let rest = self
                .reader
                .read_bits(k as usize)
                .expect("the mantissa was proven to fit the live length");
            let m = (1u64 << k) | rest;
            super::scan::record_bits_u64(code_len);
            self.position += code_len;
            return Ok(Int::Small(m - 1));
        }
        // Wide arm: the mantissa's top bit is at position `k`; the next `k`
        // stream bits fill positions `k - 1 ..= 0`, most-significant first,
        // read in machine-word chunks. The bit-index conversion is checked:
        // a mantissa at or past usize bits (a 32-bit target, a code deeper
        // than the storable bound) fails loudly instead of aliasing a low
        // bit index — and the value's backend caps below that width anyway.
        let k = usize::try_from(k).expect("a mantissa width fits usize");
        let mut m = UBig::ZERO;
        m.set_bit(k);
        let mut remaining = k;
        while remaining > 0 {
            let chunk_bits = remaining.min(u64::BITS as usize);
            let chunk = self
                .reader
                .read_bits(chunk_bits)
                .expect("the mantissa was proven to fit the live length");
            remaining -= chunk_bits;
            for j in 0..chunk_bits {
                if (chunk >> j) & 1 == 1 {
                    m.set_bit(remaining + j);
                }
            }
        }
        // One width-proportional record per wide value, exactly as the per-bit
        // wide fallback records.
        #[cfg(feature = "limb-meter")]
        super::limb_meter::record_wide(&m);
        super::scan::record_bits_u64(code_len);
        self.position += code_len;
        Ok(Int::from_base(Base::from(m - 1u32)))
    }
}

/// Native-order `u32` words over one stored stream's packed bytes: the word
/// source under the buffered reader.
///
/// The final partial word zero-fills past the stream's bytes (the tail byte's
/// dead bits arrive already masked through `bitvec`'s domain view), which
/// parallels the slice cursor's zero-filled decode window: the phantom zeros
/// can only lengthen an apparent unary prefix, and the cursor's live-length
/// checks keep them from ever surfacing in a decoded value. Reads past the last
/// byte-bearing word fail, so a truncated all-zero tail terminates instead of
/// reading zeros forever.
struct ByteWords<'a> {
    body: &'a [u8],
    /// The final partial byte, dead bits zeroed, if the stream has one.
    tail: Option<u8>,
    /// Byte offset of the next word's first byte.
    next: usize,
    /// Total stream bytes (`body` plus the tail byte if present).
    total: usize,
}

impl<'a> ByteWords<'a> {
    /// A word source whose first word begins at byte `start`.
    fn new(body: &'a [u8], tail: Option<u8>, start: usize) -> Self {
        ByteWords {
            body,
            tail,
            next: start,
            total: body.len() + usize::from(tail.is_some()),
        }
    }

    /// The stream byte at `idx`, zero past the end.
    fn byte_at(&self, idx: usize) -> u8 {
        if idx < self.body.len() {
            self.body[idx]
        } else if idx == self.body.len() {
            self.tail.unwrap_or(0)
        } else {
            0
        }
    }

    /// Gather the word at `self.next` in native byte order, zero-padded: the BE
    /// reader's `to_be()` then makes byte `next` most significant, which is
    /// exactly the stored form's MSB-first bit order.
    fn gather(&self) -> u32 {
        u32::from_ne_bytes([
            self.byte_at(self.next),
            self.byte_at(self.next + 1),
            self.byte_at(self.next + 2),
            self.byte_at(self.next + 3),
        ])
    }
}

/// The word source ran out of stream bytes.
#[derive(Debug)]
struct OutOfBytes;

impl Display for OutOfBytes {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("out of stream bytes")
    }
}

impl core::error::Error for OutOfBytes {}

impl WordRead for ByteWords<'_> {
    type Error = OutOfBytes;
    type Word = u32;

    fn read_word(&mut self) -> Result<u32, OutOfBytes> {
        if self.next >= self.total {
            return Err(OutOfBytes);
        }
        let word = self.gather();
        self.next += 4;
        Ok(word)
    }

    fn read_word_opt(&mut self) -> Option<u32> {
        if self.next >= self.total {
            return None;
        }
        let word = self.gather();
        self.next += 4;
        Some(word)
    }
}

#[cfg(test)]
mod tests;
