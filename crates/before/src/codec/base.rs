use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::ops::{Add, AddAssign, BitOr, MulAssign, Shl, Shr, Sub, SubAssign};

use num_bigint::BigUint;

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

    /// Record the limb width of a raw `BigUint` working value.
    pub(crate) fn record_biguint(n: &num_bigint::BigUint) {
        record(n.bits().div_ceil(64).max(1));
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
/// `u64` overflow class, in any build profile. The common case stays inline as
/// a `u64`; only values past `u64::MAX` spill to `BigUint`.
// `PartialEq` and `Hash` are manual (below) so the limb meter sees
// width-scale equality and hashing work; both keep exactly the structural
// semantics a derive would generate.
#[derive(Clone, Debug, Eq)]
pub enum Base {
    Small(u64),
    Big(BigUint),
}

impl Base {
    pub(crate) const ZERO: Base = Base::Small(0);

    fn from_big(n: BigUint) -> Base {
        if n.bits() <= u64::BITS as u64 {
            Base::Small(n.to_u64_digits().first().copied().unwrap_or(0))
        } else {
            Base::Big(n)
        }
    }

    fn to_biguint(&self) -> BigUint {
        match self {
            Base::Small(n) => BigUint::from(*n),
            Base::Big(n) => n.clone(),
        }
    }

    pub(crate) fn bits(&self) -> u64 {
        match self {
            Base::Small(0) => 0,
            Base::Small(n) => u64::BITS as u64 - n.leading_zeros() as u64,
            Base::Big(n) => n.bits(),
        }
    }

    /// This magnitude as a `u64`, saturating at [`u64::MAX`] for values past the
    /// inline range (those that have spilled to `BigUint`).
    pub(crate) fn to_u64_saturating(&self) -> u64 {
        match self {
            Base::Small(n) => *n,
            Base::Big(_) => u64::MAX,
        }
    }

    pub(crate) fn bit(&self, i: u64) -> bool {
        match self {
            Base::Small(n) => i < u64::BITS as u64 && (n & (1u64 << i)) != 0,
            Base::Big(n) => n.bit(i),
        }
    }

    /// The number of trailing zero bits, or `None` for zero (which has no
    /// lowest set bit). Used by [`Rank`](crate::Rank) normalization to strip
    /// factors of two out of a dyadic numerator.
    pub(crate) fn trailing_zeros(&self) -> Option<u64> {
        match self {
            Base::Small(0) => None,
            Base::Small(n) => Some(u64::from(n.trailing_zeros())),
            Base::Big(n) => n.trailing_zeros(),
        }
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
        match self {
            Base::Small(0) => Vec::new(),
            Base::Small(n) => {
                n.to_le_bytes()[..n.to_le_bytes().len() - (n.leading_zeros() as usize / 8)].to_vec()
            }
            Base::Big(n) => n.to_bytes_le(),
        }
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
    /// Remaining limbs, top first; `None` once the inline arm or the tail
    /// is consumed.
    limbs: MsbLimbs<'a>,
    /// The previously consumed limb, still owed its low bits.
    held: Option<u64>,
    /// The left-alignment shift: `64 − (bits mod 64)`, zero for a
    /// limb-aligned width.
    shift: u32,
}

/// The limb stream behind [`MsbWindows`]: one inline word, or a spilled
/// value's limbs reversed.
enum MsbLimbs<'a> {
    Small(Option<u64>),
    Big(core::iter::Rev<num_bigint::U64Digits<'a>>),
}

impl<'a> MsbLimbs<'a> {
    fn next(&mut self) -> Option<u64> {
        match self {
            MsbLimbs::Small(word) => word.take(),
            MsbLimbs::Big(rev) => rev.next(),
        }
    }
}

impl<'a> MsbWindows<'a> {
    fn new(value: &'a Base) -> Self {
        let bits = value.bits();
        let limbs = match value {
            Base::Small(0) => MsbLimbs::Small(None),
            Base::Small(n) => MsbLimbs::Small(Some(*n)),
            Base::Big(n) => MsbLimbs::Big(n.iter_u64_digits().rev()),
        };
        MsbWindows {
            limbs,
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

// Structural equality, identical to the derived semantics: variants must
// match and their payloads must be equal. Canonical form keeps every value
// in exactly one variant (`Big` only past `u64::MAX`), so cross-variant
// operands are distinct values, never two spellings of one value. Manual
// only so the limb meter records the operand widths: equality over spilled
// magnitudes is width-scale work (the decoder's equal-leaf check, the
// builder's collapse check) that every other meter is blind to.
impl PartialEq for Base {
    fn eq(&self, other: &Self) -> bool {
        meter_limbs2(self, other);
        match (self, other) {
            (Base::Small(a), Base::Small(b)) => a == b,
            (Base::Big(a), Base::Big(b)) => a == b,
            _ => false,
        }
    }
}

// The derived stream: the discriminant, then the payload. Manual only so
// the limb meter records the operand width (hashing walks every limb).
// Consistent with `PartialEq` above: equal values are structurally
// identical, so they feed identical streams to the hasher.
impl Hash for Base {
    fn hash<H: Hasher>(&self, state: &mut H) {
        meter_limbs_solo(self);
        core::mem::discriminant(self).hash(state);
        match self {
            Base::Small(n) => n.hash(state),
            Base::Big(n) => n.hash(state),
        }
    }
}

impl Ord for Base {
    fn cmp(&self, other: &Self) -> Ordering {
        meter_limbs2(self, other);
        match (self, other) {
            (Base::Small(a), Base::Small(b)) => a.cmp(b),
            (Base::Small(_), Base::Big(_)) => Ordering::Less,
            (Base::Big(_), Base::Small(_)) => Ordering::Greater,
            (Base::Big(a), Base::Big(b)) => a.cmp(b),
        }
    }
}

impl PartialOrd for Base {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Base {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Base::Small(n) => fmt::Display::fmt(n, f),
            Base::Big(n) => fmt::Display::fmt(n, f),
        }
    }
}

impl From<BigUint> for Base {
    fn from(n: BigUint) -> Self {
        Base::from_big(n)
    }
}

impl From<u8> for Base {
    fn from(n: u8) -> Self {
        Base::Small(u64::from(n))
    }
}

impl From<u32> for Base {
    fn from(n: u32) -> Self {
        Base::Small(u64::from(n))
    }
}

impl From<u64> for Base {
    fn from(n: u64) -> Self {
        Base::Small(n)
    }
}

impl From<u128> for Base {
    fn from(n: u128) -> Self {
        if let Ok(n) = u64::try_from(n) {
            Base::Small(n)
        } else {
            Base::Big(BigUint::from(n))
        }
    }
}

impl Add<&Base> for &Base {
    type Output = Base;

    fn add(self, rhs: &Base) -> Base {
        meter_limbs2(self, rhs);
        // Every spilled arm allocates exactly the result: num-bigint adds a
        // machine scalar or a borrowed `BigUint` to a borrowed `BigUint`
        // without cloning either operand.
        match (self, rhs) {
            (Base::Small(a), Base::Small(b)) => a
                .checked_add(*b)
                .map(Base::Small)
                .unwrap_or_else(|| Base::Big(BigUint::from(*a) + BigUint::from(*b))),
            (Base::Small(a), Base::Big(b)) => Base::from_big(b + *a),
            (Base::Big(a), Base::Small(b)) => Base::from_big(a + *b),
            (Base::Big(a), Base::Big(b)) => Base::from_big(a + b),
        }
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
        match self {
            Base::Small(n) => n
                .checked_add(u64::from(rhs))
                .map(Base::Small)
                .unwrap_or_else(|| Base::Big(BigUint::from(n) + rhs)),
            Base::Big(n) => Base::from_big(n + rhs),
        }
    }
}

impl Add<u32> for &Base {
    type Output = Base;

    fn add(self, rhs: u32) -> Base {
        self.clone() + rhs
    }
}

impl Add<u64> for Base {
    type Output = Base;

    fn add(self, rhs: u64) -> Base {
        meter_limbs1(&self);
        match self {
            Base::Small(n) => n
                .checked_add(rhs)
                .map(Base::Small)
                .unwrap_or_else(|| Base::Big(BigUint::from(n) + rhs)),
            Base::Big(n) => Base::from_big(n + rhs),
        }
    }
}

impl Add<u64> for &Base {
    type Output = Base;

    fn add(self, rhs: u64) -> Base {
        self.clone() + rhs
    }
}

impl AddAssign<&Base> for Base {
    fn add_assign(&mut self, rhs: &Base) {
        meter_limbs2(self, rhs);
        match (&mut *self, rhs) {
            (Base::Small(a), Base::Small(b)) => {
                if let Some(n) = a.checked_add(*b) {
                    *a = n;
                } else {
                    *self = Base::Big(BigUint::from(*a) + *b);
                }
            }
            (Base::Small(a), Base::Big(b)) => *self = Base::from_big(b + *a),
            // A `Big` is canonically past the `u64` range and addition only
            // grows it, so accumulating in place cannot need a demotion.
            (Base::Big(a), Base::Small(b)) => *a += *b,
            (Base::Big(a), Base::Big(b)) => *a += b,
        }
    }
}

impl AddAssign<u32> for Base {
    fn add_assign(&mut self, rhs: u32) {
        *self = self.clone() + rhs;
    }
}

impl Sub<&Base> for Base {
    type Output = Base;

    fn sub(self, rhs: &Base) -> Base {
        meter_limbs2(&self, rhs);
        debug_assert!(self >= *rhs, "Base subtraction underflow");
        match (&self, rhs) {
            (Base::Small(a), Base::Small(b)) => Base::Small(a - b),
            _ => Base::from_big(self.to_biguint() - rhs.to_biguint()),
        }
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
        *self = match self {
            Base::Small(n) => n
                .checked_mul(u64::from(rhs))
                .map(Base::Small)
                .unwrap_or_else(|| Base::Big(BigUint::from(*n) * rhs)),
            Base::Big(n) => Base::from_big(n.clone() * rhs),
        };
    }
}

impl Shl<u32> for Base {
    type Output = Base;

    fn shl(self, rhs: u32) -> Base {
        meter_limbs1(&self);
        match self {
            Base::Small(n) if rhs < u64::BITS && n <= (u64::MAX >> rhs) => Base::Small(n << rhs),
            Base::Small(n) => Base::from_big(BigUint::from(n) << rhs),
            Base::Big(n) => Base::from_big(n << rhs),
        }
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
        match self {
            Base::Small(n) if rhs < u64::BITS => Base::Small(n >> rhs),
            Base::Small(_) => Base::Small(0),
            Base::Big(n) => Base::from_big(n >> rhs),
        }
    }
}

impl BitOr<Base> for Base {
    type Output = Base;

    fn bitor(self, rhs: Base) -> Base {
        meter_limbs2(&self, &rhs);
        match (self, rhs) {
            (Base::Small(a), Base::Small(b)) => Base::Small(a | b),
            (a, b) => Base::from_big(a.to_biguint() | b.to_biguint()),
        }
    }
}
