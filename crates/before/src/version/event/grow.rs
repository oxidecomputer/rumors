//! `grow`: register a new event when [`fill`](EvReader::fill) cannot simplify the
//! tree, by inflating the cheapest available leaf. Two passes, each `O(n + m)`:
//!
//! 1. [`grow_probe`](EvReader::grow_probe) — a read-only cost probe that walks the
//!    `(id, event)` shape and, at every branch node, records which child the
//!    cheapest inflation descends into.
//!
//! 2. [`grow_emit`](EvReader::grow_emit) — replays the probe's choices, rebuilding
//!    only the chosen root-to-leaf path (with the inflation and the sink) and
//!    copying/skipping every off-path subtree exactly once.
//!
//! Both passes recurse on the `(id, ev)` shape, guarded by [`crate::recurse`] so
//! deep trees grow the stack onto the heap rather than overflowing; each returns
//! its subtree's end positions so a right sibling resumes without re-scanning.
//!
//! **Probe → emit contract.** The probe records a [`Route`] direction for every
//! `(id, ev)` branch node it reaches, keyed by the same `(id_pos, ev_pos)`
//! coordinates the emit pass uses. `grow_emit` only follows the chosen path
//! (copying/skipping off-path subtrees), but every branch node it reaches was
//! recorded by the probe; the coordinate agreement is what lets the two passes
//! communicate by position.

use crate::codec::{Base, Bits, BitsSlice};
use crate::idbits::{IdNode, IdReader};
use crate::recurse::descend;

use super::Builder;
use crate::version::compare::{EvNode, EvReader};
use crate::version::working::WorkingVersion;

/// Lexicographic inflation cost `(expansions, depth)`: prefer fewer
/// leaf-to-node expansions, then a shallower spot. `MAX` ([`COST_MAX`]) marks
/// an infeasible (empty-id) region. Ties between a node's two children favor
/// the *right* child, to match the oracle's choice (see
/// [`grow_probe`](EvReader::grow_probe)'s `left_chosen`).
type Cost = (u32, u32);

/// The cost of an infeasible region: an empty-id subtree can never be inflated.
const COST_MAX: Cost = (u32::MAX, u32::MAX);

/// The probe → emit channel: the cheapest inflation's route to the leaf it
/// grows, as one direction *bit* per branch node — `true` = descend the left
/// child, `false` = the right. The paper's `grow` settles a single root-to-leaf
/// path; this records its turns. It is *keyed by branch position* rather than
/// stored as one linear path because the emit pass walks only the chosen path
/// while the probe visited every branch, so emit must look up its direction by
/// where it is, not read it off a sequence.
///
/// The key `(id_pos, ev_pos)` has an alternating pinned axis (one coordinate is
/// held constant while the other descends — see the module doc), so no single
/// array is keyed by one coordinate alone. Two bit-vectors split by regime,
/// which is exactly "is the id a node?":
///
/// - [`by_id`](Route::by_id): id is a node (`Expand`/`Both`), keyed by the id
///   bit-position. Each id internal node is visited once, so its bit is unique.
/// - [`by_ev`](Route::by_ev): id is a full `1`-leaf (`FullEvNode`), keyed by the
///   event position. Each event node is reached under at most one id context.
///
/// One `Bits` per axis — a direction is a single bit, so this is ~8x smaller
/// than the former `Vec<Option<bool>>` and one allocation each. `O(n + m)`
/// space, `O(1)` access. A bit defaults to `false` (left); a probe/emit
/// coordinate mismatch would therefore misread a direction rather than panic,
/// but the grow-optimality property tests (against the brute-force search)
/// catch any such disagreement.
struct Route {
    /// Direction bit at id-node branches, by id bit-position.
    by_id: Bits,
    /// Direction bit at full-`1`-leaf branches, by event position.
    by_ev: Bits,
}

impl Route {
    /// All directions cleared, sized to the id and event position spaces.
    fn new(id_span: usize, ev_span: usize) -> Self {
        Route {
            by_id: Bits::repeat(false, id_span),
            by_ev: Bits::repeat(false, ev_span),
        }
    }

    /// Record that the cheapest inflation at the branch keyed by `key` descends
    /// into the left child (`left = true`) or the right (`false`). `key` is the
    /// event position for `FullEvNode` (keyed `by_ev`) or the id position for
    /// `Expand`/`Both` (keyed `by_id`) — see [`Route`].
    fn record(&mut self, kind: Kind, key: usize, left: bool) {
        match kind {
            Kind::FullEvNode => self.by_ev.set(key, left),
            Kind::Expand | Kind::Both => self.by_id.set(key, left),
        }
    }

    /// Whether the cheapest inflation at the branch keyed by `key` descends into
    /// the left child.
    fn descends_left(&self, kind: Kind, key: usize) -> bool {
        match kind {
            Kind::FullEvNode => self.by_ev[key],
            Kind::Expand | Kind::Both => self.by_id[key],
        }
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

/// A `grow` branch node's identity for [`ProbeWalk::combine`]: its recursion
/// shape, the [`Route`] key to record the chosen direction at, and the readers
/// just past each input's node header (the threaded ends a leaf/full side
/// consumes).
#[derive(Clone, Copy)]
struct Branch<'a> {
    kind: Kind,
    /// The position to record the direction at: the event position for
    /// `FullEvNode` (keyed `by_ev`), else the id position (keyed `by_id`).
    key: usize,
    id_after: IdReader<'a>,
    ev_after: EvReader<'a>,
}

/// A probed `grow` subtree report: the cheapest inflation `cost`, plus readers
/// just past the subtree in the id and event inputs.
#[derive(Clone, Copy)]
struct Probed<'a> {
    /// The cheapest inflation cost of the subtree.
    cost: Cost,
    /// Reader just past the subtree in the id.
    id_end: IdReader<'a>,
    /// Reader just past the subtree in the event tree.
    ev_end: EvReader<'a>,
}

/// A grown `grow` subtree report (the emit pass): the output root, plus readers
/// just past the subtree in each input.
struct Grown<'a> {
    out_root: usize,
    id_end: IdReader<'a>,
    ev_end: EvReader<'a>,
}

impl<'a> EvReader<'a> {
    /// Probe the cheapest inflation of this event tree (`self`), recording the
    /// chosen child direction (`true` = left) per `(id, ev)` branch node into
    /// `route`. Read-only; `O(n + m)`. The id is lazy-skipped where an empty
    /// region prunes the event.
    ///
    /// This is the cost-finding half of the recursive form of
    /// `oracle::Version::grow` (the paper's `grow`): where the oracle recurses
    /// once and rebuilds on the way back up, this probe pass finds the cheapest
    /// path and [`grow_emit`](EvReader::grow_emit) replays it.
    fn grow_probe(self, id_bits: &'a BitsSlice, route: &mut Route) {
        let mut walk = ProbeWalk { route };
        descend!(0, walk.rec(IdReader::root(id_bits), self, 0));
    }

    /// Emit the grown tree (`self` is the source event tree) following the
    /// probe's `route`, in normal form. `O(n + m)`: only the chosen root-to-leaf
    /// path is rebuilt (with the inflation and the sink); every off-path subtree
    /// is copied or skipped exactly once.
    ///
    /// This is the rebuilding half of the recursive form of
    /// `oracle::Version::grow`, replaying the directions
    /// [`grow_probe`](EvReader::grow_probe) recorded.
    fn grow_emit(self, id_bits: &'a BitsSlice, out: &mut Builder, route: &Route) {
        let mut walk = EmitWalk { out, route };
        descend!(0, walk.rec(IdReader::root(id_bits), self, 0));
    }

    /// `grow(id, ev)`: register a new event on this event tree (`self`) by the
    /// cheapest available inflation, in normal form. Two passes — a read-only
    /// cost probe, then an emit along the chosen path — each `O(n + m)`. The
    /// probe and emit are the same traversal; see the module doc for the `(id,
    /// ev)`-coordinate contract that links them through the [`Route`].
    pub(super) fn grow(self, id_bits: &'a BitsSlice) -> WorkingVersion {
        let mut route = Route::new(id_bits.len(), self.span());
        self.grow_probe(id_bits, &mut route);
        let mut out = Builder::with_capacity(self.node_capacity_bound() + id_bits.len());
        self.grow_emit(id_bits, &mut out, &route);
        out.finish()
    }
}

/// The mutable state of a [`grow_probe`](EvReader::grow_probe) walk: just the
/// [`Route`] being filled (the readers carry the traversal state).
struct ProbeWalk<'a> {
    route: &'a mut Route,
}

impl ProbeWalk<'_> {
    /// Probe the cheapest inflation of the event subtree at `ev` under the id
    /// subtree at `id`, routed through the amortized stack-growth guard. Returns
    /// the subtree's [`Probed`] report. The event side bottoms out at
    /// [`Zero`](EvReader::Zero) — `grow`'s virtual leaf — read like any other
    /// `Leaf(0)`, so no sentinel guard is needed.
    fn rec<'a>(&mut self, id: IdReader<'a>, ev: EvReader<'a>, depth: usize) -> Probed<'a> {
        let (id_node, id_after) = id.read();
        let (ev_node, ev_after) = ev.read();
        let ev_internal = ev_node.is_internal();
        match id_node {
            IdNode::Empty => {
                // id 0-leaf: infeasible; lazy-skip the dominated event subtree.
                Probed {
                    cost: COST_MAX,
                    id_end: id_after,
                    ev_end: ev.skip(),
                }
            }
            IdNode::Full if !ev_internal => {
                // increment here: a free inflation
                Probed {
                    cost: (0, 0),
                    id_end: id_after,
                    ev_end: ev_after,
                }
            }
            IdNode::Full => {
                // id stays full; descend both event children (right threaded).
                let left = descend!(depth + 1, self.rec(id, ev_after, depth + 1));
                let right = descend!(depth + 1, self.rec(id, left.ev_end, depth + 1));
                let branch = Branch {
                    kind: Kind::FullEvNode,
                    key: ev.pos(),
                    id_after,
                    ev_after,
                };
                self.combine(branch, left, right)
            }
            IdNode::Internal if !ev_internal => {
                // id node, event leaf/virtual: expand and descend the id (the
                // event stays a virtual `Zero` on both sides).
                let left = descend!(depth + 1, self.rec(id_after, EvReader::Zero, depth + 1));
                let right = descend!(depth + 1, self.rec(left.id_end, EvReader::Zero, depth + 1));
                let branch = Branch {
                    kind: Kind::Expand,
                    key: id.pos(),
                    id_after,
                    ev_after,
                };
                self.combine(branch, left, right)
            }
            IdNode::Internal => {
                // id node, event node: descend both (right threaded from the left).
                let left = descend!(depth + 1, self.rec(id_after, ev_after, depth + 1));
                let right = descend!(depth + 1, self.rec(left.id_end, left.ev_end, depth + 1));
                let branch = Branch {
                    kind: Kind::Both,
                    key: id.pos(),
                    id_after,
                    ev_after,
                };
                self.combine(branch, left, right)
            }
        }
    }

    /// Pick the cheaper child, record the direction, and fold the branch node's
    /// cost and end readers (a tie favors the right child; see [`Cost`]).
    fn combine<'a>(
        &mut self,
        branch: Branch<'a>,
        left: Probed<'a>,
        right: Probed<'a>,
    ) -> Probed<'a> {
        let Branch {
            kind,
            key,
            id_after,
            ev_after,
        } = branch;
        // Strict `<` makes a tie favor the right child (see [`Cost`]).
        let left_chosen = left.cost < right.cost;
        self.route.record(kind, key, left_chosen);
        let m = if left_chosen { left.cost } else { right.cost };
        let cost = match kind {
            Kind::Expand => (m.0.saturating_add(1), m.1.saturating_add(1)),
            Kind::FullEvNode | Kind::Both => (m.0, m.1.saturating_add(1)),
        };
        let id_end = match kind {
            Kind::FullEvNode => id_after, // the 1-leaf is consumed
            Kind::Expand | Kind::Both => right.id_end,
        };
        let ev_end = match kind {
            Kind::FullEvNode | Kind::Both => right.ev_end,
            Kind::Expand => ev_after, // event leaf/virtual consumed
        };
        Probed {
            cost,
            id_end,
            ev_end,
        }
    }
}

/// The mutable state of a [`grow_emit`](EvReader::grow_emit) walk: the output
/// builder and the probe's [`Route`] (the readers carry the traversal state).
struct EmitWalk<'a> {
    out: &'a mut Builder,
    route: &'a Route,
}

impl EmitWalk<'_> {
    /// Emit the grown event subtree at `ev` under the id subtree at `id`,
    /// following the probe's chosen path and copying every off-path subtree
    /// once. Routed through the amortized stack-growth guard; returns the
    /// subtree's [`Grown`] report. The event side bottoms out at
    /// [`Zero`](EvReader::Zero), read like any other `Leaf(0)`.
    fn rec<'a>(&mut self, id: IdReader<'a>, ev: EvReader<'a>, depth: usize) -> Grown<'a> {
        let (id_node, id_after) = id.read();
        let (ev_node, ev_after) = ev.read();
        let ev_internal = ev_node.is_internal();
        let ev_base = match ev_node {
            EvNode::Leaf(ref b) | EvNode::Internal(ref b) => b.clone(),
        };
        let id_internal = matches!(id_node, IdNode::Internal);
        // The inflation point: id full over a leaf/virtual event — increment.
        if !id_internal && !ev_internal {
            // Invariant: the chosen path never reaches an empty (`0`-leaf) id. A
            // normal-form id node always has a nonempty child (its min-cost child
            // is never the `COST_MAX` empty side), and a real `Party`'s root is
            // never empty — so an id leaf on the chosen path is always full.
            debug_assert!(
                matches!(id_node, IdNode::Full),
                "grow chose an empty-id region to inflate",
            );
            return Grown {
                out_root: self.out.leaf(ev_base + 1u32),
                id_end: id_after,
                ev_end: ev_after,
            };
        }
        let kind = match id_node {
            IdNode::Internal if ev_internal => Kind::Both,
            IdNode::Internal => Kind::Expand,
            _ => Kind::FullEvNode, // id full leaf, event node
        };
        let key = match kind {
            Kind::FullEvNode => ev.pos(),
            Kind::Expand | Kind::Both => id.pos(),
        };
        let node = self.out.open(ev_base);
        if self.route.descends_left(kind, key) {
            // Chosen left child: build it now, emit the off-path right on close.
            let (child_id, child_ev) = match kind {
                Kind::FullEvNode => (id, ev_after),         // id stays full, `el`
                Kind::Both => (id_after, ev_after),         // `il`, `el`
                Kind::Expand => (id_after, EvReader::Zero), // `il`, virtual
            };
            let left = descend!(depth + 1, self.rec(child_id, child_ev, depth + 1));
            let (right_root, id_end, ev_end) = match kind {
                Kind::FullEvNode => {
                    let (rr, ev_end) = self.out.copy_reader(left.ev_end); // off-path `er`
                    (rr, id_after, ev_end)
                }
                Kind::Both => {
                    let (rr, ev_end) = self.out.copy_reader(left.ev_end); // off-path `er`
                    (rr, left.id_end.skip(), ev_end)
                }
                Kind::Expand => {
                    let rr = self.out.leaf(Base::ZERO); // off-path sibling is a fresh Leaf(0)
                    (rr, left.id_end.skip(), ev_after)
                }
            };
            self.out.close_node(node, right_root);
            Grown {
                out_root: node,
                id_end,
                ev_end,
            }
        } else {
            // Chosen right child: emit the off-path left sibling now, then
            // build the chosen right.
            let (child_id, child_ev) = match kind {
                Kind::FullEvNode => {
                    let (_l, ev_right) = self.out.copy_reader(ev_after);
                    (id, ev_right)
                }
                Kind::Both => {
                    let (_l, ev_right) = self.out.copy_reader(ev_after);
                    (id_after.skip(), ev_right)
                }
                Kind::Expand => {
                    self.out.leaf(Base::ZERO);
                    (id_after.skip(), EvReader::Zero)
                }
            };
            let right = descend!(depth + 1, self.rec(child_id, child_ev, depth + 1));
            let (id_end, ev_end) = match kind {
                Kind::FullEvNode => (id_after, right.ev_end),
                Kind::Both => (right.id_end, right.ev_end),
                Kind::Expand => (right.id_end, ev_after),
            };
            self.out.close_node(node, right.out_root);
            Grown {
                out_root: node,
                id_end,
                ev_end,
            }
        }
    }
}
