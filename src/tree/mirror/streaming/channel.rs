//! Named bounded channels shared by streaming protocol implementations.
//!
//! Every edge is a [`bounded`] channel: one sender, one receiver, and a
//! capacity that bounds the items in flight. The receiver is also a stream,
//! keeping that conversion at the channel boundary. Unit tests substitute
//! wrappers around the same channel which expose named queue statistics,
//! per-kind capacity limits, and shrinkable delays at every send and receive
//! poll.

#[cfg(test)]
use bounded::SendError;
#[cfg(any(test, feature = "test-internals"))]
pub use bounded::{EdgeSend, EdgeSendHook, observe_edge_sends};
pub use bounded::{HANDOFF_CAPACITY, TrySendError, UNBOUNDED};

use super::holding::Holding;

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
    /// Supplied nodes the backend assembled, awaiting their reply's reader.
    AssembledSupplies,
    /// Decoded remote replies parked until their stage takes them.
    ProxyResponses,
    /// Questions issued by the local proxy walk.
    ProxyLocalQuestions,
    /// Scopes passed to the proxy's next level.
    ProxyNextScopes,
    /// Frames one local encoder hands the pipe's multiplexer.
    PipeOutgoing,
    /// Frames the pipe's demultiplexer routes to one level's decoder.
    PipeIncoming,
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
    pub const PROXY: [Self; 5] = [
        Self::ProxyResponses,
        Self::ProxyLocalQuestions,
        Self::ProxyNextScopes,
        Self::PipeOutgoing,
        Self::PipeIncoming,
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

impl QueueKind {
    /// The place in a session whose state this edge's items are, for the
    /// session-budget meter's attribution ([`super::holding`]), or `None` where
    /// each item's place is its sender's.
    ///
    /// Each edge whose items the ledger charges belongs to the place they are
    /// charged to. The edges of fixed capacity the charge leaves out, the
    /// assembly returns, the terminal leaf resolutions, and the assembled
    /// supplies, belong to assembly, whose bound accounts for them. The pipe's
    /// lanes carry supply runs, whose permits from the wire pools price their
    /// nodes, beside every other frame, whose nodes the charge of the chunk or
    /// the room of the question they belong to prices, so each frame's sender
    /// names its node's place.
    #[cfg_attr(not(any(test, feature = "test-internals")), expect(dead_code))]
    pub(crate) const fn holding(self) -> Option<Holding> {
        Some(match self {
            Self::InitiatorRootQuery
            | Self::ResponderChildQueries
            | Self::InternalChildQueries
            | Self::LeafRequests => Holding::Questions,
            Self::ResponderRootResolution
            | Self::InternalParentResolutions
            | Self::InternalChildResolutions
            | Self::LeafParentResolutions
            | Self::LeafChildResolutions => Holding::Resolutions,
            Self::ProxyLocalQuestions | Self::ProxyNextScopes => Holding::QuestionRecords,
            Self::ProxyResponses => Holding::ParkedReplies,
            Self::DecodedLeaves => Holding::DecodedLeaves,
            Self::OutgoingResponses => Holding::OutgoingReplies,
            Self::AssemblyLevelReturns
            | Self::InitiatorRootReturn
            | Self::ResponderRootReturns
            | Self::TerminalLeafResolutions
            | Self::AssembledSupplies => Holding::Assembly,
            Self::PipeOutgoing | Self::PipeIncoming => return None,
        })
    }
}

#[cfg(not(test))]
pub use bounded::{Receiver, Sender};

/// Create the production channel for a named protocol edge.
///
/// With `test-internals`, each send's channel node is attributed to the edge's
/// place, where it has one ([`QueueKind::holding`]), and each send is reported
/// to the process's send hook (`observe_edge_sends`).
#[cfg(not(test))]
pub fn channel<T>(role: QueueRole, capacity: usize) -> (Sender<T>, Receiver<T>) {
    #[cfg(not(feature = "test-internals"))]
    let _ = role;
    let (sender, receiver) = bounded::channel(capacity);
    #[cfg(feature = "test-internals")]
    let sender = sender.edge(role.kind);
    (sender, receiver)
}

#[cfg(test)]
pub use instrumented::{
    ChannelReport, Receiver, Sender, channel, with_kind_capacity, with_observation, with_role_hold,
    with_schedule,
};

mod bounded;

#[cfg(test)]
mod instrumented;
