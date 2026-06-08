use crate::idbits::{IdNode, IdReader};
use crate::recurse::descend;

impl IdReader<'_> {
    /// Whether `self` and `other` (normal-form ids) share no owned region. `O(n + m)`: both
    /// cursors are threaded, and a side is skipped only where the other's leaf dominates it.
    ///
    /// Recursive form of the paper's region-disjointness test, guarded by
    /// [`crate::recurse`] so deep ids grow the stack onto the heap rather than
    /// overflowing.
    // Takes the cursors by value: a reader is single-use, and the walk consumes
    // both. (`is_*`-by-value is unusual, hence the allow.)
    #[allow(clippy::wrong_self_convention)]
    pub(crate) fn is_disjoint(mut self, mut other: IdReader) -> bool {
        descend!(0, disjoint_rec(&mut self, &mut other, 0))
    }

    /// Whether `self` (a normal-form id) *covers* `other` — every region `other`
    /// owns is also owned by `self` (`self ⊇ other`). `O(n + m)`: both cursors
    /// are threaded, and a side is skipped only where the other's leaf dominates
    /// it, exactly as in [`is_disjoint`](IdReader::is_disjoint).
    ///
    /// Guarded by [`crate::recurse`] so deep ids grow the stack onto the heap
    /// rather than overflowing.
    // Single-use by-value readers, as with `is_disjoint`.
    #[allow(clippy::wrong_self_convention)]
    pub(crate) fn covers(mut self, mut other: IdReader) -> bool {
        descend!(0, covers_rec(&mut self, &mut other, 0))
    }
}

/// One subtree of the [`covers`](IdReader::covers) walk, advancing both `&mut`
/// readers past their subtrees; `false` the moment an uncovered region is found
/// unwinds the whole walk (the `&&` short-circuits). The asymmetric counterpart
/// of [`disjoint_rec`]: a full `self` dominates anything (skip the other to
/// resync); an empty `other` is covered by anything (skip the rest of `self`);
/// an empty `self` against a nonempty `other`, or a node `self` against a full
/// `other`, is a region `self` lacks; two nodes descend.
fn covers_rec(a: &mut IdReader, b: &mut IdReader, depth: usize) -> bool {
    let a_node = a.read();
    if let IdNode::Full = a_node {
        b.skip(); // a owns everything here: it covers whatever b is; resync past b
        return true;
    }
    let b_node = b.read();
    if let IdNode::Empty = b_node {
        // b owns nothing here: trivially covered. Skip the rest of a's subtree
        // (its two children) if a is a node; a leaf is already consumed.
        if let IdNode::Internal = a_node {
            a.skip();
            a.skip();
        }
        return true;
    }
    match (a_node, b_node) {
        // Both internal: a covers b iff it covers b on both halves.
        (IdNode::Internal, IdNode::Internal) => {
            descend!(depth + 1, covers_rec(a, b, depth + 1))
                && descend!(depth + 1, covers_rec(a, b, depth + 1))
        }
        // A region b owns that a does not: a empty under a nonempty b, or a
        // node under a full b (a owns only part of what b owns in full).
        _ => false,
    }
}

/// One subtree of the [`is_disjoint`](IdReader::is_disjoint) walk, advancing
/// both `&mut` readers past their subtrees; `false` the moment an overlap is
/// found unwinds the whole walk (the `&&` short-circuits). Reads as a match on
/// the two id nodes: an empty side is disjoint from anything (skip the other to
/// resync); a full side overlaps any nonempty other; two nodes descend.
fn disjoint_rec(a: &mut IdReader, b: &mut IdReader, depth: usize) -> bool {
    let a_node = a.read();
    if let IdNode::Empty = a_node {
        b.skip(); // a owns nothing here: disjoint; skip b's subtree to resync
        return true;
    }
    let b_node = b.read();
    if let IdNode::Empty = b_node {
        // b owns nothing: disjoint. Skip the rest of a's subtree (its two
        // children) if a is a node; a leaf is already consumed.
        if let IdNode::Internal = a_node {
            a.skip();
            a.skip();
        }
        return true;
    }
    match (a_node, b_node) {
        // Both internal: descend in lockstep, each cursor threaded through its
        // left subtree then its right.
        (IdNode::Internal, IdNode::Internal) => {
            descend!(depth + 1, disjoint_rec(a, b, depth + 1))
                && descend!(depth + 1, disjoint_rec(a, b, depth + 1))
        }
        // One side full, the other nonempty (neither is empty): overlap.
        _ => false,
    }
}
