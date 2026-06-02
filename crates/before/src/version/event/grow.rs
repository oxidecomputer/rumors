//! `grow`: register a new event when [`fill`](EvView::fill) cannot simplify the
//! tree, by inflating the cheapest available leaf. Two passes, each `O(n + m)`:
//!
//! 1. [`grow_probe`](EvView::grow_probe) — a read-only cost probe that walks the
//!    `(id, event)` shape and, at every branch node, records which child the
//!    cheapest inflation descends into.
//!
//! 2. [`grow_emit`](EvView::grow_emit) — replays the probe's choices, rebuilding
//!    only the chosen root-to-leaf path (with the inflation and the sink) and
//!    copying/skipping every off-path subtree exactly once.
//!
//! Both passes recurse on the `(id, ev)` shape, guarded by [`crate::recurse`] so
//! deep trees grow the stack onto the heap rather than overflowing; each returns
//! its subtree's end positions so a right sibling resumes without re-scanning.
//!
//! **Probe → emit contract.** The probe records a [`Choices`] entry for every
//! `(id, ev)` branch node it reaches, keyed by the same `(id_pos, ev_pos)`
//! coordinates the emit pass uses. `grow_emit` only follows the chosen path
//! (copying/skipping off-path subtrees), but every branch node it reaches was
//! recorded by the probe; the coordinate agreement is what lets the two passes
//! communicate by position.

use crate::codec::{Base, BitsSlice};
use crate::idbits::{IdHeader, IdView};
use crate::recurse::descend;

use super::{Builder, Built, VIRTUAL};
use crate::version::compare::{EvHeader, EvView};
use crate::version::working::WorkingVersion;

/// Lexicographic inflation cost `(expansions, depth)`: prefer fewer
/// leaf-to-node expansions, then a shallower spot. `MAX` ([`COST_MAX`]) marks
/// an infeasible (empty-id) region. Ties between a node's two children favor
/// the *right* child, to match the oracle's choice (see
/// [`grow_probe`](EvView::grow_probe)'s `left_chosen`).
type Cost = (u32, u32);

/// The cost of an infeasible region: an empty-id subtree can never be inflated.
const COST_MAX: Cost = (u32::MAX, u32::MAX);

/// The probe → emit channel: at each branch node, whether the cheapest
/// inflation descended into the *left* child (`true`) or the right (`false`).
///
/// The key `(id_pos, ev_pos)` has an alternating pinned axis (one coordinate is held
/// constant while the other descends — see the module doc), so no single array can be
/// keyed by one coordinate alone. Instead two dense arrays split by regime, which is
/// exactly "is the id a node?":
///
/// - [`by_id`](Choices::by_id): id is a node (`Expand`/`Both`), keyed by the id
///   bit-position. Each id internal node is visited once, so its slot is unique.
///
/// - [`by_ev`](Choices::by_ev): id is a full `1`-leaf (`FullEvNode`), keyed by
///   the event position. Each event node is reached under at most one id context.
///
/// Slots default to `None`; [`grow_probe`](EvView::grow_probe) fills the one
/// for each branch node it reaches and [`grow_emit`](EvView::grow_emit) reads
/// back the slot for the regime it is in. Total space `O(n + m)`, `O(1)` access
/// (no hashing).
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

/// A `grow` branch node's identity: its recursion shape and the `(id, ev)`
/// coordinates plus header ends needed to fold it (see [`ProbeWalk::combine`]).
#[derive(Clone, Copy)]
struct Branch {
    kind: Kind,
    id_pos: usize,
    ev_pos: usize,
    id_next: usize,
    ev_next: usize,
}

/// A probed `grow` subtree report: the cheapest inflation `cost`, plus where the
/// subtree ended in the packed id stream and the event tree (or [`VIRTUAL`]).
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
    /// Probe the cheapest inflation of this event tree (`self`), recording the
    /// chosen child direction (`true` = left) per `(id, ev)` branch node into
    /// `choice`. Read-only; `O(n + m)`. The id is lazy-skipped where an empty
    /// region prunes the event.
    ///
    /// This is the cost-finding half of the recursive form of
    /// `oracle::Version::grow` (the paper's `grow`): where the oracle recurses
    /// once and rebuilds on the way back up, this probe pass finds the cheapest
    /// path and [`grow_emit`](EvView::grow_emit) replays it.
    fn grow_probe(&self, id_bits: &BitsSlice, choice: &mut Choices) {
        let mut walk = ProbeWalk {
            view: *self,
            id: IdView(id_bits),
            choice,
        };
        descend!(0, walk.rec(0, 0, 0));
    }

    /// Emit the grown tree (`self` is the source event tree) using the probe's
    /// `choice` map, in normal form. `O(n + m)`: only the chosen root-to-leaf
    /// path is rebuilt (with the inflation and the sink); every off-path subtree
    /// is copied or skipped exactly once.
    ///
    /// This is the rebuilding half of the recursive form of
    /// `oracle::Version::grow`, replaying the choices
    /// [`grow_probe`](EvView::grow_probe) recorded.
    fn grow_emit(&self, id_bits: &BitsSlice, out: &mut Builder, choice: &Choices) {
        let mut walk = EmitWalk {
            view: *self,
            id: IdView(id_bits),
            out,
            choice,
        };
        descend!(0, walk.rec(0, 0, 0));
    }

    /// `grow(id, ev)`: register a new event on this event tree (`self`) by the
    /// cheapest available inflation, in normal form. Two passes — a read-only
    /// cost probe, then an emit along the chosen path — each `O(n + m)`. The
    /// probe and emit are the same traversal; see the module doc for the `(id,
    /// ev)`-coordinate contract that links them through [`Choices`].
    pub(super) fn grow(&self, id_bits: &BitsSlice) -> WorkingVersion {
        let mut choice = Choices::new(id_bits.len(), self.span());
        self.grow_probe(id_bits, &mut choice);
        let mut out = Builder::with_capacity(self.node_capacity_bound() + id_bits.len());
        self.grow_emit(id_bits, &mut out, &choice);
        out.finish()
    }
}

/// The mutable state of a [`grow_probe`](EvView::grow_probe) walk: the event view
/// and packed id being probed, and the choice map being filled.
struct ProbeWalk<'a> {
    view: EvView<'a>,
    id: IdView<'a>,
    choice: &'a mut Choices,
}

impl ProbeWalk<'_> {
    /// Probe the cheapest inflation of the event subtree at `ev_pos` (or
    /// [`VIRTUAL`]) under the id subtree at `id_pos`, routed through the
    /// amortized stack-growth guard. Returns the subtree's [`Probed`] report.
    fn rec(&mut self, id_pos: usize, ev_pos: usize, depth: usize) -> Probed {
        let id_hdr = self.id.header(id_pos);
        let id_next = id_hdr.next;
        let virt = ev_pos == VIRTUAL;
        let EvHeader {
            internal: ev_int,
            base: _,
            next: ev_next,
        } = if virt {
            EvHeader {
                internal: false,
                base: Base::ZERO,
                next: VIRTUAL,
            }
        } else {
            self.view.header(ev_pos)
        };
        if id_hdr.is_empty() {
            // id 0-leaf: infeasible; lazy-skip the dominated event subtree.
            let ev_end = if virt {
                VIRTUAL
            } else {
                self.view.skip(ev_pos)
            };
            return Probed {
                cost: COST_MAX,
                id_end: id_next,
                ev_end,
            };
        }
        if id_hdr.is_full() {
            if !ev_int {
                // increment here: a free inflation
                return Probed {
                    cost: (0, 0),
                    id_end: id_next,
                    ev_end: ev_next,
                };
            }
            // id stays full; descend both event children (right threaded).
            let left = descend!(depth + 1, self.rec(id_pos, ev_pos + 1, depth + 1));
            let right = descend!(depth + 1, self.rec(id_pos, left.ev_end, depth + 1));
            let branch = Branch {
                kind: Kind::FullEvNode,
                id_pos,
                ev_pos,
                id_next,
                ev_next,
            };
            return self.combine(branch, left, right);
        }
        if !ev_int {
            // id node, event leaf/virtual: expand and descend the id (the event
            // stays virtual on both sides).
            let left = descend!(depth + 1, self.rec(id_next, VIRTUAL, depth + 1));
            let right = descend!(depth + 1, self.rec(left.id_end, VIRTUAL, depth + 1));
            let branch = Branch {
                kind: Kind::Expand,
                id_pos,
                ev_pos,
                id_next,
                ev_next,
            };
            return self.combine(branch, left, right);
        }
        // id node, event node: descend both (right threaded from the left).
        let left = descend!(depth + 1, self.rec(id_next, ev_pos + 1, depth + 1));
        let right = descend!(depth + 1, self.rec(left.id_end, left.ev_end, depth + 1));
        let branch = Branch {
            kind: Kind::Both,
            id_pos,
            ev_pos,
            id_next,
            ev_next,
        };
        self.combine(branch, left, right)
    }

    /// Pick the cheaper child, record the direction, and fold the branch node's
    /// cost and end positions (a tie favors the right child; see [`Cost`]).
    fn combine(&mut self, branch: Branch, left: Probed, right: Probed) -> Probed {
        let Branch {
            kind,
            id_pos,
            ev_pos,
            id_next,
            ev_next,
        } = branch;
        // Strict `<` makes a tie favor the right child (see [`Cost`]).
        let left_chosen = left.cost < right.cost;
        self.choice.record(kind, id_pos, ev_pos, left_chosen);
        let m = if left_chosen { left.cost } else { right.cost };
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
        Probed {
            cost,
            id_end,
            ev_end,
        }
    }
}

/// The mutable state of a [`grow_emit`](EvView::grow_emit) walk: the source event
/// view and packed id, the output builder, and the probe's choice map.
struct EmitWalk<'a> {
    view: EvView<'a>,
    id: IdView<'a>,
    out: &'a mut Builder,
    choice: &'a Choices,
}

impl EmitWalk<'_> {
    /// Emit the grown event subtree at `ev_pos` (or [`VIRTUAL`]) under the id
    /// subtree at `id_pos`, following the probe's chosen path and copying every
    /// off-path subtree once. Routed through the amortized stack-growth guard;
    /// returns the subtree's [`Built`] report.
    fn rec(&mut self, id_pos: usize, ev_pos: usize, depth: usize) -> Built {
        let IdHeader {
            node: id_node,
            val: id_val,
            next: id_next,
        } = self.id.header(id_pos);
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
            self.view.header(ev_pos)
        };
        // The inflation point: id full over a leaf/virtual event — increment.
        if !id_node && !ev_int {
            // Invariant: the chosen path never reaches an empty (`0`-leaf)
            // id. A normal-form id node always has a nonempty child (its
            // min-cost child is never the `COST_MAX` empty side), and a real
            // `Party`'s root is never empty — so an id leaf on the chosen
            // path is always full.
            debug_assert!(id_val, "grow chose an empty-id region to inflate");
            return Built {
                out_root: self.out.leaf(ev_base + 1u32),
                id_end: id_next,
                ev_end: ev_next,
            };
        }
        let kind = if !id_node {
            Kind::FullEvNode
        } else if ev_int {
            Kind::Both
        } else {
            Kind::Expand
        };
        let node = self.out.open(ev_base);
        if self.choice.chosen(kind, id_pos, ev_pos) {
            // Chosen left child: build it now, emit the off-path right on close.
            let (child_id, child_ev) = match kind {
                Kind::FullEvNode => (id_pos, ev_pos + 1), // id stays full
                Kind::Both => (id_next, ev_pos + 1),      // `il`, `el`
                Kind::Expand => (id_next, VIRTUAL),       // `il`, virtual
            };
            let left = descend!(depth + 1, self.rec(child_id, child_ev, depth + 1));
            let (right_root, id_end, ev_end) = match kind {
                Kind::FullEvNode => {
                    let (rr, ev_node_end) = self.out.copy(&self.view, left.ev_end); // off-path `er`
                    (rr, id_next, ev_node_end)
                }
                Kind::Both => {
                    let (rr, ev_node_end) = self.out.copy(&self.view, left.ev_end); // off-path `er`
                    (rr, self.id.skip(left.id_end), ev_node_end)
                }
                Kind::Expand => {
                    let rr = self.out.leaf(Base::ZERO); // off-path sibling is a fresh Leaf(0)
                    (rr, self.id.skip(left.id_end), ev_next)
                }
            };
            self.out.close_node(node, right_root);
            Built {
                out_root: node,
                id_end,
                ev_end,
            }
        } else {
            // Chosen right child: emit the off-path left sibling now, then
            // build the chosen right.
            let (child_id, child_ev) = match kind {
                Kind::FullEvNode => {
                    let (_l, ev_right) = self.out.copy(&self.view, ev_pos + 1);
                    (id_pos, ev_right)
                }
                Kind::Both => {
                    let (_l, ev_right) = self.out.copy(&self.view, ev_pos + 1);
                    (self.id.skip(id_next), ev_right)
                }
                Kind::Expand => {
                    self.out.leaf(Base::ZERO);
                    (self.id.skip(id_next), VIRTUAL)
                }
            };
            let right = descend!(depth + 1, self.rec(child_id, child_ev, depth + 1));
            let (id_end, ev_end) = match kind {
                Kind::FullEvNode => (id_next, right.ev_end),
                Kind::Both => (right.id_end, right.ev_end),
                Kind::Expand => (right.id_end, ev_next),
            };
            self.out.close_node(node, right.out_root);
            Built {
                out_root: node,
                id_end,
                ev_end,
            }
        }
    }
}
