use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::ops::{Add, AddAssign, BitOr, MulAssign, Shl, Shr, Sub, SubAssign};

use dashu_int::ops::BitTest;
use dashu_int::{UBig, Word};
use suanpan::Accumulator;

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

/// Stored words per 64-bit limb on the current target.
const WORDS_PER_LIMB: usize = (u64::BITS / Word::BITS) as usize;

/// The stored magnitude as borrowed little-endian 64-bit limbs.
pub(crate) struct Limbs<'a> {
    chunks: core::slice::Chunks<'a, Word>,
}

impl<'a> Limbs<'a> {
    /// Borrow the limbs of `value` without allocating.
    pub(crate) fn new(value: &'a UBig) -> Limbs<'a> {
        Limbs {
            chunks: value.as_words().chunks(WORDS_PER_LIMB),
        }
    }
}

impl Iterator for Limbs<'_> {
    type Item = u64;

    fn next(&mut self) -> Option<u64> {
        self.chunks.next().map(pack_limb)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.chunks.size_hint()
    }
}

impl DoubleEndedIterator for Limbs<'_> {
    fn next_back(&mut self) -> Option<u64> {
        self.chunks.next_back().map(pack_limb)
    }
}

impl ExactSizeIterator for Limbs<'_> {}

impl core::iter::FusedIterator for Limbs<'_> {}

/// Combine one target-word chunk into a 64-bit limb.
fn pack_limb(chunk: &[Word]) -> u64 {
    #[allow(clippy::unnecessary_cast)]
    chunk.iter().enumerate().fold(0u64, |limb, (index, &word)| {
        limb | ((word as u64) << (index as u32 * Word::BITS))
    })
}

impl Base {
    pub(crate) const ZERO: Base = Base(UBig::ZERO);

    /// Whether this magnitude is zero.
    pub(crate) fn is_zero(&self) -> bool {
        self.0 == UBig::ZERO
    }

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

    /// Borrow this magnitude as minimal little-endian 64-bit limbs.
    pub(crate) fn iter_limbs(&self) -> Limbs<'_> {
        Limbs::new(&self.0)
    }

    /// Build a magnitude from a borrowed little-endian limb slice.
    fn from_limb_slice(limbs: &[u64]) -> Base {
        #[cfg(target_pointer_width = "64")]
        {
            match limbs {
                [] => return Base::ZERO,
                &[low] => return Base(UBig::from(low)),
                &[low, high] => {
                    return Base(UBig::from(u128::from(low) | (u128::from(high) << 64)));
                }
                _ => {}
            }
            Base(UBig::from_words(limbs))
        }
        #[cfg(target_pointer_width = "32")]
        {
            match limbs {
                [] => return Base::ZERO,
                &[limb] => return Base(UBig::from(limb)),
                _ => {}
            }
            let words: Vec<Word> = limbs
                .iter()
                .flat_map(|&limb| [limb as Word, (limb >> 32) as Word])
                .collect();
            Base(UBig::from_words(&words))
        }
    }

    /// Read an accumulator into this normalized magnitude representation.
    pub(crate) fn from_accumulator(acc: &Accumulator) -> (Ordering, Base) {
        acc.with_sign_limbs(|sign, limbs| (sign, Base::from_limb_slice(limbs)))
    }

    /// Read an accumulator as a magnitude with a retained power-of-two scale.
    pub(crate) fn from_accumulator_shl(acc: &Accumulator) -> (Ordering, Base, u64) {
        acc.with_sign_limbs_shl(|sign, limbs, shift| (sign, Base::from_limb_slice(limbs), shift))
    }

    /// Fold this magnitude, scaled by `2^shift`, into an accumulator.
    pub(crate) fn fold_into(&self, acc: &mut Accumulator, shift: u64, subtract: bool) {
        match self.to_u64() {
            Some(word) if subtract => acc.sub_u64_shl(word, shift),
            Some(word) => acc.add_u64_shl(word, shift),
            None if subtract => acc.sub_limbs_shl(self.iter_limbs(), shift),
            None => acc.add_limbs_shl(self.iter_limbs(), shift),
        }
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

    /// Compare two magnitudes as MSB-aligned bit strings: the order of
    /// `a · 2^x` versus `b · 2^y` whenever the two values share a magnitude
    /// class (`bits(a) − x == bits(b) − y`).
    ///
    /// The stored-magnitude instance of [`msb_cmp_windows`], which carries
    /// the streaming argument, the tail rule's normalization premise, and
    /// the per-window metering.
    pub(crate) fn msb_cmp(a: &Base, b: &Base) -> Ordering {
        msb_cmp_windows(a.msb_windows(), b.msb_windows())
    }

    /// The MSB-first 64-bit windows of this magnitude's bit string, for
    /// [`msb_cmp_windows`].
    pub(crate) fn msb_windows(&self) -> MsbWindows<impl Iterator<Item = u64> + '_> {
        MsbWindows::new(self.iter_limbs().rev(), self.bits())
    }

    #[cfg(test)]
    pub(crate) fn to_bytes_le(&self) -> Vec<u8> {
        self.0.to_le_bytes().into_vec()
    }

    /// The magnitude's minimal big-endian bytes: empty for zero, no leading
    /// zero byte otherwise.
    ///
    /// The materialization dual of [`from_be_bytes`](Self::from_be_bytes),
    /// for byte-assembled values (the rank decoder concatenates an integral's
    /// bytes with fraction groups instead of shifting by an exponent a 32-bit
    /// `usize` cannot hold). Width-scale work, so the limb meter records the
    /// operand's width.
    pub(crate) fn to_be_bytes(&self) -> Vec<u8> {
        meter_limbs_solo(self);
        self.0.to_be_bytes().into_vec()
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

/// Compare two MSB-aligned window streams ([`MsbWindows`]): the shared
/// kernel behind [`Base::msb_cmp`] and the rank numerator's cross-arm
/// class-tie comparison.
///
/// Streams 64-bit windows most-significant-first — no alignment shift is
/// ever materialized — and stops at the first differing window, so the
/// cost is O(shared-prefix limbs) with zero allocation. When every shared
/// window agrees, the longer bit string is the larger value: this rides on
/// the caller's normalization invariant that the strings end in a set bit
/// (an odd numerator), so the longer string's extension is nonzero. The
/// limb meter records one limb per streamed window pair, keeping the
/// limb meter records one limb per streamed window pair, matching the work of
/// the scan.
pub(crate) fn msb_cmp_windows(
    mut a: impl Iterator<Item = u64>,
    mut b: impl Iterator<Item = u64>,
) -> Ordering {
    loop {
        match (a.next(), b.next()) {
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

/// The 64-bit windows of a magnitude's bit string, most-significant first.
///
/// The first window is the value's top 64 bits left-aligned (the MSB in
/// bit 63); the last is zero-padded below the final significant bit. A
/// zero value has no windows. Streams the stored limbs top-down with one
/// register of carry, so a window costs O(1) and no shifted copy of the
/// value ever exists. Generic over the reversed limb source so both
/// numerator arms (the stored magnitude here, the rank's wide limb vector)
/// stream through one implementation.
pub(crate) struct MsbWindows<I> {
    /// Remaining limbs, top first; exhausted once the tail is consumed.
    limbs: I,
    /// The previously consumed limb, still owed its low bits.
    held: Option<u64>,
    /// The left-alignment shift: `64 − (bits mod 64)`, zero for a
    /// limb-aligned width.
    shift: u32,
}

impl<I: Iterator<Item = u64>> MsbWindows<I> {
    /// The windows over `limbs` — the value's 64-bit limbs, **already
    /// reversed** (most significant first) — for a value `bits` wide.
    pub(crate) fn new(limbs: I, bits: u64) -> Self {
        MsbWindows {
            limbs,
            held: None,
            shift: ((64 - bits % 64) % 64) as u32,
        }
    }
}

impl<I: Iterator<Item = u64>> Iterator for MsbWindows<I> {
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
// exponent is u64). The two directions part on totality. A left shift's
// checked conversion fails only for amounts at or past usize bits: never
// on 64-bit targets (the shifted value would dwarf the address space
// first), and on 32-bit targets only where the shifted result exceeds
// the backend's representable width anyway (its buffer caps at
// usize::MAX / word-bits words), so the expect and the backend's own
// capacity assert bound the same values — results the dependency cannot
// hold, failing loudly by name instead of wrapping. A right shift is
// total: an amount at or past the value's width yields zero, and on a
// 32-bit target an amount past usize can only name that case (the
// value's width is capped below usize::MAX bits by the same backend
// bound), so the conversion clamps, value-preserving.

impl Shl<u64> for Base {
    type Output = Base;

    fn shl(self, rhs: u64) -> Base {
        meter_limbs_shl(&self, rhs);
        let rhs = usize::try_from(rhs)
            .expect("a left shift this wide exceeds the backend's representable width");
        Base(self.0 << rhs)
    }
}

impl Shr<u64> for Base {
    type Output = Base;

    fn shr(self, rhs: u64) -> Base {
        meter_limbs1(&self);
        // The clamp is exact, never a truncation: any amount at or past the
        // value's width — everything past usize included — yields zero.
        let rhs = usize::try_from(rhs).unwrap_or(usize::MAX);
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
