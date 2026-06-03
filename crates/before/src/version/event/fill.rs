use crate::codec::BitsSlice;
use crate::idbits::{IdNode, IdReader};
use crate::recurse::descend;

use crate::version::compare::{EvNode, EvReader};
use crate::version::working::WorkingVersion;

use super::Builder;

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
    pub(super) fn fill(self, id_bits: &BitsSlice) -> WorkingVersion {
        let mut walk = FillWalk {
            out: Builder::with_capacity(self.node_capacity_bound()),
        };
        descend!(0, walk.rec(IdReader::root(id_bits), self, 0));
        walk.out.finish()
    }
}

/// The single output builder of a [`fill`](EvReader::fill) walk; the readers carry
/// the traversal state.
struct FillWalk {
    out: Builder,
}

/// A filled subtree: the output root it produced, plus the readers past the id
/// and event inputs (so a right sibling resumes without re-scanning).
struct Filled<'a> {
    out_root: usize,
    id_end: IdReader<'a>,
    ev_end: EvReader<'a>,
}

impl FillWalk {
    /// Fill the event subtree at `ev` under the id subtree at `id`, emitting into
    /// `out` and routing through the amortized stack-growth guard. Returns the
    /// subtree's [`Filled`] report. Reads as the paper's `fill`:
    ///
    /// - `fill(0, e) = e`           — id empty: copy the event unchanged.
    /// - `fill(1, e) = max(e)`      — id full: collapse to a single max-leaf.
    /// - `fill((il,ir), Leaf n) = Leaf n` — event leaf under an id node: copy it,
    ///   lazy-skip the dominated id subtree.
    /// - `fill((il,ir), (n,el,er)) = norm((n, fill(il,el), fill(ir,er)))` — with
    ///   the two `is_full` shortcuts below.
    fn rec<'a>(&mut self, id: IdReader<'a>, ev: EvReader<'a>, depth: usize) -> Filled<'a> {
        let (id_node, il) = id.read();
        match id_node {
            IdNode::Empty => {
                let (out_root, ev_end) = self.out.copy_reader(ev);
                return Filled {
                    out_root,
                    id_end: il,
                    ev_end,
                };
            }
            IdNode::Full => {
                let (mx, ev_end) = ev.max();
                return Filled {
                    out_root: self.out.leaf(mx),
                    id_end: il,
                    ev_end,
                };
            }
            IdNode::Internal => {}
        }
        // id is an internal node; `il` is the reader at its left child.
        let (ev_node, el) = ev.read();
        let ev_base = match ev_node {
            EvNode::Leaf(n) => {
                return Filled {
                    out_root: self.out.leaf(n),
                    id_end: id.skip(),
                    ev_end: el,
                };
            }
            EvNode::Internal(base) => base,
        };

        // id node, event node. A fully-owned child collapses to a single leaf
        // valued `max(child events) ⊔ (sibling's filled base)` — raising the
        // owned side to meet the sibling, which is what lets the tree simplify.
        //
        // The two shortcuts are mirror images, but the preorder builder treats
        // them asymmetrically (intrinsic, not incidental): a collapsed *left*
        // child must be emitted before its right sibling exists, so it is a
        // [`deferred_leaf`](Builder::deferred_leaf) resolved after the right is
        // built; a collapsed *right* child is emitted after its left sibling, so
        // its value is already known.
        let (il_node, ir) = il.read();
        if let IdNode::Full = il_node {
            // `il` full: defer the collapsed left, fill the right, then resolve.
            // `ir` (past the `il` leaf) is the right id child; `el.max()` ends at
            // `er`, the right event child.
            let node = self.out.open(ev_base);
            let left = self.out.deferred_leaf();
            let (max_el, er) = el.max();
            let right = descend!(depth + 1, self.rec(ir, er, depth + 1));
            let value = max_el.max(self.out.base_of(right.out_root).clone());
            self.out.resolve(left, value);
            self.out.close_node(node, right.out_root);
            return Filled {
                out_root: node,
                id_end: right.id_end,
                ev_end: right.ev_end,
            };
        }
        // `il` not full: fill the left child first — only then is `ir`'s packed
        // position known (it is where the left subtree's id ended).
        let node = self.out.open(ev_base);
        let left = descend!(depth + 1, self.rec(il, el, depth + 1));
        let (ir_node, ir_after) = left.id_end.read();
        if let IdNode::Full = ir_node {
            // `ir` full: emit the collapsed right directly over the filled left.
            let (max_er, ev_end) = left.ev_end.max();
            let value = max_er.max(self.out.base_of(left.out_root).clone());
            let right_leaf = self.out.leaf(value);
            self.out.close_node(node, right_leaf);
            return Filled {
                out_root: node,
                id_end: ir_after,
                ev_end,
            };
        }
        let right = descend!(depth + 1, self.rec(left.id_end, left.ev_end, depth + 1));
        self.out.close_node(node, right.out_root);
        Filled {
            out_root: node,
            id_end: right.id_end,
            ev_end: right.ev_end,
        }
    }
}
