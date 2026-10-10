//! Decimal and diagnostic formatting, in several stored forms.
//!
//! Decimal conversion divides the magnitude into 19-digit chunks, so values
//! next to a power of ten exercise chunks of all zeros and nines; values next
//! to a power of two exercise carries across the stored digits the read
//! normalizes.

use num_bigint::{BigInt, BigUint};

use super::{assert_value, cancelled_top, Accumulator, TestBig as _};

/// The highest power of two tested, spanning 22 digits.
const MAX_POWER_OF_TWO: u32 = 700;

/// The highest power of ten tested, just below `2^700`.
const MAX_POWER_OF_TEN: u32 = 210;

/// The stored forms a tested value is written in.
#[derive(Clone, Copy, Debug)]
enum Form {
    /// The magnitude's limbs, added or subtracted once.
    Plain,
    /// The plain form under a top that cancels high above it.
    UnderCancelledTop,
    /// The power and the offset written separately, so their digits stay
    /// redundant.
    Split,
    /// The plain form, normalized first.
    Normalized,
}

/// Write `sign · (power + offset)` in the given form.
fn build(power: &BigUint, offset: i64, negative: bool, form: Form) -> (Accumulator, BigInt) {
    let magnitude = BigInt::from(power.clone()) + offset;
    let oracle = if negative {
        -&magnitude
    } else {
        magnitude.clone()
    };
    let magnitude = BigUint::try_from(magnitude).expect("tested magnitudes are positive");
    let mut acc = match form {
        Form::UnderCancelledTop => cancelled_top(power.bits() / 32 + 4),
        _ => Accumulator::new(),
    };
    match form {
        Form::Split => {
            acc.add_limb_value(power);
            acc += offset;
            if negative {
                acc = -acc;
            }
        }
        _ if negative => acc.sub_limb_value(&magnitude),
        _ => acc.add_limb_value(&magnitude),
    }
    if let Form::Normalized = form {
        acc.normalize();
    }
    (acc, oracle)
}

/// Every value within four of a power of two through `2^700`, or of a power of
/// ten through `10^210`, formats in decimal as its big integer does.
///
/// Each value is tested in both signs and every stored form, and formatting
/// leaves a stored form that still reads as the same value.
#[test]
fn display_matches_bigint_near_powers() {
    let powers = (3..=MAX_POWER_OF_TWO)
        .map(|exponent| BigUint::from(2u8).pow(exponent))
        .chain((1..=MAX_POWER_OF_TEN).map(|exponent| BigUint::from(10u8).pow(exponent)));
    for power in powers {
        for offset in -4..=4 {
            for negative in [false, true] {
                for form in [
                    Form::Plain,
                    Form::UnderCancelledTop,
                    Form::Split,
                    Form::Normalized,
                ] {
                    let (acc, oracle) = build(&power, offset, negative, form);
                    assert_eq!(
                        acc.to_string(),
                        oracle.to_string(),
                        "decimal of {power} + {offset}, negative {negative}, {form:?}"
                    );
                    assert_value(&acc, &oracle);
                }
            }
        }
    }
}

/// Zero formats as `0` in the scalar form, the digit form, and a cancelled
/// digit form.
#[test]
fn zero_formats_as_zero_in_every_form() {
    let mut digits = Accumulator::new();
    digits.value.get_mut().ensure_digits();
    for zero in [Accumulator::new(), digits, cancelled_top(30)] {
        assert_eq!(zero.to_string(), "0");
        assert_eq!(format!("{zero:+}"), "+0");
        assert_eq!(zero.sign_biguint().1, BigUint::ZERO);
    }
}

/// `Debug` honors the caller's flags: `{:x?}` and `{:X?}` write the stored
/// numbers in hexadecimal, in both representations, and `{:#?}` pretty-prints.
#[test]
fn debug_honors_the_formatters_flags() {
    let small = Accumulator::from(0xab);
    assert!(format!("{small:?}").contains("Some(171)"));
    assert!(format!("{small:x?}").contains("Some(ab)"));
    assert!(format!("{small:X?}").contains("Some(AB)"));

    let mut digits = Accumulator::new();
    digits.add_shifted_limbs(64, [0xcd]);
    assert!(format!("{digits:?}").contains("[0, 0, 205]"));
    assert!(format!("{digits:x?}").contains("[0, 0, cd]"));
    assert!(format!("{digits:X?}").contains("[0, 0, CD]"));
    assert!(format!("{digits:#?}").contains("\n    small: None,\n"));
}
