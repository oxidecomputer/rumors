//! Redundant signed digits for wide values.
//!
//! With `B = 2^32`, the stored digits represent
//!
//! `d[0] + d[1]B + d[2]B² + ...`.
//!
//! A digit may be negative, regardless of the whole value's sign, and its
//! magnitude is kept below `2B`. This deliberate redundancy is what makes the
//! accumulator useful: an update can change a few nearby digits without first
//! normalizing the entire integer. Subtraction is the same operation with
//! negative contributions, and negation simply negates every digit.
//!
//! Every nonzero contribution enters through [`Digits::add_at`]. If adding at
//! one position takes its digit outside the permitted range, `add_at` keeps a
//! centered remainder there and carries the rest upward. Centering leaves
//! enough distance to the limit that the same digit cannot carry repeatedly
//! without receiving substantial new input. Consequently, carry work across
//! a sequence is proportional to the input pieces supplied. One update can
//! still finish a carry chain prepared by earlier updates; the bound is
//! amortized over the sequence, not per call.
//!
//! Three facts about the stored form make later work bounded:
//!
//! - `highest_nonzero` names the highest nonzero position, or zero when the
//!   value is zero;
//! - every position below `lowest_written` is zero;
//! - `zero_ranges` records untouched zero ranges between populated positions.
//!
//! The first bounds ordinary reads. The second lets a scaled read omit a known
//! zero prefix. The third lets a descending scan cross a large shift in one
//! step instead of visiting every intervening zero. These are properties of
//! the stored form, not of the normalized magnitude.
//!
//! Comparisons with zero scan downward and rewrite the part they inspect so the same
//! cancellation is not paid for twice. A normalized magnitude is instead
//! produced in temporary unsigned storage, leaving these digits unchanged.

mod add;
mod normalize;
mod read;
mod sign;
#[cfg(test)]
mod tests;
mod zero_ranges;

use super::{touch, DIGIT_BITS};
use zero_ranges::ZeroRanges;

/// Twice the base: every stored digit has magnitude strictly below this limit.
const DIGIT_LIMIT: i128 = 1 << (DIGIT_BITS + 1);

/// Bias that rounds a carry so its remainder lies in `[-2^31, 2^31)`.
const RECENTER_BIAS: i128 = 1 << (DIGIT_BITS - 1);

/// Redundant signed digits plus the bounds that keep scans proportional.
///
/// The buffer may be empty while the accumulator uses its small representation.
/// Activating this representation creates digit zero if necessary; thereafter
/// every arithmetic or read method has a nonempty buffer.
/// Reset leaves all allocated digits zero and retains the buffer for reuse.
#[derive(Clone)]
pub struct Digits {
    /// Little-endian coefficients of powers of `2^32`.
    digits: Vec<i64>,
    /// Highest nonzero digit, or zero when every digit is zero.
    highest_nonzero: usize,
    /// Lowest position written since activation or reset, or `usize::MAX`.
    ///
    /// Cancellation does not raise this bound. A comparison with zero also
    /// deposits through `add_at`, and may lower it by one digit.
    lowest_written: usize,
    /// Disjoint known-zero ranges within the stored prefix.
    zero_ranges: ZeroRanges,
}

/// Maintain the signed digits and their known-zero ranges.
impl Digits {
    /// Construct inactive digit storage without allocating.
    pub fn new() -> Self {
        Self {
            digits: Vec::new(),
            highest_nonzero: 0,
            lowest_written: usize::MAX,
            zero_ranges: ZeroRanges::default(),
        }
    }

    /// Activate zeroed storage, retaining any reservation from an earlier use.
    pub fn activate(&mut self) {
        if self.digits.is_empty() {
            self.digits.reserve_exact(1);
            self.digits.push(0);
        }
        debug_assert!(
            self.digits.iter().all(|&digit| digit == 0),
            "inactive digit storage contains only zeros"
        );
        self.highest_nonzero = 0;
        self.lowest_written = usize::MAX;
        self.zero_ranges.clear();
        self.debug_assert_valid();
    }

    /// Reserve enough capacity for `count` digits without changing the value.
    pub fn reserve(&mut self, count: usize) {
        self.digits
            .reserve_exact(count.saturating_sub(self.digits.len()));
    }

    /// The stored prefix, including digit zero even when the value is zero.
    pub fn stored_digits(&self) -> &[i64] {
        self.debug_assert_valid();
        &self.digits[..=self.highest_nonzero]
    }

    /// Count stored positions without inspecting the buffer.
    pub fn stored_digit_count(&self) -> usize {
        self.highest_nonzero + 1
    }

    /// Append the stored representation to the accumulator's debug record.
    pub fn debug_fields(&self, fields: &mut core::fmt::DebugStruct<'_, '_>) {
        fields
            .field("digits", &self.digits)
            .field("highest_nonzero", &self.highest_nonzero)
            .field("lowest_written", &self.lowest_written)
            .field("zero_ranges", &self.zero_ranges);
    }

    /// Whether the representation itself contains only zeros.
    pub fn is_known_zero(&self) -> bool {
        self.highest_nonzero == 0 && self.digits[0] == 0
    }

    /// Negate each stored digit; the symmetric range needs no carries.
    pub fn negate(&mut self) {
        self.debug_assert_valid();
        for digit in &mut self.digits[..=self.highest_nonzero] {
            touch(1);
            *digit = -*digit;
        }
        self.debug_assert_valid();
    }

    /// Zero the stored prefix and discard its scan metadata, retaining capacity.
    pub fn reset(&mut self) {
        self.debug_assert_valid();
        for digit in &mut self.digits[..=self.highest_nonzero] {
            touch(1);
            *digit = 0;
        }
        self.highest_nonzero = 0;
        self.lowest_written = usize::MAX;
        self.zero_ranges.clear();
        self.debug_assert_valid();
    }

    /// Add a nonzero contribution at `pos`, restoring every digit invariant.
    ///
    /// Callers supply a nonzero contribution of magnitude at most `2^96`.
    /// That bound leaves `i128` headroom when the existing digit is added.
    pub fn add_at(&mut self, mut position: usize, mut contribution: i128) {
        // Sign compaction deliberately clears its old high suffix before
        // restoring the compacted contribution here. Storage is safe during
        // that interval, but `highest_nonzero` becomes exact only on return.
        self.debug_assert_storage();
        debug_assert_ne!(contribution, 0, "zero needs no digit write");
        debug_assert!(
            contribution.unsigned_abs() <= 1 << 96,
            "a contribution must fit the carry intermediate"
        );

        self.lowest_written = self.lowest_written.min(position);
        self.zero_ranges.record_gap(self.highest_nonzero, position);
        let first_written = position;

        // Each iteration fixes one digit. A zero carry ends the chain; a
        // nonzero carry advances exactly one position, so the written interval
        // remains contiguous and can update the zero ranges in one operation.
        loop {
            if position >= self.digits.len() {
                let new_len = position
                    .checked_add(1)
                    .expect("a digit position must have a representable buffer length");
                self.digits.resize(new_len, 0);
            }
            touch(1);
            let total = i128::from(self.digits[position]) + contribution;
            let (digit, carry) = Self::recenter(total);
            self.digits[position] = digit;

            if carry == 0 {
                if digit != 0 {
                    self.highest_nonzero = self.highest_nonzero.max(position);
                }
                break;
            }
            contribution = carry;
            position = position
                .checked_add(1)
                .expect("a carry must have a representable destination");
        }

        // The last position is nonzero whenever the carry rose above the old
        // highest digit, so it also bounds every remainder written below it.
        self.zero_ranges.remove_written(first_written, position);
        self.trim_high_zeros();
        self.debug_assert_valid();
    }

    /// Split a total into a bounded digit and the carry for the next position.
    fn recenter(total: i128) -> (i64, i128) {
        if total.abs() < DIGIT_LIMIT {
            return (total as i64, 0);
        }

        // Rounding the carry toward the nearest multiple of B leaves the
        // remainder in [-B/2, B/2), symmetrically for either sign because the
        // right shift is arithmetic.
        let carry = (total + RECENTER_BIAS) >> DIGIT_BITS;
        let remainder = total - (carry << DIGIT_BITS);
        debug_assert_ne!(carry, 0);
        debug_assert!((-RECENTER_BIAS..RECENTER_BIAS).contains(&remainder));
        debug_assert_eq!(total, remainder + (carry << DIGIT_BITS));
        (remainder as i64, carry)
    }

    /// Move `highest_nonzero` down to the highest remaining nonzero digit.
    ///
    /// A position is visited only after an update has written through it. A
    /// large untouched gap is skipped using the range recorded when the write
    /// jumped over it. Thus the descent is charged to preceding writes rather
    /// than to the numeric distance between positions.
    fn trim_high_zeros(&mut self) {
        while self.highest_nonzero > 0 && self.digits[self.highest_nonzero] == 0 {
            touch(1);
            self.highest_nonzero = match self.zero_ranges.take_below(self.highest_nonzero) {
                Some(lo) => lo,
                None => self.highest_nonzero - 1,
            };
        }
    }

    /// Check the constant-time representation invariants at method boundaries.
    #[inline]
    fn debug_assert_valid(&self) {
        self.debug_assert_storage();
        #[cfg(debug_assertions)]
        {
            debug_assert!(
                self.highest_nonzero == 0 || self.digits[self.highest_nonzero] != 0,
                "highest_nonzero names the last nonzero digit"
            );
        }
    }

    /// Check the weaker invariant needed while sign compaction is in flight.
    #[inline]
    fn debug_assert_storage(&self) {
        #[cfg(debug_assertions)]
        {
            debug_assert!(!self.digits.is_empty());
            debug_assert!(self.highest_nonzero < self.digits.len());
            debug_assert!(
                i128::from(self.digits[self.highest_nonzero]).abs() < DIGIT_LIMIT,
                "the recorded high digit stays within the redundant range"
            );
            if self.lowest_written != usize::MAX {
                debug_assert!(self.lowest_written < self.digits.len());
            }
        }
    }
}
