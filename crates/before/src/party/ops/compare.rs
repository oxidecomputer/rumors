use crate::idbits::{IdNode, IdReader};
use crate::recurse::descend;

impl IdReader<'_> {
    /// Whether `self` and `other` (normal-form ids) share no owned region. `O(n + m)`: both
    /// cursors are threaded, and a side is skipped only where the other's leaf dominates it.
    ///
    /// Recursive form of the paper's region-disjointness test, guarded by
    /// [`crate::recurse`] so deep ids grow the stack onto the heap rather than
    /// overflowing.
    pub(crate) fn is_disjoint(self, other: IdReader) -> bool {
        // Each subtree walk returns where it ended in both inputs (so a right
        // sibling resumes without re-scanning), or `None` the instant an overlap
        // is found, unwinding the whole walk.
        descend!(0, disjoint_rec(self, other, 0)).is_some()
    }
}

/// One subtree of the [`is_disjoint`](IdReader::is_disjoint) walk. Returns readers
/// past the subtree in each input (to thread the right sibling), or `None` the
/// moment an overlap is found, which unwinds the whole walk. Reads as a match on
/// the two id nodes: an empty side is disjoint from anything (skip the other to
/// resync); a full side overlaps any nonempty other; two nodes descend.
fn disjoint_rec<'a>(
    a: IdReader<'a>,
    b: IdReader<'a>,
    depth: usize,
) -> Option<(IdReader<'a>, IdReader<'a>)> {
    let (a_node, a_after) = a.read();
    let (b_node, b_after) = b.read();
    match (a_node, b_node) {
        // An empty side owns nothing here: disjoint. Skip the other's subtree to
        // resync the cursors.
        (IdNode::Empty, _) => Some((a_after, b.skip())),
        (_, IdNode::Empty) => Some((a.skip(), b_after)),
        // One side full, the other nonempty (neither is empty): overlap.
        (IdNode::Full, _) | (_, IdNode::Full) => None,
        // Both internal: descend in lockstep, threading the right child from
        // where the left ended.
        (IdNode::Internal, IdNode::Internal) => {
            let (a_mid, b_mid) = descend!(depth + 1, disjoint_rec(a_after, b_after, depth + 1))?;
            descend!(depth + 1, disjoint_rec(a_mid, b_mid, depth + 1))
        }
    }
}
