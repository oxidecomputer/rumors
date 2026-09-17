//! Canonical bit-stream storage.
//!
//! [`Bits`] owns immutable bytes and [`BitsView`] borrows their live prefix.
//! A nonempty stream ends with one marker bit followed by zero padding; the
//! empty stream has no bytes. This makes the live length recoverable and gives
//! every stream exactly one byte encoding.

#![allow(rustdoc::private_intra_doc_links)]

use core::fmt;
use core::hash::Hasher;

use bytes::Bytes;

use super::buf::{seal_padding, BitsBuf};
use crate::error::Decode;

/// Immutable canonical bytes for a party or version.
///
/// Clones share the refcounted byte buffer. [`live`](Self::live) excludes the
/// marker and padding.
#[derive(Clone)]
pub struct Bits {
    /// Live bits followed by the marker and zero padding.
    bytes: Bytes,
}

/// Writes the live stream as binary, excluding its storage padding.
impl fmt::Binary for Bits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.write_str("0b")?;
        }
        let bits = self.live();
        for pos in 0..bits.len() {
            f.write_str(if bits.bit(pos) { "1" } else { "0" })?;
        }
        Ok(())
    }
}

impl Bits {
    /// The frozen empty stream: no bits, no bytes, no allocation.
    pub(crate) fn empty() -> Self {
        Bits {
            bytes: Bytes::new(),
        }
    }

    /// Add canonical padding and adopt a mutable buffer without copying.
    pub(crate) fn freeze(mut buf: BitsBuf) -> Self {
        seal_padding(&mut buf);
        Bits {
            bytes: Bytes::from(buf.into_bytes()),
        }
    }

    /// Adopt bytes whose canonical padding has already been validated.
    ///
    /// `bytes` must be marker-padded — the live bits, one `1`, then zeros to
    /// the byte boundary; empty for the empty stream — which is what
    /// `require_marker_padding` accepts. Debug builds assert it; release builds
    /// trust the validator.
    ///
    pub(crate) fn from_canonical(bytes: Bytes) -> Self {
        let bits = Bits { bytes };
        debug_assert!(
            padding_is_canonical(&bits),
            "from_canonical: the buffer must end in the canonical `1 0*` padding",
        );
        bits
    }

    /// The live bit length, recovered from the padding in `O(1)`.
    pub fn len(&self) -> u64 {
        match self.bytes.last() {
            None => 0,
            Some(&last) => {
                debug_assert!(last != 0, "stored stream missing its padding marker");
                self.bytes.len() as u64 * 8 - 1 - u64::from(last.trailing_zeros())
            }
        }
    }

    /// Borrow the live bits without the marker or padding.
    pub fn live(&self) -> BitsView<'_> {
        BitsView {
            bytes: &self.bytes,
            live: self.len(),
        }
    }

    /// Whether the stream holds no bits at all.
    ///
    /// This tests storage emptiness; an empty `Version` still has live bits.
    #[cfg(any(test, feature = "meter"))]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// The canonical marker-padded bytes: the wire encoding, borrowed
    /// without copying.
    pub fn as_raw_slice(&self) -> &[u8] {
        &self.bytes
    }

    /// Whether two streams share the same byte buffer.
    ///
    /// This implies equality but is not required for equality.
    pub(crate) fn ptr_eq(&self, other: &Bits) -> bool {
        self.bytes.len() == other.bytes.len() && self.bytes.as_ptr() == other.bytes.as_ptr()
    }
}

/// A borrowed byte slice and its live bit length.
///
/// Views start at the first byte. Ranges within a view are represented by bit
/// positions rather than by creating unaligned views.
#[derive(Clone, Copy)]
pub struct BitsView<'a> {
    /// The bytes holding the live bits (and possibly padding past them).
    bytes: &'a [u8],
    /// The live bit length: at most `8 · bytes.len()`.
    live: u64,
}

impl<'a> BitsView<'a> {
    /// A view of the first `live` bits of `bytes`.
    ///
    /// # Panics
    ///
    /// `live` must be at most `8 · bytes.len()`.
    pub(crate) fn new(bytes: &'a [u8], live: u64) -> Self {
        assert!(
            live <= bytes.len() as u64 * 8,
            "live bits within the buffer"
        );
        BitsView { bytes, live }
    }

    /// View every bit in a byte slice, including any encoded padding.
    pub(crate) fn whole(bytes: &'a [u8]) -> Self {
        BitsView {
            bytes,
            live: bytes.len() as u64 * 8,
        }
    }

    /// The empty view: no bits, no bytes.
    pub(crate) fn empty() -> Self {
        BitsView {
            bytes: &[],
            live: 0,
        }
    }

    /// The live bit length.
    pub fn len(&self) -> u64 {
        self.live
    }

    /// Whether the view holds no bits at all.
    pub fn is_empty(&self) -> bool {
        self.live == 0
    }

    /// The bit at `pos`, or `None` at or past the live length.
    pub(crate) fn get(&self, pos: u64) -> Option<bool> {
        if pos >= self.live {
            return None;
        }
        // `pos / 8` indexes an allocated buffer, so it fits `usize`.
        let byte = self.bytes[(pos / 8) as usize];
        Some(byte >> (7 - pos % 8) & 1 == 1)
    }

    /// The bit at `pos`.
    ///
    /// # Panics
    ///
    /// `pos` must be below the live length.
    pub(crate) fn bit(&self, pos: u64) -> bool {
        assert!(pos < self.live, "bit read past the view's live length");
        let byte = self.bytes[(pos / 8) as usize];
        byte >> (7 - pos % 8) & 1 == 1
    }

    /// Load `len <= 64` live bits at `start`, right-aligned in the result.
    pub(crate) fn load_be(&self, start: u64, len: u32) -> u64 {
        debug_assert!(
            len <= 64 && start + u64::from(len) <= self.live,
            "loaded range within the view's live length"
        );
        if len == 0 {
            return 0;
        }
        let byte = (start / 8) as usize;
        let shift = (start % 8) as u32;
        let mut buf = [0u8; 9];
        let end = (byte + buf.len()).min(self.bytes.len());
        buf[..end - byte].copy_from_slice(&self.bytes[byte..end]);
        let word = u64::from_be_bytes(buf[..8].try_into().expect("buffer has eight bytes"));
        let window = if shift == 0 {
            word
        } else {
            (word << shift) | (u64::from(buf[8]) >> (8 - shift))
        };
        window >> (64 - len)
    }

    /// Split the live data into whole bytes and an optional masked tail byte.
    pub(crate) fn body_tail(&self) -> (&'a [u8], Option<u8>) {
        let whole = (self.live / 8) as usize;
        let rem = (self.live % 8) as u32;
        if rem == 0 {
            (&self.bytes[..whole], None)
        } else {
            (
                &self.bytes[..whole],
                Some(self.bytes[whole] & !(0xFF >> rem)),
            )
        }
    }

    /// The complete underlying byte slice.
    pub(crate) fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    /// Copy the live bits into a mutable buffer.
    #[cfg(test)]
    pub(crate) fn to_buf(self) -> BitsBuf {
        let mut out = BitsBuf::with_capacity(self.live);
        super::buf::extend_from_view(&mut out, self, 0, self.live);
        out
    }

    /// Whether two views cover the same memory and live length.
    pub(crate) fn ptr_eq(&self, other: &BitsView<'_>) -> bool {
        self.bytes.as_ptr() == other.bytes.as_ptr() && self.live == other.live
    }
}

/// Compare the canonical bytes, with a shared-buffer fast path.
impl PartialEq for Bits {
    fn eq(&self, other: &Self) -> bool {
        canonical_eq(self, other)
    }
}

impl Eq for Bits {}

/// Compare two canonical streams by shared identity, then by bytes.
pub(crate) fn canonical_eq(a: &Bits, b: &Bits) -> bool {
    debug_assert!(
        padding_is_canonical(a) && padding_is_canonical(b),
        "canonical_eq compares raw bytes: both operands must be marker-padded",
    );
    a.ptr_eq(b) || a.as_raw_slice() == b.as_raw_slice()
}

/// Hash the same canonical bytes that equality compares.
pub(crate) fn canonical_hash<H: Hasher>(bits: &Bits, state: &mut H) {
    use core::hash::Hash;
    debug_assert!(
        padding_is_canonical(bits),
        "canonical_hash reads raw bytes: the operand must be marker-padded",
    );
    bits.as_raw_slice().hash(state);
}

/// Whether a stored stream has a valid marker in its final byte.
pub(crate) fn padding_is_canonical(bits: &Bits) -> bool {
    match bits.as_raw_slice() {
        [] => true,
        [0x80] => false,
        [.., 0] => false,
        _ => true,
    }
}

/// Validate the marker and zero padding after a decoded value.
///
/// No remaining bit means the marker was truncated. Any remainder other than
/// one marker followed by at most seven zeros is trailing data.
///
/// # Panics
///
/// `pos` must be at or before the end of `bytes`.
pub(crate) fn require_marker_padding(bytes: &[u8], pos: u64) -> Result<(), Decode> {
    let total = bytes.len() as u64 * 8;
    assert!(
        pos <= total,
        "padding checked at a position inside the buffer"
    );
    let remainder = total - pos;
    match remainder {
        0 => Err(Decode::Truncated),
        1..=8 => {
            // The remainder lives entirely in the final byte: its low
            // `remainder` bits must be a `1` followed by zeros.
            let last = bytes[bytes.len() - 1];
            let mask = if remainder == 8 {
                0xFF
            } else {
                (1u8 << remainder) - 1
            };
            if last & mask == 1 << (remainder - 1) {
                Ok(())
            } else {
                Err(Decode::TrailingBits)
            }
        }
        _ => Err(Decode::TrailingBits),
    }
}
