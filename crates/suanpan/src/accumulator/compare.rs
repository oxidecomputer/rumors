//! Equality, ordering, and hashing by value.
//!
//! Two accumulators compare by the sign of their difference. When one is far
//! wider, the stability query usually decides without reading the narrower
//! one; otherwise the comparison subtracts into a temporary. The same
//! accumulator on both sides compares equal before either cell is borrowed.
//!
//! An accumulator compares with a primitive integer, on its right, by sign and
//! magnitude. A value of magnitude at least `2^128` exceeds every primitive, so
//! its sign alone decides.
//!
//! Hashing reads the normalized sign and magnitude, so equal values hash
//! equally whatever their stored forms.

use core::cmp::Ordering;
use core::hash::{Hash, Hasher};

use super::conversions::SignMagnitude;
use super::operand::Update;
use super::value::Value;
use super::Accumulator;

/// The greatest working-width lead, in digits, at which a comparison
/// subtracts instead of asking the wider side's stability query.
///
/// A refused stability query leaves the side it asked at most this many digits
/// wider than the other. A side is asked only when it leads by more, so each
/// refusal strictly narrows it, and the loop in [`compare`] ends.
const CLOSE_LEAD: usize = 2;

/// Compare by value.
///
/// # Complexity
///
/// If one working width exceeds the other by more than two digits, the wider
/// side's stability query decides in amortized O(log(`W` + 1)) time unless
/// its own cancellation leaves it close to the other. Otherwise the comparison
/// subtracts in amortized O(`A` log(`A` + 1)) time and O(`A`) temporary space,
/// where `A` is the narrower working width.
impl Ord for Accumulator {
    fn cmp(&self, other: &Accumulator) -> Ordering {
        if core::ptr::eq(self, other) {
            return Ordering::Equal;
        }
        self.read(|a| other.read(|b| compare(a, b)))
    }
}

/// Compare by value, as [`Ord`] does.
impl PartialOrd for Accumulator {
    fn partial_cmp(&self, other: &Accumulator) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Compare by value, as [`Ord`] does.
impl PartialEq for Accumulator {
    fn eq(&self, other: &Accumulator) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Accumulator {}

/// Hash the normalized sign and magnitude, so equal values hash equally.
///
/// Takes the cost of [`signed_magnitude`](Accumulator::signed_magnitude).
impl Hash for Accumulator {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let (sign, magnitude) = self.signed_magnitude();
        sign.hash(state);
        magnitude.as_ref().hash(state);
    }
}

/// Return the order of `a` and `b`, two different values.
fn compare(a: &mut Value, b: &mut Value) -> Ordering {
    loop {
        let (a_digits, b_digits) = (a.stored_digit_count(), b.stored_digit_count());
        if a_digits > b_digits + CLOSE_LEAD {
            // `Some` means every value below `b`'s width leaves `a`'s sign, so
            // `a - b` has that sign too.
            if let Some(sign) = a.cmp_zero_stable_under(b.stored_bits()) {
                return sign;
            }
        } else if b_digits > a_digits + CLOSE_LEAD {
            if let Some(sign) = b.cmp_zero_stable_under(a.stored_bits()) {
                return sign.reverse();
            }
        } else {
            break;
        }
        // A refusal compacted the wider side, which may now be the narrower.
    }
    let mut difference = Value::new();
    difference.apply_accumulator(a, 0, Update::Add);
    difference.apply_accumulator(b, 0, Update::Subtract);
    difference.cmp_zero()
}

/// Compare a value with a primitive integer given by sign and magnitude.
fn compare_primitive(value: &mut Value, sign: Ordering, magnitude: u128) -> Ordering {
    let Some((value_sign, value_magnitude)) = value.primitive_parts() else {
        // The magnitude is at least 2^128, beyond every primitive.
        return value.cmp_zero();
    };
    match (value_sign, sign) {
        (Ordering::Greater, Ordering::Greater) => value_magnitude.cmp(&magnitude),
        (Ordering::Less, Ordering::Less) => magnitude.cmp(&value_magnitude),
        _ => value_sign.cmp(&sign),
    }
}

/// Comparisons against every primitive integer, with the accumulator on the
/// left.
///
/// The primitive cannot be the left operand: an impl of a standard comparison
/// trait for a primitive type would change type inference for that primitive's
/// comparisons in every crate linking this one.
macro_rules! primitive_comparisons {
    ($($integer:ty),* $(,)?) => {$(
        /// Compare by value in amortized O(log(`W` + 1)) time and O(1) space.
        impl PartialOrd<$integer> for Accumulator {
            fn partial_cmp(&self, other: &$integer) -> Option<Ordering> {
                let (sign, magnitude) = other.sign_magnitude();
                Some(self.read(|value| compare_primitive(value, sign, magnitude)))
            }
        }

        /// Compare by value, as `PartialOrd` does.
        impl PartialEq<$integer> for Accumulator {
            fn eq(&self, other: &$integer) -> bool {
                self.partial_cmp(other) == Some(Ordering::Equal)
            }
        }
    )*};
}

primitive_comparisons!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);
