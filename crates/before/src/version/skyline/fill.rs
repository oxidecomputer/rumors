//! The fused `tick` on skyline streams: one fill walk carrying the
//! changed flag and grow's route, then at most one splice.
//!
//! The paper's `event` is `fill`, kept iff it moved the tree, else the
//! cheapest inflation (`grow`). `fill(id, e)` collapses every event
//! subtree the id fully owns to a single leaf at that subtree's maximum
//! height, raising a collapsed child to its sibling's filled minimum
//! where that lets the parent simplify (the paper's shortcut arms).
//! [`tick`] runs the whole `event` in **one fill walk plus at most one
//! splice**: the walk decides `fill(id, e) ≠ e` in-pass — the *changed
//! flag*, tripping at the first emitted plateau that differs from the
//! input plateau it replaces — and folds grow's `(expansions, depth)`
//! route DP over the same `(id, event)` nodes it is already visiting
//! (both live in the `fuse` submodule; the flag's decision rules and
//! the route fold's arms are specified there against the equations).
//! While the flag is clear the output is byte-identical to the consumed
//! input prefix, so nothing is built: the first divergence materializes
//! that prefix wholesale and the walk continues as a direct emission,
//! while a walk that never diverges skips output work entirely and
//! hands its recorded route to [`grow`](super::grow)'s splice emit.
//! The changed branch therefore costs one walk; the unchanged (grow)
//! branch costs one walk plus one splice, with fill's discarded output
//! never built at all.
//!
//! The walk pairs the packed id (`IdReader`) against the skyline
//! topology and streams one `(depth, payload code)` plateau
//! per output leaf to the collapsing builder, which derives the union
//! topology and performs the equal-sibling normalization. A shortcut
//! raise needs no builder repair: the raised value is known before its
//! leaf is emitted. The right-full arm gets it by deferral — the raised
//! leaf is the *right* child's output, so the walk fills the left child
//! first, and the id cursor then sits exactly at the right child's tag:
//! one `O(1)` peek decides the arm, and the raise's minimum argument is
//! the walk's own watermark for the enclosing range. The left-full
//! arm's raised leaf precedes the range its minimum comes from, so it
//! alone pre-scans (`PreScan`) — memoized: one fresh scan records
//! every interior left-full site's minimum as a frame-ledger link
//! (the `Memo` doc), so no stream position is ever pre-scanned twice
//! and no minimum is materialized.
//!
//! # Heights stay relative
//!
//! No absolute height is materialized anywhere but the output stream's
//! first leaf (whose code is that absolute, so the read is priced by
//! the write). The walk carries the last consumed input height on one
//! cliff-immune [`Accumulator`], and every range minimum the shortcut arms
//! can ask for lives in one shared anchor web — the `watermark`
//! module's stack: `h − A` for an anchor at or above the innermost
//! open range's minimum (the excess parked in the stack's latent
//! register) plus nonnegative, zero-run-compressed differences
//! outward, so each consumed delta folds into O(1) accumulators and a
//! raise's comparison is an amortized-O(1) sign read. The output-delta
//! register (`h − prev_out` between pass-throughs, watermark-relative
//! after a raise took the tracked minimum) rides the same web, so
//! every emitted code is materialized once, post-collapse, at the
//! width the code itself prices. The pre-scan runs the same discipline
//! on its own web, and each memoized minimum travels as one ledger
//! link — a difference against a reference the walk already holds
//! when it arrives (the previous sibling site's minimum, the forest
//! parent's, or the scan-entry height), never an absolute — folded
//! into the live relation exactly once, at the raise decision it
//! serves.
//!
//! # Cost
//!
//! Scan: `O(n + m)` bits in the two packed streams [measured: e 1.00
//! on every committed board family at both scales]. The walk consumes
//! every position once; the left-full pre-scan reads a position at
//! most once more (the memo turns every interior left-full site into
//! a lookup, and distinct fresh scans cover disjoint sibling ranges);
//! the absent-sibling extremum scans read their range once ahead of
//! the walk's own copy (a flat ×2, never nesting); the pre-scan
//! replays each covered site's collapse range once at the site's
//! close (distinct sites' collapse ranges are disjoint — one more
//! flat pass, never nesting). The fused route
//! fold adds no reads of its own — its id reads are the tags the
//! walk's skips pay anyway — and each of the two branch epilogues is
//! one more bounded pass: a divergence replays the matched prefix
//! once, and the unchanged branch's splice emit reads both streams
//! once.
//!
//! Limb: accumulator digit touches are amortized linear in the two
//! packed streams [measured: exponent 1.00 with flat constants on
//! the matched spine, both wide × deep shortcut crosses, the memo
//! families — distinct and shared minima, interleaved combs, the
//! wide fan-out — the descending staircase, the close-reveal
//! genre (k sibling sites sharing one wide minimum over a low floor,
//! each site's node frame closing back into the floor frame between
//! consecutive consumes): the reveal comb reads ×2.00 across a joint
//! ×2.00 doubling and the bare-frame pure comb reads flat per byte
//! across a width doubling — and the undercut-cascade genre on both
//! its axes (the staircase's narrow full-penetration drops and the
//! ascending cliff's one wide residue through k − 1 nonzero unit
//! differences, ×2.00 across a joint doubling) — the
//! `width_circulation_cost` and memo
//! modules of `tests/meter.rs` pin the families that separate this
//! from every refuted discipline]. The cost invariant conserves
//! width: every touch is paid by a consumed input code, an emitted
//! output code, or the death of the digits it reads. Each consumed
//! delta folds into O(1) accumulators; each emission's watermark
//! update is one amortized sign read plus a propagation whose every
//! fold is a dying operand or the one surviving fold the update's own
//! priced width bounds; a close moves its popped boundary into the
//! watermark stack's latent register and an arm recycles the register
//! into the new boundary, so the close-reveal cycle's wide content
//! shuttles by moves at a narrow anchor-relative marginal cost (the
//! `watermark` module doc carries the register's discipline); each
//! emitted code is materialized once, post-collapse, at its own
//! width; the watermark compares fold and restore only the priced
//! offset (or answer post-sign by top-index domination, through the
//! latent ladder where one is parked); the extremum scans'
//! reset-on-cross folds are priced by the range they scan; the
//! absent-sibling raise compares materialized offsets both priced by
//! their own scans; the builder's equal-sibling seam is a one-bit
//! code check. The ledger's own operands obey the same lifetime
//! rules: each link is created once at its site's close (the head
//! moves into the queue, or a deferred first-child link is cloned
//! once at its own width), read once at its consume, and dies into
//! the raise decision; the keeper and suspend folds each read a
//! dying operand or a link's own priced width; the one live follower
//! (the head, walk-side the relation) receives the per-event fold
//! the watermark discipline already prices — closes excepted, where
//! it rides the latent tag untouched — and the pre-scan's recorder
//! adds one amortized sign read per site (the zero-link test). Wide
//! content is read only where an operand dies, a bounded-count
//! lifetime read, or a code prices it — the height↔watermark anchor
//! switches read the surviving web once, priced by the switch
//! emission's own code, with a parked latent cancelling symbolically
//! on the watermark-to-height switch and retiring on the other.
//!
//! Heap: O(paired depth) transient frame *bits* plus O(n + m) total
//! live digits; the memo holds one queue entry per covered site — an
//! accumulator only where the link is nonzero, so sites sharing one
//! minimum store nothing — plus one suspended entry per open
//! site-nesting level.
//!
//! Both walks are iterative: suspended ancestors live on explicit
//! stacks — control bits plus pop-able word deltas (`Frames` and
//! `PreFrames`, the route fold's own `PopStack` discipline) — so
//! paired depth costs a few heap bits per level, never a call-stack
//! frame, and no input depth can grow stacker segments or overflow.
//! The wide quantity a left-full site's raise decision needs after its
//! sibling walk is re-derived by one bounded replay of the site's own
//! collapse range (`PreScan::replay_max`) rather than parked per
//! open site, so frames stay word-free and the transient stays flat
//! on nested-site chains.
//!
//! # Testing
//!
//! Two committed differentials pin the fused walk directly to the
//! recursive oracle, and they are the entire pin of the flag seam:
//! `tick` byte-identical to the oracle's `event`, and the changed flag
//! ≡ (the oracle's `fill` moved the tree) — over the adversarial
//! families crossed with adversarial parties, arbitrary pairs, organic
//! histories, and the exhaustive small scope, with canonical
//! uniqueness making both differentials total. The grow suite
//! (`grow/tests.rs`) additionally holds the unchanged branch to the
//! oracle's inflation, the brute-force minimal-inflation search, and a
//! reference recursive route probe. The oracle walks on native frames
//! with materialized magnitudes, so these differentials run at
//! oracle-sized operands; the large-operand coverage lives in the deep
//! closed-form witnesses here, the meter suite's closed-form output
//! asserts at its pinned scales, and the board's determinism tripwire.

use core::cmp::Ordering;

use suanpan::Accumulator;

use crate::codec::{self, Base, BitCursor, BitStack, BitsMut, BitsSlice, PopStack};
use crate::idbits::{IdNode, IdReader};

use self::fuse::{decode_cost_component, encode_cost_component, Out, RouteProbe, COST_FREE};
use self::watermark::{MinStack, Signed};
use super::grow::{Cost, COST_MAX};
use super::walk::{Extremum, LeafWalk};
use super::{fold_signed, gamma_code, gamma_code_signed, unzigzag};

mod fuse;
mod watermark;

/// The follower slot carrying `min − prev_out` while the output delta
/// is watermark-anchored (a raise just emitted the tracked minimum).
const OUT_FOLLOWER: usize = 0;

/// The follower slot carrying the live ledger relation: walk-side
/// `min − m_r` while the reference is watermark-carried; pre-scan-side
/// the recording head, `min − m_ref` for the level it serves.
const REL_FOLLOWER: usize = 1;

/// Register one event on the version a skyline stream denotes, from
/// the perspective of a packed id.
///
/// The event is `fill` if it
/// simplifies the tree, else the [`grow`](super::grow) inflation —
/// one fused walk, then at most one splice (the module doc carries
/// the fusion).
///
/// The differential suite pins the whole `event` against the recursive
/// oracle and the changed flag against the oracle's `fill`, with the
/// unchanged branch additionally held to the brute-force minimal
/// inflation and a reference route probe (`grow/tests.rs`).
///
/// # Panics
///
/// Panics if the event operand is not a canonical skyline stream — run
/// [`validate`](fn@super::validate) first on untrusted bytes. The id
/// must own at least one region: an empty id leaves `fill` the identity,
/// and the grow fallback requires an owning id (debug builds assert it;
/// the result on an empty id is unspecified in release builds).
pub fn tick(ev: &BitsSlice, id: &crate::Party) -> BitsMut {
    // `n = 1` performs exactly one fused walk plus at most one splice:
    // the delta against a direct dispatch is two unmetered width tests
    // and one non-allocating `Base` construction. The committed
    // `tick_is_ticks_one` differential and the `ticks_one_is_tick` law
    // pin the outputs equal.
    ticks(ev, id, &Base::from(1u8))
}

/// Register `n` events on the version a skyline stream denotes, from
/// the perspective of a packed id — byte-identical to `n` sequential
/// [`tick`]s, in at most two fused walks plus one `+n` splice.
///
/// The branch structure compounds the paper's `event = fill if it
/// changed, else grow`:
///
/// - `fill(i, e) = e` (the steady state): the one walk records the
///   route, and one `+n` splice (the [`grow`](super::grow) module's
///   emit) registers all `n` events — a grow never re-opens the fill branch,
///   so ticks 2..n are all grows at the same site (the grow module doc
///   carries the compounding argument).
/// - `fill(i, e) ≠ e`: the first tick is the fill output; the remaining
///   `n − 1` events are grows on that output, whose route needs a
///   second walk — the first walk's route probe dies at the divergence,
///   and the route over the changed tree is a different fold. Fill is
///   idempotent (the committed `fill_is_idempotent` differential pins
///   it), so the second walk always reports the tree unchanged.
///
/// `n = 0` is the identity (the empty run); `n = 1` performs exactly
/// [`tick`]'s work. The `ticks` differentials in this module's test
/// suite hold every branch to the iterated public tick byte for byte.
///
/// # Panics
///
/// Panics if the event operand is not a canonical skyline stream — run
/// [`validate`](fn@super::validate) first on untrusted bytes. For
/// `n >= 1` the id must own at least one region, exactly as [`tick`]
/// (debug builds assert it; the result on an empty id is unspecified in
/// release builds).
pub fn ticks(ev: &BitsSlice, id: &crate::Party, n: &Base) -> BitsMut {
    // Width tests, not value compares: n = 0 has no bits, n = 1 is the
    // one-bit magnitude, and neither test touches the limb meter.
    if n.bits() == 0 {
        return ev.to_bitvec();
    }
    match fused_fill(ev, id) {
        FillOutcome::Changed(bits) => {
            if n.bits() == 1 {
                // n = 1: the fill output is the whole event.
                return bits;
            }
            let rest = n.clone() - &Base::from(1u8);
            match fused_fill(&bits, id) {
                FillOutcome::Unchanged(route) => {
                    super::grow::emit(&bits, id.as_bits(), &route, &rest)
                }
                FillOutcome::Changed(_) => {
                    unreachable!("fill is idempotent: a filled tree cannot fill again")
                }
            }
        }
        FillOutcome::Unchanged(route) => super::grow::emit(ev, id.as_bits(), &route, n),
    }
}

/// The fused walk's verdict: `fill` moved the tree (its output stands
/// as the tick), or it was the identity and the recorded route drives
/// the grow splice.
pub(super) enum FillOutcome {
    /// `fill(id, e) ≠ e`: the canonical filled stream.
    Changed(BitsMut),
    /// `fill(id, e) = e`: the inflation route for
    /// [`grow::emit`](super::grow::emit).
    Unchanged(super::grow::Route),
}

/// Run the fill walk over one `(event, id)` pair with the fused state
/// live.
///
/// The changed flag decides the branch, and the route rides
/// along for the unchanged one (the module and `fuse` docs carry
/// both).
///
/// # Panics
///
/// Panics if the event operand is not a canonical skyline stream.
pub(super) fn fused_fill(ev_bits: &BitsSlice, id: &crate::Party) -> FillOutcome {
    let id_bits = id.as_bits();
    let mut walk = FillWalk {
        ev: ev_bits,
        cursor: codec::DsiCursor::new(ev_bits),
        first_read: true,
        h: Accumulator::new(),
        gap: Accumulator::new(),
        w_anchored: false,
        started: false,
        range_is_leaf: false,
        stack: MinStack::new(),
        memo: Memo::new(),
        corr: Corr::None,
        out: Out::verbatim(),
        probe: RouteProbe::new(id_bits.len()),
    };
    let mut id = IdReader::root(id_bits);
    walk.stack.open();
    walk.walk(&mut id);
    if walk.w_anchored {
        let follower = walk.stack.follower_take(OUT_FOLLOWER);
        walk.stack.retire(follower);
    }
    walk.stack.close();
    debug_assert_eq!(walk.pos(), ev_bits.len(), "fill consumes its whole input");
    debug_assert!(
        walk.memo.cursor == walk.memo.queue.len(),
        "the walk consumes every memoized minimum"
    );
    #[cfg(debug_assertions)]
    debug_assert_eq!(
        walk.memo.recorded_check, walk.memo.consumed_check,
        "the walk consumed the recorded sites, in order"
    );
    debug_assert!(
        matches!(walk.corr, Corr::None),
        "the ledger relation dies with the outermost site"
    );
    // Canonicalizing the storage is `Version::from_bits`'s job, the
    // single gate a stream passes through when it becomes a stored value.
    match walk.out.finish(ev_bits) {
        Some(bits) => FillOutcome::Changed(bits),
        None => FillOutcome::Unchanged(walk.probe.take_route()),
    }
}

/// The fill walk: input cursor, relative-height state, the fused
/// changed-flag output, and the route probe. The `&mut` [`IdReader`]
/// threads alongside as the recursion argument, exactly as the packed
/// walks thread theirs.
struct FillWalk<'a> {
    /// The input skyline stream (kept beside the cursor for the
    /// unmetered single-flag peek and the sub-scans' spawn positions).
    ev: &'a BitsSlice,
    /// The input cursor.
    cursor: codec::DsiCursor<'a>,
    /// Whether the next payload is the stream's first (coded absolute,
    /// not as a delta).
    first_read: bool,
    /// The last consumed input leaf's height.
    h: Accumulator,
    /// `h − prev_out` while the output delta is height-anchored: every
    /// consumed step folds in, and every emitted leaf re-derives it.
    /// Idle (zero) while `w_anchored`.
    gap: Accumulator,
    /// Whether the output delta is watermark-anchored: the last
    /// emission took the tracked minimum, and `min − prev_out` rides
    /// the stack's [`OUT_FOLLOWER`] instead of `gap`.
    w_anchored: bool,
    /// Whether any output leaf has been emitted (the first is coded
    /// absolute).
    started: bool,
    /// Whether the range the last consuming max scan covered was a
    /// single leaf.
    ///
    /// The changed flag's topology test at the emission that replaces
    /// the range: a multi-leaf range collapsing to one leaf is a
    /// divergence before any code comparison.
    range_is_leaf: bool,
    /// The walk's range-minimum watermarks (the anchor web).
    stack: MinStack,
    /// Left-full minima computed ahead of the walk (the frame ledger).
    ///
    /// A fresh pre-scan records every interior left-full site it
    /// evaluates, and the walk consumes each entry exactly once on
    /// arrival — so no position is pre-scanned twice.
    memo: Memo,
    /// The relation of the walk's ledger reference to its live state.
    ///
    /// Each consume re-anchors it to the consumed site's minimum, and
    /// each site's range close re-anchors it from the walk's own web
    /// (which holds that site's minimum natively at that instant), so
    /// it is exactly the queue-front link's reference at every
    /// consume.
    corr: Corr,
    /// The changed flag's realization: the verbatim reference until the
    /// first divergent plateau, the collapsing builder after (the
    /// `fuse` doc).
    out: Out,
    /// Grow's route DP, folded in post-order while the flag is clear
    /// (the `fuse` doc).
    probe: RouteProbe,
}

/// The memoized pre-scan's output — the frame ledger: per left-full
/// site, in the walk's arrival (pre-order) order, one optional link
/// resolving the site's minimum against a reference the walk already
/// holds when it arrives.
///
/// The reference discipline: a site with an earlier sibling under the
/// same forest parent stores `m_s − m_prev`; a forest parent's first
/// child stores `m_s − m_parent`, written at the parent's own close —
/// when the parent's minimum is final — into the child's earlier
/// queue slot (the queue is written out of order, consumed in order);
/// the scan's outermost site stores `m_root − h(scan entry)`. The
/// walk's arrival-order relation is exactly that reference at every
/// consume: the site before a first child is its parent (the live
/// relation), and the site before a later sibling is the previous
/// sibling (re-anchored from the walk's own web at that sibling's
/// range close). Zero links are not stored at all, so sibling or
/// nested sites sharing one minimum cost nothing — one wide shared
/// minimum is never materialized per covering site. Each stored link
/// is created once, read once at its consume, and dies into the raise
/// decision.
struct Memo {
    /// Per site, in consumption (stream) order: 0 when the site's
    /// link is zero, else the 1-based index of its link in `links` —
    /// one machine word per site, so sites sharing minima store
    /// nothing beyond it.
    queue: Vec<u32>,
    /// The nonzero links, in write order (sibling links land at their
    /// sites' closes, deferred first-child links at their parents') —
    /// the queue's indices decouple write order from consumption
    /// order.
    links: Vec<Accumulator>,
    /// The consumption cursor into `queue`.
    cursor: usize,
    /// The end position of the current fresh scan's span: sites before
    /// it are recorded; a site at or past it launches a new scan.
    covered_until: usize,
    /// Order-sensitive checksum of the recorded sites' positions,
    /// matched against the consumed ones when the scan drains — O(1)
    /// state where a position list would bill the heap meter for a
    /// debug-only buffer.
    #[cfg(debug_assertions)]
    recorded_check: u64,
    /// The consumed positions' checksum (see `recorded_check`).
    #[cfg(debug_assertions)]
    consumed_check: u64,
}

/// Fold one position into an order-sensitive checksum (FNV-style).
#[cfg(debug_assertions)]
fn position_check(check: u64, pos: usize) -> u64 {
    (check ^ pos as u64).wrapping_mul(0x0100_0000_01b3)
}

impl Memo {
    fn new() -> Self {
        Memo {
            queue: Vec::new(),
            links: Vec::new(),
            cursor: 0,
            covered_until: 0,
            #[cfg(debug_assertions)]
            recorded_check: 0,
            #[cfg(debug_assertions)]
            consumed_check: 0,
        }
    }

    /// Reset for a new fresh scan, keeping the queue allocation.
    fn begin_scan(&mut self) {
        debug_assert_eq!(self.cursor, self.queue.len(), "the prior scan drained");
        #[cfg(debug_assertions)]
        debug_assert_eq!(
            self.recorded_check, self.consumed_check,
            "the walk consumed the recorded sites, in order"
        );
        self.queue.clear();
        self.links.clear();
        self.cursor = 0;
    }

    /// Store a nonzero link for `slot`, in write order.
    fn set_link(&mut self, slot: usize, link: Accumulator) {
        self.links.push(link);
        self.queue[slot] = u32::try_from(self.links.len()).expect("site count fits u32");
    }

    /// Take `slot`'s link out for its one consuming read, if nonzero.
    fn take_link(&mut self, slot: usize) -> Option<Accumulator> {
        match self.queue[slot] {
            0 => None,
            idx => Some(core::mem::take(&mut self.links[idx as usize - 1])),
        }
    }
}

/// The relation of the walk's ledger reference (`m_r`: the last
/// consumed site's minimum, re-anchored to each closing site on the
/// way out) to the walk's live state.
enum Corr {
    /// No site is open or resolved (only a fresh scan's outermost site
    /// consumes in this state; its reference is the scan-entry height,
    /// which is the walk's height at that instant).
    None,
    /// `h − m_r`, folding input deltas: the reference is
    /// height-carried (the last raise took the scan-maximum side).
    H(Accumulator),
    /// `min − m_r` rides the stack's [`REL_FOLLOWER`]: the reference
    /// is watermark-carried (the last raise took the minimum side, or
    /// a site's close re-anchored from the walk's own web).
    Min,
}

impl FillWalk<'_> {
    /// Fill the whole event stream under the id at the cursor, emitting
    /// its plateaus and advancing both cursors past their trees.
    ///
    /// Iterative: the loop alternates a *descend* phase (process the
    /// subtree at the cursor until it either resolves to a cost or
    /// suspends this branch node on [`Frames`] and enters a child) with
    /// an *ascend* phase (fold the completed child's inflation cost
    /// into the suspended node, resuming its remaining work — the
    /// right-full peek, the right child's walk, the site close). Each
    /// child runs inside its own watermark frame
    /// ([`MinStack::open`]/[`close`](MinStack::close)), absent children
    /// as the inlined `fill(0, e) = e` copy at infeasible cost, so the
    /// arms, emissions, and route folds run in exactly the paired
    /// preorder the fill equations prescribe. The root subtree's cost
    /// is dropped: only interior folds read costs.
    fn walk(&mut self, id: &mut IdReader) {
        let mut frames = Frames::new();
        let mut depth = 0usize;
        'descend: loop {
            debug_assert_eq!(depth, frames.len(), "one frame per open branch level");
            // Descend: resolve the subtree at the cursor to a cost, or
            // suspend its branch node and re-enter on a present child.
            let mut cost: Cost = loop {
                let (left, right) = match id.read() {
                    // fill(0, e) = e: the id owns nothing here, and grow
                    // can inflate nothing (`grow` skips absent regions).
                    // A real cursor reads this only at an empty root.
                    IdNode::Empty => {
                        self.copy_subtree(depth);
                        break COST_MAX;
                    }
                    // fill(1, e) = max(e): a fully-owned region collapses.
                    // On a route-live walk the region is a single leaf — a
                    // node would collapse to fewer plateaus and trip the
                    // flag at the emission below — so the cost is the free
                    // increment `grow(1, n) = (n + 1, 0)`.
                    IdNode::Full => {
                        let above = self.scan_max_consuming();
                        self.emit_offset(depth, above);
                        break COST_FREE;
                    }
                    IdNode::Internal { left, right } => (left, right),
                };
                // The branch's route key: the 2-bit tag `read` just
                // consumed (an `Internal` reader is always a real cursor).
                let key = id.pos() - 2;
                if self.read_flag() {
                    // fill((il, ir), Leaf n) = Leaf n: an event leaf is
                    // already simple; the dominated id children are
                    // lazy-skipped, and the route's expansion fold rides
                    // the skips.
                    let (neg, mag) = self.consume_payload();
                    self.emit_step(depth, neg, mag);
                    break self.probe.expand(key, id, left, right);
                }

                // An id node over an event node: the shortcut arms
                // collapse a fully-owned child, raised to its sibling's
                // filled minimum. Either way the branch's cost folds both
                // children — a full child is the free increment, an
                // absent one is infeasible — and the chosen direction
                // records at the branch's id key.
                if left && matches!(id.peek(), IdNode::Full) {
                    // `il` full: the left child collapses to
                    // `max(max(el), min(fill(ir, er)))`. The max comes
                    // from the consuming scan of `el`; the min — needed
                    // before the raised leaf is emitted, ahead of `er`'s
                    // walk — from the frame ledger when an enclosing
                    // pre-scan already evaluated this site, else from one
                    // fresh (and recording) pre-scan of the right
                    // sibling, anchored at `h` (which sits at `el`'s last
                    // leaf, exactly the pre-scan's entry).
                    id.skip();
                    let above = self.scan_max_consuming();
                    if !right {
                        // An absent right child is fill(0, er): its
                        // minimum is min(er), priced by the scan that
                        // reads the range; the copy runs in its own
                        // frame, exactly as a child walk would.
                        let raise = scan_min_from(self.ev, self.pos(), self.first_read);
                        let value_off = signed_max(&above, &raise);
                        self.emit_offset(depth + 1, value_off);
                        self.stack.open();
                        self.copy_subtree(depth + 1);
                        self.stack.close();
                        break self.probe.join(key, COST_FREE, COST_MAX);
                    }
                    let outermost = self.pos() >= self.memo.covered_until;
                    debug_assert_eq!(
                        outermost,
                        matches!(self.corr, Corr::None),
                        "a fresh scan starts exactly where no ledger relation is live"
                    );
                    if outermost {
                        // Uncovered: one fresh pre-scan records this site
                        // and every left-full site inside its span.
                        self.memo.begin_scan();
                        let scan_start = self.pos();
                        let mut scan = PreScan {
                            ev: self.ev,
                            cursor: codec::DsiCursor::new_at(self.ev, scan_start),
                            stack: MinStack::new(),
                            entry_net: Some(Accumulator::new()),
                            pending_rel: None,
                            memo: &mut self.memo,
                            keeper: Accumulator::new(),
                            first_slot: usize::MAX,
                            head_level: 0,
                            suspend: Vec::new(),
                        };
                        let slot = scan.reserve(scan_start);
                        scan.stack.open();
                        let mut reader = IdReader::at(id.bits(), id.pos());
                        let end = scan.run(self.first_read, &mut reader);
                        scan.record(slot, 0);
                        let rel = scan.stack.follower_take(REL_FOLLOWER);
                        scan.stack.retire(rel);
                        scan.stack.close();
                        debug_assert!(scan.suspend.is_empty(), "every suspended level resolves");
                        self.memo.covered_until = end;
                    }
                    self.consume_site(&above, depth);
                    frames.push_site(key, outermost);
                    self.stack.open();
                    depth += 1;
                    continue; // walk the right sibling range
                }
                // An ordinary node: the left child first; the id cursor
                // then sits exactly at the right child's tag, so the
                // right-full arm is one `O(1)` peek on the way back up —
                // no lookahead over the left id subtree.
                frames.push_node(key, right);
                self.stack.open();
                depth += 1;
                if left {
                    continue; // descend into the left child
                }
                // Absent left child: fill(0, el), inlined in the frame
                // just opened; its infeasible cost rises into the node
                // exactly as a child walk's would.
                self.copy_subtree(depth);
                break COST_MAX;
            };
            // Ascend: fold the completed subtree's cost upward until a
            // suspended node still has a child to walk (or the root
            // completes).
            loop {
                let Some(top) = frames.top() else {
                    debug_assert_eq!(depth, 0, "the root subtree completes at depth zero");
                    let _ = cost; // the root's inflation cost has no parent fold
                    return;
                };
                self.stack.close();
                depth -= 1;
                match top {
                    // A consume-site's sibling walk finished: close the
                    // site's range against the ledger relation, then fold
                    // `grow((1, ir), ·)` — the collapsed left child is the
                    // free increment.
                    Frame::Site => {
                        let (key, outermost) = frames.pop_site();
                        self.pop_site(outermost);
                        cost = self.probe.join(key, COST_FREE, cost);
                    }
                    // A node's left child finished: peek the right-full
                    // arm, walk the right child, or fold an absent one.
                    Frame::AwaitLeft => {
                        let right = frames.aux_top();
                        if right && matches!(id.peek(), IdNode::Full) {
                            // `ir` full: the right child collapses to
                            // `max(max(er), min(fill(il, el)))`. The
                            // minimum is the enclosing frame's own
                            // watermark — its only emissions so far are
                            // the left child's — so the decision is one
                            // sign read against the priced scan maximum.
                            id.skip();
                            let above = self.scan_max_consuming();
                            if self.stack.compare_above(&above) == Ordering::Less {
                                self.emit_at_min(depth + 1);
                            } else {
                                self.emit_offset(depth + 1, above);
                            }
                            let key = frames.pop_await_left();
                            cost = self.probe.join(key, cost, COST_FREE);
                        } else if right {
                            frames.flip_to_await_right(cost);
                            self.stack.open();
                            depth += 1;
                            continue 'descend; // walk the right child
                        } else {
                            // Absent right child: fill(0, er) in its own
                            // frame, infeasible for the route.
                            self.stack.open();
                            self.copy_subtree(depth + 1);
                            self.stack.close();
                            let key = frames.pop_await_left();
                            cost = self.probe.join(key, cost, COST_MAX);
                        }
                    }
                    // A node's right child finished: fold both children.
                    Frame::AwaitRight => {
                        let (key, l_cost) = frames.pop_await_right();
                        cost = self.probe.join(key, l_cost, cost);
                    }
                }
            }
        }
    }

    /// The cursor's bit position: the next node's flag.
    fn pos(&self) -> usize {
        self.cursor.position()
    }

    /// Read one topology flag at the cursor (`true` = leaf), recording
    /// the scanned bit. Single-flag rather than a unary run: the walk
    /// interleaves with the id stream one node at a time here, so there
    /// is no run to batch.
    fn read_flag(&mut self) -> bool {
        self.cursor.read_bit().expect("canonical skyline bits")
    }

    /// Decode the payload at the cursor as a signed step (the stream's
    /// first payload is its absolute height, a step from zero), folding
    /// it into the height-anchored accumulators, and advancing the
    /// cursor.
    fn consume_payload(&mut self) -> Signed {
        let code = self.cursor.read_int().expect("canonical skyline bits");
        let (neg, mag) = if self.first_read {
            self.first_read = false;
            (false, code)
        } else {
            unzigzag(code)
        };
        fold_signed(&mut self.h, neg, &mag);
        self.stack.fold_height(neg, &mag);
        if !self.w_anchored {
            fold_signed(&mut self.gap, neg, &mag);
        }
        if let Corr::H(acc) = &mut self.corr {
            fold_signed(acc, neg, &mag);
        }
        (neg, mag)
    }

    /// Consume the queue-front memoized site: resolve its minimum by
    /// one fold of its ledger link into the live relation, decide the
    /// raise, and emit.
    fn consume_site(&mut self, above: &Signed, depth: usize) {
        debug_assert!(
            self.memo.cursor < self.memo.queue.len(),
            "a covered site has a recorded entry"
        );
        #[cfg(debug_assertions)]
        {
            self.memo.consumed_check = position_check(self.memo.consumed_check, self.pos());
        }
        let link = self.memo.take_link(self.memo.cursor);
        self.memo.cursor += 1;
        match core::mem::replace(&mut self.corr, Corr::None) {
            Corr::None => {
                // Base: the outermost site's reference is the fresh
                // scan's entry height, which is the walk's height
                // right here — the relation starts at zero.
                let acc = self.stack.lease();
                self.consume_h_anchored(acc, link, above, depth);
            }
            Corr::H(acc) => self.consume_h_anchored(acc, link, above, depth),
            Corr::Min => {
                // d_arm = m_s − A = (m_s − m_r) − (f_stored = A − m_r):
                // the link dies into the decision, and taking the
                // relation raw keeps everything anchor-relative — the
                // latent a preceding close parked cancels out of the
                // comparison and the arming alike, so the cycle's cost
                // is the narrow inter-site movement, never the parked
                // width. (Without the tag, f = m − m_r would gross the
                // full anchor-to-floor gap into d_arm at every
                // consume.)
                let mut d = self.stack.follower_take(REL_FOLLOWER);
                d.negate();
                if let Some(link) = link {
                    d.add_accum(&link);
                    self.stack.retire(link);
                }
                if self.stack.compare_above_vs(above, &d) == Ordering::Less {
                    // The minimum side: arm at m_s and emit there.
                    self.stack.arm_relative(d);
                    self.emit_at_min(depth + 1);
                    let zero = self.stack.lease();
                    self.stack.follower_set(REL_FOLLOWER, zero);
                } else {
                    // The relation re-anchors to m_s: the negated
                    // decision quantity is `A − m_s`, exactly the
                    // anchor-relative content `follower_set` tags when
                    // a latent lives (and `min − m_s` when none does).
                    // The follower installs BEFORE the emission: the
                    // raise can arm a pending frame (moving the tracked
                    // minimum), and only an installed follower receives
                    // that arm's fold — installed after, the relation
                    // goes stale by exactly the arm's delta.
                    d.negate();
                    self.stack.follower_set(REL_FOLLOWER, d);
                    self.emit_offset(depth + 1, above.clone());
                }
                self.corr = Corr::Min;
            }
        }
    }

    /// [`consume_site`](Self::consume_site) with a height-carried
    /// relation `acc = h − m_r`.
    fn consume_h_anchored(
        &mut self,
        mut acc: Accumulator,
        link: Option<Accumulator>,
        above: &Signed,
        depth: usize,
    ) {
        // The decision is sign((h + above) − m_s) = sign(acc + above −
        // link); the link stays folded in, so acc then holds h − m_s
        // and the relation re-anchors to this site for free. The link
        // dies here — its one read.
        fold_signed(&mut acc, above.0, &above.1);
        if let Some(link) = link {
            acc.sub_accum(&link);
            self.stack.retire(link);
        }
        let sign = acc.sign();
        fold_signed(&mut acc, !above.0, &above.1);
        if sign == Ordering::Less {
            // The minimum side: the raise lifts the emitted value
            // strictly above the consumed range's maximum, so a
            // verbatim walk diverges here (the first-leaf write below
            // needs the builder live).
            self.diverge();
            // acc is exactly `h − m_s`, the below the arming moves
            // into the web.
            if !self.started {
                // First output leaf, coded absolute: v = h − below.
                let mut v = self.stack.lease();
                v.add_accum(&self.h);
                v.sub_accum(&acc);
                self.stack.emit_below_accum(acc);
                let value = self.stack.materialize(v);
                debug_assert!(!value.0, "a raised height is a natural");
                self.started = true;
                self.out.leaf(depth + 1, gamma_code(&value.1));
                // prev_out = min: the output delta anchors to the
                // watermark from the start.
                let zero = self.stack.lease();
                self.stack.follower_set(OUT_FOLLOWER, zero);
                self.w_anchored = true;
                self.gap.reset();
            } else {
                self.stack.emit_below_accum(acc);
                self.emit_at_min(depth + 1);
            }
            let zero = self.stack.lease();
            self.stack.follower_set(REL_FOLLOWER, zero);
            self.corr = Corr::Min;
        } else {
            self.emit_offset(depth + 1, above.clone());
            self.corr = Corr::H(acc);
        }
    }

    /// Close a consumed site's range: the old relation retires, and —
    /// for an interior site — the reference re-anchors to this site's
    /// minimum from the walk's own web, at zero cost.
    ///
    /// The web holds `m_s` natively at this instant: the site's node
    /// frame has absorbed exactly the range's emissions (whose minimum
    /// is `m_s`) and the raised leaf, and a raise never falls below
    /// its own site's minimum (the fill equations' `max`), so the
    /// tracked minimum IS `m_s`. The next consume at the enclosing
    /// level is this site's next sibling, whose ledger link is
    /// relative to exactly this minimum.
    fn pop_site(&mut self, outermost: bool) {
        match core::mem::replace(&mut self.corr, Corr::None) {
            Corr::None => unreachable!("a consumed site keeps a relation"),
            Corr::H(acc) => self.stack.retire(acc),
            Corr::Min => {
                let rel = self.stack.follower_take(REL_FOLLOWER);
                self.stack.retire(rel);
            }
        }
        if !outermost {
            // The fresh relation starts at zero against the tracked
            // minimum, so the anchor must be exact: a latent parked by
            // a nested site's close retires here (its one death,
            // funded by the mint the input's re-widening climb paid
            // for); the consume cycle's arm has already drained it.
            self.stack.resolve_latent();
            let zero = self.stack.lease();
            self.stack.follower_set(REL_FOLLOWER, zero);
            self.corr = Corr::Min;
        }
    }

    /// Trip the changed flag: the emission in flight differs from the
    /// input plateau it replaces.
    ///
    /// The matched prefix materializes into
    /// the real builder and the route probe dies (the flag routed this
    /// pair to the fill branch, where no route is read); a no-op once
    /// diverged.
    fn diverge(&mut self) {
        if self.out.is_verbatim() {
            self.probe.kill();
            self.out.materialize(self.ev);
        }
    }

    /// Emit a pass-through leaf at the current input height: the output
    /// delta is the live gap (the step is already folded in), which
    /// equals the input step itself whenever the streams agree.
    ///
    /// A pass-through leaf can never trip the changed flag: its depth
    /// is the consumed input leaf's own, and its delta re-codes the
    /// step just consumed against an unchanged predecessor —
    /// byte-identical by canonical uniqueness — so a verbatim walk
    /// records the match and skips the emission body outright.
    fn emit_step(&mut self, depth: usize, neg: bool, mag: Base) {
        self.stack.emit_here();
        if self.out.note_match(self.pos()) {
            self.started = true;
            self.gap.reset();
            return;
        }
        if !self.started {
            // The output's first leaf is coded absolute. A pass-through
            // first leaf is the input's first (every consumed-but-not-
            // emitted range ends in a collapse emit), whose absolute is
            // the step itself.
            debug_assert!(!neg, "the stream's first height is a natural");
            debug_assert!(!self.w_anchored, "the first emission finds no anchor");
            self.started = true;
            self.gap.reset();
            self.out.leaf(depth, gamma_code(&mag));
            return;
        }
        let _ = (neg, mag);
        let delta = if self.w_anchored {
            // d_out = h − prev_out = (min − prev_out) + (h − min): the
            // anchor switch's one bridge read of the surviving web,
            // priced by this emission's own code.
            let mut d = self.stack.follower_take(OUT_FOLLOWER);
            self.stack.bridge_add_t(&mut d);
            self.w_anchored = false;
            self.stack.materialize(d)
        } else {
            // d_out = value − prev_out = gap. One collapse-then-read —
            // the common case (nothing consumed since the last emit)
            // reads the single step just folded.
            self.gap.sign();
            let (sign, magnitude) = self.gap.sign_magnitude();
            (sign == Ordering::Less, Base::from(magnitude))
        };
        // The new gap is h − value = 0 exactly.
        self.gap.reset();
        self.out.leaf(depth, gamma_code_signed(delta.0, &delta.1));
    }

    /// Emit a leaf whose value is `h + off`: a collapsed region's max,
    /// or a shortcut arm's raised value decided against the watermark.
    ///
    /// The changed flag's decision site: the emission replaces the
    /// range the caller's consuming scan just covered, so it reproduces
    /// the input plateau iff that range was a single leaf (else the
    /// collapse moved topology — the range's first plateau sits
    /// strictly deeper than this one) and the offset's value is zero
    /// (the emitted value is exactly the consumed leaf's height, hence
    /// the same delta — or, at the stream's head, the same absolute:
    /// output position ≡ input position while the walk is verbatim, so
    /// a first leaf compares absolute against absolute). A
    /// value-reproducing raise is a match, never a divergence.
    fn emit_offset(&mut self, depth: usize, off: Signed) {
        self.stack.emit_offset(&off);
        if self.out.is_verbatim() {
            if self.range_is_leaf && off.1 == Base::ZERO {
                if self.out.note_match(self.pos()) {
                    self.started = true;
                    self.gap.reset();
                    return;
                }
            } else {
                self.diverge();
            }
        }
        if !self.started {
            // First output leaf: materialize the absolute — its width
            // is the emitted code's own (the height so far is the
            // input's own first code plus consumed deltas), so the
            // read is priced by the write.
            debug_assert!(!self.w_anchored, "the first emission finds no anchor");
            self.h.sign();
            let (sign, magnitude) = self.h.sign_magnitude();
            debug_assert_ne!(sign, Ordering::Less, "heights are nonnegative");
            let value = signed_sum_base((false, Base::from(magnitude)), &off);
            debug_assert!(!value.0, "a collapsed height is a natural");
            self.started = true;
            self.out.leaf(depth, gamma_code(&value.1));
        } else {
            let delta = if self.w_anchored {
                // d_out = (h + off) − prev_out: the bridge read plus
                // the priced offset.
                let mut d = self.stack.follower_take(OUT_FOLLOWER);
                self.stack.bridge_add_t(&mut d);
                fold_signed(&mut d, off.0, &off.1);
                self.w_anchored = false;
                self.stack.materialize(d)
            } else {
                // d_out = (h + off) − prev_out = gap + off.
                fold_signed(&mut self.gap, off.0, &off.1);
                self.gap.sign();
                let (sign, magnitude) = self.gap.sign_magnitude();
                (sign == Ordering::Less, Base::from(magnitude))
            };
            self.out.leaf(depth, gamma_code_signed(delta.0, &delta.1));
        }
        // The new gap is h − (h + off) = −off exactly.
        self.gap.reset();
        fold_signed(&mut self.gap, !off.0, &off.1);
    }

    /// Emit a leaf at exactly the enclosing frame's tracked minimum
    /// (the right-full arm's min side): the watermark web is unchanged
    /// (the value neither undercuts nor exceeds it), and the output
    /// delta re-anchors to the watermark.
    ///
    /// Always a divergence on a verbatim walk: the arm fires only when
    /// the tracked minimum strictly exceeds `h + above`, and `h + above`
    /// is the consumed range's maximum — at or above every input
    /// plateau the emission replaces — so the emitted value moved.
    fn emit_at_min(&mut self, depth: usize) {
        self.diverge();
        debug_assert!(self.started, "a tracked minimum implies an emission");
        // Emitting the true minimum retires any latent, so the fresh
        // zero follower below installs against an exact anchor. Funded:
        // a watermark-anchored delta to the minimum is at least the
        // anchor's stale excess (the previous output sits at or above
        // the anchor while the tag is set), so the emitted code prices
        // the resolve; on the height-anchored switch the resolve rides
        // the dying divergence gap and the code jointly. The consume
        // path's arm has already drained the register, so its in-cycle
        // case pays nothing here.
        self.stack.resolve_latent();
        let delta = if self.w_anchored {
            // d_out = min − prev_out: the follower verbatim — no read
            // of the wide web at all, the repeated-raise fast path.
            let d = self.stack.follower_take(OUT_FOLLOWER);
            self.stack.materialize(d)
        } else {
            // d_out = min − prev_out = (h − prev_out) − (h − min): the
            // height-to-watermark switch's one bridge read, priced by
            // this emission's own code.
            let fresh = self.stack.lease();
            let mut d = core::mem::replace(&mut self.gap, fresh);
            self.stack.bridge_sub_t(&mut d);
            self.stack.materialize(d)
        };
        // prev_out = min now: the follower restarts at zero.
        let zero = self.stack.lease();
        self.stack.follower_set(OUT_FOLLOWER, zero);
        self.w_anchored = true;
        self.gap.reset();
        self.out.leaf(depth, gamma_code_signed(delta.0, &delta.1));
    }

    /// Copy the event subtree at the cursor unchanged.
    ///
    /// Every leaf is re-emitted at its own depth, deltas passing
    /// straight through (the first through the divergence gap, if
    /// live); the watermark web absorbs each emission in amortized
    /// O(1).
    fn copy_subtree(&mut self, depth: usize) {
        let mut walk = LeafWalk::new();
        while let Some(d) = walk.descend(&mut self.cursor) {
            let (neg, mag) = self.consume_payload();
            self.emit_step(depth + d, neg, mag);
        }
    }

    /// Consume the event subtree at the cursor, returning its maximum
    /// as a nonnegative offset above the exit height.
    ///
    /// Folds the streaming maximum of the subtree's leaf heights:
    /// `max − h`, maintained by subtracting each step and resetting to
    /// zero whenever the running height overtakes it (`h` then sits at
    /// the subtree's last leaf). The offset's width is bounded by the
    /// scanned range's own content, which prices every later fold of
    /// it.
    fn scan_max_consuming(&mut self) -> Signed {
        // The changed flag's topology record: the emission replacing
        // this range reproduces the input's topology iff the range is
        // a single leaf — exactly its first flag bit (`1` = leaf). An
        // unmetered peek of the bit the scan is about to read as its
        // first flag.
        self.range_is_leaf = self.ev[self.pos()];
        let mut above = Extremum::max(self.stack.lease());
        let mut walk = LeafWalk::new();
        while walk.descend(&mut self.cursor).is_some() {
            let (neg, mag) = self.consume_payload();
            above.fold(neg, &mag);
        }
        let result = self.stack.materialize(above.into_offset());
        debug_assert!(!result.0, "the fold floors at zero");
        result
    }
}

/// What the fill walk still owes a suspended branch node.
enum Frame {
    /// A consume-site: its sibling walk is in flight; `pop_site` and
    /// the free-increment fold run at its close.
    Site,
    /// An ordinary node awaiting its left child's cost; the right-side
    /// work (peek, walk, or absent fold) runs when it arrives.
    AwaitLeft,
    /// An ordinary node awaiting its right child's cost, its left cost
    /// deferred on the value stack.
    AwaitRight,
}

/// The fill walk's suspended ancestors, held as bits.
///
/// Three control bits per open branch level plus pop-able word deltas
/// (route keys against a running register; deferred left costs), the
/// route fold's own [`PopStack`] discipline — so a deep spine costs a
/// few heap bits of transient per level, never a machine-word frame.
struct Frames {
    /// Per frame: a consume-site (true) or an ordinary node (false).
    site: BitStack,
    /// Ordinary frames: awaiting the left (false) or the right (true)
    /// child's cost. Site frames: false, unread.
    phase: BitStack,
    /// Ordinary frames: whether the right child is present. Site
    /// frames: whether the site launched the covering fresh pre-scan
    /// ([`FillWalk::pop_site`]'s argument).
    aux: BitStack,
    /// Key deltas (one per frame, against `reg`) and deferred left
    /// costs (two components per left-to-right flip), LIFO with the
    /// frames they serve.
    vals: PopStack,
    /// The top frame's route key (id positions only advance, so every
    /// pushed delta is nonnegative and a pop restores by subtraction).
    reg: usize,
}

impl Frames {
    fn new() -> Self {
        Frames {
            site: BitStack::new(),
            phase: BitStack::new(),
            aux: BitStack::new(),
            vals: PopStack::new(),
            reg: 0,
        }
    }

    /// Open branch levels (the walk's current depth).
    fn len(&self) -> usize {
        self.site.len()
    }

    /// The top frame's kind, if any frame is open.
    fn top(&self) -> Option<Frame> {
        let site = self.site.last()?;
        Some(if site {
            Frame::Site
        } else if self
            .phase
            .last()
            .expect("site and phase stack one bit per frame")
        {
            Frame::AwaitRight
        } else {
            Frame::AwaitLeft
        })
    }

    /// The top frame's aux bit (right presence, or a site's outermost
    /// flag).
    fn aux_top(&self) -> bool {
        self.aux.last().expect("an open frame carries its aux bit")
    }

    /// Suspend an ordinary node: key delta on the value stack, control
    /// bits armed for the left child.
    fn push_node(&mut self, key: usize, right: bool) {
        self.vals.push((key - self.reg) as u64);
        self.reg = key;
        self.site.push(false);
        self.phase.push(false);
        self.aux.push(right);
    }

    /// Suspend a consume-site around its sibling walk.
    fn push_site(&mut self, key: usize, outermost: bool) {
        self.vals.push((key - self.reg) as u64);
        self.reg = key;
        self.site.push(true);
        self.phase.push(false);
        self.aux.push(outermost);
    }

    /// Flip the top (ordinary, left-awaiting) frame to awaiting its
    /// right child, deferring the left cost on the value stack.
    fn flip_to_await_right(&mut self, l_cost: Cost) {
        debug_assert!(
            self.phase.last() == Some(false) && self.site.last() == Some(false),
            "a left-awaiting node flips"
        );
        self.phase.set_last(true);
        self.vals.push(encode_cost_component(l_cost.0));
        self.vals.push(encode_cost_component(l_cost.1));
    }

    /// Pop the control bits of the top frame and restore the key
    /// register, returning the frame's key.
    fn pop_key(&mut self) -> usize {
        self.site.pop();
        self.phase.pop();
        self.aux.pop();
        let key = self.reg;
        self.reg = key - self.vals.pop() as usize;
        key
    }

    /// Close a left-awaiting node whose right side resolved in place:
    /// its route key.
    fn pop_await_left(&mut self) -> usize {
        self.pop_key()
    }

    /// Close a right-awaiting node: its route key and deferred left
    /// cost.
    fn pop_await_right(&mut self) -> (usize, Cost) {
        let d = decode_cost_component(self.vals.pop());
        let e = decode_cost_component(self.vals.pop());
        (self.pop_key(), (e, d))
    }

    /// Close a site frame: its route key and outermost flag.
    fn pop_site(&mut self) -> (usize, bool) {
        let outermost = self.aux_top();
        (self.pop_key(), outermost)
    }
}

/// A local, non-consuming scan of the event subtree at `pos`: the
/// minimum of its leaf heights relative to the height at entry, as a
/// signed offset.
///
/// The absent-right-sibling raise's argument (`min(fill(0, er)) =
/// min(er)`), priced by the scan that reads the range. `first` says
/// whether the subtree's first payload is the stream's absolute first.
fn scan_min_from(ev: &BitsSlice, pos: usize, first: bool) -> Signed {
    let mut cursor = codec::DsiCursor::new_at(ev, pos);
    let mut first = first;
    // The net movement `h − h_entry`, and the minimum's offset from
    // the *current* height (`min − h`) on the streaming fold.
    let mut net = Accumulator::new();
    let mut off = Extremum::min(Accumulator::new());
    let mut walk = LeafWalk::new();
    while walk.descend(&mut cursor).is_some() {
        let code = cursor.read_int().expect("canonical skyline bits");
        let (neg, mag) = if first {
            first = false;
            (false, code)
        } else {
            unzigzag(code)
        };
        fold_signed(&mut net, neg, &mag);
        off.fold(neg, &mag);
    }
    let off = off.into_offset();
    let (n_sign, n_mag) = net.sign_magnitude();
    let (o_sign, o_mag) = off.sign_magnitude();
    let net = (n_sign == Ordering::Less, Base::from(n_mag));
    let off = (o_sign == Ordering::Less, Base::from(o_mag));
    // `min = h + off = h_entry + net + off`.
    signed_sum_base(net, &off)
}

/// The memoized pre-scan: one non-consuming pass over a left-full
/// site's right sibling.
///
/// Computes every interior left-full site's `min(fill(ir, er))` on
/// its own watermark web and records each as a frame-ledger link (the
/// [`Memo`] doc), so the walk arrives with every raise argument
/// resolved and no position is pre-scanned twice.
///
/// The image of the fill equations restricted to the minimum (each
/// arm derived from the oracle's):
///
/// - `min(fill(0, e)) = min(e)` — nothing is raised.
/// - `min(fill(1, e)) = max(e)` — the region is one max leaf.
/// - `min(fill(i, Leaf n)) = n` — a leaf is untouched.
/// - `min(fill((1, ir), (n, el, er))) = min(fill(ir, er))` — the raised
///   left leaf's value `max(max(el), min(fill(ir, er)))` never falls
///   below the right's minimum.
/// - `min(fill((il, 1), (n, el, er))) = min(fill(il, el))` — mirror.
/// - otherwise the minimum of the two children's.
///
/// Every equation is realized as a *virtual emission* into the web —
/// the same open/arm/propagate/close discipline as the walk's — so
/// per-site minima and per-range net movements are never materialized.
struct PreScan<'a, 'm> {
    /// The input skyline stream (read, never consumed; kept beside the
    /// cursor for the replay's spawn positions).
    ev: &'a BitsSlice,
    /// The scan's own forward cursor, opened at the scan's entry; the
    /// walk's cursor is untouched (the scan never consumes).
    cursor: codec::DsiCursor<'a>,
    /// The pre-scan's own range-minimum watermarks.
    stack: MinStack,
    /// `h′ − h(scan entry)`, alive until the first virtual arming
    /// seeds the recording head.
    entry_net: Option<Accumulator>,
    /// The seeded head awaiting the arming that installs it.
    pending_rel: Option<Accumulator>,
    /// The ledger under construction.
    memo: &'m mut Memo,
    /// The sibling-chain keeper for the level the head serves.
    ///
    /// `m_latest − m_first` over the level's recorded sites, folded
    /// forward one link width per sibling record. It dies into the
    /// level's deferred first-child link at the forest parent's close.
    keeper: Accumulator,
    /// The queue slot of the head's level's first site.
    ///
    /// Its link (`m_first − m_parent`) is deferred to the parent's
    /// own record — the one reference not final at the child's close.
    first_slot: usize,
    /// The site-nesting level the head currently serves (0: the
    /// outermost site's own level, whose reference is the scan-entry
    /// height and never defers).
    head_level: u32,
    /// Suspended outer levels, innermost last.
    ///
    /// Per entry: the outer head's final value (`m_first(inner) −
    /// m_ref(outer)` — immutable once pushed, both minima fixed), the
    /// outer keeper, the outer first slot, and the outer level. LIFO
    /// by the site forest's nesting.
    suspend: Vec<(Accumulator, Accumulator, usize, u32)>,
}

impl PreScan<'_, '_> {
    /// The pre-scan image of the walk's arms over the subtree at
    /// `pos`: same reads, virtual emissions; returns the range end.
    ///
    /// The iterative twin of [`FillWalk::walk`]: the descend phase
    /// resolves the range at the cursor or suspends its node on
    /// [`PreFrames`] and enters a child, and the ascend phase resumes
    /// suspended nodes as their children's ranges complete. The stream
    /// cursor threads linearly (every range starts where the previous
    /// one ended), and `first` — whether the next payload is the
    /// stream's absolute first — flips false permanently at the first
    /// payload-consuming read, exactly the value the recursion threads
    /// per call. The site-nesting level is one plus the count of open
    /// site frames: a site's own range walks at `level + 1`, and its
    /// close records at `level`.
    fn run(&mut self, first: bool, id: &mut IdReader) -> usize {
        let mut frames = PreFrames::new();
        let mut level: u32 = 1;
        let mut first = first;
        'descend: loop {
            // Descend: resolve the range at the cursor, or suspend and
            // re-enter on a present child.
            loop {
                let (left, right) = match id.read() {
                    // fill(0, e) = e: every leaf a virtual emission. A
                    // real cursor reads this only at an empty root.
                    IdNode::Empty => {
                        self.copy_range(first);
                        first = false;
                        break;
                    }
                    // Unreachable for canonical ids: every entry hands in
                    // a full child's *sibling* (never full — a `(1, 1)`
                    // node collapses) or a child the caller peeked as
                    // not-full. Kept so the walk realizes
                    // `min(fill(1, e)) = max(e)` totally.
                    IdNode::Full => {
                        let above = self.max_range(first);
                        first = false;
                        self.emit_offset(&above);
                        break;
                    }
                    IdNode::Internal { left, right } => (left, right),
                };
                let leaf = self.cursor.read_bit().expect("canonical skyline bits");
                if leaf {
                    // A leaf under an id node stays: one virtual emission.
                    let step = self.payload(first);
                    first = false;
                    let _ = step;
                    self.emit_here();
                    if left {
                        id.skip();
                    }
                    if right {
                        id.skip();
                    }
                    break;
                }
                if left && matches!(id.peek(), IdNode::Full) {
                    // An interior left-full site: its minimum is the
                    // recorded quantity, and its own raise is a virtual
                    // emission.
                    id.skip();
                    let el_start = self.cursor.position();
                    let above = self.max_range(first);
                    first = false;
                    let l_end = self.cursor.position();
                    if !right {
                        // fill(0, er): the leaves stay as they are, and
                        // the walk re-derives this raise from its own
                        // local scan — nothing is recorded.
                        self.stack.open();
                        self.copy_range(false);
                        self.stack.close();
                        if self.stack.compare_above(&above) != Ordering::Less {
                            self.emit_offset(&above);
                        }
                        break;
                    }
                    let slot = self.reserve(l_end);
                    frames.push_site(slot, el_start);
                    level += 1;
                    self.stack.open();
                    continue; // walk the sibling range
                }
                // An ordinary node: the left child's range first.
                frames.push_node(right);
                self.stack.open();
                if left {
                    continue; // descend into the left child
                }
                // Absent left child: fill(0, el), inlined in the frame
                // just opened.
                self.copy_range(first);
                first = false;
                break;
            }
            // Ascend: resume suspended nodes as their children complete.
            loop {
                let Some(top) = frames.top() else {
                    return self.cursor.position();
                };
                self.stack.close();
                match top {
                    // A site's sibling range finished: record its ledger
                    // link, then decide its raise against the collapse
                    // maximum, re-derived by one bounded replay of the
                    // site's own (disjoint) collapse range — the one wide
                    // quantity the recursion parked per open site, kept
                    // off the frames so nested-site chains stay word-free.
                    PreFrame::Site => {
                        let (slot, el_start) = frames.pop_site();
                        level -= 1;
                        self.record(slot, level);
                        let above = self.replay_max(el_start);
                        if self.stack.compare_above(&above) != Ordering::Less {
                            self.emit_offset(&above);
                        }
                    }
                    // A node's left range finished: peek the right-full
                    // arm, walk the right child, or copy an absent one.
                    PreFrame::AwaitLeft => {
                        let right = frames.aux_top();
                        if right && matches!(id.peek(), IdNode::Full) {
                            // The right-full raise never undercuts the
                            // minimum it is raised to, so only the max
                            // side is a new virtual value.
                            id.skip();
                            let above = self.max_range(false);
                            if self.stack.compare_above(&above) != Ordering::Less {
                                self.emit_offset(&above);
                            }
                            frames.pop_node();
                        } else if right {
                            frames.flip_to_await_right();
                            self.stack.open();
                            continue 'descend; // walk the right child
                        } else {
                            // Absent right child: fill(0, er) in its own
                            // frame.
                            self.stack.open();
                            self.copy_range(false);
                            self.stack.close();
                            frames.pop_node();
                        }
                    }
                    // A node's right range finished: the node is done.
                    PreFrame::AwaitRight => {
                        frames.pop_node();
                    }
                }
            }
        }
    }

    /// Re-derive a closed site's collapse maximum (`max(el) − h` at the
    /// range's own exit) by one non-consuming replay of the range.
    ///
    /// Byte-for-byte the fold [`max_range`](Self::max_range) ran before
    /// the sibling walk — the first leaf arms and is never folded, so
    /// its absolute-vs-delta coding is irrelevant and stays undecoded —
    /// on local state: heights and the entry net folded into the web
    /// once, on the first pass, never again here. Distinct sites'
    /// collapse ranges are disjoint, so the replays add one flat pass
    /// over those positions, never a nesting one.
    fn replay_max(&mut self, pos: usize) -> Signed {
        // The replay jumps back to the site's recorded range start, so
        // it runs on its own cursor; the scan's forward cursor stays
        // where the sibling walk left it.
        let mut cursor = codec::DsiCursor::new_at(self.ev, pos);
        let mut above = Extremum::max(self.stack.lease());
        let mut walk = LeafWalk::new();
        while walk.descend(&mut cursor).is_some() {
            let code = cursor.read_int().expect("canonical skyline bits");
            above.fold_zigzag(code);
        }
        let result = self.stack.materialize(above.into_offset());
        debug_assert!(!result.0, "the fold floors at zero");
        result
    }

    /// Reserve the next consumption-order queue slot for the site
    /// whose range starts at `pos`.
    fn reserve(&mut self, pos: usize) -> usize {
        let slot = self.memo.queue.len();
        self.memo.queue.push(0);
        #[cfg(debug_assertions)]
        {
            self.memo.recorded_check = position_check(self.memo.recorded_check, pos);
        }
        #[cfg(not(debug_assertions))]
        let _ = pos;
        slot
    }

    /// Record the just-closed site's ledger link and re-anchor the
    /// head to this site's minimum.
    ///
    /// Runs at the moment the site's range has closed and its raise
    /// has not yet been emitted: the innermost armed minimum is
    /// exactly this site's `m_s` (its node frame holds only the
    /// range's emissions, and a raise never falls below its own
    /// site's minimum), so the head reads `m_s − m_ref` verbatim. A
    /// sibling record moves the head into the queue as its link; a
    /// level's first record defers its link to the forest parent's
    /// own record — the parent's minimum is not final yet — and
    /// suspends the outer head, whose value is immutable from here on
    /// (both its endpoints are final minima).
    fn record(&mut self, slot: usize, level: u32) {
        // The head and the suspends store true minimum differences, so
        // no anchor-relative content may escape into the ledger: a
        // latent parked by a nested site's close retires here (its one
        // death), making every head read below exact. The recording
        // cycle's own arm has already drained the register, so the
        // sibling-chain case pays nothing.
        self.stack.resolve_latent();
        debug_assert!(
            !self.stack.latent_live(),
            "ledger links and suspends never snapshot a latent-relative quantity"
        );
        // A deeper level is complete iff the head still serves it:
        // its forest parent is THIS site, whose minimum is final now.
        while self.head_level > level {
            self.resolve_inner();
        }
        if self.head_level == level {
            // A sibling record (the scan's outermost site records
            // here too, as the sibling of the entry-height
            // pseudo-site): the head IS the link, `m_s − m_prev`.
            let mut head = self.stack.follower_take(REL_FOLLOWER);
            if head.sign() == Ordering::Equal {
                self.stack.retire(head);
            } else {
                if level > 0 {
                    // keeper: m_latest − m_first, one fold at the
                    // link's own width (its consume read prices it).
                    self.keeper.add_accum(&head);
                }
                self.memo.set_link(slot, head);
            }
        } else {
            debug_assert!(self.head_level < level, "levels resolve LIFO");
            // This level's first site: suspend the outer head by
            // move — its value (m_s − m_ref(outer)) is immutable now.
            let head = self.stack.follower_take(REL_FOLLOWER);
            let keeper = core::mem::replace(&mut self.keeper, self.stack.lease());
            self.suspend
                .push((head, keeper, self.first_slot, self.head_level));
            self.first_slot = slot;
            self.head_level = level;
        }
        // The head restarts at this site's minimum, installed before
        // the raise emission below for uniformity with the walk-side
        // consume discipline, where the ordering is load-bearing (a
        // consume's raise can arm a pending frame, and only an
        // installed follower receives that arm's fold). Here the
        // ordering carries no hazard of its own: the site's range has
        // emitted at least one leaf, arming every enclosing frame (so
        // none is pending), and the raise is min-guarded (never below
        // the tracked minimum), so no arm fold can arrive between
        // this install and the emission.
        let zero = self.stack.lease();
        self.stack.follower_set(REL_FOLLOWER, zero);
    }

    /// Resolve the innermost suspended level.
    ///
    /// Its forest parent's minimum is final — it is the tracked
    /// minimum right now — so the deferred first-child link is one
    /// fold away, and the outer head resumes through the suspended
    /// diff.
    fn resolve_inner(&mut self) {
        // x := (min − m_last) + (m_last − m_first) = min − m_first;
        // the keeper dies into it (its buffer is re-armed for the
        // outer level below — nothing is minted per resolve).
        let mut x = self.stack.follower_take(REL_FOLLOWER);
        x.add_accum(&self.keeper);
        if x.sign() != Ordering::Equal {
            // link(first) = m_first − m_parent = −x: one clone at the
            // link's own width, priced by its consume read.
            let mut link = self.stack.lease();
            link.add_accum(&x);
            link.negate();
            self.memo.set_link(self.first_slot, link);
        }
        // The outer head resumes: (m_first − m_ref(outer)) + (min −
        // m_first). The fold runs NARROW side INTO wide survivor: x
        // dies at the link's own funded width (zero when the minima
        // are shared), while the suspended diff's content — wide when
        // one wide minimum spans a whole first-child chain — is moved,
        // never re-read, so a nested chain over one wide minimum
        // costs nothing per level.
        let (mut susp, keeper, first_slot, level) = self
            .suspend
            .pop()
            .expect("a deeper head level implies a suspended outer level");
        susp.add_accum(&x);
        self.stack.retire(x);
        let dead = core::mem::replace(&mut self.keeper, keeper);
        self.stack.retire(dead);
        self.first_slot = first_slot;
        self.head_level = level;
        self.stack.follower_set(REL_FOLLOWER, susp);
    }

    /// Read one payload at the cursor, folding the step into the height
    /// side of the web (and the entry net while it lives).
    fn payload(&mut self, first: bool) -> Signed {
        let code = self.cursor.read_int().expect("canonical skyline bits");
        let (neg, mag) = if first { (false, code) } else { unzigzag(code) };
        self.stack.fold_height(neg, &mag);
        if let Some(net) = &mut self.entry_net {
            fold_signed(net, neg, &mag);
        }
        (neg, mag)
    }

    /// A virtual emission at the current height.
    fn emit_here(&mut self) {
        self.seed_rel(None);
        self.stack.emit_here();
        self.install_rel();
    }

    /// A virtual emission at `h′ + off`.
    fn emit_offset(&mut self, off: &Signed) {
        self.seed_rel(Some(off));
        self.stack.emit_offset(off);
        self.install_rel();
    }

    /// Before the scan's first arming: seed the recording relation
    /// `rel = v − h(scan entry)` from the dying entry net.
    fn seed_rel(&mut self, off: Option<&Signed>) {
        if self.stack.armed() {
            return;
        }
        let mut rel = self
            .entry_net
            .take()
            .expect("the entry net lives until the first arming");
        if let Some(off) = off {
            fold_signed(&mut rel, off.0, &off.1);
        }
        self.pending_rel = Some(rel);
    }

    /// After the arming emission: install the seeded relation.
    fn install_rel(&mut self) {
        if let Some(rel) = self.pending_rel.take() {
            self.stack.follower_set(REL_FOLLOWER, rel);
        }
    }

    /// Walk an untouched range (`fill(0, e) = e`) at the cursor: every
    /// leaf a virtual emission at its own height.
    fn copy_range(&mut self, first: bool) {
        let mut first = first;
        let mut walk = LeafWalk::new();
        while walk.descend(&mut self.cursor).is_some() {
            let _ = self.payload(first);
            first = false;
            self.emit_here();
        }
    }

    /// Scan a collapsing range at the cursor for its maximum:
    /// `max − h′` at exit as a nonnegative offset. No virtual emissions
    /// — the range's leaves vanish into the raise the caller decides.
    fn max_range(&mut self, first: bool) -> Signed {
        let mut first = first;
        let mut above = Extremum::max(self.stack.lease());
        let mut walk = LeafWalk::new();
        while walk.descend(&mut self.cursor).is_some() {
            let step = self.payload(first);
            first = false;
            above.fold(step.0, &step.1);
        }
        let result = self.stack.materialize(above.into_offset());
        debug_assert!(!result.0, "the fold floors at zero");
        result
    }
}

/// What the pre-scan still owes a suspended node (the [`Frame`] kinds,
/// minus costs — the pre-scan folds no route).
enum PreFrame {
    /// A left-full site: its sibling walk is in flight; the ledger
    /// record and the raise decision run at its close.
    Site,
    /// An ordinary node whose left child's range is in flight.
    AwaitLeft,
    /// An ordinary node whose right child's range is in flight.
    AwaitRight,
}

/// The pre-scan's suspended ancestors, held as bits: [`Frames`]' shape
/// with site payloads (ledger slot, collapse-range start) as pop-able
/// word deltas in place of route keys and costs.
struct PreFrames {
    /// Per frame: a left-full site (true) or an ordinary node (false).
    site: BitStack,
    /// Ordinary frames: awaiting the left (false) or right (true)
    /// child's range. Site frames: false, unread.
    phase: BitStack,
    /// Ordinary frames: whether the right child is present. Site
    /// frames: false, unread.
    aux: BitStack,
    /// Per site frame: the ledger slot delta, then the collapse-range
    /// start delta — both against monotone registers, LIFO with the
    /// frames they serve.
    vals: PopStack,
    /// The top site frame's ledger slot (reserves run in stream order,
    /// so the deltas are nonnegative).
    reg_slot: usize,
    /// The top site frame's collapse-range start (the stream cursor
    /// only advances, so the deltas are nonnegative).
    reg_pos: usize,
}

impl PreFrames {
    fn new() -> Self {
        PreFrames {
            site: BitStack::new(),
            phase: BitStack::new(),
            aux: BitStack::new(),
            vals: PopStack::new(),
            reg_slot: 0,
            reg_pos: 0,
        }
    }

    /// The top frame's kind, if any frame is open.
    fn top(&self) -> Option<PreFrame> {
        let site = self.site.last()?;
        Some(if site {
            PreFrame::Site
        } else if self
            .phase
            .last()
            .expect("site and phase stack one bit per frame")
        {
            PreFrame::AwaitRight
        } else {
            PreFrame::AwaitLeft
        })
    }

    /// The top frame's aux bit (an ordinary node's right presence).
    fn aux_top(&self) -> bool {
        self.aux.last().expect("an open frame carries its aux bit")
    }

    /// Suspend an ordinary node, armed for its left child.
    fn push_node(&mut self, right: bool) {
        self.site.push(false);
        self.phase.push(false);
        self.aux.push(right);
    }

    /// Suspend a left-full site around its sibling walk.
    fn push_site(&mut self, slot: usize, el_start: usize) {
        self.vals.push((slot - self.reg_slot) as u64);
        self.reg_slot = slot;
        self.vals.push((el_start - self.reg_pos) as u64);
        self.reg_pos = el_start;
        self.site.push(true);
        self.phase.push(false);
        self.aux.push(false);
    }

    /// Flip the top (ordinary, left-awaiting) frame to awaiting its
    /// right child.
    fn flip_to_await_right(&mut self) {
        debug_assert!(
            self.phase.last() == Some(false) && self.site.last() == Some(false),
            "a left-awaiting node flips"
        );
        self.phase.set_last(true);
    }

    /// Close an ordinary node's frame.
    fn pop_node(&mut self) {
        self.site.pop();
        self.phase.pop();
        self.aux.pop();
    }

    /// Close a site frame: its ledger slot and collapse-range start.
    fn pop_site(&mut self) -> (usize, usize) {
        self.site.pop();
        self.phase.pop();
        self.aux.pop();
        let el_start = self.reg_pos;
        self.reg_pos = el_start - self.vals.pop() as usize;
        let slot = self.reg_slot;
        self.reg_slot = slot - self.vals.pop() as usize;
        (slot, el_start)
    }
}

/// The sign-and-magnitude sum of two signed magnitudes, as [`Signed`].
fn signed_sum_base(x: Signed, y: &Signed) -> Signed {
    super::emit::signed_sum(x.0, x.1, y.0, &y.1)
}

/// The larger of two signed relative quantities.
fn signed_max(x: &Signed, y: &Signed) -> Signed {
    if signed_le(x, y) {
        y.clone()
    } else {
        x.clone()
    }
}

/// Whether `x <= y` over signed relative quantities. Zero compares
/// equal under either sign, so a `(true, 0)` that a fold produced is
/// ordered correctly.
fn signed_le(x: &Signed, y: &Signed) -> bool {
    let x_neg = x.0 && x.1 != Base::ZERO;
    let y_neg = y.0 && y.1 != Base::ZERO;
    match (x_neg, y_neg) {
        (true, false) => true,
        (false, true) => false,
        (false, false) => x.1 <= y.1,
        (true, true) => x.1 >= y.1,
    }
}

#[cfg(test)]
mod tests;
