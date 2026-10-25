//! Structural assertions shared by the exhaustive and randomized schedules.

use core::cmp::Ordering;

use super::normalize::MagnitudeCarry;
use super::written::ClimbCounts;
use super::{Digits, DIGIT_LIMIT};

/// Inspect signed digits and their written set without exposing mutable fields.
impl Digits {
    /// Stream the stored digits through the Euclidean carry of a value of the
    /// nonzero sign `sign`, as [`add_magnitude`](Digits::add_magnitude)
    /// streams them, and return the digits it yields, from the lowest.
    pub fn streamed_magnitude_digits(&self, sign: Ordering) -> Vec<u32> {
        let mut magnitude = MagnitudeCarry::new(sign);
        let mut streamed: Vec<u32> = self
            .stored_digits()
            .iter()
            .map(|&digit| magnitude.fold(digit))
            .collect();
        streamed.extend(core::iter::from_fn(|| magnitude.next_above()));
        streamed
    }

    /// The positions the written set holds, in ascending order.
    #[cfg(feature = "touch-meter")]
    pub fn written_positions(&self) -> Vec<usize> {
        self.written.positions()
    }

    /// The words of bitset storage the written set retains.
    pub fn written_storage_words(&self) -> usize {
        self.written.storage_words()
    }

    /// Read the written set's bitset work counters.
    pub fn written_climb_counts(&self) -> ClimbCounts {
        self.written.climb_counts()
    }

    /// Check digit bounds, exact highest position, the unwritten prefix, and the written set.
    pub fn assert_invariants(&self, idle: bool, schedule: &[u8]) {
        self.written.assert_structure(&schedule);
        if idle {
            assert!(
                self.digits.iter().all(|&digit| digit == 0),
                "a scalar value leaves every retained digit zero after {schedule:?}"
            );
            assert!(
                matches!(self.written.positions()[..], [] | [0]),
                "a scalar value leaves no written position but zero after {schedule:?}"
            );
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
        let written = self.written.positions();
        assert_eq!(
            written.first(),
            Some(&0),
            "position zero is written while the digits are active, after {schedule:?}"
        );
        assert_eq!(
            written.last(),
            Some(&self.highest_nonzero),
            "the top is the highest written position after {schedule:?}"
        );
        assert!(
            written[1..]
                .iter()
                .all(|&position| position >= self.lowest_written),
            "a written position other than zero lies below watermark {}, outside the span \
             a normalizing read rewrites, after {schedule:?}",
            self.lowest_written
        );
        for (index, &digit) in self.digits[..=self.highest_nonzero].iter().enumerate() {
            assert!(
                digit == 0 || self.written.contains(index),
                "digit {index} = {digit} is nonzero but unwritten, so a scan would skip it, \
                 after {schedule:?}"
            );
        }
    }
}
