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
/// Counts have no ceiling, so every conversion out to a fixed-width
/// integer is fallible; a count past the range answers this. Read every limb
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
    /// The input did not end in a complete tree followed by exactly its
    /// canonical padding: a `1` marker bit, then zeros to the byte
    /// boundary.
    ///
    /// Malformed padding (a cleared marker, a stray set bit) and spurious
    /// input past the one padded byte both land here; an input that ends
    /// before any padding arrives is [`Decode::Truncated`] instead.
    #[error("malformed or spurious trailing padding")]
    TrailingBits,
    /// The structure is well-formed but not in canonical normal form.
    #[error("input is not canonical")]
    NotCanonical,
    /// The underlying reader failed.
    #[error("read error: {0}")]
    Io(io::Error),
}
