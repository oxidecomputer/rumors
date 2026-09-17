//! Byte limits for batched supply frames.
//!
//! A supply frame carries a run of leaf records from one supplied subtree.
//! [`RunBudget`] counts the complete frame, including
//! [`SUPPLY_FRAME_OVERHEAD`]. The encoder flushes before the next record would
//! cross the limit. Every run must contain a record, so one oversized record
//! travels alone; runs never combine separate protocol reactions.
//!
//! Each endpoint advertises its target in the greeting, and the session uses
//! the smaller target in both directions. The encoder buffers one run per
//! stream. The decoder applies the same limit before buffering a run: a frame
//! may exceed it only when the frame contains one record. Constructed leaves
//! are charged separately by the synchronization window.
//! Applications configure the target with
//! [`Peer::target_message_size`](crate::Peer::target_message_size).
//!
//! The wire stores run length in a `u32`. [`RunBudget::from_bytes`] therefore
//! caps every target at [`MAX_RUN_BUDGET_BYTES`]. A single record beyond that
//! wire limit is rejected before the encoder writes any part of its frame.

use crate::tree::mirror::cbor;

use super::error::DecodeErrorKind;
use super::frame::{MAX_QUERY_CHILDREN, SUPPLY_HEAD_LEN, listing_entry_len};
use super::signal::FRAME_OPENER_LEN;

/// The exact wire size of one full-fan query frame.
///
/// This includes the frame opener, the listing-map head, and every radix and
/// hash. Radix keys have different CBOR head sizes, so the calculation visits
/// each key rather than multiplying one entry size.
const FULL_FAN_QUERY_FRAME_LEN: usize = {
    let mut total = FRAME_OPENER_LEN + cbor::head_len(MAX_QUERY_CHILDREN as u64);
    let mut radix = 0usize;
    while radix < MAX_QUERY_CHILDREN {
        total += listing_entry_len(radix as u8);
        radix += 1;
    }
    total
};

/// Default supply-run byte budget: the size of the maximally disputed reply.
///
/// A non-supply reply can contain one reaction per radix, each a full-fan
/// query. Giving supply runs the same byte allowance preserves that per-reply
/// memory ceiling.
pub const DEFAULT_TARGET_MESSAGE_SIZE: usize = MAX_QUERY_CHILDREN * FULL_FAN_QUERY_FRAME_LEN;

/// Most wire bytes a supply frame can add around its run body.
///
/// This includes the frame opener, embedded-sequence tag, and widest possible
/// byte-string head. Smaller runs use fewer bytes, so charging this value is
/// conservative.
pub const SUPPLY_FRAME_OVERHEAD: usize = FRAME_OPENER_LEN + SUPPLY_HEAD_LEN;

/// Largest whole-frame budget that keeps an in-budget run within the wire cap.
///
/// The wire represents the run-body length as a `u32`; subtracting the frame
/// overhead ensures that every run accumulated within budget can be encoded.
pub const MAX_RUN_BUDGET_BYTES: usize = u32::MAX as usize - SUPPLY_FRAME_OVERHEAD;

/// The byte budget one supply frame may grow to before the encoder flushes it.
///
/// A run always accepts its first record, even when the complete frame exceeds
/// this target. Thus every value is usable; a target of zero sends one record
/// per frame. Values above [`MAX_RUN_BUDGET_BYTES`] are saturated so an
/// accumulated run always fits the wire's length field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunBudget {
    /// Wire-frame bytes admitted before the next record forces a flush.
    bytes: usize,
}

impl RunBudget {
    /// Adopt a caller-selected target, saturated at [`MAX_RUN_BUDGET_BYTES`].
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
    /// This charges the complete frame after adding `record`. The encoder still
    /// accepts the first record when this returns `false`.
    pub fn admits(self, body: usize, record: usize) -> bool {
        self.covers(body.saturating_add(record))
    }

    /// Whether a whole supply frame of `body` run bytes fits this budget.
    ///
    /// The encoder uses this boundary when batching, and the decoder uses it
    /// when checking incoming batches.
    pub fn covers(self, body: usize) -> bool {
        SUPPLY_FRAME_OVERHEAD.saturating_add(body) <= self.bytes
    }

    /// Describe an incoming frame that exceeds this budget.
    pub(super) fn overbatched(self, body: usize) -> DecodeErrorKind {
        DecodeErrorKind::OverbatchedRun {
            declared: SUPPLY_FRAME_OVERHEAD.saturating_add(body),
            budget: self.bytes,
        }
    }
}

impl Default for RunBudget {
    fn default() -> Self {
        Self::from_bytes(DEFAULT_TARGET_MESSAGE_SIZE)
    }
}

#[cfg(test)]
mod tests;
