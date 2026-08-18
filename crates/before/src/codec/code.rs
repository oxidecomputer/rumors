//! A complete payload code as a value: machine words for the narrow codes that
//! dominate every organic stream, a bit buffer for the rest.
//!
//! The emitters trade whole codes — a leaf's payload moves from a gamma encoder
//! or a verbatim input range into a builder, is held there against the collapse
//! checks, and lands in the output stream. Carrying each code as an owned bit
//! buffer prices that trade at a heap allocation per leaf; this type keeps
//! every code up to 63 bits — a zigzag delta magnitude below `2^31`, far past
//! the band organic histories occupy — in two machine words, and spills wider
//! codes to the buffer form unchanged.

use super::{BitsMut, BitsView};

/// One complete payload code, value-packed when it fits a word.
pub(crate) enum Code {
    /// A code of `len` bits (1..=63), value-packed at the low end of
    /// `bits` (the code's first bit is the register's most significant
    /// live bit; bits above `len` are zero).
    Small { bits: u64, len: u8 },
    /// A code wider than 63 bits, as an owned bit buffer.
    Wide(BitsMut),
}

/// The widest code [`Code::Small`] carries.
pub(crate) const SMALL_CODE_BITS: usize = 63;

impl Code {
    /// The code's length in bits.
    pub(crate) fn len(&self) -> usize {
        match self {
            Code::Small { len, .. } => usize::from(*len),
            Code::Wide(bits) => bits.len(),
        }
    }

    /// A code copied out of the bit range `start..end` of a canonical
    /// stream's view.
    ///
    /// # Panics
    ///
    /// `start..end` must be a range within the view's live length.
    pub(crate) fn from_range(src: BitsView<'_>, start: u64, end: u64) -> Code {
        debug_assert!(start < end, "a payload code is never empty");
        assert!(
            end <= src.len(),
            "copied range within the view's live length"
        );
        let len = end - start;
        if len <= SMALL_CODE_BITS as u64 {
            Code::Small {
                bits: src.load_be(start, len as u32),
                len: len as u8,
            }
        } else {
            let mut out = BitsMut::with_capacity(len as usize);
            super::bits::extend_from_view(&mut out, src, start, end);
            Code::Wide(out)
        }
    }
}
