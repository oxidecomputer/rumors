use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::ops::{Add, AddAssign, BitOr, MulAssign, Shl, Shr, Sub, SubAssign};

use dashu_int::ops::BitTest;
use dashu_int::{UBig, Word};

/// Process-global counter of big-integer limb-scale work.
///
/// Arithmetic-width cost is invisible to every other meter: a magnitude
/// blowup performs no extra allocations a peak-heap meter would see and
/// visits no extra nodes a step counter would see — the work is wider, not
/// more frequent. The proxy counted here is the operands' 64-bit limb counts
/// per `Base` operation — arithmetic, comparison, equality, and hashing all
/// record before they run, and the wide-gamma decode in `codec::gamma`
/// records one value-width count per decoded value — so amortized-linear
/// algorithms count linearly in
/// packed input bits and magnitude-quadratic ones count quadratically.
/// Relaxed ordering suffices:
/// the metering binaries run one scenario per process and read the counter
/// only after the metered call returns.
#[cfg(feature = "limb-meter")]
pub(crate) mod limb_meter {
    use core::sync::atomic::{AtomicU64, Ordering};

    static LIMB_OPS: AtomicU64 = AtomicU64::new(0);

    /// Add `n` operand limbs to the counter.
    pub(crate) fn record(n: u64) {
        LIMB_OPS.fetch_add(n, Ordering::Relaxed);
    }

    /// Record the limb width of a raw `UBig` working value.
    pub(crate) fn record_wide(n: &dashu_int::UBig) {
        use dashu_int::ops::BitTest;
        record((n.bit_len() as u64).div_ceil(64).max(1));
    }

    /// The limb operations recorded since the last [`reset`].
    pub(crate) fn limb_ops() -> u64 {
        LIMB_OPS.load(Ordering::Relaxed)
    }

    /// Reset the counter to zero.
    pub(crate) fn reset() {
        LIMB_OPS.store(0, Ordering::Relaxed);
    }
}

/// Record a two-`Base` arithmetic operation's limb-scale work.
///
/// Compiles to nothing without the `limb-meter` feature, so every operation
/// below can call it unconditionally.
#[inline(always)]
fn meter_limbs2(a: &Base, b: &Base) {
    #[cfg(feature = "limb-meter")]
    limb_meter::record(a.limbs() + b.limbs());
    #[cfg(not(feature = "limb-meter"))]
    let _ = (a, b);
}

/// Record a `Base`-with-machine-scalar operation's limb-scale work (the
/// scalar counts as one limb).
///
/// Compiles to nothing without the `limb-meter` feature.
#[inline(always)]
fn meter_limbs1(a: &Base) {
    #[cfg(feature = "limb-meter")]
    limb_meter::record(a.limbs() + 1);
    #[cfg(not(feature = "limb-meter"))]
    let _ = a;
}

/// Record a single-operand `Base` operation's limb-scale work (hashing
/// walks every limb of its one operand).
///
/// Compiles to nothing without the `limb-meter` feature.
#[inline(always)]
fn meter_limbs_solo(a: &Base) {
    #[cfg(feature = "limb-meter")]
    limb_meter::record(a.limbs());
    #[cfg(not(feature = "limb-meter"))]
    let _ = a;
}

/// An event tree's stored integer magnitude.
///
/// ITC event counts (path sums of `tick`s, the `max`/`join` of two such sums)
/// grow without bound, so the value type preserves arbitrary precision: no
/// `u64` overflow class, in any build profile. A thin metered wrapper around
/// [`UBig`]: every operation records its operands' 64-bit limb widths into
/// the limb meter, then delegates the arithmetic whole. Values up to two
/// machine words stay inline in the wrapped representation, so the common
/// small magnitudes never allocate.
// `PartialEq` and `Hash` are manual (below) so the limb meter sees
// width-scale equality and hashing work; both keep exactly the structural
// semantics a derive would generate.
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

    /// This magnitude as a `u64`, saturating at [`u64::MAX`] for values
    /// past the `u64` range.
    pub(crate) fn to_u64_saturating(&self) -> u64 {
        self.to_u64().unwrap_or(u64::MAX)
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
}

/// Stored words per 64-bit limb: 1 where the word is 64 bits, 2 where it
/// is 32 (wasm32).
///
/// Every limb-denominated cost — the limb meter's operand widths,
/// [`MsbWindows`]'s streamed windows, the accumulator's per-limb wide-delta
/// pricing — is counted in 64-bit limbs, so pairing narrower storage words
/// keeps the measured numbers identical across targets.
pub(crate) const WORDS_PER_LIMB: usize = (u64::BITS / Word::BITS) as usize;

/// The 64-bit limbs of a magnitude, least-significant first.
///
/// Borrows the stored word slice and packs [`WORDS_PER_LIMB`] words per
/// limb, so iteration allocates nothing; the top limb zero-pads any missing
/// high words. A zero value has no limbs. Double-ended, so MSB-first
/// consumers reverse it.
pub(crate) struct U64Limbs<'a> {
    chunks: core::slice::Chunks<'a, Word>,
}

impl<'a> U64Limbs<'a> {
    pub(crate) fn new(value: &'a UBig) -> Self {
        U64Limbs {
            chunks: value.as_words().chunks(WORDS_PER_LIMB),
        }
    }
}

/// Pack one limb's worth of stored words (the top chunk may be partial).
fn pack_limb(chunk: &[Word]) -> u64 {
    // One face of this cast is a no-op: `Word` is `u64` on 64-bit targets
    // and `u32` on 32-bit ones, and the cast is what compiles on both.
    #[allow(clippy::unnecessary_cast)]
    chunk.iter().enumerate().fold(0u64, |limb, (i, &word)| {
        limb | ((word as u64) << (i as u32 * Word::BITS))
    })
}

impl Iterator for U64Limbs<'_> {
    type Item = u64;

    fn next(&mut self) -> Option<u64> {
        self.chunks.next().map(pack_limb)
    }
}

impl DoubleEndedIterator for U64Limbs<'_> {
    fn next_back(&mut self) -> Option<u64> {
        self.chunks.next_back().map(pack_limb)
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
    limbs: core::iter::Rev<U64Limbs<'a>>,
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
            limbs: U64Limbs::new(&value.0).rev(),
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
        meter_limbs1(&self);
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

impl BitOr<Base> for Base {
    type Output = Base;

    fn bitor(self, rhs: Base) -> Base {
        meter_limbs2(&self, &rhs);
        Base(self.0 | rhs.0)
    }
}
