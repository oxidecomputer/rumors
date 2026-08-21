//! Channel constructors for the remote proxy's scope-carrying edges.
//!
//! A response is published before the scopes it releases, and a complete
//! outgoing wire reply is flushed before its local question scopes are
//! published. Those orderings make one slot per edge the liveness floor.
//! Both edges carry the erased [`Scope`] — one channel-machinery
//! instantiation for the whole proxy — with each edge's height kept as
//! its runtime [`QueueRole`] label. (The third proxy edge, the one-slot
//! decoded-response relay, is created by the response pump itself: see
//! `Work::respond`.)
//!
//! - [`local_questions`] is the wire-facing question window itself, sized
//!   by the session
//!   [`Window`](crate::tree::mirror::streaming::window::Window): one slot
//!   there re-serializes the descent no matter how wide the walk's own
//!   channels are;
//! - [`next_scopes`] is the decode-side register, also window-sized, whose
//!   items are small enough to widen defensively.

use crate::tree::mirror::streaming::{
    channel::{QueueKind, QueueRole, Receiver, Sender, channel},
    remote::adapter::Scope,
};

/// Carry flushed-but-unanswered questions, window-wide, labeled at the
/// questions' height.
///
/// This queue's occupancy tracks the questions in flight on the wire at
/// this height: the encoder publishes each question once its complete
/// reply has flushed, and the decoder retires one per decoded wire reply —
/// a full round trip later. Tracks, not equals: occupancy undercounts the
/// wire by a bounded slack (a flushing batch rides the wire before
/// publication; the decoder holds one dequeued entry while its reply
/// decodes). The canonical derivation — the occupancy bound, its
/// reachability, and the slack — is in the
/// [`window`](crate::tree::mirror::streaming::window) module docs.
pub fn local_questions(height: usize, capacity: usize) -> (Sender<Scope>, Receiver<Scope>) {
    channel(
        QueueRole::new(QueueKind::ProxyLocalQuestions, height),
        capacity,
    )
}

/// Carry scopes derived from a response already published locally,
/// window-wide, labeled at the scopes' height.
pub fn next_scopes(height: usize, capacity: usize) -> (Sender<Scope>, Receiver<Scope>) {
    channel(QueueRole::new(QueueKind::ProxyNextScopes, height), capacity)
}
