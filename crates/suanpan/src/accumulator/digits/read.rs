//! Read a normalized magnitude by normalizing the digits it reads.
//!
//! A read first compares with zero, which compacts any cancelled top and
//! supplies the sign. It then normalizes the stored digits from its starting
//! position upward, collecting each unsigned magnitude digit as the pass
//! writes it. The stored form keeps the normalized digits, so a repeated read
//! costs the result's width rather than the cancellation the first read
//! removed. Packing pairs of 32-bit digits produces minimal 64-bit limbs.
//!
//! An unscaled read starts at position zero. A scaled read starts at
//! `lowest_written`, because every lower digit is zero; interior gaps still
//! cost their full width.

use core::cmp::Ordering;

use super::Digits;
use crate::accumulator::DIGIT_BITS;

/// Produce normalized output by normalizing the stored representation.
impl Digits {
    /// Return the comparison with zero and minimal limbs of the whole value.
    pub fn normalized_limbs(&mut self) -> (Ordering, Vec<u64>) {
        let (ordering, limbs, start) = self.read_normalized(false);
        debug_assert_eq!(start, 0, "an unscaled read starts at position zero");
        (ordering, limbs)
    }

    /// Omit the known-zero low prefix and return its scale separately.
    pub fn normalized_limbs_with_shift(&mut self) -> (Ordering, Vec<u64>, u64) {
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
    fn read_normalized(&mut self, scaled: bool) -> (Ordering, Vec<u64>, usize) {
        let sign = self.cmp_zero();
        // The comparison may deposit one digit below `lowest_written`, so the
        // scaled start is chosen only after it.
        let start = if scaled {
            self.lowest_written.min(self.highest_nonzero)
        } else {
            0
        };
        let mut magnitude: Vec<u32> = Vec::with_capacity(self.highest_nonzero - start + 2);
        if sign != Ordering::Equal {
            self.normalize_nonzero_from(sign, start, |digit| magnitude.push(digit));
        }
        let limbs = pack_limbs(magnitude);
        debug_assert_eq!(
            sign == Ordering::Equal,
            limbs.is_empty(),
            "the readout's limbs are empty exactly at zero"
        );
        (sign, limbs, start)
    }
}

/// Pack unsigned base-2^32 digits into minimal little-endian 64-bit limbs.
fn pack_limbs(digits: Vec<u32>) -> Vec<u64> {
    let mut limbs: Vec<u64> = digits
        .chunks(2)
        .map(|pair| u64::from(pair[0]) | (pair.get(1).copied().map_or(0, u64::from) << 32))
        .collect();
    drop(digits);
    while limbs.last() == Some(&0) {
        limbs.pop();
    }
    debug_assert!(limbs.last().is_none_or(|&limb| limb != 0));
    limbs
}
