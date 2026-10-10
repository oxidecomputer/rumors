//! The accumulator's two representations and the boundary between them.
//!
//! Small values live directly in an `i128`. Once an operation needs more
//! range, the value moves into redundant signed digits. It stays there until
//! reset, even if cancellation later makes the value small again. This
//! one-way transition prevents an alternating workload from repeatedly paying
//! to convert the same value.
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
#[cfg(feature = "num-bigint")]
mod interop;
mod operand;
mod operators;
mod small;
mod value;

use core::cell::RefCell;
use core::cmp::Ordering;
use operand::Update;
use value::Value;

#[cfg(feature = "num-bigint")]
pub use interop::NegativeValueError;

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
/// An accumulator cannot be cloned. A copy would carry the same uncompacted
/// cancellation as its original, and both would pay to compact it, so the
/// amortized costs would no longer hold. To copy the value, add it to zero with
/// `Accumulator::from(&total)`, which compacts the source first.
///
/// ```compile_fail
/// let total = suanpan::Accumulator::from(3);
/// let copy = total.clone(); // no `Clone`
/// ```
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

/// Copy an accumulator for tests that compare a value before and after an
/// operation.
///
/// Tests only: a copy duplicates uncompacted cancellation, which the public
/// type forbids so that its amortized costs hold.
#[cfg(test)]
impl Clone for Accumulator {
    fn clone(&self) -> Self {
        Accumulator::from_value(self.peek(Value::clone))
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
        self.value
            .get_mut()
            .apply_limbs(limbs.into_iter(), shift, Update::Add);
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
        self.value
            .get_mut()
            .apply_limbs(limbs.into_iter(), shift, Update::Subtract);
    }

    /// Add another accumulator's value multiplied by `2^shift`.
    ///
    /// The operand is read directly; shifting it first would construct another
    /// accumulator merely to consume it here. The read may reduce `other`'s
    /// working width without changing its value. The receiver is borrowed
    /// mutably, so `other` is always a different accumulator.
    ///
    /// ```compile_fail
    /// use suanpan::Accumulator;
    ///
    /// let mut total = Accumulator::from(3);
    /// // The receiver cannot also be the operand.
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
    /// Panics if `other` is nonzero and the result needs more digit positions
    /// than a buffer can address. A zero operand never panics.
    pub fn add_shifted(&mut self, shift: u64, other: &Accumulator) {
        let value = self.value.get_mut();
        other.read(|operand| value.apply_accumulator(operand, shift, Update::Add));
    }

    /// Subtract another accumulator's value multiplied by `2^shift`.
    ///
    /// The operand is read directly, and the read may reduce its working width
    /// without changing its value. As with [`add_shifted`](Self::add_shifted),
    /// `other` is always a different accumulator:
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
    /// the worst case. Growing the receiver may temporarily retain
    /// both its old and replacement allocations. A zero operand adds O(1)
    /// retained space at any shift.
    ///
    /// # Panics
    ///
    /// Panics if `other` is nonzero and the result needs more digit positions
    /// than a buffer can address. A zero operand never panics.
    pub fn sub_shifted(&mut self, shift: u64, other: &Accumulator) {
        let value = self.value.get_mut();
        other.read(|operand| value.apply_accumulator(operand, shift, Update::Subtract));
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
    /// This allocation hint does not change the value. Storage comes in whole
    /// 32-bit positions, the unit [`stored_bits`](Self::stored_bits) reports,
    /// so the request rounds up to a multiple of 32. Reserving the expected
    /// width avoids repeated allocation growth and its transient memory use.
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
    /// assert_eq!(total.stored_bits(), 4_032);
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
    /// This is [`sign`](Self::sign)` == Ordering::Equal`, with its cost.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.sign() == Ordering::Equal
    }

    /// Return whether the value is greater than zero.
    ///
    /// This is [`sign`](Self::sign)` == Ordering::Greater`, with its cost.
    #[inline]
    pub fn is_positive(&self) -> bool {
        self.sign() == Ordering::Greater
    }

    /// Return whether the value is less than zero.
    ///
    /// This is [`sign`](Self::sign)` == Ordering::Less`, with its cost.
    #[inline]
    pub fn is_negative(&self) -> bool {
        self.sign() == Ordering::Less
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
    /// `None` result leaves this accumulator at most two base-2^32 digit
    /// positions (64 working bits) wider than that operand.
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
        self.peek(Value::stored_digit_count)
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
        self.peek(Value::stored_bits)
    }

    /// Return the comparison with zero and normalized magnitude limbs.
    ///
    /// The little-endian 64-bit limbs have no high zero word and are empty
    /// exactly when the comparison is `Equal`. The read keeps the normalized
    /// form it computes, which has the same value in a working width that can
    /// be far narrower and is at most 32 bits wider, so a repeated read costs
    /// only the result's width.
    ///
    /// # Complexity
    ///
    /// If the result has `Q` limbs, this takes amortized O(log(`W` + 1) + `Q`)
    /// time, O(`Q`) temporary space, and O(`Q`) returned space, where `W` is
    /// the greatest working width reached. If the result needs one digit
    /// position beyond the existing capacity, reallocation may temporarily
    /// retain both buffers.
    pub fn signed_magnitude(&self) -> (Ordering, impl AsRef<[u64]>) {
        self.read(Value::signed_magnitude)
    }

    /// Return the comparison with zero, magnitude limbs, and scale.
    ///
    /// The exact value is `±magnitude * 2^shift`. As in
    /// [`signed_magnitude`](Self::signed_magnitude), the magnitude is minimal
    /// and little-endian; zero has no limbs, and the read keeps the normalized
    /// form it computes. The method may factor out a low span known to be zero
    /// from write history, avoiding a scan of that span. The factor need not be
    /// maximal: cancellation can leave low zero bits in the magnitude.
    ///
    /// # Complexity
    ///
    /// If the result has `Q` limbs, this takes amortized O(log(`W` + 1) + `Q`)
    /// time, O(`Q`) temporary space, and O(`Q`) returned space, where `W` is
    /// the greatest working width reached. Omitting the factored span keeps
    /// it out of `Q`. Reallocation may temporarily retain two buffers, as for
    /// [`signed_magnitude`](Self::signed_magnitude).
    pub fn scaled_signed_magnitude(&self) -> (Ordering, impl AsRef<[u64]>, u64) {
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
