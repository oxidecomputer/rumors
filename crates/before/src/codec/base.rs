use core::cmp::Ordering;
use core::fmt;
use core::ops::{Add, AddAssign, BitOr, MulAssign, Shl, Shr, Sub, SubAssign};

use num_bigint::BigUint;

/// Process-global counter of big-integer limb-scale work.
///
/// Arithmetic-width cost is invisible to every other meter: a magnitude
/// blowup performs no extra allocations a peak-heap meter would see and
/// visits no extra nodes a step counter would see — the work is wider, not
/// more frequent. The proxy counted here is the operands' 64-bit limb counts
/// per arithmetic operation (every operation below records before it runs,
/// and the wide-gamma accumulation in `codec::gamma` records each step), so
/// amortized-linear algorithms count linearly in packed input bits and
/// magnitude-quadratic ones count quadratically. Relaxed ordering suffices:
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

    /// Record one accumulation step on a raw `BigUint` working value.
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

/// An event tree's stored integer magnitude.
///
/// ITC event counts (path sums of `tick`s, the `max`/`join` of two such sums)
/// grow without bound, so the value type preserves arbitrary precision: no
/// `u64` overflow class, in any build profile. The common case stays inline as
/// a `u64`; only values past `u64::MAX` spill to `BigUint`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
        match (self, rhs) {
            (Base::Small(a), Base::Small(b)) => a
                .checked_add(*b)
                .map(Base::Small)
                .unwrap_or_else(|| Base::Big(BigUint::from(*a) + BigUint::from(*b))),
            _ => Base::from_big(self.to_biguint() + rhs.to_biguint()),
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
        *self = &*self + rhs;
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
