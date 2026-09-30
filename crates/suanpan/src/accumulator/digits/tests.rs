//! Structural assertions shared by the exhaustive and randomized schedules.

use super::{Digits, DIGIT_LIMIT};

/// Inspect signed digits and zero ranges without exposing mutable fields.
impl Digits {
    /// Check digit bounds, exact highest position, the unwritten prefix, and zero ranges.
    pub fn assert_invariants(&self, idle: bool, schedule: &[u8]) {
        if idle {
            assert!(
                self.digits.iter().all(|&digit| digit == 0),
                "a scalar value leaves every retained digit zero after {schedule:?}"
            );
            self.zero_ranges.assert_empty(schedule);
            return;
        }
        assert!(
            self.highest_nonzero < self.digits.len(),
            "top {} outside buffer length {} after {schedule:?}",
            self.highest_nonzero,
            self.digits.len()
        );
        for (index, &digit) in self.digits.iter().enumerate() {
            assert!(
                i128::from(digit).abs() < DIGIT_LIMIT,
                "digit {index} = {digit} outside the permitted signed range after {schedule:?}"
            );
        }
        assert!(
            self.digits[self.highest_nonzero + 1..]
                .iter()
                .all(|&digit| digit == 0),
            "nonzero digit above top {} after {schedule:?}",
            self.highest_nonzero
        );
        assert!(
            self.highest_nonzero == 0 || self.digits[self.highest_nonzero] != 0,
            "top {} rests on a zero digit after {schedule:?}",
            self.highest_nonzero
        );
        let floor = self.lowest_written.min(self.digits.len());
        assert!(
            self.digits[..floor].iter().all(|&digit| digit == 0),
            "nonzero digit below watermark {} after {schedule:?}",
            self.lowest_written
        );
        self.zero_ranges
            .assert_invariants(&self.digits, self.highest_nonzero, schedule);
    }
}
