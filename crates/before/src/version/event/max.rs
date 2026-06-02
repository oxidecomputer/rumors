use crate::codec::Base;
use crate::recurse::descend;

use crate::version::compare::{EvHeader, EvView};

impl EvView<'_> {
    /// The maximum value of the event function over the subtree at `root` (the
    /// paper's `max`: `base + max(child maxes)`), and the position just past the
    /// subtree. `O(n)`.
    ///
    /// The recursive form of the paper's `max`, guarded by [`crate::recurse`] so
    /// deep trees grow the stack onto the heap rather than overflowing. The
    /// running path sum `off` is threaded by reference; the subtree maximum is
    /// returned (no mutable accumulator), and the threaded end position lets a
    /// right sibling resume without re-scanning.
    pub(super) fn max(&self, root: usize) -> (Base, usize) {
        let zero = Base::ZERO;
        descend!(0, max_rec(*self, root, &zero, 0))
    }
}

/// The maximum cumulative path sum over the subtree at `pos`, whose
/// root-to-parent path sum is `off`, plus the position just past the subtree.
/// Routed through the amortized stack-growth guard.
fn max_rec(view: EvView, pos: usize, off: &Base, depth: usize) -> (Base, usize) {
    let EvHeader {
        internal,
        base,
        next,
    } = view.header(pos);
    let cumulative = off + &base;
    if !internal {
        // Leaf: its cumulative path sum is the subtree maximum.
        return (cumulative, next);
    }
    // Internal: descend both children under this node's path sum (the right
    // threaded from where the left ended). A child's cumulative dominates the
    // node's own, so the subtree max is the larger child max.
    let (l_max, l_end) = descend!(depth + 1, max_rec(view, next, &cumulative, depth + 1));
    let (r_max, r_end) = descend!(depth + 1, max_rec(view, l_end, &cumulative, depth + 1));
    (l_max.max(r_max).max(cumulative), r_end)
}
