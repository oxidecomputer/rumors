//! Relations between a version and a span.
//!
//! [`Placement`] gives the detailed relation. [`Dominance`] and [`Precedence`]
//! answer the two corresponding directional questions more coarsely.

/// Which [`Span`](crate::Span) endpoint a verdict concerns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Endpoint {
    /// The span's lower endpoint, `lo`.
    Start,
    /// The span's upper endpoint, `hi`.
    End,
    /// Both endpoints at once; see [`Placement::At`] and
    /// [`Placement::Concurrent`].
    Both,
}

/// Where a version sits relative to a [`Span`](crate::Span), at the finest possible
/// resolution.
///
/// A version may be below, within, or above the span; equal to an endpoint; or
/// concurrent with one or both endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Placement {
    /// Strictly below the whole span: `v < lo`, hence `v < hi`.
    Before,
    /// Exactly at an endpoint:
    ///
    /// - `At(Start)`: `v == lo < hi`.
    /// - `At(End)`: `lo < v == hi`.
    /// - `At(Both)`: `v == lo == hi`. Equality to one endpoint of a
    ///   point span is equality to both, so on `lo == hi`
    ///   every at-endpoint verdict is `At(Both)`.
    At(Endpoint),
    /// Strictly inside: `lo < v < hi`.
    Between,
    /// Concurrent with the endpoint(s) named by the variant. The opposite
    /// relation follows from `lo <= hi`:
    ///
    /// - `Concurrent(Start)`: `v ∥ lo`, forcing `v < hi` (at or above
    ///   `hi` would put `v` above `lo`).
    /// - `Concurrent(End)`: `v ∥ hi`, forcing `v > lo` (at or below
    ///   `lo` would put `v` below `hi`).
    /// - `Concurrent(Both)`: `v ∥ lo` and `v ∥ hi`.
    Concurrent(Endpoint),
    /// Strictly above the whole span: `v > hi`, hence `v > lo`.
    After,
}

/// How much of a [`Span`](crate::Span) a [`Version`](crate::Version) dominates.
///
/// This is [`Placement`] coarsened to the dominance question, "is the version
/// causally at or after the span's content?" ([`Precedence`] renders the
/// mirrored verdict over the other direction of the order.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dominance {
    /// The version dominates the whole span: `hi <= v`, and with it
    /// every version the span covers.
    After,
    /// The version dominates the start but not the whole: `lo <= v`,
    /// while `hi` is above or concurrent to the version.
    Between,
    /// The version does not dominate even the start: `lo` is above or
    /// concurrent to the version (and with it `hi` as well).
    Before,
}

/// How much of a [`Span`](crate::Span) a [`Version`](crate::Version) precedes.
///
/// This is [`Placement`] coarsened to the precedence question, "is the version
/// causally at or before the span's content?" ([`Dominance`] renders the
/// mirrored verdict over the other direction of the order.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Precedence {
    /// The version precedes the whole span: `v <= lo`, and with it
    /// every version the span covers.
    Before,
    /// The version precedes the end but not the whole: `v <= hi`,
    /// while `lo` is below or concurrent with the version.
    Between,
    /// The version does not precede even the end: `hi` is below or
    /// concurrent with the version (and with it `lo`).
    After,
}
