//! Channel constructors for the remote proxy's edges.
//!
//! Each edge keeps its height as its runtime [`QueueRole`] label.
//!
//! - [`local_questions`] holds the records of questions whose replies are in
//!   flight.
//! - [`parked_replies`] holds decoded chunks until their stage takes them.
//! - [`next_scopes`] carries the scopes a taken reply's questions create.

use crate::tree::mirror::streaming::{
    FAN,
    channel::{QueueKind, QueueRole, Receiver, Sender, UNBOUNDED, channel},
    remote::adapter::Scope,
};

use super::Parked;
use crate::tree::mirror::streaming::ledger::Charge;

/// Carry the records of asked-but-unanswered questions to the decoder,
/// labeled at the questions' height.
///
/// [`encode`](mod@super::encode) publishes each record right after the
/// frame carrying its question, often partway through a reply, and
/// [`stages`](mod@super::stages) takes one per decoded wire reply, a round
/// trip later. The queue is [`UNBOUNDED`], so publishing never stops an
/// encoder before the rest of its reply reaches the peer; what it holds is
/// already charged, each record on the endpoint's ledger as it publishes.
pub fn local_questions(height: usize) -> (Sender<(Scope, Charge)>, Receiver<(Scope, Charge)>) {
    channel(
        QueueRole::new(QueueKind::ProxyLocalQuestions, height),
        UNBOUNDED,
    )
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
