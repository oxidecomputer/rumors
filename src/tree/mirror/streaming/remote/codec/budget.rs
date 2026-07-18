//! The supply-run byte budget: how large one batched supply frame may grow.
//!
//! The streaming wire ships the leaves of a supplied subtree as *runs* — one
//! [`Supply`](super::frame::Reaction::Supply) frame carrying a delimited
//! sequence of leaf records — instead of one frame per leaf
//! (`design/streaming-latency-serialization.md` §10, lever A). Batching is
//! chunked by **bytes**, not record count: the encoder accumulates records
//! into the current run and flushes it when appending the next record would
//! push the frame's full wire size — its [`SUPPLY_FRAME_OVERHEAD`]-byte
//! signal-and-length envelope plus the run body — past the budget. A run
//! always carries at least one record, so a single record larger than the
//! budget ships alone in its own frame, exceeding the budget by exactly
//! that record's overhang. Runs never span protocol reactions: the batching
//! scope is the leaf enumeration of one supplied subtree.
//!
//! The budget is what one endpoint's *own* supply frames may grow to; it is
//! not wire-visible, and peers with different settings interoperate — each
//! side decodes whatever run sizes the other chose to send. What the budget
//! prices is memory per stream: the encoder buffers at most one run while
//! filling it, and the decoder buffers at most one run's bytes per frame
//! before yielding its records one at a time. The public knob is
//! [`Peer::target_message_size`](crate::Peer::target_message_size).
//!
//! Framing headroom: runs ride the wire's `u32` length header
//! ([`framing`](crate::tree::mirror::framing)), so budgets up to
//! `u32::MAX` bytes are representable; the encoder rejects a frame beyond
//! that before writing anything.

use crate::tree::mirror::framing::LENGTH_HEADER_LEN;
use crate::tree::mirror::streaming::window::FAN;

use super::frame::{MAX_QUERY_CHILDREN, QUERY_CHILD_LEN, QUERY_COUNT_LEN};
use super::signal::WireSignal;

/// Default supply-run byte budget: the size of the maximally disputed reply.
///
/// Derived from the wire constants, not measured: the decode side's
/// documented memory unit is one decoded *reply* (the streaming `message`
/// module docs), and the largest non-supply reply is maximally disputed —
/// `FAN` (256) reactions, each a full-fan query frame of one signal byte,
/// one count byte, and `MAX_QUERY_CHILDREN` (256) children of
/// `QUERY_CHILD_LEN` (17) bytes each — totalling 256 × (1 + 1 + 256 × 17) =
/// 1 114 624 bytes. Batching at this default therefore never raises the
/// wire's established per-reply memory ceiling.
pub const DEFAULT_TARGET_MESSAGE_SIZE: usize =
    FAN * (WireSignal::ENCODED_LEN + QUERY_COUNT_LEN + MAX_QUERY_CHILDREN * QUERY_CHILD_LEN);

/// Wire bytes a supply frame wraps around its run body: the signal byte and
/// the body's `u32` length header.
///
/// The budget prices whole wire frames, so the encoder's flush accounting
/// charges this envelope alongside the accumulated records — a frame's full
/// wire size stays within the budget except when a single record alone
/// exceeds it.
pub const SUPPLY_FRAME_OVERHEAD: usize = WireSignal::ENCODED_LEN + LENGTH_HEADER_LEN;

/// The byte budget one supply frame may grow to before the encoder flushes it.
///
/// Constructed from the public knob by [`from_bytes`](Self::from_bytes);
/// consumed by the outgoing adapter's supply-run accumulation through
/// [`admits`](Self::admits). Any value, including zero, is safe: the
/// minimum-one-record rule keeps every leaf shippable, degrading a zero
/// budget to the pre-batching one-leaf-per-frame wire traffic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunBudget {
    /// Wire-frame bytes admitted before the next record forces a flush.
    bytes: usize,
}

impl RunBudget {
    /// Adopt a caller-selected byte budget.
    pub fn from_bytes(bytes: usize) -> Self {
        Self { bytes }
    }

    /// Whether a run may absorb one more record within this budget.
    ///
    /// Charges the whole wire frame — the [`SUPPLY_FRAME_OVERHEAD`] envelope
    /// plus the run's `body` bytes plus the `record` bytes about to join it —
    /// so a flushed frame's on-wire size never exceeds the budget unless a
    /// single record alone does.
    pub fn admits(self, body: usize, record: usize) -> bool {
        SUPPLY_FRAME_OVERHEAD
            .saturating_add(body)
            .saturating_add(record)
            <= self.bytes
    }
}

impl Default for RunBudget {
    fn default() -> Self {
        Self::from_bytes(DEFAULT_TARGET_MESSAGE_SIZE)
    }
}

#[cfg(test)]
mod tests;
