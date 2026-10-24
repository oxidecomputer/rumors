//! Reference oracle for exact ranks: a dyadic rational held unnormalized.
//!
//! A [`Dyadic`] is the pair `(num, exp)` meaning `num / 2^exp`. It compares
//! by cross-multiplication and adds by aligning exponents, so it never
//! normalizes and shares nothing with production's alignment, accumulator,
//! or class-first comparison. [`Dyadic::render`] writes a value's canonical
//! binary text from the pair alone, so [`Dyadic::rank`] builds a production
//! rank without production's printer, and comparing production's `Display`
//! with the rendered text checks the printer without production's parser.

use core::cmp::Ordering;

use num_bigint::BigUint;

use crate::Rank;

/// The dyadic rational `num / 2^exp`, in any spelling: `(num, exp)` and
/// `(2 · num, exp + 1)` denote the same value.
#[derive(Clone, Debug)]
pub(crate) struct Dyadic {
    /// The numerator.
    pub(crate) num: BigUint,
    /// The power of two dividing the numerator.
    pub(crate) exp: u64,
}

impl Dyadic {
    /// The value zero.
    pub(crate) fn zero() -> Self {
        Dyadic {
            num: BigUint::ZERO,
            exp: 0,
        }
    }

    /// Compares two values by cross-multiplication: `a / 2^e` lies below
    /// `b / 2^f` exactly when `a · 2^f` lies below `b · 2^e`.
    pub(crate) fn cmp_value(&self, other: &Dyadic) -> Ordering {
        (&self.num << other.exp).cmp(&(&other.num << self.exp))
    }

    /// The exact sum, spelled at the larger of the two exponents.
    pub(crate) fn add(&self, other: &Dyadic) -> Dyadic {
        let exp = self.exp.max(other.exp);
        Dyadic {
            num: (&self.num << (exp - self.exp)) + (&other.num << (exp - other.exp)),
            exp,
        }
    }

    /// The value's canonical binary text: the integer part without leading
    /// zeroes, then, exactly when the value is not integral, a binary point and
    /// the fractional digits, ending in a one.
    ///
    /// Stripping the factors of two the numerator and denominator share first
    /// leaves an odd numerator whenever the exponent is positive, so the
    /// fraction's last digit is a one.
    pub(crate) fn render(&self) -> String {
        let Some(zeros) = self.num.trailing_zeros() else {
            return String::from("0");
        };
        let shift = zeros.min(self.exp);
        let (num, exp) = (&self.num >> shift, self.exp - shift);
        let integer = &num >> exp;
        let mut text = integer.to_str_radix(2);
        if exp > 0 {
            let fraction = &num - (&integer << exp);
            let width = usize::try_from(exp).expect("a rendered fraction fits in memory");
            text.push('.');
            text.push_str(&format!("{:0>width$}", fraction.to_str_radix(2)));
        }
        text
    }

    /// The production rank of this value, built by parsing [`Self::render`]'s
    /// text.
    pub(crate) fn rank(&self) -> Rank {
        self.render()
            .parse()
            .expect("canonical binary text parses as a rank")
    }
}
