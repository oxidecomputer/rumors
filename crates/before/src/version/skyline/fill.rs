//! `fill` and the `tick` splice on skyline streams: simplify against an
//! id without registering an event; grow when nothing simplifies.
//!
//! `fill(id, e)` collapses every event subtree the id fully owns to a
//! single leaf at that subtree's maximum height, raising a collapsed
//! child to its sibling's filled minimum where that lets the parent
//! simplify (the paper's shortcut arms); `tick` is `fill`, falling back
//! to the [`grow`](super::grow) emit when `fill` changes nothing. The
//! walk pairs the packed id (`IdReader`) against the skyline topology
//! recursively and streams one `(depth, payload code)` plateau per
//! output leaf to the collapsing builder, which derives the union
//! topology and performs the equal-sibling normalization. A shortcut
//! raise needs no builder repair: the raised value is known before its
//! leaf is emitted. The right-full arm gets it by deferral — the raised
//! leaf is the *right* child's output, so the walk fills the left child
//! first, and the id cursor then sits exactly at the right child's tag:
//! one `O(1)` peek decides the arm, and the raise's minimum argument is
//! the walk's own watermark for the enclosing range. The left-full
//! arm's raised leaf precedes the range its minimum comes from, so it
//! alone pre-scans (`min_fill_from`) — memoized: the scan records every
//! interior left-full site's minimum, so no stream position is ever
//! pre-scanned twice.
//!
//! # Heights stay relative
//!
//! No absolute height is materialized anywhere but the output stream's
//! first leaf (whose code is that absolute, so the read is priced by
//! the write). The walk carries the last consumed input height on one
//! cliff-immune [`Accum`], and every range minimum the shortcut arms
//! can ask for lives in one shared anchor web — the
//! [`watermark`](mod@watermark) stack: `h − min` for the innermost
//! open range plus nonnegative, zero-run-compressed differences
//! outward, so each consumed delta folds into O(1) accumulators and a
//! raise's comparison is an amortized-O(1) sign read. The output-delta
//! register (`h − prev_out` between pass-throughs, watermark-relative
//! after a raise took the tracked minimum) rides the same web, so
//! every emitted code is materialized once, post-collapse, at the
//! width the code itself prices.
//!
//! # Cost
//!
//! Scan: `O(n + m)` bits in the two packed streams [measured: e 1.00
//! on every committed board family at both scales]. The walk consumes
//! every position once; the left-full pre-scan reads a position at
//! most once more (the memo turns every interior left-full site into
//! a lookup, and distinct fresh scans cover disjoint sibling ranges);
//! the absent-sibling extremum scans read their range once ahead of
//! the walk's own copy (a flat ×2, never nesting).
//!
//! Limb: the paired walk's own bookkeeping is amortized O(n + m)
//! accumulator digit touches — each consumed delta folds into O(1)
//! accumulators, each emission's watermark update is an amortized
//! sign read plus propagation whose every fold is a dying operand or
//! the one surviving fold the update's own priced width bounds, and
//! each emitted code is materialized once at its own width [measured:
//! the nested-full and staircase tick cells]. The left-full pre-scan
//! (`min_fill_from`) still materializes per-site minima and per-range
//! net movements: quadratic in the worst case on wide × deep crosses
//! through that arm [measured: the mirror tick cells, red-pinned at
//! both scales]. The committed cure carries the pre-scan on the same
//! anchor web; until it lands, the red board cells are the honest
//! reading.
//!
//! Recursion is guarded by `crate::recurse` throughout; the
//! recursion-depth segments residual at the record scale belongs to
//! the explicit-stack conversion, pinned separately.
//!
//! # Testing
//!
//! `fill` is held to the recursive oracle (`oracle::Version::fill`
//! through the bridge) over the adversarial families crossed with
//! adversarial parties, arbitrary pairs, organic histories, and the
//! exhaustive small scope — canonical uniqueness makes the differential
//! total. `tick` runs through both entry points (the module function
//! and the public `Version::tick`), pinning the splice's plumbing; its
//! two branches take their value witnesses from the fill oracle and the
//! grow suite's oracle and brute-force pins, so each branch is pinned,
//! not just their composition. Deep spines are held to closed-form
//! expected values.

use core::cmp::Ordering;
use std::collections::HashMap;

use crate::codec::accum::Accum;
use crate::codec::{self, Base, Bits, BitsSlice};
use crate::idbits::{IdNode, IdReader};
use crate::recurse::descend;
use crate::step;

use self::watermark::{fold, MinStack, Signed};
use super::build::SkylineBuilder;
use super::{gamma_code, live_bits, unzigzag, zigzag_signed, Encoded};

mod watermark;

/// The follower slot carrying `min − prev_out` while the output delta
/// is watermark-anchored (a raise just emitted the tracked minimum).
const OUT_FOLLOWER: usize = 0;

/// Register one event on the version a skyline stream denotes, from the
/// perspective of a packed id: `fill` if it simplifies the tree, else
/// the [`grow`](super::grow::grow) inflation.
///
/// The differential suite pins each branch against its witness (the
/// recursive oracle's fill; the oracle's and the brute-force minimal
/// inflation). Canonical uniqueness makes the splice's test exact:
/// `fill` changed something iff its stream differs from the input.
///
/// # Panics
///
/// Panics if the event operand is not a canonical skyline stream — run
/// [`validate`](fn@super::validate) first on untrusted bytes — or
/// declares more live bits than its bytes hold. The id must own at
/// least one region: an empty id leaves `fill` the identity, and the
/// grow fallback requires an owning id (debug builds assert it; the
/// result on an empty id is unspecified in release builds).
pub fn tick(ev: &Encoded, id: &crate::Party) -> Encoded {
    let filled = fill(ev, id);
    if filled == *ev {
        super::grow::grow(ev, id)
    } else {
        filled
    }
}

/// `fill(id, e)` on skyline streams: collapse fully-owned subtrees to
/// their maxima (with the paper's sibling raises), as a canonical
/// skyline stream.
///
/// The module doc carries the walk, the relative-height discipline, and
/// the cost bounds. The output is byte-identical to the recursive
/// oracle's fill (the differential suite pins it).
///
/// # Panics
///
/// Panics if the event operand is not a canonical skyline stream — run
/// [`validate`](fn@super::validate) first on untrusted bytes — or
/// declares more live bits than its bytes hold.
pub fn fill(ev: &Encoded, id: &crate::Party) -> Encoded {
    let ev_bits = live_bits(&ev.bytes, ev.bits);
    let id_bits = id.as_bits();
    let mut walk = FillWalk {
        ev: ev_bits,
        pos: 0,
        first_read: true,
        h: Accum::new(),
        gap: Accum::new(),
        w_anchored: false,
        started: false,
        stack: MinStack::new(),
        memo: HashMap::new(),
        out: SkylineBuilder::with_capacity(ev_bits.len()),
    };
    let mut id = IdReader::root(id_bits);
    walk.stack.open();
    descend!(0, walk.rec(&mut id, 0));
    if walk.w_anchored {
        let follower = walk.stack.follower_take(OUT_FOLLOWER);
        walk.stack.retire(follower);
    }
    walk.stack.close();
    debug_assert_eq!(walk.pos, ev_bits.len(), "fill consumes its whole input");
    debug_assert!(
        walk.memo.is_empty(),
        "the walk consumes every memoized minimum"
    );
    let mut bits = walk.out.finish();
    let live = bits.len();
    codec::zero_dead_bits(&mut bits);
    Encoded {
        bytes: bits.into_vec(),
        bits: live,
    }
}

/// The fill walk: input cursor, relative-height state, and the output
/// builder. The `&mut` [`IdReader`] threads alongside as the recursion
/// argument, exactly as the packed walks thread theirs.
struct FillWalk<'a> {
    /// The input skyline stream.
    ev: &'a BitsSlice,
    /// The input cursor.
    pos: usize,
    /// Whether the next payload is the stream's first (coded absolute,
    /// not as a delta).
    first_read: bool,
    /// The last consumed input leaf's height.
    h: Accum,
    /// `h − prev_out` while the output delta is height-anchored: every
    /// consumed step folds in, and every emitted leaf re-derives it.
    /// Idle (zero) while `w_anchored`.
    gap: Accum,
    /// Whether the output delta is watermark-anchored: the last
    /// emission took the tracked minimum, and `min − prev_out` rides
    /// the stack's [`OUT_FOLLOWER`] instead of `gap`.
    w_anchored: bool,
    /// Whether any output leaf has been emitted (the first is coded
    /// absolute).
    started: bool,
    /// The walk's range-minimum watermarks (the anchor web).
    stack: MinStack,
    /// Left-full minima computed ahead of the walk, keyed by the event
    /// range's start position.
    ///
    /// A fresh pre-scan records every interior left-full site it
    /// evaluates, and the walk consumes each entry exactly once on
    /// arrival — so no position is pre-scanned twice.
    memo: HashMap<usize, Signed>,
    /// The collapsing output builder.
    out: SkylineBuilder,
}

impl FillWalk<'_> {
    /// Fill the event subtree at the cursor under the id subtree at
    /// `id`, emitting its plateaus and advancing both cursors past
    /// their subtrees.
    ///
    /// The enclosing range's watermark frame (opened by the caller)
    /// accumulates this subtree's emitted minimum; no per-subtree
    /// quantity is returned or materialized.
    fn rec(&mut self, id: &mut IdReader, depth: usize) {
        let (left, right) = match id.read() {
            // fill(0, e) = e: the id owns nothing here.
            IdNode::Empty => return self.copy_subtree(depth),
            // fill(1, e) = max(e): a fully-owned region collapses.
            IdNode::Full => {
                let above = self.scan_max_consuming();
                self.emit_offset(depth, above);
                return;
            }
            IdNode::Internal { left, right } => (left, right),
        };
        if !self.read_flag() {
            // fill((il, ir), Leaf n) = Leaf n: an event leaf is already
            // simple; lazy-skip the dominated id children.
            let (neg, mag) = self.consume_payload();
            self.emit_step(depth, neg, mag);
            if left {
                id.skip();
            }
            if right {
                id.skip();
            }
            return;
        }

        // An id node over an event node: the shortcut arms collapse a
        // fully-owned child, raised to its sibling's filled minimum.
        if left && matches!(id.peek(), IdNode::Full) {
            // `il` full: the left child collapses to
            // `max(max(el), min(fill(ir, er)))`. The max comes from the
            // consuming scan of `el`; the min — needed before the
            // raised leaf is emitted, ahead of `er`'s walk — from the
            // memo when an enclosing pre-scan already evaluated this
            // site, else from one fresh (and recording) pre-scan of the
            // right sibling, anchored at `h` (which sits at `el`'s last
            // leaf, exactly the pre-scan's entry).
            id.skip();
            let above = self.scan_max_consuming();
            let raise = match self.memo.remove(&self.pos) {
                Some(min) => min,
                None if right => {
                    min_fill_from(
                        self.ev,
                        self.pos,
                        self.first_read,
                        id.bits(),
                        id.pos(),
                        depth,
                        &mut self.memo,
                    )
                    .0
                }
                // An absent right child is fill(0, er): its minimum is
                // min(er).
                None => scan_extremum_from(self.ev, self.pos, self.first_read, Extremum::Min).0,
            };
            let value_off = signed_max(&above, &raise);
            self.emit_offset(depth + 1, value_off);
            self.child(id, right, depth);
            return;
        }
        // Fill the left child first; the id cursor then sits exactly at
        // the right child's tag, so the right-full arm is one `O(1)`
        // peek — no lookahead over the left id subtree.
        self.child(id, left, depth);
        if right && matches!(id.peek(), IdNode::Full) {
            // `ir` full: the right child collapses to
            // `max(max(er), min(fill(il, el)))`. The minimum is the
            // enclosing frame's own watermark — its only emissions so
            // far are the left child's — so the decision is one sign
            // read against the priced scan maximum.
            id.skip();
            let above = self.scan_max_consuming();
            if self.stack.compare_above(&above) == Ordering::Less {
                self.emit_at_min(depth + 1);
            } else {
                self.emit_offset(depth + 1, above);
            }
            return;
        }
        self.child(id, right, depth);
    }

    /// Fill one id child over its event child inside its own watermark
    /// frame: thread the real cursor where the child is present, a
    /// synthetic [`IdReader::Empty`] (the `fill(0, e) = e` arm) where
    /// it is absent.
    fn child(&mut self, id: &mut IdReader, present: bool, depth: usize) {
        self.stack.open();
        let mut empty = IdReader::Empty;
        let c = if present { &mut *id } else { &mut empty };
        descend!(depth + 1, self.rec(c, depth + 1));
        self.stack.close();
    }

    /// Read one topology flag at the cursor, recording the scanned bit.
    fn read_flag(&mut self) -> bool {
        step!();
        codec::scan::record_bits(1);
        let flag = self.ev[self.pos];
        self.pos += 1;
        flag
    }

    /// Decode the payload at the cursor as a signed step (the stream's
    /// first payload is its absolute height, a step from zero), folding
    /// it into the height-anchored accumulators, and advancing the
    /// cursor.
    fn consume_payload(&mut self) -> Signed {
        let (code, next) = codec::decode_int(self.ev, self.pos).expect("canonical skyline bits");
        self.pos = next;
        let (neg, mag) = if self.first_read {
            self.first_read = false;
            (false, code)
        } else {
            unzigzag(code)
        };
        fold(&mut self.h, neg, &mag);
        self.stack.fold_height(neg, &mag);
        if !self.w_anchored {
            fold(&mut self.gap, neg, &mag);
        }
        (neg, mag)
    }

    /// Emit a pass-through leaf at the current input height: the output
    /// delta is the live gap (the step is already folded in), which
    /// equals the input step itself whenever the streams agree.
    fn emit_step(&mut self, depth: usize, neg: bool, mag: Base) {
        self.stack.emit_here();
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
        self.out
            .leaf(depth, gamma_code(&zigzag_signed(delta.0, delta.1)));
    }

    /// Emit a leaf whose value is `h + off`: a collapsed region's max,
    /// or a shortcut arm's raised value decided against the watermark.
    fn emit_offset(&mut self, depth: usize, off: Signed) {
        self.stack.emit_offset(&off);
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
                fold(&mut d, off.0, &off.1);
                self.w_anchored = false;
                self.stack.materialize(d)
            } else {
                // d_out = (h + off) − prev_out = gap + off.
                fold(&mut self.gap, off.0, &off.1);
                self.gap.sign();
                let (sign, magnitude) = self.gap.sign_magnitude();
                (sign == Ordering::Less, Base::from(magnitude))
            };
            self.out
                .leaf(depth, gamma_code(&zigzag_signed(delta.0, delta.1)));
        }
        // The new gap is h − (h + off) = −off exactly.
        self.gap.reset();
        fold(&mut self.gap, !off.0, &off.1);
    }

    /// Emit a leaf at exactly the enclosing frame's tracked minimum
    /// (the right-full arm's min side): the watermark web is unchanged
    /// (the value neither undercuts nor exceeds it), and the output
    /// delta re-anchors to the watermark.
    fn emit_at_min(&mut self, depth: usize) {
        debug_assert!(self.started, "a tracked minimum implies an emission");
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
        self.out
            .leaf(depth, gamma_code(&zigzag_signed(delta.0, delta.1)));
    }

    /// Copy the event subtree at the cursor unchanged.
    ///
    /// Every leaf is re-emitted at its own depth, deltas passing
    /// straight through (the first through the divergence gap, if
    /// live); the watermark web absorbs each emission in amortized
    /// O(1).
    fn copy_subtree(&mut self, depth: usize) {
        let mut path = Bits::new();
        loop {
            while self.read_flag() {
                path.push(false);
            }
            let (neg, mag) = self.consume_payload();
            self.emit_step(depth + path.len(), neg, mag);
            loop {
                match path.pop() {
                    Some(true) => continue,
                    Some(false) => {
                        path.push(true);
                        break;
                    }
                    None => return,
                }
            }
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
        let mut path = Bits::new();
        let mut above = self.stack.lease();
        let mut armed = false;
        loop {
            while self.read_flag() {
                path.push(false);
            }
            let (neg, mag) = self.consume_payload();
            if !armed {
                armed = true;
            } else {
                fold(&mut above, !neg, &mag);
                if above.sign() == Ordering::Less {
                    above.reset();
                }
            }
            loop {
                match path.pop() {
                    Some(true) => continue,
                    Some(false) => {
                        path.push(true);
                        break;
                    }
                    None => {
                        let result = self.stack.materialize(above);
                        debug_assert!(!result.0, "the fold floors at zero");
                        return result;
                    }
                }
            }
        }
    }
}

/// Which extremum a local scan folds.
#[derive(Clone, Copy, PartialEq)]
enum Extremum {
    /// The subtree's maximum leaf height.
    Max,
    /// The subtree's minimum leaf height.
    Min,
}

/// A local, non-consuming scan of the event subtree at `pos`: the
/// chosen extremum of its leaf heights and the subtree's net height
/// movement, both relative to the height at entry, plus the subtree's
/// end position.
///
/// `first` says whether the subtree's first payload is the stream's
/// absolute first.
fn scan_extremum_from(
    ev: &BitsSlice,
    pos: usize,
    first: bool,
    which: Extremum,
) -> (Signed, Signed, usize) {
    let mut path = Bits::new();
    let mut pos = pos;
    let mut first = first;
    // The net movement `h − h_entry` and the extremum's offset from the
    // *current* height (`ext − h`), reset whenever the height crosses
    // it; the first leaf arms the offset at zero (the extremum over one
    // leaf is that leaf).
    let mut net = Accum::new();
    let mut off = Accum::new();
    let mut armed = false;
    loop {
        loop {
            step!();
            codec::scan::record_bits(1);
            let internal = ev[pos];
            pos += 1;
            if !internal {
                break;
            }
            path.push(false);
        }
        let (code, next) = codec::decode_int(ev, pos).expect("canonical skyline bits");
        pos = next;
        let (neg, mag) = if first {
            first = false;
            (false, code)
        } else {
            unzigzag(code)
        };
        fold(&mut net, neg, &mag);
        if !armed {
            armed = true;
        } else {
            fold(&mut off, !neg, &mag);
            let crossed = match which {
                Extremum::Max => off.sign() == Ordering::Less,
                Extremum::Min => off.sign() == Ordering::Greater,
            };
            if crossed {
                off = Accum::new();
            }
        }
        loop {
            match path.pop() {
                Some(true) => continue,
                Some(false) => {
                    path.push(true);
                    break;
                }
                None => {
                    let (n_sign, n_mag) = net.sign_magnitude();
                    let (o_sign, o_mag) = off.sign_magnitude();
                    let net = (n_sign == Ordering::Less, Base::from(n_mag));
                    let off = (o_sign == Ordering::Less, Base::from(o_mag));
                    // `ext = h + off = h_entry + net + off`.
                    let ext = signed_sum_base(net.clone(), &off);
                    return (ext, net, pos);
                }
            }
        }
    }
}

/// The minimum leaf height of `fill(id, e)` over the event subtree at
/// `pos`, relative to the height at entry, without consuming anything;
/// returns the minimum, the subtree's net height movement, and the two
/// end positions.
///
/// The recursive image of the fill equations restricted to the
/// minimum (each arm derived from the oracle's):
///
/// - `min(fill(0, e)) = min(e)` — nothing is raised.
/// - `min(fill(1, e)) = max(e)` — the region is one max leaf.
/// - `min(fill(i, Leaf n)) = n` — a leaf is untouched.
/// - `min(fill((1, ir), (n, el, er))) = min(fill(ir, er))` — the raised
///   left leaf's value `max(max(el), min(fill(ir, er)))` never falls
///   below the right's minimum.
/// - `min(fill((il, 1), (n, el, er))) = min(fill(il, el))` — mirror.
/// - otherwise the minimum of the two children's.
fn min_fill_from(
    ev: &BitsSlice,
    pos: usize,
    first: bool,
    id: &BitsSlice,
    id_pos: usize,
    depth: usize,
    memo: &mut HashMap<usize, Signed>,
) -> (Signed, Signed, usize) {
    let mut reader = IdReader::at(id, id_pos);
    descend!(
        depth,
        min_fill_rec(ev, pos, first, &mut reader, depth, memo)
    )
}

/// [`min_fill_from`]'s recursion, threading a live id reader; returns
/// `(min, net, ev_end)`, the relative quantities anchored at the
/// height on entry.
fn min_fill_rec(
    ev: &BitsSlice,
    pos: usize,
    first: bool,
    id: &mut IdReader,
    depth: usize,
    memo: &mut HashMap<usize, Signed>,
) -> (Signed, Signed, usize) {
    let (left, right) = match id.read() {
        IdNode::Empty => return scan_extremum_from(ev, pos, first, Extremum::Min),
        // Unreachable for canonical ids: every entry hands in a full
        // child's *sibling* (never full — a `(1, 1)` node collapses) or
        // a child the caller peeked as not-full. Kept so the recursion
        // realizes the `min(fill(1, e)) = max(e)` equation totally.
        IdNode::Full => return scan_extremum_from(ev, pos, first, Extremum::Max),
        IdNode::Internal { left, right } => (left, right),
    };
    step!();
    codec::scan::record_bits(1);
    let internal = ev[pos];
    let mut pos = pos + 1;
    if !internal {
        // A leaf under an id node: its own step is both min and net.
        let (code, next) = codec::decode_int(ev, pos).expect("canonical skyline bits");
        pos = next;
        let (neg, mag) = if first { (false, code) } else { unzigzag(code) };
        if left {
            id.skip();
        }
        if right {
            id.skip();
        }
        let step = (neg, mag);
        return (step.clone(), step, pos);
    }

    // Node × node: dispatch the arm, walking children in stream order
    // so the relative anchors compose (a right child's quantities are
    // re-anchored by the left's net movement).
    if left && matches!(id.peek(), IdNode::Full) {
        id.skip();
        // The left's contribution to the minimum is void; its net
        // movement still re-anchors the right.
        let (_, l_net, l_end) = scan_extremum_from(ev, pos, first, Extremum::Max);
        let (r_min, r_net, ev_end) = if right {
            descend!(
                depth + 1,
                min_fill_rec(ev, l_end, false, id, depth + 1, memo)
            )
        } else {
            scan_extremum_from(ev, l_end, false, Extremum::Min)
        };
        // Record this left-full site's minimum for the walk, which
        // will need it before walking the same range: the memo is what
        // keeps every position pre-scanned at most once.
        memo.insert(l_end, r_min.clone());
        let min = signed_sum_base(l_net.clone(), &r_min);
        let net = signed_sum_base(l_net, &r_net);
        return (min, net, ev_end);
    }
    // Left child first (present or the synthetic empty arm).
    let (l_min, l_net, l_end) = {
        let mut empty = IdReader::Empty;
        let c = if left { &mut *id } else { &mut empty };
        descend!(depth + 1, min_fill_rec(ev, pos, first, c, depth + 1, memo))
    };
    if right && matches!(id.peek(), IdNode::Full) {
        id.skip();
        // The raised right leaf never falls below the left's minimum;
        // only the right's net movement matters for the anchor.
        let (_, r_net, ev_end) = scan_extremum_from(ev, l_end, false, Extremum::Max);
        let net = signed_sum_base(l_net.clone(), &r_net);
        return (l_min, net, ev_end);
    }
    let (r_min, r_net, ev_end) = {
        let mut empty = IdReader::Empty;
        let c = if right { &mut *id } else { &mut empty };
        descend!(
            depth + 1,
            min_fill_rec(ev, l_end, false, c, depth + 1, memo)
        )
    };
    let r_min = signed_sum_base(l_net.clone(), &r_min);
    let min = signed_min(&l_min, &r_min);
    let net = signed_sum_base(l_net, &r_net);
    (min, net, ev_end)
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

/// The smaller of two signed relative quantities.
fn signed_min(x: &Signed, y: &Signed) -> Signed {
    if signed_le(x, y) {
        x.clone()
    } else {
        y.clone()
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
