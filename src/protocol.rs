//! Selectable wire reconciliation protocols.

/// The wire dialect a reconciliation session speaks.
///
/// Both endpoints must select the same protocol. A different wire version
/// yields [`Mismatch::Protocol`](crate::error::Mismatch::Protocol).
/// Once released, a protocol's wire format is fixed; a format change requires
/// a new protocol variant.
#[repr(u16)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum Protocol {
    /// Bounded-memory reconciliation over multiplexed logical streams.
    #[default]
    V2 = 2,
}
