//! Positive differences between adjacent minima, and how a decrease crosses them.
//!
//! If `b = inner_min - outer_min` and the inner minimum falls by `d`, the
//! difference `b - d` determines both new minima. A positive result retains
//! the outer minimum and a smaller boundary. Zero joins the minima. A negative
//! result lowers the outer minimum too, by `d - b`.
//!
//! A surviving wide value must not be scanned for every small decrease. Width
//! certificates decide well-separated values first; subtraction then reads
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

/// What remains after subtracting a decrease from a positive boundary.
pub(super) enum Remainder {
    /// The decrease stops here; the outer minimum stays fixed.
    Boundary(Boundary),
    /// The boundary vanishes; the outer minimum falls by this positive amount.
    Decrease(Accumulator),
    /// The minima meet, leaving neither a boundary nor a further decrease.
    Equal,
}

/// Arithmetic that preserves compact boundaries and moves wide buffers.
impl Boundary {
    /// Use a word when the stored width permits a constant-cost conversion.
    ///
    /// A `u64` uses at most two base-2^32 digits. Materializing at most two
    /// digits has constant cost; a wider accumulator is retained directly.
    pub(super) fn from_positive(difference: Accumulator) -> Self {
        #[cfg(debug_assertions)]
        {
            let sign = difference.clone().cmp_zero();
            debug_assert_eq!(sign, Ordering::Greater, "boundaries are positive");
        }
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
    pub(super) fn lowered_by(self, decrease: Accumulator) -> Remainder {
        match self {
            Self::Word(word) => Self::lower_word(word, decrease),
            Self::Wide(wide) => Self::lower_wide(wide, decrease),
        }
    }

    /// A word-sized boundary costs one constant-width subtraction.
    fn lower_word(boundary: u64, mut decrease: Accumulator) -> Remainder {
        decrease -= boundary;
        match decrease.cmp_zero() {
            Ordering::Greater => Remainder::Decrease(decrease),
            Ordering::Equal => Remainder::Equal,
            Ordering::Less => {
                decrease = -decrease;
                Remainder::Boundary(Self::from_positive(decrease))
            }
        }
    }

    /// Compare wide values before choosing which one to subtract from the other.
    ///
    /// Two base-2^32 digits are the first separation at which Accumulator's
    /// redundant representation can certify domination. If a certificate
    /// fails, the values are close enough in stored width that subtracting
    /// once costs no more than processing comparable operands.
    fn lower_wide(mut boundary: Accumulator, mut decrease: Accumulator) -> Remainder {
        if decrease.stored_digit_count() >= boundary.stored_digit_count() + 2 {
            match decrease.cmp_zero_stable_under(boundary.stored_bits()) {
                Some(Ordering::Greater) => {
                    decrease -= &boundary;
                    return Remainder::Decrease(decrease);
                }
                Some(_) => unreachable!("the decrease is positive"),
                None => {}
            }
        }
        if boundary.stored_digit_count() >= decrease.stored_digit_count() + 2 {
            match boundary.cmp_zero_stable_under(decrease.stored_bits()) {
                Some(Ordering::Greater) => {
                    boundary -= &decrease;
                    return Remainder::Boundary(Self::from_positive(boundary));
                }
                Some(_) => unreachable!("stored boundaries are positive"),
                None => {}
            }
        }

        boundary -= &decrease;
        match boundary.cmp_zero() {
            Ordering::Greater => Remainder::Boundary(Self::from_positive(boundary)),
            Ordering::Equal => Remainder::Equal,
            Ordering::Less => {
                boundary = -boundary;
                Remainder::Decrease(boundary)
            }
        }
    }
}
