//! The supply-run byte budget: how large one batched supply frame may grow.
//!
//! The streaming wire ships the leaves of a supplied subtree as *runs* — one
//! [`Supply`](super::frame::Reaction::Supply) frame carrying a delimited
//! sequence of leaf records — instead of one frame per leaf. Batching is
//! chunked by **bytes**, not record count: the encoder accumulates records
//! into the current run and flushes it when appending the next record would
//! push the frame's full wire size — its [`SUPPLY_FRAME_OVERHEAD`]-byte
//! head envelope plus the run body — past the budget. A run
//! always carries at least one record, so a single record larger than the
//! budget ships alone in its own frame, exceeding the budget by exactly
//! that record's overhang. Runs never span protocol reactions: the batching
//! scope is the leaf enumeration of one supplied subtree.
//!
//! Each endpoint's target rides its greeting, and a session runs at the
//! **minimum of the two**: one knob covers both directions — the frames
//! an endpoint builds and the frames built for it — so the more
//! memory-constrained end sets the pace and peers with different
//! settings interoperate. The budget prices encoded memory per stream:
//! the encoder buffers at most one run while filling it, and the decoder
//! buffers at most one run's bytes per frame before yielding its records
//! one at a time — each record passing custody of its payload to the
//! storage backend as it is read, so the constructed leaves in flight
//! are the sync budget's decode-fan charge, not this one's. The decode
//! side of that price is enforced, not assumed: ingress holds every
//! arriving supply frame to the session budget ([`covers`](RunBudget::covers))
//! and rejects a violating frame before buffering its body, so the
//! envelope holds against a conformance-buggy peer batching past the
//! session minimum, not only by counterparty courtesy. The public
//! knob is [`Peer::target_message_size`](crate::Peer::target_message_size).
//!
//! Framing headroom: the wire caps a run body at `u32::MAX` bytes, so
//! [`from_bytes`](RunBudget::from_bytes) saturates every budget at
//! [`MAX_RUN_BUDGET_BYTES`] — a run flushed within budget always fits the
//! cap. The one frame that can still outgrow it is a *single record*
//! larger than the cap (the minimum-one-record rule ships it alone): that
//! is a record-size limit of the wire, which no budget setting can lift,
//! and the encoder rejects it at record level before writing anything.

use crate::tree::mirror::cbor;
use crate::tree::mirror::streaming::window::FAN;

use super::frame::{MAX_QUERY_CHILDREN, listing_entry_len};
use super::signal::WireSignal;

/// The exact wire size of one full-fan query frame.
///
/// Its array head, its signal head (every query code takes the two-byte
/// head), the listing map's head at the full fan, and one entry per radix
/// value — the map spelling's per-entry cost varies with the key's head
/// width, so the sum walks the radix space rather than multiplying.
const FULL_FAN_QUERY_FRAME_LEN: usize = {
    let mut total = cbor::head_len(2) // the frame's two-item array head
        + WireSignal::MAX_ENCODED_LEN
        + cbor::head_len(MAX_QUERY_CHILDREN as u64);
    let mut radix = 0usize;
    while radix < MAX_QUERY_CHILDREN {
        total += listing_entry_len(radix as u8);
        radix += 1;
    }
    total
};

/// Default supply-run byte budget: the size of the maximally disputed reply.
///
/// Derived from the wire constants, not measured: the decode side's
/// documented memory unit is one decoded *reply* (the streaming `message`
/// module docs), and the largest non-supply reply is maximally disputed —
/// `FAN` reactions, each a full-fan query frame. Batching at this default
/// therefore never raises the wire's established per-reply memory ceiling.
pub const DEFAULT_TARGET_MESSAGE_SIZE: usize = FAN * FULL_FAN_QUERY_FRAME_LEN;

/// Wire bytes a supply frame wraps around its run body, charged at their
/// widest.
///
/// The envelope: the frame's array head, the signal's widest head, and
/// the run's embedded-sequence tag with the widest byte-string head the
/// run cap admits. The heads narrow for small runs; charging the envelope
/// constant keeps the flush algebra exact-or-conservative, never
/// optimistic.
///
/// The budget prices whole wire frames, so the encoder's flush accounting
/// charges this envelope alongside the accumulated records — a frame's full
/// wire size stays within the budget except when a single record alone
/// exceeds it.
pub const SUPPLY_FRAME_OVERHEAD: usize = cbor::head_len(2)
    + WireSignal::MAX_ENCODED_LEN
    + cbor::head_len(cbor::TAG_CBOR_SEQUENCE)
    + cbor::head_len(u32::MAX as u64);

/// The largest supply-run budget the wire can honor: budgets saturate
/// here at construction.
///
/// A frame's full wire size is its [`SUPPLY_FRAME_OVERHEAD`] envelope
/// plus the run body, and the wire caps a run body at `u32::MAX` bytes
/// (the cap every pricing closed form is denominated in). Capping the
/// whole-frame budget at that ceiling less the envelope keeps every
/// within-budget flush under the cap with the envelope already paid;
/// without it, an over-ceiling budget lets a run grow past 4 GiB in RAM
/// and then deterministically fail at the run head, re-failing every
/// retry while the divergence persists.
pub const MAX_RUN_BUDGET_BYTES: usize = u32::MAX as usize - SUPPLY_FRAME_OVERHEAD;

/// The byte budget one supply frame may grow to before the encoder flushes it.
///
/// Constructed from the public knob by [`from_bytes`](Self::from_bytes);
/// consumed by the outgoing adapter's supply-run accumulation through
/// [`admits`](Self::admits). Any value, including zero, is safe: the
/// minimum-one-record rule keeps every leaf shippable, degrading a zero
/// budget to the pre-batching one-leaf-per-frame wire traffic, and the
/// constructor's [`MAX_RUN_BUDGET_BYTES`] ceiling keeps every
/// within-budget flush inside the wire's run byte cap.
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
    /// single record alone does. Defined as [`covers`](Self::covers) of the
    /// grown body, so the encoder's flush rule and the decoder's ingress
    /// check share one boundary and cannot drift apart.
    pub fn admits(self, body: usize, record: usize) -> bool {
        self.covers(body.saturating_add(record))
    }

    /// Whether a whole supply frame of `body` run bytes fits this budget.
    ///
    /// Charges the frame's full wire size: the [`SUPPLY_FRAME_OVERHEAD`]
    /// envelope plus `body`. This is the boundary the encoder flushes
    /// against ([`admits`](Self::admits)) and the one the decoder enforces
    /// at ingress: every frame the encoder can produce either satisfies it
    /// or is a single record shipped alone (the minimum-one-record
    /// overhang), so a frame failing it with more than one record is a
    /// counterparty conformance bug.
    pub fn covers(self, body: usize) -> bool {
        SUPPLY_FRAME_OVERHEAD.saturating_add(body) <= self.bytes
    }
}

impl Default for RunBudget {
    fn default() -> Self {
        Self::from_bytes(DEFAULT_TARGET_MESSAGE_SIZE)
    }
}

#[cfg(test)]
mod tests;
