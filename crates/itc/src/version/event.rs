//! The event-tree mutation core (plan §6 Phase 5): `merge` (event-tree join) and
//! `tick` (= `fill`, else `grow`, the latter in the [`grow`] submodule). Everything
//! operates on the fixed-width working form and walks the packed id ([`idbits`])
//! alongside it where needed.
//!
//! All three are iterative and `O(n + m)` in their inputs. Output is built into fresh
//! `topo`/`base` arrays in preorder via a [`Builder`] — the one type that owns event
//! normalization, so every emitting walk stays normal-form-correct for free (the id
//! side's analogue is the `id_node`/`id_leaf` pair in `party::ops`, which needs no
//! working form to thread through). Normalization is the constant "sink" — pushing the
//! children's common minimum up to the parent — done as an `O(1)` base backpatch
//! ([`Builder::close_node`]) the moment a node's children are known, exactly the
//! back-reference the fixed-width form exists for.
//!
//! # The thread register
//!
//! Every two-tree machine here ([`ev_join`], [`fill`], and the [`grow`] submodule's
//! probe and emit) — and `causal_cmp` in [`super::compare`] next door — drives a single
//! iterative DFS off an explicit job stack, threading right-child positions instead of
//! re-scanning to find them. They all speak the same protocol, the **thread register**:
//!
//! - A mutable `ret`, a small named struct, holds the just-finished subtree's report:
//!   the position just past it in each input tree, plus a per-walk payload — the output
//!   root it produced ([`Joined`] for the join, [`Built`] shared by `fill` and the grow
//!   emit), the subtree's cost (`grow`'s `Probed`), or nothing (`compare`'s `Ends`).
//! - Every `Eval` arm finishes by *writing* `ret` (a completed leaf, or a `Close`/
//!   `Combine` arm folding two children).
//! - Every deferred-sibling frame (`Right`/`Close`/`Combine`) *reads* `ret` to resume:
//!   a right child starts where its left sibling's subtree ended, so it never re-scans.
//!
//! LIFO push order is what makes the bare register sound: a node pushes its `Close`
//! frame, then its `Right` frame, then its left `Eval`, so by the time a frame pops and
//! reads `ret`, the most recent write is exactly the sibling subtree it is waiting on.
//! (`sum` in `party::ops` plays the same role with a `Vec` of its `Summed` register,
//! since it must combine two child *outputs*, not just their positions.)

use crate::codec::{Bits, BitsSlice};
use crate::idbits;

use super::compare::EvView;
use super::working::WorkingVersion;

mod grow;

/// Sentinel event position: a virtual `Leaf(0)`, used by [`grow`] when it expands an
/// event leaf into a node to follow the id deeper. Never a real bit offset.
pub(super) const VIRTUAL: usize = usize::MAX;

// ───────────────────────────── output builder ─────────────────────────────

/// Accumulates the output event tree in preorder. A node's base is written as a
/// placeholder when the node opens and finalized by [`close_node`](Self::close_node)
/// once its children are in place. This is the canonical output path shared by every
/// emitting walk (`ev_join`, `fill`, the [`grow`] emit); it is the single place event
/// normalization lives, so callers never re-implement the sink/collapse.
pub(super) struct Builder {
    topo: Bits,
    base: Vec<u64>,
}

impl Builder {
    pub(super) fn new() -> Self {
        Builder {
            topo: Bits::new(),
            base: Vec::new(),
        }
    }

    fn len(&self) -> usize {
        self.base.len()
    }

    /// Append a leaf with the given base; return its index.
    pub(super) fn leaf(&mut self, base: u64) -> usize {
        let i = self.len();
        self.topo.push(false);
        self.base.push(base);
        i
    }

    /// Open an internal node with a placeholder base; its children are appended next.
    /// Return its index.
    pub(super) fn open(&mut self, base: u64) -> usize {
        let i = self.len();
        self.topo.push(true);
        self.base.push(base);
        i
    }

    /// Copy the subtree at `root` of `src` verbatim (it is already normalized); return
    /// `(new_root, src_end)` — its index here and the position just past it in `src`.
    /// Iterative single pass.
    pub(super) fn copy(&mut self, src: &EvView, root: usize) -> (usize, usize) {
        let out_root = self.len();
        let mut pos = root;
        let mut pending: i64 = 1;
        while pending > 0 {
            let (internal, base, next) = src.header(pos);
            self.topo.push(internal);
            self.base.push(base);
            pos = next;
            pending += if internal { 1 } else { -1 };
        }
        (out_root, pos)
    }

    /// Finalize the internal node at `node` whose left child is at `node + 1` and right
    /// child at `right`. Sinks the children's common minimum into the node's base
    /// (`O(1)`) and collapses `(n, m, m)` of two equal leaves to a single leaf,
    /// preserving normal form. The node's root index is unchanged.
    ///
    /// Adjacency precondition for the collapse: it fires only when *both* children are
    /// leaves (the `!self.topo[..]` guards). A leaf occupies exactly one slot, so the
    /// left child is `node + 1` and the right child is `node + 2 == right` — i.e.
    /// `[node, left, right]` are the final three slots in `topo`/`base`. That is why
    /// `truncate(node)` discards exactly those three and nothing earlier before pushing
    /// the single collapsed leaf in their place.
    pub(super) fn close_node(&mut self, node: usize, right: usize) {
        let left = node + 1;
        let m = self.base[left].min(self.base[right]);
        self.base[node] += m;
        self.base[left] -= m;
        self.base[right] -= m;
        // Collapse only when both children are leaves of equal (post-sink) base.
        if !self.topo[left] && !self.topo[right] && self.base[left] == self.base[right] {
            let collapsed = self.base[node]; // the common child base is 0 after the sink
            self.topo.truncate(node);
            self.base.truncate(node);
            self.leaf(collapsed);
        }
    }

    pub(super) fn finish(self) -> WorkingVersion {
        WorkingVersion {
            topo: self.topo,
            base: self.base,
        }
    }
}

// ───────────────────────────── merge (event-tree join) ─────────────────────────────

/// A step in the threaded two-tree `ev_join` walk. `ret` is the [`Joined`] register (see
/// the module doc). Positions (`*_pos`) address a node in each input; offsets (`*_off`)
/// are the path sum down to it; sums (`*_sum`) add the node's own base; `*_internal`
/// records whether that side was a node (so its right child threads) or a leaf (so it
/// re-broadcasts in place).
enum JoinJob {
    /// Join the subtrees at these positions, under these path-sum offsets.
    Eval {
        /// Position of `a`'s subtree root.
        a_pos: usize,
        /// Path sum down to `a`'s subtree.
        a_off: u64,
        /// Position of `b`'s subtree root.
        b_pos: usize,
        /// Path sum down to `b`'s subtree.
        b_off: u64,
    },
    /// Left child finished; launch the right child (threading each internal side from
    /// `ret`, re-broadcasting each leaf side from the carried position/offset).
    Right {
        /// Whether `a` was a node here (its right child threads) or a leaf (re-broadcast).
        a_internal: bool,
        /// `a`'s node sum, the offset for a threaded right child.
        a_sum: u64,
        /// `a`'s pinned position, reused when `a` is a leaf.
        a_pos: usize,
        /// `a`'s pinned offset, reused when `a` is a leaf.
        a_off: u64,
        /// Whether `b` was a node here (its right child threads) or a leaf (re-broadcast).
        b_internal: bool,
        /// `b`'s node sum, the offset for a threaded right child.
        b_sum: u64,
        /// `b`'s pinned position, reused when `b` is a leaf.
        b_pos: usize,
        /// `b`'s pinned offset, reused when `b` is a leaf.
        b_off: u64,
    },
    /// Right child finished; sink and close the node, reporting its end positions. The
    /// end is the threaded child end when that side descended, else its pinned `*_next`.
    Close {
        /// Output index of the node being closed.
        node: usize,
        /// Whether `a` descended (use the threaded end) or stayed a leaf (use `a_next`).
        a_internal: bool,
        /// `a`'s end position when it was a leaf here.
        a_next: usize,
        /// Whether `b` descended (use the threaded end) or stayed a leaf (use `b_next`).
        b_internal: bool,
        /// `b`'s end position when it was a leaf here.
        b_next: usize,
    },
}

/// The thread register for `ev_join` (see the module doc): the output root a
/// just-finished subtree produced, plus where it ended in each input. An `Eval` arm
/// *writes* it (a leaf directly, or via the `Close` arm folding a node); the deferred
/// `Right`/`Close` frames *read* it.
#[derive(Clone, Copy, Default)]
struct Joined {
    /// Output index of the subtree's root.
    out_root: usize,
    /// Position just past the subtree in `a`.
    a_end: usize,
    /// Position just past the subtree in `b`.
    b_end: usize,
}

/// The least upper bound of two event trees (the paper's `join` over event trees),
/// produced in normal form. Reads either storage form via [`EvView`]; `O(n + m)`.
pub(crate) fn ev_join(a: &EvView, b: &EvView) -> WorkingVersion {
    let mut out = Builder::new();
    let mut ret = Joined::default();
    let mut stack = vec![JoinJob::Eval {
        a_pos: 0,
        a_off: 0,
        b_pos: 0,
        b_off: 0,
    }];
    while let Some(job) = stack.pop() {
        match job {
            JoinJob::Eval {
                a_pos,
                a_off,
                b_pos,
                b_off,
            } => {
                let (a_internal, a_base, a_next) = a.header(a_pos);
                let (b_internal, b_base, b_next) = b.header(b_pos);
                let a_sum = a_off + a_base;
                let b_sum = b_off + b_base;
                if !a_internal && !b_internal {
                    ret = Joined {
                        out_root: out.leaf(a_sum.max(b_sum)),
                        a_end: a_next,
                        b_end: b_next,
                    };
                    continue;
                }
                let node = out.open(0);
                // Left children: an internal side descends; a leaf side broadcasts in
                // place (reuse its position/offset, so its value stays `a_sum`/`b_sum`).
                let (left_a_pos, left_a_off) = if a_internal {
                    (a_next, a_sum)
                } else {
                    (a_pos, a_off)
                };
                let (left_b_pos, left_b_off) = if b_internal {
                    (b_next, b_sum)
                } else {
                    (b_pos, b_off)
                };
                stack.push(JoinJob::Close {
                    node,
                    a_internal,
                    a_next,
                    b_internal,
                    b_next,
                });
                stack.push(JoinJob::Right {
                    a_internal,
                    a_sum,
                    a_pos,
                    a_off,
                    b_internal,
                    b_sum,
                    b_pos,
                    b_off,
                });
                stack.push(JoinJob::Eval {
                    a_pos: left_a_pos,
                    a_off: left_a_off,
                    b_pos: left_b_pos,
                    b_off: left_b_off,
                });
            }
            JoinJob::Right {
                a_internal,
                a_sum,
                a_pos,
                a_off,
                b_internal,
                b_sum,
                b_pos,
                b_off,
            } => {
                let (right_a_pos, right_a_off) = if a_internal {
                    (ret.a_end, a_sum)
                } else {
                    (a_pos, a_off)
                };
                let (right_b_pos, right_b_off) = if b_internal {
                    (ret.b_end, b_sum)
                } else {
                    (b_pos, b_off)
                };
                stack.push(JoinJob::Eval {
                    a_pos: right_a_pos,
                    a_off: right_a_off,
                    b_pos: right_b_pos,
                    b_off: right_b_off,
                });
            }
            JoinJob::Close {
                node,
                a_internal,
                a_next,
                b_internal,
                b_next,
            } => {
                out.close_node(node, ret.out_root);
                ret = Joined {
                    out_root: node,
                    a_end: if a_internal { ret.a_end } else { a_next },
                    b_end: if b_internal { ret.b_end } else { b_next },
                };
            }
        }
    }
    out.finish()
}

/// An open ancestor on the [`ev_max`] descent stack: a node whose subtree is still
/// being summed.
struct Ancestor {
    /// The path sum from the root down to and including this node.
    cumulative: u64,
    /// How many of this node's two children are not yet finished (2, then 1, then pop).
    children_left: u8,
}

/// The maximum value of the event function over the subtree at `root` (the paper's
/// `max`: `base + max(child maxes)`), and the position just past the subtree. Iterative
/// linear pass — a per-ancestor cumulative/remaining stack, no right-child re-scan.
fn ev_max(view: &EvView, root: usize) -> (u64, usize) {
    let mut max = 0u64;
    let mut pos = root;
    let mut stack: Vec<Ancestor> = Vec::new();
    loop {
        let offset = stack.last().map_or(0, |a| a.cumulative);
        let (internal, base, next) = view.header(pos);
        let cumulative = offset + base;
        max = max.max(cumulative);
        pos = next;
        if internal {
            stack.push(Ancestor {
                cumulative,
                children_left: 2,
            });
        } else {
            // A leaf completes; pop every ancestor whose children are now all done.
            loop {
                match stack.last_mut() {
                    None => return (max, pos),
                    Some(ancestor) => {
                        ancestor.children_left -= 1;
                        if ancestor.children_left == 0 {
                            stack.pop();
                        } else {
                            break;
                        }
                    }
                }
            }
        }
    }
}

// ───────────────────────────── fill ─────────────────────────────

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
        max_el: u64,
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

/// The thread register for the id-driven emitting walks — `fill` here and the `grow`
/// emit (see the module doc): the output root a just-finished subtree produced, plus
/// where it ended in the packed id stream and the event tree. An `Eval` arm *writes* it
/// (a leaf directly, or via a `*Close` arm folding a node); deferred frames *read* it.
#[derive(Clone, Copy, Default)]
pub(super) struct Built {
    /// Output index of the subtree's root.
    pub(super) out_root: usize,
    /// Position just past the subtree in the packed id stream.
    pub(super) id_end: usize,
    /// Position just past the subtree in the event tree.
    pub(super) ev_end: usize,
}

/// `fill(id, ev)` (plan Appendix A): use the available id to simplify the event tree
/// without registering a new event — wherever the id is full over a subtree, collapse
/// that subtree to its maximum. Produces normal form. Iterative, `O(n + m)`: the event
/// drives (every event node visited once, threaded), and the id is lazy-skipped only
/// where the event prunes it (an event leaf under an id node).
fn fill(id_bits: &BitsSlice, view: &EvView) -> WorkingVersion {
    let mut out = Builder::new();
    let mut ret = Built::default();
    let mut stack = vec![FillJob::Eval {
        id_pos: 0,
        ev_pos: 0,
    }];
    while let Some(job) = stack.pop() {
        match job {
            FillJob::Eval { id_pos, ev_pos } => {
                let (id_node, id_val, id_next) = idbits::header(id_bits, id_pos);
                if !id_node && !id_val {
                    // id 0-leaf: nothing owned here; the event is unchanged.
                    let (root, ev_end) = out.copy(view, ev_pos);
                    ret = Built {
                        out_root: root,
                        id_end: id_next,
                        ev_end,
                    };
                    continue;
                }
                if !id_node {
                    // id 1-leaf (full): collapse the whole event subtree to its max.
                    let (mx, ev_end) = ev_max(view, ev_pos);
                    ret = Built {
                        out_root: out.leaf(mx),
                        id_end: id_next,
                        ev_end,
                    };
                    continue;
                }
                let (ev_int, ev_base, ev_next) = view.header(ev_pos);
                if !ev_int {
                    // id node over an event leaf: unchanged; lazy-skip the id subtree.
                    ret = Built {
                        out_root: out.leaf(ev_base),
                        id_end: idbits::skip(id_bits, id_pos),
                        ev_end: ev_next,
                    };
                    continue;
                }
                // id node, event node.
                let (id_left, ev_left) = (id_next, ev_next);
                if idbits::is_full(id_bits, id_left) {
                    // `il` full: left collapses to a leaf whose value depends on the
                    // filled right; build the right first, then backpatch the leaf.
                    let node = out.open(ev_base);
                    let left_leaf = out.leaf(0); // placeholder
                    let (max_el, ev_right) = ev_max(view, ev_left);
                    let id_right = id_left + 2; // past the 1-leaf `il`
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
                out.base[left_leaf] = max_el.max(out.base[right.out_root]);
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
                if idbits::is_full(id_bits, ir) {
                    // `ir` full: right collapses to a leaf depending on the filled left.
                    let (max_er, er_end) = ev_max(view, er);
                    let x = max_er.max(out.base[left.out_root]);
                    let right_leaf = out.leaf(x);
                    out.close_node(node, right_leaf);
                    ret = Built {
                        out_root: node,
                        id_end: ir + 2, // past the 1-leaf `ir`
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

// ───────────────────────────── tick (fill, else grow) ─────────────────────────────

/// Advance `id`'s component of the event tree by one event. `fill` first (it may
/// simplify the tree using the available id); if it changes nothing, `grow`. The id is
/// the packed `enc_id` stream; `ev` is the current working form. `O(n + m)`.
pub(crate) fn tick(id: &BitsSlice, ev: &WorkingVersion) -> WorkingVersion {
    let view = EvView::Working(ev);
    let filled = fill(id, &view);
    if filled.topo != ev.topo || filled.base != ev.base {
        filled
    } else {
        grow::grow(id, &view)
    }
}
