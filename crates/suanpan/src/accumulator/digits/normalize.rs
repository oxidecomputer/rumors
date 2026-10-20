//! Remove cancellation from the stored signed digits in one pass.
//!
//! Ordinary updates deliberately leave positive and negative digits mixed: that
//! is what makes a small update cheap. A magnitude read needs the magnitude's
//! unsigned digits, and writing them back lets the next read pay only for the
//! result's width. One pass does both: it carries from low to high inside the
//! existing buffer, and the read collects the digits it writes.
//!
//! Once the value's sign is known, multiplying every digit by that sign yields
//! the positive magnitude. Euclidean division by the digit base leaves an
//! unsigned remainder at each position and carries the quotient upward. The
//! stored remainder receives the original sign again. All surviving digits
//! therefore have the value's sign and magnitude below the base; cancellation
//! is gone and the highest position is the magnitude's exact digit width.
//!
//! The output can be much narrower, but it cannot be much wider. If the input
//! occupies `h + 1` positions, the digit bound gives
//!
//! `|value| < 2B(1 + B + ... + B^h) < B^(h + 2)`.
//!
//! The last inequality holds because `B = 2^32 > 3`. The normalized magnitude
//! therefore needs at most `h + 2` positions: normalization can shrink the
//! stored width by any amount, but can grow it by only one position. The pass
//! keeps the existing digit allocation because callers commonly reuse an
//! accumulator after reading it; no second value buffer is needed unless that
//! one possible new position forces the vector to grow.
//!
//! The comparison with zero that supplies the sign first compacts any
//! cancelled top, so the pass covers only the value's own width. The pass
//! writes every position it covers, so afterwards the written set holds
//! position zero and one run from the lowest written position to the top: a
//! later descent through one of the run's zeros is paid for by the pass's
//! write, as for any other write.

use core::cmp::Ordering;

use super::{Digits, DIGIT_BITS};
use crate::accumulator::touch;

/// Rewrite a wide value into same-signed base-2^32 digits without allocating.
impl Digits {
    /// Normalize the whole stored value in place and return its comparison
    /// with zero.
    ///
    /// The comparison compacts a cancelled top first, so the pass covers only
    /// the value's own width.
    pub fn normalize(&mut self) -> Ordering {
        let sign = self.cmp_zero();
        if sign != Ordering::Equal {
            self.normalize_nonzero_from(sign, 0, |_| {});
        }
        sign
    }

    /// Rewrite the digits from `start` upward as same-signed digits below the
    /// base, passing each magnitude digit to `emit` from low to high.
    ///
    /// `sign` must be the value's nonzero sign, and every digit below `start`
    /// must be zero; any `start` at or below `lowest_written` qualifies. The
    /// pass reads nothing below `start`, because skipping that prefix is what
    /// keeps a scaled read proportional to the written span; a nonzero digit
    /// there would be left out of the value. The tests in
    /// `accumulator::tests::representation` check the zero prefix below
    /// `lowest_written` after every step, and compare the scaled readout with
    /// an oracle. The emitted digits may end in zeros that the final trimming
    /// removes from the stored prefix.
    pub(super) fn normalize_nonzero_from(
        &mut self,
        sign: Ordering,
        start: usize,
        mut emit: impl FnMut(u32),
    ) {
        self.debug_assert_valid();
        debug_assert_ne!(sign, Ordering::Equal, "a zero needs no normalization");
        debug_assert!(start <= self.highest_nonzero);

        let polarity = if sign == Ordering::Less {
            -1_i128
        } else {
            1_i128
        };
        let base = 1_i128 << DIGIT_BITS;
        let old_highest = self.highest_nonzero;
        // The value is nonzero, so the floor is `lowest_written`, and every
        // digit below it is zero before and after the pass.
        let floor = self.written_floor();
        let mut carry = 0_i128;
        for digit in &mut self.digits[start..=old_highest] {
            touch(1);
            let magnitude_piece = polarity * i128::from(*digit) + carry;
            let remainder = magnitude_piece.rem_euclid(base);
            carry = magnitude_piece.div_euclid(base);
            *digit = (polarity * remainder) as i64;
            emit(remainder as u32);
        }

        debug_assert!(
            carry >= 0,
            "the remaining high part has the magnitude's sign"
        );
        let mut highest = old_highest;
        while carry != 0 {
            let remainder = carry.rem_euclid(base);
            carry = carry.div_euclid(base);
            highest = highest
                .checked_add(1)
                .expect("a normalized carry needs an addressable digit position");
            if highest == self.digits.len() {
                // Normalization adds at most this one position, so we grow by
                // exactly one rather than letting `push` double the buffer.
                self.digits.reserve_exact(1);
                self.digits.push(0);
            }
            touch(1);
            self.digits[highest] = (polarity * remainder) as i64;
            emit(remainder as u32);
        }
        debug_assert!(
            highest <= old_highest.saturating_add(1),
            "normalization grows by at most one digit"
        );

        while highest > 0 && self.digits[highest] == 0 {
            touch(1);
            highest -= 1;
        }
        self.highest_nonzero = highest;
        // The pass wrote every position from `start` to its last carry, and
        // every digit below the floor stays zero, so the set becomes position
        // zero and one run from the floor to the new top. Every position the
        // set held other than zero lies in `floor..=old_highest`, which the
        // pass covered, so clearing them stays within the pass's cost.
        self.written.clear(floor, old_highest);
        self.written.insert(0);
        self.written.insert_span(floor, highest);
        self.debug_assert_valid();
    }
}
