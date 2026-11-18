//! Wire reconciliation protocols.

/// The wire protocol version used by this version of Rumors.
///
/// Peers exchange their protocol version during each gossip session. Different
/// versions result in [`Mismatch::Protocol`](crate::error::Mismatch::Protocol).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum Protocol {
    /// This version of the protocol was initially released in the 0.1.0 release
    /// of `rumors`.
    V2 = 2,
}

impl Protocol {
    /// Return this dialect's integer carried in the session preamble.
    pub(crate) const fn wire_version(self) -> u64 {
        self as u64
    }
}
