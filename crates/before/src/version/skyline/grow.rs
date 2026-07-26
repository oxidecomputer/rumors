//! `grow` on skyline streams: an iterative bit-coded cost probe, then a
//! splice emit that rebuilds exactly one root-to-leaf path.
//!
//! `grow` registers a new event when `fill` cannot simplify the tree, by
//! inflating the cheapest available leaf — cheapest by the lexicographic
//! cost `(expansions, depth)`, ties favoring the right child. Two passes
//! over the same `(id, event)` shape, each `O(n + m)` in the two streams'
//! bits:
//!
//! 1. `probe` — a read-only, *topology-only* cost walk (leaf payload
//!    codes are skipped, never decoded) that records which child the
//!    cheapest inflation descends into at every branch node, into the
//!    position-keyed `Route`.
//! 2. `emit` — a replay of the route along the one chosen path: every
//!    off-path subtree is copied as a verbatim bit range, the inflation
//!    point is re-coded, and the two payload codes the height change can
//!    reach — the grown leaf's own delta and its preorder successor's —
//!    are the only codes re-derived. Collapse at the inflation point
//!    rides the output builder's absorb/re-anchor cascade unchanged.
//!
//! # The three branch regimes
//!
//! A branch node's `(id, event)` shape fixes its cost formula and what
//! lies beneath it, and the regimes *absorb*: nothing below them ever
//! returns to a two-cursor walk, so the walk needs no synthetic cursor
//! state, only the regime bit itself.
//!
//! - **`Both`** (id node over event node): descend both streams.
//! - **`FullEvNode`** (full `1` id leaf over an event node): the id
//!   contributes nothing below; descend the event alone.
//! - **`Expand`** (id node over an event leaf): the event is a virtual
//!   zero below; descend the id alone — an iterative id scan whose every
//!   level costs one expansion.
//!
//! An absent id child is infeasible (`COST_MAX`) and consumes nothing;
//! its dominated event sibling range is skipped without decoding.
//!
//! # The bit-coded frame stack
//!
//! The probe holds its suspended ancestors as two parallel bit stacks —
//! fixed-width control (2 kind bits + 1 phase bit per frame) and
//! variable-width values (a pop-able unary+value integer stack) — instead
//! of machine-word frames. The width is load-bearing, not polish: the
//! alternating-binary spine packs one branch node into ~4 stream bits, so
//! any walk keeping a fixed 16-byte frame per level materializes ~32
//! bytes of transient per input byte, past the resource envelopes'
//! ceiling; these frames cost a few bits per level. Per frame:
//!
//! - **kind** (2 control bits) and **phase** (1 control bit, flipped in
//!   place when the left child's cost returns);
//! - **key delta** (value stack): the branch's `Route` key, coded
//!   relative to the nearest same-regime ancestor's key. Two registers —
//!   one per keying regime — carry the running keys: a push stores
//!   `key − register` and overwrites the register, a pop restores it by
//!   subtraction, so the full key is never stored and never wider than
//!   its preorder gap.
//! - **the deferred left cost** (value stack, pushed at the phase flip):
//!   one infeasibility flag, then the feasible cost's two components.
//!
//! Nothing else is saved. In particular the id children's presence bits
//! are *re-read* from the id stream at the frame's key when the walk
//! resumes — the key is already in the register, and the 2-bit tag is an
//! `O(1)` random access — rather than stored per frame; a frame that
//! cached them would be wider for information the input already holds.
//!
//! # The splice emit
//!
//! Every consecutive-leaf delta between two leaves that both lie outside
//! the chosen path is unchanged by the inflation, so the emit copies
//! off-path subtrees as verbatim bit ranges and repairs exactly one
//! boundary delta per splice edge that can change:
//!
//! - the grown leaf's own code moves by `+1` (decoded, stepped,
//!   re-coded — the one payload the probe never read);
//! - the first leaf *after* the inflation point moves by `−1` exactly
//!   when the grown leaf is its preorder predecessor;
//! - an expansion chain's fresh sibling leaves are `0`/`±1` deltas by
//!   construction, coded directly.
//!
//! The pre-chosen prefix — path node flags and whole left off-path
//! subtrees — precedes every changed height, so it is spliced with no
//! repair at all. The output runs through the collapsing builder (the
//! `build` sibling module), whose absorb/re-anchor cascade performs the
//! one normalization an increment can trigger (the grown leaf equaling
//! its sibling leaf); off-path ranges pass through it untouched because
//! the input was canonical.
//!
//! # Cost
//!
//! Derived, with the constants pinned by the `skyline_grow_*` rows of
//! the resource-envelope suite (`tests/meter.rs`): the probe reads every
//! topology bit of both streams at most once and skips payload codes by
//! width; frames cost bits per level as above; the emit's splices copy
//! each off-path bit once and its repairs decode/re-code two payload
//! codes. Transient state is the frame stacks, the `Route` (one bit
//! per input bit), the emit's pending-sibling path bits, and the
//! builder's per-level stacks — no node array, no per-level machine
//! word, zero grown stack segments.
//!
//! # Testing
//!
//! The recursive oracle's `grow` is the behavioral witness: the grown
//! stream must equal the oracle's normalized, encoded inflation byte
//! for byte, over the adversarial families crossed with adversarial
//! parties, arbitrary pairs, organic histories, and the exhaustive
//! small scope; the same pool is held to the brute-force
//! minimal-inflation search directly, and deep spines to closed-form
//! expected values. The `Route` contract is pinned separately: the
//! iterative probe's bit vector must equal a reference recursive
//! probe's on every pair — the explicit guard against a probe/emit
//! coordinate drift, which would misread a direction silently rather
//! than panic.

use crate::codec::{self, Base, Bits, BitsSlice};
use crate::step;

use super::build::SkylineBuilder;
use super::{gamma_code, unzigzag, zigzag_signed};

/// Lexicographic inflation cost `(expansions, depth)`: prefer fewer
/// leaf-to-node expansions, then a shallower spot.
///
/// `MAX` ([`COST_MAX`]) marks an infeasible (empty-id) region. Ties
/// between a node's two children favor the *right* child (strict `<`
/// picks the left only when strictly cheaper).
type Cost = (u32, u32);

/// The cost of an infeasible region: an empty-id subtree can never be
/// inflated.
const COST_MAX: Cost = (u32::MAX, u32::MAX);

/// Control bits per suspended probe frame: 2 kind bits + 1 phase bit.
const FRAME_CTRL_BITS: usize = 3;

/// `grow(ev, id)`: register a new event on the version a skyline stream
/// denotes, from the perspective of a packed id, as a canonical skyline
/// stream.
///
/// The cheapest available inflation by `(expansions, depth)`, ties
/// right-favoring; one read-only probe then one splice emit, each
/// `O(n + m)` in the streams' bits (the module doc carries both
/// mechanisms and the cost argument). The output is byte-identical to
/// the recursive oracle's inflation (the differential suite pins it).
///
/// The id must own at least one region: `grow` is the fallback of `tick`
/// on a real party, and an id owning nothing has no feasible inflation
/// anywhere.
///
/// # Panics
///
/// Panics if the event operand is not a canonical skyline stream — run
/// [`validate`](fn@super::validate) first on untrusted bytes. Debug
/// builds also panic when the id owns nothing (the precondition above;
/// the result on an empty id is unspecified in release builds).
pub fn grow(ev_bits: &BitsSlice, id: &crate::Party) -> Bits {
    let id_bits = id.as_bits();
    debug_assert!(
        !id_bits.is_empty(),
        "grow requires an id owning at least one region"
    );
    let mut route = Route::new(id_bits.len(), ev_bits.len());
    probe(ev_bits, id_bits, &mut route);
    // Canonicalizing the storage is `Version::from_bits`'s job, the
    // single gate a stream passes through when it becomes a stored value.
    emit(ev_bits, id_bits, &route)
}

/// A grow probe with its route storage pre-allocated, so the
/// resource-envelope suite can measure the probe's transient frame
/// stacks alone.
///
/// Production callers use [`grow`], which allocates and discards the
/// route internally. This handle exists because the route — one bit per
/// input bit, by design — would otherwise dominate a measurement whose
/// subject is the probe's per-level frame state, the quantity the
/// deep-spine stack pin bounds.
#[cfg(any(test, feature = "meter"))]
pub struct Probe(Route);

#[cfg(any(test, feature = "meter"))]
impl Probe {
    /// Pre-allocate route storage sized to the two operands.
    pub fn for_operands(ev: &BitsSlice, id: &crate::Party) -> Self {
        Probe(Route::new(id.as_bits().len(), ev.len()))
    }

    /// Run the probe over the operands, filling the pre-allocated route.
    ///
    /// # Panics
    ///
    /// Panics if the event operand is not a canonical skyline stream,
    /// exactly as [`grow`] does; the operands must be the pair the
    /// storage was sized for.
    pub fn run(&mut self, ev: &BitsSlice, id: &crate::Party) {
        probe(ev, id.as_bits(), &mut self.0);
    }
}

/// Which `(id, event)` shape a `grow` branch node has — fixes its cost
/// formula, its [`Route`] keying side, and the regime beneath it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    /// id is a node, event is a node: descend both.
    Both,
    /// id is a node, event is a leaf (or virtually zero below one):
    /// expand the leaf (one expansion), descend the id alone.
    Expand,
    /// id is the full `1` leaf, event is a node: descend the event
    /// alone, the id full below.
    FullEvNode,
}

impl Kind {
    /// The two control bits this kind stores in a frame.
    fn ctrl(self) -> (bool, bool) {
        match self {
            Kind::Both => (false, false),
            Kind::Expand => (false, true),
            Kind::FullEvNode => (true, false),
        }
    }

    /// The kind two frame control bits store.
    fn from_ctrl(hi: bool, lo: bool) -> Kind {
        match (hi, lo) {
            (false, false) => Kind::Both,
            (false, true) => Kind::Expand,
            (true, false) => Kind::FullEvNode,
            (true, true) => unreachable!("a probe frame stores one of three kinds"),
        }
    }
}

/// The probe → emit channel: the cheapest inflation's route, one
/// direction *bit* per branch node — `true` = descend the left child.
///
/// Keyed by branch position rather than stored as one linear path
/// because the emit walks only the chosen path while the probe visited
/// every branch, so the emit must look its direction up by where it is.
/// A branch is keyed by its id bit position (`Both`/`Expand`, where the
/// id is a node) or by its event bit position (`FullEvNode`, where the
/// id is a full leaf); the two position spaces both start at `0`, so
/// they are concatenated — id-keyed branches in `[0, id_span)`,
/// event-keyed branches offset into `[id_span, id_span + ev_span)`.
/// Each branch's key is unique within its block (each id node, and each
/// event node under a full id, is reached once). One allocation,
/// `O(n + m)` bits, `O(1)` access, write order irrelevant. A bit
/// defaults to `false` (right); a probe/emit mismatch would misread a
/// direction rather than panic, which is why the route differential
/// pins the iterative probe against a reference recursive one bit for
/// bit.
struct Route {
    dirs: Bits,
    /// Start of the event-position block: a `FullEvNode` key `ev_pos`
    /// lives at `id_span + ev_pos`, a `Both`/`Expand` key `id_pos` at
    /// `id_pos`.
    id_span: usize,
}

impl Route {
    /// All directions cleared, sized to the concatenated id + event
    /// position spaces.
    fn new(id_span: usize, ev_span: usize) -> Self {
        Route {
            dirs: Bits::repeat(false, id_span + ev_span),
            id_span,
        }
    }

    /// The bit index for a branch of the given `kind` at position `key`.
    fn index(&self, kind: Kind, key: usize) -> usize {
        match kind {
            Kind::Both | Kind::Expand => key,
            Kind::FullEvNode => self.id_span + key,
        }
    }

    /// Record that the cheapest inflation at the branch keyed by
    /// `(kind, key)` descends into the left child (`left = true`).
    fn record(&mut self, kind: Kind, key: usize, left: bool) {
        let i = self.index(kind, key);
        self.dirs.set(i, left);
    }

    /// Whether the cheapest inflation at the branch keyed by
    /// `(kind, key)` descends into the left child.
    fn descends_left(&self, kind: Kind, key: usize) -> bool {
        self.dirs[self.index(kind, key)]
    }
}

/// A forward topology cursor over one skyline stream.
///
/// Reads node flags and *skips* leaf payload codes by width — the probe
/// never materializes a height — and skips whole subtrees with a
/// pending-children counter. The emit shares it, additionally borrowing
/// specific payload ranges the repairs decode.
struct EvScan<'a> {
    bits: &'a BitsSlice,
    pos: usize,
}

impl<'a> EvScan<'a> {
    /// A cursor at the stream's root.
    fn new(bits: &'a BitsSlice) -> Self {
        EvScan { bits, pos: 0 }
    }

    /// The cursor's bit position: the next node's flag, and the key of
    /// an event-keyed branch about to be read.
    fn pos(&self) -> usize {
        self.pos
    }

    /// Decode the node at the cursor and advance past its header:
    /// `None` for an internal node (now at its left child), or the
    /// leaf's payload code range (now past it).
    ///
    /// # Panics
    ///
    /// Panics if the stream is not a canonical skyline encoding.
    fn read(&mut self) -> Option<core::ops::Range<usize>> {
        step!();
        codec::scan::record_bits(1);
        let internal = self.bits[self.pos];
        self.pos += 1;
        if internal {
            None
        } else {
            let start = self.pos;
            self.pos = codec::skip_int(self.bits, start).expect("canonical skyline bits");
            Some(start..self.pos)
        }
    }

    /// Advance past the whole subtree at the cursor without decoding
    /// anything.
    ///
    /// # Panics
    ///
    /// Panics if the stream is not a canonical skyline encoding.
    fn skip(&mut self) {
        let bits = self.bits;
        self.pos = crate::idbits::skip_subtree(self.pos, |at| {
            step!();
            codec::scan::record_bits(1);
            if bits[at] {
                (2, at + 1)
            } else {
                let next = codec::skip_int(bits, at + 1).expect("canonical skyline bits");
                (0, next)
            }
        });
    }
}

/// Decode the 2-bit id tag at `pos`: `(left_present, right_present)`.
///
/// Neither present is the full `1` terminal; a canonical id has no
/// `(0, 0)` node. `O(1)`, which is what lets a resuming frame re-read
/// its children's presence at its key instead of storing it.
fn id_tag(bits: &BitsSlice, pos: usize) -> (bool, bool) {
    step!();
    codec::scan::record_bits(2);
    (bits[pos], bits[pos + 1])
}

/// Position just past the id subtree whose tag sits at `pos`.
fn id_skip(bits: &BitsSlice, pos: usize) -> usize {
    crate::idbits::skip_subtree(pos, |at| {
        step!();
        codec::scan::record_bits(2);
        let children = usize::from(bits[at]) + usize::from(bits[at + 1]);
        (children, at + 2)
    })
}

/// A pop-able stack of nonnegative integers held as bits.
///
/// Each entry costs `2·w` bits for a `w`-bit value: the value's bits on
/// one stack, and `w` in pop-able unary — a terminator under `w − 1`
/// continuation bits — on the other. Depth therefore costs bits here
/// the same way it does in the control stack, keeping the probe's
/// transient free of a per-level machine word.
struct PopStack {
    /// Width markers: for each entry, one `false` under `w − 1` `true`s.
    unary: Bits,
    /// Value bits, most-significant pushed first so pops read the value
    /// least-significant first.
    value: Bits,
}

impl PopStack {
    fn new() -> Self {
        PopStack {
            unary: Bits::new(),
            value: Bits::new(),
        }
    }

    /// Push a value (zero included: it stores one value bit).
    fn push(&mut self, v: u64) {
        let width = (u64::BITS - v.leading_zeros()).max(1);
        for i in (0..width).rev() {
            self.value.push(v >> i & 1 == 1);
        }
        self.unary.push(false);
        for _ in 1..width {
            self.unary.push(true);
        }
    }

    /// Pop the most recently pushed value.
    ///
    /// # Panics
    ///
    /// Panics if the stack is empty.
    fn pop(&mut self) -> u64 {
        let mut width = 0u32;
        loop {
            let continuation = self.unary.pop().expect("probe value stack underflow");
            width += 1;
            if !continuation {
                break;
            }
        }
        let mut v = 0u64;
        for i in 0..width {
            if self
                .value
                .pop()
                .expect("probe value stack value bits underflow")
            {
                v |= 1 << i;
            }
        }
        v
    }

    /// Push a deferred left cost: an infeasibility flag on top of the
    /// feasible components, so the pop decides from one entry.
    fn push_cost(&mut self, cost: Cost) {
        if cost == COST_MAX {
            self.push(1);
        } else {
            self.push(u64::from(cost.0));
            self.push(u64::from(cost.1));
            self.push(0);
        }
    }

    /// Pop a deferred left cost.
    fn pop_cost(&mut self) -> Cost {
        if self.pop() == 1 {
            COST_MAX
        } else {
            let depth = self.pop();
            let expansions = self.pop();
            (expansions as u32, depth as u32)
        }
    }
}

/// The probe's suspended-ancestor control stack: fixed-width frames of
/// kind and phase bits, mutated in place at the top.
struct Frames {
    ctrl: Bits,
}

impl Frames {
    fn new() -> Self {
        Frames { ctrl: Bits::new() }
    }

    /// Push a frame in its left phase.
    fn push(&mut self, kind: Kind) {
        let (hi, lo) = kind.ctrl();
        self.ctrl.push(hi);
        self.ctrl.push(lo);
        self.ctrl.push(false);
    }

    /// The top frame's kind and whether it is in its right phase.
    fn top(&self) -> Option<(Kind, bool)> {
        if self.ctrl.is_empty() {
            return None;
        }
        let base = self.ctrl.len() - FRAME_CTRL_BITS;
        let kind = Kind::from_ctrl(self.ctrl[base], self.ctrl[base + 1]);
        Some((kind, self.ctrl[base + 2]))
    }

    /// Flip the top frame into its right phase.
    fn flip_top(&mut self) {
        let i = self.ctrl.len() - 1;
        self.ctrl.set(i, true);
    }

    /// Pop the top frame.
    fn pop(&mut self) {
        self.ctrl.truncate(self.ctrl.len() - FRAME_CTRL_BITS);
    }
}

/// The id side of a pending descent: the real cursor, the full regime,
/// or an absent (infeasible) child.
#[derive(Clone, Copy)]
enum IdArm {
    /// The id cursor addresses a real node here.
    At,
    /// A full `1` region: everything below is owned.
    Full,
    /// An absent child: nothing below is owned.
    Empty,
}

/// One probe transition: descend into a subtree, or carry a finished
/// subtree's cost up to the innermost suspended frame.
enum Step {
    /// Enter the subtree whose id side is `id` and whose event side is
    /// the shared cursor (or the virtual zero below an expanded leaf).
    Enter { id: IdArm, ev_zero: bool },
    /// Return a computed subtree cost to the top frame.
    Rise(Cost),
}

/// Probe the cheapest inflation of `(id, event)`, recording the chosen
/// child direction per branch node into `route`.
///
/// One loop over the two forward cursors and the bit-coded frame stack
/// (the module doc carries the frame layout and the regime argument);
/// read-only and topology-only. Returns the root's cost.
fn probe(ev_bits: &BitsSlice, id_bits: &BitsSlice, route: &mut Route) -> Cost {
    let mut ev = EvScan::new(ev_bits);
    let mut id_pos = 0usize;
    let mut frames = Frames::new();
    let mut values = PopStack::new();
    // The nearest same-regime ancestor's key, one register per keying
    // regime: pushes store deltas against these, pops restore them.
    let mut reg_id = 0usize;
    let mut reg_ev = 0usize;
    let root = if id_bits.is_empty() {
        IdArm::Empty
    } else {
        IdArm::At
    };
    let mut step = Step::Enter {
        id: root,
        ev_zero: false,
    };
    loop {
        step = match step {
            // An absent id child: infeasible, and its dominated event
            // range (when real) is skipped without decoding.
            Step::Enter {
                id: IdArm::Empty,
                ev_zero,
            } => {
                if !ev_zero {
                    ev.skip();
                }
                Step::Rise(COST_MAX)
            }
            // A full id over the virtual zero leaf: a free increment.
            Step::Enter {
                id: IdArm::Full,
                ev_zero: true,
            } => Step::Rise((0, 0)),
            // A full id over a real event: a leaf is a free increment;
            // a node opens a `FullEvNode` frame keyed by event position.
            Step::Enter {
                id: IdArm::Full,
                ev_zero: false,
            } => {
                let key = ev.pos();
                if ev.read().is_none() {
                    frames.push(Kind::FullEvNode);
                    values.push((key - reg_ev) as u64);
                    reg_ev = key;
                    Step::Enter {
                        id: IdArm::Full,
                        ev_zero: false,
                    }
                } else {
                    Step::Rise((0, 0))
                }
            }
            // A real id node: a full leaf hands the event to the full
            // regime; an internal node opens a `Both` or `Expand` frame
            // keyed by id position.
            Step::Enter {
                id: IdArm::At,
                ev_zero,
            } => {
                let key = id_pos;
                let (l, r) = id_tag(id_bits, id_pos);
                id_pos += 2;
                if !l && !r {
                    Step::Enter {
                        id: IdArm::Full,
                        ev_zero,
                    }
                } else {
                    // Under a virtual zero the event cursor must not
                    // move, so the read is short-circuited away; over a
                    // real event the read consumes the node header (and
                    // a leaf's code) exactly once.
                    let kind = if ev_zero || ev.read().is_some() {
                        Kind::Expand
                    } else {
                        Kind::Both
                    };
                    frames.push(kind);
                    values.push((key - reg_id) as u64);
                    reg_id = key;
                    Step::Enter {
                        id: if l { IdArm::At } else { IdArm::Empty },
                        ev_zero: kind == Kind::Expand,
                    }
                }
            }
            Step::Rise(cost) => match frames.top() {
                None => {
                    debug_assert_eq!(ev.pos(), ev_bits.len(), "the probe consumes the event");
                    debug_assert_eq!(id_pos, id_bits.len(), "the probe consumes the id");
                    return cost;
                }
                // The left child's cost arrived: defer it and descend
                // the right child, re-reading id-child presence at the
                // frame's key (the current register).
                Some((kind, false)) => {
                    frames.flip_top();
                    values.push_cost(cost);
                    match kind {
                        Kind::FullEvNode => Step::Enter {
                            id: IdArm::Full,
                            ev_zero: false,
                        },
                        Kind::Both | Kind::Expand => {
                            let (_, r) = id_tag(id_bits, reg_id);
                            Step::Enter {
                                id: if r { IdArm::At } else { IdArm::Empty },
                                ev_zero: kind == Kind::Expand,
                            }
                        }
                    }
                }
                // Both children are costed: pick, record, fold, rise.
                Some((kind, true)) => {
                    let left = values.pop_cost();
                    let right = cost;
                    let key = match kind {
                        Kind::FullEvNode => reg_ev,
                        Kind::Both | Kind::Expand => reg_id,
                    };
                    let delta = values.pop() as usize;
                    match kind {
                        Kind::FullEvNode => reg_ev = key - delta,
                        Kind::Both | Kind::Expand => reg_id = key - delta,
                    }
                    frames.pop();
                    // Strict `<` makes a tie favor the right child (see
                    // [`Cost`]).
                    let left_chosen = left < right;
                    route.record(kind, key, left_chosen);
                    let m = if left_chosen { left } else { right };
                    Step::Rise(match kind {
                        Kind::Expand => (m.0.saturating_add(1), m.1.saturating_add(1)),
                        Kind::Both | Kind::FullEvNode => (m.0, m.1.saturating_add(1)),
                    })
                }
            },
        };
    }
}

/// The boundary repair a spliced subtree's first payload code needs.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Repair {
    /// The predecessor leaf is unchanged: copy the code verbatim.
    None,
    /// The predecessor is the grown leaf, one higher than it was: the
    /// delta drops by one.
    MinusOne,
}

/// One complete off-path subtree, located by a forward topology scan.
struct Subtree {
    /// Just past the subtree's last bit.
    end: usize,
    /// The first (leftmost) leaf's payload code range.
    first_code: core::ops::Range<usize>,
    /// The first leaf's depth below the subtree root; `0` means the
    /// subtree is a single leaf.
    first_rel_depth: usize,
    /// The last (rightmost) leaf's payload code range.
    last_code: core::ops::Range<usize>,
    /// The last leaf's depth below the subtree root.
    last_rel_depth: usize,
}

/// Locate the subtree at `start`: its end, and the first/last leaf
/// coordinates the verbatim splice re-anchors the builder around.
///
/// One forward pass over the subtree's bits with a relative-path bit
/// stack — the same walk the leaf cursors do, restricted to one subtree.
///
/// # Panics
///
/// Panics if the stream is not a canonical skyline encoding.
fn scan_subtree(bits: &BitsSlice, start: usize) -> Subtree {
    let mut pos = start;
    let mut path = Bits::new();
    // Descend to the leftmost leaf.
    loop {
        step!();
        codec::scan::record_bits(1);
        let internal = bits[pos];
        pos += 1;
        if !internal {
            break;
        }
        path.push(false);
    }
    let code_start = pos;
    pos = codec::skip_int(bits, pos).expect("canonical skyline bits");
    let first_code = code_start..pos;
    let first_rel_depth = path.len();
    let mut last_code = first_code.clone();
    let mut last_rel_depth = first_rel_depth;
    loop {
        // Close the ancestors the consumed leaf completed; an emptied
        // path means it was the subtree's last leaf.
        let mut flipped = false;
        while let Some(bit) = path.pop() {
            if !bit {
                path.push(true);
                flipped = true;
                break;
            }
        }
        if !flipped {
            break;
        }
        // Descend to the next leaf.
        loop {
            step!();
            codec::scan::record_bits(1);
            let internal = bits[pos];
            pos += 1;
            if !internal {
                break;
            }
            path.push(false);
        }
        let code_start = pos;
        pos = codec::skip_int(bits, pos).expect("canonical skyline bits");
        last_code = code_start..pos;
        last_rel_depth = path.len();
    }
    Subtree {
        end: pos,
        first_code,
        first_rel_depth,
        last_code,
        last_rel_depth,
    }
}

/// Feed one whole off-path subtree from the cursor into the builder,
/// rooted at `depth`.
///
/// The first leaf goes through the builder's collapse checks (with the
/// successor repair when the grown leaf precedes it); the remainder is
/// one verbatim splice.
fn feed_subtree(out: &mut SkylineBuilder, ev: &mut EvScan<'_>, depth: usize, repair: Repair) {
    let info = scan_subtree(ev.bits, ev.pos);
    let orig = &ev.bits[info.first_code.clone()];
    let first_code = match repair {
        Repair::None => orig.to_bitvec(),
        // The successor is never the stream's first leaf (the grown
        // leaf precedes it), so its code is always a zigzag delta.
        Repair::MinusOne => recode(orig, false, false),
    };
    out.leaf(depth + info.first_rel_depth, first_code);
    if info.first_rel_depth > 0 {
        out.continue_verbatim(
            &ev.bits[info.first_code.end..info.end],
            depth,
            info.last_rel_depth,
            info.last_code.len(),
        );
    }
    ev.pos = info.end;
}

/// Re-code one leaf payload with its height stepped by one:
/// `increment` up for the grown leaf, down for its successor.
///
/// `absolute` distinguishes the stream's first leaf (a plain gamma
/// height) from every later one (a zigzag delta). One decode, one
/// signed step, one re-encode — `O(the code's own width)`, the only
/// payload arithmetic in the whole emit.
fn recode(code: &BitsSlice, increment: bool, absolute: bool) -> Bits {
    let (value, end) = codec::decode_int(code, 0).expect("canonical skyline bits");
    debug_assert_eq!(end, code.len(), "a payload range is exactly one code");
    let one = Base::from(1u8);
    if absolute {
        debug_assert!(increment, "only the grown leaf re-codes an absolute height");
        return gamma_code(&(value + 1u32));
    }
    let (negative, magnitude) = unzigzag(value);
    let stepped = match (increment, negative) {
        // Stepping a nonnegative delta up, or a negative one further
        // down, grows the magnitude.
        (true, false) | (false, true) => zigzag_signed(negative, magnitude + 1u32),
        // Stepping across zero: `0 − 1` is the one sign flip (a
        // negative delta's magnitude is at least 1, so `−m + 1` never
        // crosses).
        (false, false) if magnitude == Base::ZERO => zigzag_signed(true, one),
        // Otherwise the magnitude shrinks toward zero; a magnitude of
        // exactly 1 lands on the positive zero.
        (true, true) | (false, false) => {
            let shrunk = magnitude - &one;
            zigzag_signed(negative && shrunk != Base::ZERO, shrunk)
        }
    };
    gamma_code(&stepped)
}

/// Emit the grown stream: replay `route` along the chosen path, splice
/// everything off it, repair the boundary deltas, and let the builder
/// collapse the inflation point if the increment made equal siblings.
fn emit(ev_bits: &BitsSlice, id_bits: &BitsSlice, route: &Route) -> Bits {
    let mut ev = EvScan::new(ev_bits);
    let mut id_pos = 0usize;
    // Subadditivity of the coding bounds the output by the input plus
    // the expansion chain's fresh codes, each a few bits per id level.
    let mut out = SkylineBuilder::with_capacity(ev_bits.len() + id_bits.len() + 64);
    // One bit per chosen-path level: `true` = the branch descended left,
    // so its right sibling subtree is pending after the inflation point.
    let mut pending = Bits::new();
    let mut depth = 0usize;
    let mut fed_any = false;
    let mut id_full = false;

    // Phase 1: descend the chosen path, splicing left off-path subtrees
    // as they pass, until the inflation point: an event leaf under a
    // full id (increment in place) or under an id node (expansion
    // chain, walked id-only).
    let (orig_code, chain_dirs) = loop {
        step!();
        if id_full {
            // FullEvNode regime: the id contributes nothing below.
            let key = ev.pos();
            match ev.read() {
                Some(code) => break (code, Bits::new()),
                None => {
                    if route.descends_left(Kind::FullEvNode, key) {
                        pending.push(true);
                    } else {
                        feed_subtree(&mut out, &mut ev, depth + 1, Repair::None);
                        fed_any = true;
                        pending.push(false);
                    }
                    depth += 1;
                }
            }
        } else {
            let key = id_pos;
            let (l, r) = id_tag(id_bits, id_pos);
            id_pos += 2;
            if !l && !r {
                id_full = true;
                continue;
            }
            match ev.read() {
                // Both: id node over event node — one more path level.
                None => {
                    if route.descends_left(Kind::Both, key) {
                        debug_assert!(l, "the chosen path never enters an absent id child");
                        pending.push(true);
                    } else {
                        debug_assert!(r, "the chosen path never enters an absent id child");
                        feed_subtree(&mut out, &mut ev, depth + 1, Repair::None);
                        if l {
                            id_pos = id_skip(id_bits, id_pos);
                        }
                        fed_any = true;
                        pending.push(false);
                    }
                    depth += 1;
                }
                // Expand: id node over an event leaf — the chain below
                // is id-only, its directions collected for the fresh
                // leaves' preorder.
                Some(code) => {
                    let mut dirs = Bits::new();
                    let mut cur = (key, l, r);
                    loop {
                        step!();
                        let (key, l, r) = cur;
                        let left = route.descends_left(Kind::Expand, key);
                        if left {
                            debug_assert!(l, "the chosen path never enters an absent id child");
                        } else {
                            debug_assert!(r, "the chosen path never enters an absent id child");
                            if l {
                                id_pos = id_skip(id_bits, id_pos);
                            }
                        }
                        dirs.push(left);
                        let next = (id_pos, id_tag(id_bits, id_pos));
                        id_pos += 2;
                        let (nkey, (nl, nr)) = next;
                        if !nl && !nr {
                            break;
                        }
                        cur = (nkey, nl, nr);
                    }
                    break (code, dirs);
                }
            }
        }
    };

    // Phase 2: the inflation point. The original leaf spanned depth
    // `d0`; a chain of `k` expansions roots a fresh subtree there whose
    // leaves are the original height `h` everywhere except the grown
    // leaf's `h + 1` at the bottom — so every fresh code is a `0`/`±1`
    // delta, except the chain's first leaf, which keeps the original
    // code (same height, same predecessor) or re-codes it `+1` when the
    // grown leaf itself comes first.
    let d0 = depth;
    let k = chain_dirs.len();
    let orig = &ev_bits[orig_code];
    debug_assert_eq!(d0, pending.len(), "one pending record per path level");
    let mut emitted_in_chain = false;
    // Fresh sibling leaves that precede the grown leaf: one per level
    // whose branch descended right (the sibling is the left child).
    for j in 0..k {
        if !chain_dirs[j] {
            let code = if emitted_in_chain {
                gamma_code(&Base::ZERO)
            } else {
                orig.to_bitvec()
            };
            out.leaf(d0 + j + 1, code);
            emitted_in_chain = true;
        }
    }
    let grown_code = if emitted_in_chain {
        gamma_code(&zigzag_signed(false, Base::from(1u8)))
    } else {
        recode(orig, true, !fed_any)
    };
    out.leaf(d0 + k, grown_code);
    // Fresh sibling leaves that follow the grown leaf, deepest first.
    let mut first_after_grown = true;
    for j in (0..k).rev() {
        if chain_dirs[j] {
            let code = if first_after_grown {
                gamma_code(&zigzag_signed(true, Base::from(1u8)))
            } else {
                gamma_code(&Base::ZERO)
            };
            out.leaf(d0 + j + 1, code);
            first_after_grown = false;
        }
    }

    // Phase 3: unwind. The pending right subtrees follow the inflation
    // point contiguously in the input, deepest first; only the first
    // one's first leaf can need the `−1` repair, and only when the
    // grown leaf (not a trailing fresh sibling) is the last leaf the
    // inflation emitted.
    let mut repair = if first_after_grown {
        Repair::MinusOne
    } else {
        Repair::None
    };
    for level in (0..d0).rev() {
        let went_left = pending.pop().expect("one pending record per path level");
        if went_left {
            feed_subtree(&mut out, &mut ev, level + 1, repair);
            repair = Repair::None;
        }
    }
    debug_assert_eq!(ev.pos(), ev_bits.len(), "the emit consumes the event");
    out.finish()
}

#[cfg(test)]
mod tests;
