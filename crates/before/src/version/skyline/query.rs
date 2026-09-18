//! Streaming queries over a skyline's step function.
//!
//! A query walks each leaf stream once while keeping its running height in a
//! carry-cliff-free [`Accumulator`]. Rank, distance, lag, and rank comparison
//! are integrals of the height or of the difference between two heights.
//! `min_ticks` uses the equivalent sum of leaf heights minus internal subtree
//! minima. Projection masks the step function by a party's owned intervals and
//! emits a new canonical skyline. None of these operations reconstructs an
//! absolute height at every leaf or materializes an intermediate skyline.
//!
//! # The height split
//!
//! The rank integral must add `height · 2^(S − depth)` per leaf (`S` the
//! stream's maximum depth, found by one topology-only pre-scan), but a per-leaf
//! read of the full height re-imports the quadratic the delta coding invites:
//! on the boundary comb the height is a `2^k`-scale value behind 3-bit stored
//! deltas. Every fold here therefore splits its running quantity into anchored
//! components folded narrow, with a relative freeze trigger that evicts stale
//! wide drift at the first cheaper code. The rank fold and the pair co-sweep
//! run the anchored-segment split, `h* = B + P + L` — base, parked, live —
//! whose components the [`integral`] submodule defines and analyzes; the
//! min_ticks fold runs the epoch-ledger form,
//! `h = F + L`, frozen plus live (the `web` submodule).
//!
//! # The pair co-sweep: distance, lag, and the rank order
//!
//! Every pair measure is the integral of a *functional* of the running height
//! difference `D = h_a − h_b` over the overlay's elementary intervals, by the
//! valuation identities:
//!
//! - distance: `rank(a ∨ b) − rank(a ∧ b) = ∫ (max − min) = ∫ |D|`;
//! - lag: `rank(a ∨ b) − rank(a) = ∫ (max − h_a) = ∫ (−D)⁺`;
//! - rank order: `rank(a) − rank(b) = ∫ D`, of which only the total's
//!   sign is kept.
//!
//! The co-sweep maintains `D` exactly as the comparison sweep does and
//! integrates `h* = σ·D`, where `σ ∈ {−1, 0, +1}` is the measure's
//! *orientation* at `sign(D)` — constant on every interval of constant
//! `D`-sign, the whole family side by side:
//!
//! | functional           | `D > 0` | `D = 0` | `D < 0` |
//! |----------------------|---------|---------|---------|
//! | `∫ \|D\|` (distance) | `+1`    | `0`     | `−1`    |
//! | `∫ (−D)⁺` (lag)      | `0`     | `0`     | `−1`    |
//! | `∫ D` (rank order)   | `+1`    | `+1`    | `+1`    |
//!
//! The directed measures' integrand is nonnegative by construction (their
//! nonzero σ is `D`'s own sign); the signed one's carries `D`'s sign, and every
//! accumulator is signed. The rank order's constant `+1` means its walk never
//! sees an orientation change at all — the funding certificate covers it as the
//! two-ledger instance of the single-stream rank fold's constant-orientation
//! walk. The per-boundary algebra, the anchored-segment discipline, and the
//! funding certificate live with the machinery, in the [`integral`] submodule's
//! doc.
//!
//! # Cost
//!
//! Derived, with the constants pinned by the `skyline_rank_*`,
//! `skyline_min_ticks_*`, and `skyline_project_*` rows of the resource-envelope
//! suite (`tests/meter.rs`): the cursor scan, decode, and fold bounds are the
//! comparison sweep's. The freeze machinery beneath the rank and pair charges
//! is four structures — the live component, the segment mass, the promotion
//! ledger, and the settle tree — the same map the [`integral`] submodule's doc
//! opens with.
//!
//! Rank's charges, per the height split:
//!
//! - the per-leaf add: O(`L`) digits, bounded by the freeze allowance plus
//!   the width of the delta folded at the previous boundary, so each add
//!   is paid by the code that set `L`'s width;
//! - the certified freeze work: a settle per freeze at the multiplication
//!   bound over the parked width and the segment's within-segment depth
//!   variation, a ledger entry once per wide arming, and one mass-balanced
//!   product-tree settle at the sweep's close — the [`integral`]
//!   submodule's doc carries the settle bounds: `O(M(|v|))` under every
//!   power-law tier of the backend's multiplication, at most one extra
//!   tree-depth factor past its quasilinear threshold, and `Ω(M(|v|))`
//!   mandatory for any fold that answers exactly;
//! - the practical regime: the freeze machinery's two feeds (segment mass
//!   and position window) open at the first freeze, so a sweep that never
//!   freezes — word-scale heights, the regime the `RANK_CONCURRENT` row
//!   gauges — pays the integral's own folds and nothing toward the settle
//!   machinery.
//!
//! The `skyline_flatness` module's freeze-position, promotion re-arm, and
//! dense-suffix bands hold the many-freezes and many-armings genres flat, and
//! the `ledger_wide_arming` and `answer_embedded_product` bands hold the wide ×
//! dense genres flat per byte in the fold's own traffic.
//!
//! Distance and lag (the `DISTANCE_*`/`LAG_*` rows, plus the `skyline_flatness`
//! module's jump-pair and pair re-arm bands) add, per boundary, work bounded by
//! the boundary's own folded codes — the difference and integrand folds and the
//! orientation-change read — plus the same certified freeze work, and two
//! topology-only pre-scans for the overlay scale; transiently they hold the two
//! cursor paths, the integrator's accumulators, and the promotion ledger (one
//! parked value and one window per arming, dropped at the close), never an
//! emitted stream.
//!
//! min_ticks adds one fold into the range-minimum stack's gap per delta, O(1)
//! bookkeeping per node (a count bump at each close, a boundary move at each
//! pop), one settle per reign record at the record's own funded width, and the
//! epoch ledger's one product per freeze at the evicted drift's width — the
//! `web` submodule certifies every charge (the `skyline_flatness` module's
//! pure-comb and reveal-comb bands hold the cost of closing revealed ranges
//! flat in the touch counter).
//!
//! Projection adds one height materialization per ownership transition, priced
//! by the code it emits. A leaf's next boundary is derived once and cached, so
//! observing a parked event cursor costs constant time per id boundary.
//! Transient state is the cursor paths, the accumulators, min_ticks' compressed
//! difference stack, and — for projection — the output builder's per-level bit
//! stacks.
//!
//! # Verification
//!
//! Property tests compare every query with recursive tree and semantic
//! function-space oracles over generated normal-form versions and histories.
//! Distance and lag also satisfy their join/meet valuation identities, while
//! deterministic resource envelopes bound the work on difficult input shapes.

// The module doc and the fold docs (`rank`, `distance`, `lag`, `rank_cmp`)
// cite the crate-private `integral` submodule's essay by intra-doc link so a
// rename cannot rot the prose (the internal doc build resolves every link); on
// the public build those links render as plain code spans — the items are
// private — which this allow accepts.
#![allow(rustdoc::private_intra_doc_links)]

use core::cmp::Ordering;

use suanpan::Accumulator;

use num_bigint::{BigInt, BigUint, Sign};

use crate::codec::{self, accumulator, gamma, BitsBuf, BitsView};
use crate::Rank;

use self::integral::{Integrator, FREEZE_ALLOWANCE_DIGITS};
use super::build::SkylineBuilder;
use super::overlay::{
    advance, advance_diff, Crossed, IdLeafCursor, LeafCursor, OpenedPair, PlateauCursor, Side,
};
use super::walk::LeafWalk;

/// The exact causal rank of the version a skyline stream denotes.
///
/// One topology pre-scan for the maximum depth, then one leaf sweep integrating
/// the step function on the anchored-segment height split (the [`integral`]
/// submodule's doc carries the algebra and the funding certification; rank is
/// its single-stream instance). Equal to
/// [`Version::rank`](crate::Version::rank) on the decoded version, which the
/// differential suite pins exactly.
///
/// # Panics
///
/// Panics if the operand is not a canonical skyline stream — run
/// [`validate`](fn@super::validate) first on untrusted bytes.
pub fn rank(bits: BitsView<'_>) -> Rank {
    let max_depth = LeafCursor::max_depth(bits);
    // Depth counts levels of a stream held in memory, so it always fits the u64
    // rank exponent.
    let scale = max_depth;
    let (mut cursor, first) = LeafCursor::open(bits);
    // The single-stream instance of the anchored-segment integral: the
    // integrand is the height itself, opened at the first leaf's absolute (the
    // plateau anchored at position zero) and folded delta-by-delta thereafter.
    let mut integral = Integrator::new();
    integral.open(&BigInt::from(first));
    loop {
        let weight_shift = max_depth - cursor.depth();
        integral.interval(weight_shift);
        if cursor.done() {
            break;
        }
        let (_, step) = cursor.step();
        Side::A.fold(&mut integral.live, &step);
        integral.boundary(accumulator::digit_len(step.magnitude()));
    }
    let (sign, numerator) = integral.finish(max_depth);
    debug_assert_ne!(sign, Ordering::Less, "heights are nonnegative");
    Rank::from_raw(numerator, scale)
}

/// The causal distance between the versions two skyline streams denote: the
/// rank of their symmetric difference.
///
/// One fused co-sweep integrating `|h_a − h_b|` over the overlay (the
/// [`integral`] submodule's doc carries the algebra, the anchored-segment
/// freeze discipline, and the funding certification): no join or meet stream
/// is materialized and no per-operand rank is recomputed. Equal to
/// [`Version::distance`](crate::Version::distance) exactly, and digit-exact
/// against the composed `rank(join) − rank(meet)`, which the differential suite
/// pins.
///
/// # Panics
///
/// Panics on a non-canonical operand, exactly as [`rank`](fn@rank) does.
pub fn distance(a: BitsView<'_>, b: BitsView<'_>) -> Rank {
    // `∫ |D|`: σ is `sign(D)` itself, so the integrand `σ·D` is `|D|`.
    Integrator::pair_rank(a, b, |sign| match sign {
        Ordering::Greater => 1,
        Ordering::Equal => 0,
        Ordering::Less => -1,
    })
}

/// How far the first stream's version lags behind the second's: the rank of the
/// history the second records that the first does not.
///
/// The same co-sweep as [`distance`](fn@distance) integrating the directed
/// functional `(h_b − h_a)⁺` instead of the symmetric `|h_a − h_b|`. Equal to
/// [`Version::lag`](crate::Version::lag) exactly, and digit-exact against the
/// composed `rank(join) − rank(a)`, which the differential suite pins.
///
/// # Panics
///
/// Panics on a non-canonical operand, exactly as [`rank`](fn@rank)
/// does.
pub fn lag(a: BitsView<'_>, b: BitsView<'_>) -> Rank {
    // `∫ (−D)⁺`: σ is `−1` exactly where `D < 0`, so the integrand keeps the
    // history `b` records beyond `a` and nothing else.
    Integrator::pair_rank(a, b, |sign| match sign {
        Ordering::Less => -1,
        _ => 0,
    })
}

/// Compares the exact ranks of two skyline streams without constructing either
/// [`Rank`].
///
/// The walk integrates `h_a - h_b` over the streams' common intervals. That
/// integral is `rank(a) - rank(b)`, so its sign gives the order. The
/// [`integral`] module explains how the walk keeps the arithmetic bounded by
/// the input while preserving the exact result.
///
/// # Complexity
///
/// For `n` total encoded input bytes, the walk takes `O(M(n) log n)` time and
/// `O(n)` transient space in the worst case. `M(n)` is the time to multiply
/// integers whose binary width is proportional to `n`. A rank tie must settle
/// the exact difference to zero, so retaining only its sign cannot improve the
/// worst-case bound.
///
/// # Panics
///
/// Panics on a non-canonical operand, exactly as [`rank`](fn@rank) does.
pub fn rank_cmp(a: BitsView<'_>, b: BitsView<'_>) -> Ordering {
    // `∫ D`, signed: σ is constantly `+1`, the total is
    // `rank(a) − rank(b)`, and only its sign is kept.
    Integrator::pair(a, b, |_| 1).0
}

impl Integrator {
    /// Integrate a nonnegative pair measure and normalize it as a rank.
    fn pair_rank(
        a_bits: BitsView<'_>,
        b_bits: BitsView<'_>,
        orientation: impl Fn(Ordering) -> i8,
    ) -> Rank {
        let (sign, total, scale) = Self::pair(a_bits, b_bits, orientation);
        debug_assert_ne!(sign, Ordering::Less, "the measure is nonnegative");
        Rank::from_raw(total, scale)
    }

    /// Integrate `orientation(sign(a - b)) * (a - b)` in one overlay walk.
    ///
    /// `orientation` must depend only on the sign and be monotone in it. This
    /// makes every orientation-change term a debit, as [`Self::jump`] requires.
    fn pair(
        a_bits: BitsView<'_>,
        b_bits: BitsView<'_>,
        orientation: impl Fn(Ordering) -> i8,
    ) -> (Ordering, BigUint, u64) {
        // The overlay's scale: elementary intervals nest inside both operands'
        // leaves, so the deepest one sits at the deeper operand's maximum depth.
        // Depth counts levels of streams held in memory, so it always fits the u64
        // rank exponent.
        let overlay_depth = LeafCursor::max_depth(a_bits).max(LeafCursor::max_depth(b_bits));
        let scale = overlay_depth;
        let OpenedPair {
            a: mut cursor_a,
            b: mut cursor_b,
            mut diff,
            ..
        } = OpenedPair::open(a_bits, b_bits);
        let mut current_orientation = orientation(diff.sign());
        let mut integral = Integrator::new();
        if current_orientation != 0 {
            // The opening plateau: `h* = σ·D`, anchored at position zero and priced
            // by the two absolute first codes (the sign read above has collapsed
            // the spelling). Negative exactly when σ and `D` disagree in sign —
            // never for the directed measures, whose nonzero σ is `D`'s own sign.
            let (opening_sign, opening) = accumulator::value(&diff);
            let negative = match opening_sign {
                Ordering::Greater => current_orientation < 0,
                Ordering::Less => current_orientation > 0,
                Ordering::Equal => false,
            };
            let sign = if negative { Sign::Minus } else { Sign::Plus };
            integral.open(&BigInt::from_biguint(sign, opening));
        }
        loop {
            let weight_shift = overlay_depth - cursor_a.depth().max(cursor_b.depth());
            integral.interval(weight_shift);
            if cursor_a.done() && cursor_b.done() {
                break;
            }
            let (step_a, step_b) = advance_diff(&mut cursor_a, &mut cursor_b, &mut diff);
            let new_orientation = orientation(diff.sign());
            if current_orientation != 0 {
                // The `σ·dD` term: each side's consumed delta re-folds into the
                // integrand, oriented by `σ` — a side swap is exactly the negation.
                for (side, step) in [(Side::A, &step_a), (Side::B, &step_b)] {
                    if let Some(step) = step {
                        let toward = if current_orientation > 0 {
                            side
                        } else {
                            side.other()
                        };
                        toward.fold(&mut integral.live, step);
                    }
                }
            }
            if new_orientation != current_orientation {
                integral.jump(new_orientation - current_orientation, &diff);
                current_orientation = new_orientation;
            }
            // The freeze trigger, relative to this boundary's own codes: the widest
            // magnitude folded here is what funds the next interval's live add.
            // The advance law always steps at least one side inside the loop, so
            // a boundary with no step is programmer error — and a fabricated
            // funded width would silently misprice the trigger, so the violation
            // fails loudly instead.
            let funded = step_a
                .iter()
                .chain(step_b.iter())
                .map(|step| accumulator::digit_len(step.magnitude()))
                .max()
                .expect("the advance law steps at least one side per boundary");
            integral.boundary(funded);
        }
        let (sign, total) = integral.finish(overlay_depth);
        (sign, total, scale)
    }
}

/// The minimum number of ticks that could have produced the version a skyline
/// stream denotes, exact at any magnitude.
///
/// Folds `Σ leaf heights − Σ internal-node subtree minima` (each normal-form
/// base is its node's minimum less its parent's, so the sum telescopes to
/// exactly the stored-base total) over one leaf sweep. Heights enter the total
/// as narrow live offsets over a frozen component that lives entirely in an
/// epoch ledger — one drift per freeze, settled against per-epoch reference
/// counts once, at the end — and the closing nodes' minima ride a range-minimum
/// stack whose closes count against current value records instead of
/// folding widths (the `web` submodule carries both structures, the accounting,
/// and the funding certificate). Equal to
/// [`Version::min_ticks`](crate::Version::min_ticks) on the decoded version,
/// which the differential suite pins exactly.
///
/// # Panics
///
/// Panics if the operand is not a canonical skyline stream — run
/// [`validate`](fn@super::validate) first on untrusted bytes.
pub fn min_ticks(bits: BitsView<'_>) -> BigUint {
    let (mut cursor, first) = LeafCursor::open(bits);
    // The height split: `h = F + L`, with `L` folding every delta and `F`
    // living entirely in the epoch ledger — one drift per freeze, settled
    // against per-epoch reference counts once, at the end. The first leaf's
    // absolute is epoch 0's drift.
    let mut live = Accumulator::new();
    // The narrow side of the total: `Σ leaf offsets − Σ minima offsets`, every
    // term relative to its own epoch's frozen component.
    let mut total = Accumulator::new();
    let mut ledger = web::EpochLedger::new(first);
    // Subtree ranges nest, so each close uses the innermost open minimum.
    let mut minima = web::ReignTracker::new();
    minima.open(cursor.depth());
    ledger.leaf_ref();
    minima.leaf(&BigInt::from(0u8), 0, &mut total, &mut ledger);
    while !cursor.done() {
        let depth_before = cursor.depth();
        let (flip, step) = cursor.step();
        Side::A.fold(&mut live, &step);
        minima.fold_height(&step);
        // Every popped right-branch level closed one internal node: its subtree
        // minimum folds into the total (a count on the
        // web's reigning record) and merges into its parent.
        for _ in 0..depth_before - flip {
            minima.close(&mut total, &mut ledger);
        }
        // Every left branch the descent pushed opened one node's range.
        minima.open(cursor.depth() - flip);
        // The new leaf: a stale-wide live component is evicted first, so the
        // offset entering the total is paid by the codes that built it (the
        // freeze discipline's funding argument).
        if live.digit_count() > accumulator::digit_len(step.magnitude()) + FREEZE_ALLOWANCE_DIGITS {
            ledger.freeze(&mut live);
        }
        let (live_sign, leaf_offset) = accumulator::value(&live);
        let leaf_sign = if live_sign == Ordering::Less {
            Sign::Minus
        } else {
            Sign::Plus
        };
        accumulator::fold(&mut total, &leaf_offset, 0, leaf_sign == Sign::Minus);
        ledger.leaf_ref();
        let leaf_offset = BigInt::from_biguint(leaf_sign, leaf_offset);
        minima.leaf(&leaf_offset, ledger.epoch(), &mut total, &mut ledger);
    }
    // The final leaf closes every remaining ancestor from the right, then the
    // ledger folds the frozen component's every reference.
    minima.drain(&mut total, &mut ledger);
    ledger.settle(&mut total);
    let (sign, magnitude) = accumulator::value(&total);
    debug_assert_ne!(
        sign,
        Ordering::Less,
        "a subtree minimum never exceeds its leaves"
    );
    magnitude
}

/// Project the version a skyline stream denotes onto a party's owned
/// region, as a canonical skyline stream.
///
/// One overlay of the skyline leaf cursor against the id's constant regions:
/// owned intervals keep the skyline's plateaus (their deltas re-emitted
/// verbatim), unowned intervals emit height zero, and each ownership transition
/// emits the absolute height once — the jump the output must record anyway,
/// which is what prices the sweep by its input plus its mandatory output. The
/// output stream is byte-identical to the recursive oracle's semantic mask,
/// which the differential suite pins.
///
/// # Panics
///
/// Panics if the skyline operand is not a canonical stream.
pub fn project(event_bits: BitsView<'_>, id: &crate::Party) -> BitsBuf {
    let id_bits = id.as_bits();
    let (mut event_cursor, first) = LeafCursor::open(event_bits);
    let mut id_cursor = IdLeafCursor::open(id_bits);
    let mut height = Accumulator::new();
    accumulator::fold(&mut height, &first, 0, false);
    let mut owned = id_cursor.owned();
    // The normal build pre-sizes to the operands'
    // summed lengths — an estimate, since the projection's output is not
    // derivable from its inputs and can outgrow them. The `before_alloc_ab` cfg
    // is reachable only through `RUSTFLAGS` (never a cargo feature, so no
    // dependent build can select it) and compiles in one alternative arm for
    // the allocation benchmark to measure against the shipped pre-size; shipped
    // builds always take the pre-sized arm.
    #[cfg(not(before_alloc_ab = "projection_growth"))]
    let capacity = event_bits.len() + id_bits.len();
    #[cfg(before_alloc_ab = "projection_growth")]
    let capacity = 0;
    let mut out = SkylineBuilder::with_capacity(capacity);
    let opening = if owned { first } else { BigUint::ZERO };
    out.leaf(event_cursor.depth().max(id_cursor.depth()), |out| {
        gamma::encode(&opening, out)
    });
    while !(event_cursor.done() && id_cursor.done()) {
        // Ownership-gated block: while the region is unowned and the skyline
        // cursor's next flip level sits strictly below the region's depth, the
        // sibling subtree that flip opens lies wholly inside the region — its
        // projection is constantly zero — so it is consumed as one block (the
        // crossing folded, then [`LeafCursor::skip_deeper`] to the subtree's
        // own end) and emitted as one zero-delta leaf at the subtree's root
        // depth. The finer all-zero tiling the per-boundary walk would emit
        // collapses to exactly this leaf in the builder, so the output bytes
        // are unchanged.
        if !owned {
            // A final leaf peeks zero, which no region depth is below, so
            // exhaustion stops the loop unconditionally.
            loop {
                let flip = event_cursor.peek_flip();
                if flip <= id_cursor.depth() {
                    break;
                }
                let (stepped_flip, step) = event_cursor.step();
                debug_assert_eq!(stepped_flip, flip, "the peeked flip is the step's own");
                Side::A.fold(&mut height, &step);
                event_cursor.skip_deeper(flip, &mut height);
                out.leaf(flip, |out| gamma::encode_positive(&BigUint::ZERO, out));
            }
            if event_cursor.done() && id_cursor.done() {
                break;
            }
        }
        // The overlay-advance law drives the skyline × id cursor mix; an id
        // crossing carries nothing, so the fold sees exactly the skyline's
        // deltas, each folded into the running height as it is consumed.
        let (ev_step, _) = advance(&mut event_cursor, &mut id_cursor, |crossing| {
            if let Crossed::A(step) = crossing {
                Side::A.fold(&mut height, step);
            }
        });
        let now_owned = id_cursor.owned();
        let delta = match (owned, now_owned) {
            // Inside an owned run the output moves with the skyline; a boundary
            // the id alone crossed is a zero delta.
            (true, true) => ev_step.unwrap_or_else(|| BigInt::from(0u8)),
            (false, false) => BigInt::from(0u8),
            // Entering the owned region: the output jumps to the current
            // absolute height.
            (false, true) => {
                height.sign();
                let height = accumulator::signed_value(&height);
                debug_assert!(height.sign() != Sign::Minus, "heights are nonnegative");
                height
            }
            // Leaving it: the output drops from the height *before* this
            // boundary's fold — the new height minus the folded delta.
            (true, false) => {
                height.sign();
                let now = accumulator::signed_value(&height);
                debug_assert!(now.sign() != Sign::Minus, "heights are nonnegative");
                let before = match ev_step {
                    Some(step) => now - step,
                    None => now,
                };
                debug_assert!(before.sign() != Sign::Minus, "heights are nonnegative");
                -before
            }
        };
        owned = now_owned;
        out.leaf(event_cursor.depth().max(id_cursor.depth()), |out| {
            gamma::encode_signed(&delta, out)
        });
    }
    let bits = out.finish();
    // Benchmark-only alternative: one exact-size
    // copy here, where the buffer is about to become storage — the freeze
    // adopts the buffer without copying, so the pre-size estimate's slack
    // otherwise stays resident for the value's whole life. The arm prices that
    // copy against the stranded capacity.
    #[cfg(before_alloc_ab = "projection_shrink")]
    let bits = {
        let mut bits = bits;
        bits.shrink_to_fit();
        bits
    };
    // Canonicalizing the storage is `Version::from_bits`'s job, the single gate
    // a stream passes through when it becomes a stored value.
    bits
}

/// The maximum leaf depth of a skyline stream: one topology-only pre-scan,
/// payload codes skipped unread.
///
/// # Panics
///
/// Panics if the stream is not a canonical skyline encoding.
impl LeafCursor<'_> {
    /// Find the maximum leaf depth without decoding payloads.
    fn max_depth(bits: BitsView<'_>) -> u64 {
        let mut cursor = codec::DsiCursor::new(bits);
        let mut deepest = 0u64;
        let mut walk = LeafWalk::new();
        while let Some(depth) = walk.descend(&mut cursor) {
            deepest = deepest.max(depth);
            cursor.skip_int().expect("canonical skyline bits");
        }
        deepest
    }
}

pub(crate) mod integral;
mod web;

#[cfg(test)]
mod tests;
