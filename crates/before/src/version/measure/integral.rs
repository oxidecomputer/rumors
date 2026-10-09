//! Exact integration without repeatedly copying wide heights.
//!
//! A version is a step function on `[0, 1)`. At a common tree depth `S`, a
//! region at depth `d` has integer width `2^(S - d)`. Integrating means summing
//! `height * width`; dividing the result by `2^S` gives the rank. The same
//! machinery integrates the absolute or signed difference of two versions.
//!
//! The difficulty is that consecutive heights are stored as differences. A
//! very wide height followed by many tiny changes takes little input space,
//! but multiplying the full height by every region's width would repeatedly
//! traverse that wide integer. [`Accumulator`] makes adding a small change
//! cheap; the decomposition below also makes using the resulting height cheap.
//!
//! # Separate heights by how often they need to be read
//!
//! The current height is the sum of four components:
//!
//! - The opening height stays in `base`. It applies everywhere, so one shifted
//!   addition at the end accounts for its entire integral.
//! - Recent changes stay in `live`. Each region adds `live * width` directly.
//! - Older changes stay in `parked`. Their widths accumulate over a *segment*,
//!   the consecutive regions between two transfers from `live` to `parked`.
//!   One product accounts for the parked height over that whole segment.
//! - A parked height that becomes too wide for subsequent changes moves to
//!   `deferred`. Its contribution over the remaining regions is computed at
//!   the end, together with the other deferred heights.
//!
//! A **freeze** transfers `live` into `parked`. After each boundary, the driver
//! compares the width of `live` with the widest delta just read. If `live`
//! exceeds it by more than [`HEIGHT_FREEZE_ALLOWANCE_DIGITS`], it freezes before
//! processing the next region. Thus a wide change can pay for one wide region
//! addition, but later tiny changes cannot keep provoking that same wide work.
//! Oscillations made of wide deltas remain live: their own input already pays
//! for reading the wide value.
//!
//! At a freeze, the preceding segment is settled first. If its parked height
//! is much wider than the incoming live changes, that parked height is then
//! deferred before the next segment starts. This order is essential: the old
//! parked height still owes the preceding segment, whereas live changes have
//! already contributed there region by region. Moving them into the parked
//! height must affect only subsequent regions. Deferral prevents a wide parked
//! height from being multiplied again at every later narrow freeze.
//!
//! # Measure distances between boundaries
//!
//! A segment stores the sum of its regions' widths, never an absolute sweep
//! position. Using absolute positions would be expensive: two nearby deep
//! boundaries can have large binary coordinates while their distance is one
//! bit at a large power-of-two scale.
//!
//! [`width::ScaledWidth`] retains that scale separately. [`width::SparseWidth`]
//! compresses runs of ones using signed base-2^32 digits: `2^(32k) - 1` needs
//! just `-1` at digit zero and `+1` at digit `k`. Multiplication groups nearby
//! digits while keeping large gaps separate, so neither an absolute position
//! nor an unwritten zero prefix becomes a dense multiplication operand.
//!
//! No segment widths are collected before the first freeze. There is no
//! parked height to multiply by them, and a deferred height applies only to
//! widths after its deferral. A sweep that never freezes therefore pays only
//! for the opening height and its live changes.
//!
//! # Settle deferred heights once
//!
//! Suppose height `P_i` is deferred at boundary `i`, and `w_j` is the total
//! width between successive deferrals. Its remaining contribution is
//! `P_i * sum(w_j for j > i)`. Computing every suffix separately would repeat
//! work. Instead, [`deferred`] reduces adjacent groups in a balanced tree.
//! Merging an older left group with a newer right group adds exactly
//! `(sum of left heights) * (sum of right widths)`. Each required pair
//! `P_i * w_j`, `i < j`, belongs to exactly one such merge.
//!
//! The tree balances the amount of integer data, rather than the number of
//! deferrals. This keeps an unusually large height or width near the root,
//! where it is read fewer times. Signed accumulator sums cancel opposing
//! heights before multiplication; sparse width sums keep long runs compact.
//!
//! # Pairwise measures and their cost
//!
//! For two versions the driver maintains their difference `D`. The height to
//! integrate is `sigma * D`: `sigma` selects the positive part, absolute value,
//! or unchanged signed difference. At a boundary, with new values `D'` and
//! `sigma'`, its change is
//!
//! `sigma * (D' - D) + (sigma' - sigma) * D'`.
//!
//! The first term uses the input deltas directly. The second is needed only
//! when the sign changes; then `|D'| <= |D' - D|`, so reading `D'` is bounded
//! by those deltas too. [`Integrator::jump`] adds this correction.
//!
//! Each input delta therefore pays for the accumulator changes and width
//! checks it causes. Freezes and deferrals move a wide value only after input
//! established that width; relative segment widths are bounded by the tree
//! structure traversed inside them. The deferred reduction bounds repeated
//! reads by its data-weighted depth. Its clustered integer products give the
//! overall `O(M(n))` time and `O(n)` transient-space bounds in `n` input bits,
//! where `M(n)` is the cost of multiplying `n`-bit integers. Exact integration
//! can itself encode an arbitrary integer product, so the multiplication cost
//! is necessary. The measurement tests exercise that reduction as well as the wide
//! heights, dense widths, and cancellation cases that require this machinery.

use core::cmp::Ordering;

use num_bigint::{BigInt, BigUint};
use suanpan::Accumulator;

use crate::accumulator::BigIntAccumulator as _;

use deferred::DeferredIntegral;
use width::ScaledWidth;

use super::HEIGHT_FREEZE_ALLOWANCE_DIGITS;

mod deferred;
mod pair;
mod width;

#[cfg(feature = "meter")]
pub use width::{densified_digits, reset_densified_digits};

#[cfg(test)]
thread_local! {
    /// Successful transfers of nonzero live changes on this test thread.
    /// Tests use this to establish that wide-input cases reach the freeze path.
    pub static FREEZE_HITS: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };

    /// Counts nonzero parked heights deferred on this test thread.
    /// Tests use this to establish that wide-arming cases reach the deferred
    /// reduction, which a freeze count alone cannot show.
    pub static DEFERRAL_HITS: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
}

/// Integrates consecutive regions whose widths sum to one whole domain.
///
/// The driver opens at the first height, adds each region with [`Self::interval`],
/// folds the next boundary's changes into [`Self::live`], and calls
/// [`Self::boundary`] before adding another region. [`Self::finish`] accounts
/// for the constant and deferred components and returns the exact numerator.
pub struct Integrator {
    /// Contributions already integrated at the driver's common power-of-two scale.
    total: Accumulator,
    /// Changes since the last freeze. The driver folds boundary deltas here.
    pub live: Accumulator,
    /// Changes applying throughout the current segment, excluding deferred heights.
    parked: Accumulator,
    /// Sum of region widths since the last freeze.
    segment_width: Accumulator,
    /// Width tracking begins at the first nonzero freeze and remains active,
    /// even if parked heights later cancel: deferred heights still need widths.
    tracks_width: bool,
    /// The initial height, which applies to the entire domain.
    base: Accumulator,
    /// Older wide heights and the widths between their deferrals.
    deferred: DeferredIntegral,
    /// Reused for region-width additions, avoiding an integer per region.
    one: BigUint,
}

/// Advance the integral while charging each height component only over its lifetime.
impl Integrator {
    /// Start with no height or integrated regions.
    pub fn new() -> Self {
        Self {
            total: Accumulator::new(),
            live: Accumulator::new(),
            parked: Accumulator::new(),
            segment_width: Accumulator::new(),
            tracks_width: false,
            base: Accumulator::new(),
            deferred: DeferredIntegral::new(),
            one: BigUint::from(1u8),
        }
    }

    /// Set an unsigned opening height before adding any regions or boundary changes.
    pub fn open_height(&mut self, opening: &BigUint) {
        self.base.add_shifted_limbs(0, opening.iter_u64_digits());
    }

    /// Set the opening difference before adding any regions or boundary changes.
    ///
    /// `orientation` is `1` when the difference contributes as stored and `-1`
    /// when it contributes with the opposite sign. Reading the accumulator
    /// directly avoids normalizing the opening value merely to copy it here.
    pub fn open_difference(&mut self, opening: &Accumulator, orientation: i8) {
        match orientation {
            1 => self.base += opening,
            -1 => self.base -= opening,
            _ => panic!("an active orientation is either -1 or 1"),
        }
    }

    /// Add the next region, whose width at the common scale is `2^weight_shift`.
    pub fn interval(&mut self, weight_shift: u64) {
        // This cheap test may miss cancellation in buffered digits. Adding a
        // mathematically zero `live` inside `total`'s buffer is harmless, and
        // `add_shifted` compacts and skips one that would extend that buffer.
        if !self.live.is_known_zero() {
            self.total.add_shifted(weight_shift, &mut self.live);
        }
        if self.tracks_width {
            self.segment_width
                .add_shifted_limbs(weight_shift, self.one.iter_u64_digits());
        }
    }

    /// Add `(sigma' - sigma) * D'` after the pairwise driver's sign change.
    ///
    /// The orientation `sigma` is monotone in the sign of `D` and lies in
    /// `{-1, 0, 1}`. A nonzero correction is therefore positive: the coefficient
    /// and the new difference have the same sign. Its coefficient has magnitude
    /// one or two. Applying the signed accumulator directly avoids normalizing
    /// it into a temporary magnitude merely to stream that magnitude back into
    /// another accumulator.
    pub fn jump(&mut self, coefficient: i8, order: Ordering, diff: &mut Accumulator) {
        if order == Ordering::Equal {
            return;
        }
        // Keep the sign check in release builds: an invalid orientation must
        // not silently turn subtraction into addition.
        assert_eq!(
            coefficient < 0,
            order == Ordering::Less,
            "the orientation correction must be nonnegative"
        );
        let shift = if coefficient.abs() == 2 { 1 } else { 0 };
        if order == Ordering::Less {
            self.live.sub_shifted(shift, diff);
        } else {
            self.live.add_shifted(shift, diff);
        }
    }

    /// Finish a boundary, bounding the live width by the widest delta read there.
    pub fn boundary(&mut self, delta_digits: usize) {
        if self.live.stored_digit_count() > delta_digits + HEIGHT_FREEZE_ALLOWANCE_DIGITS {
            self.freeze();
        }
    }

    /// Account for the old parked height, then park the recent changes.
    fn freeze(&mut self) {
        let drift_order = self.live.cmp_zero();
        if drift_order == Ordering::Equal {
            // Buffered changes cancel. Nothing moves, and the current segment
            // remains open because its parked height has not changed.
            self.live.reset();
            return;
        }
        let drift_digits = self.live.stored_digit_count();
        self.tracks_width = true;
        #[cfg(test)]
        FREEZE_HITS.with(|hits| hits.set(hits.get() + 1));

        // This segment belongs to the old parked height. Live changes have
        // already contributed to it region by region; folding them into the
        // parked height first would charge those contributions a second time.
        let parked = self.close_segment();
        if self.parked.stored_digit_count() > drift_digits + HEIGHT_FREEZE_ALLOWANCE_DIGITS {
            self.defer_parked(parked);
        }
        self.parked += &self.live;
        self.live.reset();

        // reset() clears the whole allocated span. Segment digits can be high
        // above an unwritten zero prefix; replacing the buffer avoids scanning it.
        self.segment_width = Accumulator::new();
    }

    /// Settle the completed segment and return its normalized parked height.
    ///
    /// A following deferral reuses the value already needed for multiplication,
    /// rather than reading the parked accumulator twice at the same boundary.
    fn close_segment(&mut self) -> Option<BigInt> {
        let width = ScaledWidth::read(&self.segment_width);
        if width.is_zero() {
            debug_assert!(
                self.parked.is_known_zero(),
                "the first freeze has no parked height or preceding width"
            );
            return None;
        }
        self.deferred.add_width(&width);
        if self.parked.is_known_zero() {
            return None;
        }
        let parked = self.parked.to_bigint();
        if parked == BigInt::ZERO {
            return None;
        }
        width.add_product(&mut self.total, &parked);
        Some(parked)
    }

    /// Record the parked height's remaining contribution from this boundary on.
    /// The preceding segment must already have been closed.
    fn defer_parked(&mut self, parked: Option<BigInt>) {
        if let Some(parked) = parked {
            #[cfg(test)]
            DEFERRAL_HITS.with(|hits| hits.set(hits.get() + 1));
            self.deferred.push(parked);
        }
        self.parked.reset();
    }

    /// Settle the last parked segment without collecting an unused deferred width.
    fn finish_segment(&mut self) {
        if self.parked.is_known_zero() {
            return;
        }
        let parked = self.parked.to_bigint();
        if parked != BigInt::ZERO {
            let width = ScaledWidth::read(&self.segment_width);
            width.add_product(&mut self.total, &parked);
        }
    }

    /// Finish the exact numerator at scale `2^closing_shift`.
    ///
    /// Live changes have already contributed at every region. The parked
    /// height owes its final segment, deferred heights owe all widths after
    /// their deferrals, and the base height owes the whole domain.
    pub fn finish(mut self, closing_shift: u64) -> (Ordering, BigUint) {
        self.finish_segment();
        if !self.deferred.is_empty() {
            self.deferred
                .add_width(&ScaledWidth::read(&self.segment_width));
            self.deferred.finish(&mut self.total);
        }
        if !self.base.is_known_zero() {
            self.total.add_shifted(closing_shift, &mut self.base);
        }
        self.total.biguint_parts()
    }
}
