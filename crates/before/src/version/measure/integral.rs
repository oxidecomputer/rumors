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
//! compares the working width of `live` with the bit length of the widest
//! delta just read. If `live` exceeds it by more than
//! [`HEIGHT_FREEZE_ALLOWANCE_BITS`], it freezes before
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
//! Parking keeps a run of narrow freezes in one accumulator. `parked` holds
//! the sum of every frozen drift not yet deferred, so consecutive drifts add
//! into one value and drifts of opposite sign cancel there: however many
//! freezes fire, the integrator keeps one parked height. A deferred entry
//! instead keeps its height and the width before it until
//! [`Integrator::finish`], and the deferred reduction adds each group's
//! summed heights and widths into its neighbor's at every level, reading an
//! entry's contribution again at each level above it. Deferral is therefore
//! kept for the case that needs it: a sum much wider than the changes now
//! arriving.
//!
//! Input already read pays for each product a freeze makes. The product
//! multiplies the sum the previous freeze kept. That freeze kept the sum only
//! if it was at most the allowance wider than the drift being added, so the
//! kept sum exceeds that drift's width by at most the allowance and the
//! addition's carry; a freeze that defers keeps the drift alone. Each drift
//! therefore pays for at most one product: its own width plus a constant,
//! times one segment's width.
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
//! checks it causes, up to one logarithmic factor. `suanpan` charges each
//! accumulator added at a nonzero shift its own width plus one logarithm of
//! the receiver's width, and `total`'s greatest width is `O(n)`, as the last
//! paragraph of this section shows. `interval` adds once per region, and
//! each region is a leaf that takes input bits to encode, so its additions
//! cost `O(n log n)` over a sweep. That fits within the `O(M(n))` bound below
//! because `num-bigint`'s multiplication grows as a power of `n` above one,
//! faster than `n log n`. No known lower bound gives `M(n) = Ω(n log n)` in
//! general, so the fit rests on the backend.
//!
//! Freezes and deferrals move a wide value only after input established that
//! width; relative segment widths are bounded by the tree structure traversed
//! inside them. A width's absolute position is not bounded that way, so the
//! segment and deferred widths each keep one buffer for the whole sweep, and
//! growing it to a high position happens once; clearing either between
//! segments keeps the buffer and costs amortized O(1). The trade is space:
//! each kept buffer holds up to the scale's digits for the rest of the sweep,
//! `O(n)` and inside the space bound below. The deferred reduction
//! bounds repeated reads by its data-weighted depth. Its clustered integer
//! products give the overall `O(M(n))` time and `O(n)` transient-space
//! bounds in `n` input bits, where `M(n)` is the cost of multiplying `n`-bit
//! integers with the backend in use, `num-bigint`, rather than a schoolbook
//! `n²`. Public rustdoc states the bound without naming the backend, because
//! the backend is not part of what callers are promised. Exact integration
//! can itself encode an arbitrary integer product, so the multiplication cost
//! is necessary. The measurement tests exercise that reduction as well as the
//! wide heights, dense widths, and cancellation cases that require this
//! machinery.
//!
//! The bound also rests on one property of `suanpan` that no row of its table
//! shows on its own. [`Integrator::interval`] adds `live` into `total` at the
//! region's weight shift, `S - d` bits, so a region of a few input bits can
//! raise `total`'s top about `(S - d) / 32` digits above where cancellation
//! among earlier contributions left it, and a later contribution can cancel
//! it again. If every such raise cost its distance, a sweep alternating deep
//! and shallow regions, such as the board's `weight-comb` family, would take
//! time quadratic in its input. It does not, by the clause "Raising the top
//! across gaps" in `suanpan`'s [cost model](suanpan#raising-the-top-across-gaps):
//! a raise to a width `total` has reached before costs no growth, and each
//! crossing of the emptied stretch costs a base-64 logarithm of its length,
//! within the accumulator row's own logarithmic term. `total` is created once
//! per sweep and never cloned into or shifted, so the growth charged to it
//! over the whole sweep totals at most the greatest width it reaches, which
//! can far exceed its final width once distance and lag cancel. That width
//! is `O(n)`: every contribution's top lies within `S` plus the width of the
//! height it contributes, apart from `O(log n)` bits of carries; a tree of
//! depth `S` takes `Ω(S)` input bits to encode; and each height's width is
//! bounded by the deltas that built it. A `suanpan` change that charged a
//! raise its distance would break the `O(M(n))` bound here.

use core::cmp::Ordering;

use num_bigint::{BigInt, BigUint};
use suanpan::Accumulator;

use deferred::DeferredIntegral;
use width::ScaledWidth;

/// Extra bits of working width the live height change may hold beyond the
/// newest delta's bit length before [`Integrator::boundary`] freezes it.
///
/// The cushion avoids moving arithmetic state for small width fluctuations;
/// being fixed, it does not affect the input-proportional bound.
pub(crate) const HEIGHT_FREEZE_ALLOWANCE_BITS: u64 = 256;

mod deferred;
mod pair;
mod width;

#[cfg(feature = "meter")]
pub use width::{densified_digits, reset_densified_digits};

#[cfg(test)]
thread_local! {
    /// Freezes on this test thread.
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
    /// The sum of the frozen drifts not yet deferred, which applies throughout
    /// the current segment.
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
        // This cheap test may miss cancellation in buffered digits;
        // `add_shifted` compacts `live` and skips it if zero.
        if !self.live.is_known_zero() {
            self.total.add_shifted(weight_shift, &self.live);
        }
        if self.tracks_width {
            self.segment_width.add_shifted_limbs(weight_shift, [1]);
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
    pub fn jump(&mut self, coefficient: i8, order: Ordering, diff: &Accumulator) {
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

    /// Finish a boundary, bounding the live width by the bit length of the
    /// widest delta read there.
    pub fn boundary(&mut self, delta_bits: u64) {
        if self.live.stored_bits() > delta_bits + HEIGHT_FREEZE_ALLOWANCE_BITS {
            self.freeze();
        }
    }

    /// Account for the old parked height, then park the recent changes.
    ///
    /// The drift is nonzero whenever the trigger fires: `interval` compacts
    /// `live` before each boundary, so a drift that cancels keeps no width
    /// beyond its deltas'. Freezing a zero drift would still be exact; it would
    /// only settle the segment early and park nothing.
    fn freeze(&mut self) {
        // Compact the drift so that its width measures its value.
        self.live.sign();
        let drift_bits = self.live.stored_bits();
        self.tracks_width = true;
        #[cfg(test)]
        FREEZE_HITS.with(|hits| hits.set(hits.get() + 1));

        // This segment belongs to the old parked height. Live changes have
        // already contributed to it region by region; folding them into the
        // parked height first would charge those contributions a second time.
        // A parked height much wider than the drift then owes every later
        // region, so it is deferred rather than multiplied at each narrow
        // freeze. Reading it left its working width at its value's width.
        if let Some(parked) = self.close_segment() {
            if self.parked.stored_bits() > drift_bits + HEIGHT_FREEZE_ALLOWANCE_BITS {
                #[cfg(test)]
                DEFERRAL_HITS.with(|hits| hits.set(hits.get() + 1));
                self.deferred.push(parked);
                self.parked.clear();
            }
        }
        self.parked += &self.live;
        self.live.clear();

        // Keep the buffer. A fresh accumulator would grow from digit 0 to the
        // next segment's weight shift at every freeze, and that shift can sit
        // Θ(n) digits up. Clearing keeps the buffer, at amortized O(1).
        self.segment_width.clear();
    }

    /// Settle the completed segment and return its parked height, or `None`
    /// if that height is zero.
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
        let parked = BigInt::from(&self.parked);
        if parked == BigInt::ZERO {
            return None;
        }
        width.add_product(&mut self.total, &parked);
        Some(parked)
    }

    /// Settle the last parked segment without collecting an unused deferred width.
    fn finish_segment(&mut self) {
        if self.parked.is_known_zero() {
            return;
        }
        let parked = BigInt::from(&self.parked);
        if parked != BigInt::ZERO {
            let width = ScaledWidth::read(&self.segment_width);
            width.add_product(&mut self.total, &parked);
        }
    }

    /// Return the exact signed numerator at scale `2^closing_shift`.
    ///
    /// Live changes have already contributed at every region. The parked
    /// height owes its final segment, deferred heights owe all widths after
    /// their deferrals, and the base height owes the whole domain.
    pub fn finish(mut self, closing_shift: u64) -> Accumulator {
        self.finish_segment();
        if !self.deferred.is_empty() {
            self.deferred
                .add_width(&ScaledWidth::read(&self.segment_width));
            self.deferred.finish(&mut self.total);
        }
        if !self.base.is_known_zero() {
            self.total.add_shifted(closing_shift, &self.base);
        }
        self.total
    }
}
