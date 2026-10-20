//! Standard arithmetic traits over owned, borrowed, and primitive operands.
//!
//! Assignment is the primitive operation: an accumulator updates one running
//! value. Consuming operators reuse their left operand's storage, and owned
//! addition reuses whichever input is wider. An operator on a borrowed left
//! operand builds a new accumulator by adding that operand to zero.
//!
//! An accumulator operand is read, never copied, and the read compacts any
//! cancellation at its top. An assignment borrows its receiver mutably, so its
//! operand is always a different accumulator. Two borrowed operands may be the
//! same accumulator; their reads happen one after the other, so `&a + &a`
//! needs no special case.
//!
//! Primitive integers keep sign and magnitude separate, so signed minima never
//! pass through an overflowing cast. Shift counts are checked, never wrapped.
//!
//! Costs use the quantities of the [crate-level cost
//! model](crate#costs-and-storage).

use core::iter::Sum;
use core::ops::{Add, AddAssign, Neg, Shl, ShlAssign, Sub, SubAssign};

use super::conversions::SignMagnitude;
use super::operand::Update;
use super::Accumulator;

/// Primitives through 64 bits share the bounded small-value path.
macro_rules! word_assignments {
    ($($integer:ty),* $(,)?) => {$(
        /// Add in amortized O(log(`W` + 1)) time and O(1) retained space.
        impl AddAssign<$integer> for Accumulator {
            #[inline]
            fn add_assign(&mut self, rhs: $integer) {
                self.value.get_mut().add_word(rhs as i128);
            }
        }

        /// Subtract in amortized O(log(`W` + 1)) time and O(1) retained space.
        impl SubAssign<$integer> for Accumulator {
            /// Widen before negating, so signed minima negate exactly.
            #[inline]
            fn sub_assign(&mut self, rhs: $integer) {
                self.value.get_mut().add_word(-(rhs as i128));
            }
        }
    )*};
}

word_assignments!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

/// Full-width primitives carry their sign beside a `u128` magnitude.
macro_rules! wide_assignments {
    ($($integer:ty),* $(,)?) => {$(
        /// Add in amortized O(log(`W` + 1)) time and O(1) retained space.
        impl AddAssign<$integer> for Accumulator {
            fn add_assign(&mut self, rhs: $integer) {
                let (sign, magnitude) = rhs.sign_magnitude();
                self.value
                    .get_mut()
                    .apply_magnitude(magnitude, Update::from_sign(sign));
            }
        }

        /// Subtract in amortized O(log(`W` + 1)) time and O(1) retained space.
        impl SubAssign<$integer> for Accumulator {
            fn sub_assign(&mut self, rhs: $integer) {
                let (sign, magnitude) = rhs.sign_magnitude();
                self.value
                    .get_mut()
                    .apply_magnitude(magnitude, Update::from_sign(sign.reverse()));
            }
        }
    )*};
}

wide_assignments!(i128, u128);

/// Consuming operators, borrowed-receiver operators, and sums of primitives.
macro_rules! primitive_operators {
    ($($operand:ty),* $(,)?) => {$(
        /// Add into the owned receiver, at the cost of `+=`.
        impl Add<$operand> for Accumulator {
            type Output = Accumulator;

            fn add(mut self, rhs: $operand) -> Accumulator {
                self += rhs;
                self
            }
        }

        /// Subtract from the owned receiver, at the cost of `-=`.
        impl Sub<$operand> for Accumulator {
            type Output = Accumulator;

            fn sub(mut self, rhs: $operand) -> Accumulator {
                self -= rhs;
                self
            }
        }

        /// Add to a new accumulator holding the borrowed value, in amortized
        /// O(`A` log(`A` + 1)) time and O(`A`) space.
        impl Add<$operand> for &Accumulator {
            type Output = Accumulator;

            fn add(self, rhs: $operand) -> Accumulator {
                Accumulator::from(self) + rhs
            }
        }

        /// Subtract from a new accumulator holding the borrowed value, in
        /// amortized O(`A` log(`A` + 1)) time and O(`A`) space.
        impl Sub<$operand> for &Accumulator {
            type Output = Accumulator;

            fn sub(self, rhs: $operand) -> Accumulator {
                Accumulator::from(self) - rhs
            }
        }

        /// Sum `K` primitives in amortized O(`K` log(`W` + 1) + `G`) time.
        impl Sum<$operand> for Accumulator {
            fn sum<I: Iterator<Item = $operand>>(iter: I) -> Accumulator {
                iter.fold(Accumulator::new(), Add::add)
            }
        }
    )*};
}

primitive_operators!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

/// The remaining primitive forms, each forwarding to a form above.
///
/// A borrowed primitive is copied. A primitive on the left moves to the
/// right: addition commutes, and `value - accumulator` is
/// `-accumulator + value`.
macro_rules! forwarded_primitive_operators {
    ($($operand:ty),* $(,)?) => {$(
        /// Add a borrowed primitive, at the cost of `+=`.
        impl AddAssign<&$operand> for Accumulator {
            #[inline]
            fn add_assign(&mut self, rhs: &$operand) {
                *self += *rhs;
            }
        }

        /// Subtract a borrowed primitive, at the cost of `-=`.
        impl SubAssign<&$operand> for Accumulator {
            #[inline]
            fn sub_assign(&mut self, rhs: &$operand) {
                *self -= *rhs;
            }
        }

        /// Add a borrowed primitive into the owned receiver, at the cost of
        /// `+=`.
        impl Add<&$operand> for Accumulator {
            type Output = Accumulator;

            fn add(self, rhs: &$operand) -> Accumulator {
                self + *rhs
            }
        }

        /// Subtract a borrowed primitive from the owned receiver, at the cost
        /// of `-=`.
        impl Sub<&$operand> for Accumulator {
            type Output = Accumulator;

            fn sub(self, rhs: &$operand) -> Accumulator {
                self - *rhs
            }
        }

        /// Add a borrowed primitive to a new accumulator holding the borrowed
        /// value, as with the primitive by value.
        impl Add<&$operand> for &Accumulator {
            type Output = Accumulator;

            fn add(self, rhs: &$operand) -> Accumulator {
                self + *rhs
            }
        }

        /// Subtract a borrowed primitive from a new accumulator holding the
        /// borrowed value, as with the primitive by value.
        impl Sub<&$operand> for &Accumulator {
            type Output = Accumulator;

            fn sub(self, rhs: &$operand) -> Accumulator {
                self - *rhs
            }
        }

        /// Add into the owned right operand's storage, at the cost of `+=`.
        impl Add<Accumulator> for $operand {
            type Output = Accumulator;

            fn add(self, rhs: Accumulator) -> Accumulator {
                rhs + self
            }
        }

        /// Add to a new accumulator holding the borrowed value, as
        /// `&accumulator + value`.
        impl Add<&Accumulator> for $operand {
            type Output = Accumulator;

            fn add(self, rhs: &Accumulator) -> Accumulator {
                rhs + self
            }
        }

        /// Add into the owned right operand's storage, at the cost of `+=`.
        impl Add<Accumulator> for &$operand {
            type Output = Accumulator;

            fn add(self, rhs: Accumulator) -> Accumulator {
                rhs + *self
            }
        }

        /// Add to a new accumulator holding the borrowed value, as
        /// `&accumulator + value`.
        impl Add<&Accumulator> for &$operand {
            type Output = Accumulator;

            fn add(self, rhs: &Accumulator) -> Accumulator {
                rhs + *self
            }
        }

        /// Subtract the owned right operand, as `-rhs + self`, reusing its
        /// storage: the cost of `+=` plus O(`W`) for the negation.
        impl Sub<Accumulator> for $operand {
            type Output = Accumulator;

            fn sub(self, rhs: Accumulator) -> Accumulator {
                -rhs + self
            }
        }

        /// Subtract a borrowed accumulator into a new one, as `-rhs + self`:
        /// the cost of `-` on a borrow plus `+=`.
        impl Sub<&Accumulator> for $operand {
            type Output = Accumulator;

            fn sub(self, rhs: &Accumulator) -> Accumulator {
                -rhs + self
            }
        }

        /// Subtract the owned right operand, as with the primitive by value.
        impl Sub<Accumulator> for &$operand {
            type Output = Accumulator;

            fn sub(self, rhs: Accumulator) -> Accumulator {
                *self - rhs
            }
        }

        /// Subtract a borrowed accumulator into a new one, as with the
        /// primitive by value.
        impl Sub<&Accumulator> for &$operand {
            type Output = Accumulator;

            fn sub(self, rhs: &Accumulator) -> Accumulator {
                *self - rhs
            }
        }

        /// Sum borrowed primitives, as summing their values does.
        impl<'a> Sum<&'a $operand> for Accumulator {
            fn sum<I: Iterator<Item = &'a $operand>>(iter: I) -> Accumulator {
                iter.copied().sum()
            }
        }
    )*};
}

forwarded_primitive_operators!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

/// Copy a borrowed accumulator's value into a new one by adding it to zero.
///
/// The read compacts the source's cancelled top, so the copy carries none of
/// it. Takes amortized O(`A` log(`A` + 1)) time and O(`A`) space.
impl From<&Accumulator> for Accumulator {
    fn from(value: &Accumulator) -> Accumulator {
        let mut copy = Accumulator::new();
        copy += value;
        copy
    }
}

/// Add a borrowed accumulator in amortized O(`A` log(`W` + 1) + `G`) time,
/// adding O(`A` + `G`) retained space.
///
/// The receiver is borrowed mutably, so the operand is a different accumulator:
///
/// ```compile_fail
/// use suanpan::Accumulator;
///
/// let mut total = Accumulator::from(3);
/// total += &total;
/// ```
impl AddAssign<&Accumulator> for Accumulator {
    fn add_assign(&mut self, rhs: &Accumulator) {
        let value = self.value.get_mut();
        rhs.read(|operand| value.apply_accumulator(operand, 0, Update::Add));
    }
}

/// Subtract a borrowed accumulator in amortized O(`A` log(`W` + 1) + `G`)
/// time, adding O(`A` + `G`) retained space.
///
/// As for `+=`, the operand is a different accumulator:
///
/// ```compile_fail
/// use suanpan::Accumulator;
///
/// let mut total = Accumulator::from(3);
/// total -= &total;
/// ```
impl SubAssign<&Accumulator> for Accumulator {
    fn sub_assign(&mut self, rhs: &Accumulator) {
        let value = self.value.get_mut();
        rhs.read(|operand| value.apply_accumulator(operand, 0, Update::Sub));
    }
}

/// Add an owned accumulator, reusing the wider input's storage.
///
/// Takes amortized O(`A` log(`W` + 1) + `G`) time, where `A` is the narrower
/// input's working width.
impl AddAssign for Accumulator {
    fn add_assign(&mut self, rhs: Accumulator) {
        *self = core::mem::take(self) + rhs;
    }
}

/// Subtract an owned accumulator, at the cost of `-=` with a borrowed one.
impl SubAssign for Accumulator {
    fn sub_assign(&mut self, rhs: Accumulator) {
        *self -= &rhs;
    }
}

/// Add owned accumulators, reading the narrower into the wider's storage.
///
/// Takes amortized O(`A` log(`W` + 1) + `G`) time, where `A` is the narrower
/// input's working width. On equal widths the left operand's storage is kept.
impl Add for Accumulator {
    type Output = Accumulator;

    fn add(mut self, mut rhs: Accumulator) -> Accumulator {
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

/// Subtract owned accumulators, at the cost of `-=` with a borrowed one.
impl Sub for Accumulator {
    type Output = Accumulator;

    fn sub(mut self, rhs: Accumulator) -> Accumulator {
        self -= &rhs;
        self
    }
}

/// Add a borrowed accumulator into the owned receiver, at the cost of `+=`.
///
/// The receiver is moved in, so the operand is a different accumulator:
///
/// ```compile_fail
/// let total = suanpan::Accumulator::from(3);
/// let doubled = total + &total;
/// ```
impl Add<&Accumulator> for Accumulator {
    type Output = Accumulator;

    fn add(mut self, rhs: &Accumulator) -> Accumulator {
        self += rhs;
        self
    }
}

/// Subtract a borrowed accumulator from the owned receiver, at the cost of
/// `-=`.
impl Sub<&Accumulator> for Accumulator {
    type Output = Accumulator;

    fn sub(mut self, rhs: &Accumulator) -> Accumulator {
        self -= rhs;
        self
    }
}

/// Add into the owned right operand's storage, at the cost of `+=`.
impl Add<Accumulator> for &Accumulator {
    type Output = Accumulator;

    fn add(self, rhs: Accumulator) -> Accumulator {
        rhs + self
    }
}

/// Subtract the owned right operand, as `-(rhs - self)`, reusing its storage.
///
/// Takes the cost of `-=` with a borrowed operand plus O(`W`) for the negation.
impl Sub<Accumulator> for &Accumulator {
    type Output = Accumulator;

    fn sub(self, rhs: Accumulator) -> Accumulator {
        -(rhs - self)
    }
}

/// Add two borrowed accumulators into a new one.
///
/// Takes the cost of copying the left operand plus `+=` with the right. The
/// two may be the same accumulator: `&a + &a` is twice `a`.
impl Add<&Accumulator> for &Accumulator {
    type Output = Accumulator;

    fn add(self, rhs: &Accumulator) -> Accumulator {
        Accumulator::from(self) + rhs
    }
}

/// Subtract two borrowed accumulators into a new one.
///
/// Takes the cost of copying the left operand plus `-=` with the right. The
/// two may be the same accumulator: `&a - &a` is zero.
impl Sub<&Accumulator> for &Accumulator {
    type Output = Accumulator;

    fn sub(self, rhs: &Accumulator) -> Accumulator {
        Accumulator::from(self) - rhs
    }
}

/// Sum owned accumulators, reusing the wider input at each addition.
///
/// Takes amortized O(`T` log(`W` + 1) + `G`) time, where `T` is the sum of the
/// input working widths.
impl Sum for Accumulator {
    fn sum<I: Iterator<Item = Accumulator>>(iter: I) -> Accumulator {
        iter.reduce(Add::add).unwrap_or_default()
    }
}

/// Sum borrowed accumulators, reading each once.
///
/// Takes amortized O(`T` log(`W` + 1) + `G`) time, where `T` is the sum of the
/// input working widths. An accumulator may appear more than once.
impl<'a> Sum<&'a Accumulator> for Accumulator {
    fn sum<I: Iterator<Item = &'a Accumulator>>(iter: I) -> Accumulator {
        iter.fold(Accumulator::new(), Add::add)
    }
}

/// Negate in place in O(`W`) time and O(1) additional space.
impl Neg for Accumulator {
    type Output = Accumulator;

    fn neg(mut self) -> Accumulator {
        self.value.get_mut().negate();
        self
    }
}

/// Negate into a new accumulator, at the cost of `-=` into zero.
impl Neg for &Accumulator {
    type Output = Accumulator;

    fn neg(self) -> Accumulator {
        Accumulator::new() - self
    }
}

/// Shifts by every primitive count type.
macro_rules! shifts {
    ($($integer:ty),* $(,)?) => {$(
        /// Multiply by `2^shift`, reusing retained storage when possible.
        ///
        /// # Complexity
        ///
        /// Let `A` be the old working width and `S` the result's. A nonzero
        /// value shifts in amortized O(`A` log(`S` + 1) + `S`) time and O(`S`)
        /// temporary space, because the old and new allocations may coexist.
        /// A zero shifts in O(1) space and in the amortized time of
        /// [`sign`](Accumulator::sign).
        ///
        /// # Panics
        ///
        /// Panics if the count is negative. Panics if the value is nonzero and
        /// the count exceeds `u64::MAX`, or the result needs more storage than a
        /// buffer can address. Zero shifts by any nonnegative count.
        impl ShlAssign<$integer> for Accumulator {
            fn shl_assign(&mut self, shift: $integer) {
                self.value.get_mut().shift_left(shift_count(shift));
            }
        }

        /// Multiply an owned accumulator by `2^shift`, as `<<=` does.
        ///
        /// # Panics
        ///
        /// As for `<<=`.
        impl Shl<$integer> for Accumulator {
            type Output = Accumulator;

            fn shl(mut self, shift: $integer) -> Accumulator {
                self <<= shift;
                self
            }
        }

        /// Multiply a borrowed accumulator by `2^shift` into a new one.
        ///
        /// Takes the cost of [`add_shifted`](Accumulator::add_shifted) into
        /// zero.
        ///
        /// # Panics
        ///
        /// As for `<<=`.
        impl Shl<$integer> for &Accumulator {
            type Output = Accumulator;

            fn shl(self, shift: $integer) -> Accumulator {
                let mut shifted = Accumulator::new();
                if let Some(shift) = self.read(|value| value.addressable_shift(shift_count(shift))) {
                    shifted.add_shifted(shift, self);
                }
                shifted
            }
        }

        /// Multiply by `2^shift` for a borrowed count, as `<<=` does.
        ///
        /// # Panics
        ///
        /// As for `<<=`.
        impl ShlAssign<&$integer> for Accumulator {
            fn shl_assign(&mut self, shift: &$integer) {
                *self <<= *shift;
            }
        }

        /// Multiply an owned accumulator by `2^shift` for a borrowed count, as
        /// `<<=` does.
        ///
        /// # Panics
        ///
        /// As for `<<=`.
        impl Shl<&$integer> for Accumulator {
            type Output = Accumulator;

            fn shl(self, shift: &$integer) -> Accumulator {
                self << *shift
            }
        }

        /// Multiply a borrowed accumulator by `2^shift` for a borrowed count
        /// into a new one, as with the count by value.
        ///
        /// # Panics
        ///
        /// As for `<<=`.
        impl Shl<&$integer> for &Accumulator {
            type Output = Accumulator;

            fn shl(self, shift: &$integer) -> Accumulator {
                self << *shift
            }
        }
    )*};
}

shifts!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

/// Widen a shift count to `u128`.
///
/// # Panics
///
/// Panics if the count is negative.
fn shift_count<T: TryInto<u128>>(shift: T) -> u128 {
    shift
        .try_into()
        .unwrap_or_else(|_| panic!("a shift count must be nonnegative"))
}
