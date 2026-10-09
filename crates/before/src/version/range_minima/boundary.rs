//! Positive differences between adjacent minima, and how a decrease crosses them.
//!
//! If `b = inner_min - outer_min` and the inner minimum falls by `d`, the
//! difference `b - d` determines both new minima. A positive result retains
//! the outer minimum and a smaller boundary. Zero joins the minima. A negative
//! result lowers the outer minimum too, by `d - b`.
//!
//! A surviving wide value must not be scanned for every small decrease. Width
//! leading-digit comparisons decide well-separated values first; subtraction then reads
//! the smaller operand. Comparable widths need one ordinary subtraction.

use core::cmp::Ordering;

use suanpan::Accumulator;

/// A strictly positive difference, with a compact word form for small values.
pub(super) enum Boundary {
    /// An ordinary difference; packed storage retains only its meaningful bits.
    Word(u64),
    /// A difference retained in arbitrary-precision form.
    Wide(Accumulator),
}

/// A difference classified by its sign, each nonzero side as a magnitude.
pub(super) enum Signed {
    /// The difference was positive.
    Positive(Positive),
    /// The difference was zero.
    Zero,
    /// The difference was negative; this is its magnitude.
    Negative(Positive),
}

/// A strictly positive value.
///
/// Only this module constructs one, and only where it has just established
/// the sign: [`Signed::of`], or a stability query that proves it.
pub(super) struct Positive(Accumulator);

/// Classify a difference with one comparison against zero.
impl Signed {
    /// Compare `difference` with zero once, negating a negative difference.
    pub(super) fn of(mut difference: Accumulator) -> Self {
        match difference.cmp_zero() {
            Ordering::Greater => Self::Positive(Positive(difference)),
            Ordering::Equal => Self::Zero,
            Ordering::Less => Self::Negative(Positive(-difference)),
        }
    }
}

/// What remains after subtracting a decrease from a positive boundary.
pub(super) enum Remainder {
    /// The decrease stops here; the outer minimum stays fixed.
    Boundary(Boundary),
    /// The boundary vanishes; the outer minimum falls by this amount.
    Decrease(Positive),
    /// The minima meet, leaving neither a boundary nor a further decrease.
    Equal,
}

/// Arithmetic that preserves compact boundaries and moves wide buffers.
impl Boundary {
    /// Use a word when the stored width permits a constant-cost conversion.
    ///
    /// A `u64` uses at most two base-2^32 digits. Materializing at most two
    /// digits has constant cost; a wider accumulator is retained directly.
    pub(super) fn from_positive(Positive(difference): Positive) -> Self {
        if difference.stored_digit_count() > 2 {
            return Self::Wide(difference);
        }
        match u64::try_from(difference) {
            Ok(word) => Self::Word(word),
            Err(difference) => Self::Wide(difference),
        }
    }

    /// Transfer this boundary into a newly deferred distance.
    pub(super) fn into_accumulator(self) -> Accumulator {
        match self {
            Self::Word(word) => {
                let mut value = Accumulator::new();
                value += word;
                value
            }
            Self::Wide(value) => value,
        }
    }

    /// Add this boundary to a deferred distance, retaining the wider buffer.
    pub(super) fn add_to(self, deferred: &mut Accumulator) {
        match self {
            Self::Word(word) => *deferred += word,
            Self::Wide(wide) => *deferred += wide,
        }
    }

    /// Lower the inner minimum and return the surviving positive difference.
    pub(super) fn lowered_by(self, decrease: Positive) -> Remainder {
        match self {
            Self::Word(word) => Self::lower_word(word, decrease),
            Self::Wide(wide) => Self::lower_wide(wide, decrease),
        }
    }

    /// A word-sized boundary costs one constant-width subtraction.
    fn lower_word(boundary: u64, Positive(mut decrease): Positive) -> Remainder {
        decrease -= boundary;
        match Signed::of(decrease) {
            Signed::Positive(decrease) => Remainder::Decrease(decrease),
            Signed::Zero => Remainder::Equal,
            Signed::Negative(boundary) => Remainder::Boundary(Self::from_positive(boundary)),
        }
    }

    /// Compare wide values before choosing which one to subtract from the other.
    ///
    /// Two base-2^32 digits are the first separation at which Accumulator's
    /// redundant representation can prove domination. If that comparison
    /// cannot decide, the values are close enough in stored width that subtracting
    /// once costs no more than processing comparable operands.
    fn lower_wide(mut boundary: Accumulator, Positive(mut decrease): Positive) -> Remainder {
        if decrease.stored_digit_count() >= boundary.stored_digit_count() + 2 {
            match decrease.cmp_zero_stable_under(boundary.stored_bits()) {
                Some(Ordering::Greater) => {
                    decrease -= &boundary;
                    return Remainder::Decrease(Positive(decrease));
                }
                Some(_) => unreachable!("the decrease is positive"),
                None => {}
            }
        }
        if boundary.stored_digit_count() >= decrease.stored_digit_count() + 2 {
            match boundary.cmp_zero_stable_under(decrease.stored_bits()) {
                Some(Ordering::Greater) => {
                    boundary -= &decrease;
                    return Remainder::Boundary(Self::from_positive(Positive(boundary)));
                }
                Some(_) => unreachable!("stored boundaries are positive"),
                None => {}
            }
        }

        boundary -= &decrease;
        match Signed::of(boundary) {
            Signed::Positive(boundary) => Remainder::Boundary(Self::from_positive(boundary)),
            Signed::Zero => Remainder::Equal,
            Signed::Negative(decrease) => Remainder::Decrease(decrease),
        }
    }
}
