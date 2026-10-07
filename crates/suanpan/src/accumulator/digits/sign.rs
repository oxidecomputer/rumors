//! Compare with zero without repeatedly scanning the same cancellation.
//!
//! Let `B = 2^32`. Starting at the highest digit, the scan maintains the
//! exact value of the digits already read, expressed in units of the current
//! position. Moving down one position replaces `partial` with
//! `partial * B + digit`.
//!
//! Every unread lower digit has magnitude below `2B`, so together they
//! contribute less than `2B / (B - 1) < 2.01` in the same units. Once the
//! partial's magnitude reaches 3, no lower digits can reverse its sign. If it
//! never does, the scan reaches position zero and has the exact value.
//!
//! Merely reading the digits would make repeated comparisons with zero
//! traverse the same cancellation. Instead, the scan clears each position it incorporates
//! and writes the final partial back at the position where it stops. This
//! preserves the integer while replacing the inspected suffix with its
//! compact equivalent. Each individually visited position was written by an
//! earlier update; untouched zero ranges are skipped in one step. The work of
//! comparisons with zero is therefore amortized over the updates that made it
//! necessary.
//!
//! The stopping position can also show that an adjustment of bounded width
//! cannot change the sign. A decision at position `i` leaves magnitude greater
//! than `0.99 B^i`; an adjustment occupying positions through `f` has
//! magnitude below `2.01 B^(f + 1)`. Thus `i >= f + 2` is sufficient. Failure
//! to meet that test says only that this scan did not establish the guarantee.

use core::cmp::Ordering;

use super::{super::touch, Digits, DIGIT_BITS};

/// Smallest whole partial whose ordering no lower digits can overturn.
const COMPARISON_DECIDED: i128 = 3;

/// Compare with zero while preserving both value and the amortized bound.
impl Digits {
    /// Compare the exact value with zero, compacting cancellation along the way.
    pub fn cmp_zero(&mut self) -> Ordering {
        self.compact_until_order_known().1.cmp(&0)
    }

    /// Return the comparison when lower-width adjustments cannot change it.
    ///
    /// `adjustment_high` is the highest digit position the adjustment may
    /// occupy. It is a mathematical position, not a buffer index, so it stays
    /// in `u64`: the scan runs and compacts for every bound on every target,
    /// and a bound beyond every addressable index always yields `None`.
    pub fn cmp_zero_stable_above(&mut self, adjustment_high: u64) -> Option<Ordering> {
        let (index, partial) = self.compact_until_order_known();
        let index = u64::try_from(index).expect("an allocated digit position fits u64");
        // A bound near `u64::MAX` must still yield `None`; wrapping
        // adjustment_high + 2 would instead create a small threshold.
        (partial.abs() >= COMPARISON_DECIDED && index >= adjustment_high.saturating_add(2))
            .then(|| partial.cmp(&0))
    }

    /// Return the stopping position and replace the scanned suffix by its value.
    ///
    /// A zero partial can skip a known-zero range unchanged. A nonzero partial
    /// reaches the decision threshold at the first zero digit in a gap.
    /// Only digits the fold has incorporated are cleared.
    fn compact_until_order_known(&mut self) -> (usize, i128) {
        self.debug_assert_valid();
        let previous_highest = self.highest_nonzero;
        let mut index = previous_highest;
        let mut partial: i128 = 0;
        loop {
            debug_assert!(partial.abs() < COMPARISON_DECIDED);
            touch(1);
            partial = (partial << DIGIT_BITS) + i128::from(self.digits[index]);
            if partial.abs() >= COMPARISON_DECIDED || index == 0 {
                break;
            }
            // Descending: this digit's value lives in `partial` now;
            // zero it so the floor re-deposit preserves the value.
            self.digits[index] = 0;
            touch(1);
            if partial == 0 {
                if let Some(lo) = self.zero_ranges.take_below(index) {
                    debug_assert!(lo < index);
                    // Multiplying zero by any skipped powers of B still gives
                    // zero, so no arithmetic is needed across this range.
                    index = lo;
                    continue;
                }
            }
            index -= 1;
        }
        if index < previous_highest {
            // Everything above this position is now represented by `partial`.
            // Replace this digit as well so one ordinary deposit can restore
            // the exact scanned value and all representation invariants.
            self.digits[index] = 0;
            touch(1);
            self.highest_nonzero = index;
            if partial != 0 {
                self.add_at(index, partial);
            } else {
                // No digits remain above or below the final zero partial.
                self.zero_ranges.clear();
            }
        }
        self.debug_assert_valid();
        (index, partial)
    }
}
