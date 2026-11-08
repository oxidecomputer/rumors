//! Sets of bytes, and their one canonical encoding.
//!
//! A [`ByteSet`] holds any subset of the 256 values a `u8` can take. Its values
//! here are radixes: a child sits under its parent at a *radix*, the byte that
//! extends the parent's prefix to the child's, so a node has at most a fan of
//! 256 children, each at its own radix, and a set of radixes names any choice
//! of a node's children exactly. A backend reports the radixes of a node's
//! children as one
//! ([`ErasedNode::child_radices`](super::ErasedNode::child_radices)), so the
//! children either of two sides holds are the [`union`](ByteSet::union) of two
//! sets, and a reply's first match carries the later children it covers as one.
//!
//! In memory a set is always a 256-bit bitmap. On the wire it is one CBOR byte
//! string, in whichever of two spellings is shorter:
//!
//! | Elements  | Byte string                                     |
//! | --------- | ----------------------------------------------- |
//! | 0 to 31   | the elements, one byte each, strictly ascending |
//! | 32 to 256 | the 32-byte bitmap                              |
//!
//! The byte string's length tells the two spellings apart: a list is shorter
//! than 32 bytes, and a bitmap is exactly 32. Every set has exactly one
//! encoding, so decoding rejects every other spelling: a list that does not
//! strictly ascend, a bitmap holding fewer than 32 elements (which the list
//! spells), and anything longer than a bitmap. The empty set is the empty byte
//! string.
//!
//! In the bitmap, value `v` is bit `v % 8` of byte `v / 8`, counting bits from
//! the least significant.
//!
//! The encoding lives here alone: [`ByteSet::encoded`] and [`ByteSet::decode`]
//! give its bytes to the codec, which writes its own CBOR heads, so no other
//! code knows how a set is spelled.

use std::fmt;

use bitvec::{array::BitArray, order::Lsb0};

#[cfg(test)]
mod tests;

/// The bytes of a set's bitmap: one bit for every value a byte can take.
pub const BITMAP_LEN: usize = (u8::MAX as usize + 1) / u8::BITS as usize;

/// The fewest elements a set encodes as its bitmap: a list of this many would
/// be at least as long as the bitmap.
const BITMAP_MIN_ELEMENTS: usize = BITMAP_LEN;

/// The most bytes any set encodes to: a bitmap, which the longest list, one
/// element short of it, never exceeds.
pub const MAX_ENCODED_LEN: usize = BITMAP_LEN;

/// A set of bytes, held as a 256-bit bitmap.
///
/// The [`Default`] set is empty. Iteration yields the elements in ascending
/// order.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct ByteSet(BitArray<[u8; BITMAP_LEN], Lsb0>);

impl ByteSet {
    /// The empty set.
    pub const EMPTY: Self = Self(BitArray::ZERO);

    /// Add `value` to the set, returning whether it was absent.
    pub fn insert(&mut self, value: u8) -> bool {
        !self.0.replace(usize::from(value), true)
    }

    /// Remove `value` from the set, returning whether it was present.
    pub fn remove(&mut self, value: u8) -> bool {
        self.0.replace(usize::from(value), false)
    }

    /// Whether `value` is in the set.
    pub fn contains(&self, value: u8) -> bool {
        self.0[usize::from(value)]
    }

    /// The number of elements: at most 256.
    pub fn len(&self) -> usize {
        self.0.count_ones()
    }

    /// Whether the set has no elements.
    pub fn is_empty(&self) -> bool {
        self.0.not_any()
    }

    /// The number of elements greater than `value`.
    pub fn len_above(&self, value: u8) -> usize {
        self.0[usize::from(value) + 1..].count_ones()
    }

    /// The values in this set or in `other`.
    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// The elements in ascending order.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = u8> + '_ {
        self.0.iter_ones().map(|index| {
            u8::try_from(index).expect("a bitmap of 256 bits indexes only byte values")
        })
    }

    /// The set's canonical encoding: its elements in ascending order when it
    /// has fewer than 32, its bitmap otherwise (the module docs give the
    /// schema).
    ///
    /// The encoding is at most [`MAX_ENCODED_LEN`] bytes. A codec writes it as
    /// the content of one CBOR byte string.
    pub fn encoded(&self) -> Encoded {
        let mut bytes = [0; MAX_ENCODED_LEN];
        let len = if self.len() < BITMAP_MIN_ELEMENTS {
            let mut len = 0;
            for (slot, value) in bytes.iter_mut().zip(self.iter()) {
                *slot = value;
                len += 1;
            }
            len
        } else {
            bytes.copy_from_slice(self.0.as_raw_slice());
            BITMAP_LEN
        };
        Encoded { bytes, len }
    }

    /// Decode a set from the content of its CBOR byte string, accepting only
    /// its canonical encoding.
    ///
    /// # Errors
    ///
    /// Returns [`ByteSetError::Unordered`] if a list shorter than a bitmap does
    /// not strictly ascend, [`ByteSetError::SparseBitmap`] if a bitmap holds
    /// fewer than 32 elements, and [`ByteSetError::TooLong`] if the content is
    /// longer than a bitmap.
    pub fn decode(bytes: &[u8]) -> Result<Self, ByteSetError> {
        match bytes.len() {
            len if len < BITMAP_LEN => {
                let mut set = Self::EMPTY;
                let mut previous = None;
                for (index, &value) in bytes.iter().enumerate() {
                    if let Some(previous) = previous
                        && previous >= value
                    {
                        return Err(ByteSetError::Unordered {
                            index,
                            previous,
                            value,
                        });
                    }
                    previous = Some(value);
                    set.insert(value);
                }
                Ok(set)
            }
            BITMAP_LEN => {
                let bitmap: [u8; BITMAP_LEN] = bytes.try_into().expect("matched the bitmap length");
                let set = Self(BitArray::new(bitmap));
                if set.len() < BITMAP_MIN_ELEMENTS {
                    return Err(ByteSetError::SparseBitmap { len: set.len() });
                }
                Ok(set)
            }
            len => Err(ByteSetError::TooLong { len }),
        }
    }
}

/// Collect values into a set; a value collected twice is held once.
impl FromIterator<u8> for ByteSet {
    fn from_iter<I: IntoIterator<Item = u8>>(values: I) -> Self {
        let mut set = Self::EMPTY;
        set.extend(values);
        set
    }
}

/// Add every value to the set.
impl Extend<u8> for ByteSet {
    fn extend<I: IntoIterator<Item = u8>>(&mut self, values: I) {
        for value in values {
            self.insert(value);
        }
    }
}

/// Format the set as its elements, ascending.
impl fmt::Debug for ByteSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

/// A set's canonical encoding, held inline: at most [`MAX_ENCODED_LEN`] bytes
/// ([`ByteSet::encoded`]).
#[derive(Clone, Copy, Debug)]
pub struct Encoded {
    /// The encoding, in its first `len` bytes.
    bytes: [u8; MAX_ENCODED_LEN],
    /// The encoding's length.
    len: usize,
}

impl Encoded {
    /// The encoding's bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

/// A byte string that is not a set's canonical encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ByteSetError {
    /// A list of elements does not strictly ascend at `index`: its value
    /// repeats or descends from the one before it.
    #[error("set element {value} at index {index} does not ascend past {previous}")]
    Unordered {
        /// The index of the element out of order.
        index: usize,
        /// The element before it.
        previous: u8,
        /// The element out of order.
        value: u8,
    },
    /// A bitmap holds fewer elements than a bitmap encodes, a set the list
    /// spells instead.
    #[error("a set bitmap holds {len} elements; fewer than 32 are spelled as a list")]
    SparseBitmap {
        /// The elements the bitmap holds.
        len: usize,
    },
    /// The encoding is longer than any set's.
    #[error("a set encoding of {len} bytes is longer than the 32-byte bitmap")]
    TooLong {
        /// The encoding's length.
        len: usize,
    },
}
