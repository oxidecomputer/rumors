//! Convert redundant signed digits into a normalized magnitude.
//!
//! A low-to-high carry produces unsigned digits `M` and a signed high part
//! `c` satisfying `value = c B^n + M`, with `0 <= M < B^n` and `B = 2^32`.
//! The carry stays in `[-3, 2]`: this interval is closed under
//! `floor((digit + carry) / B)` for every stored digit. Thus the high part
//! needs at most one extra unsigned digit.
//!
//! For a negative result, the magnitude is `|c| B^n - M`. When `M` is
//! nonzero, complement its digits and use `|c| - 1` as the high part.
//! When `M` is zero, retain those zeros and use `|c|` directly. Finally,
//! packing pairs of 32-bit digits produces minimal 64-bit limbs.
//!
//! The stored digits remain unchanged. Conversion allocates an unsigned digit
//! buffer and a limb buffer, each proportional to the inspected span.
//! Scaled readout can begin at `lowest_written` because every lower digit is
//! zero; interior gaps still cost their full width.

use core::cmp::Ordering;

use super::Digits;
use crate::accumulator::{touch, DIGIT_BITS, DIGIT_MASK};

/// Produce normalized output without changing the stored representation.
impl Digits {
    /// Read the whole stored prefix as a comparison with zero and minimal limbs.
    pub fn normalized_limbs(&self) -> (Ordering, Vec<u64>) {
        self.debug_assert_valid();
        let (ordering, digits) = self.read_digits(0);
        let limbs = pack_limbs(digits);
        debug_assert_eq!(
            ordering == Ordering::Equal,
            limbs.is_empty(),
            "the readout's limbs are empty exactly at zero"
        );
        (ordering, limbs)
    }

    /// Omit the known-zero low prefix and return its scale separately.
    pub fn normalized_limbs_with_shift(&self) -> (Ordering, Vec<u64>, u64) {
        self.debug_assert_valid();
        let start = self.lowest_written.min(self.highest_nonzero);
        let (ordering, digits) = self.read_digits(start);
        let shift = u64::try_from(start)
            .expect("an allocated digit position fits u64")
            .checked_mul(u64::from(DIGIT_BITS))
            .expect("an allocated digit position has a representable bit offset");
        (ordering, pack_limbs(digits), shift)
    }

    /// Normalize the suffix beginning at `start` into unsigned 32-bit digits.
    ///
    /// Every digit below `start` must be zero, and any `start` at or below
    /// `lowest_written` qualifies. This method reads nothing below `start`,
    /// because skipping that prefix is what keeps a scaled read proportional to
    /// the written span; a nonzero digit there would be left out of the result.
    /// The tests in `accumulator::tests::representation` check the zero prefix
    /// below `lowest_written` after every step, and compare the scaled readout
    /// with an oracle.
    ///
    /// High zero digits may remain; limb packing removes them after combining
    /// pairs.
    fn read_digits(&self, start: usize) -> (Ordering, Vec<u32>) {
        debug_assert!(start <= self.highest_nonzero);
        let mut collected: Vec<u32> = Vec::with_capacity(self.highest_nonzero - start + 2);
        let mut carry: i128 = 0;
        for &digit in &self.digits[start..=self.highest_nonzero] {
            touch(1);
            let total = i128::from(digit) + carry;
            let low = total.rem_euclid(1 << DIGIT_BITS);
            collected.push(low as u32);
            carry = (total - low) >> DIGIT_BITS;
            debug_assert!((-3..=2).contains(&carry));
        }
        if carry < 0 {
            // Negative: |value| = |carry| · 2^(32·len) − M, which is
            // (|carry| − 1) high part plus the complement of M when M > 0,
            // and |carry| high part over untouched zeros when M = 0.
            let low_nonzero = collected.iter().any(|&digit| digit != 0);
            if low_nonzero {
                let mut complement_carry = 1u64;
                for digit in collected.iter_mut() {
                    touch(1);
                    let complemented = (DIGIT_MASK - u64::from(*digit)) + complement_carry;
                    *digit = (complemented & DIGIT_MASK) as u32;
                    complement_carry = complemented >> DIGIT_BITS;
                }
                debug_assert_eq!(
                    complement_carry, 0,
                    "complement of a nonzero low part cannot carry out"
                );
            }
            let mut high = (-carry) as u128 - u128::from(low_nonzero);
            while high > 0 {
                touch(1);
                collected.push((high & u128::from(DIGIT_MASK)) as u32);
                high >>= DIGIT_BITS;
            }
            // |carry| ≥ 1 makes |value| ≥ 2^(32·len) − M > 0: never zero.
            (Ordering::Less, collected)
        } else {
            let mut high = carry as u128;
            while high > 0 {
                touch(1);
                collected.push((high & u128::from(DIGIT_MASK)) as u32);
                high >>= DIGIT_BITS;
            }
            let ordering = if collected.iter().all(|&digit| digit == 0) {
                Ordering::Equal
            } else {
                Ordering::Greater
            };
            (ordering, collected)
        }
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
