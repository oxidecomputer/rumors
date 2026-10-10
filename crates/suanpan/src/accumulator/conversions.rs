//! Exact conversion at the fixed-width integer boundary.
//!
//! Conversion keeps sign and magnitude separate so that signed minima never
//! need to be negated in their own type. Construction retains the small
//! representation whenever it fits. Extraction reads a small value directly.
//! For a wider one, it first asks whether every integer below `2^128` in
//! magnitude leaves the value's sign unchanged. If so, the magnitude is at
//! least `2^128`, fits no primitive destination, and is rejected without
//! reading further. If the query does not establish that, it has compacted the
//! value to at most six digits, which a normalizing read then extracts. Either
//! way, extraction compacts only what it reads.
//!
//! `TryFrom` consumes its source, so a failed conversion returns the same
//! value as the error. This preserves both its exact value and any storage the
//! caller may want to reuse.

use core::cmp::Ordering;

use super::operand::Update;
use super::small::SMALL_MAX;
use super::value::Value;
use super::Accumulator;

/// A primitive integer as a sign and a magnitude.
///
/// A `u128` magnitude holds every primitive's exactly, including that of
/// `i128::MIN`, so no signed value is negated in its own type.
pub(super) trait SignMagnitude: Copy {
    /// Return the sign against zero and the magnitude, widened losslessly.
    fn sign_magnitude(self) -> (Ordering, u128);
}

/// Signed primitives take their magnitude from `unsigned_abs`.
macro_rules! signed_sign_magnitude {
    ($($integer:ty),* $(,)?) => {$(
        impl SignMagnitude for $integer {
            fn sign_magnitude(self) -> (Ordering, u128) {
                (self.cmp(&0), self.unsigned_abs() as u128)
            }
        }
    )*};
}

/// Unsigned primitives are their own magnitude.
macro_rules! unsigned_sign_magnitude {
    ($($integer:ty),* $(,)?) => {$(
        impl SignMagnitude for $integer {
            fn sign_magnitude(self) -> (Ordering, u128) {
                (self.cmp(&0), self as u128)
            }
        }
    )*};
}

signed_sign_magnitude!(i8, i16, i32, i64, i128, isize);
unsigned_sign_magnitude!(u8, u16, u32, u64, u128, usize);

/// Share the sign/magnitude boundary across primitive integer types.
impl Value {
    /// Construct an exact primitive value without an overflowing signed cast.
    pub(crate) fn from_magnitude(magnitude: u128, update: Update) -> Self {
        let mut result = Self::new();
        if magnitude <= SMALL_MAX {
            let value = magnitude as i128;
            result.small = Some(update.apply(value));
        } else {
            result.start_digits();
            result.digits.deposit_magnitude(magnitude, update, 0);
        }
        result
    }

    /// Read a magnitude only when it fits the widest primitive integer.
    ///
    /// Small-value reads take constant time without allocation. A wider value
    /// is read only when its magnitude might fit 128 bits, and then spans at
    /// most six digits.
    pub(crate) fn primitive_parts(&mut self) -> Option<(Ordering, u128)> {
        if let Some(value) = self.small {
            return Some((value.cmp(&0), value.unsigned_abs()));
        }
        // `Some` means no integer below `2^128` in magnitude can change the
        // sign, so the magnitude is at least `2^128`. `None` leaves at most
        // two digits above the four that 128 bits occupy.
        if self.cmp_zero_stable_under(u128::BITS.into()).is_some() {
            return None;
        }
        let (sign, limbs) = self.signed_magnitude();
        match limbs.as_ref() {
            [] => Some((sign, 0)),
            [low] => Some((sign, u128::from(*low))),
            [low, high] => Some((sign, u128::from(*low) | (u128::from(*high) << 64))),
            _ => None,
        }
    }

    /// Interpret a full primitive magnitude in the signed range, including MIN.
    fn primitive_signed(&mut self) -> Option<i128> {
        let (sign, magnitude) = self.primitive_parts()?;
        if sign == Ordering::Less {
            if magnitude == 1u128 << 127 {
                Some(i128::MIN)
            } else {
                i128::try_from(magnitude).ok().map(|value| -value)
            }
        } else {
            i128::try_from(magnitude).ok()
        }
    }

    /// Reject negative values before narrowing an unsigned magnitude.
    fn primitive_unsigned(&mut self) -> Option<u128> {
        let (sign, magnitude) = self.primitive_parts()?;
        (sign != Ordering::Less).then_some(magnitude)
    }
}

/// Implement construction and exact extraction for one signedness family.
macro_rules! conversions {
    ($parts:ident; $($integer:ty),* $(,)?) => {$(
        /// Construct an exact integer in O(1) time and additional space.
        ///
        /// Values of magnitude at most 2^96 need no heap allocation.
        impl From<$integer> for Accumulator {
            /// Keep magnitude and sign separate throughout construction.
            fn from(value: $integer) -> Self {
                let (sign, magnitude) = value.sign_magnitude();
                Accumulator::from_value(Value::from_magnitude(magnitude, Update::from_sign(sign)))
            }
        }

        /// Extract a primitive integer if and only if its value fits.
        ///
        /// Takes amortized O(log(`W` + 1)) time and O(1) temporary space,
        /// where `W` is the greatest working width reached. Failure returns
        /// the same value, though the read may have changed its working width.
        impl TryFrom<Accumulator> for $integer {
            /// The input's exact value.
            type Error = Accumulator;

            /// Read the value only as far as deciding whether it fits.
            fn try_from(mut value: Accumulator) -> Result<Self, Self::Error> {
                let number = value
                    .value
                    .get_mut()
                    .$parts()
                    .and_then(|number| Self::try_from(number).ok());
                number.ok_or(value)
            }
        }
    )*};
}

conversions!(primitive_signed; i8, i16, i32, i64, i128, isize);
conversions!(primitive_unsigned; u8, u16, u32, u64, u128, usize);
