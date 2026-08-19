//! The query folds over skyline streams: rank, distance, lag, min_ticks, and
//! projection from single leaf sweeps, never reconstructing absolute heights.
//!
//! Every fold here is a linear functional or a masking of the version's step
//! function, so each rides the same machinery as the comparison sweep — one
//! forward pass of its leaf cursors with the running height state on the
//! carry-cliff-free [`Accumulator`] (amortized O(1) digit touches where a
//! plain big integer pays each full carry) — plus the piece its own question
//! needs:
//!
//! - [`rank`](fn@rank) integrates the step function: `Σ heightᵢ · 2^(−depthᵢ)`
//!   over the leaves, telescoped through height *deltas* so no absolute
//!   height is ever rebuilt per leaf — the single-stream instance of the
//!   anchored-segment split (the [`integral`] submodule).
//! - [`distance`](fn@distance) and [`lag`](fn@lag) integrate a directed functional of
//!   the two operands' height difference in one fused co-sweep —
//!   distance = `∫ |h_a − h_b|`, lag = `∫ (h_b − h_a)⁺` — on the
//!   comparison sweep's merge walk, with no join or meet stream
//!   materialized and no per-operand rank recomputed (the [`integral`]
//!   submodule carries the algebra, the anchored-segment freeze
//!   discipline, and the funding certification).
//! - [`rank_cmp`](fn@rank_cmp) orders two versions' ranks with no
//!   `Rank` materialized: the *signed* instance of the same co-sweep —
//!   `rank(a) − rank(b) = ∫ (h_a − h_b)`, orientation constantly `+1`,
//!   so no orientation change ever fires — keeping only the exact
//!   total's sign.
//! - [`min_ticks`](fn@min_ticks) folds the identity
//!   `Σ bases = Σ leaf heights − Σ internal-node subtree minima` (each
//!   normal-form base is its node's subtree minimum less its parent's)
//!   exactly, at any magnitude: heights and minima enter the total as
//!   narrow epoch-relative offsets, the frozen component arrives
//!   through counting, and the closing nodes' minima ride a
//!   range-minimum anchor web whose closes count instead of fold — how
//!   a close can *count* (one count on the web's reigning record, the
//!   record settling once when it dies) is the `web` submodule's module
//!   doc, which carries the accounting and its funding certificate.
//! - [`project`](fn@project) overlays the skyline against a packed *id* stream
//!   and re-emits the masked skyline through the collapsing output
//!   builder: owned regions keep their plateaus, unowned regions emit
//!   zero, and the absolute height is materialized only at ownership
//!   transitions — where the emitted code itself is that height, so the
//!   work is priced by the mandatory output (the comb × scattered-party
//!   cross is Θ(teeth · magnitude) output from linear input — one wide
//!   height per comb tooth — and this sweep is I/O-linear on it).
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
//! whose components the [`integral`] submodule mints and derives along with the
//! discipline and its funding; the min_ticks fold runs the epoch-ledger form,
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
//! min_ticks adds one fold into the range-minimum web's gap per delta, O(1) web
//! bookkeeping per node (a count bump at each close, a boundary move at each
//! pop), one settle per reign record at the record's own funded width, and the
//! epoch ledger's one product per freeze at the evicted drift's width — the
//! `web` submodule certifies every charge (the `skyline_flatness` module's
//! pure-comb and reveal-comb bands hold the close-reveal genre flat in both the
//! touch and limb counters).
//!
//! Projection adds one height materialization per ownership transition, priced
//! by the code it emits. Transient state is the cursor paths, the accumulators,
//! min_ticks' compressed difference stack, and — for projection — the output
//! builder's per-level bit stacks.
//!
//! # Testing
//!
//! The recursive tree oracle is the behavioral witness: every fold is
//! differentially pinned against it through the bridge (exact `Rank` equality,
//! exact count equality, byte-identical projection streams; distance and lag
//! re-derived from the oracle's join, meet, and rank through the valuation
//! identities) over the adversarial generator families — the two version-pair
//! families included — arbitrary normal-form trees, organic op-trace histories,
//! and the exhaustive small scope. Distance and lag are additionally pinned
//! digit-exact against the composed forms (the emission sweep's join and meet
//! re-ranked, subtracted through `Rank::checked_sub`) — the same identities on
//! a code path the co-sweep shares nothing with past the cursors; that pin
//! lives in this module's own test suite, and the cross-oracle triple (tree
//! fold and function-space Riemann sums beside the production sweep) is the
//! distance and lag descriptors in the pointwise differential table. Rank is
//! additionally pinned against
//! the semantic Riemann-sum oracle, which shares no structure with the sweep.
//! The resource envelopes are the meter rows named above.

// The module doc and the fold docs (`rank`, `distance`, `lag`, `rank_cmp`)
// cite the crate-private `integral` submodule's essay by intra-doc link so a
// rename cannot rot the prose (the internal doc build resolves every link); on
// the public build those links render as plain code spans — the items are
// private — which this allow accepts.
#![allow(rustdoc::private_intra_doc_links)]

use core::cmp::Ordering;

use suanpan::{Accumulator, UBig};

use crate::codec::{self, Base, BitsBuf, BitsView, Int};
use crate::Rank;

use self::integral::{int_digits, Integrator, FREEZE_ALLOWANCE_DIGITS};
use super::build::SkylineBuilder;
use super::overlay::{
    advance, advance_diff, fold, Crossed, IdLeafCursor, LeafCursor, OpenedPair, PlateauCursor, Side,
};
use super::signed::{fold_signed, fold_signed_int, gamma_code_int, signed_sum_int, Sign, Signed};
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
    let max_depth = max_depth(bits);
    // Depth counts levels of a stream held in memory, so it always fits the u64
    // rank exponent.
    let scale = max_depth;
    let (mut cursor, first) = LeafCursor::open(bits);
    // The single-stream instance of the anchored-segment integral: the
    // integrand is the height itself, opened at the first leaf's absolute (the
    // plateau anchored at position zero) and folded delta-by-delta thereafter.
    let mut integral = Integrator::new();
    integral.open(Sign::Positive, &first);
    loop {
        let weight_shift = max_depth - cursor.depth();
        integral.interval(weight_shift);
        if cursor.done() {
            break;
        }
        let (_, step) = cursor.step();
        fold(&mut integral.live, Side::A, step.sign, &step.magnitude);
        integral.boundary(int_digits(&step.magnitude));
    }
    let (sign, numerator) = integral.finish(max_depth);
    debug_assert_ne!(sign, Ordering::Less, "heights are nonnegative");
    Rank::from_raw(Base::from(numerator), scale)
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
    pair_integral(a, b, |sign| match sign {
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
    pair_integral(a, b, |sign| match sign {
        Ordering::Less => -1,
        _ => 0,
    })
}

/// Compare the exact ranks of the versions two skyline streams denote, with no
/// `Rank` materialized: one fused co-sweep integrating the *signed* difference
/// `h_a − h_b` and answering its total's sign.
///
/// The signed instance of the pair co-sweep (the module doc's σ table):
/// `rank(a) − rank(b) = ∫ (h_a − h_b)`, so the integral's sign is the rank
/// order, and the orientation is constantly `+1` — no orientation change ever
/// fires, exactly as in the single-stream rank fold, whose funding certificate
/// (the [`integral`] submodule) therefore covers this walk with both operand
/// ledgers live. Equal to `a.rank().cmp(&b.rank())` on the decoded versions,
/// which the differential suite pins.
///
/// # Panics
///
/// Panics on a non-canonical operand, exactly as [`rank`](fn@rank) does.
pub fn rank_cmp(a: BitsView<'_>, b: BitsView<'_>) -> Ordering {
    // `∫ D`, signed: σ is constantly `+1`, the total is
    // `rank(a) − rank(b)`, and only its sign is kept.
    pair_fold(a, b, |_| 1).0
}

/// Run the nonnegative pair co-sweep and normalize its raw total into a
/// [`Rank`]: the distance/lag entry into [`pair_fold`].
fn pair_integral(
    a_bits: BitsView<'_>,
    b_bits: BitsView<'_>,
    orientation: impl Fn(Ordering) -> i8,
) -> Rank {
    let (sign, total, scale) = pair_fold(a_bits, b_bits, orientation);
    debug_assert_ne!(
        sign,
        Ordering::Less,
        "a Rank is nonnegative: a directed measure's integral cannot come out negative"
    );
    Rank::from_raw(Base::from(total), scale)
}

/// Run the pair co-sweep: one merge walk over both streams, handing back the
/// raw total as `(sign, magnitude, scale)`.
///
/// `orientation` is the integrand family's one degree of freedom (the module
/// doc's σ table): handed `sign(D)`, it answers the coefficient `σ ∈ {−1, 0,
/// +1}`, and the walk integrates `σ·D` on the anchored-segment split (the
/// [`integral`] submodule). The contract is two clauses. σ depends on nothing
/// but the sign — so σ is constant on intervals of constant `D`-sign, which
/// is what prices every orientation change at the boundary that moved the
/// sign. And σ is **monotone non-decreasing in the sign** (every row of the
/// module doc's table is) — which makes every orientation-change term `(σ′ −
/// σ) · D′` a debit, the invariant [`Integrator::jump`]'s unconditional add
/// rests on. Each caller's closure is monomorphized, the
/// [`super::overlay::advance`] / [`crate::fold::balanced_try_fold`] spelling
/// for an open algebra over one fixed walk.
///
/// # Panics
///
/// Panics on a non-canonical operand, exactly as [`rank`](fn@rank) does.
fn pair_fold(
    a_bits: BitsView<'_>,
    b_bits: BitsView<'_>,
    orientation: impl Fn(Ordering) -> i8,
) -> (Ordering, UBig, u64) {
    // The overlay's scale: elementary intervals nest inside both operands'
    // leaves, so the deepest one sits at the deeper operand's maximum depth.
    // Depth counts levels of streams held in memory, so it always fits the u64
    // rank exponent.
    let overlay_depth = max_depth(a_bits).max(max_depth(b_bits));
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
        let (opening_sign, opening) = diff.sign_magnitude();
        let sign = Sign::from_is_negative(match opening_sign {
            Ordering::Greater => current_orientation < 0,
            Ordering::Less => current_orientation > 0,
            Ordering::Equal => false,
        });
        integral.open(sign, &Int::from_ubig(opening));
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
                    fold(&mut integral.live, toward, step.sign, &step.magnitude);
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
            .map(|step| int_digits(&step.magnitude))
            .max()
            .expect("the advance law steps at least one side per boundary");
        integral.boundary(funded);
    }
    let (sign, total) = integral.finish(overlay_depth);
    (sign, total, scale)
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
/// anchor web whose closes count against reigning value records instead of
/// folding widths (the `web` submodule carries both structures, the accounting,
/// and the funding certificate). Equal to
/// [`Version::min_ticks`](crate::Version::min_ticks) on the decoded version,
/// which the differential suite pins exactly.
///
/// # Panics
///
/// Panics if the operand is not a canonical skyline stream — run
/// [`validate`](fn@super::validate) first on untrusted bytes.
pub fn min_ticks(bits: BitsView<'_>) -> Base {
    let (mut cursor, first) = LeafCursor::open(bits);
    // The height split: `h = F + L`, with `L` folding every delta and `F`
    // living entirely in the epoch ledger — one drift per freeze, settled
    // against per-epoch reference counts once, at the end. The first leaf's
    // absolute is epoch 0's drift.
    let mut live = Accumulator::new();
    // The narrow side of the total: `Σ leaf offsets − Σ minima offsets`, every
    // term relative to its own epoch's frozen component.
    let mut total = Accumulator::new();
    let mut ledger = web::EpochLedger::new(first.into_base());
    // The minima side: subtree spans nest LIFO along the sweep, so each closing
    // node's minimum is the innermost open range's — the range-minimum web (the
    // `web` module carries the discipline and the funding argument).
    let mut web = web::ReignWeb::new();
    web.open(cursor.depth());
    ledger.leaf_ref();
    web.leaf(Sign::Positive, &Base::ZERO, 0, &mut total, &mut ledger);
    while !cursor.done() {
        let depth_before = cursor.depth();
        let (flip, step) = cursor.step();
        fold(&mut live, Side::A, step.sign, &step.magnitude);
        web.fold_height(step.sign, &step.magnitude);
        // Every popped right-branch level closed one internal node: its subtree
        // minimum folds into the total (a count on the
        // web's reigning record) and merges into its parent.
        for _ in 0..depth_before - flip {
            web.close(&mut total, &mut ledger);
        }
        // Every left branch the descent pushed opened one node's range.
        web.open(cursor.depth() - flip);
        // The new leaf: a stale-wide live component is evicted first, so the
        // offset entering the total is paid by the codes that built it (the
        // freeze discipline's funding argument).
        if live.digit_count() > int_digits(&step.magnitude) + FREEZE_ALLOWANCE_DIGITS {
            ledger.freeze(&mut live);
        }
        let (live_sign, live_magnitude) = live.sign_magnitude();
        let leaf_offset = Base::from(live_magnitude);
        let leaf_sign = Sign::from_is_negative(live_sign == Ordering::Less);
        fold_signed(&mut total, leaf_sign, &leaf_offset);
        ledger.leaf_ref();
        web.leaf(
            leaf_sign,
            &leaf_offset,
            ledger.epoch(),
            &mut total,
            &mut ledger,
        );
    }
    // The final leaf closes every remaining ancestor from the right, then the
    // ledger folds the frozen component's every reference.
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

/// Project the version a skyline stream denotes onto a packed id's owned
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
    fold_signed_int(&mut height, Sign::Positive, &first);
    let mut owned = id_cursor.owned();
    // Allocation-strategy seam: the shipped arm pre-sizes to the operands'
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
    let opening = if owned { first } else { Int::ZERO };
    out.leaf(
        event_cursor.depth().max(id_cursor.depth()),
        gamma_code_int(&opening),
    );
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
                fold(&mut height, Side::A, step.sign, &step.magnitude);
                event_cursor.skip_deeper(flip, &mut height);
                out.leaf(
                    flip,
                    super::signed::gamma_code_signed_int(Sign::Positive, &Int::ZERO),
                );
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
                fold(&mut height, Side::A, step.sign, &step.magnitude);
            }
        });
        let now_owned = id_cursor.owned();
        let (sign, magnitude) = match (owned, now_owned) {
            // Inside an owned run the output moves with the skyline; a boundary
            // the id alone crossed is a zero delta.
            (true, true) => match &ev_step {
                Some(step) => (step.sign, step.magnitude.clone()),
                None => (Sign::Positive, Int::ZERO),
            },
            (false, false) => (Sign::Positive, Int::ZERO),
            // Entering the owned region: the output jumps to the current
            // absolute height.
            (false, true) => (Sign::Positive, Int::from_base(absolute_height(&mut height))),
            // Leaving it: the output drops from the height *before* this
            // boundary's fold — the new height minus the folded delta.
            (true, false) => {
                let now = Int::from_base(absolute_height(&mut height));
                let before = match &ev_step {
                    Some(step) => {
                        signed_sum_int(Sign::Positive, now, step.sign.negate(), &step.magnitude)
                    }
                    None => Signed {
                        sign: Sign::Positive,
                        magnitude: now,
                    },
                };
                debug_assert!(!before.sign.is_negative(), "heights are nonnegative");
                let sign = if before.magnitude.is_zero() {
                    Sign::Positive
                } else {
                    Sign::Negative
                };
                (sign, before.magnitude)
            }
        };
        owned = now_owned;
        out.leaf(
            event_cursor.depth().max(id_cursor.depth()),
            super::signed::gamma_code_signed_int(sign, &magnitude),
        );
    }
    let bits = out.finish();
    // Allocation-strategy arm (bench-only, as the seam above): one exact-size
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

/// The current absolute height, materialized at an ownership transition.
///
/// The sign fold's collapse compacts the accumulator first, so the read is
/// O(the height's own digits) — priced by the transition code the caller emits.
fn absolute_height(height: &mut Accumulator) -> Base {
    let sign = height.sign();
    debug_assert_ne!(sign, Ordering::Less, "heights are nonnegative");
    let (_, magnitude) = height.sign_magnitude();
    Base::from(magnitude)
}

/// The maximum leaf depth of a skyline stream: one topology-only pre-scan,
/// payload codes skipped unread.
///
/// # Panics
///
/// Panics if the stream is not a canonical skyline encoding.
fn max_depth(bits: BitsView<'_>) -> u64 {
    let mut cursor = codec::DsiCursor::new(bits);
    let mut deepest = 0u64;
    let mut walk = LeafWalk::new();
    while let Some(depth) = walk.descend(&mut cursor) {
        deepest = deepest.max(depth);
        // `skip_int` records the skipped code's full width itself, so the
        // pre-scan's payload skips carry exactly one scan record each.
        cursor.skip_int().expect("canonical skyline bits");
    }
    deepest
}

pub(crate) mod integral;
mod web;

#[cfg(test)]
mod tests;
