//! Version-height emissions, comparisons, and followers without suspended payloads.
//!
//! Values may be the running height itself or an offset from it. The generic
//! range operations handle nesting and boundary propagation; these methods
//! translate each form into an anchor move or a temporary comparison.

use core::cmp::Ordering;

use num_bigint::{BigInt, Sign};
use suanpan::Accumulator;

use super::RangeMinima;
use crate::accumulator::BigIntAccumulator as _;

/// Track emitted minima and client differences when boundaries need no payload.
impl RangeMinima<()> {
    /// Emit the running height, arming pending ranges or lowering existing minima.
    pub fn emit_here(&mut self) {
        if self.has_pending() {
            self.arm_at_height(&mut (), |_| (), |(), _| ());
        } else if self.undercuts_here() {
            self.undercut(&mut (), |(), _| ());
        }
    }

    /// Emit `v = h + offset`, changing the minima only where the value is lower.
    ///
    /// Pending ranges take `-offset` as their first height gap. Existing ranges
    /// first try a leading-digit comparison when the gap dominates a word-sized
    /// offset. Otherwise, temporarily move the height to `v`, compare, and undo
    /// the fold. An undercut instead makes `v` the new anchor; its height gap
    /// becomes `h - v = -offset`.
    pub fn emit_offset(&mut self, offset: &BigInt) {
        if offset.sign() == Sign::NoSign {
            self.emit_here();
            return;
        }
        if self.has_pending() {
            let mut below = Accumulator::new();
            below.sub_bigint(offset);
            self.arm_below(below, &mut (), |_| (), |(), _| ());
            return;
        }
        if self.emit_when_gap_dominates(offset) {
            return;
        }

        self.anchor.fold_height(offset);
        if !self.undercuts_here() {
            self.anchor.subtract_offset(offset);
            return;
        }
        self.undercut(&mut (), |(), _| ());
        self.anchor.set_gap_below_offset(offset);
    }

    /// Handle a small offset when leading digits already decide its ordering.
    ///
    /// This requires `A = m`. A dominating positive gap means the emission
    /// stays above the minimum. A negative gap means it undercuts; move the gap
    /// into `decrease = -gap - offset` and propagate that owned value. Returning
    /// `false` leaves the ordinary folded comparison to the caller.
    fn emit_when_gap_dominates(&mut self, offset: &BigInt) -> bool {
        if self.deferred_live() || u64::try_from(offset.magnitude()).is_err() {
            return false;
        }
        let Some(sign) = self.anchor.gap_dominates_word() else {
            return false;
        };
        match sign {
            Ordering::Greater => {}
            Ordering::Less => {
                let decrease = self.anchor.undercut_offset(offset);
                self.propagate_drop(decrease, &mut (), |(), _| ());
                self.anchor.set_gap_below_offset(offset);
            }
            Ordering::Equal => unreachable!("a decisive gap is nonzero"),
        }
        true
    }

    /// Arm pending ranges at `h - below`, reusing the supplied accumulator.
    pub fn emit_below_accum(&mut self, below: Accumulator) {
        self.arm_below(below, &mut (), |_| (), |(), _| ());
    }

    /// Compare `h + above` with the innermost minimum without emitting it.
    ///
    /// A wide gap can decide a word-sized offset from its leading digits.
    /// Otherwise a temporary height fold exposes the candidate to the ordinary
    /// minimum comparison. That comparison may resolve a deferred distance;
    /// undoing the offset still restores the running height under the new anchor.
    pub fn compare_above(&mut self, above: &BigInt) -> Ordering {
        debug_assert!(self.armed(), "a comparison needs an armed range");
        if !self.deferred_live() && u64::try_from(above.magnitude()).is_ok() {
            if let Some(sign) = self.anchor.gap_dominates_word() {
                return sign;
            }
        }
        self.anchor.fold_height(above);
        let ordering = self.anchor.compare_height();
        self.anchor.subtract_offset(above);
        ordering
    }

    /// Compare `h + above` with the candidate minimum `A + arm_offset`.
    ///
    /// Their difference is `gap + above - arm_offset`. Any deferred distance
    /// cancels because both values use the same anchor, so it need not be read.
    pub fn compare_above_vs(&mut self, above: &BigInt, arm_offset: &Accumulator) -> Ordering {
        debug_assert!(self.armed(), "a comparison needs an armed range");
        self.anchor.compare_offset_to(above, arm_offset)
    }

    /// Arm pending ranges at `v = A + arm_offset`, taking ownership of the offset.
    ///
    /// The same value first changes the height gap to `h - v`, then becomes
    /// the boundary `v - old_minimum`. Neither endpoint is copied or materialized.
    pub fn arm_relative(&mut self, arm_offset: Accumulator) {
        debug_assert!(
            self.has_pending(),
            "relative arming requires a pending range"
        );
        debug_assert!(self.armed(), "relative arming requires an existing anchor");
        let pending = core::mem::take(&mut self.pending);
        let above_minimum = self.anchor.arm_relative(arm_offset);
        self.armed += pending;
        self.finish_arming(above_minimum, pending, &mut (), |_| (), |(), _| ());
    }

    /// Attach the value `A - X`, which will follow changes in the anchor.
    ///
    /// Its abstract value is `m - X`. If a distance is deferred, the supplied
    /// value must still use `A`, not `m`; resolving later shifts every follower
    /// by that distance together. Attaching therefore never traverses it.
    pub fn follower_set(&mut self, slot: usize, follower: Accumulator) {
        debug_assert!(self.armed(), "a follower needs an armed range");
        self.anchor.set_follower(slot, follower);
    }

    /// Detach `A - X` exactly as stored, without resolving the deferred distance.
    ///
    /// Resolve first when the consumer needs `m - X`. Anchor-relative uses,
    /// such as [`Self::bridge_add_gap`], let the deferred terms cancel instead.
    pub fn follower_take(&mut self, slot: usize) -> Accumulator {
        self.anchor.take_follower(slot)
    }

    /// Convert `A - X` into `h - X` by adding the height gap `h - A`.
    ///
    /// The anchor cancels, so even a wide deferred distance is never read.
    pub fn bridge_add_gap(&mut self, delta: &mut Accumulator) {
        self.anchor.add_gap_to(delta);
    }

    /// Convert `h - X` into `m - X` after resolving the deferred distance.
    ///
    /// Subtracting the resolved gap `h - m` changes only the reference point.
    pub fn bridge_sub_gap(&mut self, delta: &mut Accumulator) {
        self.anchor.subtract_gap_from(delta);
    }
}
