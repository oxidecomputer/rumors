//! The query folds over skyline streams: rank, distance, lag, min_ticks,
//! and projection from single leaf sweeps, never reconstructing absolute
//! heights.
//!
//! Every fold here is a linear functional or a masking of the version's
//! step function, so each rides the same machinery as the comparison
//! sweep — one forward pass of its leaf cursors with the running height
//! state on the cliff-immune [`Accumulator`] — plus the piece its own question
//! needs:
//!
//! - [`rank`](fn@rank) integrates the step function: `Σ heightᵢ · 2^(−depthᵢ)`
//!   over the leaves, telescoped through height *deltas* so no absolute
//!   height is ever rebuilt per leaf — the single-stream instance of the
//!   anchored-segment split below.
//! - [`distance`](fn@distance) and [`lag`](fn@lag) integrate a directed functional of
//!   the two operands' height difference in one fused co-sweep —
//!   distance = `∫ |h_a − h_b|`, lag = `∫ (h_b − h_a)⁺` — on the
//!   comparison sweep's merge walk, with no join or meet stream
//!   materialized and no per-operand rank recomputed (the pair-co-sweep
//!   section below carries the algebra, the anchored-segment freeze
//!   discipline, and the funding certification).
//! - [`min_ticks`](fn@min_ticks) folds the identity
//!   `Σ bases = Σ leaf heights − Σ internal-node subtree minima` (each
//!   normal-form base is its node's subtree minimum less its parent's)
//!   exactly, at any magnitude: heights and minima enter the total as
//!   narrow epoch-relative offsets, the frozen component arrives
//!   through counting, and the closing nodes' minima ride a
//!   range-minimum anchor web whose closes count instead of fold (the
//!   `web` submodule carries the accounting and its funding
//!   certificate).
//! - [`project`](fn@project) overlays the skyline against a packed *id* stream
//!   and re-emits the masked skyline through the collapsing output
//!   builder: owned regions keep their plateaus, unowned regions emit
//!   zero, and the absolute height is materialized only at ownership
//!   transitions — where the emitted code itself is that height, so the
//!   work is priced by the mandatory output (the comb × scattered-party
//!   cross is Θ(teeth · magnitude) output from linear input, and this
//!   sweep is I/O-linear on it).
//!
//! # The height split and the freeze allowance
//!
//! The rank integral must add `height · 2^(S − depth)` per leaf (`S` the
//! stream's maximum depth, found by one topology-only pre-scan), but a
//! per-leaf read of the full height re-imports the quadratic the delta
//! coding invites: on the boundary comb the height is a `2^k`-scale value
//! behind 3-bit stored deltas. Every fold here therefore splits its
//! running quantity into anchored components folded narrow — the
//! anchored-segment split the pair co-sweep section derives, which the
//! rank fold runs on its one stream, and the epoch-ledger form the
//! min_ticks fold runs (the `web` submodule) — with one shared trigger:
//!
//! A freeze fires exactly when a folded delta leaves the live component
//! more than `FREEZE_ALLOWANCE_DIGITS` digits wider than that delta's
//! own code: stale wide drift is about to ride under cheaper codes, so
//! the sweep evicts it once — charged to the codes that built the drift,
//! which the freeze consumes and resets — and the cheap codes continue
//! on an emptied live component. Bounded oscillation at *any* width
//! keeps the live component within its own codes' width and never
//! freezes: every wide-tooth fold is paid by the tooth's own code, on
//! either side of any fixed width.
//!
//! # The pair co-sweep: distance and lag
//!
//! Both pair measures are integrals of a *directed functional* of the
//! running height difference `D = h_a − h_b` over the overlay's
//! elementary intervals, by the valuation identities:
//!
//! - distance: `rank(a ∨ b) − rank(a ∧ b) = ∫ (max − min) = ∫ |D|`;
//! - lag: `rank(a ∨ b) − rank(a) = ∫ (max − h_a) = ∫ (−D)⁺`.
//!
//! The co-sweep maintains `D` exactly as the comparison sweep does and
//! integrates `h* = σ·D`, where `σ ∈ {−1, 0, +1}` is the measure's
//! *orientation* at `sign(D)` (distance: the sign itself; lag: `−1`
//! where `D < 0`, else `0`), so `h*` is the nonnegative integrand of
//! the measure by construction. Per boundary, with `σ → σ′` and net
//! folded difference `dD`, the integrand moves by
//!
//! `dh* = (σ′ − σ) · D′ + σ · dD`
//!
//! — the `σ·dD` term re-folds the boundary's own codes (each consumed
//! delta enters `D` once and `h*` at most once, orientation being a
//! side swap), and the `(σ′ − σ)·D′` term materializes the difference
//! only at orientation changes, which in both measures require `D` to
//! have crossed, left, or entered zero at this boundary — so
//! `|D′| ≤ |dD|`, and the read (after the sign fold's collapse) is
//! priced by the codes just folded, the same argument the emission
//! sweep's side switch rests on.
//!
//! ## The anchored-segment freeze discipline
//!
//! A freeze must not settle evicted drift against its *absolute*
//! position: positions grow arbitrarily dense while the codes at hand
//! stay cheap. A single stream can alternate isolated wide drops with
//! unit drops down a spine (the freeze-position board family), firing a
//! freeze per block at ever-growing written position spans; the overlay
//! is worse — one operand's cheap boundaries fire freezes of drift the
//! other operand's wide codes deposited, at positions whose compacted
//! density neither operand's codes funded (on the two-operand jump comb
//! — a shared descent spine planting isolated position bits, then an
//! `m`-level comb where one operand's wide teeth cross the other's
//! near-flat band — every crest of `|D|` would pay a
//! drift-width × position-density product, superlinear in the packed
//! pair while each operand alone stays flat). The integral therefore
//! works in *anchored segments*: no correction in the steady state
//! multiplies by an absolute position. The integrand splits
//! `h* = B + P + L` (for rank, `h* = h` itself):
//!
//! - `L` (*live*): the drift since the last freeze. Each elementary
//!   interval adds
//!   `L · 2^(S − depth)` directly — O(`L`'s digits), bounded by the
//!   previous boundary's widest folded code plus the freeze allowance,
//!   and the trigger below empties `L` before a second unfunded
//!   interval could ride a stale width.
//! - `P` (*parked*): drift a freeze moved out of `L`, anchored at that
//!   freeze. A segment-mass accumulator sums the interval masses since
//!   `P`'s anchor; the next freeze (or the stream end) settles
//!   `P · segment` in one compacted product and re-anchors. The
//!   segment mass's nonzero span is the *depth variation inside the
//!   segment* — the dyadic positions' shared prefix never appears in
//!   it — so a crest settled one comb level later costs `P`'s width
//!   times O(1) digits however dense the absolute position is, and
//!   oscillating drift cancels digit-wise inside `P` instead of
//!   re-paying its width. The segment mass is read through the
//!   accumulator's write-watermark read (`sign_magnitude_shl`) and
//!   cleared by buffer replacement, so a segment parked deep in the
//!   stream costs its written span, never its scale.
//! - `B` (*base*): content anchored at position zero — the opening
//!   `h*` plateau, plus any `P` *promoted* down when incoming drift
//!   runs more than the allowance narrower than `P`. Promotion pays
//!   `P × position` once — the sweep's only absolute-position product,
//!   funded by the wide code that armed `P`, one promotion per arming
//!   — after which `B` closes in a single shifted add `B · 2^S`.
//!   Without promotion a wide `P` would re-settle its full width at
//!   every later narrow-drift freeze; with it, every settle's `P` is
//!   within the allowance of the drift the settling freeze itself
//!   parks.
//!
//! A freeze fires by the section-one relative trigger, with one
//! pair-specific difference of denomination: the check runs once per
//! boundary against the *boundary's* widest folded code, not per folded
//! delta. The behavior it buys is the same — bounded oscillation at any
//! width never freezes, and wide drift riding under cheaper codes is
//! parked at the first such code.
//!
//! ## Funding: the potential function and its arity
//!
//! The certificate is a **two-ledger potential, one ledger per
//! operand**: `Φ = Φ_a + Φ_b`, where folding a code of `w` digits from
//! operand `s` deposits `Θ(w)` into `Φ_s` (and each topology bit
//! deposits O(1)). The arity is the point: distance and lag are
//! two-stream operations, and a per-stream potential argument is sound
//! only if no charge draws on the ledger of an operand that did not
//! deposit — the hole the composed form fell into, where the meet's
//! emission re-coded one operand's width into switch jumps that the
//! integral then evicted at the other operand's cheap codes, priced by
//! a position density neither had funded. The rank fold is the
//! one-ledger, single-stream instance of the same integral (its
//! orientation is constantly `+1`), so its certificate is this one with
//! `Φ_b` empty. Every charge names its deposit:
//!
//! - folds into `D` and `L`, and the orientation-change read of `D′`:
//!   this boundary's own deposits (`|D′| ≤ |dD|` caps the read);
//! - the interval add of `L`: the deposit that last set `L`'s width —
//!   at most one interval rides between trigger checks;
//! - a settle `P · segment`: `P`'s width is within the allowance of
//!   the drift the settling freeze parks (else promotion fires first),
//!   so the product draws from the deposits that built that drift,
//!   times a segment span the segment's own topology deposits cover;
//! - a promotion `P × position`: once per arming, from the wide
//!   deposit that armed `P` past the allowance, at the position's
//!   compacted density.
//!
//! A cheap code from one operand can *fire* a freeze, but the work the
//! freeze performs is bounded by deposits from the codes that built the
//! state being moved — never by an absolute position the firing operand
//! chose. The honest residual: promotion pays position density once per
//! wide re-arm, and a settle pays within-segment depth variation; in
//! both, the measure's exact value embeds the product of a genuinely
//! wide plateau and its genuinely dense mass, so the work is
//! mandatory-class for any exact evaluation, and reaching it spends the
//! width and the variation in the input's own codes.
//!
//! # Cost
//!
//! Derived, with the constants pinned by the `skyline_rank_*`,
//! `skyline_min_ticks_*`, and `skyline_project_*` rows of the
//! resource-envelope suite (`tests/meter.rs`): the cursor scan, decode,
//! and fold bounds are the comparison sweep's; rank adds O(`L` digits)
//! per leaf — bounded by the freeze allowance plus the width of the
//! delta folded at the previous boundary, so each per-leaf add is paid
//! by the code that set `L`'s width — plus the co-sweep section's
//! certified freeze work (a settle per freeze at the parked width times
//! within-segment depth variation, a promotion once per wide arming;
//! the `skyline_flatness` module's freeze-position band holds the
//! many-freezes genre flat). Distance and lag (the `DISTANCE_*`/`LAG_*`
//! rows, plus the `skyline_flatness` module's jump-pair band) add, per
//! boundary, work bounded by the boundary's own folded codes — the
//! difference and integrand folds and the orientation-change read —
//! plus the same certified freeze work, and two topology-only pre-scans
//! for the overlay scale; transiently they hold the two cursor paths
//! and the integrator's accumulators, never an emitted stream.
//! min_ticks adds one fold into the range-minimum web's gap per delta,
//! O(1) web bookkeeping per node (a count bump at each close, a
//! boundary move at each pop), one settle per reign record at the
//! record's own funded width, and the epoch ledger's one product per
//! freeze at the evicted drift's width — the `web` submodule certifies
//! every charge (the `skyline_flatness` module's pure-comb and
//! reveal-comb bands hold the close-reveal genre flat in both width
//! currencies). Projection adds one height materialization
//! per ownership transition, priced by the code it emits. Transient
//! state is the cursor paths, the accumulators, min_ticks' compressed
//! difference stack, and — for projection — the output builder's
//! per-level bit stacks.
//!
//! # Testing
//!
//! The recursive tree oracle is the behavioral witness: every fold is
//! differentially pinned against it through the bridge (exact `Rank`
//! equality, exact count equality, byte-identical projection streams;
//! distance and lag re-derived from the oracle's join, meet, and rank
//! through the valuation identities) over the adversarial generator
//! families — the two version-pair families included — arbitrary
//! normal-form trees, organic op-trace histories, and the exhaustive
//! small scope. Distance and lag are additionally pinned digit-exact
//! against the composed forms (the emission sweep's join and meet
//! re-ranked, subtracted through `Rank::checked_sub`) — the same
//! identities on a code path the co-sweep shares nothing with past the
//! cursors; that pin lives in this module's own test suite, and the
//! cross-oracle triple (tree fold and function-space Riemann sums in one
//! body) is `version/tests.rs`'s `distance_and_lag_realize_both_oracles`.
//! Rank is additionally pinned against the semantic Riemann-sum oracle,
//! which shares no structure with the sweep. The resource envelopes are
//! the meter rows named above.

use core::cmp::Ordering;

use suanpan::{Accumulator, Limbs, UBig};

use crate::codec::{self, Base, BitCursor, Bits, BitsSlice, SliceCursor};
use crate::step;
use crate::Rank;

use super::build::SkylineBuilder;
use super::emit::signed_sum;
use super::sweep::{advance, fold, LeafCursor, Side, Step};
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
/// integrating the step function on the anchored-segment height split
/// (the module doc's pair-co-sweep section carries the algebra and the
/// funding certification; rank is its single-stream instance). Equal to
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
    // The single-stream instance of the anchored-segment integral: the
    // integrand is the height itself, opened at the first leaf's
    // absolute (the plateau anchored at position zero) and folded
    // delta-by-delta thereafter.
    let mut integral = Integrator::new();
    integral.open(&first);
    loop {
        let weight_shift = (max_depth - cursor.depth()) as u64;
        integral.interval(weight_shift);
        if cursor.done() {
            break;
        }
        let step = cursor.step(&mut integral.live, Side::A);
        integral.boundary(base_digits(&step.magnitude));
    }
    let (sign, num) = integral.finish(max_depth as u64);
    debug_assert_ne!(sign, Ordering::Less, "heights are nonnegative");
    Rank::from_raw(Base::from(num), scale)
}

/// Add (or, with `subtract`, remove) `factor · digits · 2^shift` in the
/// total: one `factor`-wide product per nonzero signed digit of the
/// compacted `digits` operand.
///
/// The `digits` operand's base-2^32 digits are compacted greedily into
/// balanced signed digits, so an all-ones run — the usual shape of a
/// dyadic mass — costs one subtract at its floor and one carry past its
/// top instead of a product per digit. The `shift` carries a `digits`
/// operand read out at a scale (a segment mass parked deep in the
/// stream) without ever materializing the scaled value.
fn mul_into(total: &mut Accumulator, factor: &Base, digits: &Base, shift: u64, subtract: bool) {
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
            total.add_magnitude_shl(&product, shift);
        } else {
            total.sub_magnitude_shl(&product, shift);
        }
    };
    let mut shift = shift;
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
    Limbs::new(&value.0)
        .flat_map(|limb| [(limb & 0xFFFF_FFFF) as u32, (limb >> 32) as u32])
        .collect()
}

/// The causal distance between the versions two skyline streams denote:
/// the rank of their symmetric difference.
///
/// One fused co-sweep integrating `|h_a − h_b|` over the overlay (the
/// module doc's pair-co-sweep section carries the algebra, the
/// anchored-segment freeze discipline, and the funding certification):
/// no join or meet stream is materialized and no per-operand rank is
/// recomputed. Equal to [`Version::distance`](crate::Version::distance)
/// exactly, and digit-exact against the composed
/// `rank(join) − rank(meet)`, which the differential suite pins.
///
/// # Panics
///
/// Panics on a non-canonical operand or a stream deeper than `u32::MAX`
/// levels, exactly as [`rank`](fn@rank) does.
pub fn distance(a: &BitsSlice, b: &BitsSlice) -> Rank {
    pair_integral(a, b, Measure::Distance)
}

/// How far the first stream's version lags behind the second's: the rank
/// of the history the second records that the first does not.
///
/// The same co-sweep as [`distance`](fn@distance) integrating the directed
/// functional `(h_b − h_a)⁺` instead of the symmetric `|h_a − h_b|`.
/// Equal to [`Version::lag`](crate::Version::lag) exactly, and
/// digit-exact against the composed `rank(join) − rank(a)`, which the
/// differential suite pins.
///
/// # Panics
///
/// Panics on a non-canonical operand or a stream deeper than `u32::MAX`
/// levels, exactly as [`rank`](fn@rank) does.
pub fn lag(a: &BitsSlice, b: &BitsSlice) -> Rank {
    pair_integral(a, b, Measure::Lag)
}

/// The directed functional of the running height difference
/// `D = h_a − h_b` that a pair co-sweep integrates.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Measure {
    /// `∫ |D|`: the symmetric difference's rank.
    Distance,
    /// `∫ (−D)⁺ = ∫ (h_b − h_a)⁺`: the history the second operand
    /// records that the first does not.
    Lag,
}

impl Measure {
    /// The integrand's orientation over an interval where `D` has this
    /// sign: the coefficient `σ ∈ {−1, 0, +1}` with integrand `σ·D`
    /// there.
    fn orientation(self, sign: Ordering) -> i8 {
        match (self, sign) {
            (Measure::Distance, Ordering::Greater) => 1,
            (_, Ordering::Less) => -1,
            _ => 0,
        }
    }
}

/// Run the pair co-sweep: one merge walk over both streams, integrating
/// the measure's functional of the running difference on the
/// anchored-segment split (the module doc's pair-co-sweep section).
///
/// # Panics
///
/// Panics on a non-canonical operand or a stream deeper than `u32::MAX`
/// levels, exactly as [`rank`](fn@rank) does.
fn pair_integral(a_bits: &BitsSlice, b_bits: &BitsSlice, measure: Measure) -> Rank {
    // The overlay's scale: elementary intervals nest inside both
    // operands' leaves, so the deepest one sits at the deeper operand's
    // maximum depth.
    let overlay_depth = max_depth(a_bits).max(max_depth(b_bits));
    let scale =
        u32::try_from(overlay_depth).expect("rank exponent overflows u32: stream deeper than 2^32");
    let (mut ca, a_first) = LeafCursor::open(a_bits);
    let (mut cb, b_first) = LeafCursor::open(b_bits);
    let mut diff = Accumulator::new();
    diff.add_magnitude(&a_first);
    diff.sub_magnitude(&b_first);
    let mut orient = measure.orientation(diff.sign());
    let mut integral = Integrator::new();
    if orient != 0 {
        // The opening plateau: `h* = σ·D = |D|` whenever `σ ≠ 0`,
        // anchored at position zero and priced by the two absolute
        // first codes (the sign read above has collapsed the spelling).
        let (_, opening) = diff.sign_magnitude();
        integral.open(&Base::from(opening));
    }
    loop {
        let weight_shift = (overlay_depth - ca.depth().max(cb.depth())) as u64;
        integral.interval(weight_shift);
        if ca.done() && cb.done() {
            break;
        }
        let (da, db) = advance(&mut ca, &mut cb, &mut diff);
        let new_orient = measure.orientation(diff.sign());
        if orient != 0 {
            // The `σ·dD` term: each side's consumed delta re-folds into
            // the integrand, oriented by `σ` — a side swap is exactly
            // the negation.
            for (side, step) in [(Side::A, &da), (Side::B, &db)] {
                if let Some(step) = step {
                    let toward = if orient > 0 { side } else { side.other() };
                    fold(&mut integral.live, toward, step.negative, &step.magnitude);
                }
            }
        }
        if new_orient != orient {
            integral.jump(new_orient - orient, &diff);
            orient = new_orient;
        }
        // The freeze trigger, relative to this boundary's own codes:
        // the widest magnitude folded here is what funds the next
        // interval's live add.
        let funded = da
            .iter()
            .chain(db.iter())
            .map(|step| base_digits(&step.magnitude))
            .max()
            .unwrap_or(1);
        integral.boundary(funded);
    }
    let (sign, total) = integral.finish(overlay_depth as u64);
    debug_assert_ne!(sign, Ordering::Less, "both pair measures are nonnegative");
    Rank::from_raw(Base::from(total), scale)
}

/// The anchored-segment integral of the co-sweep's nonnegative integrand
/// `h* = B + P + L` (the module doc's pair-co-sweep section derives the
/// split and certifies its funding).
struct Integrator {
    /// The running integral's raw numerator, at the overlay scale.
    total: Accumulator,
    /// `L`: the integrand's drift since the last freeze. Written by the
    /// sweep's folds directly; every other component is this
    /// integrator's own bookkeeping.
    live: Accumulator,
    /// `P`: drift parked by freezes, anchored at the last freeze.
    parked: Accumulator,
    /// The interval mass accumulated since `parked`'s anchor.
    seg: Accumulator,
    /// `B`: content anchored at position zero — the opening plateau and
    /// every promotion — closing as `B · 2^S`.
    base: Accumulator,
    /// The absolute interval mass consumed through the last settled
    /// segment, read only at promotions.
    ///
    /// Fed one compacted segment mass per freeze — the same watermark
    /// read the settle already pays — never per interval, so a sweep
    /// that never freezes never touches it.
    position: Accumulator,
    /// The unit mass every interval deposits at its own scale.
    one: Base,
}

impl Integrator {
    fn new() -> Integrator {
        Integrator {
            total: Accumulator::new(),
            live: Accumulator::new(),
            parked: Accumulator::new(),
            seg: Accumulator::new(),
            base: Accumulator::new(),
            position: Accumulator::new(),
            one: Base::from(1u8),
        }
    }

    /// Anchor the opening plateau at position zero.
    fn open(&mut self, opening: &Base) {
        self.base.add_magnitude(opening);
    }

    /// Credit one elementary interval: the live component's contribution
    /// at the interval's mass, and the mass itself into the segment sum.
    fn interval(&mut self, weight_shift: u64) {
        // The zero test is one-sided (true means zero, false means
        // unknown), which is all this skip needs: a redundantly spelled
        // zero takes the add and contributes nothing.
        if !self.live.is_literally_zero() {
            self.total.add_accum_shl(&self.live, weight_shift);
        }
        self.seg.add_magnitude_shl(&self.one, weight_shift);
    }

    /// Fold the orientation-change term `(σ′ − σ) · D′` into the live
    /// component.
    ///
    /// Called only when the orientation moved at this boundary, which
    /// bounds `|D′|` by the deltas the boundary folded; the sign read
    /// that decided the new orientation has already collapsed the
    /// difference's spelling, so the read is priced by those same
    /// codes.
    fn jump(&mut self, coefficient: i8, diff: &Accumulator) {
        let (sign, magnitude) = diff.sign_magnitude();
        if magnitude == UBig::ZERO {
            return;
        }
        let magnitude = Base::from(magnitude);
        let negative = (coefficient < 0) != (sign == Ordering::Less);
        let shift = if coefficient.abs() == 2 { 1 } else { 0 };
        if negative {
            self.live.sub_magnitude_shl(&magnitude, shift);
        } else {
            self.live.add_magnitude_shl(&magnitude, shift);
        }
    }

    /// The end-of-boundary trigger: park the live drift when this
    /// boundary's folds left it more than the allowance wider than the
    /// widest code folded here.
    fn boundary(&mut self, funded_digits: usize) {
        if self.live.digit_count() > funded_digits + FREEZE_ALLOWANCE_DIGITS {
            self.freeze();
        }
    }

    /// Park the live drift, closing the current segment.
    ///
    /// Settles the parked component over the segment (banking the
    /// segment's mass into the absolute position), promotes the parked
    /// component first if the incoming drift runs far narrower, then
    /// moves the drift in and re-anchors.
    fn freeze(&mut self) {
        let (drift_sign, drift) = self.live.sign_magnitude();
        if drift == UBig::ZERO {
            // A redundantly spelled zero tripped the width trigger:
            // there is no drift to park — empty the spelling and keep
            // the current segment open.
            self.live.reset();
            return;
        }
        let drift = Base::from(drift);
        self.settle_segment();
        if self.parked.digit_count() > base_digits(&drift) + FREEZE_ALLOWANCE_DIGITS {
            self.promote();
        }
        match drift_sign {
            Ordering::Less => self.parked.sub_magnitude(&drift),
            _ => self.parked.add_magnitude(&drift),
        }
        self.live.reset();
        // A fresh buffer, not `reset()`: the segment's digits sit at the
        // sweep position's scale, and a clearing scan would pay the
        // untouched zero prefix below them; replacing the buffer opens
        // the next segment in O(1).
        self.seg = Accumulator::new();
    }

    /// Close the current segment at a freeze: credit the parked
    /// component over it and bank the segment's mass.
    ///
    /// The credit is `total += P · segment`, as [`settle`](Self::settle);
    /// the banked mass joins the absolute position, read only at
    /// promotions — one watermark read serving both consumers, priced
    /// by the segment's depth variation.
    fn settle_segment(&mut self) {
        let (seg_sign, seg_mag, seg_shift) = self.seg.sign_magnitude_shl();
        debug_assert_ne!(seg_sign, Ordering::Less, "interval masses only accumulate");
        if seg_mag == UBig::ZERO {
            return;
        }
        let seg = Base::from(seg_mag);
        self.position.add_magnitude_shl(&seg, seg_shift);
        if self.parked.is_literally_zero() {
            return;
        }
        let (p_sign, p_mag) = self.parked.sign_magnitude();
        if p_mag == UBig::ZERO {
            return;
        }
        mul_into(
            &mut self.total,
            &Base::from(p_mag),
            &seg,
            seg_shift,
            p_sign == Ordering::Less,
        );
    }

    /// Credit the parked component over the final segment at the sweep's
    /// close: `total += P · segment`.
    ///
    /// One compacted product priced by `P`'s width times the segment's
    /// depth variation; the scaled read skips the never-written scale
    /// prefix under the segment. No position banking: the sweep is over,
    /// so no promotion can follow.
    fn settle(&mut self) {
        if self.parked.is_literally_zero() {
            return;
        }
        let (p_sign, p_mag) = self.parked.sign_magnitude();
        if p_mag == UBig::ZERO {
            return;
        }
        let (seg_sign, seg_mag, seg_shift) = self.seg.sign_magnitude_shl();
        debug_assert_ne!(seg_sign, Ordering::Less, "interval masses only accumulate");
        mul_into(
            &mut self.total,
            &Base::from(p_mag),
            &Base::from(seg_mag),
            seg_shift,
            p_sign == Ordering::Less,
        );
    }

    /// Re-anchor the parked component at position zero: the base picks
    /// it up (closing as `B · 2^S`) and the total is debited
    /// `P × position` — the sweep's one absolute-position product, paid
    /// once per wide arming.
    ///
    /// Sound only immediately after
    /// [`settle_segment`](Self::settle_segment): the segment credit
    /// covered `P` up to the current position — which the banking has
    /// just brought current — so its remaining tail is
    /// `P · (2^S − position) = P · 2^S − P · position`.
    fn promote(&mut self) {
        let (p_sign, p_mag) = self.parked.sign_magnitude();
        if p_mag != UBig::ZERO {
            let (pos_sign, pos_mag, pos_shift) = self.position.sign_magnitude_shl();
            debug_assert_eq!(
                pos_sign,
                Ordering::Greater,
                "a freeze always follows at least one interval"
            );
            mul_into(
                &mut self.total,
                &Base::from(p_mag),
                &Base::from(pos_mag),
                pos_shift,
                p_sign == Ordering::Greater,
            );
            self.base.add_accum(&self.parked);
        }
        self.parked.reset();
    }

    /// Close the sweep: the final segment settlement, then the base's
    /// whole-interval term `B · 2^S`.
    ///
    /// The parked component's final segment mass is exactly the tail
    /// from its anchor, because the interval masses tile the unit
    /// interval. The live component owes nothing here: every interval
    /// already credited it directly.
    fn finish(mut self, closing_shift: u64) -> (Ordering, UBig) {
        self.settle();
        if !self.base.is_literally_zero() {
            self.total.add_accum_shl(&self.base, closing_shift);
        }
        self.total.sign_magnitude()
    }
}

/// The minimum number of ticks that could have produced the version a
/// skyline stream denotes, exact at any magnitude.
///
/// Folds `Σ leaf heights − Σ internal-node subtree minima` (each
/// normal-form base is its node's minimum less its parent's, so the sum
/// telescopes to exactly the stored-base total) over one leaf sweep.
/// Heights enter the total as narrow live offsets over a frozen
/// component that lives entirely in an epoch ledger — one drift per
/// freeze, settled against per-epoch reference counts once, at the end
/// — and the closing nodes' minima ride a range-minimum anchor web
/// whose closes count against reigning value records instead of folding
/// widths (the `web` submodule carries both structures, the accounting,
/// and the funding certificate). Equal to
/// [`Version::min_ticks`](crate::Version::min_ticks) on the decoded
/// version, which the differential suite pins exactly.
///
/// # Panics
///
/// Panics if the operand is not a canonical skyline stream — run
/// [`validate`](fn@super::validate) first on untrusted bytes.
pub fn min_ticks(bits: &BitsSlice) -> Base {
    let (mut cursor, first) = LeafCursor::open(bits);
    // The height split: `h = F + L`, with `L` folding every delta and
    // `F` living entirely in the epoch ledger — one drift per freeze,
    // settled against per-epoch reference counts once, at the end. The
    // first leaf's absolute is epoch 0's drift.
    let mut live = Accumulator::new();
    // The narrow side of the total: `Σ leaf offsets − Σ minima offsets`,
    // every term relative to its own epoch's frozen component.
    let mut total = Accumulator::new();
    let mut ledger = web::EpochLedger::new(first);
    // The minima side: subtree spans nest LIFO along the sweep, so each
    // closing node's minimum is the innermost open range's — the
    // range-minimum web (the `web` module carries the discipline and
    // the funding argument).
    let mut web = web::MinWeb::new();
    web.open(cursor.depth());
    ledger.leaf_ref();
    web.leaf(false, &Base::ZERO, 0, &mut total, &mut ledger);
    while !cursor.done() {
        let depth_before = cursor.depth();
        let step = cursor.step(&mut live, Side::A);
        web.fold_height(step.negative, &step.magnitude);
        // Every popped right-branch level closed one internal node:
        // its subtree minimum folds into the total (a count on the
        // web's reigning record) and merges into its parent.
        for _ in 0..depth_before - step.flip {
            web.close(&mut total, &mut ledger);
        }
        // Every left branch the descent pushed opened one node's range.
        web.open(cursor.depth() - step.flip);
        // The new leaf: a stale-wide live component is evicted first,
        // so the offset entering the total is paid by the codes that
        // built it (the freeze discipline's funding argument).
        if live.digit_count() > base_digits(&step.magnitude) + FREEZE_ALLOWANCE_DIGITS {
            ledger.freeze(&mut live);
        }
        let (l_sign, l_mag) = live.sign_magnitude();
        let leaf_off = Base::from(l_mag);
        let leaf_neg = l_sign == Ordering::Less;
        fold_signed(&mut total, leaf_neg, &leaf_off);
        ledger.leaf_ref();
        web.leaf(leaf_neg, &leaf_off, ledger.epoch(), &mut total, &mut ledger);
    }
    // The final leaf closes every remaining ancestor from the right,
    // then the ledger folds the frozen component's every reference.
    web.drain(&mut total, &mut ledger);
    ledger.settle(&mut total);
    let (sign, magnitude) = total.sign_magnitude();
    debug_assert_ne!(
        sign,
        Ordering::Less,
        "a subtree minimum never exceeds its leaves"
    );
    Base::from(magnitude)
}

/// Fold a signed magnitude into an accumulator.
fn fold_signed(acc: &mut Accumulator, negative: bool, magnitude: &Base) {
    if negative {
        acc.sub_magnitude(magnitude);
    } else {
        acc.add_magnitude(magnitude);
    }
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
    let mut height = Accumulator::new();
    height.add_magnitude(&first);
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
fn absolute_height(height: &mut Accumulator) -> Base {
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
    height: &mut Accumulator,
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
///
/// Shared with the masked comparison co-walk ([`super::masked`]), which
/// runs the same overlay bookkeeping against up to two of these cursors.
pub(super) struct IdLeafCursor<'a> {
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
    pub(super) fn open(bits: &'a BitsSlice) -> Self {
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
    pub(super) fn depth(&self) -> usize {
        self.path.len()
    }

    /// Whether the current region is owned by the id.
    pub(super) fn owned(&self) -> bool {
        self.owned
    }

    /// Whether the current region is the stream's last (its interval
    /// ends at the unit interval's right edge).
    pub(super) fn done(&self) -> bool {
        self.lefts == 0
    }

    /// Advance past the current region to the next, returning the flip
    /// level's depth for the caller's tie test.
    ///
    /// # Panics
    ///
    /// Panics if the stream is not a canonical packed id. Never called
    /// on a final region (the overlay stops when both cursors are done).
    pub(super) fn step(&mut self) -> usize {
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

mod web;

#[cfg(test)]
mod tests;
