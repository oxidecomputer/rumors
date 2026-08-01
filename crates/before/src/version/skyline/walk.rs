//! The leaf-walk driver: one home for the iterative
//! descend-to-leaf/backtrack skeleton every in-order leaf pass over a
//! skyline subtree runs.
//!
//! A skyline subtree's leaves are visited by alternating one
//! word-parallel unary read (the descent: a run of internal flags ended
//! by the leaf's `1`) with a pop-flip backtrack over the root-to-leaf
//! path bits (closing the ancestors the consumed leaf completed). The
//! driver owns exactly that skeleton; everything a pass *does* at a
//! leaf — decode the payload, skip it by width, fold an extremum, emit
//! — stays at the call site, on the caller's own state. [`Extremum`]
//! is the one leaf action shared widely enough to live here too: the
//! armed, reset-on-overtake streaming max/min the scanning walks fold.

use core::cmp::Ordering;

use suanpan::Accumulator;

use crate::codec::{Base, BitCursor, BitsMut, DsiCursor};

use super::{fold_signed, unzigzag};

/// The topology walk over one skyline subtree's leaves, in preorder.
///
/// The driver reads only topology bits; the payload code at each
/// yielded leaf is the caller's. The cursor is a per-call argument
/// rather than owned state so the caller keeps it between calls — the
/// consuming walks read their payloads through the same cursor the
/// driver descends with, and their surrounding state (watermark webs,
/// output builders, height accumulators) borrows freely alongside.
pub(super) struct LeafWalk {
    /// Root-to-leaf branch directions for the current leaf, root
    /// first: `false` inside an ancestor's left child (its right
    /// subtree is still pending in the stream), `true` inside its
    /// right.
    path: BitsMut,
    /// Whether a leaf has been yielded: the first descent has no
    /// finished leaf to backtrack from.
    started: bool,
}

impl LeafWalk {
    /// A walk positioned to enter the subtree at the caller's cursor.
    pub(super) fn new() -> Self {
        LeafWalk {
            path: BitsMut::new(),
            started: false,
        }
    }

    /// Advance to the next leaf, returning its depth below the walked
    /// subtree's root — or `None` when the previous leaf was the
    /// subtree's last.
    ///
    /// Each call closes the ancestors the previous leaf completed
    /// (the pop-flip backtrack), then descends to the leaf at the
    /// cursor.
    ///
    /// Between calls the caller must advance the cursor past exactly
    /// the yielded leaf's payload code (`read_int` or `skip_int`): the
    /// driver reads topology only, and the next descent starts at the
    /// following node's flag. The backtrack is pure path bookkeeping —
    /// it reads no bits — so a caller that stops mid-subtree (a
    /// position-bounded prefix pass) simply stops calling.
    ///
    /// # Panics
    ///
    /// Panics if the stream is not a canonical skyline encoding.
    pub(super) fn descend(&mut self, cursor: &mut DsiCursor<'_>) -> Option<usize> {
        if self.started {
            loop {
                match self.path.pop() {
                    Some(true) => continue,
                    Some(false) => {
                        self.path.push(true);
                        break;
                    }
                    None => return None,
                }
            }
        }
        self.started = true;
        // One whole descent per unary read: `k` internal nodes, then
        // the leaf whose flag terminates the run.
        let k = cursor.read_unary().expect("canonical skyline bits");
        for _ in 0..k {
            self.path.push(false);
        }
        Some(self.path.len())
    }
}

/// Which extreme an [`Extremum`] tracks.
enum Direction {
    /// The maximum leaf height.
    Max,
    /// The minimum leaf height.
    Min,
}

/// A streaming extremum of the leaf heights a walk consumes, carried
/// relative to the walk's own running height.
///
/// The register holds `extremum − h`: each leaf-to-leaf step folds in
/// *reversed* (the height moved, the extremum did not), and when the
/// register's sign shows the height just crossed the tracked extreme,
/// the register resets to zero — the extremum is the current height
/// again. The first leaf *arms* the fold and is never folded: its
/// payload is the range's entry height, not a leaf-to-leaf movement,
/// so the register starts at zero on it (extremum = h) whatever its
/// coding. The finished offset's width is bounded by the scanned
/// range's own content, which prices every later fold of it.
pub(super) struct Extremum {
    /// `extremum − h`, the running register.
    acc: Accumulator,
    /// Whether the first leaf has armed the fold.
    armed: bool,
    /// The extreme the register tracks.
    direction: Direction,
}

impl Extremum {
    /// Track the maximum, resetting when the height rises past it.
    ///
    /// `acc` is a leased watermark-pool buffer (zero, returned to its
    /// pool by the caller's materialize), so resets re-zero it in
    /// place and the pool stays warm.
    pub(super) fn max(acc: Accumulator) -> Self {
        Extremum {
            acc,
            armed: false,
            direction: Direction::Max,
        }
    }

    /// Track the minimum, resetting when the height drops past it.
    ///
    /// `acc` is an owned buffer: resets replace it whole, because an
    /// in-place clear scans (and meters) every dead digit a wide swing
    /// left behind where dropping the buffer is O(1).
    pub(super) fn min(acc: Accumulator) -> Self {
        Extremum {
            acc,
            armed: false,
            direction: Direction::Min,
        }
    }

    /// Fold one consumed leaf-to-leaf step; the arming first call
    /// folds nothing.
    pub(super) fn fold(&mut self, neg: bool, mag: &Base) {
        if !self.armed {
            self.armed = true;
            return;
        }
        self.fold_armed(neg, mag);
    }

    /// Fold one undecoded zigzag payload code; the arming first call
    /// folds nothing and leaves its code undecoded (an armed leaf's
    /// absolute-vs-delta coding is irrelevant — it is never folded).
    pub(super) fn fold_zigzag(&mut self, code: Base) {
        if !self.armed {
            self.armed = true;
            return;
        }
        let (neg, mag) = unzigzag(code);
        self.fold_armed(neg, &mag);
    }

    fn fold_armed(&mut self, neg: bool, mag: &Base) {
        fold_signed(&mut self.acc, !neg, mag);
        let overtaken = match self.direction {
            Direction::Max => Ordering::Less,
            Direction::Min => Ordering::Greater,
        };
        if self.acc.sign() == overtaken {
            match self.direction {
                Direction::Max => self.acc.reset(),
                Direction::Min => self.acc = Accumulator::new(),
            }
        }
    }

    /// The finished register, `extremum − h` at the walk's exit.
    pub(super) fn into_offset(self) -> Accumulator {
        self.acc
    }
}
