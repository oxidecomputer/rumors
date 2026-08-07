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

/// A [`Span`]'s endpoints crossed during construction.
///
/// The pair is reversed or incomparable, so zero [`Version`]s lie between them
/// (see [`Span::new`](crate::Span::new)).
///
/// # Example
///
/// ```
/// use before::{Clock, causally::Span};
/// let mut clock = Clock::seed();
/// let older = clock.tick().clone();
/// let newer = clock.tick().clone();
/// assert!(Span::new(&newer, &older).is_err()); // the endpoints cross
/// ```
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default, thiserror::Error)]
#[error("span endpoints cross: the start is not within the end")]
pub struct Crossed;

/// Why bytes failed to decode into a [`Party`](crate::Party),
/// [`Version`](crate::Version), [`Clock`](crate::Clock), [`Rank`](crate::Rank),
/// [`Ranked`](crate::Ranked), or [`Span`](crate::causally::Span).
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
    /// The bit stream ended mid-tree (or mid-integer).
    #[error("unexpected end of input")]
    Truncated,
    /// The input did not end in a complete tree followed by exactly its
    /// canonical padding: a `1` marker bit, then zeros to the byte
    /// boundary.
    ///
    /// Nothing less passes (a stream flush against the boundary still
    /// owes a whole marker byte) and nothing more (no bits after the
    /// tree beyond one padded byte, however well-formed).
    #[error("missing or malformed trailing padding")]
    TrailingBits,
    /// The structure is well-formed but not in canonical normal form.
    #[error("input is not canonical")]
    NotCanonical,
    /// The underlying reader failed.
    #[error("read error: {0}")]
    Io(io::Error),
}

/// Why a string or Rust literal failed to parse into a [`Party`](crate::Party),
/// [`Version`](crate::Version), or [`Clock`](crate::Clock).
///
/// Parsing uses the original paper's notation and strictly rejects
/// non-canonical input.
///
/// # Example
///
/// ```
/// use before::{error::Parse, Clock};
/// assert_eq!("nonsense".parse::<Clock>().unwrap_err(), Parse::Syntax);
/// ```
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, thiserror::Error)]
pub enum Parse {
    /// The input is not well-formed paper notation (bad token, unbalanced
    /// parens, non-`0`/`1` id leaf, malformed integer, or trailing input).
    #[error("input is not well-formed paper notation")]
    Syntax,
    /// The structure is well-formed but not in canonical normal form.
    #[error("input is not canonical")]
    NotCanonical,
    /// The [`Party`] denotes the anonymous identity.
    ///
    /// A standalone [`Party`](crate::Party)/[`Clock`](crate::Clock) must own a
    /// nonzero share of the unit interval `[0, 1)`.
    #[error("party is anonymous")]
    Anonymous,
}
