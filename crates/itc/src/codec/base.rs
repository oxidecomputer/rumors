use core::cmp::Ordering;
use core::fmt;
use core::ops::{Add, AddAssign, BitOr, MulAssign, Shl, Sub, SubAssign};

use num_bigint::BigUint;

/// An event tree's stored integer magnitude. ITC event counts (path sums of `tick`s,
/// the `max`/`join` of two such sums) grow without bound, so the value type preserves
/// arbitrary precision: no `u64` overflow class, in any build profile. The common case
/// stays inline as a `u64`; only values past `u64::MAX` spill to `BigUint`.
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

    pub(crate) fn bit(&self, i: u64) -> bool {
        match self {
            Base::Small(n) => i < u64::BITS as u64 && (n & (1u64 << i)) != 0,
            Base::Big(n) => n.bit(i),
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

impl Ord for Base {
    fn cmp(&self, other: &Self) -> Ordering {
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

impl BitOr<Base> for Base {
    type Output = Base;

    fn bitor(self, rhs: Base) -> Base {
        match (self, rhs) {
            (Base::Small(a), Base::Small(b)) => Base::Small(a | b),
            (a, b) => Base::from_big(a.to_biguint() | b.to_biguint()),
        }
    }
}
