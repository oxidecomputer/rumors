//! The supply-run byte budget: how large one batched wire message may grow.
//!
//! The streaming wire ships the leaves of a supplied subtree as *runs* — one
//! [`Supply`](super::frame::Reaction::Supply) frame carrying a delimited
//! sequence of leaf records — instead of one frame per leaf
//! (`design/streaming-latency-serialization.md` §10, lever A). Batching is
//! chunked by **bytes**, not record count: the encoder accumulates records
//! into the current run and flushes it when appending the next record would
//! push the run's encoded size past the budget. A run always carries at
//! least one record, so a single record larger than the budget ships alone
//! in its own run, exceeding the budget by exactly that record's overhang.
//! Runs never span protocol reactions: the batching scope is the leaf
//! enumeration of one supplied subtree.
//!
//! The budget is what one endpoint's *own* runs may grow to; it is not
//! wire-visible, and peers with different settings interoperate — each side
//! decodes whatever run sizes the other chose to send. What the budget
//! prices is memory per stream: the encoder buffers at most one run while
//! filling it, and the decoder buffers at most one run's bytes per frame
//! before yielding its records one at a time. The public knob is
//! [`Peer::target_message_size`](crate::Peer::target_message_size).
//!
//! Framing headroom: runs ride the wire's `u32` length header
//! ([`framing`](crate::tree::mirror::framing)), so budgets up to
//! `u32::MAX` bytes are representable; the encoder rejects a frame beyond
//! that before writing anything.

use crate::tree::mirror::streaming::window::FAN;

use super::frame::{MAX_QUERY_CHILDREN, QUERY_CHILD_LEN, QUERY_COUNT_LEN};
use super::signal::WireSignal;

/// Default supply-run byte budget: the size of the maximally disputed reply.
///
/// Derived from the wire constants, not measured: the wire's largest
/// non-supply message is a maximally disputed reply of `FAN` (256)
/// reactions, each a full-fan query frame — one signal byte, one count
/// byte, and `MAX_QUERY_CHILDREN` (256) children of `QUERY_CHILD_LEN` (17)
/// bytes each — totalling 256 × (1 + 1 + 256 × 17) = 1 114 624 bytes. That
/// reply is already the decode side's documented per-message memory unit
/// (the streaming `message` module docs), so batching at this default never
/// raises the wire's per-message ceiling.
pub const DEFAULT_TARGET_MESSAGE_SIZE: usize =
    FAN * (WireSignal::ENCODED_LEN + QUERY_COUNT_LEN + MAX_QUERY_CHILDREN * QUERY_CHILD_LEN);

/// The byte budget one supply run may grow to before the encoder flushes it.
///
/// Constructed from the public knob by [`from_bytes`](Self::from_bytes);
/// consumed by the outgoing adapter's supply-run accumulation. Any value,
/// including zero, is safe: the minimum-one-record rule keeps every leaf
/// shippable, degrading a zero budget to the pre-batching one-leaf-per-frame
/// wire traffic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunBudget {
    /// Encoded run bytes admitted before the next record forces a flush.
    bytes: usize,
}

impl RunBudget {
    /// Adopt a caller-selected byte budget.
    pub fn from_bytes(bytes: usize) -> Self {
        Self { bytes }
    }

    /// The encoded-size bound this budget grants one run.
    pub fn bytes(self) -> usize {
        self.bytes
    }
}

impl Default for RunBudget {
    fn default() -> Self {
        Self::from_bytes(DEFAULT_TARGET_MESSAGE_SIZE)
    }
}

#[cfg(test)]
mod tests;
