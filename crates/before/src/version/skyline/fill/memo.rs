//! The frame ledger: the channel between the fill walk and its memoized
//! pre-scan.
//!
//! A left-full site (minted in [`fill`](super)'s module doc: an id node whose
//! left child is full, the shortcut direction whose raised leaf precedes the
//! range its minimum argument comes from) collapses that child to `max(max(el),
//! min(fill(ir, er)))`, and the right sibling's filled minimum is unknown until
//! the sibling range — which the walk has not reached — has been walked. The
//! walk therefore sends one pre-scan ahead (the `prescan` module) the first
//! time it meets such a site uncovered, and that scan evaluates every interior
//! left-full site inside its span in the same pass, recording each site's
//! minimum here — so no stream position is ever pre-scanned twice, and the walk
//! arrives at every covered site with its raise argument resolved.
//!
//! # Ledger links: references the walk already holds
//!
//! No minimum is materialized. [`Memo`] stores per site, in the walk's arrival
//! (pre-order) order, one optional *link*: the site's minimum as a difference
//! against a reference the walk already holds when it arrives.
//!
//! - A site with an earlier sibling under the same forest parent stores
//!   `m_s − m_prev`, the difference against that sibling's minimum.
//! - A forest parent's first child stores `m_s − m_parent`, written at the
//!   *parent's* own close — when the parent's minimum is final — into the
//!   child's earlier queue slot.
//! - The scan's outermost site stores `m_root − h(scan entry)`, the difference
//!   against the height where the scan (and the walk, on arrival) stands.
//!
//! The walk's arrival-order relation is exactly that reference at every
//! consume: the site consumed before a first child is its parent (the live
//! relation), and the site consumed before a later sibling is the previous
//! sibling (the walk re-anchors its relation from its own watermark web at that
//! sibling's range close). A zero link is not stored at all — the queue cell
//! answers `None` — so sibling or nested sites sharing one minimum cost
//! nothing: one wide shared minimum is never materialized per covering site.
//!
//! # Write order vs consumption order
//!
//! The queue is indexed in consumption (stream) order — the pre-scan reserves
//! each site's slot the moment it meets the site — but links land out of that
//! order: a sibling link lands at its own site's close, while a deferred
//! first-child link lands at its forest parent's close. The queue's indices
//! into the link store are what decouple the two orders.
//!
//! # Lifetime: one create, one consume
//!
//! Each stored link is created once at a close (the recording head moves into
//! the queue, or a deferred first-child link is cloned once at its own width),
//! read once at its site's consume, and dies into the raise decision it serves.
//! The scan's keeper and suspend folds each read a dying operand or a link's
//! own priced width; the one live follower — the recording head scan-side, the
//! ledger relation walk-side — receives the per-event fold the watermark
//! discipline already prices (closes excepted, where it rides the latent tag
//! untouched); and the recorder adds one amortized sign read per site (the
//! zero-link test). [`position_check`] pairs the two sides in debug builds: an
//! order-sensitive checksum of recorded and of consumed site positions, matched
//! when each scan's ledger drains.

use core::num::NonZeroU32;

use suanpan::Accumulator;

/// The memoized pre-scan's output — the frame ledger.
///
/// Per left-full site, in the walk's arrival (pre-order) order, one optional
/// link resolving the site's minimum against a reference the walk already
/// holds when it arrives (the module doc carries the reference discipline and
/// the one-create/one-consume lifetime).
pub(super) struct Memo {
    /// Per site, in consumption (stream) order: `None` when the site's link is
    /// zero, else the 1-based index of its link in `links`.
    ///
    /// One index-sized cell per site (`Option<NonZeroU32>` occupies the niche,
    /// pinned by the const assert below), so sites sharing minima store
    /// nothing beyond it.
    pub(super) queue: Vec<Option<NonZeroU32>>,
    /// The nonzero links, in write order (sibling links land at their sites'
    /// closes, deferred first-child links at their parents') — the queue's
    /// indices decouple write order from consumption order.
    links: Vec<Accumulator>,
    /// The consumption cursor into `queue`.
    pub(super) cursor: usize,
    /// The end position of the current fresh scan's span: sites before it are
    /// recorded; a site at or past it launches a new scan.
    pub(super) covered_until: u64,
    /// Order-sensitive checksum of the recorded sites' positions, matched
    /// against the consumed ones when the scan drains — O(1) state where a
    /// position list would bill the heap meter for a debug-only buffer.
    #[cfg(debug_assertions)]
    pub(super) recorded_check: u64,
    /// The consumed positions' checksum (see `recorded_check`).
    #[cfg(debug_assertions)]
    pub(super) consumed_check: u64,
}

// The queue's per-site cost claim: an optional index costs exactly an index.
const _: () = assert!(core::mem::size_of::<Option<NonZeroU32>>() == core::mem::size_of::<u32>());

/// Fold one position into an order-sensitive checksum (FNV-style).
#[cfg(debug_assertions)]
pub(super) fn position_check(check: u64, pos: u64) -> u64 {
    (check ^ pos).wrapping_mul(0x0100_0000_01b3)
}

impl Memo {
    pub(super) fn new() -> Self {
        Memo {
            queue: Vec::new(),
            links: Vec::new(),
            cursor: 0,
            covered_until: 0,
            #[cfg(debug_assertions)]
            recorded_check: 0,
            #[cfg(debug_assertions)]
            consumed_check: 0,
        }
    }

    /// Reset for a new fresh scan, keeping the queue allocation.
    pub(super) fn begin_scan(&mut self) {
        debug_assert_eq!(self.cursor, self.queue.len(), "the prior scan drained");
        #[cfg(debug_assertions)]
        debug_assert_eq!(
            self.recorded_check, self.consumed_check,
            "the walk consumed the recorded sites, in order"
        );
        self.queue.clear();
        self.links.clear();
        self.cursor = 0;
    }

    /// Store a nonzero link for `slot`, in write order.
    pub(super) fn set_link(&mut self, slot: usize, link: Accumulator) {
        // Push first: the store's length is then provably a valid, nonzero
        // 1-based index (the `expect` is the u32 capacity contract alone).
        self.links.push(link);
        let index = u32::try_from(self.links.len()).expect("site count fits u32");
        self.queue[slot] = NonZeroU32::new(index);
    }

    /// Take `slot`'s link out for its one consuming read, if nonzero.
    pub(super) fn take_link(&mut self, slot: usize) -> Option<Accumulator> {
        let index = self.queue[slot]?;
        Some(core::mem::take(&mut self.links[index.get() as usize - 1]))
    }
}
