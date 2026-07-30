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
//! — stays at the call site, on the caller's own state.

use crate::codec::{BitCursor, Bits, DsiCursor};
use crate::step;

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
    path: Bits,
    /// Whether a leaf has been yielded: the first descent has no
    /// finished leaf to backtrack from.
    started: bool,
}

impl LeafWalk {
    /// A walk positioned to enter the subtree at the caller's cursor.
    pub(super) fn new() -> Self {
        LeafWalk {
            path: Bits::new(),
            started: false,
        }
    }

    /// Advance to the next leaf: close the ancestors the previous leaf
    /// completed, then descend to the leaf at the cursor, returning its
    /// depth below the walked subtree's root — or `None` when the
    /// previous leaf was the subtree's last.
    ///
    /// Between calls the caller must advance the cursor past exactly
    /// the yielded leaf's payload code (`read_int` or `skip_int`): the
    /// driver reads topology only, and the next descent starts at the
    /// following node's flag. The backtrack is pure path bookkeeping —
    /// it reads no bits — so a caller that stops mid-subtree (a
    /// position-bounded prefix pass) simply stops calling.
    ///
    /// Metering: exactly one [`step!`] per descent (per unary read),
    /// the whole skeleton's step budget; the backtrack is unmetered.
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
        step!();
        let k = cursor.read_unary().expect("canonical skyline bits");
        for _ in 0..k {
            self.path.push(false);
        }
        Some(self.path.len())
    }
}
