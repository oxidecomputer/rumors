//! Named bounded channels shared by streaming protocol implementations.
//!
//! Every edge is a [`bounded`] channel: one sender, one receiver, and a
//! capacity that bounds the items in flight. The receiver is also a stream,
//! keeping that conversion at the channel boundary. Unit tests substitute
//! wrappers around the same channel which expose named queue statistics,
//! per-kind capacity limits, and shrinkable delays at every send and receive
//! poll.

pub use bounded::{SendError, TrySendError, UNBOUNDED};

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
    /// session-budget meter's attribution ([`super::holding`]).
    ///
    /// Each edge belongs to the place whose items it carries. The edges of
    /// fixed capacity, the assembly returns, the terminal leaf resolutions,
    /// and the assembled supplies, belong to assembly.
    #[cfg_attr(not(any(test, feature = "test-internals")), expect(dead_code))]
    pub(crate) const fn holding(self) -> Holding {
        match self {
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
            Self::PipeOutgoing | Self::PipeIncoming => Holding::Wire,
        }
    }
}

#[cfg(not(test))]
pub use bounded::{Receiver, Sender};

/// Create the production channel for a named protocol edge.
///
/// With `test-internals`, each send's channel node is attributed to the
/// edge's place ([`QueueKind::holding`]).
#[cfg(not(test))]
pub fn channel<T>(role: QueueRole, capacity: usize) -> (Sender<T>, Receiver<T>) {
    #[cfg(not(feature = "test-internals"))]
    let _ = role;
    let (sender, receiver) = bounded::channel(capacity);
    #[cfg(feature = "test-internals")]
    let sender = sender.holding(role.kind.holding());
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

/// The sending half of a queue the session window sizes, which counts each
/// send that blocks because the queue is full.
///
/// [`SessionStats::window_stalls`](super::stats::SessionStats::window_stalls)
/// counts every time a stage had to wait for room in a queue that sends
/// through this wrapper. A window-sized queue fed by a plain [`Sender`] goes
/// uncounted, so zero means no queue sending through this wrapper made a
/// stage wait.
pub(crate) struct WindowSender<T> {
    /// The bounded protocol edge.
    inner: Sender<T>,
    /// The session receiving the pressure signal.
    stats: super::stats::Recorder,
}

impl<T> WindowSender<T> {
    /// Wrap a sender whose capacity was derived from the session window.
    pub(crate) fn new(inner: Sender<T>, stats: &super::stats::Recorder) -> Self {
        Self {
            inner,
            stats: stats.clone(),
        }
    }

    /// Send one item, counting a stall if the send has to wait for room.
    ///
    /// A poll that returns pending while the queue is full is a blocked send;
    /// a pending poll with room left (a test schedule's injected delay) is
    /// not. Each send counts at most once.
    pub(crate) async fn send(&mut self, item: T) -> Result<(), SendError<T>> {
        let stats = &self.stats;
        self.inner.send_with(item, || stats.window_stall()).await
    }
}
