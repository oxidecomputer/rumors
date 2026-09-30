//! One shared reference for the running height and client-owned differences.
//!
//! Let `h` be the running height, `m` the innermost minimum, and `A` the
//! anchor. We store `gap = h - A` and each follower as `A - X`, where `X` is
//! a fixed client value. Normally `A = m`.
//!
//! Closing a range may expose a lower minimum. Moving every stored value at
//! each close could repeatedly traverse a wide number. Instead, keep `A`
//! fixed and accumulate `deferred = A - m > 0`. Thus the true height gap is
//! `gap + deferred`, and the true follower is `follower - deferred`.
//!
//! All followers share the anchor: attaching, detaching, and moving them never
//! changes that rule. The presence of `deferred` therefore describes every
//! active follower's reference. Resolution shifts the gap and all followers
//! together, consuming the deferred distance.

use core::cmp::Ordering;

use num_bigint::BigInt;
use suanpan::Accumulator;

use crate::accumulator::BigIntAccumulator as _;

use super::boundary::Boundary;

/// Number of independently retained client differences.
pub const FOLLOWER_SLOTS: usize = 2;

/// Relative values that must change reference together when the anchor moves.
pub(super) struct Anchor {
    /// Running height minus the anchor: `h - A`.
    gap: Accumulator,
    /// Positive distance `A - m`; absent exactly when the anchor is the minimum.
    deferred: Option<Accumulator>,
    /// Client values `A - X`, all relative to this same anchor.
    followers: [Option<Accumulator>; FOLLOWER_SLOTS],
}

/// Move relative values without reconstructing an absolute height or minimum.
impl Anchor {
    /// Construct the empty state; the first arming establishes its anchor.
    pub(super) fn new() -> Self {
        Self {
            gap: Accumulator::new(),
            deferred: None,
            followers: [None, None],
        }
    }

    /// Establish the first minimum at the supplied distance below the height.
    pub(super) fn initialize(&mut self, gap: Accumulator) {
        debug_assert!(self.followers.iter().all(Option::is_none));
        debug_assert!(self.deferred.is_none());
        drop(core::mem::replace(&mut self.gap, gap));
    }

    /// Forget the final minimum after clients have detached their followers.
    pub(super) fn clear(&mut self) {
        debug_assert!(
            self.followers.iter().all(Option::is_none),
            "followers are removed before the last range closes"
        );
        drop(self.deferred.take());
        drop(core::mem::take(&mut self.gap));
    }

    /// Advance the running height, reading only the supplied difference.
    pub(super) fn fold_height(&mut self, delta: &BigInt) {
        self.gap.add_bigint(delta);
    }

    /// Consume a running-height change without normalizing or copying it.
    pub(super) fn fold_accumulator(&mut self, delta: Accumulator) {
        self.gap += delta;
    }

    /// Undo a temporary height offset after a comparison.
    pub(super) fn subtract_offset(&mut self, offset: &BigInt) {
        self.gap.sub_bigint(offset);
    }

    /// Move the anchor to `h`, returning its signed distance above the old minimum.
    pub(super) fn arm_at_height(&mut self) -> Accumulator {
        let offset = core::mem::take(&mut self.gap);
        self.move_by(offset)
    }

    /// Move the anchor to `v = h - below`, returning `v - old_minimum`.
    pub(super) fn arm_below(&mut self, below: Accumulator) -> Accumulator {
        // (h - A) - (h - v) = v - A. The new gap keeps the supplied buffer.
        let mut offset = core::mem::replace(&mut self.gap, below);
        offset -= &self.gap;
        self.move_by(offset)
    }

    /// Move the anchor by an owned difference, returning `v - old_minimum`.
    pub(super) fn arm_relative(&mut self, offset: Accumulator) -> Accumulator {
        self.gap -= &offset;
        self.move_by(offset)
    }

    /// Finish an anchor move after the gap already refers to the new value `v`.
    ///
    /// First add `v - A` to followers, changing `A - X` to `v - X`.
    /// Then add the deferred `A - m` to obtain `v - m` for boundary updates.
    /// The merge consumes the narrower value and retains the wider buffer.
    fn move_by(&mut self, mut offset: Accumulator) -> Accumulator {
        for follower in self.followers.iter_mut().flatten() {
            *follower += &offset;
        }
        if let Some(deferred) = self.deferred.take() {
            offset += deferred;
        }
        offset
    }

    /// Compare the running height with the true minimum, resolving only if needed.
    ///
    /// With no deferred distance, the sign of `h - A` is the answer. Otherwise
    /// `A > m`, so any height at or above `A` is above `m`. A height below `A`
    /// requires comparing the distances `A - h` and `A - m`.
    pub(super) fn compare_height(&mut self) -> Ordering {
        let sign = self.gap.cmp_zero();
        if self.deferred.is_none() {
            return sign;
        }
        if sign != Ordering::Less {
            return Ordering::Greater;
        }
        self.compare_below_anchor()
    }

    /// Order two positive distances by leading digits before attempting a fold.
    ///
    /// Here `h < A`. A dominating deferred distance places `h` above `m`; a
    /// dominating negative gap places it below. If neither certificate holds,
    /// their widths are comparable: consume the deferred state once and read
    /// the sign of the resulting `h - m`.
    fn compare_below_anchor(&mut self) -> Ordering {
        let gap_bits = self.gap.stored_bits();
        let deferred = self
            .deferred
            .as_mut()
            .expect("the minimum is below the anchor");
        // Sign normalization removes cancelling leading digits before the
        // digit counts are used as width certificates.
        let sign = deferred.cmp_zero();
        debug_assert_eq!(sign, Ordering::Greater, "the deferred distance is positive");
        if deferred.cmp_zero_stable_under(gap_bits).is_some() {
            return Ordering::Greater;
        }
        if self
            .gap
            .cmp_zero_stable_under(deferred.stored_bits())
            .is_some()
        {
            return Ordering::Less;
        }
        self.resolve();
        self.gap.cmp_zero()
    }

    /// Certify the gap's sign when adding any word-sized offset cannot change it.
    pub(super) fn gap_dominates_word(&mut self) -> Option<Ordering> {
        self.gap.cmp_zero_stable_under(64)
    }

    /// Make the current height a confirmed new minimum and return `old_min - h`.
    pub(super) fn undercut_here(&mut self) -> Accumulator {
        let mut decrease = core::mem::take(&mut self.gap);
        decrease = -decrease;
        self.lower_by(decrease)
    }

    /// Apply a confirmed undercut at `h + offset` without first folding the gap.
    ///
    /// The gap is left at zero while the returned decrease propagates through
    /// boundaries. The caller then restores `h - (h + offset)` with
    /// [`Self::set_gap_below_offset`], so no offset buffer overlaps that work.
    pub(super) fn undercut_offset(&mut self, offset: &BigInt) -> Accumulator {
        let mut decrease = core::mem::take(&mut self.gap);
        decrease = -decrease;
        decrease.sub_bigint(offset);
        self.lower_by(decrease)
    }

    /// Restore the running-height reference after an offset emission became the anchor.
    ///
    /// The new anchor is `v = h + offset`, so `h - v = -offset`. The caller
    /// has already moved the old gap into the decrease; no wide value is copied.
    pub(super) fn set_gap_below_offset(&mut self, offset: &BigInt) {
        let mut gap = Accumulator::new();
        gap.sub_bigint(offset);
        self.gap = gap;
    }

    /// Lower the anchor from `A` to `v`, converting `A - v` into `m - v`.
    ///
    /// Followers first lose `A - v`, becoming `v - X`. Only then subtract the
    /// deferred `A - m` from the decrease, leaving the true drop in the minimum
    /// for the outer boundaries. The caller has already replaced the old gap.
    fn lower_by(&mut self, mut decrease: Accumulator) -> Accumulator {
        for follower in self.followers.iter_mut().flatten() {
            *follower -= &decrease;
        }
        if let Some(deferred) = self.deferred.take() {
            decrease -= &deferred;
        }
        decrease
    }

    /// Expose a parent minimum lower by `boundary`, keeping the anchor fixed.
    ///
    /// Repeated closes add to `(A - m)` at the cost of the narrower operand.
    /// Neither the height gap nor any follower is traversed here.
    pub(super) fn defer(&mut self, boundary: Boundary) {
        self.deferred = Some(match self.deferred.take() {
            None => boundary.into_accumulator(),
            Some(mut deferred) => {
                boundary.add_to(&mut deferred);
                deferred
            }
        });
    }

    /// Move the anchor to the true minimum, updating all relative values together.
    ///
    /// Adding `A - m` makes the gap `h - m`; subtracting it makes followers
    /// `m - X`. The gap merge reads the narrower buffer. The fixed number of
    /// followers adds at most a constant multiple of the consumed distance's width.
    pub(super) fn resolve(&mut self) {
        let Some(deferred) = self.deferred.take() else {
            return;
        };
        for follower in self.followers.iter_mut().flatten() {
            *follower -= &deferred;
        }
        self.gap += deferred;
    }

    /// Whether the current minimum lies below the anchor.
    pub(super) fn is_deferred(&self) -> bool {
        self.deferred.is_some()
    }

    /// Attach the anchor-relative value `A - X` to one vacant follower slot.
    pub(super) fn set_follower(&mut self, slot: usize, follower: Accumulator) {
        debug_assert!(self.followers[slot].is_none(), "one follower per slot");
        self.followers[slot] = Some(follower);
    }

    /// Detach `A - X` as stored; it equals `m - X` only after resolution.
    pub(super) fn take_follower(&mut self, slot: usize) -> Accumulator {
        self.followers[slot].take().expect("the follower is active")
    }

    /// Add `h - A` to convert an anchor-relative difference into `h - X`.
    pub(super) fn add_gap_to(&self, delta: &mut Accumulator) {
        *delta += &self.gap;
    }

    /// Subtract `h - m` to convert a height-relative difference into `m - X`.
    pub(super) fn subtract_gap_from(&self, delta: &mut Accumulator) {
        debug_assert!(
            !self.is_deferred(),
            "the anchor must first be the true minimum"
        );
        *delta -= &self.gap;
    }

    /// Compare `h + above` with `A + candidate`, where deferred terms cancel.
    pub(super) fn compare_offset_to(
        &mut self,
        above: &BigInt,
        candidate: &Accumulator,
    ) -> Ordering {
        self.gap -= candidate;
        self.gap.add_bigint(above);
        let sign = self.gap.cmp_zero();
        self.gap.sub_bigint(above);
        self.gap += candidate;
        sign
    }
}
