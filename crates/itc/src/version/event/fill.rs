use crate::codec::{Base, BitsSlice};
use crate::idbits::IdView;

use crate::version::compare::{EvHeader, EvView};
use crate::version::working::WorkingVersion;

use super::{Builder, Built};

/// A step in the threaded `fill` walk. `ret` is the [`Built`] register (see the module
/// doc). `id_pos` is a bit offset into the packed id stream; `ev_pos` a position in the
/// event tree being filled.
enum FillJob {
    /// Fill the event subtree at `ev_pos` under the id subtree at `id_pos`.
    Eval {
        /// Position into the packed id stream.
        id_pos: usize,
        /// Position into the event tree.
        ev_pos: usize,
    },
    /// `il` is full: the right child (the filled `er`) is being built; afterwards set
    /// the collapsed left leaf to `max(max_ev(el), min(er'))` and close.
    FullLeftClose {
        /// Output index of the node being filled.
        node: usize,
        /// Output index of the placeholder left leaf to backpatch.
        left_leaf: usize,
        /// `max_ev(el)`: the maximum of the (full-id-collapsed) left event subtree.
        max_el: Base,
    },
    /// `il` is not full: the left child (filled `el`) is being built; afterwards decide
    /// the right child by whether `ir` is full.
    AfterLeft {
        /// Output index of the node being filled.
        node: usize,
    },
    /// Right child (filled `er`) is being built for the general case; afterwards close.
    GeneralClose {
        /// Output index of the node being filled.
        node: usize,
    },
}

impl EvView<'_> {
    /// `fill(id, ev)`: use the available id to simplify this event tree (`self`) without
    /// registering a new event — wherever the id is full over a subtree, collapse that
    /// subtree to its maximum. Produces normal form. Iterative, `O(n + m)`: the event
    /// drives (every event node visited once, threaded), and the id is lazy-skipped only
    /// where the event prunes it (an event leaf under an id node).
    ///
    /// The iterative form of the recursive `oracle::Version::fill` (the paper's `fill`); read
    /// that recursive twin first, then this is the same algorithm with the call stack made
    /// explicit.
    pub(super) fn fill(&self, id_bits: &BitsSlice) -> WorkingVersion {
        let view = self;
        let id = IdView(id_bits);
        let mut out = Builder::with_capacity(view.node_capacity_bound());
        let mut ret = Built::default();
        let mut stack = vec![FillJob::Eval {
            id_pos: 0,
            ev_pos: 0,
        }];
        while let Some(job) = stack.pop() {
            match job {
                FillJob::Eval { id_pos, ev_pos } => {
                    let id_hdr = id.header(id_pos);
                    let id_next = id_hdr.next;
                    if id_hdr.is_empty() {
                        // id 0-leaf: nothing owned here; the event is unchanged.
                        let (root, ev_end) = out.copy(view, ev_pos);
                        ret = Built {
                            out_root: root,
                            id_end: id_next,
                            ev_end,
                        };
                        continue;
                    }
                    if id_hdr.is_full() {
                        // id 1-leaf (full): collapse the whole event subtree to its max.
                        let (mx, ev_end) = view.max(ev_pos);
                        ret = Built {
                            out_root: out.leaf(mx),
                            id_end: id_next,
                            ev_end,
                        };
                        continue;
                    }
                    let EvHeader {
                        internal: ev_int,
                        base: ev_base,
                        next: ev_next,
                    } = view.header(ev_pos);
                    if !ev_int {
                        // id node over an event leaf: unchanged; lazy-skip the id subtree.
                        ret = Built {
                            out_root: out.leaf(ev_base),
                            id_end: id.skip(id_pos),
                            ev_end: ev_next,
                        };
                        continue;
                    }
                    // id node, event node.
                    let (id_left, ev_left) = (id_next, ev_next);
                    let id_left_hdr = id.header(id_left);
                    if id_left_hdr.is_full() {
                        // `il` full: left collapses to a leaf whose value depends on the
                        // filled right; build the right first, then backpatch the leaf.
                        let node = out.open(ev_base);
                        let left_leaf = out.leaf(Base::ZERO); // placeholder
                        let (max_el, ev_right) = view.max(ev_left);
                        let id_right = id_left_hdr.next; // past the 1-leaf `il`
                        stack.push(FillJob::FullLeftClose {
                            node,
                            left_leaf,
                            max_el,
                        });
                        stack.push(FillJob::Eval {
                            id_pos: id_right,
                            ev_pos: ev_right,
                        });
                    } else {
                        // `il` not full: fill the left child first; decide the right after.
                        let node = out.open(ev_base);
                        stack.push(FillJob::AfterLeft { node });
                        stack.push(FillJob::Eval {
                            id_pos: id_left,
                            ev_pos: ev_left,
                        });
                    }
                }
                FillJob::FullLeftClose {
                    node,
                    left_leaf,
                    max_el,
                } => {
                    let right = ret; // the filled right child's report
                    out.base[left_leaf] = max_el.max(out.base[right.out_root].clone());
                    out.close_node(node, right.out_root);
                    ret = Built {
                        out_root: node,
                        id_end: right.id_end,
                        ev_end: right.ev_end,
                    };
                }
                FillJob::AfterLeft { node } => {
                    let left = ret; // the filled left child's report
                    let (ir, er) = (left.id_end, left.ev_end);
                    let ir_hdr = id.header(ir);
                    if ir_hdr.is_full() {
                        // `ir` full: right collapses to a leaf depending on the filled left.
                        let (max_er, er_end) = view.max(er);
                        let x = max_er.max(out.base[left.out_root].clone());
                        let right_leaf = out.leaf(x);
                        out.close_node(node, right_leaf);
                        ret = Built {
                            out_root: node,
                            id_end: ir_hdr.next, // past the 1-leaf `ir`
                            ev_end: er_end,
                        };
                    } else {
                        stack.push(FillJob::GeneralClose { node });
                        stack.push(FillJob::Eval {
                            id_pos: ir,
                            ev_pos: er,
                        });
                    }
                }
                FillJob::GeneralClose { node } => {
                    let right = ret; // the filled right child's report
                    out.close_node(node, right.out_root);
                    ret = Built {
                        out_root: node,
                        id_end: right.id_end,
                        ev_end: right.ev_end,
                    };
                }
            }
        }
        out.finish()
    }
}
