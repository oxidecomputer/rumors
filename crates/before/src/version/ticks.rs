//! The count-of-ticks vocabulary: [`Ticks`], an opaque unbounded count.
//!
//! The public contract lives on the type; the operations denominated in it are
//! [`Version::ticks`](crate::Version::ticks) and its mirrors, and
//! [`Version::min_ticks`](crate::Version::min_ticks). This module is private.

use core::fmt;
use core::iter::Sum;
use core::ops::{Add, AddAssign};
use core::str::FromStr;

use crate::codec::Base;
use crate::error::{Parse, TooWide};

/// A count of [`tick`](crate::Version::tick)s: an unbounded natural
/// number.
///
/// Event counts have no ceiling, so the count vocabulary is opaque and
/// unbounded rather than any fixed-width integer: every conversion *into* it is
/// total ([`From`] on the unsigned machine integers, decimal [`FromStr`] for
/// counts wider than any of them), and every conversion *out* is explicit
/// about width — `TryFrom<&Ticks> for u64` answers the machine-range case
/// fallibly, [`limbs`](Ticks::limbs) spells any count in base-2^64 for
/// consumers with their own wide arithmetic, and [`Display`](fmt::Display)
/// renders decimal.
///
/// This type is produced by [`Version::min_ticks`](crate::Version::min_ticks);
/// and consumed by [`Version::ticks`](crate::Version::ticks),
/// [`Party::ticks`](crate::Party::ticks), and
/// [`Clock::ticks`](crate::Clock::ticks), each of which take `impl
/// Into<Ticks>`, so call sites pass integer literals directly and the type
/// appears only where a count is genuinely carried or genuinely wide.
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
/// Constructionis `O(1)`; comparison and hashing `O(‖n‖)`; addition `O(‖a‖ +
/// ‖b‖)`, `Sum` `O(N)`; text I/O is superlinear but subquadratic in the count's
/// width (because it requires decimal conversion).
///
/// Parsing ([`FromStr`]) and rendering ([`Display`](fmt::Display)) are `O(d)`
/// space in the `d` decimal digits (`d = Θ(‖n‖)`), but their time additionally
/// pays decimal↔binary conversion, so it is superlinear (though subquadratic)
/// in the count's width past a machine word.
///
/// # Example
///
/// ```
/// use before::{Clock, Ticks};
/// let mut clock = Clock::seed();
/// clock.ticks(3u64); // literals convert in
/// assert_eq!(clock.version().min_ticks(), Ticks::from(3u64));
/// // Counts wider than any machine integer parse from decimal text.
/// let wide: Ticks = "340282366920938463463374607431768211456".parse().unwrap();
/// assert_eq!(wide.to_string(), "340282366920938463463374607431768211456");
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

    /// The count's base-2^64 digits — its *limbs* — least significant
    /// first.
    ///
    /// The general way out of the type: any count, however wide, is
    /// exactly `Σ limbᵢ · 2^(64·i)` over the yielded limbs, ready to
    /// feed whatever integer arithmetic the consumer keeps. Canonical —
    /// no trailing zero limbs, and the zero count yields no limbs — and
    /// exact-size, so a consumer can preallocate. For a count within
    /// machine range, `u64::try_from(&count)` skips the limbs entirely;
    /// for text, [`Display`](fmt::Display) renders decimal directly.
    ///
    /// # Complexity
    ///
    /// Construction is `O(1)`; the full drain is `O(‖n‖)` and allocates
    /// nothing (the iterator borrows the count).
    ///
    /// # Example
    ///
    /// ```
    /// use before::Ticks;
    /// let wide: Ticks = "340282366920938463463374607431768211457".parse().unwrap();
    /// // 2^128 + 1: three limbs, least significant first.
    /// assert_eq!(wide.limbs().collect::<Vec<u64>>(), vec![1, 0, 1]);
    /// assert_eq!(Ticks::ZERO.limbs().len(), 0);
    /// ```
    pub fn limbs(&self) -> Limbs<'_> {
        Limbs {
            limbs: suanpan::Limbs::new(&self.0 .0),
            remaining: usize::try_from(self.0.bits().div_ceil(64))
                .expect("a stored count's limb count fits usize"),
        }
    }
}

/// An iterator over the base-2^64 limbs of a [`Ticks`] count, least
/// significant first; see [`Ticks::limbs`].
///
/// Exact-size and [fused](core::iter::FusedIterator); borrows the count
/// and allocates nothing.
pub struct Limbs<'a> {
    limbs: suanpan::Limbs<'a>,
    /// Limbs not yet yielded, for the exact-size contract.
    remaining: usize,
}

impl Iterator for Limbs<'_> {
    type Item = u64;

    fn next(&mut self) -> Option<u64> {
        let limb = self.limbs.next();
        if limb.is_some() {
            self.remaining -= 1;
        }
        limb
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
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
/// [`From<u64>`](Ticks#impl-From<u64>-for-Ticks), `O(1)`.
///
/// A count past the `u64` range answers [`error::TooWide`](TooWide);
/// spell such a count with [`Ticks::limbs`] or render it with
/// [`Display`](fmt::Display).
///
/// # Example
///
/// ```
/// use before::Ticks;
/// assert_eq!(u64::try_from(&Ticks::from(42u64)), Ok(42));
/// let wide: Ticks = "340282366920938463463374607431768211456".parse().unwrap();
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

/// Parses a decimal digit run, e.g. `"340282366920938463463374607431768211456"`.
///
/// Strict about shape, permissive about value: any nonempty run of ASCII
/// digits parses (leading zeros are value-preserving), anything else —
/// signs, whitespace, radix prefixes — is [`Parse::Syntax`]. There is no
/// width ceiling to reject against.
impl FromStr for Ticks {
    type Err = Parse;
    fn from_str(s: &str) -> Result<Self, Parse> {
        if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
            return Err(Parse::Syntax);
        }
        Ok(Ticks(Base::parse_decimal(s)))
    }
}

/// The count in decimal, the notation [`FromStr`] parses.
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
