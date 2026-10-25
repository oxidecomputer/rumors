//! Convert each input piece into a direct signed digit contribution.
//!
//! The bit offset splits into a whole-digit offset and fewer than 32 low bits.
//! A piece is shifted only by that remainder, then deposited at its destination
//! index. No shifted input buffer is built, so arithmetic work follows the
//! number of supplied pieces rather than the receiver's width or the offset.
//!
//! Each source is decomposed only enough to keep one contribution inside the
//! carry intermediate: small values use 32-bit magnitude pieces, 64-bit limbs
//! split in half, and another value's stored digits already satisfy the
//! bound, or stream as its magnitude digits at a nonzero bit offset. They all
//! pass through [`Digits::add_stored_at`], so no alternate write path can
//! bypass the digit or written-set invariants.
//!
//! Every public method here applies the receiver's flag once, on entry, to an
//! [`Update`] of the value, producing a [`StoredUpdate`]: the methods that
//! take an `Update` apply it at once, and
//! [`deposit_value`](Digits::deposit_value) derives its update from its
//! value's sign and passes it on. An accumulator operand's flag is applied to
//! the same update first. The private deposit helpers take only a
//! `StoredUpdate`.
//!
//! Position calculations stay in `u128` until a nonzero contribution needs a
//! buffer index. Zero pieces record their operand read but neither allocate
//! storage nor impose an otherwise unnecessary addressability check.

use core::cmp::Ordering;

use super::normalize::MagnitudeCarry;
use super::{Digits, StoredUpdate};
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
        let update = Update::from_sign(value.cmp(&0));
        self.deposit_magnitude(value.unsigned_abs(), update, shift);
    }

    /// Deposit a full-width magnitude without ever negating it in a signed type.
    pub fn deposit_magnitude(&mut self, mut magnitude: u128, update: Update, shift: u64) {
        let update = self.to_stored(update);
        let (digit_shift, bit_shift) =
            (shift / u64::from(DIGIT_BITS), shift % u64::from(DIGIT_BITS));
        let mut position = u128::from(digit_shift);
        while magnitude != 0 {
            touch(1);
            let digit = (magnitude & u128::from(DIGIT_MASK)) as i128;
            if digit != 0 {
                let contribution = digit << bit_shift;
                self.add_stored_at(digit_index(position), update.apply(contribution));
            }
            magnitude >>= DIGIT_BITS;
            position += 1;
        }
    }

    /// Deposit another nonzero value at a bit offset, given its digits and
    /// its sign.
    ///
    /// Every stored position costs one operand read, including zero gaps. At
    /// bit offset zero, each stored digit is deposited as it stands: one
    /// contribution of magnitude below `2B`, with `B = 2^32`, at offset zero.
    /// At any other offset, a stored digit whose sign opposes the value's
    /// could carry by itself once shifted, leaving the next digit to cancel
    /// that carry, so the value's magnitude is deposited instead, as
    /// [`add_magnitude`](Self::add_magnitude) deposits it.
    pub fn add_digits(&mut self, operand: &Digits, sign: Ordering, shift: u64, update: Update) {
        // The operand's stored digits sum to its value, negated when its flag
        // is set, so its flag is applied to the update before the receiver's.
        let update = self.to_stored(operand.oriented(update));
        let (sign, operand) = (operand.orient(sign), operand.stored_digits());
        debug_assert!(
            operand
                .iter()
                .all(|&digit| i128::from(digit).abs() < super::DIGIT_LIMIT),
            "an accumulator operand preserves the digit bound"
        );
        let (digit_shift, bit_shift) =
            (shift / u64::from(DIGIT_BITS), shift % u64::from(DIGIT_BITS));
        if bit_shift != 0 {
            self.add_stored_magnitude(operand, sign, shift, update);
            return;
        }
        for (offset, &digit) in operand.iter().enumerate() {
            touch(1);
            if digit != 0 {
                self.add_stored_at(
                    digit_index(u128::from(digit_shift) + offset as u128),
                    update.apply(i128::from(digit)),
                );
            }
        }
    }

    /// Deposit another nonzero value's magnitude, with the value's sign, at a
    /// bit offset, given its digits and its sign.
    ///
    /// The operand's flag is applied to the update, as in
    /// [`add_digits`](Self::add_digits), and its stored digits stream as
    /// [`add_stored_magnitude`](Self::add_stored_magnitude) describes.
    pub fn add_magnitude(&mut self, operand: &Digits, sign: Ordering, shift: u64, update: Update) {
        let update = self.to_stored(operand.oriented(update));
        self.add_stored_magnitude(operand.stored_digits(), operand.orient(sign), shift, update);
    }

    /// Deposit the magnitude of a nonzero stored sum, given its digits and
    /// the sum's sign, with that sign and a stored update, at a bit offset.
    ///
    /// The stored digits stream through [`MagnitudeCarry`] and are left as
    /// they are: each position below the stored top deposits the sum's
    /// magnitude digit there, below the base, and the top deposits the
    /// magnitude's whole remaining part, below `2^34`, so the deposits end
    /// where the stored digits do. Every stored position costs one operand
    /// read, including zero gaps. Every contribution has one sign, which is
    /// the case of the crate's cost argument in which an update writes each
    /// position a constant number of times; into zeroed digits, as a shift
    /// deposits, the result's top digit has the deposits' sign, and recentering
    /// leaves any digit of the other sign within `B / 2` of zero.
    fn add_stored_magnitude(
        &mut self,
        operand: &[i64],
        sign: Ordering,
        shift: u64,
        update: StoredUpdate,
    ) {
        let (digit_shift, bit_shift) =
            (shift / u64::from(DIGIT_BITS), shift % u64::from(DIGIT_BITS));
        // A magnitude digit carries the sum's sign, so subtracting a negative
        // sum adds its magnitude.
        let update = if sign == Ordering::Less {
            update.reversed()
        } else {
            update
        };
        let (&top, below) = operand
            .split_last()
            .expect("a nonzero value has a stored digit");
        let mut magnitude = MagnitudeCarry::new(sign);
        let mut position = u128::from(digit_shift);
        for &digit in below {
            touch(1);
            let piece = u64::from(magnitude.fold(digit));
            self.deposit_magnitude_piece(piece, position, bit_shift, update);
            position += 1;
        }
        touch(1);
        let piece = magnitude.fold_top(top);
        self.deposit_magnitude_piece(piece, position, bit_shift, update);
    }

    /// Deposit one piece of a magnitude, below `2^34` and shifted by fewer
    /// than 32 bits, at `position`; a zero piece deposits nothing.
    fn deposit_magnitude_piece(
        &mut self,
        piece: u64,
        position: u128,
        bit_shift: u64,
        update: StoredUpdate,
    ) {
        if piece != 0 {
            let contribution = i128::from(piece) << bit_shift;
            self.add_stored_at(digit_index(position), update.apply(contribution));
        }
    }

    /// Deposit a wide little-endian limb stream directly at its bit offset.
    ///
    /// A limb supplies two base-2^32 contributions, each shifted by fewer
    /// than 32 bits. No operand copy is allocated, and every yielded limb,
    /// including zero limbs, records one operand touch.
    pub fn apply_limbs<I: Iterator<Item = u64>>(&mut self, limbs: I, update: Update, shift: u64) {
        let update = self.to_stored(update);
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
            // contributions stay below 2^63, inside the carry intermediate.
            let low = i128::from(limb & DIGIT_MASK) << bit_shift;
            let high = i128::from(limb >> DIGIT_BITS) << bit_shift;
            let position = u128::from(digit_shift) + 2 * limb_index;
            if low != 0 {
                self.add_stored_at(digit_index(position), update.apply(low));
            }
            if high != 0 {
                self.add_stored_at(digit_index(position + 1), update.apply(high));
            }
        }
    }
}
