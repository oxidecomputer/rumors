//! Remove cancellation from the stored signed digits in one pass.
//!
//! Ordinary updates deliberately leave positive and negative digits mixed: that
//! is what makes a small update cheap. Some callers eventually need the stored
//! width to follow the value's actual magnitude. Rebuilding through normalized
//! limbs would allocate two temporary vectors and then deposit the same value
//! again. This pass instead carries from low to high inside the existing buffer.
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

use core::cmp::Ordering;

use super::{Digits, DIGIT_BITS};
use crate::accumulator::touch;

/// Rewrite a wide value into same-signed base-2^32 digits without allocating.
impl Digits {
    /// Remove cancellation while retaining the allocated digit buffer.
    pub fn normalize(&mut self) {
        self.debug_assert_valid();
        let sign = self.cmp_zero();
        if sign == Ordering::Equal {
            return;
        }

        let polarity = if sign == Ordering::Less {
            -1_i128
        } else {
            1_i128
        };
        let base = 1_i128 << DIGIT_BITS;
        let old_highest = self.highest_nonzero;
        let mut carry = 0_i128;
        for digit in &mut self.digits[..=old_highest] {
            touch(1);
            let magnitude_piece = polarity * i128::from(*digit) + carry;
            let remainder = magnitude_piece.rem_euclid(base);
            carry = magnitude_piece.div_euclid(base);
            *digit = (polarity * remainder) as i64;
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
                self.digits.push(0);
            }
            touch(1);
            self.digits[highest] = (polarity * remainder) as i64;
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
        self.zero_ranges.rebuild(&self.digits[..=highest]);
        self.debug_assert_valid();
    }
}
