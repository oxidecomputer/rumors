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
    /// The original strictly alternating wire protocol.
    ///
    /// V1 alternates sending full levels of the tree, and each level
    /// message must be assembled whole in memory before it is sent: at
    /// high divergence the level message is unboundedly large, so a
    /// session can duplicate the set's own memory footprint. That
    /// unbounded term is what [`V2`](Protocol::V2) removes — streaming reconciliation
    /// under a fixed memory upper bound. V1 is kept for comparative
    /// measurement, behind the `protocol-v1` cargo feature, off by
    /// default.
    ///
    /// V1 has no session epilogue, so its `Ok` is weaker than
    /// [`V2`](Protocol::V2)'s: it certifies only the local commit, not the
    /// peer's (see [what a session promises](crate::link::Link#what-a-session-promises)).
    #[cfg(any(test, feature = "protocol-v1"))]
    V1 = 1,
    /// Bounded-memory reconciliation over multiplexed logical streams.
    #[default]
    V2 = 2,
}
