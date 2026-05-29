//! `grow` (plan Appendix A): register a new event when [`fill`](super::fill) cannot
//! simplify the tree, by inflating the cheapest available leaf. Two iterative passes,
//! each `O(n + m)`:
//!
//! 1. [`grow_probe`] — a read-only cost probe that walks the `(id, event)` shape and,
//!    at every branch node, records which child the cheapest inflation descends into.
//! 2. [`grow_emit`] — replays the probe's choices, rebuilding only the chosen
//!    root-to-leaf path (with the inflation and the sink) and copying/skipping every
//!    off-path subtree exactly once.
//!
//! Both passes share the **thread register** discipline documented in
//! [`super`]'s module doc: each `Eval` arm writes `ret` with the just-finished
//! subtree's end positions, and a deferred sibling frame reads it to resume.
//!
//! **Probe → emit contract.** The two passes are the *same* traversal: they visit
//! exactly the same `(id, ev)` branch nodes, in the same preorder, addressed by the
//! same `(id_pos, ev_pos)` coordinates. `grow_probe` records a [`Choices`] entry for
//! every branch node it reaches; `grow_emit` indexes that map by `(id, ev)` and relies
//! on the entry being present (a missing key is a bug, not a runtime condition). The
//! coordinate agreement is what lets the two passes communicate by position alone.

use std::collections::HashMap;

use crate::codec::BitsSlice;
use crate::idbits;

use super::{Builder, VIRTUAL};
use crate::version::compare::{skip as ev_skip, EvView};
use crate::version::working::WorkingVersion;

/// Lexicographic inflation cost `(expansions, depth)`: prefer fewer leaf-to-node
/// expansions, then a shallower spot. `MAX` ([`COST_MAX`]) marks an infeasible
/// (empty-id) region. Ties between a node's two children favor the *right* child, to
/// match the oracle's choice (see [`grow_probe`]'s `left_chosen`).
type Cost = (u32, u32);

/// The cost of an infeasible region: an empty-id subtree can never be inflated.
const COST_MAX: Cost = (u32::MAX, u32::MAX);

/// The probe → emit channel: for each `(id_pos, ev_pos)` branch node, whether the
/// cheapest inflation descended into the *left* child (`true`) or the right (`false`).
/// Written by [`grow_probe`], read by [`grow_emit`]; see the module doc for the
/// coordinate-agreement contract that makes this lookup sound.
type Choices = HashMap<(usize, usize), bool>;

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
fn grow_probe(id_bits: &BitsSlice, view: &EvView, choice: &mut Choices) {
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
                // Strict `<` makes a tie favor the right child (see [`Cost`]).
                let left_chosen = cost_l < cost_r;
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
fn grow_emit(id_bits: &BitsSlice, view: &EvView, out: &mut Builder, choice: &Choices) {
    let mut ret = (0usize, 0usize, 0usize); // (out_root, id_end, ev_end)
    let mut stack = vec![EmitJob::Eval { id: 0, ev: 0 }];
    while let Some(job) = stack.pop() {
        match job {
            EmitJob::Eval { id, ev } => {
                let (id_node, id_val, id_next) = idbits::header(id_bits, id);
                let virt = ev == VIRTUAL;
                let (ev_int, ev_base, ev_next) = if virt {
                    (false, 0u64, VIRTUAL)
                } else {
                    view.header(ev)
                };
                // The inflation point: id full over a leaf/virtual event — increment.
                if !id_node && !ev_int {
                    // Invariant: the chosen path never reaches an empty (`0`-leaf) id. A
                    // normal-form id node always has a nonempty child (its min-cost child
                    // is never the `COST_MAX` empty side), and a real `Party`'s root is
                    // never empty — so an id leaf on the chosen path is always full.
                    debug_assert!(id_val, "grow chose an empty-id region to inflate");
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
/// path — each `O(n + m)`. The probe and emit are the same traversal; see the module
/// doc for the `(id, ev)`-coordinate contract that links them through [`Choices`].
pub(super) fn grow(id_bits: &BitsSlice, view: &EvView) -> WorkingVersion {
    let mut choice: Choices = HashMap::new();
    grow_probe(id_bits, view, &mut choice);
    let mut out = Builder::new();
    grow_emit(id_bits, view, &mut out, &choice);
    out.finish()
}
