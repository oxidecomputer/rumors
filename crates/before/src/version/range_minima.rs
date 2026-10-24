//! Minima of properly nested ranges in a stream of values.
//!
//! A [`Version`](crate::Version) traversal opens subtree ranges, emits heights
//! in leaf order, and closes ranges from inner to outer. [`RangeMinima`]
//! maintains the minimum emitted inside each open range. A range is *pending*
//! until its first emission, then *armed* until it closes; pending ranges are
//! always inside all armed ranges.
//!
//! # Store differences, not repeated absolute minima
//!
//! An outer range contains every emission of its inner range, so
//!
//! `outer_min <= ... <= inner_min`.
//!
//! Store adjacent differences `inner_min - outer_min` as *boundaries*. Equal
//! minima form counted zero runs. Positive boundaries use space proportional
//! to their working widths: once a deeper boundary covers it, a boundary wider
//! than a word is copied into packed limbs, or kept whole only when it is
//! wider than [`INLINE_BOUNDARY_BITS`](boundaries::INLINE_BOUNDARY_BITS), wide
//! enough that its accumulator's fixed size is at most a constant per bit.
//! Thus deep nesting does not duplicate a wide absolute minimum at every
//! level. Expanding the zero runs gives exactly one boundary per adjacent pair
//! of armed ranges.
//!
//! # Distinct minima, in stack order
//!
//! A client that keeps state per *distinct* minimum, rather than per range,
//! learns when each one begins and ends. An emission reports whether it begins
//! a new innermost distinct minimum, and calls back once for each distinct
//! minimum it ends, innermost first. A close reports whether it ends the
//! innermost one. Every distinct minimum begins once and ends once, and the
//! one that ends is always the newest that has not yet ended, so the client's
//! state forms a stack whose top is the innermost minimum. Callbacks supply
//! client work outside this module's arithmetic bound.
//!
//! # Follow one minimum relative to the running height
//!
//! The caller advances one running height `h` by signed deltas. The tracker
//! keeps `gap = h - A`, where the *anchor* `A` normally equals the innermost
//! minimum `m`. A close exposing a lower minimum keeps the anchor fixed and
//! defers the distance `A - m`. This lets repeated closes combine boundaries
//! without repeatedly traversing an older wide gap.
//!
//! Optional *followers* hold `A - X` for fixed client values `X`. They move
//! with the anchor, so their abstract value follows the minimum as `m - X`.
//! Resolving the deferred distance moves the gap and every follower together
//! to `A = m`. Expressions in which the deferred terms cancel can use the
//! anchor-relative values directly.
//!
//! # Updating the ranges
//!
//! The first emission arms all pending ranges at one value. A later emission
//! matters only if it lowers the innermost minimum. That decrease moves
//! outward: zero boundaries transmit it unchanged; each positive boundary
//! absorbs as much of it as possible. If a boundary survives, the outer minimum
//! stays fixed and propagation stops. Every crossed boundary becomes zero and
//! ends the distinct minimum outside it. [`RangeMinima::propagate_drop`]
//! expresses those cases.
//!
//! A close consumes exactly one armed range. Equal parent and child minima
//! leave the anchor unchanged. A lower parent adds its boundary to the deferred
//! distance. Closing the final range clears the anchor; clients must detach
//! their followers first, and every range must see an emission before closing.
//!
//! # Why a wide value does not multiply the work
//!
//! Version heights have arbitrary precision. A wide early delta followed by
//! many tiny deltas must not cause a full-width traversal for every later
//! event. Under `suanpan`'s [cost rules](suanpan#costs), an accumulator
//! update costs what it adds, not its receiver's width, and a sign read
//! costs amortized O(1). Every update here is at shift zero, which adds no
//! logarithm, except the rebuild of a copied boundary: shifted updates into a
//! fresh accumulator of at most five limbs. Comparisons use leading digits
//! for well-separated widths, and consuming merges retain the wider buffer
//! while reading the narrower one.
//!
//! Each crossed boundary is consumed permanently; each closed boundary moves
//! into the deferred distance once. Storing or retrieving a boundary costs a
//! constant, because a boundary is copied only when a later push covers it,
//! and only up to a bounded width. Equal ranges use compact counts. For a
//! validated Version, arithmetic work is therefore amortized linear in encoded
//! value bits plus topology events, and tracker-owned storage is linear in
//! those inputs; state a client keeps per distinct minimum remains its own
//! responsibility. Depth alone does not multiply the cost of a shared wide
//! minimum.

use core::cmp::Ordering;

use num_bigint::BigInt;
use suanpan::Accumulator;

mod anchor;
mod boundaries;
mod boundary;
mod emission;

pub use anchor::FOLLOWER_SLOTS;

use anchor::Anchor;
use boundaries::{Boundaries, Entry};
use boundary::{Boundary, Positive, Remainder, Signed};

/// What becomes visible after closing the innermost range.
pub enum Close {
    /// No armed range remains; the innermost distinct minimum ended.
    Retired,
    /// The parent shares the minimum, which continues.
    Equal,
    /// The parent has a lower minimum; the innermost distinct minimum ended,
    /// and the parent's is innermost again.
    Lower,
}

/// Open ranges, their ordered minima, and one shared height reference.
pub struct RangeMinima {
    /// The innermost minimum relative to the running height and client values.
    anchor: Anchor,
    /// Differences between armed minima, with the innermost boundary last.
    boundaries: Boundaries,
    /// Innermost ranges still waiting for their first emission.
    pending: u64,
    /// Ranges with minima; the expanded boundary count is `armed - 1`.
    armed: u64,
}

/// Maintain range nesting and report distinct minima as they begin and end.
impl RangeMinima {
    /// Construct an empty stack; no height is retained until its first arming.
    pub fn new() -> Self {
        Self {
            anchor: Anchor::new(),
            boundaries: Boundaries::new(),
            pending: 0,
            armed: 0,
        }
    }

    /// Whether an innermost minimum exists to query or update.
    pub fn armed(&self) -> bool {
        self.armed > 0
    }

    /// Whether any innermost ranges still need their first minimum.
    pub fn has_pending(&self) -> bool {
        self.pending > 0
    }

    /// Open `count` nested ranges in O(1), without allocating boundaries.
    pub fn open(&mut self, count: u64) {
        self.pending += count;
    }

    /// Advance the running height without changing any minimum.
    ///
    /// The fold reads `delta`, not the potentially wider accumulated gap.
    /// With no armed range there is no anchor, so the change needs no storage.
    pub fn fold_height(&mut self, delta: &BigInt) {
        if self.armed() {
            self.anchor.fold_height(delta);
        }
    }

    /// Advance the running height by a change the caller already holds in an
    /// accumulator.
    ///
    /// A caller that sums a block of deltas passes the net change here. A zero
    /// anchor adopts the supplied storage directly, without normalizing or
    /// rereading the value merely to accumulate it again.
    pub fn fold_accumulator(&mut self, delta: Accumulator) {
        if self.armed() {
            self.anchor.fold_accumulator(delta);
        }
    }

    /// Give all pending ranges their first minimum at the current height.
    ///
    /// First arming establishes an anchor and equal minima. With older armed
    /// ranges, moving the anchor gives the new value's signed distance above
    /// the old minimum; that distance determines the connecting boundary.
    ///
    /// Returns whether the value begins a new innermost distinct minimum; `end`
    /// runs once for each distinct minimum it ends, innermost first.
    pub fn arm_at_height<C>(&mut self, context: &mut C, end: impl FnMut(&mut C)) -> bool {
        debug_assert!(self.has_pending(), "arming requires a pending range");
        let pending = core::mem::take(&mut self.pending);
        if !self.armed() {
            self.anchor.initialize(Accumulator::new());
            self.armed = pending;
            self.boundaries.push_equal(pending - 1);
            return true;
        }
        let above_minimum = self.anchor.arm_at_height();
        self.armed += pending;
        self.finish_arming(above_minimum, pending, context, end)
    }

    /// Give pending ranges their first minimum at `v = h - below`.
    ///
    /// The owned `below` becomes the new height gap. With an existing anchor,
    /// the displaced gap supplies `v - A = old_gap - below`; accounting for
    /// any deferred distance then yields the connecting difference `v - m`.
    /// Returns and calls back as [`Self::arm_at_height`] does.
    pub fn arm_below<C>(
        &mut self,
        below: Accumulator,
        context: &mut C,
        end: impl FnMut(&mut C),
    ) -> bool {
        debug_assert!(self.has_pending(), "arming requires a pending range");
        let pending = core::mem::take(&mut self.pending);
        if !self.armed() {
            self.anchor.initialize(below);
            self.armed = pending;
            self.boundaries.push_equal(pending - 1);
            return true;
        }
        let above_minimum = self.anchor.arm_below(below);
        self.armed += pending;
        self.finish_arming(above_minimum, pending, context, end)
    }

    /// Connect newly armed ranges to the older ranges using `v - old_minimum`.
    ///
    /// The anchor and followers already describe `v`. A positive difference
    /// begins a new distinct minimum above the older one, on a new boundary.
    /// Equality continues the older minimum. A negative difference ends the
    /// older minimum and lowers every enclosing minimum reached by the drop,
    /// then begins `v`. All new ranges share `v`.
    fn finish_arming<C>(
        &mut self,
        above_minimum: Accumulator,
        pending: u64,
        context: &mut C,
        mut end: impl FnMut(&mut C),
    ) -> bool {
        match Signed::of(above_minimum) {
            Signed::Positive(above_minimum) => {
                self.boundaries
                    .push_positive(Boundary::from_positive(above_minimum));
                self.boundaries.push_equal(pending - 1);
                true
            }
            Signed::Zero => {
                self.boundaries.push_equal(pending);
                false
            }
            Signed::Negative(drop) => {
                end(context);
                self.propagate_drop(drop, context, end);
                self.boundaries.push_equal(pending);
                true
            }
        }
    }

    /// Whether emitting at the current height would lower the innermost minimum.
    ///
    /// A deferred distance may be resolved to decide comparable-width values;
    /// the minimum itself is unchanged by this query.
    pub fn undercuts_here(&mut self) -> bool {
        self.anchor.compare_height() == Ordering::Less
    }

    /// Apply an emission already confirmed below the innermost minimum.
    ///
    /// Move the anchor to the height, update its followers, and consume any
    /// deferred distance before propagating the true drop `old_minimum - h`.
    /// The emission begins a new innermost distinct minimum; `end` runs once
    /// for each it ends, innermost first, starting with the old innermost one.
    pub fn undercut<C>(&mut self, context: &mut C, mut end: impl FnMut(&mut C)) {
        let Signed::Positive(decrease) = Signed::of(self.anchor.undercut_here()) else {
            unreachable!("a confirmed undercut lowers the minimum by a positive amount")
        };
        end(context);
        self.propagate_drop(decrease, context, end);
    }

    /// Carry a positive decrease from the innermost minimum toward outer ranges.
    ///
    /// The anchor already describes the new minimum. Equal boundaries pass
    /// the decrease through unchanged. A positive boundary either retains the
    /// part above the decrease, transmits the remaining decrease outward, or
    /// disappears exactly at equality. Every vanished boundary ends the
    /// distinct minimum outside it, and all resulting zero boundaries merge into
    /// one run.
    fn propagate_drop<C>(
        &mut self,
        mut decrease: Positive,
        context: &mut C,
        mut end: impl FnMut(&mut C),
    ) {
        let mut equal_boundaries = 0;
        while let Some(entry) = self.boundaries.pop() {
            match entry {
                Entry::Equal(count) => equal_boundaries += count,
                Entry::Positive(boundary) => match boundary.lowered_by(decrease) {
                    Remainder::Decrease(remaining) => {
                        end(context);
                        equal_boundaries += 1;
                        decrease = remaining;
                    }
                    Remainder::Boundary(remaining) => {
                        self.boundaries.push_positive(remaining);
                        break;
                    }
                    Remainder::Equal => {
                        end(context);
                        equal_boundaries += 1;
                        break;
                    }
                },
            }
        }
        self.boundaries.push_equal(equal_boundaries);
    }

    /// Close exactly one armed range and expose its parent's minimum.
    ///
    /// An equal parent needs only a zero-run decrement. A lower parent moves
    /// its boundary into the deferred anchor distance, at the cost of the
    /// narrower accumulator. The final close clears the anchor altogether.
    pub fn close(&mut self) -> Close {
        debug_assert!(
            !self.has_pending(),
            "every closing range must first see an emission"
        );
        debug_assert!(self.armed(), "a closing range must be armed");
        self.armed -= 1;
        if !self.armed() {
            debug_assert!(self.boundaries.is_empty(), "no boundaries without ranges");
            self.anchor.clear();
            return Close::Retired;
        }
        match self
            .boundaries
            .pop()
            .expect("adjacent armed ranges have a boundary")
        {
            Entry::Equal(count) => {
                if count > 1 {
                    self.boundaries.push_equal(count - 1);
                }
                Close::Equal
            }
            Entry::Positive(boundary) => {
                self.anchor.defer(boundary);
                Close::Lower
            }
        }
    }

    /// Make the anchor equal the true minimum, updating the gap and all followers.
    pub fn resolve_deferred(&mut self) {
        self.anchor.resolve();
    }

    /// Whether a close has left the anchor above the true minimum.
    pub fn deferred_live(&self) -> bool {
        self.anchor.is_deferred()
    }
}

#[cfg(test)]
mod tests;
