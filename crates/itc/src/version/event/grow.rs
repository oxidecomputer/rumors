//! `grow`: register a new event when [`fill`](EvView::fill) cannot simplify the tree, by
//! inflating the cheapest available leaf. Two iterative passes, each `O(n + m)`:
//!
//! 1. [`grow_probe`](EvView::grow_probe) — a read-only cost probe that walks the
//!    `(id, event)` shape and, at every branch node, records which child the cheapest
//!    inflation descends into.
//! 2. [`grow_emit`](EvView::grow_emit) — replays the probe's choices, rebuilding only the
//!    chosen root-to-leaf path (with the inflation and the sink) and copying/skipping
//!    every off-path subtree exactly once.
//!
//! Both passes share the **thread register** discipline documented in
//! [`super`]'s module doc: each `Eval` arm writes `ret` with the just-finished
//! subtree's end positions, and a deferred sibling frame reads it to resume.
//!
//! **Probe → emit contract.** The two passes are the *same* traversal: they visit
//! exactly the same `(id, ev)` branch nodes, in the same preorder, addressed by the
//! same `(id_pos, ev_pos)` coordinates. `grow_probe` records a [`Choices`] entry for
//! every branch node it reaches; `grow_emit` reads back the entry for each node on the
//! chosen path. The probe always set it, because the two passes walk the identical
//! branch nodes — the coordinate agreement is what lets them communicate by position.

use crate::codec::{Base, BitsSlice};
use crate::idbits::{IdHeader, IdView};

use super::{Builder, Built, VIRTUAL};
use crate::version::compare::{EvHeader, EvView};
use crate::version::working::WorkingVersion;

/// Lexicographic inflation cost `(expansions, depth)`: prefer fewer leaf-to-node
/// expansions, then a shallower spot. `MAX` ([`COST_MAX`]) marks an infeasible
/// (empty-id) region. Ties between a node's two children favor the *right* child, to
/// match the oracle's choice (see [`grow_probe`](EvView::grow_probe)'s `left_chosen`).
type Cost = (u32, u32);

/// The cost of an infeasible region: an empty-id subtree can never be inflated.
const COST_MAX: Cost = (u32::MAX, u32::MAX);

/// The probe → emit channel: at each branch node, whether the cheapest inflation
/// descended into the *left* child (`true`) or the right (`false`).
///
/// The key `(id_pos, ev_pos)` has an alternating pinned axis (one coordinate is held
/// constant while the other descends — see the module doc), so no single array can be
/// keyed by one coordinate alone. Instead two dense arrays split by regime, which is
/// exactly "is the id a node?":
/// - [`by_id`](Choices::by_id): id is a node (`Expand`/`Both`), keyed by the id
///   bit-position. Each id internal node is visited once, so its slot is unique.
/// - [`by_ev`](Choices::by_ev): id is a full `1`-leaf (`FullEvNode`), keyed by the
///   event position. Each event node is reached under at most one id context.
///
/// Slots default to `None`; [`grow_probe`](EvView::grow_probe) fills the one for each
/// branch node it reaches and [`grow_emit`](EvView::grow_emit) reads back the slot for
/// the regime it is in. Total space
/// `O(n + m)`, `O(1)` access (no hashing).
struct Choices {
    /// Indexed by id bit-position; used when the id is a node.
    by_id: Vec<Option<bool>>,
    /// Indexed by event position; used when the id is a full `1`-leaf.
    by_ev: Vec<Option<bool>>,
}

impl Choices {
    /// All slots unset, sized to the id and event position spaces.
    fn new(id_span: usize, ev_span: usize) -> Self {
        Choices {
            by_id: vec![None; id_span],
            by_ev: vec![None; ev_span],
        }
    }

    /// Record the chosen direction at the branch node of the given `kind` addressed by
    /// `(id_pos, ev_pos)`.
    fn record(&mut self, kind: Kind, id_pos: usize, ev_pos: usize, left: bool) {
        match kind {
            Kind::FullEvNode => self.by_ev[ev_pos] = Some(left),
            Kind::Expand | Kind::Both => self.by_id[id_pos] = Some(left),
        }
    }

    /// The chosen direction at the branch node addressed by `(id_pos, ev_pos)`. Panics if
    /// the probe never recorded it — a coordinate-agreement bug, not a runtime condition
    /// (see the module doc).
    fn chosen(&self, kind: Kind, id_pos: usize, ev_pos: usize) -> bool {
        let slot = match kind {
            Kind::FullEvNode => self.by_ev[ev_pos],
            Kind::Expand | Kind::Both => self.by_id[id_pos],
        };
        slot.expect("grow_emit reached a branch node grow_probe did not record")
    }
}

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
/// shape, expanding event leaves into virtual `Leaf(0)`s to follow the id). `ret` is the
/// [`Probed`] register. `id_pos` is a bit offset into the packed id stream; `ev_pos` a
/// position in the event tree (or [`VIRTUAL`]); `id_next`/`ev_next` are the positions
/// just past this node's header, threaded to its children.
enum ProbeJob {
    /// Probe the cheapest inflation of the event subtree at `ev_pos` under `id_pos`.
    Eval {
        /// Position into the packed id stream.
        id_pos: usize,
        /// Position into the event tree (or [`VIRTUAL`]).
        ev_pos: usize,
    },
    /// Left child probed; launch the right child, then combine the two costs.
    Right {
        /// Branch node id position (the cost slot key for `Expand`/`Both`).
        id_pos: usize,
        /// Branch node event position (the cost slot key for `FullEvNode`).
        ev_pos: usize,
        /// Which recursion shape this branch node has.
        kind: Kind,
        /// Position just past the id header, for threading the right child.
        id_next: usize,
        /// Position just past the event header, for threading the right child.
        ev_next: usize,
    },
    /// Both children probed; pick the cheaper, record the direction, fold the cost.
    Combine {
        /// Branch node id position (the cost slot key for `Expand`/`Both`).
        id_pos: usize,
        /// Branch node event position (the cost slot key for `FullEvNode`).
        ev_pos: usize,
        /// Which recursion shape this branch node has.
        kind: Kind,
        /// Position just past the id header.
        id_next: usize,
        /// Position just past the event header.
        ev_next: usize,
        /// The left child's cost, captured before probing the right.
        left_cost: Cost,
    },
}

/// The thread register for the cost probe (see [`super`]'s module doc): the cheapest
/// inflation `cost` of a just-finished subtree, plus where it ended in the id stream and
/// the event tree. An `Eval` arm *writes* it (a leaf directly, or via `Combine` folding
/// a node); the deferred `Right`/`Combine` frames *read* it.
#[derive(Clone, Copy)]
struct Probed {
    /// The cheapest inflation cost of the subtree.
    cost: Cost,
    /// Position just past the subtree in the packed id stream.
    id_end: usize,
    /// Position just past the subtree in the event tree (or [`VIRTUAL`]).
    ev_end: usize,
}

impl EvView<'_> {
    /// Probe the cheapest inflation of this event tree (`self`), recording the chosen child
    /// direction (`true` = left) per `(id, ev)` branch node into `choice`. Read-only;
    /// `O(n + m)`. The id is lazy-skipped where an empty region prunes the event.
    ///
    /// This is the cost-finding half of the iterative form of the recursive
    /// `oracle::Version::grow` (the paper's `grow`); read that recursive twin first. Where
    /// the oracle recurses once and rebuilds on the way back up, the iterative form splits
    /// into this probe pass and the [`grow_emit`](EvView::grow_emit) replay pass.
    fn grow_probe(&self, id_bits: &BitsSlice, choice: &mut Choices) {
        let view = self;
        let id = IdView(id_bits);
        let mut ret = Probed {
            cost: COST_MAX,
            id_end: 0,
            ev_end: 0,
        };
        let mut stack = vec![ProbeJob::Eval {
            id_pos: 0,
            ev_pos: 0,
        }];
        while let Some(job) = stack.pop() {
            match job {
                ProbeJob::Eval { id_pos, ev_pos } => {
                    let id_next = id.header(id_pos).next;
                    let virt = ev_pos == VIRTUAL;
                    let EvHeader {
                        internal: ev_int,
                        base: _ev_base,
                        next: ev_next,
                    } = if virt {
                        EvHeader {
                            internal: false,
                            base: Base::ZERO,
                            next: VIRTUAL,
                        }
                    } else {
                        view.header(ev_pos)
                    };
                    if id.is_empty(id_pos) {
                        // id 0-leaf: infeasible; lazy-skip the dominated event subtree.
                        let ev_end = if virt { VIRTUAL } else { view.skip(ev_pos) };
                        ret = Probed {
                            cost: COST_MAX,
                            id_end: id_next,
                            ev_end,
                        };
                    } else if id.is_full(id_pos) {
                        // id 1-leaf (full).
                        if !ev_int {
                            // increment here: a free inflation
                            ret = Probed {
                                cost: (0, 0),
                                id_end: id_next,
                                ev_end: ev_next,
                            };
                        } else {
                            stack.push(ProbeJob::Right {
                                id_pos,
                                ev_pos,
                                kind: Kind::FullEvNode,
                                id_next,
                                ev_next,
                            });
                            stack.push(ProbeJob::Eval {
                                id_pos,
                                ev_pos: ev_pos + 1,
                            });
                        }
                    } else if !ev_int {
                        // id node, event leaf/virtual: expand and descend the id.
                        stack.push(ProbeJob::Right {
                            id_pos,
                            ev_pos,
                            kind: Kind::Expand,
                            id_next,
                            ev_next,
                        });
                        stack.push(ProbeJob::Eval {
                            id_pos: id_next,
                            ev_pos: VIRTUAL,
                        });
                    } else {
                        // id node, event node.
                        stack.push(ProbeJob::Right {
                            id_pos,
                            ev_pos,
                            kind: Kind::Both,
                            id_next,
                            ev_next,
                        });
                        stack.push(ProbeJob::Eval {
                            id_pos: id_next,
                            ev_pos: ev_pos + 1,
                        });
                    }
                }
                ProbeJob::Right {
                    id_pos,
                    ev_pos,
                    kind,
                    id_next,
                    ev_next,
                } => {
                    let left = ret; // the left child's probe report
                    let (right_id, right_ev) = match kind {
                        Kind::FullEvNode => (id_pos, left.ev_end), // id stays full; right event child
                        Kind::Expand => (left.id_end, VIRTUAL),    // `ir`, still virtual
                        Kind::Both => (left.id_end, left.ev_end),  // `ir`, `er`
                    };
                    stack.push(ProbeJob::Combine {
                        id_pos,
                        ev_pos,
                        kind,
                        id_next,
                        ev_next,
                        left_cost: left.cost,
                    });
                    stack.push(ProbeJob::Eval {
                        id_pos: right_id,
                        ev_pos: right_ev,
                    });
                }
                ProbeJob::Combine {
                    id_pos,
                    ev_pos,
                    kind,
                    id_next,
                    ev_next,
                    left_cost,
                } => {
                    // `ret` is the right child's probe report.
                    let right = ret;
                    // Strict `<` makes a tie favor the right child (see [`Cost`]).
                    let left_chosen = left_cost < right.cost;
                    choice.record(kind, id_pos, ev_pos, left_chosen);
                    let m = if left_chosen { left_cost } else { right.cost };
                    let cost = match kind {
                        Kind::Expand => (m.0.saturating_add(1), m.1.saturating_add(1)),
                        Kind::FullEvNode | Kind::Both => (m.0, m.1.saturating_add(1)),
                    };
                    let id_end = match kind {
                        Kind::FullEvNode => id_next, // the 1-leaf is consumed
                        Kind::Expand | Kind::Both => right.id_end,
                    };
                    let ev_end = match kind {
                        Kind::FullEvNode | Kind::Both => right.ev_end,
                        Kind::Expand => ev_next, // event leaf/virtual consumed
                    };
                    ret = Probed {
                        cost,
                        id_end,
                        ev_end,
                    };
                }
            }
        }
    }
}

/// A step in the threaded `grow` emit, following the probe's choices down the chosen
/// path and copying everything off it. `ret` is the [`Built`] register (shared with
/// `fill`). `id_pos` is a bit offset into the packed id stream; `ev_pos` a position in
/// the event tree (or [`VIRTUAL`]); `id_next`/`ev_next` are the positions just past this
/// node's header.
enum EmitJob {
    /// Emit the grown event subtree at `ev_pos` under the id subtree at `id_pos`.
    Eval {
        /// Position into the packed id stream.
        id_pos: usize,
        /// Position into the event tree (or [`VIRTUAL`]).
        ev_pos: usize,
    },
    /// The chosen *left* child has just been built (`ret`); emit the off-path right
    /// sibling, then sink and close.
    CloseAfterLeft {
        /// Output index of the node being closed.
        node: usize,
        /// Which recursion shape this branch node has.
        kind: Kind,
        /// Position just past the id header (the off-path right's id context).
        id_next: usize,
        /// Position just past the event header (the off-path right's event context).
        ev_next: usize,
    },
    /// The chosen *right* child has just been built (`ret`); the off-path left sibling
    /// was already emitted. Sink and close.
    CloseAfterRight {
        /// Output index of the node being closed.
        node: usize,
        /// Which recursion shape this branch node has.
        kind: Kind,
        /// Position just past the id header.
        id_next: usize,
        /// Position just past the event header.
        ev_next: usize,
    },
}

impl EvView<'_> {
    /// Emit the grown tree (`self` is the source event tree) using the probe's `choice`
    /// map, in normal form. Iterative, `O(n + m)`: only the chosen root-to-leaf path is
    /// rebuilt (with the inflation and the sink); every off-path subtree is copied or
    /// skipped exactly once.
    ///
    /// This is the rebuilding half of the iterative form of the recursive
    /// `oracle::Version::grow` (the paper's `grow`); read that recursive twin first. It
    /// replays the choices [`grow_probe`](EvView::grow_probe) recorded, standing in for the
    /// oracle's bottom-up reconstruction on the way out of the recursion.
    fn grow_emit(&self, id_bits: &BitsSlice, out: &mut Builder, choice: &Choices) {
        let view = self;
        let id = IdView(id_bits);
        let mut ret = Built::default(); // the thread register, shared with `fill`
        let mut stack = vec![EmitJob::Eval {
            id_pos: 0,
            ev_pos: 0,
        }];
        while let Some(job) = stack.pop() {
            match job {
                EmitJob::Eval { id_pos, ev_pos } => {
                    let IdHeader {
                        node: id_node,
                        val: id_val,
                        next: id_next,
                    } = id.header(id_pos);
                    let virt = ev_pos == VIRTUAL;
                    let EvHeader {
                        internal: ev_int,
                        base: ev_base,
                        next: ev_next,
                    } = if virt {
                        EvHeader {
                            internal: false,
                            base: Base::ZERO,
                            next: VIRTUAL,
                        }
                    } else {
                        view.header(ev_pos)
                    };
                    // The inflation point: id full over a leaf/virtual event — increment.
                    if !id_node && !ev_int {
                        // Invariant: the chosen path never reaches an empty (`0`-leaf) id. A
                        // normal-form id node always has a nonempty child (its min-cost child
                        // is never the `COST_MAX` empty side), and a real `Party`'s root is
                        // never empty — so an id leaf on the chosen path is always full.
                        debug_assert!(id_val, "grow chose an empty-id region to inflate");
                        ret = Built {
                            out_root: out.leaf(ev_base + 1u32),
                            id_end: id_next,
                            ev_end: ev_next,
                        };
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
                    let left_chosen = choice.chosen(kind, id_pos, ev_pos);
                    if left_chosen {
                        // Descend the chosen left child now; the right is emitted on close.
                        let (child_id, child_ev) = match kind {
                            Kind::FullEvNode => (id_pos, ev_pos + 1), // id stays full
                            Kind::Both => (id_next, ev_pos + 1),      // `il`, `el`
                            Kind::Expand => (id_next, VIRTUAL),       // `il`, virtual
                        };
                        stack.push(EmitJob::CloseAfterLeft {
                            node,
                            kind,
                            id_next,
                            ev_next,
                        });
                        stack.push(EmitJob::Eval {
                            id_pos: child_id,
                            ev_pos: child_ev,
                        });
                    } else {
                        // Emit the off-path left sibling now, then descend the chosen right.
                        let (child_id, child_ev) = match kind {
                            Kind::FullEvNode => {
                                let (_l, ev_right) = out.copy(view, ev_pos + 1);
                                (id_pos, ev_right)
                            }
                            Kind::Both => {
                                let (_l, ev_right) = out.copy(view, ev_pos + 1);
                                (id.skip(id_next), ev_right)
                            }
                            Kind::Expand => {
                                out.leaf(Base::ZERO);
                                (id.skip(id_next), VIRTUAL)
                            }
                        };
                        stack.push(EmitJob::CloseAfterRight {
                            node,
                            kind,
                            id_next,
                            ev_next,
                        });
                        stack.push(EmitJob::Eval {
                            id_pos: child_id,
                            ev_pos: child_ev,
                        });
                    }
                }
                EmitJob::CloseAfterLeft {
                    node,
                    kind,
                    id_next,
                    ev_next,
                } => {
                    let left = ret; // the chosen left child's report (its root already placed)
                    let (right_root, id_end, ev_end) = match kind {
                        Kind::FullEvNode => {
                            let (rr, ev_node_end) = out.copy(view, left.ev_end); // off-path `er`
                            (rr, id_next, ev_node_end)
                        }
                        Kind::Both => {
                            let (rr, ev_node_end) = out.copy(view, left.ev_end); // off-path `er`
                            (rr, id.skip(left.id_end), ev_node_end)
                        }
                        Kind::Expand => {
                            let rr = out.leaf(Base::ZERO); // off-path sibling is a fresh Leaf(0)
                            (rr, id.skip(left.id_end), ev_next)
                        }
                    };
                    out.close_node(node, right_root);
                    ret = Built {
                        out_root: node,
                        id_end,
                        ev_end,
                    };
                }
                EmitJob::CloseAfterRight {
                    node,
                    kind,
                    id_next,
                    ev_next,
                } => {
                    let right = ret; // the chosen right child's report
                    let (id_end, ev_end) = match kind {
                        Kind::FullEvNode => (id_next, right.ev_end),
                        Kind::Both => (right.id_end, right.ev_end),
                        Kind::Expand => (right.id_end, ev_next),
                    };
                    out.close_node(node, right.out_root);
                    ret = Built {
                        out_root: node,
                        id_end,
                        ev_end,
                    };
                }
            }
        }
    }

    /// `grow(id, ev)`: register a new event on this event tree (`self`) by the cheapest
    /// available inflation, in normal form. Two iterative passes — a read-only cost probe,
    /// then an emit along the chosen path — each `O(n + m)`. The probe and emit are the same
    /// traversal; see the module doc for the `(id, ev)`-coordinate contract that links them
    /// through [`Choices`].
    pub(super) fn grow(&self, id_bits: &BitsSlice) -> WorkingVersion {
        let mut choice = Choices::new(id_bits.len(), self.span());
        self.grow_probe(id_bits, &mut choice);
        let mut out = Builder::new();
        self.grow_emit(id_bits, &mut out, &choice);
        out.finish()
    }
}
