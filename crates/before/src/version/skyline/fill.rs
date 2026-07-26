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
//! cliff-immune [`Accum`], and every range minimum the shortcut arms
//! can ask for lives in one shared anchor web — the `watermark`
//! module's stack: `h − min` for the innermost
//! open range plus nonnegative, zero-run-compressed differences
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
//! the walk's own copy (a flat ×2, never nesting).
//!
//! Limb: accumulator digit touches are amortized linear in the two
//! packed streams [measured: exponent 1.00 with flat constants on the
//! matched spine, both wide × deep shortcut crosses, the memo
//! families — distinct and shared minima, interleaved combs, the
//! wide fan-out — and the descending staircase; the memo module of
//! `tests/meter.rs` pins the families that separate this from every
//! refuted resolution]. Each consumed delta folds into O(1)
//! accumulators; each emission's watermark update is one amortized
//! sign read plus a propagation whose every fold is a dying operand or
//! the one surviving fold the update's own priced width bounds; each
//! emitted code is materialized once, post-collapse, at its own width;
//! the watermark compares fold and restore only the priced offset (or
//! answer post-sign by top-index domination); the extremum scans'
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
//! the watermark discipline already prices, and the pre-scan's
//! recorder adds one amortized sign read per site (the zero-link
//! test). Wide content is read only where an operand dies, a
//! bounded-count lifetime read, or a code prices it — the
//! height↔watermark anchor switches read the surviving web once,
//! priced by the switch emission's own code.
//!
//! Heap: O(paired depth) transient frames plus O(n + m) total live
//! digits; the memo holds one queue entry per covered site — an
//! accumulator only where the link is nonzero, so sites sharing one
//! minimum store nothing — plus one suspended entry per open
//! site-nesting level.
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

/// The follower slot carrying the live ledger relation: walk-side
/// `min − m_r` while the reference is watermark-carried; pre-scan-side
/// the recording head, `min − m_ref` for the level it serves.
const REL_FOLLOWER: usize = 1;

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
        memo: Memo::new(),
        corr: Corr::None,
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
    /// The collapsing output builder.
    out: SkylineBuilder,
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
    links: Vec<Accum>,
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
    fn set_link(&mut self, slot: usize, link: Accum) {
        self.links.push(link);
        self.queue[slot] = u32::try_from(self.links.len()).expect("site count fits u32");
    }

    /// Take `slot`'s link out for its one consuming read, if nonzero.
    fn take_link(&mut self, slot: usize) -> Option<Accum> {
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
    H(Accum),
    /// `min − m_r` rides the stack's [`REL_FOLLOWER`]: the reference
    /// is watermark-carried (the last raise took the minimum side, or
    /// a site's close re-anchored from the walk's own web).
    Min,
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
            // frame ledger when an enclosing pre-scan already evaluated
            // this site, else from one fresh (and recording) pre-scan
            // of the right sibling, anchored at `h` (which sits at
            // `el`'s last leaf, exactly the pre-scan's entry).
            id.skip();
            let above = self.scan_max_consuming();
            if !right {
                // An absent right child is fill(0, er): its minimum is
                // min(er), priced by the scan that reads the range.
                let raise = scan_min_from(self.ev, self.pos, self.first_read);
                let value_off = signed_max(&above, &raise);
                self.emit_offset(depth + 1, value_off);
                self.child(id, right, depth);
                return;
            }
            let outermost = self.pos >= self.memo.covered_until;
            debug_assert_eq!(
                outermost,
                matches!(self.corr, Corr::None),
                "a fresh scan starts exactly where no ledger relation is live"
            );
            if outermost {
                // Uncovered: one fresh pre-scan records this site and
                // every left-full site inside its span.
                self.memo.begin_scan();
                let mut scan = PreScan {
                    ev: self.ev,
                    stack: MinStack::new(),
                    entry_net: Some(Accum::new()),
                    pending_rel: None,
                    memo: &mut self.memo,
                    keeper: Accum::new(),
                    first_slot: usize::MAX,
                    head_level: 0,
                    suspend: Vec::new(),
                };
                let slot = scan.reserve(self.pos);
                scan.stack.open();
                let mut reader = IdReader::at(id.bits(), id.pos());
                let end = descend!(
                    depth,
                    scan.rec(self.pos, self.first_read, &mut reader, depth, 1)
                );
                scan.record(slot, 0);
                let rel = scan.stack.follower_take(REL_FOLLOWER);
                scan.stack.retire(rel);
                scan.stack.close();
                debug_assert!(scan.suspend.is_empty(), "every suspended level resolves");
                self.memo.covered_until = end;
            }
            self.consume_site(&above, depth);
            self.child(id, right, depth);
            self.pop_site(outermost);
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
        if let Corr::H(acc) = &mut self.corr {
            fold(acc, neg, &mag);
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
            self.memo.consumed_check = position_check(self.memo.consumed_check, self.pos);
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
                // d_arm = m_s − min = (m_s − m_r) − (min − m_r): the
                // link dies into the decision.
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
                    // The relation re-anchors to m_s: min − m_s. The
                    // follower installs BEFORE the emission: the raise
                    // can arm a pending frame (moving the tracked
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
        mut acc: Accum,
        link: Option<Accum>,
        above: &Signed,
        depth: usize,
    ) {
        // The decision is sign((h + above) − m_s) = sign(acc + above −
        // link); the link stays folded in, so acc then holds h − m_s
        // and the relation re-anchors to this site for free. The link
        // dies here — its one read.
        fold(&mut acc, above.0, &above.1);
        if let Some(link) = link {
            acc.sub_accum(&link);
            self.stack.retire(link);
        }
        let sign = acc.sign();
        fold(&mut acc, !above.0, &above.1);
        if sign == Ordering::Less {
            // The minimum side: acc is exactly `h − m_s`, the below
            // the arming moves into the web.
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
            let zero = self.stack.lease();
            self.stack.follower_set(REL_FOLLOWER, zero);
            self.corr = Corr::Min;
        }
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

/// A local, non-consuming scan of the event subtree at `pos`: the
/// minimum of its leaf heights relative to the height at entry, as a
/// signed offset.
///
/// The absent-right-sibling raise's argument (`min(fill(0, er)) =
/// min(er)`), priced by the scan that reads the range. `first` says
/// whether the subtree's first payload is the stream's absolute first.
fn scan_min_from(ev: &BitsSlice, pos: usize, first: bool) -> Signed {
    let mut path = Bits::new();
    let mut pos = pos;
    let mut first = first;
    // The net movement `h − h_entry` and the minimum's offset from the
    // *current* height (`min − h`), reset whenever the height crosses
    // below it; the first leaf arms the offset at zero.
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
            if off.sign() == Ordering::Greater {
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
                    // `min = h + off = h_entry + net + off`.
                    return signed_sum_base(net, &off);
                }
            }
        }
    }
}

/// The memoized pre-scan: one non-consuming pass over a left-full
/// site's right sibling.
///
/// Computes every interior left-full site's `min(fill(ir, er))` on
/// its own watermark web and records each as a frame-ledger link (the
/// [`Memo`] doc), so the walk arrives with every raise argument
/// resolved and no position is pre-scanned twice.
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
///
/// Every equation is realized as a *virtual emission* into the web —
/// the same open/arm/propagate/close discipline as the walk's — so
/// per-site minima and per-range net movements are never materialized.
struct PreScan<'a, 'm> {
    /// The input skyline stream (read, never consumed).
    ev: &'a BitsSlice,
    /// The pre-scan's own range-minimum watermarks.
    stack: MinStack,
    /// `h′ − h(scan entry)`, alive until the first virtual arming
    /// seeds the recording head.
    entry_net: Option<Accum>,
    /// The seeded head awaiting the arming that installs it.
    pending_rel: Option<Accum>,
    /// The ledger under construction.
    memo: &'m mut Memo,
    /// The sibling-chain keeper for the level the head serves.
    ///
    /// `m_latest − m_first` over the level's recorded sites, folded
    /// forward one link width per sibling record. It dies into the
    /// level's deferred first-child link at the forest parent's close.
    keeper: Accum,
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
    suspend: Vec<(Accum, Accum, usize, u32)>,
}

impl PreScan<'_, '_> {
    /// The pre-scan image of the walk's arms over the subtree at
    /// `pos`: same reads, virtual emissions; returns the range end.
    ///
    /// `level` is the site-nesting depth of this position — how many
    /// left-full sites of this scan enclose it — so the sites found
    /// directly here record at `level` and their own ranges recurse at
    /// `level + 1`.
    fn rec(
        &mut self,
        pos: usize,
        first: bool,
        id: &mut IdReader,
        depth: usize,
        level: u32,
    ) -> usize {
        let (left, right) = match id.read() {
            IdNode::Empty => return self.copy_range(pos, first),
            // Unreachable for canonical ids: every entry hands in a
            // full child's *sibling* (never full — a `(1, 1)` node
            // collapses) or a child the caller peeked as not-full.
            // Kept so the recursion realizes `min(fill(1, e)) =
            // max(e)` totally.
            IdNode::Full => {
                let (above, end) = self.max_range(pos, first);
                self.emit_offset(&above);
                return end;
            }
            IdNode::Internal { left, right } => (left, right),
        };
        step!();
        codec::scan::record_bits(1);
        let internal = self.ev[pos];
        let pos = pos + 1;
        if !internal {
            // A leaf under an id node stays: one virtual emission.
            let (step, end) = self.payload(pos, first);
            let _ = step;
            self.emit_here();
            if left {
                id.skip();
            }
            if right {
                id.skip();
            }
            return end;
        }
        if left && matches!(id.peek(), IdNode::Full) {
            // An interior left-full site: its minimum is the recorded
            // quantity, and its own raise is a virtual emission.
            id.skip();
            let (above, l_end) = self.max_range(pos, first);
            if !right {
                // fill(0, er): the leaves stay as they are, and the
                // walk re-derives this raise from its own local scan —
                // nothing is recorded.
                self.stack.open();
                let end = self.copy_range(l_end, false);
                self.stack.close();
                if self.stack.compare_above(&above) != Ordering::Less {
                    self.emit_offset(&above);
                }
                return end;
            }
            let slot = self.reserve(l_end);
            let end = self.child(l_end, false, id, true, depth, level + 1);
            self.record(slot, level);
            if self.stack.compare_above(&above) != Ordering::Less {
                self.emit_offset(&above);
            }
            return end;
        }
        let l_end = self.child(pos, first, id, left, depth, level);
        if right && matches!(id.peek(), IdNode::Full) {
            // The right-full raise never undercuts the minimum it is
            // raised to, so only the max side is a new virtual value.
            id.skip();
            let (above, end) = self.max_range(l_end, false);
            if self.stack.compare_above(&above) != Ordering::Less {
                self.emit_offset(&above);
            }
            return end;
        }
        self.child(l_end, false, id, right, depth, level)
    }

    /// One child range inside its own frame, mirroring the walk's.
    fn child(
        &mut self,
        pos: usize,
        first: bool,
        id: &mut IdReader,
        present: bool,
        depth: usize,
        level: u32,
    ) -> usize {
        self.stack.open();
        let mut empty = IdReader::Empty;
        let c = if present { &mut *id } else { &mut empty };
        let end = descend!(depth + 1, self.rec(pos, first, c, depth + 1, level));
        self.stack.close();
        end
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
        // The head restarts at this site's minimum, installed BEFORE
        // the raise emission below: the raise can arm pending frames
        // (moving the tracked minimum), and only an installed
        // follower receives that arm's fold — installed after, the
        // reference goes stale by exactly the arm's delta.
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

    /// Read one payload, folding the step into the height side of the
    /// web (and the entry net while it lives).
    fn payload(&mut self, pos: usize, first: bool) -> (Signed, usize) {
        let (code, next) = codec::decode_int(self.ev, pos).expect("canonical skyline bits");
        let (neg, mag) = if first { (false, code) } else { unzigzag(code) };
        self.stack.fold_height(neg, &mag);
        if let Some(net) = &mut self.entry_net {
            fold(net, neg, &mag);
        }
        ((neg, mag), next)
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
            fold(&mut rel, off.0, &off.1);
        }
        self.pending_rel = Some(rel);
    }

    /// After the arming emission: install the seeded relation.
    fn install_rel(&mut self) {
        if let Some(rel) = self.pending_rel.take() {
            self.stack.follower_set(REL_FOLLOWER, rel);
        }
    }

    /// Walk an untouched range (`fill(0, e) = e`): every leaf a
    /// virtual emission at its own height.
    fn copy_range(&mut self, pos: usize, first: bool) -> usize {
        let mut path = Bits::new();
        let mut pos = pos;
        let mut first = first;
        loop {
            loop {
                step!();
                codec::scan::record_bits(1);
                let internal = self.ev[pos];
                pos += 1;
                if !internal {
                    break;
                }
                path.push(false);
            }
            let (_, next) = self.payload(pos, first);
            first = false;
            pos = next;
            self.emit_here();
            loop {
                match path.pop() {
                    Some(true) => continue,
                    Some(false) => {
                        path.push(true);
                        break;
                    }
                    None => return pos,
                }
            }
        }
    }

    /// Scan a collapsing range for its maximum: `max − h′` at exit as
    /// a nonnegative offset, plus the range end. No virtual emissions
    /// — the range's leaves vanish into the raise the caller decides.
    fn max_range(&mut self, pos: usize, first: bool) -> (Signed, usize) {
        let mut path = Bits::new();
        let mut pos = pos;
        let mut first = first;
        let mut above = self.stack.lease();
        let mut armed = false;
        loop {
            loop {
                step!();
                codec::scan::record_bits(1);
                let internal = self.ev[pos];
                pos += 1;
                if !internal {
                    break;
                }
                path.push(false);
            }
            let (step, next) = self.payload(pos, first);
            first = false;
            pos = next;
            if !armed {
                armed = true;
            } else {
                fold(&mut above, !step.0, &step.1);
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
                        return (result, pos);
                    }
                }
            }
        }
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
