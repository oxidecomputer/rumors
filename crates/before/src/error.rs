//! What could possibly go wrong?

use std::io;

/// Two parties were not disjoint during [`Clock::sync`](crate::Clock::sync).
///
/// # Example
///
/// ```
/// use before::Clock;
/// let mut a = Clock::seed();
/// let mut b = Clock::seed(); // a second seed shares the first's party
/// assert!(a.sync(&mut b).is_err()); // the parties overlap
/// ```
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default, thiserror::Error)]
#[error("parties are not disjoint")]
pub struct Overlap;

/// A [`Span`](crate::Span)'s endpoints crossed during construction.
///
/// The pair is reversed or incomparable, so zero [`Version`](crate::Version)s
/// lie between them (see [`Span::new`](crate::Span::new)).
///
/// # Example
///
/// ```
/// use before::{Clock, Span};
/// let mut clock = Clock::seed();
/// let older = clock.tick().clone();
/// let newer = clock.tick().clone();
/// assert!(Span::new(&newer, &older).is_err()); // the endpoints cross
/// ```
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default, thiserror::Error)]
#[error("span endpoints cross: the start is not within the end")]
pub struct Crossed;

/// A [`Ticks`](crate::Ticks) count exceeded the machine integer it was
/// converted into.
///
/// Counts have no ceiling, so every conversion out to a fixed-width integer is
/// fallible. For an infallible way to read the count, examine each `u64` limb
/// with [`Ticks::limbs`](crate::Ticks::limbs) instead.
///
/// # Example
///
/// ```
/// use before::Ticks;
/// let wide = Ticks::from(u128::MAX);
/// assert!(u64::try_from(&wide).is_err());
/// ```
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default, thiserror::Error)]
#[error("count exceeds the machine integer's range")]
pub struct TooWide;

/// Text was not a [`Rank`](crate::Rank)'s canonical binary form.
///
/// A rank has an integer part, an optional fractional part after `.`, no
/// leading integer zeroes, and no trailing fractional zeroes.
///
/// # Example
///
/// ```
/// use before::{error::ParseRank, Rank};
/// let error = "01".parse::<Rank>().unwrap_err();
/// assert_eq!(error, ParseRank);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, thiserror::Error)]
#[error("invalid rank")]
pub struct ParseRank;

/// Why bytes failed to decode into a [`Party`](crate::Party),
/// [`Version`](crate::Version), [`Clock`](crate::Clock), [`Rank`](crate::Rank),
/// [`Ranked`](crate::Ranked), or [`Span`](crate::Span).
///
/// # Example
///
/// ```
/// use before::Clock;
/// // arbitrary bytes are not a canonical clock encoding
/// assert!(Clock::decode(&[0xff, 0xff][..]).is_err());
/// ```
///
/// An input may have more than one defect. Unless a decoder documents a
/// precedence rule, callers should handle any applicable variant.
#[derive(Debug, thiserror::Error)]
pub enum Decode {
    /// The input ended before the value did: mid-tree, mid-integer, or cut
    /// ahead of the final padding a complete tree still owes.
    ///
    /// A stream whose live bits end flush against a byte boundary carries
    /// its padding in a whole final byte, and that byte is required: an
    /// input cut just before it is truncated, not trailing-malformed.
    #[error("unexpected end of input")]
    Truncated,
    /// A complete value was followed by bits or bytes that are not part of its
    /// canonical encoding.
    ///
    /// For marker-padded values this includes a cleared marker, nonzero padding,
    /// and bytes after the padded value. An input that ends before required
    /// padding arrives is [`Decode::Truncated`] instead.
    #[error("malformed or spurious trailing input")]
    TrailingBits,
    /// The input is structurally valid but is not canonical for the requested
    /// value.
    #[error("input is not canonical")]
    NotCanonical,
    /// The underlying reader failed.
    #[error("read error: {0}")]
    Io(io::Error),
}
