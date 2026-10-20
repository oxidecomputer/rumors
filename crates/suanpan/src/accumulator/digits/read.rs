//! Read a normalized magnitude by normalizing the digits it reads.
//!
//! A read first compares with zero, which compacts any cancelled top and
//! supplies the sign. It then normalizes the stored digits from its starting
//! position upward, collecting each unsigned magnitude digit as the pass
//! writes it. The stored form keeps the normalized digits, so a repeated read
//! costs the result's width rather than the cancellation the first read
//! removed. The digits are paired into minimal 64-bit limbs as they arrive.
//!
//! An unscaled read starts at position zero. A scaled read starts at
//! `lowest_written`, because every lower digit is zero, and reports the skipped
//! prefix as a bit shift; interior gaps still cost their full width.

use core::cmp::Ordering;

use super::Digits;
use crate::accumulator::limbs::{DigitPacker, Limbs};
use crate::accumulator::DIGIT_BITS;

/// Produce normalized output by normalizing the stored representation.
impl Digits {
    /// Return the comparison with zero and minimal limbs of the whole value.
    pub fn normalized_limbs(&mut self) -> (Ordering, Limbs) {
        let (ordering, limbs, start) = self.read_normalized(false);
        debug_assert_eq!(start, 0, "an unscaled read starts at position zero");
        (ordering, limbs)
    }

    /// Omit the known-zero low prefix and return its width in bits.
    pub fn normalized_limbs_with_shift(&mut self) -> (Ordering, Limbs, u64) {
        let (ordering, limbs, start) = self.read_normalized(true);
        let shift = u64::try_from(start)
            .expect("an allocated digit position fits u64")
            .checked_mul(u64::from(DIGIT_BITS))
            .expect("an allocated digit position has a representable bit offset");
        (ordering, limbs, shift)
    }

    /// Normalize from the read's starting position and collect the magnitude.
    ///
    /// Returns the sign, the minimal limbs, and the starting digit position.
    fn read_normalized(&mut self, scaled: bool) -> (Ordering, Limbs, usize) {
        let sign = self.cmp_zero();
        // The comparison may deposit one digit below `lowest_written`, so the
        // scaled start is chosen only after it.
        let start = if scaled { self.written_floor() } else { 0 };
        // The pass emits one digit per position it covers, which sizes the
        // limbs exactly, and at most one carry digit above them, which grows
        // them by at most one limb.
        let mut magnitude = DigitPacker::new(self.highest_nonzero - start + 1);
        if sign != Ordering::Equal {
            self.normalize_nonzero_from(sign, start, |digit| magnitude.push(digit));
        }
        let limbs = magnitude.finish();
        debug_assert_eq!(
            sign == Ordering::Equal,
            limbs.is_empty(),
            "the readout's limbs are empty exactly at zero"
        );
        (sign, limbs, start)
    }
}
