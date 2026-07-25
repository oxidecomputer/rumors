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
//! first (returning the emitted minimum), and the id cursor then sits
//! exactly at the right child's tag: one `O(1)` peek decides the arm.
//! The left-full arm's raised leaf precedes the range its minimum comes
//! from, so it alone pre-scans (`min_fill_from`) — memoized: the scan
//! records every interior left-full site's minimum, so no stream
//! position is ever pre-scanned twice.
//!
//! # Heights stay relative
//!
//! No absolute height is materialized anywhere but the output stream's
//! first leaf (whose code is that absolute, so the read is priced by
//! the write). The walk carries the last consumed input height and the
//! input−output offset (`h − prev_out`) on cliff-immune [`Accum`]s: a
//! pass-through leaf's output delta is its own input delta while the
//! two streams agree (`gap` zero, no accumulator work), and the first
//! leaf after a collapse re-syncs them with one compacted signed read.
//! A collapsed region's value travels as a streaming-max offset against
//! the running height; a sibling minimum travels as an offset against
//! the height at its range's entry, re-anchored across consumed ranges
//! by a drift accumulator — so every comparison a shortcut arm makes is
//! between same-anchored relative quantities, wide only when the deltas
//! that built them were.
//!
//! # Cost
//!
//! Derived: `O(n + m)` in the two packed streams. The walk consumes
//! every position once; the left-full pre-scan reads a position at
//! most once more (the memo turns every interior left-full site into
//! a lookup, and distinct fresh scans cover disjoint sibling ranges);
//! the absent-sibling extremum scans read their range once ahead of
//! the walk's own copy (a flat ×2, never nesting); and each paired
//! node combines its children's returned quantities with `O(1)`
//! signed operations. Auxiliary state is the memo (one entry per
//! left-full site whose scan ran ahead of the walk, bounded by the
//! id's node count) and the output builder. Signed per-node sums are
//! content-genre arithmetic: wide only when the deltas that built
//! them were, the same regime as the emit kernel's accumulator work.
//! The C3 envelope round prices the constants (the tick rows of
//! `tests/meter.rs`); recursion is guarded by `crate::recurse`
//! throughout.
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

use super::build::SkylineBuilder;
use super::{gamma_code, live_bits, unzigzag, zigzag_signed, Encoded};

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
        started: false,
        memo: HashMap::new(),
        out: SkylineBuilder::with_capacity(ev_bits.len()),
    };
    let mut id = IdReader::root(id_bits);
    descend!(0, walk.rec(&mut id, 0));
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

/// A signed relative quantity: its sign and magnitude, the shape the
/// zigzag coding and the [`Accum`] reads exchange.
type Signed = (bool, Base);

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
    /// `h − prev_out`, an invariant: every consumed step folds in, and
    /// every emitted leaf resets it to the new (exact) difference.
    gap: Accum,
    /// Whether any output leaf has been emitted (the first is coded
    /// absolute).
    started: bool,
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

/// A filled subtree's contribution to its parent's bookkeeping.
///
/// The minimum leaf value this subtree *emitted*, relative to `h` at
/// the subtree's exit, and the net input-height movement its
/// consumption caused; both combine upward with `O(1)` signed
/// operations per node.
type SubtreeOut = (Signed, Signed);

impl FillWalk<'_> {
    /// Fill the event subtree at the cursor under the id subtree at
    /// `id`, returning the subtree's [`SubtreeOut`].
    ///
    /// Emits this subtree's plateaus and advances both cursors past
    /// their subtrees; the parent combines the returned quantities in
    /// `O(1)` signed operations.
    fn rec(&mut self, id: &mut IdReader, depth: usize) -> SubtreeOut {
        let (left, right) = match id.read() {
            // fill(0, e) = e: the id owns nothing here.
            IdNode::Empty => return self.copy_subtree(depth),
            // fill(1, e) = max(e): a fully-owned region collapses.
            IdNode::Full => {
                let (below_max, net) = self.scan_max_consuming();
                self.emit_offset(depth, below_max.clone());
                return (below_max, net);
            }
            IdNode::Internal { left, right } => (left, right),
        };
        if !self.read_flag() {
            // fill((il, ir), Leaf n) = Leaf n: an event leaf is already
            // simple; lazy-skip the dominated id children.
            let (neg, mag) = self.consume_payload();
            self.emit_step(depth, neg, mag.clone());
            if left {
                id.skip();
            }
            if right {
                id.skip();
            }
            // The one emitted leaf sits exactly at `h`.
            return ((false, Base::ZERO), (neg, mag));
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
            let (below_max, net_el) = self.scan_max_consuming();
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
            let value_off = signed_max(&below_max, &raise);
            self.emit_offset(depth + 1, value_off.clone());
            let (min_r, net_er) = self.child(id, right, depth);
            // The raised leaf, re-anchored past `er`'s consumption.
            let raised = signed_sum_base(value_off, &signed_neg(net_er.clone()));
            let min = signed_min(&raised, &min_r).clone();
            let net = signed_sum_base(net_el, &net_er);
            return (min, net);
        }
        // Fill the left child first; the id cursor then sits exactly at
        // the right child's tag, so the right-full arm is one `O(1)`
        // peek — no lookahead over the left id subtree.
        let (min_l, net_el) = self.child(id, left, depth);
        if right && matches!(id.peek(), IdNode::Full) {
            // `ir` full: the right child collapses to
            // `max(max(er), min(fill(il, el)))`. The minimum is the
            // left walk's own emitted minimum — the walk just produced
            // `fill(il, el)` — re-anchored past `er`'s consumption.
            id.skip();
            let (below_max, net_er) = self.scan_max_consuming();
            let min_off = signed_sum_base(min_l, &signed_neg(net_er.clone()));
            let value_off = signed_max(&below_max, &min_off);
            self.emit_offset(depth + 1, value_off.clone());
            let min = signed_min(&min_off, &value_off).clone();
            let net = signed_sum_base(net_el, &net_er);
            return (min, net);
        }
        let (min_r, net_er) = self.child(id, right, depth);
        let min_l = signed_sum_base(min_l, &signed_neg(net_er.clone()));
        let min = signed_min(&min_l, &min_r).clone();
        let net = signed_sum_base(net_el, &net_er);
        (min, net)
    }

    /// Fill one id child over its event child: thread the real cursor
    /// where the child is present, a synthetic [`IdReader::Empty`] (the
    /// `fill(0, e) = e` arm) where it is absent.
    fn child(&mut self, id: &mut IdReader, present: bool, depth: usize) -> SubtreeOut {
        let mut empty = IdReader::Empty;
        let c = if present { &mut *id } else { &mut empty };
        descend!(depth + 1, self.rec(c, depth + 1))
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
    /// it into `h` and the live `gap`, and advancing the cursor.
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
        fold(&mut self.gap, neg, &mag);
        (neg, mag)
    }

    /// Emit a pass-through leaf at the current input height: the output
    /// delta is exactly the live gap (the step is already folded in),
    /// which equals the input step itself whenever the streams agree.
    fn emit_step(&mut self, depth: usize, neg: bool, mag: Base) {
        if !self.started {
            // The output's first leaf is coded absolute. A pass-through
            // first leaf is the input's first (every consumed-but-not-
            // emitted range ends in a collapse emit), whose absolute is
            // the step itself.
            debug_assert!(!neg, "the stream's first height is a natural");
            self.started = true;
            self.gap = Accum::new();
            self.out.leaf(depth, gamma_code(&mag));
            return;
        }
        // `d_out = value − prev_out = gap`. One compacted read — the
        // common case (nothing consumed since the last emit) reads the
        // single step just folded, which is `(neg, mag)` itself.
        let _ = (neg, mag);
        let (sign, magnitude) = self.gap.sign_magnitude();
        let (d_neg, d_mag) = (sign == Ordering::Less, Base::from(magnitude));
        self.gap = Accum::new();
        self.out
            .leaf(depth, gamma_code(&zigzag_signed(d_neg, d_mag)));
    }

    /// Emit a leaf whose value is `h + off`: a collapsed region's max,
    /// or a shortcut arm's raised value. Leaves the divergence gap live
    /// at `−off` (plus nothing else: the gap is rewritten, not folded).
    fn emit_offset(&mut self, depth: usize, off: Signed) {
        if !self.started {
            // First output leaf: materialize the absolute — its width
            // is the emitted code's own, so the read is priced by the
            // write.
            let (sign, magnitude) = self.h.sign_magnitude();
            debug_assert_ne!(sign, Ordering::Less, "heights are nonnegative");
            let value = signed_sum_base((false, Base::from(magnitude)), &off);
            debug_assert!(!value.0, "a collapsed height is a natural");
            self.started = true;
            self.out.leaf(depth, gamma_code(&value.1));
        } else {
            // `d_out = (h + off) − prev_out = gap + off`.
            let (sign, magnitude) = self.gap.sign_magnitude();
            let gap = (sign == Ordering::Less, Base::from(magnitude));
            let delta = signed_sum_base(gap, &off);
            self.out
                .leaf(depth, gamma_code(&zigzag_signed(delta.0, delta.1)));
        }
        // The new gap is `h − (h + off) = −off` exactly.
        self.gap = Accum::new();
        fold(&mut self.gap, !off.0, &off.1);
    }

    /// Copy the event subtree at the cursor unchanged, returning its
    /// minimum and net movement.
    ///
    /// Every leaf is re-emitted at its own depth, deltas passing
    /// straight through (the first through the divergence gap, if
    /// live); the returned quantities fold as the copy streams.
    fn copy_subtree(&mut self, depth: usize) -> SubtreeOut {
        let mut path = Bits::new();
        // `min − h`, reset whenever the running height crosses below it
        // (the copied minimum follows `h` down), and the range's net
        // movement; the first leaf arms the offset at zero.
        let mut off = Accum::new();
        let mut net = Accum::new();
        let mut armed = false;
        loop {
            while self.read_flag() {
                path.push(false);
            }
            let (neg, mag) = self.consume_payload();
            fold(&mut net, neg, &mag);
            if !armed {
                armed = true;
            } else {
                fold(&mut off, !neg, &mag);
                if off.sign() == Ordering::Greater {
                    off = Accum::new();
                }
            }
            self.emit_step(depth + path.len(), neg, mag);
            loop {
                match path.pop() {
                    Some(true) => continue,
                    Some(false) => {
                        path.push(true);
                        break;
                    }
                    None => {
                        let (o_sign, o_mag) = off.sign_magnitude();
                        let (n_sign, n_mag) = net.sign_magnitude();
                        return (
                            (o_sign == Ordering::Less, Base::from(o_mag)),
                            (n_sign == Ordering::Less, Base::from(n_mag)),
                        );
                    }
                }
            }
        }
    }

    /// Consume the event subtree at the cursor, returning its maximum
    /// and net movement.
    ///
    /// Folds the streaming maximum of the subtree's leaf heights:
    /// `max − h` (`h` then sits at the subtree's last leaf), a
    /// nonnegative offset, alongside the range's net height movement.
    fn scan_max_consuming(&mut self) -> (Signed, Signed) {
        let mut path = Bits::new();
        // `max − h`, maintained by subtracting each step and resetting
        // to zero whenever the running height overtakes it. The first
        // leaf arms it at zero: the maximum over one leaf is that leaf.
        let mut above = Accum::new();
        let mut net = Accum::new();
        let mut armed = false;
        loop {
            while self.read_flag() {
                path.push(false);
            }
            let (neg, mag) = self.consume_payload();
            fold(&mut net, neg, &mag);
            if !armed {
                armed = true;
            } else {
                fold(&mut above, !neg, &mag);
                if above.sign() == Ordering::Less {
                    above = Accum::new();
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
                        let (sign, magnitude) = above.sign_magnitude();
                        debug_assert_ne!(sign, Ordering::Less, "the fold floors at zero");
                        let (n_sign, n_mag) = net.sign_magnitude();
                        return (
                            (false, Base::from(magnitude)),
                            (n_sign == Ordering::Less, Base::from(n_mag)),
                        );
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
/// absolute first; `depth` guards the (iterative) walk's meter step
/// context only.
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
/// `(min, net, ev_end, id_end)`, the relative quantities anchored at
/// the height on entry.
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

/// Fold a signed step into an accumulator.
fn fold(acc: &mut Accum, neg: bool, mag: &Base) {
    if neg {
        acc.sub_base(mag);
    } else {
        acc.add_base(mag);
    }
}

/// The sign-and-magnitude sum of two signed magnitudes, as [`Signed`].
fn signed_sum_base(x: Signed, y: &Signed) -> Signed {
    super::emit::signed_sum(x.0, x.1, y.0, &y.1)
}

/// A signed relative quantity, negated (zero stays positive zero).
fn signed_neg(x: Signed) -> Signed {
    if x.1 == Base::ZERO {
        (false, x.1)
    } else {
        (!x.0, x.1)
    }
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
