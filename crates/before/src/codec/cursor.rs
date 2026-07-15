//! Sequential access to canonical packed bits.

use crate::error::Decode;

use super::BitsSlice;

/// A cursor which yields canonical encoding bits from left to right.
pub(crate) trait BitCursor {
    /// Read the next bit.
    fn read_bit(&mut self) -> Result<bool, Decode>;

    /// The position immediately after the last bit read.
    fn position(&self) -> usize;
}

/// A sequential cursor over an existing packed bit slice.
pub(crate) struct SliceCursor<'a> {
    bits: &'a BitsSlice,
    position: usize,
}

impl<'a> SliceCursor<'a> {
    pub(crate) fn new(bits: &'a BitsSlice, position: usize) -> Self {
        SliceCursor { bits, position }
    }
}

impl BitCursor for SliceCursor<'_> {
    fn read_bit(&mut self) -> Result<bool, Decode> {
        let bit = *self.bits.get(self.position).ok_or(Decode::Truncated)?;
        self.position += 1;
        Ok(bit)
    }

    fn position(&self) -> usize {
        self.position
    }
}
