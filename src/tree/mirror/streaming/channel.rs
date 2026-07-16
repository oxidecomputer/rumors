//! Named bounded channels shared by streaming protocol implementations.
//!
//! Production uses Tokio's channel types directly. Unit tests substitute a
//! wrapper which preserves Tokio's capacity and wakeup behavior while exposing
//! named queue statistics, per-role capacity limits, and shrinkable delays at
//! every send and receive poll.

/// One semantic edge in the materialized protocol's channel graph.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum QueueKind {
    OutgoingResponses,
    AssemblyLevelReturns,
    InitiatorRootQuery,
    InitiatorRootReturn,
    ResponderChildQueries,
    ResponderRootResolution,
    ResponderRootReturns,
    InternalChildQueries,
    InternalParentResolutions,
    InternalChildResolutions,
    LeafRequests,
    LeafParentResolutions,
    LeafChildResolutions,
    TerminalLeafResolutions,
    ProxyResponses,
    ProxyLocalQuestions,
    ProxyNextScopes,
}

impl QueueKind {
    /// Every materialized semantic edge, for its coverage assertions.
    pub const ALL: [Self; 14] = [
        Self::OutgoingResponses,
        Self::AssemblyLevelReturns,
        Self::InitiatorRootQuery,
        Self::InitiatorRootReturn,
        Self::ResponderChildQueries,
        Self::ResponderRootResolution,
        Self::ResponderRootReturns,
        Self::InternalChildQueries,
        Self::InternalParentResolutions,
        Self::InternalChildResolutions,
        Self::LeafRequests,
        Self::LeafParentResolutions,
        Self::LeafChildResolutions,
        Self::TerminalLeafResolutions,
    ];

    /// Every remote-proxy semantic edge, for its coverage assertions.
    pub const PROXY: [Self; 3] = [
        Self::ProxyResponses,
        Self::ProxyLocalQuestions,
        Self::ProxyNextScopes,
    ];
}

/// A semantic queue edge at the height carried by its item type.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct QueueRole {
    /// The semantic dataflow edge.
    pub kind: QueueKind,
    /// The typed height carried by the channel's item.
    pub height: usize,
}

impl QueueRole {
    /// Name one semantic edge at its item height.
    pub const fn new(kind: QueueKind, height: usize) -> Self {
        Self { kind, height }
    }
}

#[cfg(not(test))]
pub use tokio::sync::mpsc::{Receiver, Sender};

/// Create the production Tokio channel for a named protocol edge.
#[cfg(not(test))]
pub fn channel<T>(_: QueueRole, capacity: usize) -> (Sender<T>, Receiver<T>) {
    tokio::sync::mpsc::channel(capacity)
}

#[cfg(test)]
pub use instrumented::{
    ChannelReport, Receiver, RoleStats, Sender, channel, with_capacity_limit, with_kind_capacity,
    with_observation, with_role_capacity, with_schedule,
};

#[cfg(test)]
mod instrumented;
