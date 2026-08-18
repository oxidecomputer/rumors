use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::ops::{Add, AddAssign, BitOr, MulAssign, Shl, Shr, Sub, SubAssign};

use dashu_int::ops::BitTest;
use dashu_int::UBig;
use suanpan::Limbs;

// Test-only metering for big-arithmetic operations:
#[cfg(feature = "limb-meter")]
pub(crate) mod limb_meter;
pub(crate) mod limb_metered;
use limb_metered::*;

/// An event tree's stored integer magnitude.
///
/// ITC event counts (path sums of `tick`s, the `max`/`join` of two such sums)
/// grow without bound, so the value type preserves arbitrary precision: no
/// `u64` overflow class, in any build profile. A thin metered wrapper around
/// [`UBig`]: every operation records its operands' 64-bit limb widths into the
/// limb meter, then delegates the arithmetic whole. Values up to two machine
/// words stay inline in the wrapped representation, so the common small
/// magnitudes never allocate.
#[derive(Clone, Debug, Eq)]
pub struct Base(pub(crate) UBig);

impl Base {
    pub(crate) const ZERO: Base = Base(UBig::ZERO);

    /// The magnitude's bit length: zero for zero, `floor(log2 n) + 1`
    /// otherwise.
    pub(crate) fn bits(&self) -> u64 {
        self.0.bit_len() as u64
    }

    /// This magnitude as a `u64`, or `None` past the `u64` range.
    ///
    /// The dispatch point for the word-sized fast paths (the rank fold's
    /// inline arithmetic, the accumulator's amortized-O(1) small adds):
    /// O(1), no allocation.
    pub(crate) fn to_u64(&self) -> Option<u64> {
        u64::try_from(&self.0).ok()
    }

    pub(crate) fn bit(&self, i: u64) -> bool {
        // A bit index past `usize` can only address zeros: the value's own
        // bit length always fits a `usize`.
        usize::try_from(i).map(|i| self.0.bit(i)).unwrap_or(false)
    }

    /// The number of trailing zero bits, or `None` for zero (which has no
    /// lowest set bit). Used by [`Rank`](crate::Rank) normalization to strip
    /// factors of two out of a dyadic numerator.
    ///
    /// Width-scale work — the backend scans limbs bottom-up for the lowest
    /// set bit — so the limb meter records the operand's width like every
    /// other width-scale operation here.
    pub(crate) fn trailing_zeros(&self) -> Option<u64> {
        meter_limbs_solo(self);
        self.0.trailing_zeros().map(|n| n as u64)
    }

    /// The number of 64-bit limbs this magnitude occupies, at least one:
    /// even a zero costs a word of arithmetic.
    #[cfg(feature = "limb-meter")]
    fn limbs(&self) -> u64 {
        self.bits().div_ceil(64).max(1)
    }

    /// Parse a run of ASCII decimal digits into a magnitude.
    ///
    /// The radix conversion is delegated whole to the backend, whose
    /// divide-and-conquer parser is subquadratic in the digit count
    /// \[measured — the dependency-selection probe: parse exponent 1.49
    /// over doubling digit counts\]. The conversion therefore runs inside
    /// the dependency, below the limb shim, so this records one
    /// width-proportional limb count for the materialized value — the
    /// same convention as the wide-gamma decode — and the bench judge's
    /// time leg is what judges the conversion's complexity class.
    ///
    /// The caller guarantees `digits` is nonempty pure ASCII digits;
    /// leading zeros are value-preserving (`"007"` is 7).
    pub(crate) fn parse_decimal(digits: &str) -> Base {
        debug_assert!(
            !digits.is_empty() && digits.bytes().all(|d| d.is_ascii_digit()),
            "parse_decimal takes a nonempty ASCII digit run"
        );
        let value: UBig = digits
            .parse()
            .expect("a nonempty ASCII digit run is a valid decimal magnitude");
        #[cfg(feature = "limb-meter")]
        limb_meter::record_wide(&value);
        Base(value)
    }

    /// Compare two magnitudes as MSB-aligned bit strings: the order of
    /// `a · 2^x` versus `b · 2^y` whenever the two values share a magnitude
    /// class (`bits(a) − x == bits(b) − y`).
    ///
    /// Streams 64-bit windows most-significant-first — no alignment shift
    /// is ever materialized — and stops at the first differing window, so
    /// the cost is O(shared-prefix limbs) with zero allocation. When every
    /// shared window agrees, the longer bit string is the larger value:
    /// this rides on the caller's normalization invariant that the strings
    /// end in a set bit (an odd numerator), so the longer string's
    /// extension is nonzero. The limb meter records one limb per streamed
    /// window, keeping the metered cost honest about the scan.
    pub(crate) fn msb_cmp(a: &Base, b: &Base) -> Ordering {
        let mut wa = MsbWindows::new(a);
        let mut wb = MsbWindows::new(b);
        loop {
            match (wa.next(), wb.next()) {
                (Some(x), Some(y)) => {
                    #[cfg(feature = "limb-meter")]
                    limb_meter::record(2);
                    match x.cmp(&y) {
                        Ordering::Equal => continue,
                        decided => return decided,
                    }
                }
                (Some(_), None) => return Ordering::Greater,
                (None, Some(_)) => return Ordering::Less,
                (None, None) => return Ordering::Equal,
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn to_bytes_le(&self) -> Vec<u8> {
        self.0.to_le_bytes().into_vec()
    }

    /// Assemble a magnitude from big-endian bytes.
    ///
    /// The materialization point for values parsed out of a bit stream
    /// (the rank decoder's integral and fraction reads), so it records
    /// one width-proportional limb count — the wide-gamma decode and
    /// `parse_decimal` convention: the backend materializes every limb
    /// of the value, and a meter that missed it would let a decoder
    /// build arbitrarily wide values while reading zero.
    pub(crate) fn from_be_bytes(bytes: &[u8]) -> Base {
        let value = UBig::from_be_bytes(bytes);
        #[cfg(feature = "limb-meter")]
        limb_meter::record_wide(&value);
        Base(value)
    }
}

/// The 64-bit windows of a magnitude's bit string, most-significant first.
///
/// The first window is the value's top 64 bits left-aligned (the MSB in
/// bit 63); the last is zero-padded below the final significant bit. A
/// zero value has no windows. Streams the stored limbs top-down with one
/// register of carry, so a window costs O(1) and no shifted copy of the
/// value ever exists.
struct MsbWindows<'a> {
    /// Remaining limbs, top first; exhausted once the tail is consumed.
    limbs: core::iter::Rev<Limbs<'a>>,
    /// The previously consumed limb, still owed its low bits.
    held: Option<u64>,
    /// The left-alignment shift: `64 − (bits mod 64)`, zero for a
    /// limb-aligned width.
    shift: u32,
}

impl<'a> MsbWindows<'a> {
    fn new(value: &'a Base) -> Self {
        let bits = value.bits();
        MsbWindows {
            limbs: Limbs::new(&value.0).rev(),
            held: None,
            shift: ((64 - bits % 64) % 64) as u32,
        }
    }
}

impl Iterator for MsbWindows<'_> {
    type Item = u64;

    fn next(&mut self) -> Option<u64> {
        if self.shift == 0 {
            // Limb-aligned: every window is a stored limb verbatim.
            return self.limbs.next();
        }
        match (self.held.take(), self.limbs.next()) {
            // The first window: the top limb left-aligned, topped up from
            // the next limb if there is one.
            (None, Some(top)) => match self.limbs.next() {
                Some(next) => {
                    self.held = Some(next);
                    Some((top << self.shift) | (next >> (64 - self.shift)))
                }
                None => Some(top << self.shift),
            },
            // A middle window: the held limb's low bits over the next
            // limb's high bits.
            (Some(held), Some(next)) => {
                self.held = Some(next);
                Some((held << self.shift) | (next >> (64 - self.shift)))
            }
            // The final window: the last held limb's low bits, zero-padded.
            (Some(held), None) => Some(held << self.shift),
            (None, None) => None,
        }
    }
}

// Structural equality, identical to the derived semantics: the wrapped
// magnitudes must be equal. Manual only so the limb meter records the
// operand widths: equality over spilled magnitudes is width-scale work
// (the decoder's equal-leaf check, the builder's collapse check) that
// every other meter is blind to.
impl PartialEq for Base {
    fn eq(&self, other: &Self) -> bool {
        meter_limbs2(self, other);
        self.0 == other.0
    }
}

// The derived stream: the wrapped magnitude's own hash. Manual only so the
// limb meter records the operand width (hashing walks every limb).
// Consistent with `PartialEq` above: equal values are structurally
// identical, so they feed identical streams to the hasher.
impl Hash for Base {
    fn hash<H: Hasher>(&self, state: &mut H) {
        meter_limbs_solo(self);
        self.0.hash(state);
    }
}

// The accumulator seam: `Base` drives `suanpan::Accumulator`'s
// width-dispatched entry points (`add_magnitude`, `sub_magnitude_shl`, …) —
// a word-scale magnitude takes the amortized-O(1) small path, a spilled one
// the O(operand limbs) wide path — with the inline storage answering the
// dispatch read in O(1). The differential tests below drive both dispatch
// arms against an exact `IBig` oracle; the dispatch pins alongside them hold
// `to_word` to that O(1) — word-scale answers exact, zero digit touches
// under the limb-metered build.
impl suanpan::Magnitude for Base {
    fn to_word(&self) -> Option<u64> {
        self.to_u64()
    }

    fn as_wide(&self) -> &UBig {
        &self.0
    }
}

impl Ord for Base {
    fn cmp(&self, other: &Self) -> Ordering {
        meter_limbs2(self, other);
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for Base {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Base {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl From<UBig> for Base {
    fn from(n: UBig) -> Self {
        Base(n)
    }
}

impl From<u8> for Base {
    fn from(n: u8) -> Self {
        Base(UBig::from(n))
    }
}

impl From<u32> for Base {
    fn from(n: u32) -> Self {
        Base(UBig::from(n))
    }
}

impl From<u64> for Base {
    fn from(n: u64) -> Self {
        Base(UBig::from(n))
    }
}

impl From<u128> for Base {
    fn from(n: u128) -> Self {
        Base(UBig::from(n))
    }
}

impl Add<&Base> for &Base {
    type Output = Base;

    fn add(self, rhs: &Base) -> Base {
        meter_limbs2(self, rhs);
        Base(&self.0 + &rhs.0)
    }
}

impl Add<Base> for &Base {
    type Output = Base;

    fn add(self, rhs: Base) -> Base {
        self + &rhs
    }
}

impl Add<&Base> for Base {
    type Output = Base;

    fn add(self, rhs: &Base) -> Base {
        &self + rhs
    }
}

impl Add<Base> for Base {
    type Output = Base;

    fn add(self, rhs: Base) -> Base {
        &self + &rhs
    }
}

impl Add<u32> for Base {
    type Output = Base;

    fn add(self, rhs: u32) -> Base {
        meter_limbs1(&self);
        Base(self.0 + rhs)
    }
}

impl Add<u32> for &Base {
    type Output = Base;

    fn add(self, rhs: u32) -> Base {
        meter_limbs1(self);
        Base(&self.0 + rhs)
    }
}

impl Add<u64> for Base {
    type Output = Base;

    fn add(self, rhs: u64) -> Base {
        meter_limbs1(&self);
        Base(self.0 + rhs)
    }
}

impl Add<u64> for &Base {
    type Output = Base;

    fn add(self, rhs: u64) -> Base {
        meter_limbs1(self);
        Base(&self.0 + rhs)
    }
}

impl AddAssign<&Base> for Base {
    fn add_assign(&mut self, rhs: &Base) {
        meter_limbs2(self, rhs);
        self.0 += &rhs.0;
    }
}

impl AddAssign<u32> for Base {
    fn add_assign(&mut self, rhs: u32) {
        meter_limbs1(self);
        self.0 += rhs;
    }
}

impl Sub<&Base> for Base {
    type Output = Base;

    fn sub(self, rhs: &Base) -> Base {
        meter_limbs2(&self, rhs);
        debug_assert!(self >= *rhs, "Base subtraction underflow");
        Base(self.0 - &rhs.0)
    }
}

impl SubAssign<&Base> for Base {
    fn sub_assign(&mut self, rhs: &Base) {
        *self = self.clone() - rhs;
    }
}

impl MulAssign<u32> for Base {
    fn mul_assign(&mut self, rhs: u32) {
        meter_limbs1(self);
        self.0 *= rhs;
    }
}

impl Shl<u32> for Base {
    type Output = Base;

    fn shl(self, rhs: u32) -> Base {
        meter_limbs_shl(&self, u64::from(rhs));
        Base(self.0 << rhs as usize)
    }
}

impl Shl<i32> for Base {
    type Output = Base;

    fn shl(self, rhs: i32) -> Base {
        debug_assert!(rhs >= 0, "Base left shift must be non-negative");
        self << rhs as u32
    }
}

impl Shr<u32> for Base {
    type Output = Base;

    fn shr(self, rhs: u32) -> Base {
        meter_limbs1(&self);
        Base(self.0 >> rhs as usize)
    }
}

// The u64 shift forms serve exponent-denominated callers (a `Rank`'s
// exponent is u64). A shift amount is realizable only when the shifted
// value fits the address space, so the conversion to the backend's
// usize is checked, not truncating: an amount past usize denotes a
// value that could not be allocated anyway.

impl Shl<u64> for Base {
    type Output = Base;

    fn shl(self, rhs: u64) -> Base {
        meter_limbs_shl(&self, rhs);
        let rhs = usize::try_from(rhs).expect("shift amount fits the address space");
        Base(self.0 << rhs)
    }
}

impl Shr<u64> for Base {
    type Output = Base;

    fn shr(self, rhs: u64) -> Base {
        meter_limbs1(&self);
        let rhs = usize::try_from(rhs).expect("shift amount fits the address space");
        Base(self.0 >> rhs)
    }
}

impl BitOr<Base> for Base {
    type Output = Base;

    fn bitor(self, rhs: Base) -> Base {
        meter_limbs2(&self, &rhs);
        Base(self.0 | rhs.0)
    }
}

#[cfg(test)]
mod tests;
