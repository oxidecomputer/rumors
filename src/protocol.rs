//! Wire reconciliation protocols.

/// The wire dialect spoken by this version of Rumors.
///
/// Peers exchange their dialect during session setup. Different versions yield
/// [`Mismatch::Protocol`](crate::error::Mismatch::Protocol). Once released, a
/// protocol's wire format is fixed; a format change requires a new variant.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum Protocol {
    /// Bounded-memory reconciliation over multiplexed logical streams.
    V2 = 2,
}

impl Protocol {
    /// Return this dialect's integer carried in the session preamble.
    pub(crate) const fn wire_version(self) -> u64 {
        self as u64
    }
}
