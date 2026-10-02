//! Arbitrary-precision nonnegative counts.
//!
//! The public contract lives on the type. This module is private.

use core::fmt;
use core::iter::Sum;
use core::ops::{Add, AddAssign};

use num_bigint::{BigUint, U64Digits};

use crate::error::{ParseValue, TooWide};

/// An unbounded natural-number count.
///
/// Counts have no semantic ceiling, so this type is unbounded rather than
/// fixed-width. Conversions from unsigned machine integers are total. Owned
/// and borrowed conversions out to every unsigned machine integer return
/// [`TooWide`] when the count does not fit. [`limbs`](Count::limbs) spells any
/// count in base-2^64 for consumers with their own wide arithmetic, and
/// [`Display`](fmt::Display) and [`FromStr`](core::str::FromStr) use canonical
/// unsigned decimal.
///
/// Counts are totally ordered ([`Ord`]), can be added ([`Add`], [`AddAssign`],
/// [`Sum`]), and support checked or saturating subtraction. [`ZERO`](Count::ZERO)
/// is the additive identity.
///
/// # Complexity
///
/// A count's *numeric size* `‖n‖` is its bit width. Cloning, comparison, and
/// hashing cost `O(‖n‖)`. For a [`Sum`] of `k` counts, `N` is their total
/// numeric size.
///
/// Construction and conversion to a fixed-width integer are `O(1)`; comparison
/// and hashing `O(‖n‖)`; addition `O(‖a‖ + ‖b‖)`, subtraction `O(‖a‖ + ‖b‖)`,
/// and `Sum` `O(k + N)`. The `k` term accounts for visiting zero counts, whose
/// numeric size is zero. Decimal rendering is superlinear but subquadratic in
/// the count's width.
///
/// # Example
///
/// ```
/// use before::{Clock, Count};
/// let mut clock = Clock::seed();
/// clock.ticks(3u64); // literals convert in
/// assert_eq!(clock.version().min_ticks(), Count::from(3u64));
/// ```
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscape-assets.html")))]
pub struct Count(pub(crate) BigUint);

/// The zero count (same as [`Count::ZERO`]).
///
/// # Example
///
/// ```
/// assert_eq!(before::Count::default(), before::Count::ZERO);
/// ```
impl Default for Count {
    fn default() -> Self {
        Count::ZERO
    }
}

impl Count {
    /// The zero count: the empty run of ticks, and the identity for
    /// [`Count`] addition. Equal to
    /// [`Version::new().min_ticks()`](crate::Version::min_ticks).
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Count, Version};
    /// assert_eq!(Version::new().min_ticks(), Count::ZERO);
    /// assert_eq!(Count::from(7u64) + Count::ZERO, Count::from(7u64));
    /// ```
    pub const ZERO: Count = Count(BigUint::ZERO);

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
    /// use before::Count;
    /// let wide = Count::from(u128::MAX) + Count::from(2u8);
    /// assert_eq!(wide.limbs().collect::<Vec<u64>>(), vec![1, 0, 1]);
    /// assert_eq!(Count::ZERO.limbs().len(), 0);
    /// ```
    pub fn limbs(&self) -> Limbs<'_> {
        Limbs {
            limbs: self.0.iter_u64_digits(),
        }
    }

    /// The difference `self - rhs`, or [`None`] when `rhs` exceeds `self`.
    ///
    /// # Complexity
    ///
    /// `O(‖self‖ + ‖rhs‖)` time and output-proportionate space. An underflow
    /// allocates nothing.
    ///
    /// # Example
    ///
    /// ```
    /// use before::Count;
    /// let five = Count::from(5u8);
    /// assert_eq!(five.checked_sub(&Count::from(3u8)), Some(Count::from(2u8)));
    /// assert_eq!(five.checked_sub(&Count::from(6u8)), None);
    /// ```
    #[must_use = "`Count::checked_sub` does not modify `self` or `rhs`; discarding its result means that it has no effect"]
    pub fn checked_sub(&self, rhs: &Count) -> Option<Count> {
        (self >= rhs).then(|| Count(&self.0 - &rhs.0))
    }

    /// The difference `self - rhs`, floored at [`Count::ZERO`].
    ///
    /// # Complexity
    ///
    /// `O(‖self‖ + ‖rhs‖)` time and output-proportionate space. A floored
    /// result allocates nothing.
    ///
    /// # Example
    ///
    /// ```
    /// use before::Count;
    /// let three = Count::from(3u8);
    /// assert_eq!(three.saturating_sub(&Count::from(5u8)), Count::ZERO);
    /// ```
    #[must_use = "`Count::saturating_sub` does not modify `self` or `rhs`; discarding its result means that it has no effect"]
    pub fn saturating_sub(&self, rhs: &Count) -> Count {
        self.checked_sub(rhs).unwrap_or(Count::ZERO)
    }
}

/// Builds a count directly while a serializer yields canonical `u64` limbs.
///
/// The decoder-facing form avoids retaining both the serialized limbs and the
/// integer's backing digits. [`finish`](CanonicalLimbs::finish) enforces the
/// one canonicality rule: a nonempty sequence may not end in zero.
#[cfg(any(feature = "serde", feature = "borsh"))]
pub(crate) struct CanonicalLimbs {
    /// Base-2^32 digits accepted directly by [`BigUint`].
    digits: Vec<u32>,
    /// The most recently decoded limb, if any.
    last: Option<u64>,
}

#[cfg(any(feature = "serde", feature = "borsh"))]
impl CanonicalLimbs {
    /// Begin an empty limb sequence.
    pub(crate) fn new() -> CanonicalLimbs {
        CanonicalLimbs {
            digits: Vec::new(),
            last: None,
        }
    }

    /// Append one least-significant-first `u64` limb.
    pub(crate) fn push(&mut self, limb: u64) {
        self.digits.push(limb as u32);
        self.digits.push((limb >> 32) as u32);
        self.last = Some(limb);
    }

    /// Return the decoded count, or [`None`] for a redundant high zero limb.
    pub(crate) fn finish(self) -> Option<Count> {
        (self.last != Some(0)).then(|| Count(BigUint::new(self.digits)))
    }
}

/// An iterator over a [`Count`]'s base-2^64 limbs, least significant first;
/// see [`Count::limbs`].
///
/// Exact-size and [fused](core::iter::FusedIterator).
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct Limbs<'a> {
    limbs: U64Digits<'a>,
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
macro_rules! count_from_unsigned {
    ($($t:ty),*) => {
        $(
            impl From<$t> for Count {
                fn from(n: $t) -> Count {
                    Count(BigUint::from(u128::from(n)))
                }
            }
        )*
    };
}

count_from_unsigned!(u8, u16, u32, u64, u128);

/// Converts a count to an unsigned machine integer, returning [`TooWide`] when
/// it does not fit. Both borrowed and owned counts are accepted.
macro_rules! count_try_into_unsigned {
    ($($t:ty),*) => {
        $(
            impl TryFrom<&Count> for $t {
                type Error = TooWide;

                fn try_from(count: &Count) -> Result<$t, TooWide> {
                    <$t>::try_from(&count.0).map_err(|_| TooWide)
                }
            }

            impl TryFrom<Count> for $t {
                type Error = TooWide;

                fn try_from(count: Count) -> Result<$t, TooWide> {
                    <$t>::try_from(&count)
                }
            }
        )*
    };
}

count_try_into_unsigned!(u8, u16, u32, u64, u128, usize);

/// A count from a machine size: total, `O(1)`.
impl From<usize> for Count {
    fn from(n: usize) -> Count {
        Count(BigUint::from(n as u128))
    }
}

/// The count in decimal.
///
/// # Example
///
/// ```
/// assert_eq!(before::Count::from(42u64).to_string(), "42");
/// ```
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/count_display.html")))]
#[cfg_attr(
    not(doc),
    doc = "superlinear but subquadratic in the count's numeric width"
)]
impl fmt::Display for Count {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// Parses the canonical decimal form produced by [`Display`](fmt::Display).
/// Signs, leading zeroes, and whitespace are not accepted.
///
/// # Errors
///
/// Returns [`ParseValue::InvalidSyntax`] unless the input is one or more
/// decimal digits with no redundant leading zero.
///
/// Parsing `d` decimal digits takes `O(d²)` time and `O(d)` space.
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/count_parse.html")))]
#[cfg_attr(
    not(doc),
    doc = "`O(n^2)` in total input bytes; `O(n^2)` in text bytes"
)]
impl core::str::FromStr for Count {
    type Err = ParseValue;

    fn from_str(text: &str) -> Result<Self, ParseValue> {
        let bytes = text.as_bytes();
        if bytes.is_empty()
            || !bytes.iter().all(u8::is_ascii_digit)
            || (bytes.len() > 1 && bytes[0] == b'0')
        {
            return Err(ParseValue::InvalidSyntax);
        }
        let value = BigUint::parse_bytes(bytes, 10).ok_or(ParseValue::InvalidSyntax)?;
        Ok(Count(value))
    }
}

/// The same format as `Display`.
impl fmt::Debug for Count {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Display>::fmt(self, f)
    }
}

impl Add<&Count> for &Count {
    type Output = Count;
    fn add(self, rhs: &Count) -> Count {
        Count(&self.0 + &rhs.0)
    }
}

impl Add<Count> for Count {
    type Output = Count;
    fn add(self, rhs: Count) -> Count {
        &self + &rhs
    }
}

impl Add<&Count> for Count {
    type Output = Count;
    fn add(self, rhs: &Count) -> Count {
        &self + rhs
    }
}

impl Add<Count> for &Count {
    type Output = Count;
    fn add(self, rhs: Count) -> Count {
        self + &rhs
    }
}

impl AddAssign<&Count> for Count {
    fn add_assign(&mut self, rhs: &Count) {
        self.0 += &rhs.0;
    }
}

impl AddAssign<Count> for Count {
    fn add_assign(&mut self, rhs: Count) {
        *self += &rhs;
    }
}

/// Sums the iterator's counts; the empty sum is [`Count::ZERO`].
impl Sum<Count> for Count {
    fn sum<I: Iterator<Item = Count>>(iter: I) -> Count {
        iter.fold(Count::ZERO, |mut acc, t| {
            acc += t;
            acc
        })
    }
}

/// Sums the iterator's counts; the empty sum is [`Count::ZERO`].
impl<'a> Sum<&'a Count> for Count {
    fn sum<I: Iterator<Item = &'a Count>>(iter: I) -> Count {
        iter.fold(Count::ZERO, |mut acc, t| {
            acc += t;
            acc
        })
    }
}

#[cfg(test)]
mod tests;
