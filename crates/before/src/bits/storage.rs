//! Canonical bit-stream storage.
//!
//! [`Bits`] owns immutable bytes. A nonempty stream ends with one marker bit
//! followed by zero padding; the empty stream has no bytes. This makes the live
//! length recoverable and gives every stream exactly one byte encoding.

#![allow(rustdoc::private_intra_doc_links)]

use core::fmt;
use core::hash::{Hash, Hasher};

use bytes::Bytes;

use super::{BitRead, BitsReader};
use crate::error::Decode;

/// Immutable canonical bytes for a party or version.
///
/// Clones share the refcounted byte buffer. Readers exclude the marker and
/// padding.
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
        let mut reader = self.reader();
        while reader.position() < reader.len() {
            f.write_str(if reader.read_bit().expect("position is live") {
                "1"
            } else {
                "0"
            })?;
        }
        Ok(())
    }
}

impl Bits {
    /// Adopt bytes whose canonical padding has already been validated.
    ///
    /// `bytes` must be marker-padded — the live bits, one `1`, then zeros to
    /// the byte boundary; empty for the empty stream — which is what
    /// [`validate_padding`](Self::validate_padding) accepts. Debug builds
    /// assert it; release builds trust the validator.
    ///
    pub(crate) fn from_canonical(bytes: Bytes) -> Self {
        let bits = Bits { bytes };
        debug_assert!(
            bits.has_canonical_padding(),
            "from_canonical: the buffer must end in the canonical `1 0*` padding",
        );
        bits
    }

    /// Start reading at the first live bit.
    pub(crate) fn reader(&self) -> BitsReader<'_> {
        BitsReader::at(self, 0)
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

    /// Whether the stored bytes end in their unique marker and zero padding.
    pub(crate) fn has_canonical_padding(&self) -> bool {
        match self.as_raw_slice() {
            [] => true,
            [0x80] | [.., 0] => false,
            _ => true,
        }
    }

    /// Validate the marker and zero padding after a decoded value.
    ///
    /// No remaining bit means the marker was truncated. Any remainder other
    /// than one marker followed by at most seven zeros is trailing data.
    ///
    /// # Panics
    ///
    /// `position` must be at or before the end of `bytes`.
    pub(crate) fn validate_padding(bytes: &[u8], position: u64) -> Result<(), Decode> {
        let total = bytes.len() as u64 * 8;
        assert!(
            position <= total,
            "padding checked at a position inside the buffer"
        );
        if position == 0 && !bytes.is_empty() {
            // A value has at least one live bit. Treating the first bit as its
            // marker would give the empty stream a nonempty spelling.
            return Err(Decode::TrailingBits);
        }
        let remainder = total - position;
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

    /// Consume unique test storage and report its retained allocation.
    #[cfg(test)]
    pub(crate) fn allocation_capacity(self) -> usize {
        self.bytes
            .try_into_mut()
            .expect("the test owns the only reference to these bytes")
            .capacity()
    }
}

/// Compare the canonical bytes, with a shared-buffer fast path.
impl PartialEq for Bits {
    fn eq(&self, other: &Self) -> bool {
        debug_assert!(
            self.has_canonical_padding() && other.has_canonical_padding(),
            "Bits equality requires canonical marker padding",
        );
        self.ptr_eq(other) || self.as_raw_slice() == other.as_raw_slice()
    }
}

impl Eq for Bits {}

/// Hash the same canonical bytes that equality compares.
impl Hash for Bits {
    fn hash<H: Hasher>(&self, state: &mut H) {
        debug_assert!(
            self.has_canonical_padding(),
            "Bits hashing requires canonical marker padding",
        );
        self.as_raw_slice().hash(state);
    }
}
