//! Selectable wire reconciliation protocols.

/// The reconciliation protocol used for future wire sessions.
///
/// Both endpoints of a session must select the same protocol. [`V2`] is the
/// default; earlier dialects remain selectable behind cargo features (see
/// the variants).
///
/// [`V2`]: Self::V2
#[repr(u16)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Protocol {
    /// The original materialized, strictly alternating wire protocol.
    ///
    /// Behind the `protocol-v1` cargo feature: kept for wire compatibility
    /// with V1 peers and comparative measurement, and off by default because
    /// its state machines are a large monomorphization surface that every
    /// downstream binary would otherwise compile.
    #[cfg(any(test, feature = "protocol-v1"))]
    V1 = 1,
    /// Fixed-memory reconciliation over multiplexed logical streams.
    #[default]
    V2 = 2,
}
