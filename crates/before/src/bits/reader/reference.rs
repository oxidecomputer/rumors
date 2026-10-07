//! Bitwise reader used as the optimized reader's test oracle.

use num_bigint::BigUint;

use super::{BitRead, BitsReader, Truncated};
use crate::error::Decode;
use crate::testing::instrument::scan;

/// A deliberately bit-at-a-time bounded reader.
pub(crate) struct ReferenceBitsReader<'a> {
    /// Source bytes.
    bytes: &'a [u8],
    /// Number of readable bits.
    len: u64,
    /// Position immediately after the last bit read.
    position: u64,
}

impl<'a> ReferenceBitsReader<'a> {
    /// Open at `position` within the first `len` source bits.
    pub(crate) fn new(bytes: &'a [u8], len: u64, position: u64) -> Self {
        assert!(position <= len && len <= bytes.len() as u64 * 8);
        Self {
            bytes,
            len,
            position,
        }
    }
}

impl BitRead for ReferenceBitsReader<'_> {
    type Error = Truncated;

    fn read_bit(&mut self) -> Result<bool, Truncated> {
        if self.position == self.len {
            return Err(Truncated);
        }
        let bit = self.bytes[(self.position / 8) as usize] >> (7 - self.position % 8) & 1 == 1;
        scan::record_bits(1);
        self.position += 1;
        Ok(bit)
    }

    fn position(&self) -> u64 {
        self.position
    }

    fn read_gamma(&mut self) -> Result<BigUint, Decode> {
        if let Some((value, next)) =
            BitsReader::gamma_from_window(self.bytes, self.len, self.position)
        {
            scan::record_bits(next - self.position);
            self.position = next;
            return Ok(BigUint::from(value));
        }
        self.read_gamma_slow()
    }
}
