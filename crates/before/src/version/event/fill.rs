use crate::codec::BitsSlice;
use crate::idbits::IdView;
use crate::recurse::descend;

use crate::version::compare::{EvHeader, EvView};
use crate::version::working::WorkingVersion;

use super::{Builder, Built};

impl EvView<'_> {
    /// `fill(id, ev)`: use the available id to simplify this event tree
    /// (`self`) without registering a new event — wherever the id is full over
    /// a subtree, collapse that subtree to its maximum. Produces normal form.
    /// `O(n + m)`: the event drives (every event node visited once, threaded),
    /// and the id is lazy-skipped only where the event prunes it (an event leaf
    /// under an id node).
    ///
    /// The recursive form of `oracle::Version::fill` (the paper's `fill`),
    /// guarded by [`crate::recurse`] so deep trees grow the stack onto the heap
    /// rather than overflowing.
    pub(super) fn fill(&self, id_bits: &BitsSlice) -> WorkingVersion {
        let mut walk = FillWalk {
            view: *self,
            id: IdView(id_bits),
            out: Builder::with_capacity(self.node_capacity_bound()),
        };
        descend!(0, walk.rec(0, 0, 0));
        walk.out.finish()
    }
}

/// The mutable state of a [`fill`](EvView::fill) walk: the event view being
/// filled, the packed id driving the fill, and the single output builder.
struct FillWalk<'a> {
    view: EvView<'a>,
    id: IdView<'a>,
    out: Builder,
}

impl FillWalk<'_> {
    /// Fill the event subtree at `ev_pos` under the id subtree at `id_pos`,
    /// emitting into `out` and routing through the amortized stack-growth guard.
    /// Returns the subtree's [`Built`] report.
    fn rec(&mut self, id_pos: usize, ev_pos: usize, depth: usize) -> Built {
        let id_hdr = self.id.header(id_pos);
        let id_next = id_hdr.next;
        if id_hdr.is_empty() {
            // id 0-leaf: nothing owned here; the event is unchanged.
            let (root, ev_end) = self.out.copy(&self.view, ev_pos);
            return Built {
                out_root: root,
                id_end: id_next,
                ev_end,
            };
        }
        if id_hdr.is_full() {
            // id 1-leaf (full): collapse the whole event subtree to its max.
            let (mx, ev_end) = self.view.max(ev_pos);
            return Built {
                out_root: self.out.leaf(mx),
                id_end: id_next,
                ev_end,
            };
        }
        let EvHeader {
            internal: ev_int,
            base: ev_base,
            next: ev_next,
        } = self.view.header(ev_pos);
        if !ev_int {
            // id node over an event leaf: unchanged; lazy-skip the id subtree.
            return Built {
                out_root: self.out.leaf(ev_base),
                id_end: self.id.skip(id_pos),
                ev_end: ev_next,
            };
        }
        // id node, event node: the paper's
        // `fill((il,ir),(n,el,er)) = norm((n, fill(il,el), fill(ir,er)))`, plus
        // the two `is_full` shortcuts — a fully-owned child collapses to a
        // single leaf valued `max(child events) ⊔ (sibling's filled base)`.
        //
        // The two shortcuts are mirror images, but the preorder builder treats
        // them asymmetrically (this is intrinsic, not incidental): a collapsed
        // *left* child must be emitted before its right sibling exists, so it is
        // a [`deferred_leaf`](Builder::deferred_leaf) resolved after the right is
        // built; a collapsed *right* child is emitted after its left sibling, so
        // its value is already known.
        let id_left = id_next;
        let id_left_hdr = self.id.header(id_left);
        if id_left_hdr.is_full() {
            // `il` full: defer the collapsed left, fill the right, then resolve.
            let node = self.out.open(ev_base);
            let left = self.out.deferred_leaf();
            let (max_el, ev_right) = self.view.max(ev_next);
            let right = descend!(depth + 1, self.rec(id_left_hdr.next, ev_right, depth + 1));
            let value = max_el.max(self.out.base_of(right.out_root).clone());
            self.out.resolve(left, value);
            self.out.close_node(node, right.out_root);
            return Built {
                out_root: node,
                id_end: right.id_end,
                ev_end: right.ev_end,
            };
        }
        // `il` not full: fill the left child first — only then is `ir`'s packed
        // position known (it is where the left subtree's id ended).
        let node = self.out.open(ev_base);
        let left = descend!(depth + 1, self.rec(id_left, ev_next, depth + 1));
        let ir_hdr = self.id.header(left.id_end);
        if ir_hdr.is_full() {
            // `ir` full: emit the collapsed right directly over the filled left.
            let (max_er, ev_end) = self.view.max(left.ev_end);
            let value = max_er.max(self.out.base_of(left.out_root).clone());
            let right_leaf = self.out.leaf(value);
            self.out.close_node(node, right_leaf);
            return Built {
                out_root: node,
                id_end: ir_hdr.next,
                ev_end,
            };
        }
        let right = descend!(depth + 1, self.rec(left.id_end, left.ev_end, depth + 1));
        self.out.close_node(node, right.out_root);
        Built {
            out_root: node,
            id_end: right.id_end,
            ev_end: right.ev_end,
        }
    }
}
