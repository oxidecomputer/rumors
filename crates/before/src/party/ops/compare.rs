use core::cmp::Ordering;

use crate::idbits::IdView;
use crate::recurse::descend;

impl IdView<'_> {
    /// Whether `self` and `other` (normal-form ids) share no owned region. `O(n + m)`: both
    /// cursors are threaded, and a side is skipped only where the other's leaf dominates it.
    ///
    /// Recursive form of the paper's region-disjointness test, guarded by
    /// [`crate::recurse`] so deep ids grow the stack onto the heap rather than
    /// overflowing.
    pub(crate) fn is_disjoint(&self, other: &IdView) -> bool {
        // Each subtree walk returns where it ended in both inputs (so a right
        // sibling resumes without re-scanning), or `None` the instant an overlap
        // is found, unwinding the whole walk.
        descend!(0, disjoint_rec(*self, *other, 0, 0, 0)).is_some()
    }

    /// The descent order on `self` and `other` (normal-form ids), in a single
    /// `O(n + m)` pass. `Some(Less)` means `self` is an ancestor of (its region
    /// contains) `other`; `Some(Greater)` the reverse; `Some(Equal)` equal
    /// regions; `None` incomparable (cousins).
    ///
    /// The recursive form of `oracle::Party::contains`, run in both directions at
    /// once. Tracks both containment directions together — `a ⊇ b` as `le` and
    /// `b ⊇ a` as `ge` — so the two reverse-inclusion scans share one traversal
    /// instead of running `contains` twice; the walk stops early once both are
    /// excluded. Only a both-node pair descends: wherever at least one side is a
    /// leaf, that region's value (empty / full) settles both directions locally,
    /// and the other side is skipped once to resync (bounded lazy-skip), so each
    /// node is still visited at most once. Recursion is guarded by
    /// [`crate::recurse`] against deep ids.
    pub(crate) fn compare(&self, other: &IdView) -> Option<Ordering> {
        // Both ids are canonical normal form, so bit-equality is semantic
        // equality: settle `Equal` with one length-checked memcmp before
        // recursing. Differing lengths fail in O(1); only equal-length inputs
        // pay the scan.
        if self.bits() == other.bits() {
            return Some(Ordering::Equal);
        }
        let mut walk = CmpWalk {
            a: *self,
            b: *other,
            le: true, // `a ⊇ b` (a is an ancestor of b) still possible
            ge: true, // `b ⊇ a` still possible
        };
        match descend!(0, walk.rec(0, 0, 0)) {
            None => None, // incomparable: neither containment can recover
            Some(_) => match (walk.le, walk.ge) {
                (true, true) => Some(Ordering::Equal),
                (true, false) => Some(Ordering::Less),
                (false, true) => Some(Ordering::Greater),
                (false, false) => unreachable!("both-false returns `None` inside `rec`"),
            },
        }
    }
}

/// One subtree of the [`is_disjoint`](IdView::is_disjoint) walk. Returns where
/// the subtree ended in each input (to thread the right sibling), or `None` the
/// moment an overlap is found, which unwinds the whole walk.
fn disjoint_rec(
    a: IdView,
    b: IdView,
    a_pos: usize,
    b_pos: usize,
    depth: usize,
) -> Option<(usize, usize)> {
    let a_hdr = a.header(a_pos);
    let b_hdr = b.header(b_pos);
    if a_hdr.is_empty() {
        // a owns nothing here: disjoint. Skip b's subtree to resync.
        return Some((a_hdr.next, b.skip(b_pos)));
    }
    if b_hdr.is_empty() {
        // b owns nothing here: disjoint. Skip a's subtree to resync.
        return Some((a.skip(a_pos), b_hdr.next));
    }
    if a_hdr.is_full() || b_hdr.is_full() {
        // One side is full and the other nonempty: overlap.
        return None;
    }
    // Both internal: descend in lockstep, threading the right child from where
    // the left ended.
    let (a_mid, b_mid) = descend!(
        depth + 1,
        disjoint_rec(a, b, a_hdr.next, b_hdr.next, depth + 1)
    )?;
    descend!(depth + 1, disjoint_rec(a, b, a_mid, b_mid, depth + 1))
}

/// The mutable state of a [`compare`](IdView::compare) walk: the two id views and
/// the two still-possible containment directions.
struct CmpWalk<'a> {
    a: IdView<'a>,
    b: IdView<'a>,
    /// `a ⊇ b` still possible.
    le: bool,
    /// `b ⊇ a` still possible.
    ge: bool,
}

impl CmpWalk<'_> {
    /// Compare the subtrees rooted at the given positions, routing through the
    /// amortized stack-growth guard. Returns where each subtree ended (to thread
    /// the right sibling), or `None` to signal a decided incomparable that
    /// unwinds the whole walk.
    fn rec(&mut self, a_pos: usize, b_pos: usize, depth: usize) -> Option<(usize, usize)> {
        let a_hdr = self.a.header(a_pos);
        let b_hdr = self.b.header(b_pos);
        let (a_node, a_next) = (a_hdr.node, a_hdr.next);
        let (b_node, b_next) = (b_hdr.node, b_hdr.next);
        if a_node && b_node {
            // Both internal: descend in lockstep (left now, right threaded from
            // where the left ended). The node ends where its right subtree ends.
            let (a_mid, b_mid) = descend!(depth + 1, self.rec(a_next, b_next, depth + 1))?;
            return descend!(depth + 1, self.rec(a_mid, b_mid, depth + 1));
        }
        // At least one leaf: this region is decided. `a ⊇ b` holds iff `b` owns
        // nothing here or `a` owns everything; `b ⊇ a` is the mirror.
        self.le &= b_hdr.is_empty() || a_hdr.is_full();
        self.ge &= a_hdr.is_empty() || b_hdr.is_full();
        if !self.le && !self.ge {
            return None; // incomparable: neither containment can recover
        }
        // Resync: advance the leaf side past its header, skip the node side once.
        let a_end = if a_node { self.a.skip(a_pos) } else { a_next };
        let b_end = if b_node { self.b.skip(b_pos) } else { b_next };
        Some((a_end, b_end))
    }
}
