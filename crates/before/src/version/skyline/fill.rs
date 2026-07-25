//! `fill` and the `tick` splice on skyline streams: simplify against an
//! id without registering an event; grow when nothing simplifies.
//!
//! `fill(id, e)` collapses every event subtree the id fully owns to a
//! single leaf at that subtree's maximum height, raising a collapsed
//! child to its sibling's filled minimum where that lets the parent
//! simplify (the paper's shortcut arms); `tick` is `fill`, falling back
//! to the [`grow`](super::grow) emit when `fill` changes nothing. The
//! walk pairs the packed id ([`IdReader`]) against the skyline topology
//! recursively and streams one `(depth, payload code)` plateau per
//! output leaf to the collapsing builder, which derives the union
//! topology and performs the equal-sibling normalization. A shortcut
//! raise needs no builder repair: the raised value is computed *before*
//! its leaf is emitted, by a local pre-scan of the range the minimum
//! comes from ([`min_fill_from`]) — for the right-full arm the id is
//! peeked one subtree ahead (a topology-only skip over the left id
//! child) so the pre-scan runs before the left child is walked.
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
//! Derived: one pass over both streams, plus per shortcut site (an
//! event node one of whose id children is the full leaf) one extra
//! local scan of the range its minimum comes from, and per potential
//! right-full site one topology-only skip of the left id subtree.
//! Shortcut sites nest only where the id alternates full children down
//! a spine, so the worst case is `O(input × nested-shortcut depth)`;
//! a walk without nested shortcuts is linear in the two streams. The
//! C3 envelope round prices the constants (the tick rows of
//! `tests/meter.rs`); recursion is guarded by [`crate::recurse`]
//! throughout.
//!
//! # Testing
//!
//! The packed-form `tick` is the behavioral oracle: ticking through the
//! transcoders must reproduce its output stream byte for byte, over the
//! adversarial families crossed with adversarial parties, arbitrary
//! pairs, organic histories, and the exhaustive small scope; `fill`
//! alone is additionally held to `oracle::Version::fill` through the
//! bridge on the same pool, so the splice's two branches are each
//! pinned, not just their composition.

use core::cmp::Ordering;

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
/// The output is byte-identical to transcoding the packed-form `tick`
/// (the differential suite pins it). Canonical uniqueness makes the
/// splice's test exact: `fill` changed something iff its stream differs
/// from the input.
///
/// # Panics
///
/// Panics if the event operand is not a canonical skyline stream — run
/// [`validate`](fn@super::validate) first on untrusted bytes — or
/// declares more live bits than its bytes hold.
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
/// the cost bounds. The output is byte-identical to transcoding the
/// packed-form `fill` (the differential suite pins it).
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
        drifts: Vec::new(),
        out: SkylineBuilder::with_capacity(ev_bits.len()),
    };
    let mut id = IdReader::root(id_bits);
    descend!(0, walk.rec(&mut id, 0));
    debug_assert_eq!(walk.pos, ev_bits.len(), "fill consumes its whole input");
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
    /// Re-anchoring accumulators for suspended right-full arms, one per
    /// nesting level: each folds every consumed input delta, measuring
    /// how far `h` has moved since its arm's pre-scan anchored.
    drifts: Vec<Accum>,
    /// The collapsing output builder.
    out: SkylineBuilder,
}

impl FillWalk<'_> {
    /// Fill the event subtree at the cursor under the id subtree at
    /// `id`, emitting this subtree's plateaus and advancing both
    /// cursors past their subtrees.
    fn rec(&mut self, id: &mut IdReader, depth: usize) {
        let (left, right) = match id.read() {
            // fill(0, e) = e: the id owns nothing here.
            IdNode::Empty => return self.copy_subtree(depth),
            // fill(1, e) = max(e): a fully-owned region collapses.
            IdNode::Full => {
                let below_max = self.scan_max_consuming();
                return self.emit_offset(depth, below_max);
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
            // consuming scan of `el`; the min from a local pre-scan of
            // the right sibling, anchored at `h` (which sits at `el`'s
            // last leaf, exactly the pre-scan's entry) — both known
            // before the collapsed leaf is emitted, so no repair.
            id.skip();
            let below_max = self.scan_max_consuming();
            let raise = if right {
                min_fill_from(
                    self.ev,
                    self.pos,
                    self.first_read,
                    id.bits(),
                    id.pos(),
                    depth,
                )
                .0
            } else {
                // An absent right child is fill(0, er): its minimum is
                // min(er).
                scan_extremum_from(self.ev, self.pos, self.first_read, Extremum::Min).0
            };
            let value_off = signed_max(&below_max, &raise);
            self.emit_offset(depth + 1, value_off);
            self.child(id, right, depth);
            return;
        }
        if right && self.right_id_child_is_full(id, left) {
            // `ir` full: the right child collapses to
            // `max(max(er), min(fill(il, el)))`. The minimum is
            // pre-scanned over the still-unconsumed left range, anchored
            // at `h` here; a drift accumulator re-anchors it to where
            // `h` stands once `el` and `er` have actually been walked.
            let min_left = if left {
                min_fill_from(
                    self.ev,
                    self.pos,
                    self.first_read,
                    id.bits(),
                    id.pos(),
                    depth,
                )
                .0
            } else {
                // An absent left child is fill(0, el): its minimum is
                // min(el).
                scan_extremum_from(self.ev, self.pos, self.first_read, Extremum::Min).0
            };
            self.drifts.push(Accum::new());
            self.child(id, left, depth);
            debug_assert!(
                matches!(id.peek(), IdNode::Full),
                "the lookahead promised a full right id child"
            );
            id.skip();
            let below_max = self.scan_max_consuming();
            let drift = self
                .drifts
                .pop()
                .expect("the arm pushed its drift accumulator");
            let (sign, magnitude) = drift.sign_magnitude();
            // Re-anchor: the pre-scanned minimum was relative to `h` at
            // `el`'s entry; `h` has since moved by `drift`, so the
            // minimum now sits at `min_left − drift` from `h`.
            let min_off = signed_sum_base(
                min_left,
                &signed_neg((sign == Ordering::Less, Base::from(magnitude))),
            );
            let value_off = signed_max(&below_max, &min_off);
            self.emit_offset(depth + 1, value_off);
            return;
        }
        self.child(id, left, depth);
        self.child(id, right, depth);
    }

    /// Whether the id node's right child (known present) is the full
    /// leaf: a topology-only lookahead over the left id subtree, the
    /// reader untouched.
    fn right_id_child_is_full(&self, id: &IdReader, left_present: bool) -> bool {
        let bits = id.bits();
        let pos = id.pos();
        let right_pos = if left_present {
            let mut probe = IdReader::at(bits, pos);
            probe.skip();
            probe.pos()
        } else {
            pos
        };
        matches!(IdReader::at(bits, right_pos).peek(), IdNode::Full)
    }

    /// Fill one id child over its event child: thread the real cursor
    /// where the child is present, a synthetic [`IdReader::Empty`] (the
    /// `fill(0, e) = e` arm) where it is absent.
    fn child(&mut self, id: &mut IdReader, present: bool, depth: usize) {
        let mut empty = IdReader::Empty;
        let c = if present { &mut *id } else { &mut empty };
        descend!(depth + 1, self.rec(c, depth + 1));
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
    /// it into `h`, the live `gap`, and every suspended arm's drift,
    /// and advancing the cursor.
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
        for drift in &mut self.drifts {
            fold(drift, neg, &mag);
        }
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

    /// Copy the event subtree at the cursor unchanged: every leaf
    /// re-emitted at its own depth, deltas passing straight through
    /// (the first through the divergence gap, if live).
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

    /// Consume the event subtree at the cursor, folding the streaming
    /// maximum of its leaf heights; returns `max − h` (`h` then sits at
    /// the subtree's last leaf), a nonnegative offset.
    fn scan_max_consuming(&mut self) -> Signed {
        let mut path = Bits::new();
        // `max − h`, maintained by subtracting each step and resetting
        // to zero whenever the running height overtakes it. The first
        // leaf arms it at zero: the maximum over one leaf is that leaf.
        let mut above = Accum::new();
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
                        return (false, Base::from(magnitude));
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
) -> (Signed, Signed, usize) {
    let mut reader = IdReader::at(id, id_pos);
    descend!(depth, min_fill_rec(ev, pos, first, &mut reader, depth))
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
) -> (Signed, Signed, usize) {
    let (left, right) = match id.read() {
        IdNode::Empty => return scan_extremum_from(ev, pos, first, Extremum::Min),
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
            descend!(depth + 1, min_fill_rec(ev, l_end, false, id, depth + 1))
        } else {
            scan_extremum_from(ev, l_end, false, Extremum::Min)
        };
        let min = signed_sum_base(l_net.clone(), &r_min);
        let net = signed_sum_base(l_net, &r_net);
        return (min, net, ev_end);
    }
    // Left child first (present or the synthetic empty arm).
    let (l_min, l_net, l_end) = {
        let mut empty = IdReader::Empty;
        let c = if left { &mut *id } else { &mut empty };
        descend!(depth + 1, min_fill_rec(ev, pos, first, c, depth + 1))
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
        descend!(depth + 1, min_fill_rec(ev, l_end, false, c, depth + 1))
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
