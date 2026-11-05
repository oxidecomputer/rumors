//! Channel constructors for the remote proxy's edges.
//!
//! Each edge keeps its height as its runtime [`QueueRole`] label.
//!
//! - [`local_questions`] holds questions whose replies are in flight. Its
//!   capacity can limit the whole descent even when the walk's queues are wider.
//! - [`parked_replies`] holds decoded replies until their stage takes them.
//! - [`next_scopes`] carries the scopes a taken reply's questions create.

use crate::tree::mirror::streaming::{
    channel::{QueueKind, QueueRole, Receiver, Sender, WindowSender, channel},
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

/// The number of replies at `height` that can be outstanding toward this
/// side when its question queue there holds `window` records: its parking
/// capacity.
///
/// A reply exists only because this side asked the question it answers, so
/// every reply in transit, being decoded, or parked answers a distinct
/// outstanding question. A question at height `h` is outstanding from the
/// moment the stage asking it hands its carrying reply toward the wire until
/// the stage consuming it takes the answer. Each such question is in one of
/// three places: recorded in the question queue and not yet dequeued (at most
/// `window`); dequeued by the consuming stage, which awaits its answer (at
/// most one); or carried by the asking stage's current reply and not yet
/// recorded (at most `FAN`, since the asking stage records all of one reply's
/// questions before handing on another). Above the leaves only
/// `256^(31 − h)` questions exist at height `h` at all, and the capacity
/// takes the smaller count. At leaf height, where each terminal request
/// names one leaf, the population exceeds every `usize`, as `256^31` does,
/// so the in-flight count alone applies.
///
/// Correctness rests on computing this from the same `window` that sizes the
/// walk's question queue at `height`, whatever that value is. The count holds
/// at exactly this value: a smaller capacity fails a conforming session.
pub fn parking_capacity(window: usize, height: usize) -> usize {
    let invited = window.saturating_add(FAN + 1);
    let exponent = u32::try_from(31_usize.saturating_sub(height)).unwrap_or(u32::MAX);
    let population = FAN.checked_pow(exponent).unwrap_or(usize::MAX);
    invited.min(population)
}

/// Hold decoded replies at `height` until their stage takes them.
///
/// The channel's one capacity is [`parking_capacity`], which no conforming
/// reply finds exhausted. The parking task never waits for room: it sends
/// without waiting and fails on a full channel, so the decoder that fills
/// this queue never waits on the walk that drains it, by construction. The
/// returned capacity is that derived one, for the overflow error to report.
pub fn parked_replies<E>(
    height: usize,
    window: usize,
) -> (Sender<Parked<E>>, Receiver<Parked<E>>, usize) {
    let capacity = parking_capacity(window, height);
    let (sender, receiver) = channel(QueueRole::new(QueueKind::ProxyResponses, height), capacity);
    (sender, receiver, capacity)
}

/// Carry the scopes a taken reply's questions create, labeled at the
/// scopes' height.
///
/// `FAN + 1` slots mean a take never waits, whatever the encoder consuming
/// this queue is doing. The encoder dequeues an answer's scope before it
/// takes the answer itself, and the walk takes its next reply only after
/// handing on every answer to the current one, through a channel that holds
/// one answer. So when a stage takes a reply, the encoder has received
/// every earlier answer but possibly the last, and so has dequeued every
/// earlier scope but possibly that answer's. That one scope and the taken
/// reply's at most `FAN` new ones fit, even while the encoder waits
/// elsewhere: for room in its question queue, or on the peer.
///
/// With fewer slots a take can wait on the encoder. During a take the
/// encoder dequeues at most two scopes, the earlier one and the take's
/// first, whose answer the stage produces only after the take; so `FAN − 2`
/// slots stall every take of a full-fan reply. At `FAN − 1` or `FAN` slots
/// such a take completes only once the encoder dequeues, so it waits on
/// whatever the encoder waits on, the peer included.
pub fn next_scopes(height: usize) -> (Sender<Scope>, Receiver<Scope>) {
    channel(QueueRole::new(QueueKind::ProxyNextScopes, height), FAN + 1)
}
