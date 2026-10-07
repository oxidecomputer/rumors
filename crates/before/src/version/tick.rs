//! Advances a version for one party.
//!
//! A tick first raises and collapses heights where ownership permits:
//!
//! - A fully owned subtree becomes one leaf at its maximum height.
//! - At a partially owned branch, an owned child becomes one leaf at the
//!   greater of its old maximum and the simplified sibling's minimum.
//! - An unowned region stays unchanged. A version leaf under a partially owned
//!   region also stays unchanged because it is already constant.
//!
//! These steps never lower a height. If they change the version, the change
//! itself records the tick. Otherwise an owned region must be incremented. The
//! simplifying walk also selects that region, preferring fewer new version
//! branches, then a shallower terminal, and choosing right on a tie.
//!
//! Output is lazy. While simplification still matches the input, the walk
//! retains no output copy. At the first difference it copies the matched prefix
//! once and emits the remainder. If simplification never differs, the recorded
//! route drives one splice that raises the selected region.
//!
//! An owned left child's replacement needs the later right sibling's minimum.
//! A pre-scan computes it and every nested lookahead within the same range.
//! The main walk consumes those memoized results as it catches up, and starts
//! another scan only after that range ends. Separate pre-scans thus cover
//! disjoint ranges. Child results wait on compact stacks, keeping traversal
//! linear without one machine-word frame per tree level.
//!
//! One accumulator tracks the current input height. Other arithmetic state is
//! stored as differences from that height, a range minimum, or the preceding
//! output leaf. Balanced accumulators absorb deltas without repeatedly copying
//! a wide absolute height. An integer is materialized when its input or output
//! code accounts for that width, so arithmetic and transient storage remain
//! proportional to the input and required output.

use core::cmp::Ordering;

use num_bigint::{BigInt, BigUint, Sign};
use suanpan::Accumulator;

use crate::accumulator::BigIntAccumulator as _;
use crate::party::io::{PartyNode, PartyReader};

use self::frames::{Frame, Frames};
use self::memo::Memo;
use self::output::Output;
use self::prescan::PreScan;
use self::probe::RaiseProbe;
pub(crate) use self::route::Cost;
use super::range_minima::RangeMinima;
use crate::version::io::regions::{Extremum, PayloadKind, RegionSkip, VersionSubtreeReader};
use crate::version::io::tree::{VersionNode, VersionTreeReader};
use crate::Version;

mod frames;
mod memo;
mod output;
mod prescan;
mod probe;
pub mod raise;
mod route;

/// Slots per memo block, for the co-generation census that counts which
/// pre-scans reserve more than one block.
#[cfg(test)]
pub(crate) use self::memo::BLOCK_SLOTS;

/// Tracks the minimum minus the previous output height after emitting a
/// minimum. A follower is a value that `RangeMinima` updates whenever its
/// reference minimum changes.
const OUT_FOLLOWER: usize = 0;

/// Tracks the live minimum minus the reference used by the next memo entry.
const REL_FOLLOWER: usize = 1;

/// An accumulator retained across traversal steps.
pub enum StoredAccumulator {
    /// A value that fits in one signed machine word.
    Small(i64),
    /// A wider value retained behind one pointer.
    Wide(Box<Accumulator>),
}

const _: () = assert!(core::mem::size_of::<StoredAccumulator>() <= 2 * core::mem::size_of::<u64>());

impl StoredAccumulator {
    /// Retain a narrow value inline, without normalizing a wide accumulator.
    fn new(value: Accumulator) -> Self {
        // At most two held digits make exact conversion constant-width.
        // A wider representation stays boxed even if its digits might cancel.
        if value.stored_digit_count() > 2 {
            return Self::Wide(Box::new(value));
        }
        match i64::try_from(value) {
            Ok(small) => Self::Small(small),
            Err(wide) => Self::Wide(Box::new(wide)),
        }
    }

    /// Restore this retained value for arithmetic.
    fn restore(self) -> Accumulator {
        match self {
            Self::Small(small) => Accumulator::from(small),
            Self::Wide(wide) => *wide,
        }
    }
}

// Both relative values must fit in the minimum tracker's follower array.
const _: () = assert!(
    OUT_FOLLOWER < super::range_minima::FOLLOWER_SLOTS
        && REL_FOLLOWER < super::range_minima::FOLLOWER_SLOTS
);

/// How one tick records its event.
enum Decision {
    /// Simplification itself changed the version and records the event.
    Simplified(Version),
    /// Simplification was the identity, so this route must be raised.
    Raise(self::route::Route),
}

/// A simplifying traversal and the raise route it computes in parallel.
pub struct TickWalk<'a> {
    /// The input version, retained for subtree scans and lazy output copying.
    version: &'a Version,
    /// The next unread node or payload in the version.
    cursor: VersionTreeReader<'a>,
    /// Whether the next payload stores an absolute height rather than a delta.
    next_payload: PayloadKind,
    /// The last consumed input leaf's height.
    height: Accumulator,
    /// The input height minus the previous output while height-anchored.
    gap: Accumulator,
    /// Whether the output reference lives in `OUT_FOLLOWER` instead of `gap`.
    w_anchored: bool,
    /// Whether the last collapsed range contained one leaf.
    range_is_leaf: bool,
    /// Minimum values for currently open version ranges.
    minima: RangeMinima<()>,
    /// Minima computed ahead of the main cursor.
    memo: Memo,
    /// How the memo's reference minimum relates to the live walk.
    memo_reference: MemoReference,
    /// Lazy output and first-difference detector.
    output: Output,
    /// Cheapest raise route, computed while simplification still matches.
    probe: RaiseProbe,
}

impl TickWalk<'_> {
    /// Advance a version once for `party`.
    ///
    /// The walk first tries to simplify the version over owned regions. If
    /// simplification makes no change, it raises the cheapest owned region.
    ///
    /// # Panics
    ///
    /// Panics if `version` is not a canonical Version stream. Every
    /// [`Party`](crate::Party) owns at least one region, because its encoding
    /// has no spelling for an empty party, so the walk always finds a region
    /// to raise.
    pub fn tick(version: &Version, party: &crate::Party) -> Version {
        match Self::decide(version, party) {
            Decision::Simplified(bits) => bits,
            Decision::Raise(route) => {
                let one = BigUint::from(1u8);
                route.apply(version, party, &one)
            }
        }
    }

    /// Advance a version by `n` events for `party`.
    ///
    /// The result is identical to calling [`tick`](Self::tick) `n` times, but
    /// needs at most two simplifying walks and one compound raise. If the first
    /// walk simplifies, it records one event; simplification is idempotent, so
    /// a second walk can apply the remaining events with one raise. `n = 0`
    /// returns the input unchanged.
    ///
    /// # Panics
    ///
    /// Panics if `version` is not a canonical Version stream. Every
    /// [`Party`](crate::Party) owns at least one region, because its encoding
    /// has no spelling for an empty party, so the walk always finds a region
    /// to raise.
    pub fn ticks(version: &Version, party: &crate::Party, n: &BigUint) -> Version {
        // Width distinguishes zero from one without inspecting the digits.
        if n.bits() == 0 {
            return version.clone();
        }
        match Self::decide(version, party) {
            Decision::Simplified(bits) => {
                if n.bits() == 1 {
                    return bits;
                }
                let remaining = n.clone() - &BigUint::from(1u8);
                let simplified = bits;
                match Self::decide(&simplified, party) {
                    Decision::Raise(route) => route.apply(&simplified, party, &remaining),
                    Decision::Simplified(_) => unreachable!("simplification is idempotent"),
                }
            }
            Decision::Raise(route) => route.apply(version, party, n),
        }
    }

    /// Decide whether simplification records the tick or a region must be
    /// raised instead.
    ///
    /// The traversal computes both answers together. Until simplification
    /// differs from the input, [`Output`] avoids constructing output and
    /// [`RaiseProbe`] records the cheapest region that could be raised. The
    /// first difference materializes the simplified output and discards that
    /// route because it will not be used.
    fn decide(version: &Version, party: &crate::Party) -> Decision {
        let mut walk = TickWalk {
            version,
            cursor: VersionTreeReader::new(version),
            next_payload: PayloadKind::Absolute,
            height: Accumulator::new(),
            gap: Accumulator::new(),
            w_anchored: false,
            range_is_leaf: false,
            minima: RangeMinima::new(),
            memo: Memo::new(),
            memo_reference: MemoReference::None,
            output: Output::Unstarted,
            probe: RaiseProbe::new(party.stored_len()),
        };
        let mut party = party.reader();

        walk.minima.open(1);
        walk.walk(&mut party);
        if walk.w_anchored {
            drop(walk.minima.follower_take(OUT_FOLLOWER));
        }
        walk.minima.close();

        debug_assert_eq!(
            walk.pos(),
            version.stored_len(),
            "tick consumes the version"
        );
        debug_assert_eq!(
            walk.memo.cursor,
            walk.memo.len(),
            "tick consumes every memoized minimum"
        );
        #[cfg(debug_assertions)]
        debug_assert_eq!(
            walk.memo.recorded_check, walk.memo.consumed_check,
            "tick consumes memoized minima in order"
        );
        debug_assert!(
            matches!(walk.memo_reference, MemoReference::None),
            "the outer range closes its memo relation"
        );

        match walk.output.finish(version) {
            Some(bits) => Decision::Simplified(bits),
            None => Decision::Raise(walk.probe.take_route()),
        }
    }
}

/// Where the minimum referenced by the next memo entry is retained.
///
/// A pre-scan stores each minimum as a difference from the preceding reference
/// minimum. As the main walk catches up, that reference is either recoverable
/// from the current height or from the minimum tracker. [`Min`](Self::Min) always
/// owns the tracker's [`REL_FOLLOWER`] slot; every transition changes the variant
/// and slot together.
enum MemoReference {
    /// No pre-scanned range is active.
    None,
    /// The current height minus the referenced minimum.
    Height(Accumulator),
    /// The live minimum minus the referenced minimum is in the follower slot.
    Min,
}

impl TickWalk<'_> {
    /// Simplify the complete version under `party` and compute raise costs.
    ///
    /// The iterative walk descends until a subtree is resolved or a branch must
    /// wait for a child, then folds completed children while ascending. Each
    /// open child has a matching minimum range and one compact [`Frames`]
    /// entry. The same postorder folds each child's [`Cost`] into the raise
    /// route. The root cost is unused because only its descendants can be
    /// selected by a parent.
    fn walk(&mut self, party: &mut PartyReader) {
        let mut frames = Frames::new();
        // The current version depth equals the number of suspended branches.
        let mut depth = 0u64;
        'descend: loop {
            debug_assert_eq!(depth, frames.len(), "one frame per open branch level");
            // Descend: resolve the subtree at the cursor to a cost, or suspend
            // its branch node and re-enter on a present child.
            let mut cost: Cost = loop {
                let branch = match party.read() {
                    // A fully owned region simplifies to its maximum. If the
                    // output still matches, that region was already one leaf,
                    // which can be raised without expanding the party tree.
                    PartyNode::Owned => {
                        let above = self.scan_max_consuming();
                        self.emit_offset(depth, above);
                        break Cost::FREE;
                    }
                    PartyNode::Branch(branch) => branch,
                };
                // Route directions are indexed by the start of the party tag
                // that was just consumed.
                let key = party.offset() - 2;
                if self.read_node() == VersionNode::Leaf {
                    // A version leaf is already constant across both party
                    // children. It passes through unchanged while the raise
                    // probe searches the skipped party subtrees.
                    self.consume_payload();
                    self.emit_step(depth);
                    break self.probe.expand(key, party, branch);
                }

                // A party branch over a version branch may collapse a fully
                // owned child, raising it to the simplified sibling minimum.
                // Either way the branch's cost folds both children: an owned
                // terminal needs no expansion, and an absent child is infeasible.
                // The chosen direction is stored at the branch's party key.
                if branch.has_left_child() && matches!(party.peek(), PartyNode::Owned) {
                    // The owned left child becomes one leaf. Its height is the
                    // greater of its old maximum and the simplified right
                    // child's minimum. The maximum is consumed now; the later
                    // minimum comes from an enclosing pre-scan or one new scan
                    // that also records every nested lookahead it encounters.
                    party.skip();
                    let above = self.scan_max_consuming();
                    if !branch.has_right_child() {
                        // An unowned right child remains unchanged. Scan its
                        // minimum for the left leaf, then copy it as its own
                        // child range.
                        let raise = self.scan_min();
                        let value_offset = above.max(raise);
                        self.emit_offset(depth + 1, value_offset);
                        self.minima.open(1);
                        self.copy_subtree(depth + 1);
                        self.minima.close();
                        break self.probe.join(key, Cost::FREE, Cost::INFEASIBLE);
                    }
                    let outermost = self.pos() >= self.memo.covered_until;
                    debug_assert_eq!(
                        outermost,
                        matches!(self.memo_reference, MemoReference::None),
                        "a fresh scan starts exactly where no memo reference is live"
                    );
                    if outermost {
                        // One fresh pre-scan records this minimum and every
                        // nested lookahead inside the same range.
                        self.memo.begin_scan();
                        let scan_start = self.pos();
                        let mut scan = PreScan::new(self.version, scan_start, &mut self.memo);
                        let slot = scan.reserve(scan_start);
                        scan.minima.open(1);
                        let mut reader = party.restart_at(party.offset());
                        let end = scan.run(&mut reader);
                        scan.record(slot, 0);
                        let relation = scan.minima.follower_take(REL_FOLLOWER);
                        drop(relation);
                        scan.minima.close();
                        debug_assert!(scan.all_levels_resolved(), "every suspended level resolves");
                        self.memo.covered_until = end;
                    }
                    self.consume_lookahead(&above, depth);
                    frames.push_lookahead(key, outermost);
                    self.minima.open(1);
                    depth += 1;
                    continue; // walk the right sibling range
                }
                // An ordinary node: the left child first; the party cursor then
                // sits exactly at the right child's tag, so an owned right child
                // is one `O(1)` peek on the way back up — no lookahead over the
                // left party subtree.
                frames.push_node(key, branch.has_right_child());
                self.minima.open(1);
                depth += 1;
                if branch.has_left_child() {
                    continue; // descend into the left child
                }
                // An unowned left child remains unchanged and cannot be raised.
                self.copy_subtree(depth);
                break Cost::INFEASIBLE;
            };
            // Ascend: fold the completed subtree's cost upward until a
            // suspended node still has a child to walk (or the root completes).
            loop {
                let Some(top) = frames.top() else {
                    debug_assert_eq!(depth, 0, "the root subtree completes at depth zero");
                    let _ = cost; // the root cost has no parent to combine it
                    return;
                };
                // The completed child has one open minimum range and one
                // traversal frame. Close its range before resuming the parent;
                // entering a right child will open another range for that frame.
                self.minima.close();
                depth -= 1;
                match top {
                    // The pre-scanned sibling is complete. Close its memo
                    // reference and combine its raise cost with the owned left
                    // leaf's zero cost.
                    Frame::Lookahead => {
                        let (key, outermost) = frames.pop_lookahead();
                        self.pop_lookahead(outermost);
                        cost = self.probe.join(key, Cost::FREE, cost);
                    }
                    // The left child finished. Resolve an owned right terminal,
                    // descend into a right branch, or copy an unowned right side.
                    Frame::AwaitLeft => {
                        let right = frames.aux_top();
                        if right && matches!(party.peek(), PartyNode::Owned) {
                            // The owned right child becomes one leaf. The open
                            // range already tracks the simplified left minimum,
                            // so compare it with the consumed right maximum.
                            party.skip();
                            let above = self.scan_max_consuming();
                            if self.minima.compare_above(&above) == Ordering::Less {
                                self.emit_at_min(depth + 1);
                            } else {
                                self.emit_offset(depth + 1, above);
                            }
                            let key = frames.pop_await_left();
                            cost = self.probe.join(key, cost, Cost::FREE);
                        } else if right {
                            frames.flip_to_await_right(cost);
                            self.minima.open(1);
                            depth += 1;
                            continue 'descend; // walk the right child
                        } else {
                            // An unowned right child remains unchanged and
                            // cannot be raised.
                            self.minima.open(1);
                            self.copy_subtree(depth + 1);
                            self.minima.close();
                            let key = frames.pop_await_left();
                            cost = self.probe.join(key, cost, Cost::INFEASIBLE);
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
    fn pos(&self) -> u64 {
        self.cursor.position()
    }

    /// Read one Version node while interleaving with the Party tree.
    fn read_node(&mut self) -> VersionNode {
        self.cursor.node()
    }

    /// Decode the payload at the cursor as a signed step (the stream's first
    /// payload is its absolute height, a step from zero), folding it into the
    /// height-anchored accumulators, and advancing the cursor.
    fn consume_payload(&mut self) -> BigInt {
        let code = self.cursor.payload();
        let delta = self.next_payload.decode(code);
        self.next_payload = PayloadKind::Delta;
        self.height.add_bigint(&delta);
        self.minima.fold_height(&delta);
        if !self.w_anchored {
            self.gap.add_bigint(&delta);
        }
        if let MemoReference::Height(relation) = &mut self.memo_reference {
            relation.add_bigint(&delta);
        }
        delta
    }

    /// Advance all height-relative state by one scanned block's net delta.
    ///
    /// Exactly what [`consume_payload`](Self::consume_payload) would have
    /// folded leaf by leaf: nothing reads the registers between a block's
    /// leaves, so the batched fold is observationally the per-leaf sequence.
    fn fold_block(&mut self, net: &BigInt) {
        self.height.add_bigint(net);
        self.minima.fold_height(net);
        if !self.w_anchored {
            self.gap.add_bigint(net);
        }
        if let MemoReference::Height(relation) = &mut self.memo_reference {
            relation.add_bigint(net);
        }
    }

    /// Consume the next pre-scanned minimum and emit the owned leaf it governs.
    fn consume_lookahead(&mut self, above: &BigInt, depth: u64) {
        debug_assert!(
            self.memo.cursor < self.memo.len(),
            "a covered lookahead has a recorded entry"
        );
        #[cfg(debug_assertions)]
        {
            self.memo.consumed_check = Memo::check_position(self.memo.consumed_check, self.pos());
        }
        let link = self.memo.take_link(self.memo.cursor);
        let link = link.map(StoredAccumulator::restore);
        self.memo.cursor += 1;
        match core::mem::replace(&mut self.memo_reference, MemoReference::None) {
            MemoReference::None => {
                // The outermost entry is relative to the pre-scan's starting
                // height, which is the current height at this point.
                let relation = Accumulator::new();
                self.consume_h_anchored(relation, link, above, depth);
            }
            MemoReference::Height(relation) => {
                self.consume_h_anchored(relation, link, above, depth)
            }
            MemoReference::Min => {
                // Let `target` be the memoized minimum, `reference` the prior
                // minimum, and `anchor` the minimum tracker's live anchor.
                // The memo link is `target - reference`; the follower is
                // `anchor - reference`. Their difference gives
                // `target - anchor`, the offset needed to install that minimum.
                let mut arm_offset = self.minima.follower_take(REL_FOLLOWER);
                arm_offset = -arm_offset;
                if let Some(link) = link {
                    arm_offset += &link;
                    drop(link);
                }
                if self.minima.compare_above_vs(above, &arm_offset) == Ordering::Less {
                    // The memoized minimum dominates the old maximum.
                    self.minima.arm_relative(arm_offset);
                    self.emit_at_min(depth + 1);
                    let zero = Accumulator::new();
                    self.minima.follower_set(REL_FOLLOWER, zero);
                } else {
                    // Store `anchor - target` before emitting; if emission
                    // moves the anchor, the tracker updates this follower with it.
                    arm_offset = -arm_offset;
                    self.minima.follower_set(REL_FOLLOWER, arm_offset);
                    self.emit_offset(depth + 1, above.clone());
                }
                self.memo_reference = MemoReference::Min;
            }
        }
    }

    /// Consume a memo entry whose reference is retained relative to height.
    fn consume_h_anchored(
        &mut self,
        mut relation: Accumulator,
        link: Option<Accumulator>,
        above: &BigInt,
        depth: u64,
    ) {
        // Let `height` be the current input height, `target` the memoized
        // minimum, and `reference` its prior reference. Then:
        //
        //   relation = height - reference
        //   link     = target - reference
        //
        // `relation + above - link` compares the old maximum with `target`.
        // Keeping the link folded in leaves `height - target`, ready to serve
        // as the next reference without materializing either absolute value.
        relation.add_bigint(above);
        if let Some(link) = link {
            relation -= &link;
            drop(link);
        }
        let sign = relation.cmp_zero();
        relation.sub_bigint(above);
        if sign == Ordering::Less {
            // The memoized minimum dominates the old maximum, so the new leaf
            // necessarily differs from the consumed range.
            let first = self.output.is_unstarted();
            self.diverge();
            // `relation` is now current height minus the memoized minimum.
            if first {
                // First output leaf, coded absolute: value = h − below.
                let mut absolute = Accumulator::new();
                absolute += &self.height;
                absolute -= &relation;
                self.minima.emit_below_accum(relation);
                let value = absolute.into_bigint();
                debug_assert!(value.sign() != Sign::Minus, "a raised height is a natural");
                self.output.height(depth + 1, value.magnitude());
                // Future output deltas are now relative to this minimum.
                let zero = Accumulator::new();
                self.minima.follower_set(OUT_FOLLOWER, zero);
                self.w_anchored = true;
                self.gap.reset();
            } else {
                self.minima.emit_below_accum(relation);
                self.emit_at_min(depth + 1);
            }
            let zero = Accumulator::new();
            self.minima.follower_set(REL_FOLLOWER, zero);
            self.memo_reference = MemoReference::Min;
        } else {
            self.emit_offset(depth + 1, above.clone());
            self.memo_reference = MemoReference::Height(relation);
        }
    }

    /// Close a pre-scanned range and prepare its minimum as the next reference.
    ///
    /// At this point the open range contains exactly the simplified
    /// sibling and its raised owned leaf, so its tracked minimum is the value
    /// from the memo. An enclosing range can therefore refer to that minimum
    /// directly, without reconstructing an absolute height.
    fn pop_lookahead(&mut self, outermost: bool) {
        match core::mem::replace(&mut self.memo_reference, MemoReference::None) {
            MemoReference::None => unreachable!("a consumed lookahead keeps a reference"),
            MemoReference::Height(relation) => drop(relation),
            MemoReference::Min => {
                let relation = self.minima.follower_take(REL_FOLLOWER);
                drop(relation);
            }
        }
        if !outermost {
            // The new zero relation is against the true minimum, so make the
            // anchor exact before storing it.
            self.minima.resolve_deferred();
            let zero = Accumulator::new();
            self.minima.follower_set(REL_FOLLOWER, zero);
            self.memo_reference = MemoReference::Min;
        }
    }

    /// Materialize simplified output at its first difference from the input.
    ///
    /// The raise route becomes irrelevant once simplification records the tick.
    fn diverge(&mut self) {
        if self.output.is_verbatim() {
            self.probe.kill();
            self.output.materialize(self.version);
        }
    }

    /// Emit a pass-through leaf at the current input height.
    ///
    /// Before the first difference, its unchanged depth and value extend the
    /// matched prefix without writing output. Afterward, its delta is derived
    /// from the live output reference.
    fn emit_step(&mut self, depth: u64) {
        self.minima.emit_here();
        if self.output.note_match(self.pos()) {
            self.gap.reset();
            return;
        }
        let delta = if self.w_anchored {
            // Convert the minimum-relative output reference back to the
            // current input height.
            let mut out_delta = self.minima.follower_take(OUT_FOLLOWER);
            self.minima.bridge_add_gap(&mut out_delta);
            self.w_anchored = false;
            out_delta.into_bigint()
        } else {
            // `gap` is current input height minus previous output height.
            self.gap.cmp_zero();
            self.gap.to_bigint()
        };
        // The new gap is h − value = 0 exactly.
        self.gap.reset();
        self.output.change(depth, &delta);
    }

    /// Emit a collapsed leaf at current input height plus `offset`.
    ///
    /// It matches the input only when the consumed range was already one leaf
    /// and the offset is zero. Any topology change or value change materializes
    /// the simplified output.
    fn emit_offset(&mut self, depth: u64, offset: BigInt) {
        self.minima.emit_offset(&offset);
        if self.output.is_verbatim() && self.range_is_leaf && offset.sign() == Sign::NoSign {
            let matched = self.output.note_match(self.pos());
            debug_assert!(matched, "a verbatim walk records a value-reproducing raise");
            self.gap.reset();
            return;
        }
        // The first-leaf question is asked before the divergence erases it
        // (`diverge` is a no-op on an already-built output).
        let first = self.output.is_unstarted();
        self.diverge();
        if first {
            // The first output payload stores an absolute height.
            debug_assert!(!self.w_anchored, "the first emission finds no anchor");
            self.height.cmp_zero();
            let value = self.height.to_bigint() + &offset;
            debug_assert!(
                value.sign() != Sign::Minus,
                "a collapsed height is a natural"
            );
            self.output.height(depth, value.magnitude());
        } else {
            let delta = if self.w_anchored {
                // Convert the minimum-relative reference, then apply `offset`.
                let mut out_delta = self.minima.follower_take(OUT_FOLLOWER);
                self.minima.bridge_add_gap(&mut out_delta);
                out_delta.add_bigint(&offset);
                self.w_anchored = false;
                out_delta.into_bigint()
            } else {
                // `gap + offset` is the new value minus previous output.
                self.gap.add_bigint(&offset);
                self.gap.cmp_zero();
                self.gap.to_bigint()
            };
            self.output.change(depth, &delta);
        }
        // The new gap is h − (h + offset) = −offset exactly.
        self.gap.reset();
        self.gap.sub_bigint(&offset);
    }

    /// Emit a leaf at the innermost tracked minimum.
    ///
    /// The emission leaves the tracked minimum unchanged and makes subsequent
    /// output deltas relative to it. It always diverges from a verbatim walk:
    /// this path is chosen only when the minimum is above the consumed range's
    /// maximum, hence above every plateau the new leaf replaces.
    fn emit_at_min(&mut self, depth: u64) {
        debug_assert!(
            !self.output.is_unstarted(),
            "a tracked minimum implies an emission"
        );
        self.diverge();
        // Emitting the true minimum makes the anchor exact, so the new zero
        // follower is relative to that minimum.
        self.minima.resolve_deferred();
        let delta = if self.w_anchored {
            // d_out = min - prev_out is already stored in the follower.
            let out_delta = self.minima.follower_take(OUT_FOLLOWER);
            out_delta.into_bigint()
        } else {
            // min - prev_out = (h - prev_out) - (h - min).
            let fresh = Accumulator::new();
            let mut out_delta = core::mem::replace(&mut self.gap, fresh);
            self.minima.bridge_sub_gap(&mut out_delta);
            out_delta.into_bigint()
        };
        // prev_out = min now: the follower restarts at zero.
        let zero = Accumulator::new();
        self.minima.follower_set(OUT_FOLLOWER, zero);
        self.w_anchored = true;
        self.gap.reset();
        self.output.change(depth, &delta);
    }

    /// Copy the version subtree at the cursor unchanged.
    ///
    /// The party owns nothing in this region. Before the first output
    /// difference, one block scan updates the height and minimum summaries and
    /// extends the matched prefix. After a difference, the first leaf passes
    /// through the normal emission path and the remaining canonical range can
    /// be spliced unchanged.
    fn copy_subtree(&mut self, depth: u64) {
        // Three regimes: a sufficiently large matched region is summarized in
        // one scan; a sufficiently large built region emits its first leaf and
        // splices the rest; a tiny region proceeds leaf by leaf.
        //
        // The first descent already reveals which regime is cheaper. For a
        // lone leaf or shallow pair, the block summary costs more than simply
        // visiting each leaf.
        let mut walk = VersionSubtreeReader::new();
        let first_leaf_depth = walk
            .descend(&mut self.cursor)
            .expect("a subtree has at least one leaf");
        if first_leaf_depth >= 2 && self.output.is_verbatim() {
            debug_assert!(!self.w_anchored, "a verbatim walk is height-anchored");
            let skip = walk
                .summarize_remaining(&mut self.cursor, self.next_payload, Some(first_leaf_depth))
                .expect("the descended leaf is pending");
            self.next_payload = PayloadKind::Delta;
            self.fold_block(&skip.net);
            self.minima.emit_offset(&skip.min_from_exit);
            // The matched region ends at its last input leaf.
            self.gap.reset();
            let matched = self.output.note_match(self.pos());
            debug_assert!(matched, "a verbatim walk records the region as matched");
            return;
        }
        // Emit the first leaf against the live output reference.
        self.consume_payload();
        self.emit_step(depth + first_leaf_depth);
        if first_leaf_depth >= 2 {
            // Every remaining delta has both endpoints inside this unchanged
            // subtree, so the remainder can be spliced verbatim.
            let rest_start = self.pos();
            let skip = walk
                .summarize_remaining(&mut self.cursor, PayloadKind::Delta, None)
                .expect("a region whose first leaf sits below its root has more leaves");
            self.fold_block(&skip.net);
            self.minima.emit_offset(&skip.min_from_exit);
            // The region's last leaf is the last emission.
            self.gap.reset();
            self.output.copy_subtree_remainder(
                self.version,
                rest_start,
                self.pos(),
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

    /// Consume the version subtree at the cursor, returning its maximum as a
    /// nonnegative offset above the exit height.
    ///
    /// Folds the streaming maximum of the subtree's leaf heights: `max − h`,
    /// maintained by subtracting each step and resetting to zero whenever the
    /// running height overtakes it (`h` then sits at the subtree's last leaf).
    /// The offset's width is bounded by the scanned range's own content, which
    /// prices every later fold of it.
    fn scan_max_consuming(&mut self) -> BigInt {
        // Replacing this range with one leaf preserves its topology only if it
        // was already a leaf. Remember that fact before consuming the range.
        self.range_is_leaf = matches!(
            VersionTreeReader::at(self.version, self.pos()).node(),
            VersionNode::Leaf
        );
        let mut above = Extremum::max(Accumulator::new());
        let mut walk = VersionSubtreeReader::new();
        let first_leaf_depth = walk
            .descend(&mut self.cursor)
            .expect("a subtree has at least one leaf");
        if first_leaf_depth < 2 {
            // A tiny range is cheaper to fold leaf by leaf than to summarize.
            let step = self.consume_payload();
            above.fold(&step);
            while walk.descend(&mut self.cursor).is_some() {
                let step = self.consume_payload();
                above.fold(&step);
            }
        } else {
            let mut net = Accumulator::new();
            walk.fold_remaining(
                &mut self.cursor,
                self.next_payload,
                &mut net,
                &mut above,
                Some(first_leaf_depth),
            );
            self.next_payload = PayloadKind::Delta;
            let net = net.to_bigint();
            self.fold_block(&net);
        }
        let result = above.into_offset().into_bigint();
        debug_assert!(result.sign() != Sign::Minus, "the fold floors at zero");
        result
    }

    /// Scan the next subtree's minimum without moving this walk.
    fn scan_min(&self) -> BigInt {
        let mut cursor = VersionTreeReader::at(self.version, self.pos());
        let skip = RegionSkip::read_subtree(&mut cursor);
        // `min = h_entry + net + (min - h_exit)`.
        skip.net + skip.min_from_exit
    }
}

#[cfg(test)]
mod tests;
