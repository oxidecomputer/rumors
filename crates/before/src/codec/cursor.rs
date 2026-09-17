//! Sequential access to encoded bits.

use num_bigint::BigUint;

use crate::error::Decode;

use super::{gamma, BitsView};

/// A lightweight error used when an in-memory stream ends early.
///
/// It converts to [`Decode::Truncated`] only on the failure path, avoiding a
/// richer error value on every successful bit read.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Truncated;

impl From<Truncated> for Decode {
    fn from(_: Truncated) -> Self {
        Decode::Truncated
    }
}

/// Reads an encoded bit stream from left to right.
pub(crate) trait BitCursor {
    /// The error returned by a failed bit read.
    type Error: Into<Decode>;

    /// Read the next bit.
    fn read_bit(&mut self) -> Result<bool, Self::Error>;

    /// The position immediately after the last bit read.
    ///
    /// Positions use `u64` so large 32-bit buffers remain representable.
    fn position(&self) -> u64;

    /// Consume a run of `false` bits and its terminating `true`, returning the
    /// number of `false` bits.
    fn read_unary(&mut self) -> Result<u64, Self::Error> {
        let mut k = 0u64;
        while !self.read_bit()? {
            k += 1;
        }
        Ok(k)
    }

    /// Read one Elias-gamma-coded integer starting at the cursor.
    ///
    /// The default implementation decodes one bit at a time. Cursors may
    /// override it with an equivalent word-at-a-time implementation.
    fn read_int(&mut self) -> Result<BigUint, Decode>
    where
        Self: Sized,
        Decode: From<Self::Error>,
    {
        gamma::decode_from(self)
    }
}

/// A sequential cursor over an in-memory bit view.
pub(crate) struct SliceCursor<'a> {
    bits: BitsView<'a>,
    /// The position immediately after the last bit read.
    ///
    /// Uses the view's `u64` position width.
    position: u64,
}

impl<'a> SliceCursor<'a> {
    pub(crate) fn new(bits: BitsView<'a>, position: u64) -> Self {
        SliceCursor { bits, position }
    }
}

impl BitCursor for SliceCursor<'_> {
    type Error = Truncated;

    fn read_bit(&mut self) -> Result<bool, Truncated> {
        // `ok_or`'s eager argument is fine here: `Truncated` is a ZST.
        let bit = self.bits.get(self.position).ok_or(Truncated)?;
        // One live bit scanned: this cursor is the sequential read primitive
        // under the id-tree parsers and the per-bit gamma decode path, so the
        // scan meter records here once for both. The skyline kernels read
        // through `DsiCursor`, which carries its own records.
        super::scan::record_bits(1);
        self.position += 1;
        Ok(bit)
    }

    fn position(&self) -> u64 {
        self.position
    }

    fn read_int(&mut self) -> Result<BigUint, Decode> {
        // Word fast path over the view; anything the window cannot prove —
        // every reject included — is decided by the default per-bit loop, so
        // the two paths accept and reject identically by construction.
        if let Some((n, next)) = gamma::decode_window(self.bits, self.position) {
            // The window proves the same `2k + 1` code bits the per-bit loop
            // reads one at a time, so it records the same count: the scan meter
            // prices work by bits examined, not by how the examining path
            // batches them.
            super::scan::record_bits_u64(next - self.position);
            self.position = next;
            return Ok(BigUint::from(n));
        }
        gamma::decode_from(self)
    }
}
