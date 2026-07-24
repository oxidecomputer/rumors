//! The supply-run byte budget: how large one batched supply frame may grow.
//!
//! The streaming wire ships the leaves of a supplied subtree as *runs* — one
//! [`Supply`](super::frame::Reaction::Supply) frame carrying a delimited
//! sequence of leaf records — instead of one frame per leaf. Batching is
//! chunked by **bytes**, not record count: the encoder accumulates records
//! into the current run and flushes it when appending the next record would
//! push the frame's full wire size — its [`SUPPLY_FRAME_OVERHEAD`]-byte
//! signal-and-length envelope plus the run body — past the budget. A run
//! always carries at least one record, so a single record larger than the
//! budget ships alone in its own frame, exceeding the budget by exactly
//! that record's overhang. Runs never span protocol reactions: the batching
//! scope is the leaf enumeration of one supplied subtree.
//!
//! Each endpoint's target rides its greeting, and a session runs at the
//! **minimum of the two**: the knob states both how large a frame its
//! peer builds for it and how large a frame it builds for its peer, so
//! the more memory-constrained end sets the pace and peers with
//! different settings interoperate. What the budget prices is encoded
//! memory per stream: the encoder buffers at most one run while filling
//! it, and the decoder buffers at most one run's bytes per frame before
//! yielding its records one at a time — each record passing custody of
//! its payload to the storage backend as it is read, so the constructed
//! leaves in flight are the sync budget's decode-fan charge, not this
//! one's. The public knob is
//! [`Peer::target_message_size`](crate::Peer::target_message_size).
//!
//! Framing headroom: runs ride the wire's `u32` length header
//! ([`framing`](crate::tree::mirror::framing)), so
//! [`from_bytes`](RunBudget::from_bytes) saturates every budget at
//! [`MAX_RUN_BUDGET_BYTES`] — a run flushed within budget always fits the
//! header. The one frame that can still outgrow it is a *single record*
//! larger than the header's ceiling (the minimum-one-record rule ships it
//! alone): that is a record-size limit of the wire, which no budget
//! setting can lift, and the encoder rejects it at the header before
//! writing anything.

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

/// The largest supply-run budget the wire's framing can honor: budgets
/// saturate here at construction.
///
/// A frame's full wire size is its [`SUPPLY_FRAME_OVERHEAD`] envelope
/// plus the run body, and the body's length must encode in the `u32`
/// header ([`framing`](crate::tree::mirror::framing)). Capping the
/// whole-frame budget at `u32::MAX` less the envelope keeps every
/// within-budget flush under the header's ceiling with the envelope
/// already paid; without the cap, an over-ceiling budget lets a run
/// grow past 4 GiB in RAM and then deterministically fail at the length
/// header, re-failing every retry while the divergence persists.
pub const MAX_RUN_BUDGET_BYTES: usize = u32::MAX as usize - SUPPLY_FRAME_OVERHEAD;

/// The byte budget one supply frame may grow to before the encoder flushes it.
///
/// Constructed from the public knob by [`from_bytes`](Self::from_bytes);
/// consumed by the outgoing adapter's supply-run accumulation through
/// [`admits`](Self::admits). Any value, including zero, is safe: the
/// minimum-one-record rule keeps every leaf shippable, degrading a zero
/// budget to the pre-batching one-leaf-per-frame wire traffic, and the
/// constructor's [`MAX_RUN_BUDGET_BYTES`] ceiling keeps every
/// within-budget flush inside the wire's length header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunBudget {
    /// Wire-frame bytes admitted before the next record forces a flush.
    bytes: usize,
}

impl RunBudget {
    /// Adopt a caller-selected byte budget, saturated at
    /// [`MAX_RUN_BUDGET_BYTES`].
    ///
    /// Saturation here is what makes every value safe: this is the single
    /// constructor — the default, the public knob, and the negotiated
    /// session minimum all pass through it — so no stored budget exceeds
    /// what the framing can flush, and the greeting advertises the
    /// saturated value.
    pub fn from_bytes(bytes: usize) -> Self {
        Self {
            bytes: bytes.min(MAX_RUN_BUDGET_BYTES),
        }
    }

    /// The byte budget, as the greeting carries it.
    pub fn bytes(self) -> usize {
        self.bytes
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
