use crate::codec::Base;
use crate::recurse::descend;

use crate::version::compare::EvReader;

impl EvReader<'_> {
    /// The maximum value of the event function over this subtree (the paper's
    /// `max`: `base + max(child maxes)`), advancing the cursor past the subtree.
    /// `O(n)`. A synthetic `Zero` subtree maxes to 0.
    ///
    /// The recursive form of the paper's `max`, guarded by [`crate::recurse`] so
    /// deep trees grow the stack onto the heap rather than overflowing. The
    /// running path sum `off` is threaded by reference; the subtree maximum is
    /// returned (no mutable accumulator), and the cursor advances in place so a
    /// right sibling resumes from it without re-scanning.
    pub(super) fn max(&mut self) -> Base {
        let zero = Base::ZERO;
        descend!(0, max_rec(self, &zero, 0))
    }
}

/// The maximum cumulative path sum over the subtree at `ev`, whose
/// root-to-parent path sum is `off`, advancing `ev` past the subtree. Routed
/// through the amortized stack-growth guard.
fn max_rec(ev: &mut EvReader, off: &Base, depth: usize) -> Base {
    let node = ev.read();
    let cumulative = off + node.base();
    if !node.is_internal() {
        // Leaf: its cumulative path sum is the subtree maximum.
        return cumulative;
    }
    // Internal: descend both children under this node's path sum (the `&mut`
    // advances through the left subtree, then the right resumes from it). A
    // child's cumulative dominates the node's own, so the subtree max is the
    // larger child max.
    let l_max = descend!(depth + 1, max_rec(ev, &cumulative, depth + 1));
    let r_max = descend!(depth + 1, max_rec(ev, &cumulative, depth + 1));
    l_max.max(r_max).max(cumulative)
}
