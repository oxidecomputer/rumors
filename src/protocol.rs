//! Selectable wire reconciliation protocols.

/// The wire dialect a reconciliation session speaks.
///
/// Both endpoints of a session must speak the same dialect; the preamble
/// enforces this, diagnosing a skewed pairing as
/// [`Error::VersionMismatch`](crate::Error::VersionMismatch). The wire
/// format of a shipped version is frozen: a wire change means a new
/// variant here, never a mutation of a released dialect (see the crate
/// docs' stability notes), which is why the discriminant crosses the wire
/// as a version number.
#[repr(u16)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum Protocol {
    /// Bounded-memory reconciliation over multiplexed logical streams.
    #[default]
    V2 = 2,
}
