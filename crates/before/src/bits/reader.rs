//! Word-parallel bit, unary, and Elias-gamma reads.
//!
//! [`BitsReader`] has the same behavior as the per-bit [`BitRead`] loop but
//! reads unary runs and small integer codes a word at a time. Explicit bounds
//! prevent the reader's zero fill from becoming input data.
//!
//! Integer values may exceed 64 bits, so the reader uses the crate's decoder
//! instead of `dsi-bitstream`'s bounded gamma decoder.
//!
//! [`read_gamma`](BitsReader::read_gamma) handles short codes by table or machine
//! word and longer codes with [`BigUint`].

use dsi_bitstream::impls::BufBitReader;
use dsi_bitstream::traits::{BitRead as DsiBitRead, BE};
use num_bigint::BigUint;

use crate::error::Decode;
use crate::testing::instrument::scan;

use super::Bits;

mod gamma;
mod words;

#[cfg(test)]
mod reference;

#[cfg(test)]
pub(crate) use reference::ReferenceBitsReader;
use words::ByteWords;

/// A lightweight error used when an in-memory stream ends early.
///
/// It converts to [`Decode::Truncated`] only on failure, avoiding a richer
/// error value on every successful bit read.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Truncated;

impl From<Truncated> for Decode {
    fn from(_: Truncated) -> Self {
        Decode::Truncated
    }
}

/// Reads an encoded bit stream from left to right.
///
/// Both the in-memory reader and the streaming Borsh decoder implement this
/// interface so the validators and gamma decoder have one sequential input
/// contract.
pub(crate) trait BitRead {
    /// The error returned by a failed bit read.
    type Error: Into<Decode>;

    /// Read the next bit.
    fn read_bit(&mut self) -> Result<bool, Self::Error>;

    /// Position immediately after the last bit read.
    fn position(&self) -> u64;

    /// Consume a run of zero bits and its terminating one, returning the
    /// number of zeros.
    fn read_unary(&mut self) -> Result<u64, Self::Error> {
        let mut zeros = 0u64;
        while !self.read_bit()? {
            zeros += 1;
        }
        Ok(zeros)
    }

    /// Read one Elias-gamma-coded integer at the current position.
    ///
    /// Implementations may replace this bitwise default with an equivalent
    /// word-parallel decoder.
    fn read_gamma(&mut self) -> Result<BigUint, Decode>
    where
        Self: Sized,
        Decode: From<Self::Error>,
    {
        self.read_gamma_slow()
    }

    /// Decode one Elias-gamma integer through the bitwise reference path.
    ///
    /// Optimized readers use this when a word window cannot prove the complete
    /// code. Keeping rejection here makes the fast path conservative by
    /// construction.
    fn read_gamma_slow(&mut self) -> Result<BigUint, Decode>
    where
        Self: Sized,
        Decode: From<Self::Error>,
    {
        gamma::decode_slow(self)
    }
}

/// A word-parallel sequential reader over a bounded bit range.
///
/// Domain readers build their tree operations on this cursor. Every operation
/// records the same scan-meter bits as an equivalent per-bit loop: the meter
/// prices the input examined, not how reads are batched internally.
pub(crate) struct BitsReader<'a> {
    reader: BufBitReader<BE, ByteWords<'a>>,
    /// The position immediately after the last live bit read.
    ///
    /// `u64` keeps every bit in a large 32-bit allocation representable.
    position: u64,
    /// The stream's live bit length, in the same `u64` denomination.
    len: u64,
}

impl<'a> BitsReader<'a> {
    /// Read sealed canonical storage starting at `position`.
    pub(crate) fn at(bits: &'a Bits, position: u64) -> Self {
        Self::from_storage(bits.as_raw_slice(), live_len(bits.as_raw_slice()), position)
    }

    /// Read every bit in a byte slice.
    ///
    /// Decoding may pass a whole padded input; the marker is checked after the
    /// value has been read.
    pub(crate) fn from_bytes(bytes: &'a [u8]) -> Self {
        Self::from_storage(bytes, bytes.len() as u64 * 8, 0)
    }

    /// Read the first `len` bits of a byte slice.
    pub(crate) fn with_len(bytes: &'a [u8], len: u64) -> Self {
        Self::from_storage(bytes, len, 0)
    }

    /// Number of bits in the readable range.
    pub(crate) fn len(&self) -> u64 {
        self.len
    }

    /// Read at `position` within the first `len` bits of a byte slice.
    ///
    /// `O(1)`: the word source starts at `pos`'s byte and the cursor discards
    /// the at most 7 leading bits before `pos` unrecorded (the walk never
    /// examines them).
    ///
    /// # Panics
    ///
    /// Panics if `position` lies past the readable range.
    pub(super) fn from_storage(bytes: &'a [u8], len: u64, position: u64) -> Self {
        assert!(len <= bytes.len() as u64 * 8, "live bits within storage");
        assert!(position <= len, "reader opened past the stream's end");
        let (body, tail) = body_tail(bytes, len);
        let mut reader = BufBitReader::new(ByteWords::new(body, tail, (position / 8) as usize));
        let skip = (position % 8) as usize;
        if skip != 0 {
            reader
                .skip_bits(skip)
                .expect("a mid-byte start has at least its own byte to skip within");
        }
        BitsReader {
            reader,
            position,
            len,
        }
    }

    /// Decode a gamma code from one already-buffered word when it fits whole.
    ///
    /// `None` means the sequential reader must decide the input. This includes
    /// every truncated code and every code wider than one word.
    #[cfg(any(test, feature = "borsh"))]
    pub(crate) fn gamma_from_window(bytes: &[u8], len: u64, position: u64) -> Option<(u64, u64)> {
        gamma::from_window(bytes, len, position)
    }

    /// Read `len <= 64` bits at the current position as a right-aligned word.
    pub(crate) fn read_word(&mut self, len: u32) -> Result<u64, Truncated> {
        assert!(len <= 64, "one word contains at most 64 bits");
        if self.position + u64::from(len) > self.len {
            return Err(self.truncated());
        }
        let value = self
            .reader
            .read_bits(len as usize)
            .expect("the requested word fits the live range");
        scan::record_bits(u64::from(len));
        self.position += u64::from(len);
        Ok(value)
    }

    /// Read the unary prefix without recording a successful run:
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
    /// leaves its own reader.
    fn truncated(&mut self) -> Truncated {
        scan::record_bits(self.len - self.position);
        self.position = self.len;
        Truncated
    }

    /// Skip one Elias-gamma-coded integer without materializing
    /// its value; `Truncated` exactly where [`read_gamma`](BitRead::read_gamma)
    /// would be.
    ///
    /// The prefix length alone determines the code's width, so the skip is one
    /// unary read plus a bit discard; it records the same scan-meter bits a
    /// read would.
    pub(crate) fn skip_gamma(&mut self) -> Result<(), Truncated> {
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
        scan::record_bits(code_len);
        self.position += code_len;
        Ok(())
    }
}

/// Split `live` bits into whole bytes and an optional masked tail byte.
fn body_tail(bytes: &[u8], live: u64) -> (&[u8], Option<u8>) {
    assert!(live <= bytes.len() as u64 * 8, "live bits within storage");
    let whole = (live / 8) as usize;
    let rem = (live % 8) as u32;
    if rem == 0 {
        (&bytes[..whole], None)
    } else {
        (&bytes[..whole], Some(bytes[whole] & !(0xFF >> rem)))
    }
}

/// Recover the live length from canonical marker padding.
fn live_len(bytes: &[u8]) -> u64 {
    match bytes.last() {
        None => 0,
        Some(&last) => {
            debug_assert!(last != 0, "stored stream missing its padding marker");
            bytes.len() as u64 * 8 - 1 - u64::from(last.trailing_zeros())
        }
    }
}

impl BitRead for BitsReader<'_> {
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
        // Record the logical bit examined, independent of the word-sized
        // reads used underneath.
        scan::record_bits(1);
        self.position += 1;
        Ok(bit)
    }

    fn position(&self) -> u64 {
        self.position
    }

    fn read_unary(&mut self) -> Result<u64, Truncated> {
        let k = self.unary_raw()?;
        scan::record_bits(k + 1);
        self.position += k + 1;
        Ok(k)
    }

    /// Read one Elias-gamma-coded integer: accepting and rejecting on exactly
    /// the same inputs as the bitwise reference decoder
    /// over this cursor.
    ///
    /// Short codes use a lookup table, word-sized codes decode into a `u64`,
    /// and wider codes fill a [`BigUint`] one word at a time. Each fast path is
    /// taken only after proving that the complete code lies inside the live
    /// range, so truncation agrees with the bitwise reference decoder.
    fn read_gamma(&mut self) -> Result<BigUint, Decode> {
        gamma::decode(self)
    }
}

#[cfg(test)]
mod tests;
