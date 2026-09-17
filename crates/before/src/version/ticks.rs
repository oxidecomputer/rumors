//! The unbounded-count vocabulary: [`Ticks`].
//!
//! The public contract lives on the type. This module is private.

use core::fmt;
use core::iter::Sum;
use core::ops::{Add, AddAssign};

use crate::codec::base::Limbs as BaseLimbs;
use crate::codec::Base;
use crate::error::TooWide;

/// An unbounded natural-number count.
///
/// Tick and fork counts have no semantic ceiling, so this type is unbounded
/// rather than fixed-width. Conversions from unsigned machine integers are
/// total. Conversions out are explicit about width: `TryFrom<&Ticks> for u64`
/// answers the machine-range case fallibly, [`limbs`](Ticks::limbs) spells any
/// count in base-2^64 for consumers with their own wide arithmetic, and
/// [`Display`](fmt::Display) renders decimal.
///
/// Counts are totally ordered ([`Ord`]) and can be added ([`Add`],
/// [`AddAssign`], [`Sum`]); [`ZERO`](Ticks::ZERO) is the additive identity.
///
/// # Complexity
///
/// A count's *numeric size* `‖n‖` is its bit width; cloning costs as
/// comparison and hashing do, and an n-ary [`Sum`]'s `N` is the
/// summands' total numeric size.
///
/// Construction is `O(1)`; comparison and hashing `O(‖n‖)`; addition `O(‖a‖ +
/// ‖b‖)`, `Sum` `O(N)`. Decimal rendering is superlinear but subquadratic in
/// the count's width.
///
/// # Example
///
/// ```
/// use before::{Clock, Ticks};
/// let mut clock = Clock::seed();
/// clock.ticks(3u64); // literals convert in
/// assert_eq!(clock.version().min_ticks(), Ticks::from(3u64));
/// ```
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ticks(pub(crate) Base);

/// The zero count (same as [`Ticks::ZERO`]).
///
/// # Example
///
/// ```
/// assert_eq!(before::Ticks::default(), before::Ticks::ZERO);
/// ```
impl Default for Ticks {
    fn default() -> Self {
        Ticks::ZERO
    }
}

impl Ticks {
    /// The zero count: the empty run of ticks, and the identity for
    /// [`Ticks`] addition. Equal to
    /// [`Version::new().min_ticks()`](crate::Version::min_ticks).
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Ticks, Version};
    /// assert_eq!(Version::new().min_ticks(), Ticks::ZERO);
    /// assert_eq!(Ticks::from(7u64) + Ticks::ZERO, Ticks::from(7u64));
    /// ```
    pub const ZERO: Ticks = Ticks(Base::ZERO);

    /// The count's base-2^64 "digits", i.e. its *limbs*, least significant
    /// first, then ascending.
    ///
    /// For a count known to be within machine range, `u64::try_from(&count)`
    /// skips the limbs entirely.
    ///
    /// # Complexity
    ///
    /// Construction is `O(1)`; the full drain is `O(‖n‖)` and allocates
    /// nothing.
    ///
    /// # Example
    ///
    /// ```
    /// use before::Ticks;
    /// let wide = Ticks::from(u128::MAX) + Ticks::from(2u8);
    /// assert_eq!(wide.limbs().collect::<Vec<u64>>(), vec![1, 0, 1]);
    /// assert_eq!(Ticks::ZERO.limbs().len(), 0);
    /// ```
    pub fn limbs(&self) -> Limbs<'_> {
        Limbs {
            limbs: self.0.iter_limbs(),
        }
    }
}

/// An iterator over the base-2^64 limbs of a [`Ticks`] count, least
/// significant first; see [`Ticks::limbs`].
///
/// Exact-size and [fused](core::iter::FusedIterator).
pub struct Limbs<'a> {
    limbs: BaseLimbs<'a>,
}

impl Iterator for Limbs<'_> {
    type Item = u64;

    fn next(&mut self) -> Option<u64> {
        self.limbs.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.limbs.size_hint()
    }
}

impl ExactSizeIterator for Limbs<'_> {}

impl core::iter::FusedIterator for Limbs<'_> {}

/// A count from a machine integer: total, `O(1)`.
macro_rules! ticks_from_unsigned {
    ($($t:ty),*) => {
        $(
            impl From<$t> for Ticks {
                fn from(n: $t) -> Ticks {
                    Ticks(Base::from(u128::from(n)))
                }
            }
        )*
    };
}

ticks_from_unsigned!(u8, u16, u32, u64, u128);

/// The count as a machine word, when it fits: the narrow dual of
/// [`From<u64>`](Ticks#impl-From<u64>-for-Ticks).
///
/// A count past the `u64` range answers [`error::TooWide`](TooWide);
/// spell such a count with [`Ticks::limbs`] or render it with
/// [`Display`](fmt::Display).
///
/// # Complexity
///
/// `O(1)`.
///
/// # Example
///
/// ```
/// use before::Ticks;
/// assert_eq!(u64::try_from(&Ticks::from(42u64)), Ok(42));
/// let wide = Ticks::from(u128::MAX);
/// assert!(u64::try_from(&wide).is_err());
/// ```
impl TryFrom<&Ticks> for u64 {
    type Error = TooWide;
    fn try_from(count: &Ticks) -> Result<u64, TooWide> {
        count.0.to_u64().ok_or(TooWide)
    }
}

/// A count from a machine size: total, `O(1)`.
impl From<usize> for Ticks {
    fn from(n: usize) -> Ticks {
        Ticks(Base::from(n as u128))
    }
}

/// The count in decimal.
///
/// # Example
///
/// ```
/// assert_eq!(before::Ticks::from(42u64).to_string(), "42");
/// ```
impl fmt::Display for Ticks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// The same format as `Display`.
impl fmt::Debug for Ticks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Display>::fmt(self, f)
    }
}

impl Add<&Ticks> for &Ticks {
    type Output = Ticks;
    fn add(self, rhs: &Ticks) -> Ticks {
        Ticks(&self.0 + &rhs.0)
    }
}

impl Add<Ticks> for Ticks {
    type Output = Ticks;
    fn add(self, rhs: Ticks) -> Ticks {
        &self + &rhs
    }
}

impl Add<&Ticks> for Ticks {
    type Output = Ticks;
    fn add(self, rhs: &Ticks) -> Ticks {
        &self + rhs
    }
}

impl Add<Ticks> for &Ticks {
    type Output = Ticks;
    fn add(self, rhs: Ticks) -> Ticks {
        self + &rhs
    }
}

impl AddAssign<&Ticks> for Ticks {
    fn add_assign(&mut self, rhs: &Ticks) {
        self.0 += &rhs.0;
    }
}

impl AddAssign<Ticks> for Ticks {
    fn add_assign(&mut self, rhs: Ticks) {
        *self += &rhs;
    }
}

/// Sums the iterator's counts; the empty sum is [`Ticks::ZERO`].
impl Sum<Ticks> for Ticks {
    fn sum<I: Iterator<Item = Ticks>>(iter: I) -> Ticks {
        iter.fold(Ticks::ZERO, |mut acc, t| {
            acc += t;
            acc
        })
    }
}

/// Sums the iterator's counts; the empty sum is [`Ticks::ZERO`].
impl<'a> Sum<&'a Ticks> for Ticks {
    fn sum<I: Iterator<Item = &'a Ticks>>(iter: I) -> Ticks {
        iter.fold(Ticks::ZERO, |mut acc, t| {
            acc += t;
            acc
        })
    }
}

#[cfg(test)]
mod tests;
