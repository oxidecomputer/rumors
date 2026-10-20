//! The accumulator's two representations and the boundary between them.
//!
//! Small values live directly in an `i128`. Once an operation needs more
//! range, the value moves into redundant signed digits. It stays there until
//! reset, even if cancellation later makes the value small again. This
//! one-way transition prevents an alternating workload from repeatedly paying
//! to convert the same value. An owned operator that reuses its other
//! operand's storage returns that operand's representation instead.
//!
//! Every update to the wide representation passes through
//! [`Digits`](digits::Digits). That single write boundary maintains the digit
//! bounds and the information that later comparisons with zero need to skip
//! untouched zero ranges.
//!
//! [`Accumulator`] holds its [`Value`] in a `RefCell`, so that reads, which
//! compact what they touch, can take `&self`.

mod compare;
mod conversions;
mod digits;
mod format;
mod limbs;
mod operand;
mod operators;
mod small;
mod value;

use core::cell::RefCell;
use core::cmp::Ordering;
use operand::Update;
use value::Value;

pub use limbs::Limbs;

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

/// A running signed integer with exact updates and amortized reads.
///
/// An accumulator behaves as a standard integer for addition, subtraction,
/// negation, left shifts, comparison, equality, hashing, and formatting, with
/// owned, borrowed, and primitive operands. Dedicated methods add shifted
/// accumulators and streams of 64-bit limbs without first constructing them.
/// There is no multiplication, division, or right shift.
///
/// Every read compacts the cancellation it touches, so no later operation pays
/// for it again. A read can therefore change the stored form, and with it the
/// working width that [`stored_bits`](Self::stored_bits) reports, but never
/// the value. Reads take `&self`: the stored form lives in a [`RefCell`]. As a
/// result an accumulator is [`Send`] but not [`Sync`].
///
/// Cloning normalizes the source in place first, so a copy starts with no
/// deferred cancellation and costs the value's own width, whatever the
/// source's working width.
///
/// ```compile_fail
/// fn shared<T: Sync>(_: &T) {}
/// shared(&suanpan::Accumulator::new()); // no `Sync`: reads go through a `RefCell`
/// ```
///
/// See the [crate documentation](crate) for operation costs and input guidance.
/// `Default` creates zero without allocating. [`reset`](Self::reset) clears
/// the value and retains its allocation for reuse.
pub struct Accumulator {
    /// The value, in a cell so that reads through `&self` can compact it.
    ///
    /// Only [`read`](Self::read) and [`peek`](Self::peek) borrow the cell,
    /// each for one call of a closure that runs `Value` code, so no borrow
    /// outlives its call or is held while another accumulator method or
    /// caller code runs. An operation on two accumulators nests their borrows
    /// only after it has checked that they differ.
    value: RefCell<Value>,
}

/// Copy the value, normalizing the source in place first.
///
/// The copy holds the value in its normalized form, with no deferred
/// cancellation, so its amortized costs hold from its own operations alone.
/// Normalizing changes the source's stored form, as any read may: its
/// [`stored_bits`](Accumulator::stored_bits) can shrink, never its value.
///
/// ```
/// use suanpan::Accumulator;
///
/// let mut total = Accumulator::new();
/// // A top that cancels across two positions, held until something reads it.
/// total.add_shifted_limbs(4_096, [1]);
/// total.sub_shifted_limbs(4_064, [1 << 32]);
/// total += 7;
/// let cancelled_width = total.stored_bits();
/// let copy = total.clone();
/// assert_eq!(copy, 7);
/// assert!(copy.stored_bits() < cancelled_width); // the cancelled span is not copied
/// ```
///
/// # Complexity
///
/// The cost of a magnitude read, amortized O(log(`W` + 1) + `Q`) time, plus
/// O(`Q`) for the copy, where `W` is the source's greatest working width and
/// `Q` the value's width. [`clone_from`](Clone::clone_from) reuses the
/// receiver's allocation when it suffices.
impl Clone for Accumulator {
    fn clone(&self) -> Self {
        let mut copy = Value::new();
        self.read(|value| value.clone_normalized_into(&mut copy));
        Accumulator::from_value(copy)
    }

    fn clone_from(&mut self, source: &Self) {
        let target = self.value.get_mut();
        source.read(|value| value.clone_normalized_into(target));
    }
}

/// Construction, updates, and reads.
impl Accumulator {
    /// Create a zero accumulator without allocating.
    pub fn new() -> Accumulator {
        Accumulator::from_value(Value::new())
    }

    /// Wrap a value.
    fn from_value(value: Value) -> Accumulator {
        Accumulator {
            value: RefCell::new(value),
        }
    }

    /// Run `f` on the value for a read that may compact it.
    ///
    /// # Panics
    ///
    /// Panics if the cell is already borrowed, which no method of this crate
    /// leaves possible: see the field's documentation.
    fn read<R>(&self, f: impl FnOnce(&mut Value) -> R) -> R {
        f(&mut self.value.borrow_mut())
    }

    /// Run `f` on the value for a read that leaves it as stored.
    fn peek<R>(&self, f: impl FnOnce(&Value) -> R) -> R {
        f(&self.value.borrow())
    }

    /// Borrow the stored value for a test to inspect.
    #[cfg(test)]
    fn stored(&self) -> core::cell::Ref<'_, Value> {
        self.value.borrow()
    }

    /// Add little-endian 64-bit limbs multiplied by `2^shift`.
    ///
    /// The stream is consumed directly, without a normalized or shifted copy
    /// of the operand. An empty stream changes nothing. High zero limbs are
    /// permitted, but every yielded limb counts toward `L` in the cost below.
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
    /// Panics if the limbs are nonzero and the result needs more storage than
    /// a buffer can address. Zero limbs never panic.
    pub fn add_shifted_limbs<I: IntoIterator<Item = u64>>(&mut self, shift: u64, limbs: I) {
        self.value
            .get_mut()
            .apply_limbs(limbs.into_iter(), shift, Update::Add);
    }

    /// Subtract little-endian 64-bit limbs multiplied by `2^shift`.
    ///
    /// The stream is consumed directly, without a normalized or shifted copy
    /// of the operand. An empty stream changes nothing. High zero limbs are
    /// permitted, but every yielded limb counts toward `L` in the cost below.
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
    /// Panics if the limbs are nonzero and the result needs more storage than
    /// a buffer can address. Zero limbs never panic.
    pub fn sub_shifted_limbs<I: IntoIterator<Item = u64>>(&mut self, shift: u64, limbs: I) {
        self.value
            .get_mut()
            .apply_limbs(limbs.into_iter(), shift, Update::Sub);
    }

    /// Add another accumulator's value multiplied by `2^shift`.
    ///
    /// The operand is read directly, without first shifting it into another
    /// accumulator. The read may reduce `other`'s working width without
    /// changing its value. The receiver is borrowed mutably, so `other` is
    /// always a different accumulator:
    ///
    /// ```compile_fail
    /// let mut total = suanpan::Accumulator::from(3);
    /// total.add_shifted(1, &total);
    /// ```
    ///
    /// # Complexity
    ///
    /// Using the quantities defined in the [crate-level cost
    /// model](crate#costs-and-storage), this takes amortized
    /// O(`A` log(`W` + 1) + `G`) time and adds O(`A` + `G`) retained space in
    /// the worst case. Growing the receiver may temporarily retain both its old
    /// and replacement allocations. A zero operand adds O(1) retained space at
    /// any shift.
    ///
    /// # Panics
    ///
    /// Panics if `other` is nonzero and the result needs more storage than a
    /// buffer can address. A zero operand never panics.
    pub fn add_shifted(&mut self, shift: u64, other: &Accumulator) {
        let value = self.value.get_mut();
        other.read(|operand| value.apply_accumulator(operand, shift, Update::Add));
    }

    /// Subtract another accumulator's value multiplied by `2^shift`.
    ///
    /// The operand is read directly, without first shifting it into another
    /// accumulator. The read may reduce `other`'s working width without
    /// changing its value. The receiver is borrowed mutably, so `other` is
    /// always a different accumulator:
    ///
    /// ```compile_fail
    /// let mut total = suanpan::Accumulator::from(3);
    /// total.sub_shifted(1, &total);
    /// ```
    ///
    /// # Complexity
    ///
    /// Using the quantities defined in the [crate-level cost
    /// model](crate#costs-and-storage), this takes amortized
    /// O(`A` log(`W` + 1) + `G`) time and adds O(`A` + `G`) retained space in
    /// the worst case. Growing the receiver may temporarily retain both its old
    /// and replacement allocations. A zero operand adds O(1) retained space at
    /// any shift.
    ///
    /// # Panics
    ///
    /// Panics if `other` is nonzero and the result needs more storage than a
    /// buffer can address. A zero operand never panics.
    pub fn sub_shifted(&mut self, shift: u64, other: &Accumulator) {
        let value = self.value.get_mut();
        other.read(|operand| value.apply_accumulator(operand, shift, Update::Sub));
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
        self.value.get_mut().reset();
    }

    /// Reserve storage for a working width of at least `bits` bits, if the
    /// request can be honored.
    ///
    /// This allocation hint does not change the value, and the reservation may
    /// round the request up. Reserving the expected width avoids repeated
    /// allocation growth and its transient memory use.
    /// [`reset`](Self::reset) retains the reservation; a nontrivial
    /// `<<=` may release it.
    ///
    /// A request that cannot be honored, because it is too large to address
    /// or because the allocator refuses it, is ignored: the call returns
    /// normally and keeps the existing allocation, and later updates grow
    /// storage as they would without the hint.
    ///
    /// ```
    /// use suanpan::Accumulator;
    ///
    /// let mut total = Accumulator::new();
    /// // Reserve once for the width the following write reaches.
    /// total.reserve_bits(4_096);
    /// total.add_shifted_limbs(4_000, [1]);
    /// assert!(total.stored_bits() >= 4_001);
    ///
    /// // No platform can address this much storage, so the hint is ignored:
    /// // the call returns normally and the value is unchanged.
    /// total.reserve_bits(u64::MAX);
    /// total.sub_shifted_limbs(4_000, [1]);
    /// assert!(total.is_zero());
    /// ```
    ///
    /// # Complexity
    ///
    /// O(1) time and space if the existing allocation suffices. Otherwise this
    /// makes at most one allocation attempt. A successful attempt may copy
    /// retained storage; allocator cost, retained growth, and temporary space
    /// depend on the old and requested capacities. An ignored request retains
    /// no additional space.
    pub fn reserve_bits(&mut self, bits: u64) {
        self.value.get_mut().reserve_bits(bits);
    }

    /// Return the value's ordering against zero.
    ///
    /// The read may reduce the working width without changing the value.
    ///
    /// # Complexity
    ///
    /// Amortized O(log(`W` + 1)) time and O(1) additional space, where `W` is
    /// the greatest working width reached. One call may finish cancellation left
    /// by earlier updates; the bound holds over the sequence that produced that
    /// work.
    #[inline]
    pub fn sign(&self) -> Ordering {
        self.read(Value::cmp_zero)
    }

    /// Return whether the value is zero.
    ///
    /// This is [`sign`](Self::sign)` == Ordering::Equal`.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.sign() == Ordering::Equal
    }

    /// Return whether the value is greater than zero.
    ///
    /// This is [`sign`](Self::sign)` == Ordering::Greater`.
    #[inline]
    pub fn is_positive(&self) -> bool {
        self.sign() == Ordering::Greater
    }

    /// Return whether the value is less than zero.
    ///
    /// This is [`sign`](Self::sign)` == Ordering::Less`.
    #[inline]
    pub fn is_negative(&self) -> bool {
        self.sign() == Ordering::Less
    }

    /// Return the order of `self` against `other`, and the magnitude of
    /// their difference.
    ///
    /// Like the primitive integers' `abs_diff`, the second element is
    /// `|self − other|`; the first says which operand was larger, so the
    /// signed difference is recoverable. The comparison and the subtraction
    /// are one pass: the wider operand's storage receives the narrower one,
    /// and the result's sign decides the order. Comparing first and then
    /// subtracting would read the operands twice, or subtract into a
    /// temporary.
    ///
    /// ```
    /// use core::cmp::Ordering;
    /// use suanpan::Accumulator;
    ///
    /// let (order, gap) = Accumulator::from(3).abs_diff(Accumulator::from(10));
    /// assert_eq!(order, Ordering::Less);
    /// assert_eq!(gap, 7);
    /// ```
    ///
    /// # Complexity
    ///
    /// Amortized O(`A` log(`W` + 1)) time, where `A` is the narrower operand's
    /// working width and `W` the wider's, with O(1) additional space beyond
    /// the wider operand's storage, which the result keeps. When the wider
    /// operand is the smaller value, the result is negated in place, which
    /// costs its working width after the sign read: O(`A`) when both operands
    /// are nonnegative, since the result is then below the narrower operand's
    /// magnitude, and up to O(`W`), the result's own width, when the wider
    /// operand is negative.
    pub fn abs_diff(self, other: Accumulator) -> (Ordering, Accumulator) {
        let (mut wide, narrow, swapped) = if other.stored_digit_count() > self.stored_digit_count()
        {
            (other, self, true)
        } else {
            (self, other, false)
        };
        wide -= &narrow;
        let order = wide.sign();
        if order == Ordering::Less {
            wide = -wide;
        }
        (if swapped { order.reverse() } else { order }, wide)
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
    /// other accumulator leave this value positive. Comparing two accumulators
    /// with `<` or [`cmp`](Ord::cmp) uses this query to avoid reading the
    /// narrower one when the widths differ greatly.
    ///
    /// `None` makes no claim about the answer: the smaller operand may or may
    /// not change it. The scan nevertheless compacts what it reads. When `bits`
    /// comes from another accumulator's [`stored_bits`](Self::stored_bits), a
    /// `None` result leaves this accumulator's working width at most 64 bits
    /// wider than that operand's.
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
    /// the greatest working width reached. As with [`sign`](Self::sign), this
    /// may finish work prepared by earlier updates.
    pub fn cmp_zero_stable_under(&self, bits: u64) -> Option<Ordering> {
        self.read(|value| value.cmp_zero_stable_under(bits))
    }

    /// Return whether zero can be established without scanning the value.
    ///
    /// `true` guarantees zero; `false` makes no claim. Use
    /// [`is_zero`](Self::is_zero) for an exact zero test. After that returns
    /// `true`, this method returns `true`.
    ///
    /// ```
    /// use suanpan::Accumulator;
    ///
    /// let mut acc = Accumulator::new();
    /// acc.add_shifted_limbs(32, [1]);
    /// acc -= 1_i64 << 32;
    /// assert!(!acc.is_known_zero()); // The constant-time check need not see cancellation.
    /// assert!(acc.is_zero());
    /// assert!(acc.is_known_zero());
    /// ```
    ///
    /// # Complexity
    ///
    /// O(1) time and space, without scanning or rewriting the value.
    #[inline]
    pub fn is_known_zero(&self) -> bool {
        self.peek(Value::is_known_zero)
    }

    /// Return the working width in the representation's digits, at least one.
    #[inline]
    pub(crate) fn stored_digit_count(&self) -> usize {
        self.peek(Value::stored_digit_count)
    }

    /// Return the working width in bits: every bit position the accumulator
    /// still retains, including gaps.
    ///
    /// This is not the value's bit length, which it can far exceed after
    /// cancellation; a read compacts it toward the value's own width. Whatever
    /// the stored form, the value's magnitude is below `2^(stored_bits() + 2)`.
    /// The width's granularity is unspecified.
    ///
    /// # Complexity
    ///
    /// O(1) time and space.
    #[inline]
    pub fn stored_bits(&self) -> u64 {
        self.peek(Value::stored_bits)
    }

    /// Return the comparison with zero and the magnitude's [`Limbs`].
    ///
    /// The limbs are minimal and empty exactly when the comparison is
    /// `Equal`. The read keeps the normalized form it computes, which has the
    /// same value in a working width that can be far narrower and is at most
    /// a constant wider, so a repeated read costs only the result's width.
    ///
    /// # Complexity
    ///
    /// If the result has `Q` limbs, this takes amortized O(log(`W` + 1) + `Q`)
    /// time, where `W` is the greatest working width reached, and returns
    /// O(`Q`) space, allocated only when `Q` exceeds two. If the normalized
    /// form outgrows the existing capacity, reallocation may temporarily
    /// retain both buffers.
    pub fn signed_magnitude(&self) -> (Ordering, Limbs) {
        self.read(Value::signed_magnitude)
    }

    /// Return the comparison with zero, the magnitude's [`Limbs`], and a scale
    /// in bits.
    ///
    /// The exact value is `±limbs · 2^shift`. As in
    /// [`signed_magnitude`](Self::signed_magnitude), the limbs are minimal and
    /// empty exactly for zero, and the read keeps the normalized form it
    /// computes. The method may factor out a low span known to be zero from
    /// write history, avoiding a scan of it. The factor need not be maximal,
    /// since cancellation can leave zero bits at the bottom of `limbs`, and
    /// its granularity is unspecified.
    ///
    /// ```
    /// use num_bigint::BigUint;
    /// use suanpan::Accumulator;
    ///
    /// let mut total = Accumulator::new();
    /// total.add_shifted_limbs(100_000, [3]);
    /// let (_, limbs, shift) = total.scaled_signed_magnitude();
    /// let magnitude = BigUint::new(limbs.iter().flat_map(|&l| [l as u32, (l >> 32) as u32]).collect());
    /// assert_eq!(magnitude << shift, BigUint::from(3u8) << 100_000u32);
    /// ```
    ///
    /// # Complexity
    ///
    /// If the result has `Q` limbs, this takes amortized O(log(`W` + 1) + `Q`)
    /// time and returns O(`Q`) space, allocated only when `Q` exceeds two,
    /// where `W` is the greatest working width reached. Omitting the factored
    /// span keeps it out of `Q`. Reallocation may temporarily retain two
    /// buffers, as for [`signed_magnitude`](Self::signed_magnitude).
    pub fn scaled_signed_magnitude(&self) -> (Ordering, Limbs, u64) {
        self.read(Value::scaled_signed_magnitude)
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

#[cfg(test)]
mod tests;
