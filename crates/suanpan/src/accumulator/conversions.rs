//! Exact conversion at the fixed-width integer boundary.
//!
//! Conversion keeps sign and magnitude separate so that signed minima never
//! need to be negated in their own type. Construction retains the small
//! representation whenever it fits. Extraction reads that value directly or
//! temporarily normalizes stored digits; more than two 64-bit limbs cannot fit
//! any primitive destination and is rejected without truncation.
//!
//! `TryFrom` consumes its source, so a failed conversion returns the original
//! accumulator as the error. This preserves both its exact value and any
//! storage the caller may want to reuse.

use core::cmp::Ordering;

use super::operand::Update;
use super::small::SMALL_MAX;
use super::Accumulator;

/// Share the sign/magnitude boundary across primitive integer types.
impl Accumulator {
    /// Construct an exact primitive value without an overflowing signed cast.
    fn from_magnitude(magnitude: u128, update: Update) -> Self {
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
    /// Small-value reads take constant time without allocation. Other reads
    /// pay the ordinary normalized-output cost.
    fn primitive_parts(&self) -> Option<(Ordering, u128)> {
        if let Some(value) = self.small {
            return Some((value.cmp(&0), value.unsigned_abs()));
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
    fn primitive_signed(&self) -> Option<i128> {
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
    fn primitive_unsigned(&self) -> Option<u128> {
        let (sign, magnitude) = self.primitive_parts()?;
        (sign != Ordering::Less).then_some(magnitude)
    }
}

/// Implement construction and exact extraction for one signedness family.
macro_rules! conversions {
    ($parts:ident, $magnitude:expr, $update:expr; $($integer:ty),* $(,)?) => {$(
        /// Construct an exact integer in O(1) time and additional space.
        ///
        /// Values of magnitude at most 2^96 need no heap allocation.
        impl From<$integer> for Accumulator {
            /// Keep magnitude and sign separate throughout construction.
            fn from(value: $integer) -> Self {
                Self::from_magnitude($magnitude(value), $update(value))
            }
        }

        /// Extract a primitive integer if and only if its value fits.
        ///
        /// Takes O(`W`) time and O(`W`) temporary space for working width `W`.
        /// Failure returns the original accumulator unchanged.
        impl TryFrom<Accumulator> for $integer {
            /// The exact input, returned unchanged.
            type Error = Accumulator;

            /// Normalize only when direct extraction is unavailable.
            fn try_from(value: Accumulator) -> Result<Self, Self::Error> {
                value.$parts().and_then(|number| Self::try_from(number).ok()).ok_or(value)
            }
        }
    )*};
}

conversions!(primitive_signed, |value: i128| value.unsigned_abs(), |value: i128| if value.is_negative() { Update::Subtract } else { Update::Add }; i128);
conversions!(primitive_unsigned, |value: u128| value, |_| Update::Add; u128);

/// Widen the remaining primitives before sharing conversion machinery.
macro_rules! narrow_conversions {
    ($wide:ty, $parts:ident; $($integer:ty),* $(,)?) => {$(
        /// Construct a machine-sized integer without allocation.
        impl From<$integer> for Accumulator {
            /// The widening conversion preserves every input bit.
            fn from(value: $integer) -> Self {
                Self::from(value as $wide)
            }
        }

        /// Extract an exactly fitting integer, returning the input on failure.
        ///
        /// Takes O(`W`) time and O(`W`) temporary space for working width `W`.
        impl TryFrom<Accumulator> for $integer {
            /// The original accumulator, unchanged.
            type Error = Accumulator;

            /// Narrow only after an exact signed or unsigned 128-bit conversion.
            fn try_from(value: Accumulator) -> Result<Self, Self::Error> {
                let number = value.$parts().and_then(|number| Self::try_from(number).ok());
                number.ok_or(value)
            }
        }
    )*};
}

narrow_conversions!(i128, primitive_signed; i8, i16, i32, i64, isize);
narrow_conversions!(u128, primitive_unsigned; u8, u16, u32, u64, usize);
