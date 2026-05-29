//! The event-tree mutation core (plan §6 Phase 5): `merge` (event-tree join) and
//! `tick` (= `fill`, else `grow`). Everything operates on the fixed-width working form
//! and walks the packed id ([`idbits`]) alongside it where needed.
//!
//! All three are iterative and `O(n + m)` in their inputs. Output is built into fresh
//! `topo`/`base` arrays in preorder via a [`Builder`]; normalization is the constant
//! "sink" — pushing the children's common minimum up to the parent — done as an `O(1)`
//! base backpatch ([`Builder::close_node`]) the moment a node's children are known,
//! exactly the back-reference the fixed-width form exists for. Right-child positions
//! are threaded (discovered when the left subtree's walk ends), and the packed id is
//! lazy-skipped only where the event prunes it — so no traversal re-scans.

use crate::codec::{Bits, BitsSlice};
use crate::idbits;

use super::compare::{skip as ev_skip, EvView};
use super::working::WorkingVersion;

/// Sentinel event position: a virtual `Leaf(0)`, used by `grow` when it expands an
/// event leaf into a node to follow the id deeper. Never a real bit offset.
const VIRTUAL: usize = usize::MAX;

// ───────────────────────────── output builder ─────────────────────────────

/// Accumulates the output event tree in preorder. A node's base is written as a
/// placeholder when the node opens and finalized by [`close_node`](Self::close_node)
/// once its children are in place.
struct Builder {
    topo: Bits,
    base: Vec<u64>,
}

impl Builder {
    fn new() -> Self {
        Builder {
            topo: Bits::new(),
            base: Vec::new(),
        }
    }

    fn len(&self) -> usize {
        self.base.len()
    }

    /// Append a leaf with the given base; return its index.
    fn leaf(&mut self, base: u64) -> usize {
        let i = self.len();
        self.topo.push(false);
        self.base.push(base);
        i
    }

    /// Open an internal node with a placeholder base; its children are appended next.
    /// Return its index.
    fn open(&mut self, base: u64) -> usize {
        let i = self.len();
        self.topo.push(true);
        self.base.push(base);
        i
    }

    /// Copy the subtree at `root` of `src` verbatim (it is already normalized); return
    /// `(new_root, src_end)` — its index here and the position just past it in `src`.
    /// Iterative single pass.
    fn copy(&mut self, src: &EvView, root: usize) -> (usize, usize) {
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
    fn close_node(&mut self, node: usize, right: usize) {
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

    fn finish(self) -> WorkingVersion {
        WorkingVersion {
            topo: self.topo,
            base: self.base,
        }
    }
}

// ───────────────────────────── merge (event-tree join) ─────────────────────────────

/// A step in the threaded two-tree `ev_join` walk. `ret` carries the just-finished
/// subtree's `(out_root, a_end, b_end)`.
enum JoinJob {
    Eval {
        ap: usize,
        ao: u64,
        bp: usize,
        bo: u64,
    },
    /// Left child finished; launch the right child (threading each internal side,
    /// re-broadcasting each leaf side).
    Right {
        a_int: bool,
        an: u64,
        ap: usize,
        ao: u64,
        b_int: bool,
        bn: u64,
        bp: usize,
        bo: u64,
    },
    /// Right child finished; sink and close the node, reporting its end positions.
    Close {
        node: usize,
        a_int: bool,
        a_next: usize,
        b_int: bool,
        b_next: usize,
    },
}

/// The least upper bound of two event trees (the paper's `join` over event trees),
/// produced in normal form. Reads either storage form via [`EvView`]; `O(n + m)`.
pub(crate) fn ev_join(a: &EvView, b: &EvView) -> WorkingVersion {
    let mut out = Builder::new();
    let mut ret = (0usize, 0usize, 0usize); // (out_root, a_end, b_end)
    let mut stack = vec![JoinJob::Eval {
        ap: 0,
        ao: 0,
        bp: 0,
        bo: 0,
    }];
    while let Some(job) = stack.pop() {
        match job {
            JoinJob::Eval { ap, ao, bp, bo } => {
                let (a_int, a_base, a_next) = a.header(ap);
                let (b_int, b_base, b_next) = b.header(bp);
                let an = ao + a_base;
                let bn = bo + b_base;
                if !a_int && !b_int {
                    let root = out.leaf(an.max(bn));
                    ret = (root, a_next, b_next);
                    continue;
                }
                let node = out.open(0);
                // Left children: an internal side descends; a leaf side broadcasts in
                // place (reuse its position/offset, so its value stays `an`/`bn`).
                let (la_p, la_o) = if a_int { (a_next, an) } else { (ap, ao) };
                let (lb_p, lb_o) = if b_int { (b_next, bn) } else { (bp, bo) };
                stack.push(JoinJob::Close {
                    node,
                    a_int,
                    a_next,
                    b_int,
                    b_next,
                });
                stack.push(JoinJob::Right {
                    a_int,
                    an,
                    ap,
                    ao,
                    b_int,
                    bn,
                    bp,
                    bo,
                });
                stack.push(JoinJob::Eval {
                    ap: la_p,
                    ao: la_o,
                    bp: lb_p,
                    bo: lb_o,
                });
            }
            JoinJob::Right {
                a_int,
                an,
                ap,
                ao,
                b_int,
                bn,
                bp,
                bo,
            } => {
                let (_, a_left_end, b_left_end) = ret;
                let (ra_p, ra_o) = if a_int { (a_left_end, an) } else { (ap, ao) };
                let (rb_p, rb_o) = if b_int { (b_left_end, bn) } else { (bp, bo) };
                stack.push(JoinJob::Eval {
                    ap: ra_p,
                    ao: ra_o,
                    bp: rb_p,
                    bo: rb_o,
                });
            }
            JoinJob::Close {
                node,
                a_int,
                a_next,
                b_int,
                b_next,
            } => {
                let (right_root, a_right_end, b_right_end) = ret;
                out.close_node(node, right_root);
                let a_end = if a_int { a_right_end } else { a_next };
                let b_end = if b_int { b_right_end } else { b_next };
                ret = (node, a_end, b_end);
            }
        }
    }
    out.finish()
}

/// The maximum value of the event function over the subtree at `root` (the paper's
/// `max`: `base + max(child maxes)`), and the position just past the subtree. Iterative
/// linear pass — a per-ancestor cumulative/remaining stack, no right-child re-scan.
fn ev_max(view: &EvView, root: usize) -> (u64, usize) {
    let mut max = 0u64;
    let mut pos = root;
    let mut stack: Vec<(u64, u8)> = Vec::new(); // (node cumulative, children remaining)
    loop {
        let offset = stack.last().map_or(0, |&(c, _)| c);
        let (internal, base, next) = view.header(pos);
        let cumulative = offset + base;
        max = max.max(cumulative);
        pos = next;
        if internal {
            stack.push((cumulative, 2));
        } else {
            // A leaf completes; pop every ancestor whose children are now all done.
            loop {
                match stack.last_mut() {
                    None => return (max, pos),
                    Some(frame) => {
                        frame.1 -= 1;
                        if frame.1 == 0 {
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

/// A step in the threaded `fill` walk. `ret` carries the just-finished subtree's
/// `(out_root, id_end, ev_end)`.
enum FillJob {
    Eval {
        id: usize,
        ev: usize,
    },
    /// `il` is full: the right child (the filled `er`) is being built; afterwards set
    /// the collapsed left leaf to `max(max_ev(el), min(er'))` and close.
    FullLeftClose {
        node: usize,
        left_leaf: usize,
        max_el: u64,
    },
    /// `il` is not full: the left child (filled `el`) is being built; afterwards decide
    /// the right child by whether `ir` is full.
    AfterLeft {
        node: usize,
    },
    /// Right child (filled `er`) is being built for the general case; afterwards close.
    GeneralClose {
        node: usize,
    },
}

/// `fill(id, ev)` (plan Appendix A): use the available id to simplify the event tree
/// without registering a new event — wherever the id is full over a subtree, collapse
/// that subtree to its maximum. Produces normal form. Iterative, `O(n + m)`: the event
/// drives (every event node visited once, threaded), and the id is lazy-skipped only
/// where the event prunes it (an event leaf under an id node).
fn fill(id_bits: &BitsSlice, view: &EvView) -> WorkingVersion {
    let mut out = Builder::new();
    let mut ret = (0usize, 0usize, 0usize); // (out_root, id_end, ev_end)
    let mut stack = vec![FillJob::Eval { id: 0, ev: 0 }];
    while let Some(job) = stack.pop() {
        match job {
            FillJob::Eval { id, ev } => {
                let (id_node, id_val, id_next) = idbits::header(id_bits, id);
                if !id_node && !id_val {
                    // id 0-leaf: nothing owned here; the event is unchanged.
                    let (root, ev_end) = out.copy(view, ev);
                    ret = (root, id_next, ev_end);
                    continue;
                }
                if !id_node {
                    // id 1-leaf (full): collapse the whole event subtree to its max.
                    let (mx, ev_end) = ev_max(view, ev);
                    ret = (out.leaf(mx), id_next, ev_end);
                    continue;
                }
                let (ev_int, ev_base, ev_next) = view.header(ev);
                if !ev_int {
                    // id node over an event leaf: unchanged; lazy-skip the id subtree.
                    ret = (out.leaf(ev_base), idbits::skip(id_bits, id), ev_next);
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
                        id: id_right,
                        ev: ev_right,
                    });
                } else {
                    // `il` not full: fill the left child first; decide the right after.
                    let node = out.open(ev_base);
                    stack.push(FillJob::AfterLeft { node });
                    stack.push(FillJob::Eval {
                        id: id_left,
                        ev: ev_left,
                    });
                }
            }
            FillJob::FullLeftClose {
                node,
                left_leaf,
                max_el,
            } => {
                let (er_root, id_end, ev_end) = ret;
                out.base[left_leaf] = max_el.max(out.base[er_root]);
                out.close_node(node, er_root);
                ret = (node, id_end, ev_end);
            }
            FillJob::AfterLeft { node } => {
                let (el_root, id_end_left, ev_end_left) = ret;
                let (ir, er) = (id_end_left, ev_end_left);
                if idbits::is_full(id_bits, ir) {
                    // `ir` full: right collapses to a leaf depending on the filled left.
                    let (max_er, er_end) = ev_max(view, er);
                    let x = max_er.max(out.base[el_root]);
                    let right_leaf = out.leaf(x);
                    out.close_node(node, right_leaf);
                    ret = (node, ir + 2 /* past the 1-leaf `ir` */, er_end);
                } else {
                    stack.push(FillJob::GeneralClose { node });
                    stack.push(FillJob::Eval { id: ir, ev: er });
                }
            }
            FillJob::GeneralClose { node } => {
                let (er_root, id_end, ev_end) = ret;
                out.close_node(node, er_root);
                ret = (node, id_end, ev_end);
            }
        }
    }
    out.finish()
}

// ───────────────────────────── grow ─────────────────────────────

/// Lexicographic inflation cost `(expansions, depth)`: prefer fewer leaf-to-node
/// expansions, then a shallower spot. `MAX` marks an infeasible (empty-id) region.
type Cost = (u32, u32);
const COST_MAX: Cost = (u32::MAX, u32::MAX);

/// Which `(id, ev)` recursion shape a `grow` branch node has — fixes its cost formula
/// and how its children are positioned.
#[derive(Clone, Copy)]
enum Kind {
    /// id is full (`1`), event is a node: descend the event, id stays full.
    FullEvNode,
    /// id is a node, event is a leaf/virtual: expand the leaf (one expansion), descend id.
    Expand,
    /// id is a node, event is a node: descend both.
    Both,
}

/// One step of the read-only cost probe (a threaded postorder over the `(id, event)`
/// shape, expanding event leaves into virtual `Leaf(0)`s to follow the id). `ret`
/// carries `(cost, id_end, ev_end)`.
enum ProbeJob {
    Eval {
        id: usize,
        ev: usize,
    },
    Right {
        id: usize,
        ev: usize,
        kind: Kind,
        id_next: usize,
        ev_next: usize,
    },
    Combine {
        id: usize,
        ev: usize,
        kind: Kind,
        id_next: usize,
        ev_next: usize,
        cost_l: Cost,
    },
}

/// Probe the cheapest inflation, recording the chosen child direction (`true` = left)
/// per `(id, ev)` branch node into `choice`. Read-only; `O(n + m)`. The id is
/// lazy-skipped where an empty region prunes the event.
fn grow_probe(
    id_bits: &BitsSlice,
    view: &EvView,
    choice: &mut std::collections::HashMap<(usize, usize), bool>,
) {
    let mut ret: (Cost, usize, usize) = (COST_MAX, 0, 0);
    let mut stack = vec![ProbeJob::Eval { id: 0, ev: 0 }];
    while let Some(job) = stack.pop() {
        match job {
            ProbeJob::Eval { id, ev } => {
                let (id_node, id_val, id_next) = idbits::header(id_bits, id);
                let virt = ev == VIRTUAL;
                let (ev_int, _ev_base, ev_next) = if virt {
                    (false, 0u64, VIRTUAL)
                } else {
                    view.header(ev)
                };
                if !id_node && !id_val {
                    // id 0-leaf: infeasible; lazy-skip the dominated event subtree.
                    let ev_end = if virt { VIRTUAL } else { ev_skip(view, ev) };
                    ret = (COST_MAX, id_next, ev_end);
                } else if !id_node {
                    // id 1-leaf (full).
                    if !ev_int {
                        ret = ((0, 0), id_next, ev_next); // increment here
                    } else {
                        stack.push(ProbeJob::Right {
                            id,
                            ev,
                            kind: Kind::FullEvNode,
                            id_next,
                            ev_next,
                        });
                        stack.push(ProbeJob::Eval { id, ev: ev + 1 });
                    }
                } else if !ev_int {
                    // id node, event leaf/virtual: expand and descend the id.
                    stack.push(ProbeJob::Right {
                        id,
                        ev,
                        kind: Kind::Expand,
                        id_next,
                        ev_next,
                    });
                    stack.push(ProbeJob::Eval {
                        id: id_next,
                        ev: VIRTUAL,
                    });
                } else {
                    // id node, event node.
                    stack.push(ProbeJob::Right {
                        id,
                        ev,
                        kind: Kind::Both,
                        id_next,
                        ev_next,
                    });
                    stack.push(ProbeJob::Eval {
                        id: id_next,
                        ev: ev + 1,
                    });
                }
            }
            ProbeJob::Right {
                id,
                ev,
                kind,
                id_next,
                ev_next,
            } => {
                let (cost_l, id_end_l, ev_end_l) = ret;
                let (rid, rev) = match kind {
                    Kind::FullEvNode => (id, ev_end_l), // id stays full; right event child
                    Kind::Expand => (id_end_l, VIRTUAL), // `ir`, still virtual
                    Kind::Both => (id_end_l, ev_end_l), // `ir`, `er`
                };
                stack.push(ProbeJob::Combine {
                    id,
                    ev,
                    kind,
                    id_next,
                    ev_next,
                    cost_l,
                });
                stack.push(ProbeJob::Eval { id: rid, ev: rev });
            }
            ProbeJob::Combine {
                id,
                ev,
                kind,
                id_next,
                ev_next,
                cost_l,
            } => {
                let (cost_r, id_end_r, ev_end_r) = ret;
                let left_chosen = cost_l < cost_r; // tie favors the right (matches oracle)
                choice.insert((id, ev), left_chosen);
                let m = if left_chosen { cost_l } else { cost_r };
                let cost = match kind {
                    Kind::Expand => (m.0.saturating_add(1), m.1.saturating_add(1)),
                    Kind::FullEvNode | Kind::Both => (m.0, m.1.saturating_add(1)),
                };
                let id_end = match kind {
                    Kind::FullEvNode => id_next, // the 1-leaf is consumed
                    Kind::Expand | Kind::Both => id_end_r,
                };
                let ev_end = match kind {
                    Kind::FullEvNode | Kind::Both => ev_end_r,
                    Kind::Expand => ev_next, // event leaf/virtual consumed
                };
                ret = (cost, id_end, ev_end);
            }
        }
    }
}

/// A step in the threaded `grow` emit, following the probe's choices down the chosen
/// path and copying everything off it. `ret` carries `(out_root, id_end, ev_end)`.
enum EmitJob {
    Eval {
        id: usize,
        ev: usize,
    },
    /// The chosen *left* child has just been built (`ret`); emit the off-path right
    /// sibling, then sink and close.
    CloseAfterLeft {
        node: usize,
        kind: Kind,
        id_next: usize,
        ev_next: usize,
    },
    /// The chosen *right* child has just been built (`ret`); the off-path left sibling
    /// was already emitted. Sink and close.
    CloseAfterRight {
        node: usize,
        kind: Kind,
        id_next: usize,
        ev_next: usize,
    },
}

/// Emit the grown tree using the probe's `choice` map, in normal form. Iterative,
/// `O(n + m)`: only the chosen root-to-leaf path is rebuilt (with the inflation and the
/// sink); every off-path subtree is copied or skipped exactly once.
fn grow_emit(
    id_bits: &BitsSlice,
    view: &EvView,
    out: &mut Builder,
    choice: &std::collections::HashMap<(usize, usize), bool>,
) {
    let mut ret = (0usize, 0usize, 0usize); // (out_root, id_end, ev_end)
    let mut stack = vec![EmitJob::Eval { id: 0, ev: 0 }];
    while let Some(job) = stack.pop() {
        match job {
            EmitJob::Eval { id, ev } => {
                let (id_node, _id_val, id_next) = idbits::header(id_bits, id);
                let virt = ev == VIRTUAL;
                let (ev_int, ev_base, ev_next) = if virt {
                    (false, 0u64, VIRTUAL)
                } else {
                    view.header(ev)
                };
                // The inflation point: id full over a leaf/virtual event — increment.
                if !id_node && !ev_int {
                    ret = (out.leaf(ev_base + 1), id_next, ev_next);
                    continue;
                }
                let kind = if !id_node {
                    Kind::FullEvNode
                } else if ev_int {
                    Kind::Both
                } else {
                    Kind::Expand
                };
                let node = out.open(ev_base);
                let left_chosen = choice[&(id, ev)];
                if left_chosen {
                    // Descend the chosen left child now; the right is emitted on close.
                    let (cid, cev) = match kind {
                        Kind::FullEvNode => (id, ev + 1),   // id stays full
                        Kind::Both => (id_next, ev + 1),    // `il`, `el`
                        Kind::Expand => (id_next, VIRTUAL), // `il`, virtual
                    };
                    stack.push(EmitJob::CloseAfterLeft {
                        node,
                        kind,
                        id_next,
                        ev_next,
                    });
                    stack.push(EmitJob::Eval { id: cid, ev: cev });
                } else {
                    // Emit the off-path left sibling now, then descend the chosen right.
                    let (cid, cev) = match kind {
                        Kind::FullEvNode => {
                            let (_l, ev_right) = out.copy(view, ev + 1);
                            (id, ev_right)
                        }
                        Kind::Both => {
                            let (_l, ev_right) = out.copy(view, ev + 1);
                            (idbits::skip(id_bits, id_next), ev_right)
                        }
                        Kind::Expand => {
                            out.leaf(0);
                            (idbits::skip(id_bits, id_next), VIRTUAL)
                        }
                    };
                    stack.push(EmitJob::CloseAfterRight {
                        node,
                        kind,
                        id_next,
                        ev_next,
                    });
                    stack.push(EmitJob::Eval { id: cid, ev: cev });
                }
            }
            EmitJob::CloseAfterLeft {
                node,
                kind,
                id_next,
                ev_next,
            } => {
                let (_left_root, id_end_l, ev_end_l) = ret;
                let (right_root, id_end, ev_end) = match kind {
                    Kind::FullEvNode => {
                        let (rr, ev_node_end) = out.copy(view, ev_end_l); // off-path `er`
                        (rr, id_next, ev_node_end)
                    }
                    Kind::Both => {
                        let (rr, ev_node_end) = out.copy(view, ev_end_l); // off-path `er`
                        (rr, idbits::skip(id_bits, id_end_l), ev_node_end)
                    }
                    Kind::Expand => {
                        let rr = out.leaf(0); // off-path sibling is a fresh Leaf(0)
                        (rr, idbits::skip(id_bits, id_end_l), ev_next)
                    }
                };
                out.close_node(node, right_root);
                ret = (node, id_end, ev_end);
            }
            EmitJob::CloseAfterRight {
                node,
                kind,
                id_next,
                ev_next,
            } => {
                let (right_root, id_end_r, ev_end_r) = ret;
                let (id_end, ev_end) = match kind {
                    Kind::FullEvNode => (id_next, ev_end_r),
                    Kind::Both => (id_end_r, ev_end_r),
                    Kind::Expand => (id_end_r, ev_next),
                };
                out.close_node(node, right_root);
                ret = (node, id_end, ev_end);
            }
        }
    }
}

/// `grow(id, ev)`: register a new event by the cheapest available inflation, in normal
/// form. Two iterative passes — a read-only cost probe, then an emit along the chosen
/// path — each `O(n + m)`.
fn grow(id_bits: &BitsSlice, view: &EvView) -> WorkingVersion {
    let mut choice = std::collections::HashMap::new();
    grow_probe(id_bits, view, &mut choice);
    let mut out = Builder::new();
    grow_emit(id_bits, view, &mut out, &choice);
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
        grow(id, &view)
    }
}
