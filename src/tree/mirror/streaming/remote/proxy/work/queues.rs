//! Typed channel constructors for the remote proxy's three dataflow edges.
//!
//! A response is published before the scopes it releases, and a complete
//! outgoing wire reply is flushed before its local question scopes are
//! published. Those orderings make one slot per edge the liveness floor.
//!
//! - [`local_questions`] is the wire-facing question window itself, sized
//!   by the session
//!   [`Window`](crate::tree::mirror::streaming::window::Window): one slot
//!   there re-serializes the descent no matter how wide the walk's own
//!   channels are;
//! - [`next_scopes`] is the decode-side register, also window-sized, whose
//!   items are small enough to widen defensively;
//! - [`responses`] stays at one slot: it is an in-order relay pump, so a
//!   full slot only stalls when its consumer is itself stalled, and the
//!   single slot is what bounds decoded replies in flight per stage.

use crate::tree::{
    mirror::streaming::channel::{QueueKind, QueueRole, Receiver, Sender, channel},
    typed::height::Height,
};

/// Buffer one decoded response on its way to the local protocol participant.
pub fn responses<T, H: Height>() -> (Sender<T>, Receiver<T>) {
    channel(QueueRole::new(QueueKind::ProxyResponses, H::HEIGHT), 1)
}

/// Carry flushed-but-unanswered questions, window-wide.
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
pub fn local_questions<T, H: Height>(capacity: usize) -> (Sender<T>, Receiver<T>) {
    channel(
        QueueRole::new(QueueKind::ProxyLocalQuestions, H::HEIGHT),
        capacity,
    )
}

/// Carry scopes derived from a response already published locally,
/// window-wide.
pub fn next_scopes<T, H: Height>(capacity: usize) -> (Sender<T>, Receiver<T>) {
    channel(
        QueueRole::new(QueueKind::ProxyNextScopes, H::HEIGHT),
        capacity,
    )
}
