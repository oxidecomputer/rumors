//! Computes minima that lie ahead of the main tick cursor.
//!
//! Simplifying an owned left child may require the simplified minimum of its
//! right sibling. That sibling occurs later in the version stream, so the main
//! cursor cannot know the value when it must emit the left result. [`PreScan`]
//! walks ahead without moving the main cursor and stores the needed minimum in
//! [`Memo`]. The same pass records nested lookaheads, ensuring that overlapping
//! ranges are not scanned repeatedly.
//!
//! The pre-scan computes only minima, without building the simplified version.
//! It reports values to [`RangeMinima`] as if it were emitting leaves. An owned
//! left child can be omitted from those reports: simplification raises it to
//! at least its right sibling's minimum, so it cannot lower the branch's
//! minimum. An owned right child is handled after the left has been scanned:
//! report its old maximum if that already reaches the left minimum, or omit
//! the replacement leaf when it would equal that known minimum. Unowned
//! regions retain their input heights.
//!
//! Memo entries are reserved when lookaheads are encountered, in the order the
//! main walk will read them, but their minima become known when ranges close.
//! The lookahead ranges are nested or disjoint. Within each nesting level, the
//! first minimum is stored relative to its enclosing lookahead's minimum and
//! later minima relative to the preceding one. The outermost entry is relative
//! to the input height at scan entry. These are precisely the references the
//! main walk has available when it consumes each entry.
//!
//! Recording a first nested minimum therefore has to wait for its enclosing
//! range to finish. [`Levels`] suspends the outer recording state during that
//! wait. The active state retains two differences: the tracked minimum minus
//! the latest recorded minimum, and the latest minus the first at this level.
//! Their sum recovers the enclosing minimum minus that first minimum when the
//! enclosing range closes. This avoids storing a wide absolute height at every
//! nesting level. Each sibling difference is folded once, and each deferred
//! first difference is stored once; arithmetic is charged to those differences
//! rather than repeatedly to a shared absolute height.

use core::cmp::Ordering;

use suanpan::Accumulator;

use crate::accumulator::BigIntAccumulator as _;
use crate::party::io::{PartyNode, PartyReader};

use super::super::range_minima::RangeMinima;
use super::memo::Memo;
use super::{StoredAccumulator, REL_FOLLOWER};
use crate::version::io::regions::{Extremum, PayloadKind, VersionSubtreeReader};
use crate::version::io::tree::{VersionNode, VersionTreeReader};
use crate::Version;
use num_bigint::{BigInt, Sign};

mod frames;
mod suspended;

use frames::{Frame, Frames};
use suspended::{Level, Levels};

/// Cursor and relative-minimum state for one lookahead pass.
pub struct PreScan<'a, 'm> {
    /// A forward cursor independent of the main tick cursor.
    cursor: VersionTreeReader<'a>,
    /// Minima of the simplified ranges simulated by this scan.
    pub minima: RangeMinima<()>,
    /// Current input height minus scan-entry height, retained until the first
    /// reported value establishes a minimum.
    entry_net: Option<Accumulator>,
    /// First reported value minus scan-entry height, awaiting installation as
    /// the minimum tracker's relative reference.
    pending_relation: Option<Accumulator>,
    /// The completed minima being recorded for the main walk.
    memo: &'m mut Memo,
    /// Latest recorded minimum minus the first minimum at the active level.
    ///
    /// Each subsequent record adds its difference from the preceding minimum.
    /// This sum resolves the deferred first entry when its enclosing range
    /// closes. The outermost level has no deferred entry and leaves it unused.
    latest_from_first: Accumulator,
    /// The active level's first memo slot, whose value is deferred.
    ///
    /// It will store `first_minimum - enclosing_minimum` once the enclosing
    /// range closes. The outermost level uses the known scan-entry height as
    /// its reference, so it has no deferred slot.
    first_slot: Option<usize>,
    /// Lookahead nesting level of the active recording reference.
    reference_level: u64,
    /// Outer recording levels parked while an inner level is active.
    ///
    /// The stack separates tags from values and bit-packs its integer fields,
    /// so narrow values do not pay for two enum tags and two machine-word
    /// positions at every nesting level.
    suspend: Levels,
}

impl<'a, 'm> PreScan<'a, 'm> {
    /// Start a scan at `start`, using that position's preceding leaf height as
    /// the initial reference for memo differences.
    pub fn new(version: &'a Version, start: u64, memo: &'m mut Memo) -> Self {
        PreScan {
            cursor: VersionTreeReader::at(version, start),
            minima: RangeMinima::new(),
            entry_net: Some(Accumulator::new()),
            pending_relation: None,
            memo,
            latest_from_first: Accumulator::new(),
            first_slot: None,
            reference_level: 0,
            suspend: Levels::new(),
        }
    }

    /// Simulate simplification over the subtree at the cursor and return its
    /// ending bit position.
    ///
    /// Like the main walk, this is iterative: descent either resolves a range
    /// or suspends its branch in [`Frames`]; ascent resumes branches after a
    /// child closes. `level` counts nested lookaheads, not tree depth. The
    /// pre-scan starts at a partially owned right sibling after the left
    /// subtree has been consumed. It therefore begins after the version's
    /// first payload, and every payload it reads is a signed delta.
    pub fn run(&mut self, party: &mut PartyReader) -> u64 {
        let mut frames = Frames::new();
        let mut level: u64 = 1;
        'descend: loop {
            // Descend: resolve the range at the cursor, or suspend and re-enter
            // on a present child.
            loop {
                let branch = match party.read() {
                    // A fully owned region simplifies to its maximum. Valid
                    // scan entries are partially owned, and descent handles
                    // owned children at their parent, so this arm is not
                    // reached by the main tick walk.
                    PartyNode::Owned => {
                        let above = self.max_range();
                        self.emit_offset(&above);
                        break;
                    }
                    PartyNode::Branch(branch) => branch,
                };
                if self.cursor.node() == VersionNode::Leaf {
                    // A version leaf remains constant across the party branch.
                    self.payload();
                    self.emit_here();
                    if branch.has_left_child() {
                        party.skip();
                    }
                    if branch.has_right_child() {
                        party.skip();
                    }
                    break;
                }
                if branch.has_left_child() && matches!(party.peek(), PartyNode::Owned) {
                    // This owned left child needs the later right minimum. The
                    // main walk decides the resulting leaf height; this pass
                    // consumes the left range only to maintain relative height.
                    party.skip();
                    self.skip_collapse();
                    if !branch.has_right_child() {
                        // With no owned right child, that range remains
                        // unchanged and needs no memo entry.
                        self.minima.open(1);
                        self.copy_range();
                        self.minima.close();
                        break;
                    }
                    let slot = self.reserve(self.cursor.position());
                    frames.push_lookahead(slot);
                    level += 1;
                    self.minima.open(1);
                    continue; // walk the sibling range
                }
                // An ordinary node: the left child's range first.
                frames.push_node(branch.has_right_child());
                self.minima.open(1);
                if branch.has_left_child() {
                    continue; // descend into the left child
                }
                // An unowned left child remains unchanged.
                self.copy_range();
                break;
            }
            // Ascend: resume suspended nodes as their children complete.
            loop {
                let Some(top) = frames.top() else {
                    return self.cursor.position();
                };
                self.minima.close();
                match top {
                    // The later sibling is complete, so its simplified minimum
                    // can now be recorded. The owned leaf created beside it
                    // cannot lower that minimum and need not be simulated.
                    Frame::Lookahead => {
                        let slot = frames.pop_lookahead();
                        level -= 1;
                        self.record(slot, level);
                    }
                    // The left range is complete. Resolve a fully owned right
                    // child now, descend into a partial one, or copy an unowned
                    // one unchanged.
                    Frame::AwaitLeft => {
                        let right = frames.right_present();
                        if right && matches!(party.peek(), PartyNode::Owned) {
                            // A fully owned right child becomes its maximum,
                            // unless the left minimum raises it still higher.
                            party.skip();
                            let above = self.max_range();
                            if self.minima.compare_above(&above) != Ordering::Less {
                                self.emit_offset(&above);
                            }
                            frames.pop_node();
                        } else if right {
                            frames.await_right();
                            self.minima.open(1);
                            continue 'descend; // walk the right child
                        } else {
                            // An unowned right child remains unchanged.
                            self.minima.open(1);
                            self.copy_range();
                            self.minima.close();
                            frames.pop_node();
                        }
                    }
                    // A node's right range finished: the node is done.
                    Frame::AwaitRight => {
                        frames.pop_node();
                    }
                }
            }
        }
    }

    /// Whether every nested recording level closed before the scan ended.
    pub fn all_levels_resolved(&self) -> bool {
        self.suspend.is_empty()
    }

    /// Reserve a memo slot for the lookahead starting at `pos`, in the order
    /// the main walk will encounter it.
    pub fn reserve(&mut self, pos: u64) -> usize {
        let slot = self.memo.reserve();
        #[cfg(debug_assertions)]
        {
            self.memo.recorded_check = Memo::check_position(self.memo.recorded_check, pos);
        }
        #[cfg(not(debug_assertions))]
        let _ = pos;
        slot
    }

    /// Record a completed lookahead's minimum relative to the reference its
    /// consumer will hold.
    ///
    /// The enclosing branch's minimum now equals the completed right range's
    /// minimum: the omitted owned left child cannot lower it. The relative
    /// reference in `REL_FOLLOWER` therefore gives the completed minimum minus
    /// the preceding reference. A later lookahead at the same level stores
    /// that difference immediately. The first at a deeper level suspends the
    /// outer reference until the enclosing lookahead's minimum is final.
    pub fn record(&mut self, slot: usize, level: u64) {
        // Memo entries store differences between true minima. Resolve the
        // temporary reference before retaining such a difference.
        self.minima.resolve_deferred();
        debug_assert!(
            !self.minima.deferred_live(),
            "memo entries and suspended levels retain only differences between final minima"
        );
        // Completing an enclosing lookahead makes the first-entry references
        // of any still-active nested levels final.
        while self.reference_level > level {
            self.resolve_inner();
        }
        if self.reference_level == level {
            // The follower already holds current_minimum - previous_minimum.
            // For the outermost entry, the previous reference is scan-entry
            // height instead.
            let mut difference = self.minima.follower_take(REL_FOLLOWER);
            if difference.cmp_zero() == Ordering::Equal {
                drop(difference);
            } else {
                if level > 0 {
                    // Add this difference to latest_minimum - first_minimum.
                    // Work follows the new difference's width, even if the sum
                    // is wide. Level zero has no deferred first entry.
                    self.latest_from_first += &difference;
                }
                self.memo.set_link(slot, StoredAccumulator::new(difference));
            }
        } else {
            debug_assert!(self.reference_level < level, "levels resolve LIFO");
            // The first minimum at this level is known before its enclosing
            // minimum. Move first_minimum - outer_reference into suspended
            // storage and begin a new sequence at this level.
            let first_from_outer = self.minima.follower_take(REL_FOLLOWER);
            let latest_from_first =
                core::mem::replace(&mut self.latest_from_first, Accumulator::new());
            self.suspend.push(Level {
                first_from_outer: StoredAccumulator::new(first_from_outer),
                latest_from_first: StoredAccumulator::new(latest_from_first),
                first_slot: self.first_slot.take(),
                level: self.reference_level,
            });
            self.first_slot = Some(slot);
            self.reference_level = level;
        }
        // The current minimum becomes the reference for subsequent records.
        // No owned-left value is reported here: it cannot lower this minimum.
        let zero = Accumulator::new();
        self.minima.follower_set(REL_FOLLOWER, zero);
    }

    /// Resolve the innermost suspended level.
    ///
    /// The tracked minimum now belongs to the enclosing lookahead. Use it to
    /// finish this level's first memo entry and restore the outer reference.
    fn resolve_inner(&mut self) {
        // (enclosing_minimum - latest_minimum)
        //   + (latest_minimum - first_minimum)
        //   = enclosing_minimum - first_minimum.
        let mut chain_span = self.minima.follower_take(REL_FOLLOWER);
        chain_span += &self.latest_from_first;
        if chain_span.cmp_zero() != Ordering::Equal {
            // The first memo entry needs the reverse difference. It is copied
            // once at the width of the value that will be stored and consumed.
            let first_slot = self
                .first_slot
                .expect("a nested level records its first lookahead when suspended");
            let mut link = Accumulator::new();
            link += &chain_span;
            link = -link;
            self.memo.set_link(first_slot, StoredAccumulator::new(link));
        }
        // Restore the outer relation:
        // (first_minimum - outer_reference)
        //   + (enclosing_minimum - first_minimum).
        // Moving the suspended accumulator preserves its buffer; only the
        // newly resolved difference is read by the addition.
        let outer = self
            .suspend
            .pop()
            .expect("a deeper reference level implies a suspended outer level");
        let mut resumed = outer.first_from_outer.restore();
        resumed += &chain_span;
        drop(chain_span);
        let latest_from_first = outer.latest_from_first.restore();
        let dead = core::mem::replace(&mut self.latest_from_first, latest_from_first);
        drop(dead);
        self.first_slot = outer.first_slot;
        self.reference_level = outer.level;
        self.minima.follower_set(REL_FOLLOWER, resumed);
    }

    /// Read one leaf delta and advance the minimum tracker's input height.
    ///
    /// Decodes unconditionally as a zigzag-coded leaf-to-leaf delta: the
    /// stream's absolute first payload — the one coded as a height — is
    /// behind every scan's entry ([`run`](Self::run)'s doc), so the scan
    /// never reads it.
    fn payload(&mut self) -> BigInt {
        let code = self.cursor.payload();
        let delta = PayloadKind::Delta.decode(code);
        self.minima.fold_height(&delta);
        if let Some(net) = &mut self.entry_net {
            net.add_bigint(&delta);
        }
        delta
    }

    /// Report a simplified leaf at the current input height, without output.
    fn emit_here(&mut self) {
        self.seed_relation(None);
        self.minima.emit_here();
        self.install_relation();
    }

    /// Report a simplified value at current input height plus `offset`.
    fn emit_offset(&mut self, offset: &BigInt) {
        self.seed_relation(Some(offset));
        self.minima.emit_offset(offset);
        self.install_relation();
    }

    /// Prepare the first reported value's difference from scan-entry height.
    /// Later emissions update that reference through the minimum tracker.
    fn seed_relation(&mut self, offset: Option<&BigInt>) {
        if self.minima.armed() {
            return;
        }
        let mut relation = self
            .entry_net
            .take()
            .expect("the entry net is retained until the first minimum");
        if let Some(offset) = offset {
            relation.add_bigint(offset);
        }
        self.pending_relation = Some(relation);
    }

    /// Install the initial relative reference after the first minimum exists.
    fn install_relation(&mut self) {
        if let Some(relation) = self.pending_relation.take() {
            self.minima.follower_set(REL_FOLLOWER, relation);
        }
    }

    /// Report an unowned range's unchanged heights to the minimum tracker.
    ///
    /// A block summary advances the input height to the range's final leaf and
    /// reports its minimum once. This has the same effect on open minima as
    /// reporting every leaf separately.
    fn copy_range(&mut self) {
        let mut walk = VersionSubtreeReader::new();
        let first_leaf_depth = walk
            .descend(&mut self.cursor)
            .expect("a subtree has at least one leaf");
        if first_leaf_depth < 2 {
            // A shallow first descent favors reporting leaves directly over
            // paying the fixed cost of a separate block summary.
            let _ = self.payload();
            self.emit_here();
            while walk.descend(&mut self.cursor).is_some() {
                let _ = self.payload();
                self.emit_here();
            }
            return;
        }
        // `first: false`: the scan reads only deltas (`payload`'s doc).
        let skip = walk
            .summarize_remaining(&mut self.cursor, PayloadKind::Delta, Some(first_leaf_depth))
            .expect("the descended leaf is pending");
        self.minima.fold_height(&skip.net);
        if let Some(net) = &mut self.entry_net {
            net.add_bigint(&skip.net);
        }
        self.emit_offset(&skip.min_from_exit);
    }

    /// Advance over an owned left subtree without reporting its heights.
    ///
    /// Simplification replaces this subtree with one leaf at least as high as
    /// the simplified right minimum. Only the right range can determine the
    /// branch minimum, so the left contributes input height movement alone.
    fn skip_collapse(&mut self) {
        let mut walk = VersionSubtreeReader::new();
        let first_leaf_depth = walk
            .descend(&mut self.cursor)
            .expect("a subtree has at least one leaf");
        if first_leaf_depth < 2 {
            // Read short descents directly. In particular, one wide leaf folds
            // its delta once, without first copying it into a block summary.
            let _ = self.payload();
            while walk.descend(&mut self.cursor).is_some() {
                let _ = self.payload();
            }
            return;
        }
        let net = walk.net_remaining(&mut self.cursor);
        if let Some(entry) = &mut self.entry_net {
            *entry += &net;
        }
        self.minima.fold_accumulator(net);
    }

    /// Consume an owned right subtree and return its maximum minus its final
    /// input height, a nonnegative difference.
    ///
    /// The caller compares this maximum with the completed left minimum before
    /// reporting a value. Completing that left range necessarily reported at
    /// least one value: even skipping an owned left subtree is followed by
    /// scanning its right sibling. The initial entry-relative accumulator has
    /// therefore already transferred into the minimum tracker's reference.
    fn max_range(&mut self) -> BigInt {
        debug_assert!(
            self.entry_net.is_none(),
            "a completed range emits before any raise scans for its maximum, so the entry net is already retired"
        );
        let mut above = Extremum::max(Accumulator::new());
        let mut walk = VersionSubtreeReader::new();
        let first_leaf_depth = walk
            .descend(&mut self.cursor)
            .expect("a subtree has at least one leaf");
        if first_leaf_depth < 2 {
            // Short descents are cheaper to fold directly.
            let step = self.payload();
            above.fold(&step);
            while walk.descend(&mut self.cursor).is_some() {
                let step = self.payload();
                above.fold(&step);
            }
        } else {
            let mut net = Accumulator::new();
            walk.fold_remaining(
                &mut self.cursor,
                // `first: false`: the scan reads only deltas (`payload`'s doc).
                PayloadKind::Delta,
                &mut net,
                &mut above,
                Some(first_leaf_depth),
            );
            self.minima.fold_accumulator(net);
        }
        let result = above.into_offset().into_bigint();
        debug_assert!(result.sign() != Sign::Minus, "the fold floors at zero");
        result
    }
}
