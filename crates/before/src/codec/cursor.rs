//! Sequential access to canonical packed bits.

use crate::error::Decode;

use super::{decode_int_from, gamma, Base, BitsSlice};

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

    /// Read one Elias-gamma-coded integer starting at the cursor.
    ///
    /// The provided default is the per-bit loop ([`decode_int_from`]); a
    /// cursor with cheap access to its byte-backed window overrides it to
    /// route through the word decoder ([`gamma::decode_int_window`]), which
    /// reads a whole code in `O(1)` words. Both cursors override:
    /// [`SliceCursor`] windows over its whole slice, and the wire-side
    /// `ReaderCursor` (in `borsh_impls`) windows over the bytes it has
    /// already pulled from its reader — never bytes beyond them — falling
    /// back to this loop whenever the window cannot prove a whole code.
    fn read_int(&mut self) -> Result<Base, Decode>
    where
        Self: Sized,
        Decode: From<Self::Error>,
    {
        decode_int_from(self)
    }
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
        // One live bit scanned: this cursor is the sequential read primitive
        // under `decode`, the gamma decoder, and the skyline
        // validator/decoder, so the scan meter records here once for all of
        // them.
        super::scan::record_bits(1);
        self.position += 1;
        Ok(bit)
    }

    fn position(&self) -> usize {
        self.position
    }

    fn read_int(&mut self) -> Result<Base, Decode> {
        // Word fast path over the slice; anything the window cannot prove —
        // every reject included — is decided by the default per-bit loop, so
        // the two paths accept and reject identically by construction.
        if let Some((n, next)) = gamma::decode_int_window(self.bits, self.position) {
            self.position = next;
            return Ok(Base::from(n));
        }
        decode_int_from(self)
    }
}
