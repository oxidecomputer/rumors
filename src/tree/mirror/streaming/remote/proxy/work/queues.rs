//! Channel constructors for the remote proxy's scope-carrying edges.
//!
//! A response is published before the scopes it releases, and a complete
//! outgoing wire reply is flushed before its local question scopes are
//! published. Those orderings make one slot per edge the liveness floor.
//! Both edges carry the erased [`Scope`] — one channel-machinery
//! instantiation for the whole proxy — with each edge's height kept as
//! its runtime [`QueueRole`] label. (The third proxy edge, the one-slot
//! decoded-response relay, is created by the response task itself: see
//! `Work::respond`.)
//!
//! - [`local_questions`] holds questions whose replies are in flight. Its
//!   capacity can limit the whole descent even when the walk's queues are wider.
//! - [`next_scopes`] carries scopes released by decoded replies.

use crate::tree::mirror::streaming::{
    channel::{QueueKind, QueueRole, Receiver, Sender, WindowSender, channel},
    remote::adapter::Scope,
    stats::Recorder,
};

/// Carry flushed-but-unanswered questions, window-wide, labeled at the
/// questions' height.
///
/// This queue's occupancy tracks the questions in flight on the wire at
/// this height: [`encode`](mod@super::encode) publishes each question once its
/// complete reply has flushed, and [`stages`](mod@super::stages) removes one per
/// decoded wire reply — a full round trip later. Occupancy can undercount the
/// wire slightly: a flushing batch has not been published yet, and the decoder
/// holds one dequeued question while reading its reply. An upstream queue may
/// recycle its slots while those replies remain outstanding, so it cannot
/// bound this edge. Giving this queue the same depth-specific window preserves
/// the session's population bound.
///
/// Sized by the window, so it counts its stalls with the walk's window
/// queues.
pub fn local_questions(
    height: usize,
    capacity: usize,
    stats: &Recorder,
) -> (WindowSender<Scope>, Receiver<Scope>) {
    let (sender, receiver) = channel(
        QueueRole::new(QueueKind::ProxyLocalQuestions, height),
        capacity,
    );
    (WindowSender::new(sender, stats), receiver)
}

/// Carry scopes derived from a response already published locally,
/// window-wide, labeled at the scopes' height.
pub fn next_scopes(height: usize, capacity: usize) -> (Sender<Scope>, Receiver<Scope>) {
    channel(QueueRole::new(QueueKind::ProxyNextScopes, height), capacity)
}
