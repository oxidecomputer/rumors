//! Sequential access to canonical packed bits.

use crate::error::Decode;

use super::BitsSlice;

/// The bit stream ended before the requested bit.
///
/// The per-bit failure type of [`SliceCursor`]. It is fieldless and `Copy`
/// where [`Decode`] is not: `Decode` carries an `Io(std::io::Error)` variant,
/// so it has drop glue, and using it as the per-bit error meant constructing
/// and dropping a `Decode` on every *successful* bit read (an `ok_or`
/// argument is evaluated unconditionally) — one glue call per bit, in every
/// gamma decode, in every version comparison. With this marker the per-bit
/// `Result<bool, Truncated>` is glue-free; the `?` in the decode loops
/// converts it (via [`From`]) to [`Decode::Truncated`] only on the failure
/// branch.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Truncated;

impl From<Truncated> for Decode {
    fn from(_: Truncated) -> Self {
        Decode::Truncated
    }
}

/// A cursor which yields canonical encoding bits from left to right.
///
/// `Error` is the per-bit failure type. The decode loops are generic over it,
/// bounded by `Decode: From<C::Error>`, and convert with `?` — so the cost of
/// a rich error is paid only by cursors that can actually fail richly.
/// [`SliceCursor`] uses the fieldless [`Truncated`], keeping the hot per-bit
/// path free of [`Decode`] construction; the wire-side `ReaderCursor` (in
/// `borsh_impls`) uses `Decode` itself, because a failed read must carry the
/// underlying [`Decode::Io`] error.
pub(crate) trait BitCursor {
    /// The per-bit failure type; see the trait docs for how to choose it.
    type Error: Into<Decode>;

    /// Read the next bit.
    fn read_bit(&mut self) -> Result<bool, Self::Error>;

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
    type Error = Truncated;

    fn read_bit(&mut self) -> Result<bool, Truncated> {
        // `ok_or`'s eager argument is fine here: `Truncated` is a ZST.
        let bit = *self.bits.get(self.position).ok_or(Truncated)?;
        self.position += 1;
        Ok(bit)
    }

    fn position(&self) -> usize {
        self.position
    }
}
