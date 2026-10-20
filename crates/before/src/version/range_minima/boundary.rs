//! Positive differences between adjacent minima, and how a decrease crosses them.
//!
//! If `b = inner_min - outer_min` and the inner minimum falls by `d`, the
//! difference `b - d` determines both new minima. A positive result retains
//! the outer minimum and a smaller boundary. Zero joins the minima. A negative
//! result lowers the outer minimum too, by `d - b`.
//!
//! A surviving wide value must not be scanned for every small decrease.
//! [`Accumulator::abs_diff`] subtracts the narrower operand from the wider
//! one's storage and decides the order from the result's top, so a small
//! decrease reads only itself and the boundary's top.

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
/// the sign: [`Signed::of`], or the order [`Accumulator::abs_diff`] reports.
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
    /// The minimum tracker's cost argument rests on every push and pop of a
    /// boundary costing a constant, whatever its width: storage copies a wide
    /// boundary only when a later push covers it and its working width is
    /// bounded, and otherwise moves it unread until a drop crosses and
    /// consumes it. A wider difference therefore stays `Wide` even if its
    /// value would fit a word. Converting it would read its top through the
    /// accumulator's stability query, compacting the top only to learn that it
    /// does not fit, once for every boundary pushed.
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

    /// Subtract the smaller of a wide boundary and a decrease from the larger,
    /// in one pass that also decides which is smaller.
    fn lower_wide(boundary: Accumulator, Positive(decrease): Positive) -> Remainder {
        // The boundary receives the subtraction when the widths tie: a
        // surviving boundary then needs no negation.
        match boundary.abs_diff(decrease) {
            (Ordering::Greater, rest) => Remainder::Boundary(Self::from_positive(Positive(rest))),
            (Ordering::Less, rest) => Remainder::Decrease(Positive(rest)),
            (Ordering::Equal, _) => Remainder::Equal,
        }
    }
}
