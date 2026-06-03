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
//! deep trees grow the stack onto the heap rather than overflowing, advancing
//! their `&mut` cursors in place so a right sibling resumes without re-scanning
//! (see the [traversal overview](super)).
//!
//! **Probe → emit contract.** The probe records a [`Route`] direction for every
//! `(id, ev)` branch node it reaches, keyed by the same `(id_pos, ev_pos)`
//! coordinates the emit pass uses. `grow_emit` only follows the chosen path
//! (copying/skipping off-path subtrees), but every branch node it reaches was
//! recorded by the probe; the coordinate agreement is what lets the two passes
//! communicate by position.
//!
//! **Worked example.** `grow` of id `(1, 0)` over event `(0, 1, 0)` — the id
//! owns the left half only. At the root (id node, event node) the probe descends
//! both: the left is an id-full leaf over an event leaf, a free increment (cost
//! `(0, 0)`); the right is an empty-id region, infeasible (`COST_MAX`). So it
//! records "descend left" at the root. The emit rebuilds the left leaf as
//! `Leaf(2)` and copies the untouched right, giving `(0, 2, 0)` — the cheapest
//! available inflation.

use crate::codec::{Base, Bits, BitsSlice};
use crate::idbits::{IdNode, IdReader};
use crate::recurse::descend;

use super::Builder;
use crate::version::compare::EvReader;
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
/// A branch is keyed by id bit-position (`Expand`/`Both`, where the id is a
/// node) or by event position (`FullEvNode`, where the id is a full `1`-leaf).
/// Those two position spaces both start at `0`, so keyed naively into one array
/// they would collide; instead they are *concatenated* into a single bit-vector
/// — id-node branches in `[0, id_span)`, full-`1`-leaf branches offset into
/// `[id_span, id_span + ev_span)`. The two blocks are disjoint, so the range a
/// bit falls in implicitly recovers its regime. Each branch's key is unique
/// within its block (each id node, and each event node under a full id, is
/// reached once). One allocation, `O(n + m)` bits, `O(1)` access — ~8x smaller
/// than the former `Vec<Option<bool>>` pair. A bit defaults to `false` (left); a
/// probe/emit mismatch would misread a direction rather than panic, but the
/// grow-optimality property tests (against the brute-force search) catch any
/// such disagreement.
struct Route {
    dirs: Bits,
    /// Start of the event-position block: a `FullEvNode` key `ev_pos` lives at
    /// `id_span + ev_pos`, an `Expand`/`Both` key `id_pos` at `id_pos`.
    id_span: usize,
}

impl Route {
    /// All directions cleared, sized to the concatenated id + event position
    /// spaces.
    fn new(id_span: usize, ev_span: usize) -> Self {
        Route {
            dirs: Bits::repeat(false, id_span + ev_span),
            id_span,
        }
    }

    /// The bit index for a branch of the given `kind` at position `key` — the id
    /// position for `Expand`/`Both`, the event position for `FullEvNode`
    /// (offset into the upper block).
    fn index(&self, kind: Kind, key: usize) -> usize {
        match kind {
            Kind::Expand | Kind::Both => key,
            Kind::FullEvNode => self.id_span + key,
        }
    }

    /// Record that the cheapest inflation at the branch keyed by `(kind, key)`
    /// descends into the left child (`left = true`) or the right (`false`).
    fn record(&mut self, kind: Kind, key: usize, left: bool) {
        let i = self.index(kind, key);
        self.dirs.set(i, left);
    }

    /// Whether the cheapest inflation at the branch keyed by `(kind, key)`
    /// descends into the left child.
    fn descends_left(&self, kind: Kind, key: usize) -> bool {
        self.dirs[self.index(kind, key)]
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

impl EvReader<'_> {
    /// Probe the cheapest inflation of this event tree (`self`), recording the
    /// chosen child direction (`true` = left) per `(id, ev)` branch node into
    /// `route`. Read-only; `O(n + m)`. The event side is lazy-skipped where an
    /// empty id region prunes it.
    ///
    /// This is the cost-finding half of the recursive form of
    /// `oracle::Version::grow` (the paper's `grow`): where the oracle recurses
    /// once and rebuilds on the way back up, this probe pass finds the cheapest
    /// path and [`grow_emit`](EvReader::grow_emit) replays it.
    fn grow_probe(self, id_bits: &BitsSlice, route: &mut Route) {
        let mut walk = ProbeWalk { route };
        let mut ev = self;
        let mut id = IdReader::root(id_bits);
        descend!(0, walk.rec(&mut id, &mut ev, 0));
    }

    /// Emit the grown tree (`self` is the source event tree) following the
    /// probe's `route`, in normal form. `O(n + m)`: only the chosen root-to-leaf
    /// path is rebuilt (with the inflation and the sink); every off-path subtree
    /// is copied or skipped exactly once.
    ///
    /// This is the rebuilding half of the recursive form of
    /// `oracle::Version::grow`, replaying the directions
    /// [`grow_probe`](EvReader::grow_probe) recorded.
    fn grow_emit(self, id_bits: &BitsSlice, out: &mut Builder, route: &Route) {
        let mut walk = EmitWalk { out, route };
        let mut ev = self;
        let mut id = IdReader::root(id_bits);
        descend!(0, walk.rec(&mut id, &mut ev, 0));
    }
}

/// `grow(id, ev)`: register a new event on the event tree `ev` by the cheapest
/// available inflation, in normal form. Two passes — a read-only cost probe,
/// then an emit along the chosen path — each `O(n + m)`. The probe and emit are
/// the same traversal; see the module doc for the `(id, ev)`-coordinate
/// contract that links them through the [`Route`].
///
/// Takes the source working form rather than a cursor: each pass builds its own
/// fresh cursor from it (as `tick` does), so the one operation that reads a tree
/// twice needs no cursor duplication.
pub(super) fn grow(ev: &WorkingVersion, id_bits: &BitsSlice) -> WorkingVersion {
    let mut route = Route::new(id_bits.len(), ev.base.len());
    // Conservative: the grown tree is the source plus the nodes a single
    // expansion adds along the chosen path, bounded by the id's bit length.
    let cap = ev.base.len() + id_bits.len();
    EvReader::working(ev).grow_probe(id_bits, &mut route);
    let mut out = Builder::with_capacity(cap);
    EvReader::working(ev).grow_emit(id_bits, &mut out, &route);
    out.finish()
}

/// The mutable state of a [`grow_probe`](EvReader::grow_probe) walk: just the
/// [`Route`] being filled (the `&mut` readers carry the traversal state).
struct ProbeWalk<'a> {
    route: &'a mut Route,
}

impl ProbeWalk<'_> {
    /// Probe the cheapest inflation of the event subtree at `ev` under the id
    /// subtree at `id`, advancing both readers past their subtrees and routing
    /// through the amortized stack-growth guard. Returns the subtree's cheapest
    /// [`Cost`]. A leaf/full side broadcasts a fresh synthetic — `grow`'s
    /// virtual leaf [`Zero`](EvReader::Zero) for an expanded event leaf, a
    /// [`Full`](IdReader::Full) id re-presented to both event children — read
    /// like any real node, so no sentinel guards are needed.
    fn rec(&mut self, id: &mut IdReader, ev: &mut EvReader, depth: usize) -> Cost {
        // Capture the keying positions before the reads advance the cursors. The
        // keying side (id for `Expand`/`Both`, ev for `FullEvNode`) is always a
        // real cursor; the synthetic side's `None` is never the chosen key.
        let id_pos = id.pos_opt();
        match id.read() {
            IdNode::Empty => {
                // id 0-leaf: infeasible; lazy-skip the dominated event subtree.
                ev.skip();
                COST_MAX
            }
            IdNode::Full => {
                let ev_pos = ev.pos_opt();
                if !ev.read().is_internal() {
                    return (0, 0); // a free inflation: increment this leaf
                }
                // id stays full; descend both event children (a synthetic `Full`
                // id re-presented to each), threading the event cursor.
                let mut full = IdReader::Full;
                let left = descend!(depth + 1, self.rec(&mut full, ev, depth + 1));
                let right = descend!(depth + 1, self.rec(&mut full, ev, depth + 1));
                self.combine(Kind::FullEvNode, ev_pos.unwrap(), left, right)
            }
            IdNode::Internal if !ev.read().is_internal() => {
                // id node, event leaf/virtual: expand and descend the id, the
                // event a virtual `Zero` on both sides.
                let mut z1 = EvReader::Zero;
                let mut z2 = EvReader::Zero;
                let left = descend!(depth + 1, self.rec(id, &mut z1, depth + 1));
                let right = descend!(depth + 1, self.rec(id, &mut z2, depth + 1));
                self.combine(Kind::Expand, id_pos.unwrap(), left, right)
            }
            IdNode::Internal => {
                // id node, event node: descend both, threading both cursors.
                let left = descend!(depth + 1, self.rec(id, ev, depth + 1));
                let right = descend!(depth + 1, self.rec(id, ev, depth + 1));
                self.combine(Kind::Both, id_pos.unwrap(), left, right)
            }
        }
    }

    /// Pick the cheaper child, record the direction at `key`, and fold the
    /// branch node's cost (a tie favors the right child; see [`Cost`]).
    fn combine(&mut self, kind: Kind, key: usize, left: Cost, right: Cost) -> Cost {
        // Strict `<` makes a tie favor the right child (see [`Cost`]).
        let left_chosen = left < right;
        self.route.record(kind, key, left_chosen);
        let m = if left_chosen { left } else { right };
        match kind {
            Kind::Expand => (m.0.saturating_add(1), m.1.saturating_add(1)),
            Kind::FullEvNode | Kind::Both => (m.0, m.1.saturating_add(1)),
        }
    }
}

/// The mutable state of a [`grow_emit`](EvReader::grow_emit) walk: the output
/// builder and the probe's [`Route`] (the `&mut` readers carry the traversal
/// state).
struct EmitWalk<'a> {
    out: &'a mut Builder,
    route: &'a Route,
}

impl EmitWalk<'_> {
    /// Emit the grown event subtree at `ev` under the id subtree at `id`,
    /// following the probe's chosen path, copying every off-path subtree once,
    /// and advancing both readers. Routed through the amortized stack-growth
    /// guard; returns the output root. The event side bottoms out at
    /// [`Zero`](EvReader::Zero), the id-full side at [`Full`](IdReader::Full),
    /// each read like any real node.
    fn rec(&mut self, id: &mut IdReader, ev: &mut EvReader, depth: usize) -> usize {
        let id_pos = id.pos_opt();
        let id_node = id.read();
        let ev_pos = ev.pos_opt();
        let ev_node = ev.read();
        let ev_internal = ev_node.is_internal();
        let ev_base = ev_node.base().clone();
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
            return self.out.leaf(ev_base + 1u32);
        }
        let kind = match id_node {
            IdNode::Internal if ev_internal => Kind::Both,
            IdNode::Internal => Kind::Expand,
            _ => Kind::FullEvNode, // id full leaf, event node
        };
        let key = match kind {
            Kind::FullEvNode => ev_pos.unwrap(),
            Kind::Expand | Kind::Both => id_pos.unwrap(),
        };
        let node = self.out.open(ev_base);
        // At this branch, one child is on the chosen inflation path (rebuilt by
        // recursion) and the other is off it (copied/skipped once). The reads
        // above already advanced `id`/`ev` past the node header — `id` is now at
        // `il`, `ev` at `el` (a `FullEvNode` keeps `id` full via a synthetic).
        if self.route.descends_left(kind, key) {
            // Left is chosen: rebuild it, then deal with the off-path right.
            let right = match kind {
                Kind::FullEvNode => {
                    let mut full = IdReader::Full;
                    descend!(depth + 1, self.rec(&mut full, ev, depth + 1)); // left `el`
                    self.out.copy_reader(ev) // off-path `er`
                }
                Kind::Both => {
                    descend!(depth + 1, self.rec(id, ev, depth + 1)); // left `il`/`el`
                    let right = self.out.copy_reader(ev); // off-path `er`
                    id.skip(); // off-path `ir`
                    right
                }
                Kind::Expand => {
                    let mut z = EvReader::Zero;
                    descend!(depth + 1, self.rec(id, &mut z, depth + 1)); // left `il`, virtual
                    id.skip(); // off-path `ir`
                    self.out.leaf(Base::ZERO) // off-path sibling is a fresh Leaf(0)
                }
            };
            self.out.close_node(node, right);
        } else {
            // Right is chosen: emit the off-path left, then rebuild the right.
            let right = match kind {
                Kind::FullEvNode => {
                    self.out.copy_reader(ev); // off-path `el`
                    let mut full = IdReader::Full;
                    descend!(depth + 1, self.rec(&mut full, ev, depth + 1)) // right `er`
                }
                Kind::Both => {
                    self.out.copy_reader(ev); // off-path `el`
                    id.skip(); // off-path `il`
                    descend!(depth + 1, self.rec(id, ev, depth + 1)) // right `ir`/`er`
                }
                Kind::Expand => {
                    self.out.leaf(Base::ZERO); // off-path sibling is a fresh Leaf(0)
                    id.skip(); // off-path `il`
                    let mut z = EvReader::Zero;
                    descend!(depth + 1, self.rec(id, &mut z, depth + 1)) // right `ir`, virtual
                }
            };
            self.out.close_node(node, right);
        }
        node
    }
}
