//! The value behind an accumulator, and its operations that dispatch on the
//! representation.
//!
//! [`Value`] holds either a small `i128` or redundant signed digits. Its reads
//! take `&mut self` because they compact what they touch. The public
//! [`Accumulator`](super::Accumulator) keeps a `Value` in a cell, so that its
//! own reads can take `&self`. Sibling modules extend `Value` with their own
//! concerns, such as operand routing and primitive conversion.

use core::cmp::Ordering;

use super::digits::Digits;
use super::operand::Update;
use super::small::{limbs_from_u128, SMALL_MAX, SMALL_SHIFT_MAX};
use super::{touch, DIGIT_BITS};

/// Owned normalized limbs, inline for small values and allocated when wide.
pub(crate) enum MagnitudeLimbs {
    /// Up to two limbs stored inside the return value.
    Inline { limbs: [u64; 2], len: usize },
    /// Arbitrarily wide normalized limbs.
    Wide(Vec<u64>),
}

/// Expose normalized limbs without exposing their storage choice.
impl AsRef<[u64]> for MagnitudeLimbs {
    fn as_ref(&self) -> &[u64] {
        match self {
            Self::Inline { limbs, len } => &limbs[..*len],
            Self::Wide(limbs) => limbs,
        }
    }
}

/// An exact signed integer in one of two representations.
///
/// Copying a value duplicates its uncompacted cancellation, which is why
/// [`Accumulator`](super::Accumulator) is not `Clone`; its `Debug` copies the
/// stored form only to format it.
#[derive(Clone)]
pub(crate) struct Value {
    /// An exact small value, or `None` once the digit representation is active.
    ///
    /// While this is `Some`, `digits` is empty or contains only zeros.
    pub(crate) small: Option<i128>,
    /// Retained digit storage and known zero ranges, active when `small` is `None`.
    pub(crate) digits: Digits,
}

/// Keep the representation choice behind one arithmetic interface.
impl Value {
    /// Construct zero without allocating.
    pub(crate) fn new() -> Value {
        Value {
            small: Some(0),
            digits: Digits::new(),
        }
    }

    /// Reset to zero, retaining allocated storage.
    pub(crate) fn reset(&mut self) {
        if self.small.is_none() {
            self.digits.reset();
        }
        self.small = Some(0);
    }

    /// Reserve storage for `bits` bits of working width, if it can be honored.
    pub(crate) fn reserve_bits(&mut self, bits: u64) {
        // A position count beyond `usize` can never be allocated, so
        // saturating sends it to the same refusal as any oversized request.
        let positions = usize::try_from(bits.div_ceil(u64::from(DIGIT_BITS))).unwrap_or(usize::MAX);
        self.digits.reserve(positions);
    }

    /// Compare with zero, compacting any cancellation the scan reads.
    #[inline]
    pub(crate) fn cmp_zero(&mut self) -> Ordering {
        if let Some(value) = self.small {
            touch(1);
            return value.cmp(&0);
        }
        self.digits.cmp_zero()
    }

    /// Return the sign if no adjustment below `2^bits` can change it.
    pub(crate) fn cmp_zero_stable_under(&mut self, bits: u64) -> Option<Ordering> {
        // Whole digits are the unit used by both representations. At least one
        // digit is retained even for a zero-width adjustment.
        let adjustment_digits = bits.div_ceil(u64::from(DIGIT_BITS)).max(1);

        if let Some(value) = self.small {
            touch(1);
            // Three times the first power above the adjustment covers the
            // extra range of a redundant top digit as well as an ordinary
            // magnitude. Overflow merely means this small value cannot prove
            // stability at the requested width.
            let threshold_bits = u32::try_from(adjustment_digits.checked_mul(32)?).ok()?;
            let threshold = 3u128.checked_shl(threshold_bits)?;
            return (value.unsigned_abs() >= threshold).then(|| value.cmp(&0));
        }
        self.digits.cmp_zero_stable_above(adjustment_digits - 1)
    }

    /// Return whether the representation alone shows zero.
    #[inline]
    pub(crate) fn is_known_zero(&self) -> bool {
        match self.small {
            Some(value) => value == 0,
            None => self.digits.is_known_zero(),
        }
    }

    /// Return the working width in 32-bit units, at least one.
    #[inline]
    pub(crate) fn stored_digit_count(&self) -> usize {
        match self.small {
            Some(0) => 1,
            Some(value) => (128 - value.unsigned_abs().leading_zeros() as usize).div_ceil(32),
            None => self.digits.stored_digit_count(),
        }
    }

    /// Return the working width in bits.
    #[inline]
    pub(crate) fn stored_bits(&self) -> u64 {
        u64::try_from(self.stored_digit_count())
            .expect("an allocated accumulator's digit count fits u64")
            .checked_mul(u64::from(DIGIT_BITS))
            .expect("an allocated accumulator's bit span fits u64")
    }

    /// Return the sign and minimal magnitude limbs, normalizing what the read
    /// touches.
    pub(crate) fn signed_magnitude(&mut self) -> (Ordering, MagnitudeLimbs) {
        if let Some(value) = self.small {
            touch(self.stored_digit_count() as u64);
            let (limbs, len) = limbs_from_u128(value.unsigned_abs());
            return (value.cmp(&0), MagnitudeLimbs::Inline { limbs, len });
        }
        let (ordering, limbs) = self.digits.normalized_limbs();
        (ordering, MagnitudeLimbs::Wide(limbs))
    }

    /// Return the sign, minimal magnitude limbs, and a power-of-two scale,
    /// normalizing what the read touches.
    pub(crate) fn scaled_signed_magnitude(&mut self) -> (Ordering, MagnitudeLimbs, u64) {
        if let Some(value) = self.small {
            touch(self.stored_digit_count() as u64);
            let (limbs, len) = limbs_from_u128(value.unsigned_abs());
            return (value.cmp(&0), MagnitudeLimbs::Inline { limbs, len }, 0);
        }
        let (ordering, limbs, shift) = self.digits.normalized_limbs_with_shift();
        (ordering, MagnitudeLimbs::Wide(limbs), shift)
    }

    /// Negate in place.
    pub(crate) fn negate(&mut self) {
        if let Some(value) = &mut self.small {
            touch(1);
            *value = -*value;
        } else {
            self.digits.negate();
        }
    }

    /// Narrow a shift count, returning `None` for a zero shifted by `2^64`
    /// bits or more, which deposits nothing.
    ///
    /// # Panics
    ///
    /// Panics if the value is nonzero and the count is `2^64` or more. Such a
    /// value starts above digit position `2^59`, a 4 EiB buffer that no
    /// allocator can supply, so we panic before anything is written rather
    /// than in the allocator.
    pub(crate) fn addressable_shift(&mut self, shift: u128) -> Option<u64> {
        let shift = u64::try_from(shift).ok();
        assert!(
            shift.is_some() || self.cmp_zero() == Ordering::Equal,
            "a nonzero contribution needs an addressable digit position"
        );
        shift
    }

    /// Multiply by `2^shift`, retaining the small representation whenever it fits.
    ///
    /// A shift of `2^64` bits or more is accepted only for zero.
    pub(crate) fn shift_left(&mut self, shift: u128) {
        if shift == 0 || self.is_known_zero() {
            return;
        }
        let Some(shift) = self.addressable_shift(shift) else {
            return;
        };
        if let Some(value) = self.small {
            touch(1);
            if shift <= SMALL_SHIFT_MAX {
                let shifted = value << shift;
                if shifted.unsigned_abs() <= SMALL_MAX {
                    self.small = Some(shifted);
                } else {
                    self.start_digits();
                    self.digits.deposit_value(shifted, 0);
                }
            } else {
                self.start_digits();
                self.digits.deposit_value(value, shift);
            }
            return;
        }
        // A zero whose digits cancel shifts to itself. Returning here keeps it
        // in the digit representation with its buffer, as the one-way rule
        // requires, and never lands its cancelling digits at the shifted
        // position.
        if self.digits.cmp_zero() == Ordering::Equal {
            return;
        }
        let previous = core::mem::replace(&mut self.digits, Digits::new());
        self.digits.activate();
        self.digits
            .add_digits(previous.stored_digits(), shift, Update::Add);
    }
}
