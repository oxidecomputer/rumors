//! Convert each input piece into a direct signed digit contribution.
//!
//! The bit offset splits into a whole-digit offset and fewer than 32 low bits.
//! A piece is shifted only by that remainder, then deposited at its destination
//! index. No shifted input buffer is built, so arithmetic work follows the
//! number of supplied pieces rather than the receiver's width or the offset.
//!
//! Each source is decomposed only enough to keep one contribution inside the
//! carry intermediate: small values use 32-bit magnitude pieces, 64-bit limbs
//! split in half, and stored signed digits already satisfy the bound. They all
//! pass through [`Digits::add_at`], so no alternate write path can bypass the
//! digit or zero-range invariants.
//!
//! Position calculations stay in `u128` until a nonzero contribution needs a
//! buffer index. Zero pieces record their operand read but neither allocate
//! storage nor impose an otherwise unnecessary addressability check.

use super::Digits;
use crate::accumulator::operand::Update;
use crate::accumulator::{digit_index, touch, DIGIT_BITS, DIGIT_MASK};

/// Deposit scaled inputs while preserving their individual metering rules.
impl Digits {
    /// Deposit an exact signed value at a bit offset without a wide intermediate.
    ///
    /// Splitting into base-2^32 pieces keeps each shifted contribution below
    /// 2^63. The caller's small value may have up to 127 magnitude bits;
    /// this path therefore costs a fixed number of deposits rather than work
    /// proportional to the shift.
    pub fn deposit_value(&mut self, value: i128, shift: u64) {
        let update = if value.is_negative() {
            Update::Subtract
        } else {
            Update::Add
        };
        self.deposit_magnitude(value.unsigned_abs(), update, shift);
    }

    /// Deposit a full-width magnitude without ever negating it in a signed type.
    pub fn deposit_magnitude(&mut self, mut magnitude: u128, update: Update, shift: u64) {
        let (digit_shift, bit_shift) =
            (shift / u64::from(DIGIT_BITS), shift % u64::from(DIGIT_BITS));
        let mut position = u128::from(digit_shift);
        while magnitude != 0 {
            touch(1);
            let digit = (magnitude & u128::from(DIGIT_MASK)) as i128;
            if digit != 0 {
                let contribution = digit << bit_shift;
                self.add_at(digit_index(position), update.apply(contribution));
            }
            magnitude >>= DIGIT_BITS;
            position += 1;
        }
    }

    /// Read an operand's stored signed digits and deposit them at a bit offset.
    ///
    /// Every stored position costs one operand read, including zero gaps.
    pub fn add_digits(&mut self, operand: &[i64], shift: u64, update: Update) {
        debug_assert!(!operand.is_empty());
        debug_assert!(
            operand
                .iter()
                .all(|&digit| i128::from(digit).abs() < super::DIGIT_LIMIT),
            "an accumulator operand preserves the digit bound"
        );
        let (digit_shift, bit_shift) =
            (shift / u64::from(DIGIT_BITS), shift % u64::from(DIGIT_BITS));
        for (offset, &digit) in operand.iter().enumerate() {
            touch(1);
            if digit != 0 {
                let contribution = i128::from(digit) << bit_shift;
                self.add_at(
                    digit_index(u128::from(digit_shift) + offset as u128),
                    update.apply(contribution),
                );
            }
        }
    }

    /// Deposit a wide little-endian limb stream directly at its bit offset.
    ///
    /// A limb supplies two base-2^32 contributions, each shifted by fewer
    /// than 32 bits. No operand copy is allocated, and every yielded limb,
    /// including zero limbs, records one operand touch.
    pub fn apply_limbs<I: Iterator<Item = u64>>(&mut self, limbs: I, update: Update, shift: u64) {
        let (digit_shift, bit_shift) =
            (shift / u64::from(DIGIT_BITS), shift % u64::from(DIGIT_BITS));
        // We count limbs in `u128` rather than with `enumerate`: a lazy stream
        // may yield more than `usize::MAX` limbs, and a `usize` counter would
        // then wrap on a 32-bit target built without overflow checks, landing
        // a later nonzero limb 2^33 digit positions below its true position
        // instead of rejecting that unaddressable position. No stream can
        // yield enough limbs to exhaust a `u128` counter.
        for (limb_index, limb) in (0_u128..).zip(limbs) {
            touch(1);
            // Each 32-bit half shifts by at most 31 bits, so both
            // contributions fit comfortably in the carry intermediate.
            let low = i128::from(limb & DIGIT_MASK) << bit_shift;
            let high = i128::from(limb >> DIGIT_BITS) << bit_shift;
            let position = u128::from(digit_shift) + 2 * limb_index;
            if low != 0 {
                self.add_at(digit_index(position), update.apply(low));
            }
            if high != 0 {
                self.add_at(digit_index(position + 1), update.apply(high));
            }
        }
    }
}
