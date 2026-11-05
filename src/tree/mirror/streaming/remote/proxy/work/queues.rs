//! Channel constructors for the remote proxy's edges.
//!
//! Each edge keeps its height as its runtime [`QueueRole`] label.
//!
//! - [`local_questions`] holds questions whose replies are in flight. Its
//!   capacity can limit the whole descent even when the walk's queues are wider.
//! - [`parked_replies`] holds decoded chunks until their stage takes them.
//! - [`next_scopes`] carries the scopes a taken reply's questions create.

use crate::tree::mirror::streaming::{
    channel::{QueueKind, QueueRole, Receiver, Sender, UNBOUNDED, WindowSender, channel},
    remote::adapter::Scope,
    stats::Recorder,
    window::FAN,
};

use super::Parked;
use crate::tree::mirror::streaming::ledger::Charge;

/// Carry asked-but-unanswered questions, window-wide, labeled at the
/// questions' height.
///
/// This queue's occupancy tracks the questions in flight at this height:
/// [`encode`](mod@super::encode) publishes each question once its complete
/// reply is in the multiplexer's channel, and [`stages`](mod@super::stages)
/// removes one per decoded wire reply, a full round trip later. Occupancy
/// can undercount the questions in flight slightly: a batch still reaching
/// the multiplexer has not been published yet, and the decoder holds one
/// dequeued question while reading its reply. An upstream queue may
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
) -> (WindowSender<(Scope, Charge)>, Receiver<(Scope, Charge)>) {
    let (sender, receiver) = channel(
        QueueRole::new(QueueKind::ProxyLocalQuestions, height),
        capacity,
    );
    (WindowSender::new(sender, stats), receiver)
}

/// Hold decoded chunks of replies at `height` until their stage takes
/// them.
///
/// The queue is [`UNBOUNDED`]: the decoder that fills it must never wait on
/// the walk that drains it, and what it holds needs no count of its own,
/// since every chunk lies within the room the question its reply answers
/// set aside on the endpoint's ledger before that question left.
pub fn parked_replies<E>(height: usize) -> (Sender<Parked<E>>, Receiver<Parked<E>>) {
    channel(QueueRole::new(QueueKind::ProxyResponses, height), UNBOUNDED)
}

/// Carry the scopes a taken chunk's questions create, labeled at the
/// scopes' height.
///
/// `FAN + 1` slots mean a take never waits, whatever the encoder consuming
/// this queue is doing. The encoder dequeues an answer's scope before it
/// takes the answer itself, and the walk takes its next chunk only after
/// handing on every answer to the questions of the current one, through a
/// channel that holds one answer. So when a stage takes a chunk, the
/// encoder has received every earlier answer but possibly the last, and so
/// has dequeued every earlier scope but possibly that answer's. That one
/// scope and the chunk's new ones fit, even while the encoder waits
/// elsewhere: for room in its question queue, or on the peer. A reply's
/// questions take distinct positions of the record it answers, so its
/// chunks together create at most `FAN` scopes.
///
/// With fewer slots a take can wait on the encoder. During a take the
/// encoder dequeues at most two scopes, the earlier one and the take's
/// first, whose answer the stage produces only after the take; so `FAN − 2`
/// slots stall every take of a full-fan chunk. At `FAN − 1` or `FAN` slots
/// such a take completes only once the encoder dequeues, so it waits on
/// whatever the encoder waits on, the peer included.
pub fn next_scopes(height: usize) -> (Sender<Scope>, Receiver<Scope>) {
    channel(QueueRole::new(QueueKind::ProxyNextScopes, height), FAN + 1)
}
