//! The causal rank: [`Rank`], the exact measure of an event tree.
//!
//! The public contract lives on the type and on
//! [`Version::rank`](crate::Version::rank); the fold that computes it is
//! the skyline query kernel. This module is private.

use core::cmp::Ordering;
use core::fmt;
use core::iter::Sum;
use core::ops::{Add, AddAssign};

use crate::codec::accum::Accum;
use crate::codec::Base;

/// The causal rank of a [`Version`](crate::Version).
///
/// The exact area under its event tree, a nonnegative dyadic rational `num ·
/// 2⁻ᵉˣᵖ` with arbitrary-precision numerator. Produced by
/// [`Version::rank`](crate::Version::rank).
///
/// An event tree is a height function over the unit id interval: a leaf
/// `n` is height `n` everywhere, and a node `(n, l, r)` lifts its children
/// by `n`, each over half the parent's width. The area under that function
/// — `Σ base · 2⁻ᵈᵉᵖᵗʰ` over every node — grows whenever the function
/// grows anywhere, and the causal order on versions *is* pointwise
/// comparison of their height functions. The area is therefore a
/// **strictly monotone rank**:
///
/// > if `v < w` then `v.rank() < w.rank()`.
///
/// Heights are step functions on dyadic intervals, so two distinct
/// versions ordered by `<` differ over an interval of positive width, and
/// the dominated one strictly loses area there. The contrapositive is what
/// consumers lean on: **equal ranks are never causally ordered** (they are
/// the same version or concurrent). Any tiebreak between equal ranks — a
/// content hash, [`as_bytes`](crate::Version::as_bytes) — therefore
/// extends the causal order to a total one, which is what makes `Rank` fit
/// for sorted-container keys that must deliver causes before effects.
///
/// [`min_ticks`](crate::Version::min_ticks) is the integer shadow of this
/// measure (every width rounded up to the whole interval): a valid but
/// only *weakly* monotone rank, blind to growth that fills concurrent gaps
/// — `(0, 1, 0) < 1`, yet both count one tick. The rank separates every
/// such pair exactly.
///
/// Totally ordered ([`Ord`]), unlike the versions it ranks. Comparison is
/// exact at any magnitude: mismatched magnitude classes are decided from
/// the stored widths in O(1), and class ties stream the numerators
/// most-significant-first, so no alignment of the exponents is ever
/// materialized; equality is structural (the stored form is normalized, so
/// equal values are identical representations, consistent with [`Hash`]).
///
/// # Complexity
///
/// A rank has no byte encoding; its costs are denominated in *numeric
/// size* `‖r‖` — the numerator's bit width plus the exponent — which every
/// producing fold ([`Version::rank`](crate::Version::rank),
/// [`distance`](crate::Version::distance), [`lag`](crate::Version::lag))
/// keeps linear in the packed bits it read. Comparison (`==`, [`Ord`]) is
/// `O(1)` when the two magnitudes differ in scale and `O(‖a‖ + ‖b‖)` time
/// with no allocation on scale ties; hashing and cloning are `O(‖r‖)`.
/// Addition (`+`, `+=`) is `O(‖a‖ + ‖b‖)` time and space, and an n-ary
/// [`Sum`] is `O(N)` in the summands' total numeric size `N`: the fold
/// carries one running accumulator, and each summand pays its own width
/// rather than the accumulator's. Rendering (`Display`) is `O(d)` space
/// in the `d` decimal digits printed, but its time additionally pays
/// binary-to-decimal conversion of the numerator, superlinear (though
/// subquadratic) in its width past a machine word.
///
/// ```
/// use before::Version;
/// let half: Version = "(0, 1, 0)".parse().unwrap(); // height 1 over half the interval
/// let one = Version::try_from(1).unwrap();          // height 1 everywhere
/// assert!(half < one);                              // strictly dominated...
/// assert!(half.rank() < one.rank());                // ...so strictly smaller rank
/// assert_eq!(half.min_ticks(), one.min_ticks());    // the tick floor cannot see it
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Rank {
    /// The numerator. Normalized: odd, or zero with `exp` zero, so each
    /// value has exactly one representation.
    num: Base,
    /// The (binary) exponent of the denominator `2^exp`. Bounded by the
    /// event tree's depth, since each level halves the interval width.
    exp: u32,
}

impl Rank {
    /// The zero rank: the area under the empty [`Version`](crate::Version),
    /// and the identity for [`Rank`] addition. Equal to
    /// [`Version::new().rank()`](crate::Version::rank).
    ///
    /// ```
    /// use before::{Rank, Version};
    /// assert_eq!(Version::new().rank(), Rank::ZERO);
    /// assert_eq!(Version::try_from(7).unwrap().rank() + Rank::ZERO,
    ///            Version::try_from(7).unwrap().rank());
    /// ```
    pub const ZERO: Rank = Rank {
        num: Base::ZERO,
        exp: 0,
    };

    /// The difference `self - rhs`, or [`None`] when `rhs` exceeds `self`.
    ///
    /// Ranks are nonnegative dyadic rationals — a totally ordered commutative
    /// monoid under [`+`](Add), not a group — so subtraction is partial. The
    /// difference exists exactly when `rhs <= self`; the
    /// [`distance`](crate::Version::distance) and [`lag`](crate::Version::lag)
    /// measures call this where the lattice guarantees the minuend dominates,
    /// so the [`None`] arm is unreachable for them.
    ///
    /// # Complexity
    ///
    /// `O(‖a‖ + ‖b‖)` time and space in the operands' numeric size (see
    /// [the type's note](Rank#complexity)); a [`None`] or zero result costs
    /// only the comparison, which allocates nothing.
    ///
    /// ```
    /// use before::Version;
    /// let five = Version::try_from(5).unwrap().rank();
    /// let three = Version::try_from(3).unwrap().rank();
    /// assert_eq!(five.checked_sub(&three).unwrap().to_string(), "2");
    /// assert!(three.checked_sub(&five).is_none()); // 3 - 5 has no nonnegative value
    /// ```
    pub fn checked_sub(&self, rhs: &Rank) -> Option<Rank> {
        // The ordering pre-check rides the class-first comparison, so the
        // `None` and zero arms cost no alignment at all; only a strictly
        // positive difference aligns to the common exponent and subtracts,
        // and that transient is the output's own value content.
        match self.cmp(rhs) {
            Ordering::Less => None,
            Ordering::Equal => Some(Rank::ZERO),
            Ordering::Greater => {
                let e = self.exp.max(rhs.exp);
                let a = self.num.clone() << (e - self.exp);
                let b = rhs.num.clone() << (e - rhs.exp);
                Some(Rank::from_raw(a - &b, e))
            }
        }
    }

    /// The rank's value content in bits: `bits(num) + exp`.
    ///
    /// The meter denominator of record for `Rank` operands, which have no
    /// packed encoding to charge against: the numerator's bit width plus
    /// the exponent bounds the information the value carries, and every
    /// public construction path emits ranks whose content is linear in the
    /// packed bits it read, so a cost linear in this quantity is linear in
    /// wire terms too.
    #[cfg(any(test, feature = "meter"))]
    pub(crate) fn content_bits(&self) -> u64 {
        self.num.bits() + u64::from(self.exp)
    }

    /// The stored parts `(numerator, exponent)`, for the reference
    /// computations and differential oracles that re-derive the order and
    /// the arithmetic from the raw normalized form.
    #[cfg(test)]
    pub(crate) fn raw_parts(&self) -> (&Base, u32) {
        (&self.num, self.exp)
    }

    /// Normalize raw fold output `num · 2⁻ᵉˣᵖ` into canonical form: strip
    /// the factors of two shared by numerator and denominator, and pin zero
    /// to exponent zero, so structural equality is value equality.
    ///
    /// `pub(crate)` for the reference computations (the oracle's tree fold,
    /// the semantic oracle's Riemann sum), which produce the same raw form.
    pub(crate) fn from_raw(num: Base, exp: u32) -> Self {
        match num.trailing_zeros() {
            None => Rank {
                num: Base::ZERO,
                exp: 0,
            },
            Some(tz) => {
                let shift = u32::try_from(tz.min(u64::from(exp))).expect("min with a u32");
                Rank {
                    num: num >> shift,
                    exp: exp - shift,
                }
            }
        }
    }
}

impl Ord for Rank {
    fn cmp(&self, other: &Self) -> Ordering {
        // Class first: `bits(num) − exp` is `floor(log2 value) + 1`, so
        // unequal classes order the values in O(1) — value ranges
        // `[2^(c−1), 2^c)` at distinct `c` never overlap. A class tie
        // means the two numerators' bit strings are already MSB-aligned
        // as binary fractions, and the streamed window comparison settles
        // them without materializing an alignment shift; its
        // longer-string-wins tail rule is sound because normalization
        // keeps numerators odd (the longer string ends in a set bit). The
        // order is exact at any magnitude — a false tie here would let a
        // consumer deliver an effect before its cause. Zero (the one
        // even-numerator form, pinned to exponent zero) is settled before
        // classes: its class value would collide with genuine
        // `(0, 1]`-range ranks.
        match (self.num.bits() == 0, other.num.bits() == 0) {
            (true, true) => return Ordering::Equal,
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            (false, false) => {}
        }
        let class = |r: &Rank| i128::from(r.num.bits()) - i128::from(r.exp);
        class(self)
            .cmp(&class(other))
            .then_with(|| Base::msb_cmp(&self.num, &other.num))
    }
}

impl PartialOrd for Rank {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// `Rank` under `+` is a commutative monoid with identity [`Rank::ZERO`]: the
// exact sum of two dyadic rationals, normalized so equal values stay
// structurally equal. It is *not* the [`Version`](crate::Version) join — the
// join takes a pointwise maximum, whereas this adds areas — but the two meet in
// the valuation law `rank(a | b) + rank(a & b) == rank(a) + rank(b)`, which is
// what makes [`Version::distance`](crate::Version::distance) a metric. The four
// reference forms mirror [`Base`]'s own `Add` matrix so callers need not place
// borrows by hand.

/// Adds two ranks: the exact sum of the two areas.
///
/// Rank addition is *measure* arithmetic, not history arithmetic: areas
/// add, histories join. Its meaning comes from the rank being a valuation
/// on the version lattice — `rank(a | b) + rank(a & b) == rank(a) +
/// rank(b)` — which is what makes
/// [`distance`](crate::Version::distance) a metric and lets its directed
/// halves recombine (`a.lag(b) + b.lag(a) == a.distance(b)`). Use `+` to
/// aggregate measures — a total replication backlog across peers, a
/// budget consumed so far — never to combine histories; that is the
/// version join `|`.
impl Add<&Rank> for &Rank {
    type Output = Rank;
    fn add(self, rhs: &Rank) -> Rank {
        let e = self.exp.max(rhs.exp);
        let a = self.num.clone() << (e - self.exp);
        let b = rhs.num.clone() << (e - rhs.exp);
        Rank::from_raw(a + &b, e)
    }
}

impl Add<Rank> for Rank {
    type Output = Rank;
    fn add(self, rhs: Rank) -> Rank {
        &self + &rhs
    }
}

impl Add<&Rank> for Rank {
    type Output = Rank;
    fn add(self, rhs: &Rank) -> Rank {
        &self + rhs
    }
}

impl Add<Rank> for &Rank {
    type Output = Rank;
    fn add(self, rhs: Rank) -> Rank {
        self + &rhs
    }
}

impl AddAssign<&Rank> for Rank {
    fn add_assign(&mut self, rhs: &Rank) {
        *self = &*self + rhs;
    }
}

impl AddAssign<Rank> for Rank {
    fn add_assign(&mut self, rhs: Rank) {
        *self = &*self + &rhs;
    }
}

/// The empty sum is [`Rank::ZERO`], the additive identity.
impl Sum<Rank> for Rank {
    fn sum<I: Iterator<Item = Rank>>(iter: I) -> Rank {
        sum_ranks(iter)
    }
}

/// The empty sum is [`Rank::ZERO`], the additive identity.
impl<'a> Sum<&'a Rank> for Rank {
    fn sum<I: Iterator<Item = &'a Rank>>(iter: I) -> Rank {
        sum_ranks(iter)
    }
}

/// Sum ranks through one raw accumulator with a single final
/// normalization.
///
/// The accumulator holds the running numerator at the largest exponent
/// seen so far: a summand at a smaller exponent is digit-routed in at the
/// exponent gap (O(its own limbs), independent of the gap), and a summand
/// raising the maximum rescales the accumulator once, O(held digits) —
/// paid by the exponent the summand itself carries. Nothing renormalizes
/// per element, so a high-exponent summand costs its own width once
/// instead of once per later element, and the result is the identical
/// [`Rank`] the pairwise fold produces (one exact value, one shared
/// normalization at the end).
fn sum_ranks<T: core::borrow::Borrow<Rank>, I: Iterator<Item = T>>(iter: I) -> Rank {
    let mut acc = Accum::new();
    let mut exp = 0u32;
    for rank in iter {
        let rank = rank.borrow();
        if rank.exp > exp {
            acc.shl(u64::from(rank.exp - exp));
            exp = rank.exp;
        }
        acc.add_base_shl(&rank.num, u64::from(exp - rank.exp));
    }
    let (sign, magnitude) = acc.sign_magnitude();
    debug_assert_ne!(
        sign,
        Ordering::Less,
        "a sum of nonnegative ranks is nonnegative"
    );
    Rank::from_raw(Base::from(magnitude), exp)
}

/// [`Rank::ZERO`], the additive identity.
impl Default for Rank {
    fn default() -> Self {
        Rank::ZERO
    }
}

/// Renders as the exact rational: the numerator alone when integral,
/// `num/2^exp` otherwise.
///
/// ```
/// use before::Version;
/// assert_eq!(Version::try_from(5).unwrap().rank().to_string(), "5");
/// let half: Version = "(0, 1, 0)".parse().unwrap();
/// assert_eq!(half.rank().to_string(), "1/2^1");
/// ```
impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.exp {
            0 => fmt::Display::fmt(&self.num, f),
            exp => write!(f, "{}/2^{}", self.num, exp),
        }
    }
}

/// The same format as `Display`.
impl fmt::Debug for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Display>::fmt(self, f)
    }
}
