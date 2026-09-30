//! Mutable bit-stream storage.
//!
//! A [`BitsWriter`] is the stream's bytes beside a `u64` live bit length, under
//! two invariants:
//!
//! - the vector contains exactly `live.div_ceil(8)` bytes;
//! - unused bits in the final byte are zero.
//!
//! Consequently, equal buffers have equal bytes and finalization only
//! needs to append the marker.
//!
//! Lengths and bit positions use `u64` to avoid overflow on 32-bit platforms;
//! byte indexes use `usize`, since there can never be more than `usize::MAX`
//! bytes in memory on any platform.

use super::storage::Bits;
use super::BitsReader;
use crate::testing::instrument::scan;
use bytes::Bytes;
#[cfg(any(test, feature = "meter"))]
use num_bigint::BigUint;

/// Mutable bytes with an explicit live bit length.
///
/// Equality compares bit content bytewise because unused bits are always zero.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct BitsWriter {
    /// The live bits, most-significant bit first, with unused bits zeroed.
    bytes: Vec<u8>,
    /// The live bit length.
    ///
    /// Every stored bit occupies memory, so this cannot approach `u64::MAX`.
    live: u64,
}

impl BitsWriter {
    /// An empty buffer: no bits, no bytes, no allocation.
    pub(crate) fn new() -> Self {
        BitsWriter::default()
    }

    /// Append the canonical Elias-gamma code of `value`.
    #[cfg(any(test, feature = "meter"))]
    pub(crate) fn write_gamma(&mut self, value: &BigUint) {
        let mantissa = value + 1u32;
        self.push_zeros(mantissa.bits() - 1);
        self.push_magnitude(&mantissa);
    }

    /// An empty buffer with room for `bits` bits before reallocation.
    ///
    /// The capacity is a hint: a request past the target's address space
    /// allocates nothing up front, and the buffer still grows to whatever
    /// the pushes actually demand. A positive hint includes room for the
    /// stream's mandatory marker byte, so finalizing a byte-aligned stream at
    /// the hinted size does not reallocate.
    pub(crate) fn with_capacity(bits: u64) -> Self {
        let bytes = if bits == 0 {
            0
        } else {
            usize::try_from(bits / 8 + 1).unwrap_or(0)
        };
        BitsWriter {
            bytes: Vec::with_capacity(bytes),
            live: 0,
        }
    }

    /// A buffer of `len` copies of `bit`.
    pub(crate) fn repeat(bit: bool, len: u64) -> Self {
        let whole = usize::try_from(len.div_ceil(8)).expect("a repeated buffer is allocatable");
        let mut this = BitsWriter {
            bytes: vec![if bit { 0xFF } else { 0x00 }; whole],
            live: len,
        };
        this.mask_tail();
        this
    }

    /// The live bit length.
    pub fn len(&self) -> u64 {
        self.live
    }

    /// Whether the buffer holds no bits at all.
    pub fn is_empty(&self) -> bool {
        self.live == 0
    }

    /// The bytes containing the live bits.
    pub fn as_raw_slice(&self) -> &[u8] {
        &self.bytes
    }

    /// Read the bits written so far from the beginning.
    pub(crate) fn reader(&self) -> BitsReader<'_> {
        BitsReader::with_len(&self.bytes, self.live)
    }

    /// Add canonical marker padding and return the encoded bytes.
    ///
    /// Production domain writers use [`finalize`](Self::finalize), which
    /// preserves the [`Bits`] abstraction. Instruments use this form when they
    /// deliberately need raw encoded bytes.
    #[cfg(any(test, feature = "meter"))]
    pub(crate) fn into_padded_bytes(mut self) -> Vec<u8> {
        self.seal_padding();
        self.bytes
    }

    /// The bit at `pos`.
    ///
    /// # Panics
    ///
    /// `pos` must be below the live length.
    pub(crate) fn bit(&self, pos: u64) -> bool {
        assert!(pos < self.live, "bit read past the buffer's live length");
        self.bytes[(pos / 8) as usize] >> (7 - pos % 8) & 1 == 1
    }

    /// Overwrite the bit at `pos`.
    ///
    /// # Panics
    ///
    /// `pos` must be below the live length.
    pub(crate) fn patch_bit(&mut self, pos: u64, bit: bool) {
        assert!(pos < self.live, "bit write past the buffer's live length");
        let mask = 0x80 >> (pos % 8);
        if bit {
            self.bytes[(pos / 8) as usize] |= mask;
        } else {
            self.bytes[(pos / 8) as usize] &= !mask;
        }
    }

    /// Append one bit.
    pub(crate) fn push(&mut self, bit: bool) {
        scan::record_bits(1);
        self.append_bit(bit);
    }

    /// Append one bit without recording a second scan charge.
    fn append_bit(&mut self, bit: bool) {
        let within = (self.live % 8) as u32;
        if within == 0 {
            self.bytes.push(if bit { 0x80 } else { 0x00 });
        } else if bit {
            // The target bit is zero (the invariant), so setting it is one OR.
            *self.bytes.last_mut().expect("a partial byte exists") |= 0x80 >> within;
        }
        self.live += 1;
    }

    /// Append the low `len <= 64` bits of `value`, most-significant first.
    pub(crate) fn push_bits(&mut self, value: u64, len: u32) {
        debug_assert!(len <= 64, "an append stages at most one machine word");
        debug_assert!(
            len == 64 || value >> len == 0,
            "append value has bits above its stated width"
        );
        scan::record_bits(len as usize);
        self.append_bits(value, len);
    }

    /// Append one word without recording a second scan charge.
    fn append_bits(&mut self, value: u64, len: u32) {
        if len == 0 {
            return;
        }
        let within = (self.live % 8) as u32;
        if within + len <= 8 {
            // Tree tags and flags usually fit in the current byte. Keep that
            // byte in place instead of rebuilding it through the general
            // two-word staging path.
            let byte = (value as u8) << (8 - within - len);
            if within == 0 {
                self.bytes.push(byte);
            } else {
                *self.bytes.last_mut().expect("a partial byte exists") |= byte;
            }
            self.live += u64::from(len);
            return;
        }
        // Reload the partial tail byte's live bits, merge in a double-word
        // register, and write back whole bytes plus the new (zero-padded)
        // partial byte.
        let staged = if within == 0 {
            0
        } else {
            u64::from(self.bytes.pop().expect("a partial byte exists") >> (8 - within))
        };
        let total = within + len;
        let acc = (u128::from(staged) << len) | u128::from(value);
        let aligned = (acc << (128 - total)).to_be_bytes();
        let whole = (total / 8) as usize;
        self.bytes.extend_from_slice(&aligned[..whole]);
        if !total.is_multiple_of(8) {
            // The next byte carries the remaining bits at its top and zeros
            // below: the dead-bits invariant by construction.
            self.bytes.push(aligned[whole]);
        }
        self.live += u64::from(len);
    }

    /// Append `len` zero bits.
    #[cfg(any(test, feature = "meter"))]
    fn push_zeros(&mut self, mut len: u64) {
        while len >= u64::from(u64::BITS) {
            self.push_bits(0, u64::BITS);
            len -= u64::from(u64::BITS);
        }
        self.push_bits(0, len as u32);
    }

    /// Append a magnitude's binary digits, most-significant first.
    #[cfg(any(test, feature = "meter"))]
    fn push_magnitude(&mut self, value: &BigUint) {
        let bits = value.bits();
        if bits == 0 {
            return;
        }
        let mut words = value.iter_u64_digits().rev();
        let top = words.next().expect("a nonzero magnitude has one word");
        let top_len = ((bits - 1) % u64::from(u64::BITS) + 1) as u32;
        self.push_bits(top, top_len);
        for word in words {
            self.push_bits(word, u64::BITS);
        }
    }

    /// Read `len <= 63` written bits at `start`, right-aligned in a word.
    ///
    /// Domain writers use this to inspect a small header that they reserved
    /// earlier. The range must lie within the current output.
    pub(crate) fn read_word(&self, start: u64, len: u32) -> u64 {
        assert!(
            len <= 63 && start + u64::from(len) <= self.len(),
            "word read lies within the writer"
        );
        scan::record_bits(len as usize);
        let mut value = 0;
        for position in start..start + u64::from(len) {
            value = value << 1 | u64::from(self.bit(position));
        }
        value
    }

    /// Append `width` zero bits and return the start of the reserved range.
    pub(crate) fn reserve(&mut self, width: usize) -> u64 {
        let start = self.len();
        let mut remaining = width;
        while remaining > 0 {
            let chunk = remaining.min(u64::BITS as usize) as u32;
            self.push_bits(0, chunk);
            remaining -= chunk as usize;
        }
        start
    }

    /// Copy `start..end` from sealed storage onto the output.
    pub(crate) fn splice(&mut self, source: &Bits, start: u64, end: u64) {
        debug_assert!(
            end <= source.reader().len(),
            "copied range lies within the source's live bits"
        );
        self.splice_storage(source.as_raw_slice(), start, end);
    }

    /// Copy a range from another unfinished writer.
    pub(crate) fn splice_writer(&mut self, source: &BitsWriter, start: u64, end: u64) {
        debug_assert!(end <= source.len(), "copied range lies within the writer");
        self.splice_storage(source.as_raw_slice(), start, end);
    }

    /// Append another buffer's live bits.
    #[cfg(test)]
    pub(crate) fn extend_from_writer(&mut self, other: &BitsWriter) {
        self.splice_storage(other.as_raw_slice(), 0, other.len());
    }

    /// Discard every bit at or after `len` and zero the new unused tail.
    ///
    /// # Panics
    ///
    /// Panics if `len` exceeds the current length: truncation only ever
    /// shortens.
    pub(crate) fn truncate(&mut self, len: u64) {
        assert!(
            len <= self.live,
            "buffer truncation target {len} exceeds the {} bits held",
            self.live,
        );
        self.bytes.truncate(len.div_ceil(8) as usize);
        self.live = len;
        self.mask_tail();
    }

    /// Seal the completed stream into immutable canonical storage.
    pub(crate) fn finalize(mut self) -> Bits {
        self.seal_padding();
        Bits::from_canonical(Bytes::from(self.bytes))
    }

    /// Append the canonical marker bit after the live stream.
    fn seal_padding(&mut self) {
        if !self.is_empty() {
            self.append_bit(true);
        }
        debug_assert!(
            self.tail_is_zeroed(),
            "a sealed stream's dead bits are zero: the writer invariant"
        );
    }

    /// Re-establish the zeroed-dead-bits invariant on the final partial
    /// byte, after a truncation exposed formerly live bits as dead.
    fn mask_tail(&mut self) {
        let within = (self.live % 8) as u32;
        if within != 0 {
            *self.bytes.last_mut().expect("a partial byte exists") &= 0xFF << (8 - within);
        }
    }

    /// Whether the final partial byte's dead bits are zero: the invariant,
    /// as a probe for the debug asserts.
    fn tail_is_zeroed(&self) -> bool {
        let within = (self.live % 8) as u32;
        within == 0 || self.bytes.last().is_some_and(|b| b & (0xFF >> within) == 0)
    }

    /// Append whole bytes: a `memcpy` when the live length is
    /// byte-aligned, a two-shift merge per byte otherwise.
    fn extend_bytes(&mut self, body: &[u8]) {
        let within = (self.live % 8) as u32;
        if within == 0 {
            self.bytes.extend_from_slice(body);
        } else {
            self.bytes.reserve(body.len());
            for &b in body {
                *self.bytes.last_mut().expect("a partial byte exists") |= b >> within;
                // The shift zero-fills below the carried bits: the dead-bits
                // invariant by construction.
                self.bytes.push(b << (8 - within));
            }
        }
        self.live += body.len() as u64 * 8;
    }

    /// Copy one range from raw storage.
    fn splice_storage(&mut self, bytes: &[u8], start: u64, end: u64) {
        assert!(
            start <= end && end <= bytes.len() as u64 * 8,
            "copied range lies within the source storage"
        );
        scan::record_bits_u64(end - start);
        let mut position = start;
        while position < end && !position.is_multiple_of(8) {
            self.append_bit(bit(bytes, position));
            position += 1;
        }
        let whole = ((end - position) / 8) as usize;
        if whole > 0 {
            let byte = (position / 8) as usize;
            self.extend_bytes(&bytes[byte..byte + whole]);
            position += whole as u64 * 8;
        }
        while position < end {
            self.append_bit(bit(bytes, position));
            position += 1;
        }
    }
}

/// Renders the live bits most-significant-first as `0`/`1`, the test
/// suites' failure-message spelling.
impl core::fmt::Debug for BitsWriter {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("BitsWriter[")?;
        for pos in 0..self.live {
            f.write_str(if self.bit(pos) { "1" } else { "0" })?;
        }
        f.write_str("]")
    }
}

/// Collect bits into a buffer, oldest first: the test generators'
/// construction form.
impl FromIterator<bool> for BitsWriter {
    fn from_iter<I: IntoIterator<Item = bool>>(iter: I) -> Self {
        let mut out = BitsWriter::new();
        out.extend(iter);
        out
    }
}

/// Append bits oldest-first: [`FromIterator`]'s in-place form.
impl Extend<bool> for BitsWriter {
    fn extend<I: IntoIterator<Item = bool>>(&mut self, iter: I) {
        for bit in iter {
            self.push(bit);
        }
    }
}

/// A [`BitsWriter`] literal for the test suites: `bits_writer![1, 0, 1]` builds
/// from listed bits, `bits_writer![1; 8]` repeats one.
#[cfg(test)]
macro_rules! bits_writer {
    ($bit:literal; $n:expr) => {
        $crate::bits::BitsWriter::repeat($bit != 0, $n)
    };
    ($($bit:literal),+ $(,)?) => {
        [$($bit != 0),+]
            .into_iter()
            .collect::<$crate::bits::BitsWriter>()
    };
}
#[cfg(test)]
pub(crate) use bits_writer;

/// Read one bit from raw storage.
fn bit(bytes: &[u8], position: u64) -> bool {
    bytes[(position / 8) as usize] >> (7 - position % 8) & 1 == 1
}
