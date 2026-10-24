//! Equality, ordering, and hashing by value.
//!
//! Two accumulators compare by the sign of their difference. When one is far
//! wider, the stability query usually decides without reading the narrower
//! one. Otherwise both are compared with zero, which compacts a cancelled top
//! so that their stored widths lie within two digits of their values' own,
//! and decides at once when the signs differ; if their widths still do not
//! decide, the comparison subtracts into a temporary. The same
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

/// The working-width lead, in digits, from which a comparison asks the wider
/// side's stability query rather than subtracting at once.
///
/// A refused stability query leaves the side it asked at most this many digits
/// wider than the other. Past this lead each refusal therefore strictly
/// narrows the side it asked, so [`compare_by_widths`]'s loop, which asks
/// repeatedly there, ends. At exactly this lead a refusal need not narrow it: compacting
/// a top digit can carry the same digit back. The query is asked there once,
/// and a refusal falls back to subtraction.
pub(super) const CLOSE_LEAD: usize = 2;

/// Compare by value.
///
/// # Complexity
///
/// If one working width far exceeds the other, the comparison usually decides
/// from the wider value's top in amortized O(1) time, without reading the
/// narrower one. Otherwise it subtracts in amortized O(`A`) time and O(`A`)
/// temporary space, where `A` is the narrower working width.
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
    if let Some(order) = compare_by_widths(a, b) {
        return order;
    }
    // Subtracting reads both stored widths, which a cancelled top can leave
    // far wider than the values. Compacting both first bounds the read by the
    // narrower value, and can open a lead that decides without it.
    let widths = (a.stored_digit_count(), b.stored_digit_count());
    let (a_sign, b_sign) = (a.compact(), b.compact());
    if a_sign != b_sign || a_sign == Ordering::Equal {
        // Values of different signs order as their signs do, and two zeros
        // are equal.
        return a_sign.cmp(&b_sign);
    }
    if (a.stored_digit_count(), b.stored_digit_count()) != widths {
        if let Some(order) = compare_by_widths(a, b) {
            return order;
        }
    }
    let mut difference = Value::new();
    difference.apply_accumulator(a, 0, Update::Add);
    difference.apply_accumulator(b, 0, Update::Sub);
    difference.cmp_zero()
}

/// Return the order of `a` and `b` if the wider one's stability query
/// decides it, asking that query while one leads by more than
/// [`CLOSE_LEAD`] digits and once at exactly that lead.
fn compare_by_widths(a: &mut Value, b: &mut Value) -> Option<Ordering> {
    loop {
        let (a_digits, b_digits) = (a.stored_digit_count(), b.stored_digit_count());
        if a_digits > b_digits + CLOSE_LEAD {
            // `Some` means every value below `b`'s width leaves `a`'s sign, so
            // `a - b` has that sign too.
            if let Some(sign) = a.cmp_zero_stable_under(b.stored_bits()) {
                return Some(sign);
            }
        } else if b_digits > a_digits + CLOSE_LEAD {
            if let Some(sign) = b.cmp_zero_stable_under(a.stored_bits()) {
                return Some(sign.reverse());
            }
        } else {
            break;
        }
        // A refusal compacted the wider side, which may now be the narrower.
    }
    // At exactly the close lead the query may still decide, and is asked once.
    let (a_digits, b_digits) = (a.stored_digit_count(), b.stored_digit_count());
    if a_digits == b_digits + CLOSE_LEAD {
        a.cmp_zero_stable_under(b.stored_bits())
    } else if b_digits == a_digits + CLOSE_LEAD {
        b.cmp_zero_stable_under(a.stored_bits())
            .map(Ordering::reverse)
    } else {
        None
    }
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
        /// Compare by value in amortized O(1) time and O(1) space.
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
