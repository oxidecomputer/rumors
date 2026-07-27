//! The query folds over skyline streams: rank, distance, lag, min_ticks,
//! and projection from single leaf sweeps, never reconstructing absolute
//! heights.
//!
//! Every fold here is a linear functional or a masking of the version's
//! step function, so each rides the same machinery as the comparison
//! sweep — one forward pass of its leaf cursors with the running height
//! state on the cliff-immune [`Accum`] — plus the piece its own question
//! needs:
//!
//! - [`rank`](fn@rank) integrates the step function: `Σ heightᵢ · 2^(−depthᵢ)`
//!   over the leaves, telescoped through height *deltas* so no absolute
//!   height is ever rebuilt per leaf (the frozen/live split below).
//! - [`distance`](fn@distance) and [`lag`](fn@lag) are rank differences over the
//!   emission sweep's join and meet streams, subtracted through the
//!   class-first [`Rank::checked_sub`] — each factor linear, the
//!   composition linear.
//! - [`min_ticks`](fn@min_ticks) folds the identity
//!   `Σ bases = Σ leaf heights − Σ internal-node subtree minima` (each
//!   normal-form base is its node's subtree minimum less its parent's) on
//!   saturating machine words, with an early exit the moment any height
//!   leaves the `u64` range — such a height alone forces the saturated
//!   answer, because the tick floor dominates every leaf height.
//! - [`project`](fn@project) overlays the skyline against a packed *id* stream
//!   and re-emits the masked skyline through the collapsing output
//!   builder: owned regions keep their plateaus, unowned regions emit
//!   zero, and the absolute height is materialized only at ownership
//!   transitions — where the emitted code itself is that height, so the
//!   work is priced by the mandatory output (the comb × scattered-party
//!   cross is Θ(teeth · magnitude) output from linear input, and this
//!   sweep is I/O-linear on it).
//!
//! # The frozen/live height split
//!
//! The rank integral must add `height · 2^(S − depth)` per leaf (`S` the
//! stream's maximum depth, found by one topology-only pre-scan), but a
//! per-leaf read of the full height re-imports the quadratic the delta
//! coding invites: on the boundary comb the height is a `2^k`-scale value
//! behind 3-bit stored deltas. The sweep therefore splits the height as
//! `F + L`: `L` (*live*) an accumulator holding the drift since the last
//! freeze, `F` (*frozen*) the rest — an accumulator touched only when a
//! freeze evicts `L` into it. Per leaf the sweep adds only `L`'s digits,
//! and `F` reaches the total through summation by parts rather than
//! through per-leaf or per-segment products:
//!
//! `Σᵢ F(i) · massᵢ = F_final · 2^S − Σ_freezes drift · position`
//!
//! — one `F_final`-wide shifted add when the stream ends (the leaf
//! masses tile the interval, so the closing weight is exactly `2^S`)
//! plus one correction per freeze, priced by the *drift* being evicted:
//! a drift-wide product per nonzero signed digit of the compacted freeze
//! position (a ones-run position — every comb's shape — compacts to two
//! digits) and one position-wide read. Nothing ever multiplies by `F`'s
//! width, so a wide frozen value set once is never re-read per freeze.
//!
//! A freeze fires exactly when a folded delta leaves `L` more than
//! `FREEZE_ALLOWANCE_DIGITS` digits wider than that delta's own code:
//! stale wide drift is about to ride under cheaper codes, so the sweep
//! evicts it once — charged to the codes that built the drift, which the
//! freeze consumes and resets — and the cheap codes continue on an
//! emptied `L`. Bounded oscillation at *any* width keeps `L` within its
//! own codes' width and never freezes: every wide-tooth fold is paid by
//! the tooth's own code, on either side of any fixed width.
//!
//! # Cost
//!
//! Derived, with the constants pinned by the `skyline_rank_*`,
//! `skyline_min_ticks_*`, and `skyline_project_*` rows of the
//! resource-envelope suite (`tests/meter.rs`): the cursor scan, decode,
//! and fold bounds are the comparison sweep's; rank adds O(`L` digits)
//! per leaf — bounded by the freeze allowance plus the width of the
//! delta folded at the previous boundary, so each per-leaf add is paid
//! by the code that set `L`'s width — plus the freeze corrections: one
//! position read and one drift-wide product per nonzero compacted
//! position digit each, funded by the codes that built the drift
//! wherever the freeze position compacts to O(1) digits (every comb: its
//! freeze positions are ones-runs). A stream that re-arms wide drift
//! under cheap codes *at a dense position* — deep alternating topology
//! around every freeze — still pays its corrections at position density,
//! the one shape whose funding the freeze discipline does not certify.
//! min_ticks adds one
//! machine-word min-merge per node on a `u64` pending stack (the one
//! per-level word this module keeps — 8 bytes against the level's ≥ 3
//! stream bits); projection adds one height materialization per
//! ownership transition, priced by the code it emits. Transient state is
//! the cursor paths, the accumulators, min_ticks' word stack, and — for
//! projection — the output builder's per-level bit stacks.
//!
//! # Testing
//!
//! The recursive tree oracle is the behavioral witness: every fold is
//! differentially pinned against it through the bridge (exact `Rank`
//! equality, exact `u64` equality, byte-identical projection streams;
//! distance and lag re-derived from the oracle's join, meet, and rank
//! through the valuation identities) over the adversarial generator
//! families, arbitrary normal-form trees, organic op-trace histories,
//! and the exhaustive small scope; rank additionally against the
//! semantic Riemann-sum oracle, which shares no structure with the
//! sweep. The resource envelopes are the meter rows named above.

use core::cmp::Ordering;

use crate::codec::accum::Accum;
use crate::codec::{self, Base, BitCursor, Bits, BitsSlice, SliceCursor};
use crate::step;
use crate::Rank;

use super::build::SkylineBuilder;
use super::emit::{self, signed_sum};
use super::sweep::{LeafCursor, Side, Step};
use super::{gamma_code, zigzag_signed};

/// The live accumulator's tolerated width overshoot, in base-2^32
/// digits, over the just-folded delta's own width: a fold that leaves
/// `L` wider than its delta by more than this freezes the height split.
///
/// Relative to the delta, so bounded oscillation never freezes at any
/// width — a tooth's fold is paid by the tooth's own code — while stale
/// drift under cheaper codes is evicted at the first such code. 8 digits
/// (256 bits) of slack: reaching it from the codes' own widths would
/// take more small folds than any real stream holds, and it caps how far
/// a per-leaf `L` add can outgrow the code that last set `L`'s width.
const FREEZE_ALLOWANCE_DIGITS: usize = 8;

/// The exact causal rank of the version a skyline stream denotes.
///
/// One topology pre-scan for the maximum depth, then one leaf sweep
/// integrating the step function on the frozen/live height split (the
/// module doc carries the algebra and the cost argument). Equal to
/// [`Version::rank`](crate::Version::rank) on the decoded version, which
/// the differential suite pins exactly.
///
/// # Panics
///
/// Panics if the operand is not a canonical skyline stream — run
/// [`validate`](fn@super::validate) first on untrusted bytes — or is
/// deeper than `u32::MAX` levels (the rank exponent would overflow; such
/// a stream exceeds 2 GiB).
pub fn rank(bits: &BitsSlice) -> Rank {
    let max_depth = max_depth(bits);
    let scale =
        u32::try_from(max_depth).expect("rank exponent overflows u32: stream deeper than 2^32");
    let (mut cursor, first) = LeafCursor::open(bits);
    let mut total = Accum::new();
    let mut live_height = Accum::new();
    let mut frozen = Accum::new();
    frozen.add_base(&first);
    let mut position = Accum::new();
    let one = Base::from(1u8);
    loop {
        // Per-leaf: the live component's contribution and the leaf's mass.
        let weight_shift = (max_depth - cursor.depth()) as u64;
        if !live_height.is_zero() {
            total.add_accum_shl(&live_height, weight_shift);
        }
        position.add_base_shl(&one, weight_shift);
        if cursor.done() {
            break;
        }
        let step = cursor.step(&mut live_height, Side::A);
        if live_height.digit_count() > base_digits(&step.magnitude) + FREEZE_ALLOWANCE_DIGITS {
            freeze(&mut total, &mut frozen, &mut live_height, &position);
        }
    }
    // The closing term: the frozen component weighted by the whole
    // interval — the leaf masses tile it, so the weight is exactly 2^S.
    total.add_accum_shl(&frozen, max_depth as u64);
    let (sign, num) = total.sign_magnitude();
    debug_assert_ne!(sign, Ordering::Less, "heights are nonnegative");
    Rank::from_raw(Base::from(num), scale)
}

/// Freeze the height split: evict the live drift into the frozen
/// component and debit the eviction's correction from the total.
///
/// The closing `F_final · 2^S` term will credit the evicted drift across
/// the *whole* interval, but the drift belongs only to the leaves after
/// this freeze, so the sweep debits `drift · position` here — the
/// summation-by-parts correction, priced by the drift's own width times
/// the compacted position's nonzero digits, never by the frozen width.
fn freeze(total: &mut Accum, frozen: &mut Accum, live_height: &mut Accum, position: &Accum) {
    let (drift_sign, drift) = live_height.sign_magnitude();
    debug_assert_ne!(
        drift_sign,
        Ordering::Equal,
        "a freeze evicts a nonzero drift: the trigger requires a wide live component"
    );
    let (position_sign, position) = position.sign_magnitude();
    debug_assert_eq!(
        position_sign,
        Ordering::Greater,
        "leaf masses only accumulate"
    );
    let drift = Base::from(drift);
    mul_into(
        total,
        &drift,
        &Base::from(position),
        drift_sign == Ordering::Greater,
    );
    match drift_sign {
        Ordering::Less => frozen.sub_base(&drift),
        _ => frozen.add_base(&drift),
    }
    *live_height = Accum::new();
}

/// Add (or, with `subtract`, remove) `factor · digits` in the total: one
/// `factor`-wide product per nonzero signed digit of the compacted
/// `digits` operand.
///
/// The `digits` operand's base-2^32 digits are compacted greedily into
/// balanced signed digits, so an all-ones run — the usual shape of a
/// freeze position's dyadic mass — costs one subtract at its floor and
/// one carry past its top instead of a product per digit.
fn mul_into(total: &mut Accum, factor: &Base, digits: &Base, subtract: bool) {
    if *factor == Base::ZERO || *digits == Base::ZERO {
        return;
    }
    let mut carry = 0u64;
    let mut add_term = |digit: u64, negative: bool, shift: u64| {
        if digit == 0 {
            return;
        }
        let mut product = factor.clone();
        product *= u32::try_from(digit).expect("a compacted signed digit fits 32 bits");
        if negative == subtract {
            total.add_base_shl(&product, shift);
        } else {
            total.sub_base_shl(&product, shift);
        }
    };
    let mut shift = 0u64;
    for digit in u32_digits(digits) {
        let t = u64::from(digit) + carry;
        if t > 1 << 31 {
            // Balanced arm: `t − 2^32` with a carry, so ones-runs cancel.
            add_term((1u64 << 32) - t, true, shift);
            carry = 1;
        } else {
            add_term(t, false, shift);
            carry = 0;
        }
        shift += 32;
    }
    if carry == 1 {
        add_term(1, false, shift);
    }
}

/// A stored magnitude's width in base-2^32 digits (minimum 1).
fn base_digits(value: &Base) -> usize {
    let digits = usize::try_from(value.bits().div_ceil(32)).expect("digit counts fit usize");
    digits.max(1)
}

/// A magnitude's little-endian base-2^32 digits.
///
/// The top digit of the top limb may be zero (the compaction loop skips
/// zero digits, so the padding is free).
fn u32_digits(value: &Base) -> Vec<u32> {
    crate::codec::base::U64Limbs::new(&value.0)
        .flat_map(|limb| [(limb & 0xFFFF_FFFF) as u32, (limb >> 32) as u32])
        .collect()
}

/// The causal distance between the versions two skyline streams denote:
/// the rank of their symmetric difference.
///
/// `rank(join) − rank(meet)` with both factors on this module's linear
/// sweeps and the subtraction on the class-first
/// [`Rank::checked_sub`]. Equal to
/// [`Version::distance`](crate::Version::distance) exactly.
///
/// # Panics
///
/// Panics on a non-canonical operand or a stream deeper than `u32::MAX`
/// levels, exactly as [`rank`](fn@rank) does.
pub fn distance(a: &BitsSlice, b: &BitsSlice) -> Rank {
    let join = rank(&emit::join(a, b));
    let meet = rank(&emit::meet(a, b));
    join.checked_sub(&meet)
        .expect("the join dominates the meet, so its rank is at least the meet's")
}

/// How far the first stream's version lags behind the second's: the rank
/// of the history the second records that the first does not.
///
/// `rank(join) − rank(a)`, the directed half of [`distance`](fn@distance).
/// Equal to [`Version::lag`](crate::Version::lag) exactly.
///
/// # Panics
///
/// Panics on a non-canonical operand or a stream deeper than `u32::MAX`
/// levels, exactly as [`rank`](fn@rank) does.
pub fn lag(a: &BitsSlice, b: &BitsSlice) -> Rank {
    let join = rank(&emit::join(a, b));
    join.checked_sub(&rank(a))
        .expect("the join dominates self, so its rank is at least self's")
}

/// The minimum number of ticks that could have produced the version a
/// skyline stream denotes, saturating at [`u64::MAX`].
///
/// Folds `Σ leaf heights − Σ internal-node subtree minima` (each
/// normal-form base is its node's minimum less its parent's, so the sum
/// telescopes to exactly the stored-base total) over one leaf sweep: the
/// heights ride the delta accumulator, the pending minima ride a `u64`
/// stack merged as ancestors close. The moment any height leaves the
/// `u64` range the answer is already saturated — the tick floor
/// dominates every leaf height — so the sweep exits early and no wide
/// arithmetic ever reaches the sums. Equal to
/// [`Version::min_ticks`](crate::Version::min_ticks) on the decoded
/// version, which the differential suite pins exactly.
///
/// # Panics
///
/// Panics if the operand is not a canonical skyline stream — run
/// [`validate`](fn@super::validate) first on untrusted bytes.
pub fn min_ticks(bits: &BitsSlice) -> u64 {
    let (mut cursor, first) = LeafCursor::open(bits);
    let mut height = Accum::new();
    height.add_base(&first);
    // Sums fit u128 by construction: at most 2^64-scale values times a
    // leaf count bounded by the stream's bit length.
    let mut leaf_sum: u128 = 0;
    let mut min_sum: u128 = 0;
    let mut pending: Vec<u64> = Vec::new();
    let mut current_min: u64;
    match height_word(&mut height) {
        Some(v) => {
            leaf_sum += u128::from(v);
            current_min = v;
        }
        None => return u64::MAX,
    }
    while !cursor.done() {
        let depth_before = cursor.depth();
        let step = cursor.step(&mut height, Side::A);
        // Every popped right-branch level closed one internal node: merge
        // its pending left minimum with the completed right subtree's.
        for _ in 0..depth_before - step.flip {
            let left = pending
                .pop()
                .expect("every closing node has a pending left minimum");
            current_min = current_min.min(left);
            min_sum += u128::from(current_min);
        }
        // The flip level's left sibling is complete: its minimum waits
        // for the right subtree that starts here.
        pending.push(current_min);
        match height_word(&mut height) {
            Some(v) => {
                leaf_sum += u128::from(v);
                current_min = v;
            }
            None => return u64::MAX,
        }
    }
    // The final leaf closes every remaining ancestor from the right.
    while let Some(left) = pending.pop() {
        current_min = current_min.min(left);
        min_sum += u128::from(current_min);
    }
    debug_assert!(
        leaf_sum >= min_sum,
        "a subtree minimum never exceeds its leaves"
    );
    u64::try_from(leaf_sum - min_sum).unwrap_or(u64::MAX)
}

/// The height accumulator's value as a machine word, or [`None`] past
/// the `u64` range.
///
/// The sign fold's collapse compacts the representation first, so a
/// small value always reads O(1) digits; a value needing more than three
/// digits after the collapse is necessarily past `2^64`.
fn height_word(height: &mut Accum) -> Option<u64> {
    match height.sign() {
        Ordering::Equal => return Some(0),
        Ordering::Less => unreachable!("a canonical stream keeps heights nonnegative"),
        Ordering::Greater => {}
    }
    if height.digit_count() > 3 {
        return None;
    }
    let (_, magnitude) = height.sign_magnitude();
    u64::try_from(magnitude).ok()
}

/// Project the version a skyline stream denotes onto a packed id's owned
/// region, as a canonical skyline stream.
///
/// One overlay of the skyline leaf cursor against the id's constant
/// regions: owned intervals keep the skyline's plateaus (their deltas
/// re-emitted verbatim), unowned intervals emit height zero, and each
/// ownership transition emits the absolute height once — the jump the
/// output must record anyway, which is what prices the sweep by its
/// input plus its mandatory output. The output stream is byte-identical
/// to the recursive oracle's semantic mask, which the differential suite
/// pins.
///
/// # Panics
///
/// Panics if the skyline operand is not a canonical stream.
pub fn project(ev_bits: &BitsSlice, id: &crate::Party) -> Bits {
    let id_bits = id.as_bits();
    let (mut sc, first) = LeafCursor::open(ev_bits);
    let mut ic = IdLeafCursor::open(id_bits);
    let mut height = Accum::new();
    height.add_base(&first);
    let mut owned = ic.owned();
    let mut out = SkylineBuilder::with_capacity(ev_bits.len() + id_bits.len());
    let opening = if owned { first } else { Base::ZERO };
    out.leaf(sc.depth().max(ic.depth()), gamma_code(&opening));
    while !(sc.done() && ic.done()) {
        let ev_step = advance_overlay(&mut sc, &mut ic, &mut height);
        let now_owned = ic.owned();
        let (negative, magnitude) = match (owned, now_owned) {
            // Inside an owned run the output moves with the skyline; a
            // boundary the id alone crossed is a zero delta.
            (true, true) => match &ev_step {
                Some(step) => (step.negative, step.magnitude.clone()),
                None => (false, Base::ZERO),
            },
            (false, false) => (false, Base::ZERO),
            // Entering the owned region: the output jumps to the current
            // absolute height.
            (false, true) => (false, absolute_height(&mut height)),
            // Leaving it: the output drops from the height *before* this
            // boundary's fold — the new height minus the folded delta.
            (true, false) => {
                let now = absolute_height(&mut height);
                let (negative, magnitude) = match &ev_step {
                    Some(step) => signed_sum(false, now, !step.negative, &step.magnitude),
                    None => (false, now),
                };
                debug_assert!(!negative, "heights are nonnegative");
                (magnitude != Base::ZERO, magnitude)
            }
        };
        owned = now_owned;
        out.leaf(
            sc.depth().max(ic.depth()),
            gamma_code(&zigzag_signed(negative, magnitude)),
        );
    }
    // Canonicalizing the storage is `Version::from_bits`'s job, the
    // single gate a stream passes through when it becomes a stored value.
    out.finish()
}

/// The current absolute height, materialized at an ownership transition.
///
/// The sign fold's collapse compacts the accumulator first, so the read
/// is O(the height's own digits) — priced by the transition code the
/// caller emits.
fn absolute_height(height: &mut Accum) -> Base {
    let sign = height.sign();
    debug_assert_ne!(sign, Ordering::Less, "heights are nonnegative");
    let (_, magnitude) = height.sign_magnitude();
    Base::from(magnitude)
}

/// Advance the skyline × id overlay one boundary.
///
/// The deeper cursor steps, and the other in the same step on a tie
/// (the comparison sweep's bookkeeping, with the id side's flip levels
/// playing the same role). Returns the skyline's consumed delta when
/// that side stepped.
fn advance_overlay(
    sc: &mut LeafCursor<'_>,
    ic: &mut IdLeafCursor<'_>,
    height: &mut Accum,
) -> Option<Step> {
    match sc.depth().cmp(&ic.depth()) {
        Ordering::Greater => {
            let step = sc.step(height, Side::A);
            if step.flip <= ic.depth() {
                let flip = ic.step();
                debug_assert_eq!(
                    step.flip, flip,
                    "tied boundaries close to one shared flip level"
                );
            }
            Some(step)
        }
        Ordering::Less => {
            let flip = ic.step();
            (flip <= sc.depth()).then(|| {
                let step = sc.step(height, Side::A);
                debug_assert_eq!(
                    flip, step.flip,
                    "tied boundaries close to one shared flip level"
                );
                step
            })
        }
        Ordering::Equal => {
            let step = sc.step(height, Side::A);
            let flip = ic.step();
            debug_assert_eq!(
                step.flip, flip,
                "equal-depth leaves share their whole path, so their flip levels agree"
            );
            Some(step)
        }
    }
}

/// A cursor at the current constant-ownership region of a packed id
/// stream.
///
/// The id-side mirror of the skyline [`LeafCursor`]: the same
/// root-to-leaf path bits and the same flip bookkeeping, with a 1-bit
/// payload (owned or not) instead of a height delta. Absent children in
/// the packed form are unowned regions, so the cursor synthesizes an
/// empty leaf wherever a present-child flag is clear without consuming
/// stream bits; exhaustion is therefore tracked by the path's
/// left-branch count (zero means the current leaf is the preorder last),
/// not by stream position.
struct IdLeafCursor<'a> {
    cursor: SliceCursor<'a>,
    /// Root-to-leaf branch directions, root first.
    path: Bits,
    /// Parallel to `path`: whether each level's right child is present
    /// in the stream (a clear flag is a synthetic unowned leaf).
    right_present: Bits,
    /// Left-branch levels still open; zero exactly at the final leaf.
    lefts: usize,
    /// Whether the current leaf's region is owned.
    owned: bool,
}

impl<'a> IdLeafCursor<'a> {
    /// Open a packed id stream at its first constant region.
    ///
    /// The empty stream is the empty id — one unowned region over the
    /// whole interval — mirroring the packed coding, where absence *is*
    /// the empty region.
    ///
    /// # Panics
    ///
    /// Panics if the stream is not a canonical packed id.
    fn open(bits: &'a BitsSlice) -> Self {
        let mut this = IdLeafCursor {
            cursor: SliceCursor::new(bits, 0),
            path: Bits::new(),
            right_present: Bits::new(),
            lefts: 0,
            owned: false,
        };
        if !bits.is_empty() {
            this.descend();
        }
        this
    }

    /// The current region's depth: its interval has width `2^-depth`.
    fn depth(&self) -> usize {
        self.path.len()
    }

    /// Whether the current region is owned by the id.
    fn owned(&self) -> bool {
        self.owned
    }

    /// Whether the current region is the stream's last (its interval
    /// ends at the unit interval's right edge).
    fn done(&self) -> bool {
        self.lefts == 0
    }

    /// Advance past the current region to the next, returning the flip
    /// level's depth for the caller's tie test.
    ///
    /// # Panics
    ///
    /// Panics if the stream is not a canonical packed id. Never called
    /// on a final region (the overlay stops when both cursors are done).
    fn step(&mut self) -> usize {
        loop {
            match self.path.pop() {
                Some(true) => {
                    self.right_present.pop();
                    continue;
                }
                Some(false) => break,
                None => unreachable!(
                    "the advanced cursor is never at its final region: an all-right path means the stream is consumed"
                ),
            }
        }
        self.lefts -= 1;
        self.path.push(true);
        let flip = self.path.len();
        if *self
            .right_present
            .last()
            .expect("a flipped level recorded its right-child flag")
        {
            self.descend();
        } else {
            // The absent right child: one synthetic unowned region at the
            // flip level itself.
            self.owned = false;
        }
        flip
    }

    /// Descend from the cursor to the next stored region in preorder,
    /// extending the path with a left branch per internal node passed.
    ///
    /// # Panics
    ///
    /// Panics if the stream is not a canonical packed id.
    fn descend(&mut self) {
        loop {
            step!();
            // The two tag-bit reads below record themselves through the
            // cursor's recording `read_bit`: no separate tag record, or
            // every 2-bit tag would count twice.
            let left = self.cursor.read_bit().expect("canonical id bits");
            let right = self.cursor.read_bit().expect("canonical id bits");
            if !left && !right {
                // The full leaf: an owned terminal region.
                self.owned = true;
                return;
            }
            self.path.push(false);
            self.lefts += 1;
            self.right_present.push(right);
            if !left {
                // The absent left child: a synthetic unowned region.
                self.owned = false;
                return;
            }
        }
    }
}

/// The maximum leaf depth of a skyline stream: one topology-only
/// pre-scan, payload codes skipped unread.
///
/// # Panics
///
/// Panics if the stream is not a canonical skyline encoding.
fn max_depth(bits: &BitsSlice) -> usize {
    let mut cursor = codec::DsiCursor::new(bits);
    let mut path = Bits::new();
    let mut deepest = 0usize;
    loop {
        // Descend to the next leaf: one unary read per descent.
        step!();
        let k = cursor.read_unary().expect("canonical skyline bits");
        for _ in 0..k {
            path.push(false);
        }
        deepest = deepest.max(path.len());
        // TODO-recalibrate: `skip_int` already records the skipped code's
        // width internally, so this caller-side record double-counts every
        // payload code (uniformly 2x, deterministic). Deleting it is a
        // separate recalibration: the scan envelopes in `tests/meter.rs`
        // and the board's pinned scan constants that price this walk must
        // be re-measured when the caller-side record goes.
        let code_start = cursor.position();
        cursor.skip_int().expect("canonical skyline bits");
        codec::scan::record_bits(cursor.position() - code_start);
        // Close finished ancestors; the flip continues, no open left
        // branch means the stream is complete.
        loop {
            match path.pop() {
                Some(true) => continue,
                Some(false) => {
                    path.push(true);
                    break;
                }
                None => return deepest,
            }
        }
    }
}

#[cfg(test)]
mod tests;
