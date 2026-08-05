//! The count-of-ticks vocabulary: [`Ticks`], an opaque unbounded count.
//!
//! The public contract lives on the type; the operations denominated in
//! it are [`Version::ticks`](crate::Version::ticks) and its mirrors, and
//! [`Version::min_ticks`](crate::Version::min_ticks). This module is
//! private.

use core::fmt;
use core::iter::Sum;
use core::ops::{Add, AddAssign};
use core::str::FromStr;

use crate::codec::Base;
use crate::error::Parse;

/// A count of [`tick`](crate::Version::tick)s: an unbounded natural
/// number.
///
/// Event counts have no ceiling — heights in a [`Version`](crate::Version)
/// are arbitrary-precision, and [`ticks`](crate::Version::ticks) can skip
/// forward by any count in one call — so the count vocabulary is opaque
/// and unbounded rather than any fixed-width integer: every conversion
/// *into* it is total ([`From`] on the unsigned machine integers, decimal
/// [`FromStr`] for counts wider than any of them), and no accessor
/// converts back out to a machine integer, because any such accessor
/// would be partial in exactly the range this type exists to carry.
/// Render a count with [`Display`](fmt::Display) (decimal) instead.
///
/// Produced by [`Version::min_ticks`](crate::Version::min_ticks);
/// consumed by [`Version::ticks`](crate::Version::ticks),
/// [`Party::ticks`](crate::Party::ticks), and
/// [`Clock::ticks`](crate::Clock::ticks) — all of which take
/// `impl Into<Ticks>`, so call sites pass integer literals directly and
/// the type appears only where a count is genuinely carried or genuinely
/// wide.
///
/// Counts are totally ordered ([`Ord`]) and add ([`Add`], [`AddAssign`],
/// [`Sum`]) as the naturals they are; [`ZERO`](Ticks::ZERO) is the empty
/// run and the additive identity.
///
/// # Complexity
///
/// Construction `O(1)`; comparison and hashing `O(‖n‖)`; addition `O(‖a‖ +
/// ‖b‖)`, `Sum` `O(N)`; text superlinear in the count's width (decimal
/// conversion).
/// A count's *numeric size* `‖n‖` is its bit width; cloning costs as
/// comparison and hashing do, and an n-ary [`Sum`]'s `N` is the
/// summands' total numeric size.
/// Parsing ([`FromStr`]) and rendering ([`Display`](fmt::Display)) are
/// `O(d)` space in the `d` decimal digits, but their time additionally
/// pays decimal↔binary conversion, superlinear (though subquadratic) in
/// the count's width past a machine word.
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
    /// ```
    /// use before::{Ticks, Version};
    /// assert_eq!(Version::new().min_ticks(), Ticks::ZERO);
    /// assert_eq!(Ticks::from(7u64) + Ticks::ZERO, Ticks::from(7u64));
    /// ```
    pub const ZERO: Ticks = Ticks(Base::ZERO);
}

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
