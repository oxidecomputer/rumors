use crate::codec::Base;
use crate::recurse::descend;

use crate::version::compare::EvReader;

impl<'a> EvReader<'a> {
    /// The maximum value of the event function over this subtree (the paper's
    /// `max`: `base + max(child maxes)`), and a reader positioned just past the
    /// subtree. `O(n)`. A synthetic `Zero` subtree maxes to 0.
    ///
    /// The recursive form of the paper's `max`, guarded by [`crate::recurse`] so
    /// deep trees grow the stack onto the heap rather than overflowing. The
    /// running path sum `off` is threaded by reference; the subtree maximum is
    /// returned (no mutable accumulator), and the threaded end reader lets a
    /// right sibling resume without re-scanning.
    pub(super) fn max(self) -> (Base, EvReader<'a>) {
        let zero = Base::ZERO;
        descend!(0, max_rec(self, &zero, 0))
    }
}

/// The maximum cumulative path sum over the subtree at `ev`, whose
/// root-to-parent path sum is `off`, plus a reader just past the subtree.
/// Routed through the amortized stack-growth guard.
fn max_rec<'a>(ev: EvReader<'a>, off: &Base, depth: usize) -> (Base, EvReader<'a>) {
    let (node, after) = ev.read();
    let cumulative = off + node.base();
    if !node.is_internal() {
        // Leaf: its cumulative path sum is the subtree maximum.
        return (cumulative, after);
    }
    // Internal: descend both children under this node's path sum (the right
    // threaded from where the left ended). A child's cumulative dominates the
    // node's own, so the subtree max is the larger child max.
    let (l_max, l_end) = descend!(depth + 1, max_rec(after, &cumulative, depth + 1));
    let (r_max, r_end) = descend!(depth + 1, max_rec(l_end, &cumulative, depth + 1));
    (l_max.max(r_max).max(cumulative), r_end)
}
