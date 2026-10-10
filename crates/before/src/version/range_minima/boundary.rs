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
    pub(super) fn of(difference: Accumulator) -> Self {
        match difference.sign() {
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
    /// Use a word when the difference's working width is at most 64 bits.
    ///
    /// The minimum tracker's cost argument rests on moving boundaries on push
    /// and pop without reading them: a wide boundary is read only when a drop
    /// crosses and consumes it. A wider difference therefore stays `Wide` even
    /// if its value would fit a word. Converting it would read its top through
    /// the accumulator's stability query, compacting the top only to learn
    /// that it does not fit, once for every boundary pushed.
    pub(super) fn from_positive(Positive(difference): Positive) -> Self {
        if difference.stored_bits() > 64 {
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

    /// Subtract the smaller of two wide values from the larger, deciding which
    /// is smaller along the way.
    ///
    /// This fuses the comparison with the subtraction, which `Ord` does not:
    /// for close values, one in-place subtraction followed by a sign read both
    /// decides the order and leaves the difference, where `Ord` would first
    /// subtract into a temporary. When one value's working width leads by at
    /// least 64 bits, its stability query is asked first, since it can prove
    /// the order without reading the narrower value, and a refusal compacts
    /// the wider one before the subtraction.
    fn lower_wide(mut boundary: Accumulator, Positive(mut decrease): Positive) -> Remainder {
        if decrease.stored_bits() >= boundary.stored_bits() + 64 {
            match decrease.cmp_zero_stable_under(boundary.stored_bits()) {
                Some(Ordering::Greater) => {
                    decrease -= &boundary;
                    return Remainder::Decrease(Positive(decrease));
                }
                Some(_) => unreachable!("the decrease is positive"),
                None => {}
            }
        }
        if boundary.stored_bits() >= decrease.stored_bits() + 64 {
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
