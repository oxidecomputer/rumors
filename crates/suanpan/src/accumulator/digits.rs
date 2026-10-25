//! Redundant signed digits for wide values.
//!
//! With `B = 2^32`, the stored digits represent
//!
//! `d[0] + d[1]B + d[2]B² + ...`.
//!
//! A digit may be negative, regardless of the whole value's sign, and its
//! magnitude is kept below `2B`. This redundancy lets an update change a few
//! nearby digits without first normalizing the entire integer. Subtraction is
//! the same operation with negative contributions.
//!
//! The value is that sum, or its negation when a flag kept beside the digits
//! is set, so negation flips the flag and writes no digit. The flag lives at
//! the boundary of [`Digits`]: its public methods take contributions and
//! return signs of the value, and apply the flag once there. An update
//! applies the flag to its [`Update`] before depositing, producing a
//! [`StoredUpdate`], which the deposit paths require; a read applies it to the
//! sign it returns; and an operand read applies the operand's flag to the
//! update it deposits with. Carrying, the scan behind a comparison with zero,
//! normalizing, and the written set work on the stored digits alone and
//! never see the flag. Activation and a clear reset the flag, and a copy
//! keeps it.
//!
//! Every nonzero contribution enters through [`Digits::add_stored_at`]. If
//! adding at one position takes its digit outside the permitted range,
//! `add_stored_at` keeps a centered remainder there and carries the rest
//! upward. Centering leaves enough distance to the limit that the same digit
//! cannot carry repeatedly without receiving substantial new input.
//! Consequently, carry work across a sequence is proportional to the input
//! pieces supplied. One update can still finish a carry chain prepared by
//! earlier updates; the bound is amortized over the sequence, not per call.
//!
//! These facts about the stored form make later work bounded:
//!
//! - `highest_nonzero` names the highest nonzero position, or zero when every
//!   digit is zero;
//! - every position below `lowest_written` is zero;
//! - `written` records which positions updates have written, and its field
//!   states what that guarantees about the digits.
//!
//! The first bounds ordinary reads. The second lets a scaled read omit a known
//! zero prefix. The third lets a descending scan cross a large shift in one
//! step instead of visiting every intervening zero. These are properties of
//! the stored form, not of the normalized magnitude. [`costs`] derives the
//! crate's time and space bounds from them and from the written set's
//! structure.
//!
//! Comparisons with zero scan downward and rewrite the part they inspect, so
//! the same cancellation is not paid for twice. A magnitude read likewise
//! normalizes the digits it reads in place.

mod add;
mod costs;
mod normalize;
mod read;
mod sign;
mod stored;
#[cfg(test)]
mod tests;
mod written;

use core::cmp::Ordering;

use super::operand::Update;
use super::{touch, DIGIT_BITS};
use stored::StoredUpdate;
use written::WrittenPositions;

/// Twice the base: every stored digit has magnitude strictly below this limit.
const DIGIT_LIMIT: i128 = 1 << (DIGIT_BITS + 1);

/// Bias that rounds a carry so its remainder lies in `[-2^31, 2^31)`.
const RECENTER_BIAS: i128 = 1 << (DIGIT_BITS - 1);

/// Redundant signed digits, a flag that negates their sum, and the bounds
/// that keep scans proportional.
///
/// The buffer may be empty while the accumulator uses its small representation.
/// Activating this representation creates digit zero if necessary; thereafter
/// every arithmetic or read method has a nonempty buffer.
/// A clear leaves all allocated digits zero and keeps the buffer for reuse.
#[derive(Clone)]
pub struct Digits {
    /// Little-endian coefficients of powers of `2^32`.
    digits: Vec<i64>,
    /// Whether the value is the negation of the digits' sum rather than the
    /// sum itself.
    ///
    /// [`negate`](Self::negate) flips it, and [`activate`](Self::activate),
    /// [`clear`](Self::clear), and [`discard`](Self::discard) reset it, so it
    /// is clear whenever the storage is inactive;
    /// [`copy_into`](Self::copy_into) copies it. Nothing else changes it.
    negated: bool,
    /// Highest nonzero digit, or zero when every digit is zero.
    highest_nonzero: usize,
    /// Lowest position written since activation, or `usize::MAX`.
    ///
    /// Cancellation does not raise this bound. A comparison with zero also
    /// deposits through `add_stored_at`, and may lower it by one digit.
    lowest_written: usize,
    /// Positions that updates have written, so a descending scan can cross
    /// the unwritten zeros between them in one step.
    ///
    /// At every completed operation:
    ///
    /// - every position at or below `highest_nonzero` that the set does not
    ///   hold is zero, which is what lets a scan skip it, and lets
    ///   [`clear`](Self::clear) zero only the positions the set holds; a held
    ///   position may hold zero after cancellation;
    /// - the set holds nothing above `highest_nonzero`;
    /// - while the buffer holds digits, the set holds position zero and
    ///   `highest_nonzero`, so a descent always finds a position to stop at;
    /// - apart from position zero, the set holds nothing below
    ///   `lowest_written`, so every position a normalizing read clears from
    ///   the set lies in the span its pass rewrote, which pays for clearing
    ///   it.
    ///
    /// Storage left by `clear` holds exactly position zero; storage emptied
    /// by [`discard`](Self::discard) holds nothing. Every set this field holds
    /// receives position zero as its first insertion, from
    /// [`activate`](Self::activate) or [`copy_into`](Self::copy_into); the
    /// set's [`clear`](WrittenPositions::clear) requires that.
    written: WrittenPositions,
}

/// Maintain the signed digits and the positions written into them.
impl Digits {
    /// Construct inactive digit storage without allocating.
    pub fn new() -> Self {
        Self {
            digits: Vec::new(),
            negated: false,
            highest_nonzero: 0,
            lowest_written: usize::MAX,
            written: WrittenPositions::default(),
        }
    }

    /// Activate zeroed storage, retaining any reservation from an earlier use.
    ///
    /// Every retained digit must already be zero. Inactive storage is either
    /// fresh, and so empty, or left by [`clear`](Self::clear), which zeroes
    /// every digit the written set holds, and so every nonzero one.
    /// Activation reinitializes only the scan metadata, so a stale digit would
    /// enter the next value. After a clear,
    /// `pooled_reuse_after_clear_matches_the_oracle` checks the reused
    /// accumulator against an oracle, and `complete_surface_matches_bigint`
    /// checks that the idle buffer holds only zeros.
    pub fn activate(&mut self) {
        if self.digits.is_empty() {
            self.digits.reserve_exact(1);
            self.digits.push(0);
        }
        debug_assert!(
            matches!(self.written.last_below(usize::MAX), None | Some(0)),
            "inactive digit storage holds no written position but zero"
        );
        self.negated = false;
        self.highest_nonzero = 0;
        self.lowest_written = usize::MAX;
        // Storage left by `clear` already holds position zero; empty storage
        // gains it here, as the first insertion into its set, which the set's
        // `clear` requires.
        self.written.insert(0);
        self.debug_assert_valid();
    }

    /// Reserve enough capacity for `count` digits without changing the value,
    /// ignoring a request that cannot be honored.
    pub fn reserve(&mut self, count: usize) {
        // The reservation is only a hint. A refused one, whether by capacity
        // overflow or by the allocator, leaves the buffer unchanged, and a
        // later write that needs the space grows the buffer itself.
        let _ = self
            .digits
            .try_reserve_exact(count.saturating_sub(self.digits.len()));
    }

    /// Return the update that applies `update` to the value by changing the
    /// stored digits' sum: `update` itself, or its reverse when the flag is
    /// set.
    ///
    /// Every deposit path takes the result, so each update has the flag
    /// applied once, where it enters.
    fn to_stored(&self, update: Update) -> StoredUpdate {
        StoredUpdate::new(update, self.negated)
    }

    /// Return the update that applies `update` of this value through its
    /// stored digits' sum: `update` itself, or its reverse when the flag is
    /// set.
    ///
    /// An operand read deposits the operand's stored digits with this update.
    fn oriented(&self, update: Update) -> Update {
        if self.negated {
            update.reversed()
        } else {
            update
        }
    }

    /// Convert between the sign of the stored digits' sum and the value's
    /// sign, which are opposite when the flag is set and equal when it is
    /// clear, and both `Equal` at zero; the conversion is its own inverse.
    fn orient(&self, sign: Ordering) -> Ordering {
        if self.negated {
            sign.reverse()
        } else {
            sign
        }
    }

    /// The stored prefix, including digit zero even when the value is zero.
    ///
    /// The digits sum to the value, or to its negation when the flag is set.
    pub fn stored_digits(&self) -> &[i64] {
        self.debug_assert_valid();
        &self.digits[..=self.highest_nonzero]
    }

    /// Count stored positions without inspecting the buffer.
    pub fn stored_digit_count(&self) -> usize {
        self.highest_nonzero + 1
    }

    /// Make `target` hold this stored prefix and flag as freshly written
    /// digits, reusing `target`'s allocation when it suffices.
    ///
    /// The copy holds no digits above the stored prefix, so its retained
    /// length is the prefix's. Its metadata records the copy's own writes
    /// rather than the source's history, so it is valid for any source;
    /// `clone` normalizes the source first, so the copy carries no
    /// cancellation.
    pub fn copy_into(&self, target: &mut Digits) {
        self.debug_assert_valid();
        let prefix = self.stored_digits();
        touch(prefix.len() as u64);
        target.digits.clear();
        target.digits.extend_from_slice(prefix);
        target.negated = self.negated;
        target.highest_nonzero = self.highest_nonzero;
        // The copy holds only the prefix, so its watermark is the floor
        // clamped into that prefix.
        target.lowest_written = self.written_floor();
        // The copy writes every position from that floor to its top, so its
        // set is position zero and that one run, held inline, and takes no
        // bitset sized to the source's. The target's old set is replaced
        // rather than cleared, so the copy keeps no bitset storage sized to
        // the target's former width. Position zero goes in first, as the
        // set's `clear` requires.
        target.written = WrittenPositions::default();
        target.written.insert(0);
        target
            .written
            .insert_span(target.lowest_written, target.highest_nonzero);
        target.debug_assert_valid();
    }

    /// Discard every digit and the written set, so that the storage is
    /// inactive and empty, retaining only the buffer's capacity.
    ///
    /// The written set is replaced rather than cleared, releasing any bitset,
    /// because the storage this leaves behaves as new: the next write grows
    /// the buffer from position zero.
    pub fn discard(&mut self) {
        self.digits.clear();
        self.negated = false;
        self.highest_nonzero = 0;
        self.lowest_written = usize::MAX;
        self.written = WrittenPositions::default();
    }

    /// Count the digit positions the buffer holds, including zeros above the
    /// stored prefix.
    ///
    /// Tests use this to measure the space an operation leaves behind, which
    /// the stored digit count understates after cancellation.
    #[cfg(test)]
    pub fn retained_len(&self) -> usize {
        self.digits.len()
    }

    /// Append the stored representation to the accumulator's debug record.
    pub fn debug_fields(&self, fields: &mut core::fmt::DebugStruct<'_, '_>) {
        fields
            .field("digits", &self.digits)
            .field("negated", &self.negated)
            .field("highest_nonzero", &self.highest_nonzero)
            .field("lowest_written", &self.lowest_written)
            .field("written", &self.written);
    }

    /// Whether the representation itself contains only zeros.
    pub fn is_known_zero(&self) -> bool {
        self.highest_nonzero == 0 && self.digits[0] == 0
    }

    /// Negate the value by flipping the flag, in O(1), writing no digit.
    pub fn negate(&mut self) {
        touch(1);
        self.negated = !self.negated;
    }

    /// Zero every digit, clear the flag, and reduce the written set to
    /// position zero, keeping the buffer and the set's storage.
    ///
    /// Only the positions the set holds are zeroed: every digit above
    /// `highest_nonzero` is zero, and so is every digit at or below it that
    /// the set does not hold, as the field `written` guarantees. Each held
    /// position was put there by a write, which pays for zeroing it once, so a
    /// clear costs amortized O(1), however wide the positions it zeroes lie
    /// apart; the written set's [`clear`](WrittenPositions::clear) explains
    /// the cost of finding them.
    pub fn clear(&mut self) {
        self.debug_assert_valid();
        let digits = &mut self.digits;
        // The set passes every position but zero, which it keeps, as the
        // field requires while the buffer holds digits.
        touch(1);
        digits[0] = 0;
        self.written.clear(|position| {
            touch(1);
            digits[position] = 0;
        });
        self.negated = false;
        self.highest_nonzero = 0;
        self.lowest_written = usize::MAX;
        self.debug_assert_valid();
    }

    /// The lowest position that can hold a nonzero digit, never above the top.
    ///
    /// Every digit below `lowest_written` is zero, and cancellation can leave
    /// that watermark above `highest_nonzero`, so the floor clamps it to the
    /// top. Every digit below the floor is zero, and the floor never exceeds
    /// `highest_nonzero`, so `floor..=highest_nonzero` is the span a scaled
    /// read or a copy must cover.
    fn written_floor(&self) -> usize {
        self.lowest_written.min(self.highest_nonzero)
    }

    /// Add a nonzero contribution to the value at `position`, applying the
    /// flag to it.
    ///
    /// Callers supply a nonzero contribution of magnitude at most `2^96`, as
    /// [`add_stored_at`](Self::add_stored_at) requires. The flag is tested by
    /// a branch, and the negating path kept out of line, because the flag is
    /// usually clear: computing the negation and selecting would cost every
    /// unflagged deposit a 128-bit negation it discards.
    pub fn add_at(&mut self, position: usize, contribution: i128) {
        if self.negated {
            self.add_negated_at(position, contribution);
        } else {
            self.add_stored_at(position, contribution);
        }
    }

    /// Add the negation of a nonzero contribution to the stored digits' sum
    /// at `position`: [`add_at`](Self::add_at)'s path for flagged digits.
    #[cold]
    #[inline(never)]
    fn add_negated_at(&mut self, position: usize, contribution: i128) {
        self.add_stored_at(position, -contribution);
    }

    /// Add a nonzero contribution to the stored digits' sum at `position`,
    /// restoring every digit invariant.
    ///
    /// The contribution is as stored, with the flag already applied: a
    /// [`StoredUpdate`] applied it, or it is a comparison's write-back of
    /// stored digits. Callers supply a nonzero contribution of magnitude at
    /// most `2^96`. That bound leaves `i128` headroom when the existing digit
    /// is added.
    fn add_stored_at(&mut self, mut position: usize, mut contribution: i128) {
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
        let first_written = position;

        // Each iteration fixes one digit. A zero carry ends the chain; a
        // nonzero carry advances exactly one position, so the written
        // positions form one interval and join the written set in one
        // operation.
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

        // A jump above the top leaves the positions it skips out of the set,
        // which is what lets a later descending scan cross them in one step.
        // The last position is nonzero whenever the carry rose above the old
        // top, so the whole interval lies at or below the new one.
        self.written.insert_span(first_written, position);
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
    #[inline]
    fn trim_high_zeros(&mut self) {
        if self.highest_nonzero > 0 && self.digits[self.highest_nonzero] == 0 {
            self.descend_to_nonzero();
        }
    }

    /// Descend from a zero top digit to the highest nonzero one.
    ///
    /// A position is visited only after an update has written through it, and
    /// a gap of unwritten zeros is crossed in one query. Thus the descent is
    /// charged to preceding writes rather than to the numeric distance between
    /// positions. The positions it passes leave the written set together once
    /// it stops, since a query below a position ignores everything above it.
    ///
    /// Kept out of line so that a write whose top digit stays nonzero, the
    /// common case, carries only the test in [`trim_high_zeros`](Self::trim_high_zeros).
    #[inline(never)]
    fn descend_to_nonzero(&mut self) {
        while self.highest_nonzero > 0 && self.digits[self.highest_nonzero] == 0 {
            touch(1);
            self.highest_nonzero = self
                .written
                .last_below(self.highest_nonzero)
                .expect("position zero stays written, and lies below the top");
        }
        self.written.truncate(self.highest_nonzero);
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
            debug_assert!(
                self.written.contains(0) && self.written.contains(self.highest_nonzero),
                "position zero and the top stay written once the buffer is allocated"
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
