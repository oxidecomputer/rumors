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
