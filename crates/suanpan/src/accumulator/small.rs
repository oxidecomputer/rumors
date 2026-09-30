//! Keep small values in one machine integer.
//!
//! An accumulator begins with an exact `i128` value whose magnitude is at most
//! `2^96`. Primitive updates and shifts of at most 30 bits can then be computed
//! directly in `i128`: `2^96 + 2^126` remains below its positive limit. No
//! allocation is needed while the result stays within the small-value bound.
//!
//! A wider value, a larger shift, or a streamed operand moves the exact result
//! into signed digits. The accumulator does not move back until reset. Without
//! that rule, alternating around the boundary could repeatedly pay to convert
//! the same wide value in both directions. Reset keeps any allocated digit
//! buffer so a later wide value can reuse it.
//!
//! The sign-preservation query can inspect this exact value directly. Its
//! threshold, `3 * 2^(32 * (floor + 1))`, exceeds the greatest adjustment the
//! supplied width can represent, including the extra range of redundant
//! signed digits.

use super::{touch, Accumulator};

/// Largest magnitude retained as one `i128` value.
pub const SMALL_MAX: u128 = 1 << 96;

/// Largest shift that leaves room to add two small values inside `i128`.
///
/// `SMALL_MAX << 30` is `2^126`; adding another value of at most `2^96`
/// remains below `i128::MAX`.
pub const SMALL_SHIFT_MAX: u64 = 30;

/// Operate on the small value and transfer it into digits when necessary.
impl Accumulator {
    /// Add a bounded delta, returning whether the small path handled it.
    ///
    /// A handled update may move its sum into digits. `false` means the
    /// accumulator already uses digits and the caller must add there instead.
    /// Callers pass at most a small value shifted by 30 bits.
    #[inline]
    pub(crate) fn try_small_add(&mut self, delta: i128) -> bool {
        let Some(current) = self.small else {
            return false;
        };
        debug_assert!(current.unsigned_abs() <= SMALL_MAX);
        debug_assert!(delta.unsigned_abs() <= SMALL_MAX << SMALL_SHIFT_MAX);
        touch(1);
        let sum = current + delta;
        if sum.unsigned_abs() <= SMALL_MAX {
            self.small = Some(sum);
        } else {
            self.start_digits();
            self.digits.deposit_value(sum, 0);
        }
        debug_assert!(self
            .small
            .is_none_or(|value| value.unsigned_abs() <= SMALL_MAX));
        true
    }

    /// Transfer a small value into digits, or leave existing digits alone.
    pub(crate) fn ensure_digits(&mut self) {
        if let Some(value) = self.small {
            self.start_digits();
            self.digits.deposit_value(value, 0);
        }
    }

    /// Select the empty or retained digit buffer as the active representation.
    pub(crate) fn start_digits(&mut self) {
        debug_assert!(
            self.small.is_some(),
            "the digit representation activates once per reset"
        );
        self.small = None;
        self.digits.activate();
    }
}

/// Place an unsigned small-value magnitude in a two-limb stack buffer.
pub fn limbs_from_u128(value: u128) -> ([u64; 2], usize) {
    let limbs = [value as u64, (value >> 64) as u64];
    let len = usize::from(value != 0) + usize::from(value > u128::from(u64::MAX));
    debug_assert!(len <= 2);
    debug_assert_eq!(len == 0, value == 0);
    debug_assert!(len < 2 || limbs[1] != 0);
    (limbs, len)
}
