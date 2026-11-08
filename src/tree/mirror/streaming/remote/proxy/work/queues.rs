//! Channel constructors for the remote proxy's edges.
//!
//! Each edge keeps its height as its runtime [`QueueRole`] label. Each
//! constructor's documentation says what its edge carries, between which tasks,
//! and why its capacity is what it is.

use crate::tree::mirror::streaming::{
    FAN,
    channel::{QueueKind, QueueRole, Receiver, Sender, UNBOUNDED, channel},
    materialized::OUTGOING_REPLY_CHUNKS,
};

use super::{Derived, Outstanding, Parked};

/// Carry the records of asked-but-unanswered questions to the decoder, labeled
/// at the questions' height.
///
/// [`encode`](mod@super::encode) publishes each record right after the frame
/// carrying its question, often partway through a reply, and
/// [`stages`](mod@super::stages) takes one per decoded wire reply, a round trip
/// later. The queue is [`UNBOUNDED`], so publishing never stops an encoder
/// before the rest of its reply reaches the peer; what it holds is already
/// charged, each record on the endpoint's ledger as it publishes, beside the
/// room its reply parks in.
pub fn local_questions(height: usize) -> (Sender<Outstanding>, Receiver<Outstanding>) {
    channel(
        QueueRole::new(QueueKind::ProxyLocalQuestions, height),
        UNBOUNDED,
    )
}

/// Hold decoded chunks of replies at `height` until their stage takes them.
///
/// The queue is [`UNBOUNDED`]: the decoder that fills it must never wait on the
/// walk that drains it, and what it holds needs no count of its own, since
/// every chunk carries its part of the room the question its reply answers set
/// aside on the endpoint's ledger before that question left.
pub fn parked_replies<E>(height: usize) -> (Sender<Parked<E>>, Receiver<Parked<E>>) {
    channel(QueueRole::new(QueueKind::ProxyResponses, height), UNBOUNDED)
}

/// Carry the scopes a taken chunk's questions create, labeled at the scopes'
/// height, each with its charge ([`Derived`]).
///
/// `FAN + OUTGOING_REPLY_CHUNKS` slots mean a stage taking a chunk never waits
/// to publish its scopes, whatever the encoder consuming this queue is doing.
/// The encoder dequeues an answer's scope before it takes any of the answer's
/// chunks, and the walk takes its next chunk only after handing on every answer
/// to the questions of the current one, through a channel that holds
/// [`OUTGOING_REPLY_CHUNKS`] chunks. So when a stage takes a chunk, every
/// earlier answer the encoder has not begun lies wholly in that channel, one
/// chunk at least each, and the encoder has dequeued every earlier scope but at
/// most [`OUTGOING_REPLY_CHUNKS`] of them. Those scopes and the chunk's new
/// ones fit, even while the encoder waits elsewhere: for room in its question
/// queue, or on the peer. A reply's questions take distinct positions of the
/// record it answers, so its chunks together create at most `FAN` scopes.
///
/// With fewer slots, a stage taking a chunk can wait on the encoder. While the
/// proxy publishes a taken chunk's scopes, the encoder dequeues the earlier
/// scopes and then the chunk's first, whose answer the stage produces only once
/// it has received the chunk, and no more; so a full-fan chunk needs `FAN − 1`
/// of its scopes queued at once, and with `FAN − 2` slots a stage can never
/// take one. At `FAN − 1` slots or more, below `FAN + OUTGOING_REPLY_CHUNKS`, a
/// stage can take a full-fan chunk only once the encoder dequeues, so it waits
/// on whatever the encoder waits on, the peer included.
pub fn next_scopes(height: usize) -> (Sender<Derived>, Receiver<Derived>) {
    channel(
        QueueRole::new(QueueKind::ProxyNextScopes, height),
        FAN + OUTGOING_REPLY_CHUNKS,
    )
}
