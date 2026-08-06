//! A decoded payload integer as a value: a machine word for the values that
//! dominate every organic stream, the arbitrary-precision [`Base`] past that.
//!
//! The write-side twin is [`Code`](super::Code): together they keep a narrow
//! payload in machine words end to end — decoded from the stream as a word,
//! folded into an accumulator through the word entry points, re-coded by shift
//! arithmetic — with the wide forms carrying every value past the word range
//! unchanged. Every reader (for example,
//! [`BitCursor::read_int`](super::BitCursor::read_int)) hands values out in
//! this form; a consumer with genuinely wide arithmetic converts through
//! [`into_base`](Int::into_base) at its own seam.

use core::cmp::Ordering;

use super::Base;

/// One decoded payload integer, word-sized when it fits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Int {
    /// A value within the machine-word range.
    Small(u64),
    /// A value past the word range (decode constructs this only for such
    /// values; arithmetic may park smaller values here, which costs the word
    /// fast path but never correctness).
    Wide(Base),
}

impl Int {
    /// The zero value.
    pub(crate) const ZERO: Int = Int::Small(0);

    /// Whether the value is zero.
    pub(crate) fn is_zero(&self) -> bool {
        match self {
            Int::Small(n) => *n == 0,
            Int::Wide(base) => base.bits() == 0,
        }
    }

    /// The value as a machine word, when it fits.
    pub(crate) fn to_u64(&self) -> Option<u64> {
        match self {
            Int::Small(n) => Some(*n),
            Int::Wide(base) => base.to_u64(),
        }
    }

    /// The value widened to a [`Base`].
    pub(crate) fn into_base(self) -> Base {
        match self {
            Int::Small(n) => Base::from(n),
            Int::Wide(base) => base,
        }
    }

    /// A [`Base`] value, parked word-sized when it fits.
    pub(crate) fn from_base(base: Base) -> Int {
        match base.to_u64() {
            Some(n) => Int::Small(n),
            None => Int::Wide(base),
        }
    }

    /// Magnitude order across widths (a [`Wide`](Int::Wide) parking a
    /// word-scale value compares by value, exactly as its [`Small`](Int::Small)
    /// spelling would).
    pub(crate) fn cmp_magnitude(&self, other: &Int) -> Ordering {
        match (self, other) {
            (Int::Small(a), Int::Small(b)) => a.cmp(b),
            (Int::Small(a), Int::Wide(b)) => match b.to_u64() {
                Some(b) => a.cmp(&b),
                None => Ordering::Less,
            },
            (Int::Wide(a), Int::Small(b)) => match a.to_u64() {
                Some(a) => a.cmp(b),
                None => Ordering::Greater,
            },
            (Int::Wide(a), Int::Wide(b)) => a.cmp(b),
        }
    }

    /// A raw magnitude, parked word-sized when it fits.
    pub(crate) fn from_ubig(magnitude: suanpan::UBig) -> Int {
        match u64::try_from(&magnitude) {
            Ok(n) => Int::Small(n),
            Err(_) => Int::Wide(Base::from(magnitude)),
        }
    }
}
