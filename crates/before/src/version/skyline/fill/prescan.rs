//! The memoized pre-scan: one non-consuming pass over a left-full site's right
//! sibling, ahead of the walk.
//!
//! [`PreScan`] computes every interior left-full site's `min(fill(ir, er))` on
//! its own watermark web and records each as a frame-ledger link (the `memo`
//! module carries the ledger's discipline), so the walk arrives with every
//! raise argument resolved and no position is pre-scanned twice.
//!
//! The scan is the image of the fill equations restricted to the minimum,
//! each arm derived from the oracle's:
//!
//! - `min(fill(0, e)) = min(e)` — nothing is raised.
//! - `min(fill(1, e)) = max(e)` — the region is one max leaf.
//! - `min(fill(i, Leaf n)) = n` — a leaf is untouched.
//! - `min(fill((1, ir), (n, el, er))) = min(fill(ir, er))` — the raised left
//!   leaf's value `max(max(el), min(fill(ir, er)))` never falls below the
//!   right's minimum.
//! - `min(fill((il, 1), (n, el, er))) = min(fill(il, el))` — mirror.
//! - otherwise the minimum of the two children's.
//!
//! The leaf and copy equations are realized as *virtual emissions* into the
//! web — the same open/arm/propagate/close discipline as the walk's — so
//! per-site minima and per-range net movements are never materialized: heights
//! stay relative here exactly as they do in the walk (the `fill` module doc),
//! and each recorded minimum leaves as one ledger link. A left-full site's
//! own raise emits nothing: the raised value never falls below the sibling
//! minimum the site is here to record (its equation above), so mirroring it
//! could not move any tracked minimum — that raise decision belongs to the
//! walk, which derives it from its own consuming scan and the minimum
//! recorded here.

use core::cmp::Ordering;

use suanpan::Accumulator;

use crate::codec::{self, BitCursor, BitStack, BitsSlice, PopStack};
use crate::idbits::{IdNode, IdReader};

use super::super::signed::{fold_signed_int, unzigzag, Sign, Signed};
use super::super::walk::{fold_region, skip_leaves, Extremum, LeafWalk};
use super::super::watermark::MinWeb;
use super::memo::Memo;
use super::{DeltaReg, REL_FOLLOWER};

/// The pre-scan's cursor, web, and recording state (module doc); the `&mut`
/// [`IdReader`] threads alongside as [`run`](Self::run)'s argument, exactly as
/// the fill walk threads its own.
pub(super) struct PreScan<'a, 'm> {
    /// The scan's own forward cursor, opened at the scan's entry; the walk's
    /// cursor is untouched (the scan never consumes).
    cursor: codec::DsiCursor<'a>,
    /// The pre-scan's own range-minimum watermarks.
    pub(super) web: MinWeb<()>,
    /// `h′ − h(scan entry)`, alive until the first virtual arming seeds the
    /// recording head.
    entry_net: Option<Accumulator>,
    /// The seeded head awaiting the arming that installs it.
    pending_relation: Option<Accumulator>,
    /// The ledger under construction.
    memo: &'m mut Memo,
    /// The sibling-chain keeper for the level the head serves.
    ///
    /// `m_latest − m_first` over the level's recorded sites, folded forward one
    /// link width per sibling record. It dies into the level's deferred
    /// first-child link at the forest parent's close.
    keeper: Accumulator,
    /// The queue slot of the head's level's first site, `None` at the
    /// outermost level (which never defers).
    ///
    /// Its link (`m_first − m_parent`) is deferred to the parent's own record —
    /// the one reference not final at the child's close.
    first_slot: Option<usize>,
    /// The site-nesting level the head currently serves (0: the outermost
    /// site's own level, whose reference is the scan-entry height and never
    /// defers).
    head_level: u32,
    /// Suspended outer levels, innermost last, LIFO by the site forest's
    /// nesting.
    ///
    /// Recorder invariants, jointly over the four fields above: `head_level ==
    /// 0` iff `first_slot.is_none()` (only the outermost level never defers a
    /// first-child link), and `suspend.len() == head_level` (one suspended
    /// record per outer level; the terminal case is asserted where the walk
    /// drains the scan).
    pub(super) suspend: Vec<SuspendedLevel>,
}

/// One suspended outer recording level, parked while a deeper level's sites
/// record ([`PreScan::record`]'s deferral arm).
pub(super) struct SuspendedLevel {
    /// The outer head's final value, `m_first(inner) − m_ref(outer)` —
    /// immutable once pushed, both minima final.
    head: Accumulator,
    /// The outer level's sibling-chain keeper.
    keeper: Accumulator,
    /// The outer level's deferred first-site queue slot.
    first_slot: Option<usize>,
    /// The outer level's site-nesting level.
    level: u32,
}

impl<'a, 'm> PreScan<'a, 'm> {
    /// A fresh scan entered at `start`: the cursor at the entry, an empty web,
    /// the entry net alive at zero, no head seeded, the outermost level.
    pub(super) fn new(event: &'a BitsSlice, start: usize, memo: &'m mut Memo) -> Self {
        PreScan {
            cursor: codec::DsiCursor::new_at(event, start),
            web: MinWeb::new(),
            entry_net: Some(Accumulator::new()),
            pending_relation: None,
            memo,
            keeper: Accumulator::new(),
            first_slot: None,
            head_level: 0,
            suspend: Vec::new(),
        }
    }

    /// The pre-scan image of the walk's arms over the subtree at the cursor:
    /// same reads, virtual emissions; returns the range end.
    ///
    /// The iterative twin of the fill walk: the descend phase resolves the
    /// range at the cursor or suspends its node on [`PreFrames`] and enters a
    /// child, and the ascend phase resumes suspended nodes as their children's
    /// ranges complete. One vocabulary caution against the twin file:
    /// `level` here is *site-nesting* level (it moves only at site
    /// open/close), not the fill walk's `depth` (tree depth, one per branch
    /// node) — the two counters are different notions under near-synonymous
    /// names. The stream cursor threads linearly (every range starts
    /// where the previous one ended), and `first` — whether the next payload is
    /// the stream's absolute first — flips false permanently at the first
    /// payload-consuming read, exactly the value the recursion threads per
    /// call. The site-nesting level is one plus the count of open site frames:
    /// a site's own range walks at `level + 1`, and its close records at
    /// `level`.
    pub(super) fn run(&mut self, first: bool, id: &mut IdReader) -> usize {
        let mut frames = PreFrames::new();
        let mut level: u32 = 1;
        let mut first = first;
        'descend: loop {
            // Descend: resolve the range at the cursor, or suspend and re-enter
            // on a present child.
            loop {
                let (left, right) = match id.read() {
                    // fill(0, e) = e: every leaf a virtual emission. A
                    // real cursor reads this only at an empty root.
                    IdNode::Empty => {
                        self.copy_range(first);
                        first = false;
                        break;
                    }
                    // Unreachable for canonical ids: every entry hands in a
                    // full child's *sibling* (never full — a `(1, 1)` node
                    // collapses) or a child the caller peeked as not-full. Kept
                    // so the walk realizes `min(fill(1, e)) = max(e)` totally.
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
                    // An interior left-full site: its minimum is the recorded
                    // quantity. The collapse range is consumed for its height
                    // movement alone — the site's raise is the walk's decision,
                    // never mirrored here (the site-close arm below carries
                    // the argument).
                    id.skip();
                    self.skip_collapse(first);
                    first = false;
                    if !right {
                        // fill(0, er): the leaves stay as they are, and the
                        // walk re-derives this raise from its own local scan —
                        // nothing is recorded.
                        self.web.open(1);
                        self.copy_range(false);
                        self.web.close();
                        break;
                    }
                    let slot = self.reserve(self.cursor.position());
                    frames.push_site(slot);
                    level += 1;
                    self.web.open(1);
                    continue; // walk the sibling range
                }
                // An ordinary node: the left child's range first.
                frames.push_node(right);
                self.web.open(1);
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
                self.web.close();
                match top {
                    // A site's sibling range finished: record its ledger link.
                    // The site's own raise leaves no mark in the web: the
                    // raised value is `max(max(el), m_s)` where `m_s` is
                    // exactly the sibling minimum this range just tracked, so
                    // it never falls below the innermost tracked minimum and a
                    // mirrored emission could not move any web state. The
                    // raise decision itself is the walk's alone — made at the
                    // site's own vantage from its consuming scan and the
                    // ledger link recorded here, the only height where
                    // comparing the collapse maximum against `m_s` is
                    // denominated correctly (the sibling range has moved `h`
                    // by its net between the site and this close).
                    PreFrame::Site => {
                        let slot = frames.pop_site();
                        level -= 1;
                        self.record(slot, level);
                    }
                    // A node's left range finished: peek the right-full arm,
                    // walk the right child, or copy an absent one.
                    PreFrame::AwaitLeft => {
                        let right = frames.aux_top();
                        if right && matches!(id.peek(), IdNode::Full) {
                            // The right-full raise never undercuts the minimum
                            // it is raised to, so only the max side is a new
                            // virtual value.
                            id.skip();
                            let above = self.max_range(false);
                            if self.web.compare_above(&above) != Ordering::Less {
                                self.emit_offset(&above);
                            }
                            frames.pop_node();
                        } else if right {
                            frames.flip_to_await_right();
                            self.web.open(1);
                            continue 'descend; // walk the right child
                        } else {
                            // Absent right child: fill(0, er) in its own
                            // frame.
                            self.web.open(1);
                            self.copy_range(false);
                            self.web.close();
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

    /// Reserve the next consumption-order queue slot for the site whose range
    /// starts at `pos`.
    pub(super) fn reserve(&mut self, pos: usize) -> usize {
        let slot = self.memo.queue.len();
        self.memo.queue.push(None);
        #[cfg(debug_assertions)]
        {
            self.memo.recorded_check = super::memo::position_check(self.memo.recorded_check, pos);
        }
        #[cfg(not(debug_assertions))]
        let _ = pos;
        slot
    }

    /// Record the just-closed site's ledger link and re-anchor the head to this
    /// site's minimum.
    ///
    /// Runs at the moment the site's range has closed: the innermost armed
    /// minimum is exactly this site's `m_s` (its node frame holds only the
    /// range's own emissions — a left-full site's raise is not mirrored at
    /// all, and a raise emission never falls below its own site's minimum),
    /// so the head reads `m_s − m_ref` verbatim. A sibling record
    /// moves the head into the queue as its link; a level's first record
    /// defers its link to the forest parent's own record — the parent's
    /// minimum is not final yet — and suspends the outer head, whose value is
    /// immutable from here on (both its endpoints are final minima).
    pub(super) fn record(&mut self, slot: usize, level: u32) {
        // The head and the suspends store true minimum differences, so no
        // anchor-relative content may escape into the ledger: a latent parked
        // by a nested site's close retires here (its one death), making every
        // head read below exact. The recording cycle's own arm has already
        // drained the register, so the sibling-chain case pays nothing.
        self.web.resolve_latent();
        debug_assert!(
            !self.web.latent_live(),
            "ledger links and suspends never snapshot a latent-relative quantity"
        );
        // A deeper level is complete iff the head still serves it: its forest
        // parent is THIS site, whose minimum is final now.
        while self.head_level > level {
            self.resolve_inner();
        }
        if self.head_level == level {
            // A sibling record (the scan's outermost site records here too, as
            // the sibling of the entry-height pseudo-site): the head IS the
            // link, `m_s − m_prev`.
            let mut head = self.web.follower_take(REL_FOLLOWER);
            if head.sign() == Ordering::Equal {
                self.web.retire(head);
            } else {
                if level > 0 {
                    // keeper: m_latest − m_first, one fold at the
                    // link's own width (its consume read prices it).
                    // Level 0 never defers a first-child link, so its
                    // keeper is never read — skip the fold.
                    self.keeper.add_accum(&head);
                }
                self.memo.set_link(slot, head);
            }
        } else {
            debug_assert!(self.head_level < level, "levels resolve LIFO");
            // This level's first site: suspend the outer head by
            // move — its value (m_s − m_ref(outer)) is immutable now.
            let head = self.web.follower_take(REL_FOLLOWER);
            let keeper = core::mem::replace(&mut self.keeper, self.web.lease());
            self.suspend.push(SuspendedLevel {
                head,
                keeper,
                first_slot: self.first_slot.take(),
                level: self.head_level,
            });
            self.first_slot = Some(slot);
            self.head_level = level;
        }
        // The head restarts at this site's minimum. On the walk side the
        // mirror install runs before the site's raise emission and the
        // ordering is load-bearing (a consume's raise can arm a pending
        // frame, and only an installed follower receives that arm's fold);
        // here no emission follows at all — the site's raise leaves no mark
        // in the web (the site-close arm's argument) — so the install just
        // seats the head for the next range's emissions.
        let zero = self.web.lease();
        self.web.follower_set(REL_FOLLOWER, zero);
    }

    /// Resolve the innermost suspended level.
    ///
    /// Its forest parent's minimum is final — it is the tracked minimum right
    /// now — so the deferred first-child link is one fold away, and the outer
    /// head resumes through the suspended diff.
    fn resolve_inner(&mut self) {
        // chain_span := (min − m_last) + (m_last − m_first) = min − m_first;
        // the keeper dies into it (its buffer is re-armed for the outer level
        // below — nothing is minted per resolve).
        let mut chain_span = self.web.follower_take(REL_FOLLOWER);
        chain_span.add_accum(&self.keeper);
        if chain_span.sign() != Ordering::Equal {
            // link(first) = m_first − m_parent = −chain_span: one clone at the
            // link's own width, priced by its consume read.
            let first_slot = self
                .first_slot
                .expect("a nested level's first site is recorded at its suspension");
            let mut link = self.web.lease();
            link.add_accum(&chain_span);
            link.negate();
            self.memo.set_link(first_slot, link);
        }
        // The outer head resumes: (m_first − m_ref(outer)) + (min − m_first).
        // The fold runs NARROW side INTO wide survivor: chain_span dies at the
        // link's own funded width (zero when the minima are shared), while the
        // suspended diff's content — wide when one wide minimum spans a whole
        // first-child chain — is moved, never re-read, so a nested chain over
        // one wide minimum costs nothing per level.
        let outer = self
            .suspend
            .pop()
            .expect("a deeper head level implies a suspended outer level");
        let mut resumed = outer.head;
        resumed.add_accum(&chain_span);
        self.web.retire(chain_span);
        let dead = core::mem::replace(&mut self.keeper, outer.keeper);
        self.web.retire(dead);
        self.first_slot = outer.first_slot;
        self.head_level = outer.level;
        self.web.follower_set(REL_FOLLOWER, resumed);
    }

    /// Read one payload at the cursor, folding the step into the height side of
    /// the web (and the entry net while it lives).
    fn payload(&mut self, first: bool) -> Signed {
        let code = self.cursor.read_int().expect("canonical skyline bits");
        let (sign, magnitude) = if first {
            (Sign::Positive, code)
        } else {
            unzigzag(code)
        };
        self.web.fold_height(sign, &magnitude);
        if let Some(net) = &mut self.entry_net {
            fold_signed_int(net, sign, &magnitude);
        }
        Signed { sign, magnitude }
    }

    /// A virtual emission at the current height.
    fn emit_here(&mut self) {
        self.seed_relation(None);
        self.web.emit_here();
        self.install_relation();
    }

    /// A virtual emission at `h′ + offset`.
    fn emit_offset(&mut self, offset: &Signed) {
        self.seed_relation(Some(offset));
        self.web.emit_offset(offset);
        self.install_relation();
    }

    /// Before the scan's first arming: seed the recording relation `rel = v −
    /// h(scan entry)` from the dying entry net.
    fn seed_relation(&mut self, offset: Option<&Signed>) {
        if self.web.armed() {
            return;
        }
        let mut relation = self
            .entry_net
            .take()
            .expect("the entry net lives until the first arming");
        if let Some(offset) = offset {
            fold_signed_int(&mut relation, offset.sign, &offset.magnitude);
        }
        self.pending_relation = Some(relation);
    }

    /// After the arming emission: install the seeded relation.
    fn install_relation(&mut self) {
        if let Some(relation) = self.pending_relation.take() {
            self.web.follower_set(REL_FOLLOWER, relation);
        }
    }

    /// Walk an untouched range (`fill(0, e) = e`) at the cursor as one block.
    ///
    /// The skip-scanned summary folds the net movement into the height side and
    /// lands the range's minimum as one virtual emission — the same two
    /// quantities the leaf-by-leaf virtual emissions would have left in the
    /// web.
    fn copy_range(&mut self, first: bool) {
        let mut walk = LeafWalk::new();
        let first_leaf_depth = walk
            .descend(&mut self.cursor)
            .expect("a subtree has at least one leaf");
        if first_leaf_depth < 2 {
            // A tiny range (the first descent's depth routes for free):
            // per-leaf virtual emissions are cheaper than a block summary's
            // fixed cost.
            let _ = self.payload(first);
            self.emit_here();
            while walk.descend(&mut self.cursor).is_some() {
                let _ = self.payload(false);
                self.emit_here();
            }
            return;
        }
        let skip = skip_leaves(&mut walk, &mut self.cursor, first, Some(first_leaf_depth))
            .expect("the descended leaf is pending");
        self.web.fold_height(skip.net.sign, &skip.net.magnitude);
        if let Some(net) = &mut self.entry_net {
            fold_signed_int(net, skip.net.sign, &skip.net.magnitude);
        }
        self.emit_offset(&skip.min_from_exit);
    }

    /// Consume a collapsing range at the cursor: fold its net height movement
    /// into the height side of the web (and the entry net while it lives),
    /// emitting nothing.
    ///
    /// The range's leaves vanish into a raise only the walk decides, and that
    /// raise never falls below the sibling minimum the scan is here to record
    /// (the module doc's raise arms) — so the scan owes the web nothing but
    /// the cursor's height movement across the range.
    fn skip_collapse(&mut self, first: bool) {
        let mut walk = LeafWalk::new();
        let first_leaf_depth = walk
            .descend(&mut self.cursor)
            .expect("a subtree has at least one leaf");
        if first_leaf_depth < 2 {
            // A tiny range (the first descent's depth routes for free):
            // per-leaf height folds are cheaper than a block summary's
            // fixed cost — a single wide leaf folds once, straight into
            // the web, never through a materialized net.
            let _ = self.payload(first);
            while walk.descend(&mut self.cursor).is_some() {
                let _ = self.payload(false);
            }
            return;
        }
        let skip = skip_leaves(&mut walk, &mut self.cursor, first, Some(first_leaf_depth))
            .expect("the descended leaf is pending");
        self.web.fold_height(skip.net.sign, &skip.net.magnitude);
        if let Some(net) = &mut self.entry_net {
            fold_signed_int(net, skip.net.sign, &skip.net.magnitude);
        }
    }

    /// Scan a collapsing range at the cursor for its maximum: `max − h′` at
    /// exit as a nonnegative offset. No virtual emissions — the range's leaves
    /// vanish into the raise the caller decides.
    fn max_range(&mut self, first: bool) -> Signed {
        let mut above = Extremum::max(self.web.lease());
        let mut walk = LeafWalk::new();
        let first_leaf_depth = walk
            .descend(&mut self.cursor)
            .expect("a subtree has at least one leaf");
        if first_leaf_depth < 2 {
            // A tiny range (the first descent's depth routes for free):
            // per-leaf is cheaper than a block summary.
            let step = self.payload(first);
            above.fold(step.sign, &step.magnitude);
            while walk.descend(&mut self.cursor).is_some() {
                let step = self.payload(false);
                above.fold(step.sign, &step.magnitude);
            }
        } else {
            let mut net = Accumulator::new();
            fold_region(
                &mut walk,
                &mut self.cursor,
                first,
                &mut net,
                &mut above,
                Some(first_leaf_depth),
            );
            let (net_sign, net_magnitude) = net.sign_magnitude();
            let net = Signed::from_sign_magnitude(net_sign, net_magnitude);
            self.web.fold_height(net.sign, &net.magnitude);
            if let Some(entry) = &mut self.entry_net {
                fold_signed_int(entry, net.sign, &net.magnitude);
            }
        }
        let result = self.web.materialize(above.into_offset());
        debug_assert!(!result.sign.is_negative(), "the fold floors at zero");
        result
    }
}

/// What the pre-scan still owes a suspended node (the fill walk's frame kinds,
/// minus costs — the pre-scan folds no route).
enum PreFrame {
    /// A left-full site: its sibling walk is in flight; the ledger
    /// record runs at its close.
    Site,
    /// An ordinary node whose left child's range is in flight.
    AwaitLeft,
    /// An ordinary node whose right child's range is in flight.
    AwaitRight,
}

/// The pre-scan's suspended ancestors, held as bits: the fill walk's stack
/// shape with the site payload (its ledger slot) as a pop-able word delta in
/// place of route keys and costs.
struct PreFrames {
    /// Per frame: a left-full site (true) or an ordinary node (false).
    site: BitStack,
    /// Ordinary frames: awaiting the left (false) or right (true) child's
    /// range. Site frames: false, unread.
    phase: BitStack,
    /// Ordinary frames: whether the right child is present. Site frames: false,
    /// unread.
    aux: BitStack,
    /// Per site frame: the ledger slot delta against a monotone register,
    /// LIFO with the frames it serves.
    values: PopStack,
    /// The top site frame's ledger slot (reserves run in stream order, so the
    /// register only advances).
    slots: DeltaReg,
}

impl PreFrames {
    fn new() -> Self {
        PreFrames {
            site: BitStack::new(),
            phase: BitStack::new(),
            aux: BitStack::new(),
            values: PopStack::new(),
            slots: DeltaReg::new(),
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
    fn push_site(&mut self, slot: usize) {
        self.slots.push(&mut self.values, slot);
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

    /// Close a site frame: its ledger slot.
    fn pop_site(&mut self) -> usize {
        self.site.pop();
        self.phase.pop();
        self.aux.pop();
        self.slots.pop(&mut self.values)
    }
}
