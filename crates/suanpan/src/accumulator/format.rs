//! Decimal display and diagnostic formatting.

use core::cmp::Ordering;
use core::fmt::{self, Write as _};

use super::value::Value;
use super::Accumulator;

/// The largest power of ten that fits a `u64`, the chunk of decimal digits
/// each division by a word produces.
const DECIMAL_CHUNK: u64 = 10_000_000_000_000_000_000;

/// Decimal digits in one [`DECIMAL_CHUNK`].
const DECIMAL_CHUNK_DIGITS: usize = 19;

/// Format the value in decimal, as the primitive integers and `num-bigint` do.
///
/// The sign, `{:+}`, width, fill, alignment, and zero padding follow
/// [`Formatter::pad_integral`](fmt::Formatter::pad_integral).
///
/// ```
/// use suanpan::Accumulator;
///
/// let mut total = Accumulator::new();
/// total.add_shifted_limbs(64, [1]);
/// total -= 1;
/// assert_eq!(total.to_string(), "18446744073709551615");
/// assert_eq!(format!("{:+}", Accumulator::from(7)), "+7");
/// assert_eq!(format!("{:>5}", Accumulator::from(-7)), "   -7");
/// ```
///
/// # Complexity
///
/// The read takes the cost of
/// [`signed_magnitude`](Accumulator::signed_magnitude). Converting its `Q`
/// limbs to decimal divides them by a word once per 19 digits, O(`Q`²) time
/// and O(`Q`) temporary space.
impl fmt::Display for Accumulator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (sign, magnitude) = self.signed_magnitude();
        let digits = decimal(magnitude.as_ref());
        formatter.pad_integral(sign != Ordering::Less, "", &digits)
    }
}

/// Format the stored representation, including retained storage.
///
/// The output describes how the value is stored, not just the value, so it can
/// change when the value does not: reads compact the stored form. The
/// formatter's flags apply as they do to any derived `Debug`, so `{:#?}`
/// pretty-prints and `{:x?}` writes the stored numbers in hexadecimal.
///
/// # Complexity
///
/// Time and space linear in the storage width, which can far exceed the
/// value's width.
impl fmt::Debug for Accumulator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // We copy the stored form while the cell is borrowed and format the
        // copy after the borrow ends, so a sink that reads this accumulator
        // never meets a borrowed cell, and the caller's formatter applies its
        // own flags.
        let stored = self.peek(Value::clone);
        fmt::Debug::fmt(&StoredForm(&stored), formatter)
    }
}

/// A value's stored representation, for [`Debug`](fmt::Debug).
struct StoredForm<'a>(&'a Value);

impl fmt::Debug for StoredForm<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut fields = formatter.debug_struct("Accumulator");
        fields.field("small", &self.0.small);
        self.0.digits.debug_fields(&mut fields);
        fields.finish()
    }
}

/// Write little-endian 64-bit limbs of a magnitude in decimal.
fn decimal(limbs: &[u64]) -> String {
    // Dividing the whole magnitude by 10^19 leaves its lowest 19 decimal
    // digits as the remainder; repeating collects them from low to high.
    let mut quotient = limbs.to_vec();
    let mut chunks = Vec::new();
    while !quotient.is_empty() {
        let mut remainder = 0u128;
        for limb in quotient.iter_mut().rev() {
            let current = (remainder << 64) | u128::from(*limb);
            *limb = (current / u128::from(DECIMAL_CHUNK)) as u64;
            remainder = current % u128::from(DECIMAL_CHUNK);
        }
        chunks.push(remainder as u64);
        while quotient.last() == Some(&0) {
            quotient.pop();
        }
    }
    let Some((highest, lower)) = chunks.split_last() else {
        return "0".to_owned();
    };
    let mut digits = highest.to_string();
    for chunk in lower.iter().rev() {
        write!(digits, "{chunk:0width$}", width = DECIMAL_CHUNK_DIGITS)
            .expect("writing to a String cannot fail");
    }
    digits
}
