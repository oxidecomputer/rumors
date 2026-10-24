//! Remove cancellation from the stored signed digits in one pass.
//!
//! Ordinary updates leave positive and negative digits mixed, which keeps a
//! small update cheap. A magnitude read needs the magnitude's unsigned
//! digits, and writing them back lets the next read pay only for the result's
//! width. One pass does both: it carries from low to high inside the existing
//! buffer, and the read collects the digits it writes.
//!
//! The pass works on the stored digits' sum, which is the value or, when the
//! flag is set, its negation; it neither reads nor changes the flag. Once the
//! sum's sign is known, multiplying every digit by that sign yields the
//! positive magnitude. Euclidean division by the digit base leaves an
//! unsigned remainder at each position and carries the quotient upward. The
//! stored remainder receives the original sign again. All surviving digits
//! therefore have the sum's sign and magnitude below the base; cancellation
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
use crate::accumulator::{touch, DIGIT_MASK};

/// The Euclidean carry that turns stored signed digits of a nonzero sum,
/// read from the lowest position up, into the digits of its magnitude.
///
/// The sum is the stored digits' own, so a flagged value's digits fold with
/// the sign of the sum and not of the value.
///
/// At each position, the stored digit times the sum's sign, plus the carry
/// from below, is a piece of the magnitude. Its Euclidean remainder by the
/// base `B = 2^32` is the magnitude's digit there, never negative and below
/// the base, and its quotient carries upward. For a power-of-two base these
/// are the piece's low 32 bits and its arithmetic right shift, and every
/// piece fits an `i64`: a stored digit has magnitude below `2B`, so the
/// carry stays at most 3 in magnitude. Once every stored digit is folded in,
/// the carry is nonnegative, because the sum has that sign, and holds the
/// magnitude's digits above the stored ones: at most one, since a sum stored
/// in `h + 1` positions has magnitude below `B^(h + 2)`.
///
/// [`normalize_nonzero_from`](Digits::normalize_nonzero_from) writes the
/// digits back in place. [`add_magnitude`](Digits::add_magnitude) deposits
/// them into another value and leaves the stored digits as they are; at the
/// top stored position it takes the whole remaining magnitude as one piece,
/// so that its deposits end where the stored digits do.
pub(super) struct MagnitudeCarry {
    /// 1 for a positive sum, -1 for a negative one.
    polarity: i64,
    /// The magnitude's part not yet returned, in units of the next position.
    carry: i64,
}

/// Fold stored digits into magnitude digits, low to high.
impl MagnitudeCarry {
    /// Start the carry for a sum of the nonzero sign `sign`.
    pub(super) fn new(sign: Ordering) -> Self {
        debug_assert_ne!(sign, Ordering::Equal, "a zero has no magnitude digits");
        let polarity = if sign == Ordering::Less { -1 } else { 1 };
        Self { polarity, carry: 0 }
    }

    /// Fold in the stored digit at the next position and return the
    /// magnitude's digit there.
    pub(super) fn fold(&mut self, digit: i64) -> u32 {
        let piece = self.polarity * digit + self.carry;
        self.carry = piece >> DIGIT_BITS;
        (piece & DIGIT_MASK as i64) as u32
    }

    /// Fold in the top stored digit and return the magnitude's whole
    /// remaining part, in units of the top position: its digit there and
    /// every digit above, as one piece.
    ///
    /// The piece is never negative, because the sum has the sign the carry
    /// was started with, and below `2^34`, because the stored digit is below
    /// `2B` in magnitude and the carry at most 3. Nothing remains above it.
    pub(super) fn fold_top(&mut self, digit: i64) -> u64 {
        let piece = self.polarity * digit + self.carry;
        self.carry = 0;
        u64::try_from(piece).expect("the remaining high part has the magnitude's sign")
    }

    /// Return the magnitude's next digit above the stored ones, once every
    /// stored digit is folded in, or `None` when the carry is spent.
    pub(super) fn next_above(&mut self) -> Option<u32> {
        debug_assert!(
            self.carry >= 0,
            "the remaining high part has the magnitude's sign"
        );
        (self.carry != 0).then(|| self.fold(0))
    }

    /// Return the stored digit of the sum's sign whose magnitude is
    /// `magnitude_digit`.
    pub(super) fn signed(&self, magnitude_digit: u32) -> i64 {
        self.polarity * i64::from(magnitude_digit)
    }
}

/// Rewrite a wide value into same-signed base-2^32 digits without allocating.
impl Digits {
    /// Normalize the whole stored value in place and return the value's
    /// comparison with zero.
    ///
    /// The comparison compacts a cancelled top first, so the pass covers only
    /// the value's own width.
    pub fn normalize(&mut self) -> Ordering {
        let sign = self.cmp_stored_zero();
        if sign != Ordering::Equal {
            self.normalize_nonzero_from(sign, 0, |_| {});
        }
        self.orient(sign)
    }

    /// Rewrite the digits from `start` upward as same-signed digits below the
    /// base, passing each magnitude digit to `emit` from low to high.
    ///
    /// `sign` must be the stored digits' nonzero sign, the value's sign
    /// reversed when the flag is set, and every digit below `start`
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

        let old_highest = self.highest_nonzero;
        // The value is nonzero, so the floor is `lowest_written`, and every
        // digit below it is zero before and after the pass.
        let floor = self.written_floor();
        let mut magnitude = MagnitudeCarry::new(sign);
        for digit in &mut self.digits[start..=old_highest] {
            touch(1);
            let magnitude_digit = magnitude.fold(*digit);
            *digit = magnitude.signed(magnitude_digit);
            emit(magnitude_digit);
        }

        // The magnitude is below `B^(old_highest + 2)`, as the module
        // documentation derives, so at most one digit lies above the stored
        // ones: the carry left after the top is at most 3, below the base.
        let mut highest = old_highest;
        if let Some(magnitude_digit) = magnitude.next_above() {
            highest = highest
                .checked_add(1)
                .expect("a normalized carry needs an addressable digit position");
            if highest == self.digits.len() {
                // This is the one position normalization can add, so we grow
                // by exactly one rather than letting `push` double the buffer.
                self.digits.reserve_exact(1);
                self.digits.push(0);
            }
            touch(1);
            self.digits[highest] = magnitude.signed(magnitude_digit);
            emit(magnitude_digit);
            debug_assert_eq!(
                magnitude.next_above(),
                None,
                "a carry below the base leaves nothing above its digit"
            );
        }

        while highest > 0 && self.digits[highest] == 0 {
            touch(1);
            highest -= 1;
        }
        self.highest_nonzero = highest;
        // The pass wrote every position from `start` to its last carry, and
        // every digit below the floor stays zero, so the set becomes position
        // zero, which the clear keeps, and one run from the floor to the new
        // top. The digits the clear passes were just rewritten and stay as
        // they are. Every position the set held other than zero lies in
        // `floor..=old_highest`, which the pass covered, so clearing them
        // stays within the pass's cost.
        self.written.clear(|_| {});
        self.written.insert_span(floor, highest);
        self.debug_assert_valid();
    }
}
