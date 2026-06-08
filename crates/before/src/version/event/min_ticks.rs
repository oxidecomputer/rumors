use crate::recurse::descend;

use crate::version::compare::EvReader;

impl EvReader<'_> {
    /// The sum of every base in this event subtree, saturating at [`u64::MAX`]:
    /// the minimum number of `tick`s that could have produced it (see
    /// [`Version::min_ticks`](crate::Version::min_ticks)). Advances the cursor
    /// past the subtree. `O(n)`.
    ///
    /// The sibling of [`max`](Self::max): both fold the event tree, but `max`
    /// takes the largest root-to-leaf path sum (a single causal chain), whereas
    /// this sums *every* base. A `tick` raises the stored base total by at most
    /// one (`grow` inflates one leaf; `fill` only sinks existing mass), so this
    /// total is the exact floor on the number of `tick`s — never below `max`,
    /// and strictly above it wherever the history forked.
    pub(in crate::version) fn min_ticks(&mut self) -> u64 {
        descend!(0, min_ticks_rec(self, 0))
    }
}

/// The saturating sum of all bases in the subtree at `ev`, advancing `ev` past
/// it. The recursive form, routed through the amortized stack-growth guard so a
/// deep tree grows the stack onto the heap rather than overflowing.
fn min_ticks_rec(ev: &mut EvReader, depth: usize) -> u64 {
    let node = ev.read();
    let base = node.base().to_u64_saturating();
    if !node.is_internal() {
        // Leaf: its own base is the whole subtree sum.
        return base;
    }
    // Internal: this node's base plus both children's sums (the `&mut` advances
    // through the left subtree, then the right resumes from it).
    let l = descend!(depth + 1, min_ticks_rec(ev, depth + 1));
    let r = descend!(depth + 1, min_ticks_rec(ev, depth + 1));
    base.saturating_add(l).saturating_add(r)
}
