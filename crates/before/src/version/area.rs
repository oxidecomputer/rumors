//! The causal rank: [`Area`], the exact measure of an event tree, and the
//! fold that computes it. The public contract lives on the type and on
//! [`Version::area`](crate::Version::area); this module is private.

use core::cmp::Ordering;
use core::fmt;

use crate::codec::Base;
use crate::recurse::descend;

use super::compare::EvReader;

/// The exact area under a [`Version`](crate::Version)'s event tree: a
/// nonnegative dyadic rational `num · 2⁻ᵉˣᵖ`, with arbitrary-precision
/// numerator. Produced by [`Version::area`](crate::Version::area).
///
/// An event tree is a height function over the unit id interval: a leaf
/// `n` is height `n` everywhere, and a node `(n, l, r)` lifts its children
/// by `n`, each over half the parent's width. The area under that function
/// — `Σ base · 2⁻ᵈᵉᵖᵗʰ` over every node — grows whenever the function
/// grows anywhere, and the causal order on versions *is* pointwise
/// comparison of their height functions. The area is therefore a
/// **strictly monotone rank**:
///
/// > if `v < w` then `v.area() < w.area()`.
///
/// Heights are step functions on dyadic intervals, so two distinct
/// versions ordered by `<` differ over an interval of positive width, and
/// the dominated one strictly loses area there. The contrapositive is what
/// consumers lean on: **equal areas are never causally ordered** (they are
/// the same version or concurrent). Any tiebreak between equal areas — a
/// content hash, [`as_bytes`](crate::Version::as_bytes) — therefore
/// extends the causal order to a total one, which is what makes `Area` fit
/// for sorted-container keys that must deliver causes before effects.
///
/// [`min_ticks`](crate::Version::min_ticks) is the integer shadow of this
/// measure (every width rounded up to the whole interval): a valid but
/// only *weakly* monotone rank, blind to growth that fills concurrent gaps
/// — `(0, 1, 0) < 1`, yet both count one tick. The area separates every
/// such pair exactly.
///
/// Totally ordered ([`Ord`]), unlike the versions it ranks. Comparison
/// aligns the two exponents and compares numerators, exact at any
/// magnitude; equality is structural (the stored form is normalized, so
/// equal values are identical representations, consistent with [`Hash`]).
///
/// ```
/// use before::Version;
/// let half: Version = "(0, 1, 0)".parse().unwrap(); // height 1 over half the interval
/// let one = Version::try_from(1).unwrap();          // height 1 everywhere
/// assert!(half < one);                              // strictly dominated...
/// assert!(half.area() < one.area());                // ...so strictly smaller area
/// assert_eq!(half.min_ticks(), one.min_ticks());    // the tick floor cannot see it
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Area {
    /// The numerator. Normalized: odd, or zero with `exp` zero, so each
    /// value has exactly one representation.
    num: Base,
    /// The (binary) exponent of the denominator `2^exp`. Bounded by the
    /// event tree's depth, since each level halves the interval width.
    exp: u32,
}

impl Area {
    /// Normalize raw fold output `num · 2⁻ᵉˣᵖ` into canonical form: strip
    /// the factors of two shared by numerator and denominator, and pin zero
    /// to exponent zero, so structural equality is value equality.
    /// `pub(crate)` for the reference computations (the oracle's tree fold,
    /// the semantic oracle's Riemann sum), which produce the same raw form.
    pub(crate) fn from_raw(num: Base, exp: u32) -> Self {
        match num.trailing_zeros() {
            None => Area {
                num: Base::ZERO,
                exp: 0,
            },
            Some(tz) => {
                let shift = u32::try_from(tz.min(u64::from(exp))).expect("min with a u32");
                Area {
                    num: num >> shift,
                    exp: exp - shift,
                }
            }
        }
    }
}

impl Ord for Area {
    fn cmp(&self, other: &Self) -> Ordering {
        // Align the exponents, then compare numerators: `a/2^x` versus
        // `b/2^y` is `a·2^(e−x)` versus `b·2^(e−y)` at the common `e`. The
        // shift is exact at any magnitude (`Base` spills to a bignum), so
        // the order is never approximated — a false tie here would let a
        // consumer deliver an effect before its cause.
        match self.exp.cmp(&other.exp) {
            Ordering::Equal => self.num.cmp(&other.num),
            _ => {
                let e = self.exp.max(other.exp);
                let a = self.num.clone() << (e - self.exp);
                let b = other.num.clone() << (e - other.exp);
                a.cmp(&b)
            }
        }
    }
}

impl PartialOrd for Area {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Renders as the exact rational: the numerator alone when integral,
/// `num/2^exp` otherwise.
///
/// ```
/// use before::Version;
/// assert_eq!(Version::try_from(5).unwrap().area().to_string(), "5");
/// let half: Version = "(0, 1, 0)".parse().unwrap();
/// assert_eq!(half.area().to_string(), "1/2^1");
/// ```
impl fmt::Display for Area {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.exp {
            0 => fmt::Display::fmt(&self.num, f),
            exp => write!(f, "{}/2^{}", self.num, exp),
        }
    }
}

/// The same format as `Display`.
impl fmt::Debug for Area {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Display>::fmt(self, f)
    }
}

impl EvReader<'_> {
    /// The exact area under this event subtree, in units of its own
    /// interval width (see [`Version::area`](crate::Version::area)).
    /// Advances the cursor past the subtree. `O(n)` node visits.
    pub(in crate::version) fn area(&mut self) -> Area {
        let (num, exp) = descend!(0, area_rec(self, 0));
        Area::from_raw(num, exp)
    }
}

/// The area of the subtree at `ev` as a raw `(numerator, exponent)` pair in
/// subtree-relative units (the subtree's interval has width 1), advancing
/// `ev` past it: a leaf is its base; a node is its base plus half the sum of
/// its children's areas. The recursive form, routed through the amortized
/// stack-growth guard so a deep tree grows the stack onto the heap rather
/// than overflowing.
fn area_rec(ev: &mut EvReader, depth: usize) -> (Base, u32) {
    let node = ev.read();
    let base = node.base().clone();
    if !node.is_internal() {
        return (base, 0);
    }
    // Internal: the `&mut` advances through the left subtree, then the right
    // resumes from it.
    let (l_num, l_exp) = descend!(depth + 1, area_rec(ev, depth + 1));
    let (r_num, r_exp) = descend!(depth + 1, area_rec(ev, depth + 1));
    // Children's sum at their common exponent, halved (exponent + 1), plus
    // this node's base lifted to that scale.
    let exp = l_exp.max(r_exp);
    let sum = (l_num << (exp - l_exp)) + (r_num << (exp - r_exp));
    ((base << (exp + 1)) + sum, exp + 1)
}
