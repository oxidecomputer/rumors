//! Named bounded channels shared by streaming protocol implementations.
//!
//! Every edge is a [`bounded`] channel: one sender, one receiver, and a
//! capacity that bounds the items in flight. The receiver is also a stream,
//! keeping that conversion at the channel boundary. Unit tests substitute
//! wrappers around the same channel which expose named queue statistics,
//! per-kind capacity limits, and shrinkable delays at every send and receive
//! poll.

pub use bounded::SendError;

/// One semantic edge in the streaming protocol's channel graph.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum QueueKind {
    /// Responses waiting for the protocol schedule.
    OutgoingResponses,
    /// Completed nodes returning to the next assembly level.
    AssemblyLevelReturns,
    /// The initiator's query below the root.
    InitiatorRootQuery,
    /// The initiator's reconstructed root.
    InitiatorRootReturn,
    /// Child queries issued by the responder.
    ResponderChildQueries,
    /// The responder's resolution below the root.
    ResponderRootResolution,
    /// Nodes returning to the responder's root assembly.
    ResponderRootReturns,
    /// Child queries issued within a recursive level.
    InternalChildQueries,
    /// Resolutions sent to a recursive level's parent.
    InternalParentResolutions,
    /// Resolutions returned by a recursive level's children.
    InternalChildResolutions,
    /// Leaf prefixes awaiting lookup.
    LeafRequests,
    /// Leaf resolutions sent to their parent level.
    LeafParentResolutions,
    /// Leaf resolutions returned by child work.
    LeafChildResolutions,
    /// Leaf resolutions awaiting terminal assembly.
    TerminalLeafResolutions,
    /// Decoded wire leaves awaiting backend assembly.
    DecodedLeaves,
    /// Remote-proxy responses awaiting encoding.
    ProxyResponses,
    /// Questions issued by the local proxy walk.
    ProxyLocalQuestions,
    /// Scopes passed to the proxy's next level.
    ProxyNextScopes,
}

impl QueueKind {
    /// Every materialized semantic edge, for its coverage assertions.
    #[cfg(test)]
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
    #[cfg(test)]
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
pub use bounded::{Receiver, Sender};

/// Create the production channel for a named protocol edge.
#[cfg(not(test))]
pub fn channel<T>(_: QueueRole, capacity: usize) -> (Sender<T>, Receiver<T>) {
    bounded::channel(capacity)
}

#[cfg(test)]
pub use instrumented::{
    ChannelReport, Receiver, Sender, channel, with_kind_capacity, with_observation, with_schedule,
};

mod bounded;

#[cfg(test)]
mod instrumented;
