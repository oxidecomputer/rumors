/// Two parties were not disjoint. (`join` instead hands the clock back.)
#[derive(Debug, thiserror::Error)]
#[error("parties are not disjoint")]
pub struct OverlapError;

/// Why a byte string failed to decode into a [`Party`], [`Version`], or
/// [`Clock`].
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
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
    /// standalone [`Party`]/[`Clock`] must be a nonzero share, so this is
    /// rejected — though `0` is valid as a sub-tree inside a larger id (e.g.
    /// `(0, 1)`).
    #[error("party is anonymous")]
    Anonymous,
    /// The underlying reader failed. Only possible for `decode` from a fallible
    /// [`Read`](std::io::Read); decoding an in-memory byte slice never errors
    /// here. The [`ErrorKind`](std::io::ErrorKind) is kept (rather than the full
    /// `io::Error`) so [`DecodeError`] stays `PartialEq`/`Eq`.
    #[error("read error: {0:?}")]
    Io(std::io::ErrorKind),
}

/// Why a string (or a literal tuple/`u8`/`bool`) failed to parse into a
/// [`Party`], [`Version`], or [`Clock`].
///
/// Parsing uses the paper's notation and, like [`DecodeError`], strictly
/// rejects non-canonical input.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    /// The input is not well-formed paper notation (bad token, unbalanced
    /// parens, non-`0`/`1` id leaf, malformed integer, or trailing input).
    #[error("input is not well-formed paper notation")]
    Syntax,
    /// The input is well-formed but does not denote a value in canonical normal
    /// form (e.g. a collapsible `(1, 1)` id or `(n, m, m)` event, or an event
    /// node with no zero-base child).
    #[error("input is not canonical")]
    NotCanonical,
    /// The input denotes the anonymous identity `0` (an id owning no region). A
    /// standalone [`Party`]/[`Clock`] must be a nonzero share, so this is
    /// rejected — though `0` is valid as a sub-tree inside a larger id (e.g.
    /// `(0, 1)`).
    #[error("party is anonymous")]
    Anonymous,
}
