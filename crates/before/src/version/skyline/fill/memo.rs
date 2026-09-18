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
//! sibling (the walk re-anchors its relation from its range-minimum stack at that
//! sibling's range close). A zero link is not stored at all — the queue cell
//! answers `None` — so sibling or nested sites sharing one minimum cost
//! nothing: one wide shared minimum is never materialized per covering site.
//!
//! The queue is reserved in consumption order, but links may land later: a
//! sibling link lands at its own close, while a first child's link lands at its
//! parent's close. Each queue slot therefore holds an optional index into the
//! nonzero links' separate write-order store.
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

use core::num::NonZeroUsize;

use super::StoredAccumulator;

// Every site costs one machine word; only nonzero links allocate values.
const _: () =
    assert!(core::mem::size_of::<Option<NonZeroUsize>>() == core::mem::size_of::<usize>());

/// The memoized pre-scan's output — the frame ledger.
///
/// Per left-full site, in the walk's arrival (pre-order) order, one optional
/// link resolving the site's minimum against a reference the walk already
/// holds when it arrives (the module doc carries the reference discipline and
/// the one-create/one-consume lifetime).
pub(super) struct Memo {
    /// Per site, in consumption order: `None` for a zero link, otherwise the
    /// one-based index of its value in `links`.
    pub(super) queue: Vec<Option<NonZeroUsize>>,
    /// Nonzero links, in write order.
    links: Vec<StoredAccumulator>,
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

/// Fold one position into an order-sensitive checksum (FNV-style).
#[cfg(debug_assertions)]
pub(super) fn position_check(check: u64, pos: u64) -> u64 {
    (check ^ pos).wrapping_mul(0x0100_0000_01b3)
}

impl Memo {
    /// Construct an empty ledger.
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

    /// Reset for a new fresh scan, retaining both vector allocations.
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

    /// Store a nonzero link in its reserved queue slot.
    pub(super) fn set_link(&mut self, slot: usize, link: StoredAccumulator) {
        debug_assert!(self.queue[slot].is_none(), "a memo slot is written once");
        self.links.push(link);
        self.queue[slot] = NonZeroUsize::new(self.links.len());
    }

    /// Take `slot`'s link out for its one consuming read, if nonzero.
    pub(super) fn take_link(&mut self, slot: usize) -> Option<StoredAccumulator> {
        let index = self.queue[slot].take()?;
        Some(core::mem::replace(
            &mut self.links[index.get() - 1],
            StoredAccumulator::Small(0),
        ))
    }
}
