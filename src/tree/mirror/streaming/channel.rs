//! Named bounded channels shared by streaming protocol implementations.
//!
//! The receiver is also a stream, keeping that conversion at the channel
//! boundary. Unit tests substitute wrappers which preserve Tokio's capacity
//! and wakeup behavior while exposing named queue statistics, per-kind capacity
//! limits, and shrinkable delays at every send and receive poll.

#[cfg(not(test))]
use std::{
    pin::Pin,
    task::{Context, Poll},
};

#[cfg(not(test))]
use futures::Stream;

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
pub use tokio::sync::mpsc::Sender;

/// The receiving half of a production protocol channel.
#[cfg(not(test))]
pub struct Receiver<T>(tokio::sync::mpsc::Receiver<T>);

#[cfg(not(test))]
impl<T> Receiver<T> {
    /// Receive the next item, or `None` after every sender is dropped.
    pub async fn recv(&mut self) -> Option<T> {
        self.0.recv().await
    }
}

#[cfg(not(test))]
impl<T> Stream for Receiver<T> {
    type Item = T;

    /// Poll the underlying bounded channel for its next item.
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.0.poll_recv(cx)
    }
}

/// Create the production Tokio channel for a named protocol edge.
#[cfg(not(test))]
pub fn channel<T>(_: QueueRole, capacity: usize) -> (Sender<T>, Receiver<T>) {
    let (sender, receiver) = tokio::sync::mpsc::channel(capacity);
    (sender, Receiver(receiver))
}

#[cfg(test)]
pub use instrumented::{
    ChannelReport, Receiver, Sender, channel, with_kind_capacity, with_observation, with_schedule,
};

#[cfg(test)]
mod instrumented;
