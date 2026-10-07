//! Express ordinary arithmetic with standard traits while preserving storage.
//!
//! Assignment is the primitive operation because the accumulator is designed
//! to update one running value. Consuming operators build on it so they can
//! reuse an owned operand rather than allocate a fresh destination. Addition
//! may choose the wider owned buffer and read the narrower value; subtraction
//! must retain operand order and therefore writes into the left operand.
//!
//! Primitive inputs through 64 bits share the bounded small-value path.
//! Full-width integers keep sign separate from magnitude so signed minima and
//! large unsigned values never pass through an overflowing cast. Borrowed
//! accumulators are read as stored, without normalization or cloning.
//!
//! Shift counts are checked, never wrapped. All primitive count types are
//! accepted so literals and machine-sized indices work without casts.

use core::iter::Sum;
use core::ops::{Add, AddAssign, Neg, Shl, ShlAssign, Sub, SubAssign};

use super::operand::Update;
use super::small::{SMALL_MAX, SMALL_SHIFT_MAX};
use super::{touch, Accumulator};

/// Give every at-most-64-bit integer the same bounded update path.
macro_rules! word_assignments {
    ($($integer:ty),* $(,)?) => {$(
        /// Add in amortized O(log(`W` + 1)) time and O(1) retained space,
        /// where `W` is the greatest working width reached by the receiver.
        impl AddAssign<$integer> for Accumulator {
            /// Widen the operand before depositing it.
            #[inline]
            fn add_assign(&mut self, rhs: $integer) {
                self.add_word(rhs as i128);
            }
        }

        /// Subtract in amortized O(log(`W` + 1)) time and O(1) retained space,
        /// where `W` is the greatest working width reached by the receiver.
        impl SubAssign<$integer> for Accumulator {
            /// Widen before negating, including signed minima.
            #[inline]
            fn sub_assign(&mut self, rhs: $integer) {
                self.add_word(-(rhs as i128));
            }
        }
    )*};
}

word_assignments!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

/// Implement full-width inputs without an overflowing signed intermediate.
macro_rules! wide_assignments {
    ($integer:ty, $magnitude:expr, $negative:expr) => {
        /// Add in amortized O(log(`W` + 1)) time and O(1) retained space,
        /// where `W` is the greatest working width reached by the receiver.
        impl AddAssign<$integer> for Accumulator {
            /// Carry the input's sign separately from its magnitude.
            fn add_assign(&mut self, rhs: $integer) {
                let update = if $negative(rhs) {
                    Update::Subtract
                } else {
                    Update::Add
                };
                self.apply_magnitude($magnitude(rhs), update);
            }
        }

        /// Subtract in amortized O(log(`W` + 1)) time and O(1) retained space,
        /// where `W` is the greatest working width reached by the receiver.
        impl SubAssign<$integer> for Accumulator {
            /// Reverse the input sign without negating its signed value.
            fn sub_assign(&mut self, rhs: $integer) {
                let update = if $negative(rhs) {
                    Update::Add
                } else {
                    Update::Subtract
                };
                self.apply_magnitude($magnitude(rhs), update);
            }
        }
    };
}

wide_assignments!(i128, i128::unsigned_abs, i128::is_negative);
wide_assignments!(u128, |value| value, |_| false);

/// Add a borrowed accumulator in time nearly linear in its working width.
///
/// The operand is read directly without normalization or an intermediate
/// allocation. Let `A` be its working width, `W` the greatest receiver working
/// width, and `G` the receiver's growth. The amortized time is
/// O(`A` log(`W` + 1) + `G`), and the operation adds O(`A` + `G`) retained
/// space in the worst case. Growing the receiver may temporarily retain both
/// its old and replacement allocations.
/// Repeatedly adding a growing operand re-reads its width.
impl AddAssign<&Accumulator> for Accumulator {
    /// Apply the operand directly without normalizing or copying it.
    fn add_assign(&mut self, rhs: &Accumulator) {
        self.apply_accumulator(rhs, 0, Update::Add);
    }
}

/// Subtract a borrowed accumulator in amortized
/// O(`A` log(`W` + 1) + `G`) time.
///
/// Let `A` be the operand's working width, `W` the greatest receiver working
/// width, and `G` the receiver's growth. This adds O(`A` + `G`) retained space
/// in the worst case. Growing the receiver may temporarily retain both its old
/// and replacement allocations.
impl SubAssign<&Accumulator> for Accumulator {
    /// Deposit each operand contribution with its sign reversed.
    fn sub_assign(&mut self, rhs: &Accumulator) {
        self.apply_accumulator(rhs, 0, Update::Subtract);
    }
}

/// Add an owned value while reusing an input allocation.
///
/// Let `A` be the lesser input working width, `W` the greatest result working
/// width, and `G` its growth. This takes amortized
/// O(`A` log(`W` + 1) + `G`) time and adds O(`A` + `G`) retained space in the
/// worst case. Growing the selected receiver may temporarily retain both its
/// old and replacement allocations.
impl AddAssign for Accumulator {
    /// Transfer the destination into owned addition without allocating.
    fn add_assign(&mut self, rhs: Self) {
        *self = core::mem::take(self) + rhs;
    }
}

/// Subtract an owned value, retaining the receiver's storage.
///
/// Let `A` be the right operand's working width, `W` the greatest receiver
/// working width, and `G` its growth. This takes amortized
/// O(`A` log(`W` + 1) + `G`) time and adds O(`A` + `G`) retained space in the
/// worst case. Growing the receiver may temporarily retain both its old and
/// replacement allocations.
impl SubAssign for Accumulator {
    /// Read the owned operand directly, then release its storage.
    fn sub_assign(&mut self, rhs: Self) {
        *self -= &rhs;
    }
}

/// Derive consuming arithmetic and sums from each primitive assignment.
macro_rules! consuming_arithmetic {
    ($($operand:ty),* $(,)?) => {$(
        /// Add into the owned receiver in amortized O(log(`W` + 1)) time,
        /// where `W` is its greatest working width.
        ///
        /// The operation adds O(1) retained space. Growing the receiver may
        /// temporarily retain O(`W`) space while replacing its allocation.
        impl Add<$operand> for Accumulator {
            /// The exact running sum.
            type Output = Self;

            /// Apply the corresponding assignment without cloning.
            fn add(mut self, rhs: $operand) -> Self {
                self += rhs;
                self
            }
        }

        /// Subtract from the owned receiver in amortized O(log(`W` + 1)) time,
        /// where `W` is its greatest working width.
        ///
        /// The operation adds O(1) retained space. Growing the receiver may
        /// temporarily retain O(`W`) space while replacing its allocation.
        impl Sub<$operand> for Accumulator {
            /// The exact running difference.
            type Output = Self;

            /// Apply the corresponding assignment without cloning.
            fn sub(mut self, rhs: $operand) -> Self {
                self -= rhs;
                self
            }
        }

        /// Sum `K` inputs in amortized O(`K` log(`W` + 1) + `G`) time, where
        /// `W` is the greatest result working width and `G` its growth.
        ///
        /// The result adds O(`K` + `G`) retained space in the worst case.
        /// Allocation growth may temporarily retain O(`W` + `G`) space.
        impl Sum<$operand> for Accumulator {
            /// Fold directly into a single accumulator, starting at zero.
            fn sum<I: Iterator<Item = $operand>>(iter: I) -> Self {
                iter.fold(Self::new(), |mut total, operand| {
                    total += operand;
                    total
                })
            }
        }
    )*};
}

consuming_arithmetic!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

/// Add a borrowed value into the owned receiver without cloning either value.
///
/// Let `A` be the operand's working width, `W` the greatest receiver working
/// width, and `G` its growth. This takes amortized
/// O(`A` log(`W` + 1) + `G`) time and adds
/// O(`A` + `G`) retained space in the worst case. Growing the receiver may
/// temporarily retain both its old and replacement allocations.
impl Add<&Accumulator> for Accumulator {
    /// The exact sum in the receiver's storage.
    type Output = Self;

    /// Read the borrowed operand directly into the owned receiver.
    fn add(mut self, rhs: &Self) -> Self {
        self += rhs;
        self
    }
}

/// Subtract a borrowed value from the owned receiver without cloning.
///
/// Let `A` be the operand's working width, `W` the greatest receiver working
/// width, and `G` its growth. This takes amortized
/// O(`A` log(`W` + 1) + `G`) time and adds
/// O(`A` + `G`) retained space in the worst case. Growing the receiver may
/// temporarily retain both its old and replacement allocations.
impl Sub<&Accumulator> for Accumulator {
    /// The exact difference in the receiver's storage.
    type Output = Self;

    /// Read the borrowed operand directly into the owned receiver.
    fn sub(mut self, rhs: &Self) -> Self {
        self -= rhs;
        self
    }
}

/// Sum borrowed values by reading each operand's working width once.
///
/// If `T` is the sum of the input working widths, `W` the greatest result
/// working width, and `G` its growth, this takes amortized
/// O(`T` log(`W` + 1) + `G`) time and adds O(`T` + `G`) retained space in the
/// worst case. Allocation growth may temporarily retain O(`W` + `G`) space.
impl<'a> Sum<&'a Accumulator> for Accumulator {
    /// Accumulate directly without cloning or normalizing the inputs.
    fn sum<I: Iterator<Item = &'a Accumulator>>(iter: I) -> Self {
        iter.fold(Self::new(), |mut total, operand| {
            total += operand;
            total
        })
    }
}

/// Add owned accumulators while reusing an input allocation.
///
/// Here `A` is the lesser input working width in the [crate-level cost
/// model](crate#costs-and-storage). The amortized time is
/// O(`A` log(`W` + 1) + `G`). It adds
/// O(`A` + `G`) retained space in the worst case and O(1) temporary space. On
/// equal widths, the left operand keeps its allocation. The other allocation is
/// released; no normalized or shifted intermediate is allocated.
impl Add for Accumulator {
    /// The sum in the selected destination's storage.
    type Output = Self;

    /// Swap only when doing so avoids reading the operand with greater working
    /// width; addition is commutative, so this changes cost without changing
    /// meaning.
    fn add(mut self, mut rhs: Self) -> Self {
        if self.is_known_zero() {
            return rhs;
        }
        if rhs.is_known_zero() {
            return self;
        }
        if rhs.stored_digit_count() > self.stored_digit_count() {
            core::mem::swap(&mut self, &mut rhs);
        }
        self += &rhs;
        self
    }
}

/// Subtract owned accumulators, retaining the left operand's storage.
///
/// Let `A` be the right operand's working width, `W` the greatest receiver
/// working width, and `G` its growth. This takes amortized
/// O(`A` log(`W` + 1) + `G`) time and adds O(`A` + `G`) retained space in the
/// worst case. Growing the receiver may temporarily retain both its old and
/// replacement allocations.
impl Sub for Accumulator {
    /// The left value minus the right value.
    type Output = Self;

    /// Preserve operand order and read the right operand directly.
    fn sub(mut self, rhs: Self) -> Self {
        self -= &rhs;
        self
    }
}

/// Sum owned values, reusing the input that avoids more work at each addition.
///
/// If `T` is the sum of the input working widths, `W` the greatest result
/// working width, and `G` its growth, this takes amortized
/// O(`T` log(`W` + 1) + `G`) time and adds O(`T` + `G`)
/// retained space in the worst case. Allocation growth may temporarily retain
/// O(`W` + `G`) space.
impl Sum for Accumulator {
    /// Return zero for an empty iterator; otherwise reuse an owned allocation.
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(Add::add).unwrap_or_default()
    }
}

/// Negate in O(`W`) time and O(1) additional space.
impl Neg for Accumulator {
    /// The opposite integer in the same allocation.
    type Output = Self;

    /// Reverse every contribution without normalizing it.
    fn neg(mut self) -> Self {
        if let Some(value) = &mut self.small {
            touch(1);
            *value = -*value;
        } else {
            self.digits.negate();
        }
        self
    }
}

/// Route checked primitive shift counts through the same shift implementation.
macro_rules! shifts {
    ($($integer:ty),* $(,)?) => {$(
        /// Multiply by a power of two while reusing retained storage when possible.
        ///
        /// Let `A` be the old working width and `S` the shifted result's
        /// working width. A nonzero shift takes at most amortized
        /// O(`A` log(`S` + 1) + `S`) time and O(`S`) temporary space because
        /// the old and new allocations may coexist until the operation
        /// finishes. A zero count, or a value for which
        /// [`is_known_zero`](Accumulator::is_known_zero) returns `true`, takes
        /// O(1) time and space. Any other zero value shifts in O(1) space and
        /// O(`A`) time, a scan that is not amortized, whatever its working
        /// width.
        ///
        /// # Panics
        ///
        /// Panics if the count is negative or exceeds `u64::MAX`, or if the
        /// value is nonzero and its working width, shifted, would reach digit
        /// position `usize::MAX - 1`, where no allocation can hold the buffer
        /// the result needs. Growing the buffer is an ordinary vector
        /// allocation, which can also fail short of that position: it panics
        /// on capacity overflow, and aborts when the allocator cannot satisfy
        /// it. With a valid count, shifting zero never panics.
        impl ShlAssign<$integer> for Accumulator {
            /// Validate the count before changing the receiver.
            fn shl_assign(&mut self, shift: $integer) {
                self.shift_left(u64::try_from(shift).expect("shift count must fit u64"));
            }
        }

        /// Multiply an owned accumulator by a power of two.
        ///
        /// Let `A` be the old working width and `S` the result's working width.
        /// A nonzero shift takes at most amortized
        /// O(`A` log(`S` + 1) + `S`) time and O(`S`) temporary space. A zero
        /// count, or a value for which
        /// [`is_known_zero`](Accumulator::is_known_zero) returns `true`, takes
        /// O(1) time and space. Any other zero value shifts in O(1) space and
        /// O(`A`) time, a scan that is not amortized, whatever its working
        /// width.
        ///
        /// # Panics
        ///
        /// Panics if the count is negative or exceeds `u64::MAX`, or if the
        /// value is nonzero and its working width, shifted, would reach digit
        /// position `usize::MAX - 1`, where no allocation can hold the buffer
        /// the result needs. Growing the buffer is an ordinary vector
        /// allocation, which can also fail short of that position: it panics
        /// on capacity overflow, and aborts when the allocator cannot satisfy
        /// it. With a valid count, shifting zero never panics.
        impl Shl<$integer> for Accumulator {
            /// The exact value multiplied by the requested power of two.
            type Output = Self;

            /// Reuse the receiver through shift assignment.
            fn shl(mut self, shift: $integer) -> Self {
                self <<= shift;
                self
            }
        }
    )*};
}

shifts!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

/// Keep the shift kernel independent of the caller's primitive count type.
impl Accumulator {
    /// Shift exactly, retaining the small representation whenever it fits.
    fn shift_left(&mut self, shift: u64) {
        if shift == 0 || self.is_known_zero() {
            return;
        }
        if let Some(value) = self.small {
            touch(1);
            if shift <= SMALL_SHIFT_MAX {
                let shifted = value << shift;
                if shifted.unsigned_abs() <= SMALL_MAX {
                    self.small = Some(shifted);
                } else {
                    self.start_digits();
                    self.digits.deposit_value(shifted, 0);
                }
            } else {
                self.start_digits();
                self.digits.deposit_value(value, shift);
            }
            return;
        }
        // The fresh receiver holds no buffer, so every deposit would extend
        // it and `add_shifted` first scans the operand for zero. A zero whose
        // stored digits cancel therefore leaves a fresh zero rather than
        // landing those digits at the shifted position.
        let previous = core::mem::take(self);
        self.add_shifted(shift, &previous);
    }
}
