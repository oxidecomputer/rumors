//! The fused `tick` on skyline streams: one fill walk carrying the changed flag
//! and grow's route, then at most one splice.
//!
//! The paper's `event` is `fill`, kept iff it moved the tree, else the cheapest
//! inflation (`grow`). `fill(id, e)` collapses every event subtree the id fully
//! owns to a single leaf at that subtree's maximum height, raising a collapsed
//! child to its sibling's filled minimum where that lets the parent simplify
//! (the paper's shortcut arms). [`tick`] runs the whole `event` in **one fill
//! walk plus at most one splice**: the walk decides `fill(id, e) ≠ e` in-pass —
//! the *changed flag*, tripping at the first emitted plateau that differs from
//! the input plateau it replaces — and folds grow's `(expansions, depth)` route
//! DP over the same `(id, event)` nodes it is already visiting (both live in
//! the `fuse` submodule; the flag's decision rules and the route fold's arms
//! are specified there against the equations). While the flag is clear the
//! output is byte-identical to the consumed input prefix, so nothing is built:
//! the first divergence materializes that prefix wholesale and the walk
//! continues as a direct emission, while a walk that never diverges skips
//! output work entirely and hands its recorded route to [`grow`](super::grow)'s
//! splice emit. The changed branch therefore costs one walk; the unchanged
//! (grow) branch costs one walk plus one splice, with fill's discarded output
//! never built at all.
//!
//! The walk pairs the packed id (`IdReader`) against the skyline topology and
//! streams one `(depth, payload code)` plateau per output leaf to the
//! collapsing builder, which derives the union topology and performs the
//! equal-sibling normalization. A child is *full* when the id owns it entirely
//! (`IdNode::Full`); the shortcut arms fire at an id node with one full child,
//! collapsing that child to a single raised leaf, and the two directions differ
//! only in whether the raised leaf precedes or follows the range whose minimum
//! it needs. A shortcut raise needs no builder repair: the raised value is
//! known before its leaf is emitted. The right-full arm gets it by deferral —
//! the raised leaf is the *right* child's output, so the walk fills the left
//! child first, and the id cursor then sits exactly at the right child's tag:
//! one `O(1)` peek decides the arm, and the raise's minimum argument is the
//! walk's own watermark for the enclosing range. A *left-full site* — an id
//! node whose left child is full — is the other direction: its raised leaf
//! precedes the range its minimum comes from, so it alone pre-scans (the
//! `prescan` submodule) — memoized: one fresh scan records every interior
//! left-full site's minimum as a frame-ledger link, so no stream position is
//! ever pre-scanned twice and no minimum is materialized (the `memo` submodule
//! carries the ledger's discipline).
//!
//! # Heights stay relative
//!
//! No absolute height is materialized anywhere but the output stream's first
//! leaf (whose code is that absolute, so the read is priced by the write). The
//! walk carries the last consumed input height on one cliff-immune
//! [`Accumulator`], and every range minimum the shortcut arms can ask for lives
//! in one shared anchor web — the `watermark` module's web: `h − A` for an
//! anchor at or above the innermost open range's minimum (the excess parked in
//! the web's latent register) plus nonnegative, zero-run-compressed
//! differences outward, so each consumed delta folds into O(1) accumulators and
//! a raise's comparison is an amortized-O(1) sign read. The output-delta
//! register (`h − prev_out` between pass-throughs, watermark-relative after a
//! raise took the tracked minimum) rides the same web, so every emitted code is
//! materialized once, post-collapse, at the width the code itself prices. The
//! pre-scan runs the same discipline on its own web, and each memoized minimum
//! travels as one ledger link — a difference against a reference the walk
//! already holds when it arrives (the `memo` submodule's doc carries the
//! reference discipline), never an absolute — folded into the live relation
//! exactly once, at the raise decision it serves.
//!
//! # Cost
//!
//! Scan: `O(n + m)` bits in the two packed streams [measured: e 1.00 on every
//! committed board family at both scales]. The walk consumes every position
//! once; the left-full pre-scan reads a position at most once more (the memo
//! turns every interior left-full site into a lookup, and distinct fresh scans
//! cover disjoint sibling ranges); the absent-sibling extremum scans read their
//! range once ahead of the walk's own copy (a flat ×2, never nesting). The
//! fused route fold adds no reads of its own — its id reads are
//! the tags the walk's skips pay anyway — and each of the two branch epilogues
//! is one more bounded pass: a divergence replays the matched prefix once, and
//! the unchanged branch's splice emit reads both streams once.
//!
//! Limb: accumulator digit touches are amortized linear in the two packed
//! streams [measured: exponent 1.00 with flat constants across the committed
//! families — the `width_circulation_cost` and memo modules of
//! `tests/meter.rs` name each family, state its shape, and pin the readings
//! that separate this from every refuted discipline].
//!
//! The cost invariant conserves width — every touch is paid by a consumed
//! input code, an emitted output code, or the death of the digits it reads —
//! and every charge names its funding:
//!
//! - a consumed delta: folds into O(1) accumulators, at the consumed code's
//!   own width;
//! - a region the id owns nothing under: consumed as one block — its leaves
//!   fold into a net movement and a streaming minimum, every fold priced by
//!   the code the block scan just read, and the whole region then enters the
//!   walk's registers and the watermark web as one delta and one emission
//!   (the `tick_ownership_hole` envelope pins the block scan engaging; the
//!   `tick_ownership_comb` envelope pins the gate free when regions are too
//!   small to open it);
//! - an emission's watermark update: one amortized sign read, plus a
//!   propagation whose every fold is a dying operand or the one surviving
//!   fold the update's own priced width bounds;
//! - a range close and the next arm: the close *moves* its popped boundary
//!   into the watermark web's latent register and the arm recycles the
//!   register into the new boundary, so the close-reveal cycle's wide content
//!   shuttles by moves at a narrow anchor-relative marginal cost (the
//!   `watermark` module doc carries the register's discipline);
//! - an emitted code: materialized once, post-collapse, at its own width;
//! - a watermark comparison: folds and restores only the priced offset, or
//!   answers post-sign by top-index domination, through the latent ladder
//!   where one is parked;
//! - the extremum scans' reset-on-cross folds: priced by the range they scan;
//! - the absent-sibling raise: compares materialized offsets both priced by
//!   their own scans;
//! - the builder's equal-sibling seam: a one-bit code check;
//! - the frame ledger's operands: each link created once, read once at its
//!   consume, and dead into the raise decision it serves — the `memo`
//!   submodule's doc itemizes the lifetime rules and their funding;
//! - wide content at the anchor switches: the height↔watermark switches read
//!   the surviving web once, priced by the switch emission's own code, with a
//!   parked latent cancelling symbolically on the watermark-to-height switch
//!   and retiring on the other — wide content is never read anywhere but
//!   where an operand dies, a bounded-count lifetime read, or a code prices
//!   it.
//!
//! Heap: O(paired depth) transient frame *bits* plus O(n + m) total live
//! digits; the memo holds one queue entry per covered site — an accumulator
//! only where the link is nonzero, so sites sharing one minimum store nothing —
//! plus one suspended entry per open site-nesting level.
//!
//! Both walks are iterative: suspended ancestors live on explicit stacks —
//! control bits plus pop-able word deltas (`Frames` and the pre-scan's
//! `PreFrames`, the route fold's own `PopStack` discipline) — so paired depth
//! costs a few heap bits per level, never a call-stack frame, and no input
//! depth can grow stacker segments or overflow. The pre-scan parks no wide
//! quantity per open site: a left-full site's raise decision belongs to the
//! walk alone (the `prescan` module doc carries the argument), so its frames
//! hold bits and unit deltas — never an accumulator per open site-nesting
//! level — and the transient stays flat on nested-site chains.
//!
//! # Testing
//!
//! Two committed differentials pin the fused walk directly to the recursive
//! oracle, and they are the entire pin of the flag seam: `tick` byte-identical
//! to the oracle's `event`, and the changed flag ≡ (the oracle's `fill` moved
//! the tree) — over the adversarial families crossed with adversarial parties,
//! arbitrary pairs, organic histories, and the exhaustive small scope, with
//! canonical uniqueness making both differentials total. The grow suite
//! (`grow/tests.rs`) additionally holds the unchanged branch to the oracle's
//! inflation, the brute-force minimal-inflation search, and a reference
//! recursive route probe. The oracle walks on native frames with materialized
//! magnitudes, so these differentials run at oracle-sized operands; the
//! large-operand coverage lives in the deep closed-form witnesses here, the
//! meter suite's closed-form output asserts at its pinned scales, and the
//! board's determinism tripwire.

use core::cmp::Ordering;

use suanpan::Accumulator;

use crate::codec::{self, Base, BitCursor, BitStack, BitsMut, BitsSlice, Int, PopStack};
use crate::idbits::{IdNode, IdReader};

use self::fuse::{decode_cost_component, encode_cost_component, Out, RouteProbe};
use self::memo::Memo;
use self::prescan::PreScan;
use super::grow::Cost;
use super::signed::{
    fold_signed_int, gamma_code_int, gamma_code_signed_int, signed_max, unzigzag, Sign, Signed,
};
use super::walk::{fold_region, skip_leaves, skip_region, Extremum, LeafWalk};
use super::watermark::MinWeb;

mod fuse;
mod memo;
mod prescan;

/// The follower slot carrying `min − prev_out` while the output delta is
/// watermark-anchored (a raise just emitted the tracked minimum).
const OUT_FOLLOWER: usize = 0;

/// The follower slot carrying the live ledger relation: walk-side `min − m_r`
/// while the reference is watermark-carried; pre-scan-side the recording head,
/// `min − m_ref` for the level it serves.
const REL_FOLLOWER: usize = 1;

// The slot constants index the web's follower array: a new follower means
// widening `FOLLOWER_SLOTS` in `watermark.rs` beside its constant here, and
// this binds the two files' rosters at compile time.
const _: () = assert!(
    OUT_FOLLOWER < super::watermark::FOLLOWER_SLOTS
        && REL_FOLLOWER < super::watermark::FOLLOWER_SLOTS
);

/// Register one event on the version a skyline stream denotes, from the
/// perspective of a packed id.
///
/// The event is `fill` if it simplifies the tree, else the
/// [`grow`](super::grow) inflation — one fused walk, then at most one splice
/// (the module doc carries the fusion).
///
/// The differential suite pins the whole `event` against the recursive oracle
/// and the changed flag against the oracle's `fill`, with the unchanged branch
/// additionally held to the brute-force minimal inflation and a reference route
/// probe (`grow/tests.rs`).
///
/// # Panics
///
/// Panics if the event operand is not a canonical skyline stream — run
/// [`validate`](fn@super::validate) first on untrusted bytes. The id must own
/// at least one region: an empty id leaves `fill` the identity, and the grow
/// fallback requires an owning id (debug builds assert it; the result on an
/// empty id is unspecified in release builds).
pub fn tick(event: &BitsSlice, id: &crate::Party) -> BitsMut {
    // `n = 1` performs exactly one fused walk plus at most one splice:
    // the delta against a direct dispatch is two unmetered width tests
    // and one non-allocating `Base` construction. The committed
    // `tick_is_ticks_one` differential and the `ticks_one_is_tick` law
    // pin the outputs equal.
    ticks(event, id, &Base::from(1u8))
}

/// Register `n` events on the version a skyline stream denotes, from the
/// perspective of a packed id — byte-identical to `n` sequential [`tick`]s, in
/// at most two fused walks plus one `+n` splice.
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
/// `n = 0` is the identity (the empty run); `n = 1` performs exactly [`tick`]'s
/// work. The `ticks` differentials in this module's test suite hold every
/// branch to the iterated public tick byte for byte.
///
/// # Panics
///
/// Panics if the event operand is not a canonical skyline stream — run
/// [`validate`](fn@super::validate) first on untrusted bytes. For `n >= 1` the
/// id must own at least one region, exactly as [`tick`] (debug builds assert
/// it; the result on an empty id is unspecified in release builds).
pub fn ticks(event: &BitsSlice, id: &crate::Party, n: &Base) -> BitsMut {
    // Width tests, not value compares: n = 0 has no bits, n = 1 is the
    // one-bit magnitude, and neither test touches the limb meter.
    if n.bits() == 0 {
        return event.to_bitvec();
    }
    match fused_fill(event, id) {
        FillOutcome::Changed(bits) => {
            if n.bits() == 1 {
                // n = 1: the fill output is the whole event.
                return bits;
            }
            let remaining = n.clone() - &Base::from(1u8);
            match fused_fill(&bits, id) {
                FillOutcome::Unchanged(route) => {
                    super::grow::emit(&bits, id.as_bits(), &route, &remaining)
                }
                FillOutcome::Changed(_) => {
                    unreachable!("fill is idempotent: a filled tree cannot fill again")
                }
            }
        }
        FillOutcome::Unchanged(route) => super::grow::emit(event, id.as_bits(), &route, n),
    }
}

/// The fused walk's verdict: `fill` moved the tree (its output stands as the
/// tick), or it was the identity and the recorded route drives the grow splice.
pub(super) enum FillOutcome {
    /// `fill(id, e) ≠ e`: the canonical filled stream.
    Changed(BitsMut),
    /// `fill(id, e) = e`: the inflation route for
    /// [`grow::emit`](super::grow::emit).
    Unchanged(super::grow::Route),
}

/// Run the fill walk over one `(event, id)` pair with the fused state live.
///
/// The changed flag decides the branch, and the route rides along for the
/// unchanged one (the module and `fuse` docs carry both).
///
/// # Panics
///
/// Panics if the event operand is not a canonical skyline stream.
pub(super) fn fused_fill(event_bits: &BitsSlice, id: &crate::Party) -> FillOutcome {
    let id_bits = id.as_bits();
    let mut walk = FillWalk {
        event: event_bits,
        cursor: codec::DsiCursor::new(event_bits),
        first_read: true,
        height: Accumulator::new(),
        gap: Accumulator::new(),
        w_anchored: false,
        range_is_leaf: false,
        web: MinWeb::new(),
        memo: Memo::new(),
        relation: Relation::None,
        out: Out::Unstarted,
        probe: RouteProbe::new(id_bits.len()),
    };
    let mut reader = IdReader::root(id_bits);
    walk.web.open(1);
    walk.walk(&mut reader);
    if walk.w_anchored {
        let follower = walk.web.follower_take(OUT_FOLLOWER);
        walk.web.retire(follower);
    }
    walk.web.close();
    debug_assert_eq!(
        walk.pos(),
        event_bits.len(),
        "fill consumes its whole input"
    );
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
        matches!(walk.relation, Relation::None),
        "the ledger relation dies with the outermost site"
    );
    // Canonicalizing the storage is `Version::from_bits`'s job, the single gate
    // a stream passes through when it becomes a stored value.
    match walk.out.finish(event_bits) {
        Some(bits) => FillOutcome::Changed(bits),
        None => FillOutcome::Unchanged(walk.probe.take_route()),
    }
}

/// The fill walk: input cursor, relative-height state, the fused changed-flag
/// output, and the route probe. The `&mut` [`IdReader`] threads alongside as
/// the recursion argument, exactly as the packed walks thread theirs.
struct FillWalk<'a> {
    /// The input skyline stream (kept beside the cursor for the unmetered
    /// single-flag peek and the sub-scans' spawn positions).
    event: &'a BitsSlice,
    /// The input cursor.
    cursor: codec::DsiCursor<'a>,
    /// Whether the next payload is the stream's first (coded absolute, not as a
    /// delta).
    first_read: bool,
    /// The last consumed input leaf's height.
    height: Accumulator,
    /// `h − prev_out` while the output delta is height-anchored: every consumed
    /// step folds in, and every emitted leaf re-derives it. Idle (zero) while
    /// `w_anchored`.
    gap: Accumulator,
    /// Whether the output delta is watermark-anchored: the last emission took
    /// the tracked minimum, and `min − prev_out` rides the web's
    /// [`OUT_FOLLOWER`] instead of `gap`.
    w_anchored: bool,
    /// Whether the range the last consuming max scan covered was a single leaf.
    ///
    /// The changed flag's topology test at the emission that replaces the
    /// range: a multi-leaf range collapsing to one leaf is a divergence before
    /// any code comparison.
    range_is_leaf: bool,
    /// The walk's range-minimum watermarks (the anchor web), payload-free.
    web: MinWeb<()>,
    /// Left-full minima computed ahead of the walk (the frame ledger).
    ///
    /// A fresh pre-scan records every interior left-full site it evaluates, and
    /// the walk consumes each entry exactly once on arrival — so no position is
    /// pre-scanned twice.
    memo: Memo,
    /// The relation of the walk's ledger reference to its live state.
    ///
    /// Each consume re-anchors it to the consumed site's minimum, and each
    /// site's range close re-anchors it from the walk's own web (which holds
    /// that site's minimum natively at that instant), so it is exactly the
    /// queue-front link's reference at every consume.
    relation: Relation,
    /// The changed flag's realization: the verbatim reference until the first
    /// divergent plateau, the collapsing builder after (the `fuse` doc).
    out: Out,
    /// Grow's route DP, folded in post-order while the flag is clear (the
    /// `fuse` doc).
    probe: RouteProbe,
}

/// The relation of the walk's ledger reference (`m_r`: the last consumed site's
/// minimum, re-anchored to each closing site on the way out) to the walk's live
/// state.
///
/// Invariant: [`Relation::Min`] holds exactly while the web's
/// [`REL_FOLLOWER`] slot is occupied — the variant is the tag, the slot is
/// the storage, and every transition sets or clears both together (the
/// slot's `expect` is what a violation trips).
enum Relation {
    /// No site is open or resolved (only a fresh scan's outermost site consumes
    /// in this state; its reference is the scan-entry height, which is the
    /// walk's height at that instant).
    None,
    /// `h − m_r`, folding input deltas: the reference is height-carried (the
    /// last raise took the scan-maximum side).
    Height(Accumulator),
    /// `min − m_r` rides the web's [`REL_FOLLOWER`]: the reference is
    /// watermark-carried (the last raise took the minimum side, or a site's
    /// close re-anchored from the walk's own web).
    Min,
}

impl FillWalk<'_> {
    /// Fill the whole event stream under the id at the cursor, emitting its
    /// plateaus and advancing both cursors past their trees.
    ///
    /// Iterative: the loop alternates a *descend* phase (process the subtree at
    /// the cursor until it either resolves to a cost or suspends this branch
    /// node on [`Frames`] and enters a child) with an *ascend* phase (fold the
    /// completed child's inflation cost into the suspended node, resuming its
    /// remaining work — the right-full peek, the right child's walk, the site
    /// close). Each child runs inside its own watermark range
    /// ([`MinWeb::open`]/[`close`](MinWeb::close)), absent children as the
    /// inlined `fill(0, e) = e` copy at infeasible cost, so the arms,
    /// emissions, and route folds run in exactly the paired preorder the fill
    /// equations prescribe. The root subtree's cost is dropped: only interior
    /// folds read costs.
    fn walk(&mut self, id: &mut IdReader) {
        let mut frames = Frames::new();
        // Derived state: always equal to `frames.len()` (the assert below),
        // carried as a word so the hot loop never recounts a bit stack.
        let mut depth = 0usize;
        'descend: loop {
            debug_assert_eq!(depth, frames.len(), "one frame per open branch level");
            // Descend: resolve the subtree at the cursor to a cost, or suspend
            // its branch node and re-enter on a present child.
            let mut cost: Cost = loop {
                let (left, right) = match id.read() {
                    // fill(0, e) = e: the id owns nothing here, and grow can
                    // inflate nothing (`grow` skips absent regions). Reachable
                    // only at an empty root: below the root the walk descends
                    // only into children whose presence bits are set, so an
                    // interior read never lands here. Kept so the walk
                    // realizes fill's equation totally instead of asserting
                    // the shape away.
                    IdNode::Empty => {
                        self.copy_subtree(depth);
                        break Cost::MAX;
                    }
                    // fill(1, e) = max(e): a fully-owned region collapses. On a
                    // route-live walk the region is a single leaf — a node
                    // would collapse to fewer plateaus and trip the flag at the
                    // emission below — so the cost is the free increment
                    // `grow(1, n) = (n + 1, 0)`.
                    IdNode::Full => {
                        let above = self.scan_max_consuming();
                        self.emit_offset(depth, above);
                        break Cost::FREE;
                    }
                    IdNode::Internal { left, right } => (left, right),
                };
                // The branch's route key (`Route`'s convention: the bit
                // position of the branch's 2-bit id tag) — here the tag `read`
                // just consumed (an `Internal` reader is always a real cursor).
                let key = id.pos() - 2;
                if self.read_flag() {
                    // fill((il, ir), Leaf n) = Leaf n: an event leaf is already
                    // simple; the dominated id children are lazy-skipped, and
                    // the route's expansion fold rides the skips.
                    self.consume_payload();
                    self.emit_step(depth);
                    break self.probe.expand(key, id, left, right);
                }

                // An id node over an event node: the shortcut arms collapse a
                // fully-owned child, raised to its sibling's filled minimum.
                // Either way the branch's cost folds both children — a full
                // child is the free increment, an absent one is infeasible —
                // and the chosen direction records at the branch's id key.
                if left && matches!(id.peek(), IdNode::Full) {
                    // `il` full: the left child collapses to `max(max(el),
                    // min(fill(ir, er)))`. The max comes from the consuming
                    // scan of `el`; the min — needed before the raised leaf is
                    // emitted, ahead of `er`'s walk — from the frame ledger
                    // when an enclosing pre-scan already evaluated this site,
                    // else from one fresh (and recording) pre-scan of the right
                    // sibling, anchored at `h` (which sits at `el`'s last leaf,
                    // exactly the pre-scan's entry).
                    id.skip();
                    let above = self.scan_max_consuming();
                    if !right {
                        // An absent right child is fill(0, er): its minimum is
                        // min(er), priced by the scan that reads the range; the
                        // copy runs in its own frame, exactly as a child walk
                        // would.
                        let raise = scan_min_from(self.event, self.pos(), self.first_read);
                        let value_offset = signed_max(&above, &raise);
                        self.emit_offset(depth + 1, value_offset);
                        self.web.open(1);
                        self.copy_subtree(depth + 1);
                        self.web.close();
                        break self.probe.join(key, Cost::FREE, Cost::MAX);
                    }
                    let outermost = self.pos() >= self.memo.covered_until;
                    debug_assert_eq!(
                        outermost,
                        matches!(self.relation, Relation::None),
                        "a fresh scan starts exactly where no ledger relation is live"
                    );
                    if outermost {
                        // Uncovered: one fresh pre-scan records this site and
                        // every left-full site inside its span.
                        self.memo.begin_scan();
                        let scan_start = self.pos();
                        let mut scan = PreScan::new(self.event, scan_start, &mut self.memo);
                        let slot = scan.reserve(scan_start);
                        scan.web.open(1);
                        let mut reader = IdReader::at(id.bits(), id.pos());
                        let end = scan.run(&mut reader);
                        scan.record(slot, 0);
                        let relation = scan.web.follower_take(REL_FOLLOWER);
                        scan.web.retire(relation);
                        scan.web.close();
                        debug_assert!(scan.suspend.is_empty(), "every suspended level resolves");
                        self.memo.covered_until = end;
                    }
                    self.consume_site(&above, depth);
                    frames.push_site(key, outermost);
                    self.web.open(1);
                    depth += 1;
                    continue; // walk the right sibling range
                }
                // An ordinary node: the left child first; the id cursor then
                // sits exactly at the right child's tag, so the right-full arm
                // is one `O(1)` peek on the way back up — no lookahead over the
                // left id subtree.
                frames.push_node(key, right);
                self.web.open(1);
                depth += 1;
                if left {
                    continue; // descend into the left child
                }
                // Absent left child: fill(0, el), inlined in the frame just
                // opened; its infeasible cost rises into the node exactly as a
                // child walk's would.
                self.copy_subtree(depth);
                break Cost::MAX;
            };
            // Ascend: fold the completed subtree's cost upward until a
            // suspended node still has a child to walk (or the root completes).
            loop {
                let Some(top) = frames.top() else {
                    debug_assert_eq!(depth, 0, "the root subtree completes at depth zero");
                    let _ = cost; // the root's inflation cost has no parent fold
                    return;
                };
                // One open web range per `Frames` entry: every push above
                // opened exactly one, and this single close retires the one
                // belonging to the entry popped in this iteration — the
                // absent-child copies open and close their own ranges
                // locally, so they never reach here unbalanced.
                self.web.close();
                depth -= 1;
                match top {
                    // A consume-site's sibling walk finished: close the
                    // site's range against the ledger relation, then fold
                    // `grow((1, ir), ·)` — the collapsed left child is the
                    // free increment.
                    Frame::Site => {
                        let (key, outermost) = frames.pop_site();
                        self.pop_site(outermost);
                        cost = self.probe.join(key, Cost::FREE, cost);
                    }
                    // A node's left child finished: peek the right-full
                    // arm, walk the right child, or fold an absent one.
                    Frame::AwaitLeft => {
                        let right = frames.aux_top();
                        if right && matches!(id.peek(), IdNode::Full) {
                            // `ir` full: the right child collapses to
                            // `max(max(er), min(fill(il, el)))`. The minimum is
                            // the enclosing frame's own watermark — its only
                            // emissions so far are the left child's — so the
                            // decision is one sign read against the priced scan
                            // maximum.
                            id.skip();
                            let above = self.scan_max_consuming();
                            if self.web.compare_above(&above) == Ordering::Less {
                                self.emit_at_min(depth + 1);
                            } else {
                                self.emit_offset(depth + 1, above);
                            }
                            let key = frames.pop_await_left();
                            cost = self.probe.join(key, cost, Cost::FREE);
                        } else if right {
                            frames.flip_to_await_right(cost);
                            self.web.open(1);
                            depth += 1;
                            continue 'descend; // walk the right child
                        } else {
                            // Absent right child: fill(0, er) in its own frame,
                            // infeasible for the route.
                            self.web.open(1);
                            self.copy_subtree(depth + 1);
                            self.web.close();
                            let key = frames.pop_await_left();
                            cost = self.probe.join(key, cost, Cost::MAX);
                        }
                    }
                    // A node's right child finished: fold both children.
                    Frame::AwaitRight => {
                        let (key, left_cost) = frames.pop_await_right();
                        cost = self.probe.join(key, left_cost, cost);
                    }
                }
            }
        }
    }

    /// The cursor's bit position: the next node's flag.
    fn pos(&self) -> usize {
        self.cursor.position()
    }

    /// Read one topology flag at the cursor (`true` = leaf), recording the
    /// scanned bit. Single-flag rather than a unary run: the walk interleaves
    /// with the id stream one node at a time here, so there is no run to batch.
    fn read_flag(&mut self) -> bool {
        self.cursor.read_bit().expect("canonical skyline bits")
    }

    /// Decode the payload at the cursor as a signed step (the stream's first
    /// payload is its absolute height, a step from zero), folding it into the
    /// height-anchored accumulators, and advancing the cursor.
    fn consume_payload(&mut self) -> Signed {
        let code = self.cursor.read_int().expect("canonical skyline bits");
        let (sign, magnitude) = if self.first_read {
            self.first_read = false;
            (Sign::Positive, code)
        } else {
            unzigzag(code)
        };
        fold_signed_int(&mut self.height, sign, &magnitude);
        self.web.fold_height(sign, &magnitude);
        if !self.w_anchored {
            fold_signed_int(&mut self.gap, sign, &magnitude);
        }
        if let Relation::Height(relation) = &mut self.relation {
            fold_signed_int(relation, sign, &magnitude);
        }
        Signed { sign, magnitude }
    }

    /// Fold one consumed block's net movement into every height-carried
    /// register: the block scans' one re-entry fold.
    ///
    /// Exactly what [`consume_payload`](Self::consume_payload) would have
    /// folded leaf by leaf: nothing reads the registers between a block's
    /// leaves, so the batched fold is observationally the per-leaf sequence.
    fn fold_block(&mut self, net: &Signed) {
        fold_signed_int(&mut self.height, net.sign, &net.magnitude);
        self.web.fold_height(net.sign, &net.magnitude);
        if !self.w_anchored {
            fold_signed_int(&mut self.gap, net.sign, &net.magnitude);
        }
        if let Relation::Height(relation) = &mut self.relation {
            fold_signed_int(relation, net.sign, &net.magnitude);
        }
    }

    /// Consume the queue-front memoized site: resolve its minimum by one fold
    /// of its ledger link into the live relation, decide the raise, and emit.
    fn consume_site(&mut self, above: &Signed, depth: usize) {
        debug_assert!(
            self.memo.cursor < self.memo.queue.len(),
            "a covered site has a recorded entry"
        );
        #[cfg(debug_assertions)]
        {
            self.memo.consumed_check =
                self::memo::position_check(self.memo.consumed_check, self.pos());
        }
        let link = self.memo.take_link(self.memo.cursor);
        self.memo.cursor += 1;
        match core::mem::replace(&mut self.relation, Relation::None) {
            Relation::None => {
                // Base: the outermost site's reference is the fresh scan's
                // entry height, which is the walk's height right here — the
                // relation starts at zero.
                let relation = self.web.lease();
                self.consume_h_anchored(relation, link, above, depth);
            }
            Relation::Height(relation) => self.consume_h_anchored(relation, link, above, depth),
            Relation::Min => {
                // arm_offset = m_s − A = (m_s − m_r) − (f_stored = A − m_r): the
                // link dies into the decision, and taking the relation raw
                // keeps everything anchor-relative — the latent a preceding
                // close parked cancels out of the comparison and the arming
                // alike, so the cycle's cost is the narrow inter-site movement,
                // never the parked width. (Without the tag, f = m − m_r would
                // gross the full anchor-to-floor gap into arm_offset at every
                // consume.)
                let mut arm_offset = self.web.follower_take(REL_FOLLOWER);
                arm_offset.negate();
                if let Some(link) = link {
                    arm_offset.add_accum(&link);
                    self.web.retire(link);
                }
                if self.web.compare_above_vs(above, &arm_offset) == Ordering::Less {
                    // The minimum side: arm at m_s and emit there.
                    self.web.arm_relative(arm_offset);
                    self.emit_at_min(depth + 1);
                    let zero = self.web.lease();
                    self.web.follower_set(REL_FOLLOWER, zero);
                } else {
                    // The relation re-anchors to m_s: the negated decision
                    // quantity is `A − m_s`, exactly the anchor-relative
                    // content `follower_set` tags when a latent lives (and `min
                    // − m_s` when none does). The follower installs BEFORE the
                    // emission: the raise can arm a pending frame (moving the
                    // tracked minimum), and only an installed follower receives
                    // that arm's fold — installed after, the relation goes
                    // stale by exactly the arm's delta.
                    arm_offset.negate();
                    self.web.follower_set(REL_FOLLOWER, arm_offset);
                    self.emit_offset(depth + 1, above.clone());
                }
                self.relation = Relation::Min;
            }
        }
    }

    /// [`consume_site`](Self::consume_site) with a height-carried relation
    /// `relation = h − m_r`.
    fn consume_h_anchored(
        &mut self,
        mut relation: Accumulator,
        link: Option<Accumulator>,
        above: &Signed,
        depth: usize,
    ) {
        // The decision is sign((h + above) − m_s) = sign(relation + above −
        // link); the link stays folded in, so the accumulator then holds h −
        // m_s and the relation re-anchors to this site for free. The link dies
        // here — its one read.
        fold_signed_int(&mut relation, above.sign, &above.magnitude);
        if let Some(link) = link {
            relation.sub_accum(&link);
            self.web.retire(link);
        }
        let sign = relation.sign();
        fold_signed_int(&mut relation, above.sign.negate(), &above.magnitude);
        if sign == Ordering::Less {
            // The minimum side: the raise lifts the emitted value strictly
            // above the consumed range's maximum, so a verbatim walk diverges
            // here (the first-leaf write below needs the builder live); the
            // first-leaf question is asked before the divergence erases it.
            let first = self.out.is_unstarted();
            self.diverge();
            // The relation accumulator is exactly `h − m_s`, the below the
            // arming moves into the web.
            if first {
                // First output leaf, coded absolute: value = h − below.
                let mut absolute = self.web.lease();
                absolute.add_accum(&self.height);
                absolute.sub_accum(&relation);
                self.web.emit_below_accum(relation);
                let value = self.web.materialize(absolute);
                debug_assert!(!value.sign.is_negative(), "a raised height is a natural");
                self.out.leaf(depth + 1, gamma_code_int(&value.magnitude));
                // prev_out = min: the output delta anchors to the
                // watermark from the start.
                let zero = self.web.lease();
                self.web.follower_set(OUT_FOLLOWER, zero);
                self.w_anchored = true;
                self.gap.reset();
            } else {
                self.web.emit_below_accum(relation);
                self.emit_at_min(depth + 1);
            }
            let zero = self.web.lease();
            self.web.follower_set(REL_FOLLOWER, zero);
            self.relation = Relation::Min;
        } else {
            self.emit_offset(depth + 1, above.clone());
            self.relation = Relation::Height(relation);
        }
    }

    /// Close a consumed site's range: the old relation retires, and — for an
    /// interior site — the reference re-anchors to this site's minimum from the
    /// walk's own web, at zero cost.
    ///
    /// The web holds `m_s` natively at this instant: the site's node frame has
    /// absorbed exactly the range's emissions (whose minimum is `m_s`) and the
    /// raised leaf, and a raise never falls below its own site's minimum (the
    /// fill equations' `max`), so the tracked minimum IS `m_s`. The next
    /// consume at the enclosing level is this site's next sibling, whose ledger
    /// link is relative to exactly this minimum.
    fn pop_site(&mut self, outermost: bool) {
        match core::mem::replace(&mut self.relation, Relation::None) {
            Relation::None => unreachable!("a consumed site keeps a relation"),
            Relation::Height(relation) => self.web.retire(relation),
            Relation::Min => {
                let relation = self.web.follower_take(REL_FOLLOWER);
                self.web.retire(relation);
            }
        }
        if !outermost {
            // The fresh relation starts at zero against the tracked minimum, so
            // the anchor must be exact: a latent parked by a nested site's
            // close retires here (its one death, funded by the mint the input's
            // re-widening climb paid for); the consume cycle's arm has already
            // drained it.
            self.web.resolve_latent();
            let zero = self.web.lease();
            self.web.follower_set(REL_FOLLOWER, zero);
            self.relation = Relation::Min;
        }
    }

    /// Trip the changed flag: the emission in flight differs from the input
    /// plateau it replaces.
    ///
    /// The matched prefix materializes into the real builder and the route
    /// probe dies (the flag routed this pair to the fill branch, where no route
    /// is read); a no-op once diverged.
    fn diverge(&mut self) {
        if self.out.is_verbatim() {
            self.probe.kill();
            self.out.materialize(self.event);
        }
    }

    /// Emit a pass-through leaf at the current input height: the output delta
    /// is the live gap (the caller has just consumed and folded the leaf's
    /// step), which equals the input step itself whenever the streams agree.
    ///
    /// A pass-through leaf can never trip the changed flag: its depth is the
    /// consumed input leaf's own, and its delta re-codes the step just consumed
    /// against an unchanged predecessor — byte-identical by canonical
    /// uniqueness — so a verbatim walk records the match and skips the emission
    /// body outright. When the match is declined the output is built, hence
    /// past its absolute-coded first leaf ([`Out::Built`]'s contract), so the
    /// body codes a delta unconditionally — from the registers, never from
    /// the step itself.
    fn emit_step(&mut self, depth: usize) {
        self.web.emit_here();
        if self.out.note_match(self.pos()) {
            self.gap.reset();
            return;
        }
        let delta = if self.w_anchored {
            // d_out = h − prev_out = (min − prev_out) + (h − min): the anchor
            // switch's one bridge read of the surviving web, priced by this
            // emission's own code.
            let mut out_delta = self.web.follower_take(OUT_FOLLOWER);
            self.web.bridge_add_gap(&mut out_delta);
            self.w_anchored = false;
            self.web.materialize(out_delta)
        } else {
            // d_out = value − prev_out = gap. One collapse-then-read — the
            // common case (nothing consumed since the last emit) reads the
            // single step just folded.
            self.gap.sign();
            let (sign, magnitude) = self.gap.sign_magnitude();
            Signed::from_sign_magnitude(sign, magnitude)
        };
        // The new gap is h − value = 0 exactly.
        self.gap.reset();
        self.out
            .leaf(depth, gamma_code_signed_int(delta.sign, &delta.magnitude));
    }

    /// Emit a leaf whose value is `h + offset`: a collapsed region's max, or a
    /// shortcut arm's raised value decided against the watermark.
    ///
    /// The changed flag's decision site: the emission replaces the range the
    /// caller's consuming scan just covered, so it reproduces the input plateau
    /// iff that range was a single leaf (else the collapse moved topology — the
    /// range's first plateau sits strictly deeper than this one) and the
    /// offset's value is zero (the emitted value is exactly the consumed leaf's
    /// height, hence the same delta — or, at the stream's head, the same
    /// absolute: output position ≡ input position while the walk is verbatim,
    /// so a first leaf compares absolute against absolute). A value-reproducing
    /// raise is a match, never a divergence.
    fn emit_offset(&mut self, depth: usize, offset: Signed) {
        self.web.emit_offset(&offset);
        if self.out.is_verbatim() && self.range_is_leaf && offset.is_zero() {
            // A value-reproducing emission on a verbatim walk always
            // matches (the doc's argument), unlike `emit_step`'s
            // unguarded call, where the bool genuinely discriminates.
            let matched = self.out.note_match(self.pos());
            debug_assert!(matched, "a verbatim walk records a value-reproducing raise");
            let _ = matched;
            self.gap.reset();
            return;
        }
        // The first-leaf question is asked before the divergence erases it
        // (`diverge` is a no-op on an already-built output).
        let first = self.out.is_unstarted();
        self.diverge();
        if first {
            // First output leaf: materialize the absolute — its width is the
            // emitted code's own (the height so far is the input's own first
            // code plus consumed deltas), so the read is priced by the write.
            debug_assert!(!self.w_anchored, "the first emission finds no anchor");
            self.height.sign();
            let (sign, magnitude) = self.height.sign_magnitude();
            debug_assert_ne!(sign, Ordering::Less, "heights are nonnegative");
            let value = Signed {
                sign: Sign::Positive,
                magnitude: Int::from_ubig(magnitude),
            }
            .sum(&offset);
            debug_assert!(!value.sign.is_negative(), "a collapsed height is a natural");
            self.out.leaf(depth, gamma_code_int(&value.magnitude));
        } else {
            let delta = if self.w_anchored {
                // d_out = (h + offset) − prev_out: the bridge read plus
                // the priced offset.
                let mut out_delta = self.web.follower_take(OUT_FOLLOWER);
                self.web.bridge_add_gap(&mut out_delta);
                fold_signed_int(&mut out_delta, offset.sign, &offset.magnitude);
                self.w_anchored = false;
                self.web.materialize(out_delta)
            } else {
                // d_out = (h + offset) − prev_out = gap + offset.
                fold_signed_int(&mut self.gap, offset.sign, &offset.magnitude);
                self.gap.sign();
                let (sign, magnitude) = self.gap.sign_magnitude();
                Signed::from_sign_magnitude(sign, magnitude)
            };
            self.out
                .leaf(depth, gamma_code_signed_int(delta.sign, &delta.magnitude));
        }
        // The new gap is h − (h + offset) = −offset exactly.
        self.gap.reset();
        fold_signed_int(&mut self.gap, offset.sign.negate(), &offset.magnitude);
    }

    /// Emit a leaf at exactly the enclosing frame's tracked minimum (the
    /// right-full arm's min side): the watermark web is unchanged (the value
    /// neither undercuts nor exceeds it), and the output delta re-anchors to
    /// the watermark.
    ///
    /// Always a divergence on a verbatim walk: the arm fires only when the
    /// tracked minimum strictly exceeds `h + above`, and `h + above` is the
    /// consumed range's maximum — at or above every input plateau the emission
    /// replaces — so the emitted value moved.
    fn emit_at_min(&mut self, depth: usize) {
        debug_assert!(
            !self.out.is_unstarted(),
            "a tracked minimum implies an emission"
        );
        self.diverge();
        // Emitting the true minimum retires any latent, so the fresh zero
        // follower below installs against an exact anchor. Funded: a
        // watermark-anchored delta to the minimum is at least the anchor's
        // stale excess (the previous output sits at or above the anchor while
        // the tag is set), so the emitted code prices the resolve; on the
        // height-anchored switch the resolve rides the dying divergence gap and
        // the code jointly. The consume path's arm has already drained the
        // register, so its in-cycle case pays nothing here.
        self.web.resolve_latent();
        let delta = if self.w_anchored {
            // d_out = min − prev_out: the follower verbatim — no read of the
            // wide web at all, the repeated-raise fast path.
            let out_delta = self.web.follower_take(OUT_FOLLOWER);
            self.web.materialize(out_delta)
        } else {
            // d_out = min − prev_out = (h − prev_out) − (h − min): the
            // height-to-watermark switch's one bridge read, priced by this
            // emission's own code.
            let fresh = self.web.lease();
            let mut out_delta = core::mem::replace(&mut self.gap, fresh);
            self.web.bridge_sub_gap(&mut out_delta);
            self.web.materialize(out_delta)
        };
        // prev_out = min now: the follower restarts at zero.
        let zero = self.web.lease();
        self.web.follower_set(OUT_FOLLOWER, zero);
        self.w_anchored = true;
        self.gap.reset();
        self.out
            .leaf(depth, gamma_code_signed_int(delta.sign, &delta.magnitude));
    }

    /// Copy the event subtree at the cursor unchanged.
    ///
    /// `fill(0, e) = e`: the id owns nothing under this subtree, so by the
    /// operation's own semantics no plateau can move. While the walk is
    /// verbatim that makes the whole region one block — a pass-through leaf can
    /// never trip the changed flag, so the region is skip-scanned
    /// ([`skip_region`]) and the walk re-enters on its summary: the net
    /// movement folds once into every height-carried register, the region's
    /// minimum lands as one watermark emission, and the output side records one
    /// matched prefix extension. Post-divergence, every leaf is re-emitted at
    /// its own depth, deltas passing straight through (the first through the
    /// divergence gap); the watermark web absorbs each emission in amortized
    /// O(1).
    fn copy_subtree(&mut self, depth: usize) {
        // Three regimes, selected in order: (1) a verbatim walk over a
        // depth-2+ region block-scans it as one matched prefix extension;
        // (2) post-divergence a depth-2+ region feeds its first leaf
        // through the emission machinery and splices the rest wholesale
        // (`continue_verbatim`); (3) a shallower region feeds leaf by leaf.
        //
        // The first descent's depth routes the region for free — its bits are
        // consumed either way. A depth under 2 (a lone leaf, or a leaf-first
        // pair whose left half is one leaf) stays per-leaf: under a finely
        // interleaved id those tiny shapes dominate, and the block summary's
        // fixed cost exceeds their freight.
        let mut walk = LeafWalk::new();
        let first_leaf_depth = walk
            .descend(&mut self.cursor)
            .expect("a subtree has at least one leaf");
        if first_leaf_depth >= 2 && self.out.is_verbatim() {
            debug_assert!(!self.w_anchored, "a verbatim walk is height-anchored");
            let skip = skip_leaves(
                &mut walk,
                &mut self.cursor,
                self.first_read,
                Some(first_leaf_depth),
            )
            .expect("the descended leaf is pending");
            self.first_read = false;
            self.fold_block(&skip.net);
            self.web.emit_offset(&skip.min_from_exit);
            // The region's last leaf is the last emission: the gap closes
            // exactly, whatever it held at entry.
            self.gap.reset();
            let matched = self.out.note_match(self.pos());
            debug_assert!(matched, "a verbatim walk records the region as matched");
            let _ = matched;
            return;
        }
        // Per-leaf for the first leaf (and, post-divergence, its re-code
        // against the live output delta through the full emission machinery).
        self.consume_payload();
        self.emit_step(depth + first_leaf_depth);
        if first_leaf_depth >= 2 {
            // Post-divergence the rest of the region is byte-identical to the
            // input — every consecutive-leaf delta lies strictly inside the
            // canonical subtree — and splices wholesale; the first leaf is
            // still held at its own depth, which the builder's splice owns
            // and asserts.
            let rest_start = self.pos();
            let skip = skip_leaves(&mut walk, &mut self.cursor, false, None)
                .expect("a region whose first leaf sits below its root has more leaves");
            self.fold_block(&skip.net);
            self.web.emit_offset(&skip.min_from_exit);
            // The region's last leaf is the last emission.
            self.gap.reset();
            self.out.continue_verbatim(
                &self.event[rest_start..self.pos()],
                depth,
                first_leaf_depth,
                skip.last_depth,
                skip.last_code_len,
            );
            return;
        }
        while let Some(leaf_depth) = walk.descend(&mut self.cursor) {
            self.consume_payload();
            self.emit_step(depth + leaf_depth);
        }
    }

    /// Consume the event subtree at the cursor, returning its maximum as a
    /// nonnegative offset above the exit height.
    ///
    /// Folds the streaming maximum of the subtree's leaf heights: `max − h`,
    /// maintained by subtracting each step and resetting to zero whenever the
    /// running height overtakes it (`h` then sits at the subtree's last leaf).
    /// The offset's width is bounded by the scanned range's own content, which
    /// prices every later fold of it.
    fn scan_max_consuming(&mut self) -> Signed {
        // The changed flag's topology record: the emission replacing this range
        // reproduces the input's topology iff the range is a single leaf —
        // exactly its first flag bit (`1` = leaf). An unmetered peek of the bit
        // the scan is about to read as its first flag.
        self.range_is_leaf = self.event[self.pos()];
        let mut above = Extremum::max(self.web.lease());
        let mut walk = LeafWalk::new();
        let first_leaf_depth = walk
            .descend(&mut self.cursor)
            .expect("a subtree has at least one leaf");
        if first_leaf_depth < 2 {
            // A tiny range (the first descent's depth routes for free): the
            // per-leaf fold is cheaper than a block summary's fixed cost.
            // The `tick_collapse_hole` and `tick_raise_hole` envelopes pin
            // the block side engaging on deep ranges, one per arm of this
            // scan (descend-site collapse, ascend-site raise).
            let step = self.consume_payload();
            above.fold(step.sign, &step.magnitude);
            while walk.descend(&mut self.cursor).is_some() {
                let step = self.consume_payload();
                above.fold(step.sign, &step.magnitude);
            }
        } else {
            let mut net = Accumulator::new();
            fold_region(
                &mut walk,
                &mut self.cursor,
                self.first_read,
                &mut net,
                &mut above,
                Some(first_leaf_depth),
            );
            self.first_read = false;
            let (net_sign, net_magnitude) = net.sign_magnitude();
            let net = Signed::from_sign_magnitude(net_sign, net_magnitude);
            self.fold_block(&net);
        }
        let result = self.web.materialize(above.into_offset());
        debug_assert!(!result.sign.is_negative(), "the fold floors at zero");
        result
    }
}

/// What the fill walk still owes a suspended branch node.
enum Frame {
    /// A consume-site: its sibling walk is in flight; `pop_site` and the
    /// free-increment fold run at its close.
    Site,
    /// An ordinary node awaiting its left child's cost; the right-side work
    /// (peek, walk, or absent fold) runs when it arrives.
    AwaitLeft,
    /// An ordinary node awaiting its right child's cost, its left cost deferred
    /// on the value stack.
    AwaitRight,
}

/// A monotone position register whose suspended values ride a [`PopStack`] as
/// deltas.
///
/// The positions pushed only advance (id route keys, ledger slots, stream
/// positions), so every stored delta is nonnegative and a pop restores the
/// previous position exactly by subtraction — a suspended position costs its
/// delta's width in transient, never a machine word.
struct DeltaReg {
    /// The most recently pushed position (zero before any push).
    register: usize,
}

impl DeltaReg {
    fn new() -> Self {
        DeltaReg { register: 0 }
    }

    /// Suspend `position`: its delta against the register goes onto `values`,
    /// and the register advances to it.
    fn push(&mut self, values: &mut PopStack, position: usize) {
        debug_assert!(
            position >= self.register,
            "registered positions only advance"
        );
        values.push((position - self.register) as u64);
        self.register = position;
    }

    /// Restore the register to the previous position, returning the popped
    /// one.
    fn pop(&mut self, values: &mut PopStack) -> usize {
        let position = self.register;
        self.register = position - values.pop() as usize;
        position
    }
}

/// The fill walk's suspended ancestors, held as bits.
///
/// Three control bits per open branch level plus pop-able word deltas (route
/// keys against the [`DeltaReg`] register; deferred left costs), the route
/// fold's own [`PopStack`] discipline — so a deep spine costs a few heap bits
/// of transient per level, never a machine-word frame.
struct Frames {
    /// Per frame: a consume-site (true) or an ordinary node (false).
    site: BitStack,
    /// Ordinary frames: awaiting the left (false) or the right (true) child's
    /// cost. Site frames: false, unread.
    phase: BitStack,
    /// Ordinary frames: whether the right child is present. Site frames:
    /// whether the site launched the covering fresh pre-scan
    /// ([`FillWalk::pop_site`]'s argument).
    aux: BitStack,
    /// Key deltas (one per frame, against the `keys` register) and deferred
    /// left costs (two components per left-to-right flip), LIFO with the
    /// frames they serve.
    values: PopStack,
    /// The top frame's route key (id positions only advance).
    keys: DeltaReg,
}

impl Frames {
    fn new() -> Self {
        Frames {
            site: BitStack::new(),
            phase: BitStack::new(),
            aux: BitStack::new(),
            values: PopStack::new(),
            keys: DeltaReg::new(),
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

    /// The top frame's aux bit (right presence, or a site's outermost flag).
    fn aux_top(&self) -> bool {
        self.aux.last().expect("an open frame carries its aux bit")
    }

    /// Suspend an ordinary node: key delta on the value stack, control bits
    /// armed for the left child.
    fn push_node(&mut self, key: usize, right: bool) {
        self.keys.push(&mut self.values, key);
        self.site.push(false);
        self.phase.push(false);
        self.aux.push(right);
    }

    /// Suspend a consume-site around its sibling walk.
    fn push_site(&mut self, key: usize, outermost: bool) {
        self.keys.push(&mut self.values, key);
        self.site.push(true);
        self.phase.push(false);
        self.aux.push(outermost);
    }

    /// Flip the top (ordinary, left-awaiting) frame to awaiting its right
    /// child, deferring the left cost on the value stack.
    fn flip_to_await_right(&mut self, left_cost: Cost) {
        debug_assert!(
            self.phase.last() == Some(false) && self.site.last() == Some(false),
            "a left-awaiting node flips"
        );
        self.phase.set_last(true);
        self.values
            .push(encode_cost_component(left_cost.expansions));
        self.values.push(encode_cost_component(left_cost.depth));
    }

    /// Pop the control bits of the top frame and restore the key register,
    /// returning the frame's key.
    fn pop_key(&mut self) -> usize {
        self.site.pop();
        self.phase.pop();
        self.aux.pop();
        self.keys.pop(&mut self.values)
    }

    /// Close a left-awaiting node whose right side resolved in place: its route
    /// key.
    fn pop_await_left(&mut self) -> usize {
        self.pop_key()
    }

    /// Close a right-awaiting node: its route key and deferred left cost.
    fn pop_await_right(&mut self) -> (usize, Cost) {
        let depth = decode_cost_component(self.values.pop());
        let expansions = decode_cost_component(self.values.pop());
        (self.pop_key(), Cost { expansions, depth })
    }

    /// Close a site frame: its route key and outermost flag.
    fn pop_site(&mut self) -> (usize, bool) {
        let outermost = self.aux_top();
        (self.pop_key(), outermost)
    }
}

/// A local, non-consuming scan of the event subtree at `pos`: the minimum of
/// its leaf heights relative to the height at entry, as a signed offset.
///
/// The absent-right-sibling raise's argument (`min(fill(0, er)) = min(er)`),
/// priced by the scan that reads the range. `first` says whether the subtree's
/// first payload is the stream's absolute first.
fn scan_min_from(event: &BitsSlice, pos: usize, first: bool) -> Signed {
    let mut cursor = codec::DsiCursor::new_at(event, pos);
    let skip = skip_region(&mut cursor, first);
    // `min = h_entry + net + (min − h_exit)`.
    skip.net.sum(&skip.min_from_exit)
}

#[cfg(test)]
mod tests;
