//! Typed channel constructors for the remote proxy's three dataflow edges.
//!
//! Every edge has capacity one. A response is published before the scopes it
//! releases, while a complete outgoing wire reply is flushed before its local
//! question scopes are published. Those orderings let each consumer drain the
//! occupied slot without requiring a second slot for progress.

use crate::tree::{
    mirror::streaming::channel::{QueueKind, QueueRole, Receiver, Sender, channel},
    typed::height::Height,
};

/// Buffer one decoded response on its way to the local protocol participant.
pub fn responses<T, H: Height>() -> (Sender<T>, Receiver<T>) {
    channel(QueueRole::new(QueueKind::ProxyResponses, H::HEIGHT), 1)
}

/// Carry questions whose complete outgoing wire reply has flushed.
pub fn local_questions<T, H: Height>() -> (Sender<T>, Receiver<T>) {
    channel(QueueRole::new(QueueKind::ProxyLocalQuestions, H::HEIGHT), 1)
}

/// Carry scopes derived from a response already published locally.
pub fn next_scopes<T, H: Height>() -> (Sender<T>, Receiver<T>) {
    channel(QueueRole::new(QueueKind::ProxyNextScopes, H::HEIGHT), 1)
}
