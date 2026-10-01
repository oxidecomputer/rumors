//! The accumulator's two representations and the boundary between them.
//!
//! Small values live directly in an `i128`. Once an operation needs more
//! range, the value moves into redundant signed digits. It stays there until
//! reset, even if cancellation later makes the value small again. This
//! one-way transition prevents an alternating workload from repeatedly paying
//! to convert the same value.
//!
//! Every update to the wide representation passes through [`Digits`]. That
//! single write boundary maintains the digit bounds and the information that
//! later comparisons with zero need to skip untouched zero ranges.

mod conversions;
mod digits;
mod operand;
mod operators;
mod small;

use core::cmp::Ordering;
use digits::Digits;
use operand::Update;
use small::limbs_from_u128;

/// Owned normalized limbs, inline for small values and allocated when wide.
enum MagnitudeLimbs {
    /// Up to two limbs stored inside the return value.
    Inline { limbs: [u64; 2], len: usize },
    /// Arbitrarily wide normalized limbs.
    Wide(Vec<u64>),
}

/// Expose normalized limbs without exposing their storage choice.
impl AsRef<[u64]> for MagnitudeLimbs {
    fn as_ref(&self) -> &[u64] {
        match self {
            Self::Inline { limbs, len } => &limbs[..*len],
            Self::Wide(limbs) => limbs,
        }
    }
}

/// Record digit work; compile away completely without the meter feature.
#[inline(always)]
fn touch(count: u64) {
    #[cfg(feature = "touch-meter")]
    crate::touch_meter::record(count);
    #[cfg(not(feature = "touch-meter"))]
    let _ = count;
}

/// Bits in the positional base shared by storage and operand conversion.
const DIGIT_BITS: u32 = 32;

/// Mask selecting the low base-2^32 piece of a 64-bit operand limb.
const DIGIT_MASK: u64 = (1 << DIGIT_BITS) - 1;

/// A running signed integer supporting exact updates and cheap comparison with zero.
///
/// Use `+=` and `-=` with primitive integers or accumulator operands, and
/// `<<=` with a nonnegative integer shift. Dedicated methods accept shifted
/// operands and streams of 64-bit limbs without first constructing them.
/// Comparing with zero may reduce the working width without changing the value.
/// Read a normalized magnitude with [`signed_magnitude`](Self::signed_magnitude).
///
/// See the [crate documentation](crate) for operation costs and input guidance.
/// `Default` creates zero without allocating. Cloning and debug formatting
/// take time and space linear in the retained allocation, which can exceed the
/// current working width after cancellation.
/// [`reset`](Self::reset) retains that allocation for reuse.
#[derive(Clone)]
pub struct Accumulator {
    /// An exact small value, or `None` once the digit representation is active.
    ///
    /// While this is `Some`, `digits` is empty or contains only zeros.
    small: Option<i128>,
    /// Retained digit storage and known zero ranges, active when `small` is `None`.
    digits: Digits,
}

/// Keep the representation choice behind one arithmetic interface.
impl Accumulator {
    /// Create a zero accumulator without allocating.
    pub fn new() -> Accumulator {
        Accumulator {
            small: Some(0),
            digits: Digits::new(),
        }
    }

    /// Add little-endian 64-bit limbs multiplied by `2^shift`.
    ///
    /// The stream is consumed directly without a normalized or shifted copy.
    /// An empty stream changes nothing. No operand copy is allocated. High
    /// zero limbs are permitted, but every yielded limb contributes to the
    /// input length used by the complexity bound.
    ///
    /// # Complexity
    ///
    /// Using the quantities defined in the [crate-level cost
    /// model](crate#costs-and-storage), this takes amortized
    /// O(`L` log(`W` + 1) + `G`) time and adds O(`L` + `G`) retained space in
    /// the worst case. Growing the receiver may temporarily retain both its old
    /// and replacement allocations.
    ///
    /// # Panics
    ///
    /// Panics if a nonzero contribution would land at or beyond `usize::MAX`,
    /// where the required retained span is unrepresentable.
    pub fn add_shifted_limbs<I: IntoIterator<Item = u64>>(&mut self, shift: u64, limbs: I) {
        self.apply_limbs(limbs.into_iter(), shift, Update::Add);
    }

    /// Subtract little-endian 64-bit limbs multiplied by `2^shift`.
    ///
    /// The stream is consumed directly. An empty stream changes nothing. High
    /// zero limbs are permitted, but every yielded limb contributes to `L` in
    /// the [crate-level cost model](crate#costs-and-storage). This takes
    /// amortized O(`L` log(`W` + 1) + `G`) time and adds
    /// O(`L` + `G`) retained space in the worst case. Growing the receiver may
    /// temporarily retain both its old and replacement allocations.
    ///
    /// # Panics
    ///
    /// Panics if a nonzero contribution would land at or beyond `usize::MAX`,
    /// where the required working width is unrepresentable.
    pub fn sub_shifted_limbs<I: IntoIterator<Item = u64>>(&mut self, shift: u64, limbs: I) {
        self.apply_limbs(limbs.into_iter(), shift, Update::Subtract);
    }

    /// Add another accumulator's value multiplied by `2^shift`.
    ///
    /// The operand is read directly; shifting it first would construct another
    /// accumulator merely to consume it here.
    ///
    /// # Complexity
    ///
    /// Using the quantities defined in the [crate-level cost
    /// model](crate#costs-and-storage), this takes amortized
    /// O(`A` log(`W` + 1) + `G`) time and adds O(`A` + `G`) retained space in
    /// the worst case. Growing the receiver may temporarily retain both its old
    /// and replacement allocations.
    ///
    /// # Panics
    ///
    /// Panics if a nonzero contribution would land at or beyond `usize::MAX`,
    /// where the required working width is unrepresentable.
    pub fn add_shifted(&mut self, shift: u64, other: &Accumulator) {
        self.apply_accumulator(other, shift, Update::Add);
    }

    /// Subtract another accumulator's value multiplied by `2^shift`.
    ///
    /// The operand is read directly. Using the quantities defined in the
    /// [crate-level cost model](crate#costs-and-storage), this takes amortized
    /// O(`A` log(`W` + 1) + `G`) time and adds O(`A` + `G`) retained space in
    /// the worst case. Growing the receiver may temporarily retain both its old
    /// and replacement allocations.
    ///
    /// # Panics
    ///
    /// Panics if a nonzero contribution would land at or beyond `usize::MAX`,
    /// where the required working width is unrepresentable.
    pub fn sub_shifted(&mut self, shift: u64, other: &Accumulator) {
        self.apply_accumulator(other, shift, Update::Subtract);
    }

    /// Reset to zero, retaining allocated storage for reuse.
    ///
    /// Clearing takes work proportional to the working width. Replacing the
    /// accumulator with [`new`](Self::new) releases its storage instead.
    ///
    /// # Complexity
    ///
    /// O(`W`) time and O(1) additional space, where `W` is the working width
    /// before the reset. The accumulator retains its main allocation.
    pub fn reset(&mut self) {
        if self.small.is_none() {
            self.digits.reset();
        }
        self.small = Some(0);
    }

    /// Reserve capacity for at least `digits` base-2^32 digit positions.
    ///
    /// This allocation hint does not change the value. Reserving the expected
    /// width avoids repeated allocation growth and its transient memory use.
    /// [`reset`](Self::reset) retains the reservation; a nontrivial
    /// `<<=` may release it.
    ///
    /// # Complexity
    ///
    /// O(1) time and space if the existing allocation suffices. Otherwise this
    /// performs at most one allocation and may copy retained storage;
    /// allocator cost, retained growth, and temporary space depend on the old
    /// and requested capacities.
    pub fn reserve_digits(&mut self, digits: usize) {
        self.digits.reserve(digits);
    }

    /// Compare the exact value with zero.
    ///
    /// The query may reduce the working width without changing the value. This
    /// is why it takes `&mut self`.
    ///
    /// # Complexity
    ///
    /// Amortized O(log(`W` + 1)) time and O(1) additional space, where `W` is
    /// the greatest working width reached. One call may finish cancellation left
    /// by earlier updates; the bound holds over the sequence that produced that
    /// work.
    #[inline]
    pub fn cmp_zero(&mut self) -> Ordering {
        if let Some(value) = self.small {
            touch(1);
            return value.cmp(&0);
        }
        self.digits.cmp_zero()
    }

    /// Reduce the working width after cancellation.
    ///
    /// Cancellation can leave the working width much larger than the result's
    /// bit length. This method makes the two agree. It is useful before
    /// repeatedly cloning or reading such a value, and retains allocated
    /// capacity for later updates.
    ///
    /// # Complexity
    ///
    /// Let `W` be the working width before normalization and `Q` the width
    /// afterward. Cancellation can make `Q` arbitrarily smaller than `W`, but
    /// the bounded stored digits guarantee `Q <= W + 32`.
    ///
    /// The operation takes O(`W` + `Q` log(`Q` + 1)) time and O(1) arithmetic
    /// scratch. It rewrites the existing allocation instead of constructing a
    /// normalized copy, and deliberately retains that O(`W`) capacity for
    /// later updates even when `Q` is much smaller. The rebuilt skip metadata
    /// occupies O(`Q`) space, within the same O(`W`) retained bound. If the one
    /// possible new digit exceeds the existing capacity, reallocation may
    /// temporarily retain both O(`W`) buffers.
    pub fn normalize(&mut self) {
        if self.small.is_none() {
            self.digits.normalize();
        }
    }

    /// Try to compare with zero without first applying a smaller operand.
    ///
    /// `bits` describes the operand that might be added or subtracted next.
    /// `Some(ordering)` means the current value is so much larger that every
    /// integer with magnitude below `2^bits` leaves that ordering unchanged.
    /// The result is exact and is always `Less` or `Greater`.
    ///
    /// To compare against another accumulator, pass its working-width bound
    /// from [`stored_bits`](Self::stored_bits). For example, a
    /// `Some(Greater)` result guarantees that both adding and subtracting the
    /// other accumulator leave this value positive.
    ///
    /// `None` makes no claim about the answer: the smaller operand may or may
    /// not change it. The scan nevertheless compacts what it reads. When `bits`
    /// comes from another accumulator's [`stored_bits`](Self::stored_bits), a
    /// `None` result leaves this accumulator at most two base-2^32 digit
    /// positions (64 working bits) wider than that operand. If compaction
    /// reveals that the other accumulator is now wider, the caller can retry
    /// with their roles reversed. Once neither is much wider, apply the operand
    /// and call [`cmp_zero`](Self::cmp_zero) to decide.
    ///
    /// ```
    /// use core::cmp::Ordering;
    /// use suanpan::Accumulator;
    ///
    /// let mut total = Accumulator::new();
    /// total.add_shifted_limbs(300, [1]);
    /// assert_eq!(total.cmp_zero_stable_under(128), Some(Ordering::Greater));
    /// // Any value fitting below 2^128 is too small to make `total` nonpositive.
    /// ```
    ///
    /// # Complexity
    ///
    /// Amortized O(log(`W` + 1)) time and O(1) additional space, where `W` is
    /// the greatest working width reached. As with [`cmp_zero`](Self::cmp_zero),
    /// this may finish work prepared by earlier updates.
    pub fn cmp_zero_stable_under(&mut self, bits: u64) -> Option<Ordering> {
        // Whole digits are the unit used by both representations. At least one
        // digit is retained even for a zero-width adjustment.
        let adjustment_digits = bits.div_ceil(u64::from(DIGIT_BITS)).max(1);
        let adjustment_high = usize::try_from(adjustment_digits - 1).ok()?;

        if let Some(value) = self.small {
            touch(1);
            // Three times the first power above the adjustment covers the
            // extra range of a redundant top digit as well as an ordinary
            // magnitude. Overflow merely means this small value cannot prove
            // stability at the requested width.
            let threshold_bits = u32::try_from(adjustment_digits.checked_mul(32)?).ok()?;
            let threshold = 3u128.checked_shl(threshold_bits)?;
            return (value.unsigned_abs() >= threshold).then(|| value.cmp(&0));
        }
        self.digits.cmp_zero_stable_above(adjustment_high)
    }

    /// Return whether zero can be established without scanning the value.
    ///
    /// `true` guarantees zero; `false` makes no claim. Use
    /// [`cmp_zero`](Self::cmp_zero) for an exact zero test. After that returns
    /// `Equal`, this method returns `true`.
    ///
    /// ```
    /// use core::cmp::Ordering;
    /// use suanpan::Accumulator;
    ///
    /// let mut acc = Accumulator::new();
    /// acc.add_shifted_limbs(32, [1]);
    /// acc -= 1_i64 << 32;
    /// assert!(!acc.is_known_zero()); // The constant-time check need not see cancellation.
    /// assert_eq!(acc.cmp_zero(), Ordering::Equal);
    /// assert!(acc.is_known_zero());
    /// ```
    ///
    /// # Complexity
    ///
    /// O(1) time and space, without scanning or rewriting the value.
    #[inline]
    pub fn is_known_zero(&self) -> bool {
        match self.small {
            Some(value) => value == 0,
            None => self.digits.is_known_zero(),
        }
    }

    /// Return the working width in 32-bit units, at least one.
    ///
    /// The working width covers every bit position the accumulator still has
    /// to retain, including gaps. It can exceed the result's bit length after
    /// cancellation. Prefer [`stored_bits`](Self::stored_bits) unless an
    /// allocation calculation specifically needs 32-bit units.
    ///
    /// # Complexity
    ///
    /// O(1) time and space.
    #[inline]
    pub fn stored_digit_count(&self) -> usize {
        match self.small {
            Some(0) => 1,
            Some(value) => (128 - value.unsigned_abs().leading_zeros() as usize).div_ceil(32),
            None => self.digits.stored_digit_count(),
        }
    }

    /// Return `32 * stored_digit_count()`.
    ///
    /// This is the accumulator's working width, not the result's bit length.
    /// The result's magnitude is less than `2.01 * 2^stored_bits()`.
    ///
    /// # Complexity
    ///
    /// O(1) time and space.
    #[inline]
    pub fn stored_bits(&self) -> u64 {
        u64::try_from(self.stored_digit_count())
            .expect("an allocated accumulator's digit count fits u64")
            .checked_mul(u64::from(DIGIT_BITS))
            .expect("an allocated accumulator's bit span fits u64")
    }

    /// Return the comparison with zero and normalized magnitude limbs.
    ///
    /// The little-endian 64-bit limbs have no high zero word and are empty
    /// exactly when the comparison is `Equal`. This read does not change the
    /// value or its working width.
    ///
    /// # Complexity
    ///
    /// If the result has `Q` limbs, this takes O(`W` + `Q`) time, O(`W`)
    /// temporary space, and O(`Q`) returned space, where `W` is the working
    /// width.
    pub fn signed_magnitude(&self) -> (Ordering, impl AsRef<[u64]>) {
        if let Some(value) = self.small {
            touch(self.stored_digit_count() as u64);
            let (limbs, len) = limbs_from_u128(value.unsigned_abs());
            return (value.cmp(&0), MagnitudeLimbs::Inline { limbs, len });
        }
        let (ordering, limbs) = self.digits.normalized_limbs();
        (ordering, MagnitudeLimbs::Wide(limbs))
    }

    /// Return the comparison with zero, magnitude limbs, and scale.
    ///
    /// The exact value is `±magnitude * 2^shift`. As in
    /// [`signed_magnitude`](Self::signed_magnitude), the magnitude is minimal
    /// and little-endian; zero has no limbs. This read does not change the
    /// accumulator. The method may factor out a low span known to be zero from
    /// write history, avoiding a scan of that span. The factor need not be
    /// maximal: cancellation can leave low zero bits in the magnitude.
    ///
    /// # Complexity
    ///
    /// If the result has `Q` limbs, this takes O(`R` + `Q`) time, O(`R`)
    /// temporary space, and O(`Q`) returned space. `R` spans the lowest
    /// relevant update since construction or reset through the top of the
    /// working range, including gaps.
    pub fn scaled_signed_magnitude(&self) -> (Ordering, impl AsRef<[u64]>, u64) {
        if let Some(value) = self.small {
            touch(self.stored_digit_count() as u64);
            let (limbs, len) = limbs_from_u128(value.unsigned_abs());
            return (value.cmp(&0), MagnitudeLimbs::Inline { limbs, len }, 0);
        }
        let (ordering, limbs, shift) = self.digits.normalized_limbs_with_shift();
        (ordering, MagnitudeLimbs::Wide(limbs), shift)
    }
}

/// Convert a mathematical digit position into an addressable buffer index.
fn digit_index(position: u128) -> usize {
    usize::try_from(position)
        .ok()
        .filter(|&index| index < usize::MAX)
        .expect("a nonzero contribution needs an addressable digit position")
}

/// Construct a zero accumulator without allocating.
impl Default for Accumulator {
    /// Return the same initial value as [`Accumulator::new`].
    fn default() -> Accumulator {
        Accumulator::new()
    }
}

/// Format the accumulator's diagnostic state, including retained storage.
impl core::fmt::Debug for Accumulator {
    /// Include every field needed to diagnose arithmetic and storage behavior.
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut fields = formatter.debug_struct("Accumulator");
        fields.field("small", &self.small);
        self.digits.debug_fields(&mut fields);
        fields.finish()
    }
}

#[cfg(test)]
mod tests;
