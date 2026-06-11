//! What can go wrong, and which operations raise it: [`Overlap`] (joining
//! non-disjoint parties), [`Decode`] (rejecting non-canonical bytes), and
//! [`Parse`] (rejecting malformed display text).

/// Two parties were not disjoint during [`Party::join`](crate::Party::join) or
/// [`Clock::join`](crate::Clock::join).
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

/// Why a byte string failed to decode into a [`Party`](crate::Party),
/// [`Version`](crate::Version), or [`Clock`](crate::Clock).
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
    /// Non-padding bits remained after a complete tree, or the padding was
    /// nonzero.
    #[error("trailing or nonzero padding bits")]
    TrailingBits,
    /// The structure is well-formed but not in canonical normal form.
    #[error("input is not canonical")]
    NotCanonical,
    /// The id region is the anonymous identity `0` (it owns no region). A
    /// standalone [`Party`](crate::Party)/[`Clock`](crate::Clock) must own a
    /// nonzero share of the unit interval `[0, 1)`.
    #[error("party is anonymous")]
    Anonymous,
    /// The underlying reader failed.
    #[error("read error: {0:?}")]
    Io(std::io::Error),
}

/// Why a string or Rust literal failed to parse into a [`Party`](crate::Party),
/// [`Version`](crate::Version), or [`Clock`](crate::Clock).
///
/// Parsing uses the paper's notation and strictly rejects non-canonical input.
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
    /// The id region is the anonymous identity `0` (it owns no region). A
    /// standalone [`Party`](crate::Party)/[`Clock`](crate::Clock) must own a
    /// nonzero share of the unit interval `[0, 1)`.
    #[error("party is anonymous")]
    Anonymous,
}
