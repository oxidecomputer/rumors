use crate::codec::BitsSlice;
use crate::idbits::{IdNode, IdReader};
use crate::recurse::descend;

use crate::version::compare::{EvNode, EvReader};
use crate::version::working::WorkingVersion;

use super::{Builder, Slot};

impl EvReader<'_> {
    /// `fill(id, ev)`: use the available id to simplify this event tree
    /// (`self`) without registering a new event — wherever the id is full over
    /// a subtree, collapse that subtree to its maximum. Produces normal form.
    /// `O(n + m)`: the event drives (every event node visited once, threaded),
    /// and the id is lazy-skipped only where the event prunes it (an event leaf
    /// under an id node).
    ///
    /// The recursive form of `oracle::Version::fill` (the paper's `fill`): the
    /// walk reads as the oracle's `match (id, ev)`, threading an [`IdReader`] and
    /// an [`EvReader`] in place of bare positions. Guarded by [`crate::recurse`]
    /// so deep trees grow the stack onto the heap rather than overflowing.
    ///
    /// Worked example: `fill` of id `1` (fully owned) over event `(0, 2, 3)`
    /// collapses the whole subtree to its maximum, `0 + max(2, 3) = 3` — a single
    /// `Leaf(3)`. A *partial* id collapses only the subtrees it fully owns,
    /// raising each to its max (and to its sibling, where that lets the parent
    /// simplify) and leaving unowned subtrees in place — which is how a `tick`
    /// shrinks the tree when `fill` alone suffices.
    pub(super) fn fill(self, id_bits: &BitsSlice) -> WorkingVersion {
        let mut walk = FillWalk {
            out: Builder::with_capacity(self.node_capacity_bound()),
        };
        let mut ev = self;
        let mut id = IdReader::root(id_bits);
        descend!(0, walk.rec(&mut id, &mut ev, 0));
        walk.out.finish()
    }
}

/// The single output builder of a [`fill`](EvReader::fill) walk; the `&mut`
/// readers carry the traversal state.
struct FillWalk {
    out: Builder,
}

impl FillWalk {
    /// Fill the event subtree at `ev` under the id subtree at `id`, emitting into
    /// `out`, advancing both readers past their subtrees, and routing through the
    /// amortized stack-growth guard. Returns the output root. Reads as the
    /// paper's `fill`:
    ///
    /// - `fill(0, e) = e`           — id empty: copy the event unchanged.
    /// - `fill(1, e) = max(e)`      — id full: collapse to a single max-leaf.
    /// - `fill((il,ir), Leaf n) = Leaf n` — event leaf under an id node: copy it,
    ///   lazy-skip the dominated id subtree.
    /// - `fill((il,ir), (n,el,er)) = norm((n, fill(il,el), fill(ir,er)))` — with
    ///   the two `is_full` shortcuts below.
    fn rec(&mut self, id: &mut IdReader, ev: &mut EvReader, depth: usize) -> Slot {
        match id.read() {
            IdNode::Empty => return self.out.copy_reader(ev), // fill(0, e) = e
            IdNode::Full => return self.out.leaf(ev.max()).into(), // fill(1, e) = max(e)
            IdNode::Internal => {}                            // id now at `il`
        }
        let ev_base = match ev.read() {
            EvNode::Leaf(n) => {
                // fill((il,ir), Leaf n) = Leaf n: lazy-skip the dominated id
                // subtree (both children, `il` then `ir`).
                id.skip();
                id.skip();
                return self.out.leaf(n).into();
            }
            EvNode::Internal(base) => base, // ev now at `el`
        };

        // id node, event node. A fully-owned child collapses to a single leaf
        // valued `max(child events) ⊔ (sibling's filled base)` — raising the
        // owned side to meet the sibling, which is what lets the tree simplify.
        // [`peek`](IdReader::peek) tests a child's fullness without consuming it.
        //
        // The two shortcuts are mirror images, but the preorder builder treats
        // them asymmetrically (intrinsic, not incidental): a collapsed *left*
        // child must be emitted before its right sibling exists, so it is a
        // [`deferred_leaf`](Builder::deferred_leaf) resolved after the right is
        // built; a collapsed *right* child is emitted after its left sibling, so
        // its value is already known.
        if let IdNode::Full = id.peek() {
            // `il` full: defer the collapsed left, fill the right, then resolve.
            let node = self.out.open(ev_base);
            let left = self.out.deferred_leaf();
            id.skip(); // consume the `il` 1-leaf → id at `ir`
            let max_el = ev.max(); // ev past `el` → at `er`
            let right = descend!(depth + 1, self.rec(id, ev, depth + 1));
            let value = max_el.max(self.out.base_of(right).clone());
            self.out.resolve(left, value);
            return self.out.close_node(node, right);
        }
        // `il` not full: fill the left child (id → `ir`, ev → `er`), then check `ir`.
        let node = self.out.open(ev_base);
        let left = descend!(depth + 1, self.rec(id, ev, depth + 1));
        if let IdNode::Full = id.peek() {
            // `ir` full: emit the collapsed right directly over the filled left.
            id.skip(); // consume the `ir` 1-leaf
            let max_er = ev.max(); // ev past `er`
            let value = max_er.max(self.out.base_of(left).clone());
            let right_leaf = self.out.leaf(value);
            return self.out.close_node(node, right_leaf);
        }
        let right = descend!(depth + 1, self.rec(id, ev, depth + 1));
        self.out.close_node(node, right)
    }
}
