//! Route operands without normalizing them or allocating shifted copies.
//!
//! A primitive value is added directly while the accumulator and result fit
//! the small representation. A limb iterator is read only far enough to
//! distinguish empty, one-word, and wider input: one word shares the same
//! path, while a wider stream is deposited directly into signed digits.
//!
//! Another accumulator is likewise read as stored. A small operand enters as
//! one exact value; a digit-stored operand is read through its highest nonzero
//! position and deposited at the requested offset. Work therefore follows the
//! operand's stored width, not the receiver's width.
//!
//! An operand's stored digits can cancel to zero. Before a deposit that would
//! extend the receiver's buffer, the operand is scanned, and a zero operand is
//! skipped. Adding a zero at a shift therefore never grows the buffer toward
//! the shift. A zero deposited inside the buffer can still carry one position
//! past its top.
//!
//! Position arithmetic uses `u128` until the contribution is known nonzero.
//! Only then does [`digit_index`] validate its destination. Zero contributions
//! therefore neither grow the buffer nor reject an otherwise unusable shift.

use super::small::{SMALL_MAX, SMALL_SHIFT_MAX};
use super::{digit_index, touch, Accumulator, DIGIT_BITS};

/// Whether a contribution is added to or subtracted from the accumulator.
#[derive(Clone, Copy)]
pub enum Update {
    /// Add the contribution as supplied.
    Add,
    /// Subtract the contribution as supplied.
    Subtract,
}

impl Update {
    /// Apply this update to a bounded positive or signed contribution.
    pub fn apply(self, contribution: i128) -> i128 {
        debug_assert_ne!(
            contribution,
            i128::MIN,
            "input decomposition keeps each contribution safely negatable"
        );
        match self {
            Update::Add => contribution,
            Update::Subtract => -contribution,
        }
    }
}

/// Add each operand in the form that avoids normalization and copying.
impl Accumulator {
    /// Add a full-width integer without forcing it through signed `i128`.
    ///
    /// Smaller primitives use `add_word` and its proven headroom instead.
    /// A `u128` magnitude also represents the absolute value of `i128::MIN`,
    /// so subtraction never needs an overflowing signed negation.
    pub(crate) fn apply_magnitude(&mut self, magnitude: u128, update: Update) {
        if magnitude == 0 {
            return;
        }
        if let Some(value) = self.small {
            touch(1);
            let sum = match update {
                Update::Add => value.checked_add_unsigned(magnitude),
                Update::Subtract => value.checked_sub_unsigned(magnitude),
            };
            if let Some(sum) = sum {
                if sum.unsigned_abs() <= SMALL_MAX {
                    self.small = Some(sum);
                } else {
                    self.start_digits();
                    self.digits.deposit_value(sum, 0);
                }
                return;
            }
        }
        self.ensure_digits();
        self.digits.deposit_magnitude(magnitude, update, 0);
    }

    /// Add a widened, possibly negated 64-bit operand without splitting it.
    ///
    /// One whole contribution preserves the word path's carry behavior and
    /// meter count. Zero operands do not touch either representation.
    #[inline]
    pub(crate) fn add_word(&mut self, delta: i128) {
        if delta != 0 && !self.try_small_add(delta) {
            self.digits.add_at(0, delta);
        }
    }

    /// Add or subtract a shifted accumulator by reading its stored form.
    pub(crate) fn apply_accumulator(&mut self, other: &Accumulator, shift: u64, update: Update) {
        if let Some(operand_value) = other.small {
            touch(1);
            if operand_value == 0 {
                return;
            }
            let operand_value = update.apply(operand_value);
            if shift == 0 {
                if !self.try_small_add(operand_value) {
                    self.digits.add_at(0, operand_value);
                }
            } else if shift > SMALL_SHIFT_MAX || !self.try_small_add(operand_value << shift) {
                self.ensure_digits();
                self.digits.deposit_value(operand_value, shift);
            }
            return;
        }
        let operand = other.digits.stored_digits();
        // Depositing a zero's cancelling digits past the buffer would grow it
        // to the shifted position, and panic if that position is unaddressable.
        // We scan only when the deposit would extend the buffer: one inside it
        // already costs the operand's width, so a scan there would only add
        // cost.
        if self.digits.deposit_extends_buffer(shift, operand.len()) && other.digits.value_is_zero()
        {
            return;
        }
        self.ensure_digits();
        self.digits.add_digits(operand, shift, update);
    }

    /// Add or subtract a shifted word, staying small when headroom permits.
    pub(crate) fn apply_shifted_word(&mut self, word: u64, shift: u64, update: Update) {
        if word == 0 {
            return;
        }
        if shift <= SMALL_SHIFT_MAX {
            // At most 94 bits shifted: inside the small value's headroom.
            let value = i128::from(word) << shift;
            let value = update.apply(value);
            if self.try_small_add(value) {
                return;
            }
        } else {
            self.ensure_digits();
        }
        let (digit_shift, bit_shift) =
            (shift / u64::from(DIGIT_BITS), shift % u64::from(DIGIT_BITS));
        // At most 96 bits after the sub-digit shift: well inside `i128`.
        let value = i128::from(word) << bit_shift;
        self.digits
            .add_at(digit_index(u128::from(digit_shift)), update.apply(value));
    }

    /// Peek at most two limbs so the common one-word case stays allocation-free.
    ///
    /// Replaying the two peeked limbs with `chain` keeps the stream lazy and
    /// allocation-free. High zero limbs still select the wide path and are
    /// metered like all other yielded limbs.
    pub(crate) fn apply_limbs<I: Iterator<Item = u64>>(
        &mut self,
        mut limbs: I,
        shift: u64,
        update: Update,
    ) {
        let Some(first) = limbs.next() else {
            return;
        };
        let Some(second) = limbs.next() else {
            self.apply_shifted_word(first, shift, update);
            return;
        };

        self.ensure_digits();
        self.digits.apply_limbs(
            core::iter::once(first)
                .chain(core::iter::once(second))
                .chain(limbs),
            update,
            shift,
        );
    }
}
