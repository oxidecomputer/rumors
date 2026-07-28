//! Differential pins for the query folds against the recursive oracle
//! and the composed forms.
//!
//! The recursive tree oracle (through the bridge) is the behavioral
//! witness over the adversarial families, arbitrary trees, organic
//! histories, and the exhaustive small scope: rank and min_ticks by its
//! own folds, projection by its mask, distance and lag re-derived from
//! its join, meet, and rank through the valuation identities the rustdoc
//! states. Distance and lag are additionally pinned against the composed
//! forms — the same identities assembled from this crate's own emission
//! and rank kernels — and the pair sweeps include the two version-pair
//! families (the two-operand jump comb, the concurrent pair), both as
//! deterministic dimension sweeps and as proptests over their generator
//! dimensions. Rank is additionally pinned against the semantic
//! Riemann-sum oracle, which shares no structure with either
//! implementation.
//!
//! Every equality here is exact — `Rank` equality is structural on the
//! normalized form, projection agreement is byte identity of the emitted
//! canonical streams — so a fold that drifts by any amount anywhere has
//! no rounding to hide behind.

use proptest::prelude::*;

use crate::meter::{
    alt_spine, bigroot, cancelling_chain, cliff_comb, cliff_fan, concurrent_pair, dense, harmonic,
    hugeleaf, jump_comb, jump_pair, wide_tooth_comb, Packed,
};
use crate::testing::bridge::{from_oracle_version, to_oracle_party, to_oracle_version};
use crate::testing::exhaustive::{all_normal_events, all_normal_ids, EV_SMALL_DEPTH};
use crate::testing::generators::arb_oracle_version;
use crate::testing::{optrace, semantic_oracle};
use crate::version::skyline::{emit, encode};
use crate::{Clock, Party, Version};

use super::{distance, lag, min_ticks, project, rank};

/// Decode a meter-generated packed shape as a [`Version`].
fn version_of(p: &Packed) -> Version {
    p.version()
}

/// Assert the single-operand folds against the recursive tree oracle's
/// own folds.
fn assert_single(v: &Version) {
    let enc = encode(v);
    let tree = to_oracle_version(v);
    assert_eq!(
        rank(&enc),
        tree.rank(),
        "rank kernel disagrees with the tree-fold oracle: {v}"
    );
    assert_eq!(
        crate::Ticks(min_ticks(&enc)),
        tree.min_ticks(),
        "min_ticks kernel disagrees with the tree-fold oracle: {v}"
    );
}

/// Assert the projection kernel against the recursive oracle's mask for
/// one `(version, party)` operand pair: byte identity of the canonical
/// streams.
fn assert_projection(v: &Version, p: &Party) {
    let enc = encode(v);
    let masked = from_oracle_version(&to_oracle_version(v).project(&to_oracle_party(p)));
    assert_eq!(
        project(&enc, p),
        encode(&masked),
        "projection must match the oracle mask: {v} / {p}"
    );
}

/// Assert the pair folds against the recursive oracle *and* the composed
/// forms.
///
/// Two independent witnesses per measure, both exact:
///
/// - **The paper oracle**: each measure re-derived from the recursive
///   oracle's join, meet, and rank through the valuation identities the
///   rustdoc states — `distance = rank(a ∨ b) − rank(a ∧ b)` and
///   `lag(a, b) = rank(a ∨ b) − rank(a)`.
/// - **The composed forms**: the same identities assembled from this
///   crate's own kernels — the emission sweep's join/meet streams, the
///   rank fold over them, and [`Rank::checked_sub`] — so the pair
///   measures are pinned digit-exact against rank-of-meet arithmetic
///   computed along a code path they do not share.
fn assert_pair(a: &Version, b: &Version) {
    let (ea, eb) = (encode(a), encode(b));
    let (ta, tb) = (to_oracle_version(a), to_oracle_version(b));
    let join_rank = (ta.clone() | tb.clone()).rank();
    let meet_rank = (ta.clone() & tb.clone()).rank();
    let dist = join_rank
        .checked_sub(&meet_rank)
        .expect("rank is monotone: the meet's rank never exceeds the join's");
    assert_eq!(distance(&ea, &eb), dist, "distance: {a} vs {b}");
    assert_eq!(distance(&eb, &ea), dist, "distance: {b} vs {a}");
    let lag_a = join_rank
        .checked_sub(&ta.rank())
        .expect("rank is monotone: an operand's rank never exceeds the join's");
    let lag_b = join_rank
        .checked_sub(&tb.rank())
        .expect("rank is monotone: an operand's rank never exceeds the join's");
    assert_eq!(lag(&ea, &eb), lag_a, "lag: {a} vs {b}");
    assert_eq!(lag(&eb, &ea), lag_b, "lag: {b} vs {a}");
    // The composed forms, on this crate's own kernels.
    let kernel_join = rank(&emit::join(&ea, &eb));
    let kernel_meet = rank(&emit::meet(&ea, &eb));
    let composed_dist = kernel_join
        .checked_sub(&kernel_meet)
        .expect("rank is monotone: the meet's rank never exceeds the join's");
    assert_eq!(
        distance(&ea, &eb),
        composed_dist,
        "distance vs the composed rank-of-meet arithmetic: {a} vs {b}"
    );
    let composed_lag_a = kernel_join
        .checked_sub(&rank(&ea))
        .expect("rank is monotone: an operand's rank never exceeds the join's");
    assert_eq!(
        lag(&ea, &eb),
        composed_lag_a,
        "lag vs the composed rank-of-join arithmetic: {a} vs {b}"
    );
}

/// The adversarial family pool the deterministic sweeps run over.
fn family_pool() -> Vec<Version> {
    vec![
        Version::new(),
        version_of(&dense(1)),
        version_of(&dense(2)),
        version_of(&dense(64)),
        version_of(&bigroot(7, 3)),
        version_of(&bigroot(64, 16)),
        version_of(&hugeleaf(1)),
        version_of(&hugeleaf(64)),
        version_of(&cliff_comb(3, 2)),
        version_of(&cliff_comb(16, 16)),
        version_of(&wide_tooth_comb(16, 8, 8)),
        // Wide teeth over the freeze allowance: bounded oscillation that
        // must ride the live component without freezing.
        version_of(&wide_tooth_comb(320, 300, 6)),
        // The stale-drift shape: the mid-stream jump is wide enough that
        // the first cheap delta behind it fires a freeze.
        version_of(&jump_comb(16, 8)),
        version_of(&jump_comb(320, 4)),
        version_of(&cliff_fan(16, 8)),
        version_of(&cancelling_chain(16, 8)),
        version_of(&alt_spine(3)),
        version_of(&alt_spine(64)),
        version_of(&harmonic(16)),
    ]
}

/// The id operand pool for projection: the whole interval, one half, a
/// quarter, and the scattered fragments that keep every other comb tooth.
fn party_pool() -> Vec<Party> {
    let mut seed = Party::seed();
    let mut half = seed.fork();
    let quarter = half.fork();
    vec![
        seed,
        half,
        quarter,
        Party::decode(&crate::meter::scattered_id(1).bytes[..])
            .expect("scattered id is strict normal form"),
        Party::decode(&crate::meter::scattered_id(9).bytes[..])
            .expect("scattered id is strict normal form"),
    ]
}

/// Every adversarial family shape agrees with the tree-fold oracle's
/// rank and min_ticks; crosses and pairs agree on projection, distance,
/// and lag.
///
/// The id-pool cross checks the oracle's projection mask; the ordered
/// pairs check distance and lag as re-derived from the oracle's
/// lattice folds.
///
/// The families are exactly the shapes whose costs the meter rows pin —
/// carry cliffs, wide teeth, cancelling chains, harmonic spines — so a
/// height-tracking or freeze bookkeeping error surfaces here before any
/// envelope moves.
#[test]
fn families_agree_with_the_packed_forms() {
    let pool = family_pool();
    let parties = party_pool();
    for v in &pool {
        assert_single(v);
        for p in &parties {
            assert_projection(v, p);
        }
    }
    for a in &pool {
        for b in &pool {
            assert_pair(a, b);
        }
    }
}

/// The two version-pair families agree with the oracle and the composed
/// forms on distance and lag, at their own constructed pairings.
///
/// The jump-pair dimensions straddle the freeze allowance (`k = 3` up to
/// `k = 512`, past the 256-bit digit bound), so the pair measures are
/// pinned on shapes where the difference's live component rides wide
/// drift across the other operand's cheap boundaries — the interleaving
/// the family exists to reach; the concurrent pair pins the side-switch
/// density population, where the difference's sign flips at every one of
/// the `n − 1` overlay boundaries.
#[test]
fn pair_families_agree() {
    for (k, m, d) in [(3, 1, 1), (16, 4, 2), (320, 6, 3), (512, 8, 2)] {
        let (pa, pb) = jump_pair(k, m, d);
        assert_pair(&version_of(&pa), &version_of(&pb));
    }
    for n in [2, 4, 16, 64] {
        let (v, w) = concurrent_pair(n);
        assert_pair(&v, &w);
    }
}

/// The exhaustive small scope: every normal-form event tree to the small
/// depth agrees on rank and min_ticks, and every tree × normal-form id
/// agrees on projection — every boundary genre by brute force rather
/// than sampling.
#[test]
fn exhaustive_small_scope_agrees() {
    let events: Vec<Version> = all_normal_events(EV_SMALL_DEPTH)
        .iter()
        .map(from_oracle_version)
        .collect();
    let ids: Vec<Party> = all_normal_ids(2)
        .iter()
        .map(crate::testing::bridge::from_oracle_party)
        .collect();
    for v in &events {
        assert_single(v);
        for p in &ids {
            assert_projection(v, p);
        }
    }
}

/// Every *ordered pair* of normal-form event trees to the small depth
/// agrees between the pair co-sweep and the composed forms: distance
/// equals `rank(join) − rank(meet)` and lag equals `rank(join) −
/// rank(a)`, digit-exact.
///
/// The total check over the pair space: every boundary genre the
/// comparison sweep's exhaustive suite reaches (aligned ties,
/// flush-right ties at unequal depths, plateau consumption, zero deltas
/// across subtree boundaries) crossed with every orientation schedule
/// reachable at this scope, by brute force rather than sampling. The
/// oracle leg over the same identities rides the family and proptest
/// sweeps; here the composed kernels are the witness so the quadratic
/// pair product stays fast.
#[test]
fn exhaustive_small_scope_pairs_agree() {
    let events: Vec<crate::codec::Bits> = all_normal_events(EV_SMALL_DEPTH)
        .iter()
        .map(|t| encode(&from_oracle_version(t)))
        .collect();
    for ea in &events {
        let rank_a = rank(ea);
        for eb in &events {
            let join = rank(&emit::join(ea, eb));
            let meet = rank(&emit::meet(ea, eb));
            let composed_dist = join
                .checked_sub(&meet)
                .expect("rank is monotone: the meet's rank never exceeds the join's");
            assert_eq!(
                distance(ea, eb),
                composed_dist,
                "distance at a small-scope pair"
            );
            let composed_lag = join
                .checked_sub(&rank_a)
                .expect("rank is monotone: an operand's rank never exceeds the join's");
            assert_eq!(lag(ea, eb), composed_lag, "lag at a small-scope pair");
        }
    }
}

proptest! {
    /// Arbitrary normal-form trees agree with the recursive oracle on
    /// every query fold.
    ///
    /// Rank is additionally realized by the semantic oracle's Riemann sum
    /// over the resolving grid — the geometric ground truth that shares
    /// no recursion, no delta, and no accumulator with either
    /// implementation.
    #[test]
    fn arbitrary_trees_agree(oa in arb_oracle_version(), ob in arb_oracle_version()) {
        let a = from_oracle_version(&oa);
        let b = from_oracle_version(&ob);
        assert_single(&a);
        assert_pair(&a, &b);
        let ev = semantic_oracle::lift_ev(oa);
        let g = semantic_oracle::ev_res(&ev);
        prop_assert_eq!(
            semantic_oracle::rank(&ev, g),
            rank(&encode(&a)),
            "the Riemann sum disagrees with the rank kernel: {}", a
        );
        prop_assert_eq!(
            semantic_oracle::min_ticks(&ev, g),
            min_ticks(&encode(&a)),
            "the semantic tick floor disagrees with the min_ticks kernel: {}", a
        );
    }

    /// One organic fork/tick/send/sync/join history agrees with the
    /// recursive oracle on every query fold.
    ///
    /// Projection runs onto the history's own parties — the operand
    /// pairing production code actually builds.
    #[test]
    fn organic_histories_agree(ops in optrace::world_strategy_up_to(40)) {
        let mut clocks = vec![Clock::seed()];
        for op in &ops {
            optrace::step_impl(&mut clocks, op);
        }
        for c in &clocks {
            assert_single(c.version());
            for other in &clocks {
                assert_projection(c.version(), other.party());
                assert_pair(c.version(), other.version());
            }
        }
    }

    /// Jump-comb pairs at arbitrary dimensions agree with the oracle and
    /// the composed forms on distance and lag.
    ///
    /// The tooth width `k` is drawn across the freeze allowance's 256-bit
    /// digit bound, so the sampled family covers both the
    /// bounded-oscillation regime (wide folds cancel adjacently, nothing
    /// freezes) and the wide-drift regime (the difference's live
    /// component crosses cheap boundaries wide), at varying comb depth
    /// `m` and spine density `d`.
    #[test]
    fn arbitrary_jump_pairs_agree(k in 3usize..400, m in 1usize..8, d in 1usize..4) {
        let (pa, pb) = jump_pair(k, m, d);
        assert_pair(&version_of(&pa), &version_of(&pb));
    }

    /// Concurrent pairs at arbitrary power-of-two fork widths agree with
    /// the oracle and the composed forms on distance and lag: the
    /// side-switch density family, where every overlay boundary flips
    /// which operand dominates.
    #[test]
    fn arbitrary_concurrent_pairs_agree(log_n in 1u32..7) {
        let (v, w) = concurrent_pair(1 << log_n);
        assert_pair(&v, &w);
    }

    /// The projection kernel agrees with the oracle's semantic mask over
    /// arbitrary tree × arbitrary id operands, where the id's absent
    /// children exercise the synthetic-empty arm at every depth.
    #[test]
    fn arbitrary_projections_agree(
        ov in arb_oracle_version(),
        oi in proptest::sample::select(all_normal_ids(3)),
    ) {
        let v = from_oracle_version(&ov);
        let p = crate::testing::bridge::from_oracle_party(&oi);
        assert_projection(&v, &p);
    }
}

/// The committed known-bad freeze accounting: the freeze-position
/// family's adequacy tripwire.
///
/// The anchored-segment integral exists because a freeze must not
/// settle evicted drift against its absolute position (the module doc's
/// discipline). This module keeps the refuted accounting — the
/// frozen/live split whose every freeze correction multiplies the drift
/// by the whole position accumulator, read across its full written span
/// — committed and *failing*: the tripwire proves `FP(k)` still catches
/// the mechanism red, so the family's green flatness band
/// (`skyline_rank_freeze_position_is_flat_per_unit`, `tests/meter.rs`)
/// is never decoration. The kernel is value-exact against the shipped
/// rank, so the demonstrator is a real implementation, not a strawman.
#[cfg(feature = "limb-meter")]
mod adequacy {
    use core::cmp::Ordering;

    use suanpan::{touch_meter, Accumulator};

    use crate::codec::{Base, BitsSlice};
    use crate::meter::freeze_position;
    use crate::version::skyline::encode;
    use crate::version::skyline::sweep::{LeafCursor, Side};
    use crate::Rank;

    use super::super::{base_digits, max_depth, mul_into, FREEZE_ALLOWANCE_DIGITS};

    /// The absolute-position rank fold: heights on a frozen/live split
    /// whose freeze correction is `drift × position` with the position
    /// accumulator read whole per freeze.
    ///
    /// Value-exact — the summation-by-parts identity
    /// `Σᵢ F(i)·massᵢ = F_final·2^S − Σ_freezes drift·position` is
    /// sound — and superlinear exactly where the tripwire asserts it:
    /// freeze `i`'s position read walks the accumulator's whole written
    /// span, which `FP(k)`'s descending spine grows with every block.
    fn absolute_position_rank(bits: &BitsSlice) -> Rank {
        let max_depth = max_depth(bits);
        let scale = u32::try_from(max_depth).expect("the tripwire streams stay shallow");
        let (mut cursor, first) = LeafCursor::open(bits);
        let mut total = Accumulator::new();
        let mut live_height = Accumulator::new();
        let mut frozen = Accumulator::new();
        frozen.add_magnitude(&first);
        let mut position = Accumulator::new();
        let one = Base::from(1u8);
        loop {
            let weight_shift = (max_depth - cursor.depth()) as u64;
            if !live_height.is_literally_zero() {
                total.add_accum_shl(&live_height, weight_shift);
            }
            position.add_magnitude_shl(&one, weight_shift);
            if cursor.done() {
                break;
            }
            let step = cursor.step(&mut live_height, Side::A);
            if live_height.digit_count() > base_digits(&step.magnitude) + FREEZE_ALLOWANCE_DIGITS {
                let (drift_sign, drift) = live_height.sign_magnitude();
                let (_, position_mag) = position.sign_magnitude();
                let drift = Base::from(drift);
                mul_into(
                    &mut total,
                    &drift,
                    &Base::from(position_mag),
                    0,
                    drift_sign == Ordering::Greater,
                );
                match drift_sign {
                    Ordering::Less => frozen.sub_magnitude(&drift),
                    _ => frozen.add_magnitude(&drift),
                }
                live_height = Accumulator::new();
            }
        }
        total.add_accum_shl(&frozen, max_depth as u64);
        let (sign, num) = total.sign_magnitude();
        debug_assert_ne!(sign, Ordering::Less, "heights are nonnegative");
        Rank::from_raw(Base::from(num), scale)
    }

    /// One tripwire run: packed bytes and the touch count over the
    /// known-bad fold, value-pinned against the shipped kernel.
    fn run(k: usize) -> (u64, u64) {
        let v = freeze_position(k).version();
        let enc = encode(&v);
        let expected = v.rank();
        touch_meter::reset();
        let r = absolute_position_rank(&enc);
        let touches = touch_meter::touches();
        assert_eq!(
            r, expected,
            "the known-bad fold must stay value-exact: a wrong demonstrator \
             proves nothing about the family's coverage"
        );
        (enc.len().div_ceil(8) as u64, touches)
    }

    /// `FP(k)` catches the absolute-position accounting red: its
    /// per-byte touch cost grows across the doubling.
    ///
    /// A linear fold reads ~x1.00 here; the floor 1.25 sits midway
    /// between linear and the measured x1.50, while the shipped
    /// kernel's flatness band holds the same family at x1.25.
    ///
    /// [measured 2026-07-28, dev profile, exact counters: touches
    /// 124,368 -> 372,859 across FP(1,000) -> FP(2,000), packed
    /// 73,328B -> 146,579B: per-byte growth x1.50.]
    #[test]
    fn absolute_position_accounting_reads_superlinear_on_freeze_position() {
        let (small_bytes, small_touches) = run(1_000);
        let (large_bytes, large_touches) = run(2_000);
        eprintln!(
            "MEASURED adequacy_absolute_position: small={small_touches}/{small_bytes}B \
             large={large_touches}/{large_bytes}B"
        );
        assert!(
            u128::from(large_touches) * u128::from(small_bytes) * 100
                >= u128::from(small_touches) * u128::from(large_bytes) * 125,
            "the absolute-position accounting reads flat on the freeze-position \
             family ({small_touches}/{small_bytes}B -> {large_touches}/{large_bytes}B): \
             the family no longer catches the mechanism it was built for, so the \
             flatness band it backs is decoration until a new witness lands"
        );
    }

    // ── the span-reading promotion accounting ──────────────────────────
    //
    // The promotion ledger exists because a promotion must not re-read
    // whole-history position state (the module doc's promotion-ledger
    // bullet). This kernel keeps the refuted accounting — the full
    // anchored-segment integrator whose promotion debits `P × position`
    // by reading an absolute position accumulator across its written
    // span, re-anchoring the parked component into the base — committed
    // and failing on the promotion re-arm family, through both the
    // single-stream and the pair integrals, so the green re-arm
    // flatness bands (`skyline_flatness`, `tests/meter.rs`) are never
    // decoration. Value-exact against the shipped folds: the identity
    // `P · (2^S − position) = P · 2^S − P · position` is sound; only
    // its cost class is not.

    use crate::meter::{promotion_rearm, promotion_rearm_mate};
    use crate::version::skyline::sweep::{advance, fold};

    /// The anchored-segment integral with the span-reading promotion.
    ///
    /// Segments settle at the write watermark (linear on the
    /// freeze-position family), but a promotion multiplies the parked
    /// component by the absolute position accumulator, read across its
    /// full written span, and re-anchors it into the base.
    struct SpanIntegrator {
        total: Accumulator,
        live: Accumulator,
        parked: Accumulator,
        seg: Accumulator,
        base: Accumulator,
        /// The absolute interval mass consumed through the last settled
        /// segment: the whole-history state the promotion re-reads.
        position: Accumulator,
        one: Base,
    }

    impl SpanIntegrator {
        fn new() -> SpanIntegrator {
            SpanIntegrator {
                total: Accumulator::new(),
                live: Accumulator::new(),
                parked: Accumulator::new(),
                seg: Accumulator::new(),
                base: Accumulator::new(),
                position: Accumulator::new(),
                one: Base::from(1u8),
            }
        }

        fn open(&mut self, opening: &Base) {
            self.base.add_magnitude(opening);
        }

        fn interval(&mut self, weight_shift: u64) {
            if !self.live.is_literally_zero() {
                self.total.add_accum_shl(&self.live, weight_shift);
            }
            self.seg.add_magnitude_shl(&self.one, weight_shift);
        }

        fn jump(&mut self, coefficient: i8, diff: &Accumulator) {
            let (sign, magnitude) = diff.sign_magnitude();
            if magnitude == suanpan::UBig::ZERO {
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

        fn boundary(&mut self, funded_digits: usize) {
            if self.live.digit_count() > funded_digits + FREEZE_ALLOWANCE_DIGITS {
                self.freeze();
            }
        }

        fn freeze(&mut self) {
            let (drift_sign, drift) = self.live.sign_magnitude();
            if drift == suanpan::UBig::ZERO {
                self.live.reset();
                return;
            }
            let drift = Base::from(drift);
            self.settle_segment();
            if self.parked.digit_count()
                > super::super::base_digits(&drift) + FREEZE_ALLOWANCE_DIGITS
            {
                self.promote();
            }
            match drift_sign {
                Ordering::Less => self.parked.sub_magnitude(&drift),
                _ => self.parked.add_magnitude(&drift),
            }
            self.live.reset();
            self.seg = Accumulator::new();
        }

        fn settle_segment(&mut self) {
            let (_, seg_mag, seg_shift) = self.seg.sign_magnitude_shl();
            if seg_mag == suanpan::UBig::ZERO {
                return;
            }
            let seg = Base::from(seg_mag);
            self.position.add_magnitude_shl(&seg, seg_shift);
            if self.parked.is_literally_zero() {
                return;
            }
            let (p_sign, p_mag) = self.parked.sign_magnitude();
            if p_mag == suanpan::UBig::ZERO {
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

        fn settle(&mut self) {
            if self.parked.is_literally_zero() {
                return;
            }
            let (p_sign, p_mag) = self.parked.sign_magnitude();
            if p_mag == suanpan::UBig::ZERO {
                return;
            }
            let (_, seg_mag, seg_shift) = self.seg.sign_magnitude_shl();
            mul_into(
                &mut self.total,
                &Base::from(p_mag),
                &Base::from(seg_mag),
                seg_shift,
                p_sign == Ordering::Less,
            );
        }

        /// The refuted move: `P × position` with the position read
        /// whole, then `P` re-anchored into the base.
        fn promote(&mut self) {
            let (p_sign, p_mag) = self.parked.sign_magnitude();
            if p_mag != suanpan::UBig::ZERO {
                let (_, pos_mag, pos_shift) = self.position.sign_magnitude_shl();
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

        fn finish(mut self, closing_shift: u64) -> Rank {
            self.settle();
            if !self.base.is_literally_zero() {
                self.total.add_accum_shl(&self.base, closing_shift);
            }
            let (sign, num) = self.total.sign_magnitude();
            debug_assert_ne!(sign, Ordering::Less, "the integrands are nonnegative");
            let scale = u32::try_from(closing_shift).expect("the tripwire streams stay shallow");
            Rank::from_raw(Base::from(num), scale)
        }
    }

    /// The rank fold on the span-reading integrator: the shipped
    /// [`rank`](super::super::rank) loop verbatim, integrator swapped.
    fn span_promotion_rank(bits: &BitsSlice) -> Rank {
        let max_depth = max_depth(bits);
        let (mut cursor, first) = LeafCursor::open(bits);
        let mut integral = SpanIntegrator::new();
        integral.open(&first);
        loop {
            let weight_shift = (max_depth - cursor.depth()) as u64;
            integral.interval(weight_shift);
            if cursor.done() {
                break;
            }
            let step = cursor.step(&mut integral.live, Side::A);
            integral.boundary(super::super::base_digits(&step.magnitude));
        }
        integral.finish(max_depth as u64)
    }

    /// The distance co-sweep on the span-reading integrator: the
    /// shipped pair loop verbatim (distance orientation), integrator
    /// swapped.
    fn span_promotion_distance(a_bits: &BitsSlice, b_bits: &BitsSlice) -> Rank {
        let orientation = |sign: Ordering| -> i8 {
            match sign {
                Ordering::Greater => 1,
                Ordering::Less => -1,
                Ordering::Equal => 0,
            }
        };
        let overlay_depth = max_depth(a_bits).max(max_depth(b_bits));
        let (mut ca, a_first) = LeafCursor::open(a_bits);
        let (mut cb, b_first) = LeafCursor::open(b_bits);
        let mut diff = Accumulator::new();
        diff.add_magnitude(&a_first);
        diff.sub_magnitude(&b_first);
        let mut orient = orientation(diff.sign());
        let mut integral = SpanIntegrator::new();
        if orient != 0 {
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
            let new_orient = orientation(diff.sign());
            if orient != 0 {
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
            let funded = da
                .iter()
                .chain(db.iter())
                .map(|step| super::super::base_digits(&step.magnitude))
                .max()
                .unwrap_or(1);
            integral.boundary(funded);
        }
        integral.finish(overlay_depth as u64)
    }

    /// One rank tripwire run over `PR(p)`: packed bytes and the touch
    /// count over the known-bad fold, value-pinned against the shipped
    /// kernel.
    fn span_rank_run(p: usize) -> (u64, u64) {
        let v = promotion_rearm(p).version();
        let enc = encode(&v);
        let expected = v.rank();
        touch_meter::reset();
        let r = span_promotion_rank(&enc);
        let touches = touch_meter::touches();
        assert_eq!(
            r, expected,
            "the known-bad fold must stay value-exact: a wrong demonstrator \
             proves nothing about the family's coverage"
        );
        (enc.len().div_ceil(8) as u64, touches)
    }

    /// One pair tripwire run over `(PR(p), PRM(p))`: the pair's packed
    /// bytes and the touch count over the known-bad co-sweep,
    /// value-pinned against the shipped kernel.
    fn span_pair_run(p: usize) -> (u64, u64) {
        let a = promotion_rearm(p).version();
        let b = promotion_rearm_mate(p).version();
        let ea = encode(&a);
        let eb = encode(&b);
        let expected = a.distance(&b);
        touch_meter::reset();
        let d = span_promotion_distance(&ea, &eb);
        let touches = touch_meter::touches();
        assert_eq!(
            d, expected,
            "the known-bad co-sweep must stay value-exact: a wrong \
             demonstrator proves nothing about the family's coverage"
        );
        ((ea.len() + eb.len()).div_ceil(8) as u64, touches)
    }

    /// `PR(p)` catches the span-reading promotion red on the
    /// single-stream integral: its per-byte touch cost grows across
    /// the doubling.
    ///
    /// A linear fold reads ~x1.00 here; the floor 1.36 sits midway
    /// between linear and the measured x1.74, while the shipped
    /// kernel's re-arm flatness band holds the same family at x1.25.
    ///
    /// [measured 2026-07-28, dev profile, exact counters: touches
    /// 1,440,756 -> 5,006,506 across PR(1,000) -> PR(2,000), packed
    /// 246,501B -> 493,001B: per-byte growth x1.74.]
    #[test]
    fn span_promotion_accounting_reads_superlinear_on_rearm_spine() {
        let (small_bytes, small_touches) = span_rank_run(1_000);
        let (large_bytes, large_touches) = span_rank_run(2_000);
        eprintln!(
            "MEASURED adequacy_span_promotion_rank: small={small_touches}/{small_bytes}B \
             large={large_touches}/{large_bytes}B"
        );
        assert!(
            u128::from(large_touches) * u128::from(small_bytes) * 100
                >= u128::from(small_touches) * u128::from(large_bytes) * 136,
            "the span-reading promotion reads flat on the re-arm spine \
             ({small_touches}/{small_bytes}B -> {large_touches}/{large_bytes}B): \
             the family no longer catches the mechanism it was built for, so the \
             flatness band it backs is decoration until a new witness lands"
        );
    }

    /// `(PR(p), PRM(p))` catches the span-reading promotion red on the
    /// pair integral: its per-byte touch cost grows across the doubling.
    ///
    /// The committed proof that the pair family drives promotions
    /// through the co-sweep, not just freezes.
    ///
    /// [measured 2026-07-28, dev profile, exact counters: touches
    /// 1,504,885 -> 5,134,635 across p = 1,000 -> 2,000, packed pair
    /// 269,001B -> 538,001B: per-byte growth x1.71; the floor 1.36
    /// sits midway between linear and the measured growth, as the rank
    /// tripwire's.]
    #[test]
    fn span_promotion_accounting_reads_superlinear_on_rearm_pair() {
        let (small_bytes, small_touches) = span_pair_run(1_000);
        let (large_bytes, large_touches) = span_pair_run(2_000);
        eprintln!(
            "MEASURED adequacy_span_promotion_pair: small={small_touches}/{small_bytes}B \
             large={large_touches}/{large_bytes}B"
        );
        assert!(
            u128::from(large_touches) * u128::from(small_bytes) * 100
                >= u128::from(small_touches) * u128::from(large_bytes) * 136,
            "the span-reading promotion reads flat on the re-arm pair \
             ({small_touches}/{small_bytes}B -> {large_touches}/{large_bytes}B): \
             the pair family no longer drives promotions through the co-sweep, \
             so the pair flatness band it backs is decoration until a new \
             witness lands"
        );
    }
    // ── the per-arming suffix-walk settle ──────────────────────────────
    //
    // The balanced product-tree settle exists because the ledger's debt
    // must not be charged by walking a shared suffix once per arming
    // (the module doc's settle bound). This kernel keeps the refuted
    // accounting — the ledger assembled newest-first into one running
    // suffix mass, each arming charged at its parked width times that
    // suffix's whole balanced density — committed and failing on the
    // dense-suffix family, through both the single-stream and the pair
    // integrals, so the green dense-suffix flatness bands
    // (`skyline_flatness`, `tests/meter.rs`) are never decoration.
    // Value-exact against the shipped folds: the suffix walk computes
    // the same cross-term sum, term by term; only its cost class is
    // not the tree's.

    use crate::meter::{dense_suffix, dense_suffix_mate};
    use crate::version::skyline::query::{Arming, WindowMass};

    /// The anchored-segment integral with the per-arming suffix-walk
    /// settle.
    ///
    /// Promotions record funded-width ledger entries exactly as the
    /// shipped integrator does; the close then walks one running
    /// suffix mass per arming instead of reducing the entries through
    /// the balanced product tree.
    struct SuffixWalkIntegrator {
        total: Accumulator,
        live: Accumulator,
        parked: Accumulator,
        seg: Accumulator,
        base: Accumulator,
        pos_local: Accumulator,
        promotions: Vec<Arming>,
        one: Base,
    }

    impl SuffixWalkIntegrator {
        fn new() -> SuffixWalkIntegrator {
            SuffixWalkIntegrator {
                total: Accumulator::new(),
                live: Accumulator::new(),
                parked: Accumulator::new(),
                seg: Accumulator::new(),
                base: Accumulator::new(),
                pos_local: Accumulator::new(),
                promotions: Vec::new(),
                one: Base::from(1u8),
            }
        }

        fn open(&mut self, opening: &Base) {
            self.base.add_magnitude(opening);
        }

        fn interval(&mut self, weight_shift: u64) {
            if !self.live.is_literally_zero() {
                self.total.add_accum_shl(&self.live, weight_shift);
            }
            self.seg.add_magnitude_shl(&self.one, weight_shift);
        }

        fn jump(&mut self, coefficient: i8, diff: &Accumulator) {
            let (sign, magnitude) = diff.sign_magnitude();
            if magnitude == suanpan::UBig::ZERO {
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

        fn boundary(&mut self, funded_digits: usize) {
            if self.live.digit_count() > funded_digits + FREEZE_ALLOWANCE_DIGITS {
                self.freeze();
            }
        }

        fn freeze(&mut self) {
            let (drift_sign, drift) = self.live.sign_magnitude();
            if drift == suanpan::UBig::ZERO {
                self.live.reset();
                return;
            }
            let drift = Base::from(drift);
            self.settle_segment();
            if self.parked.digit_count()
                > super::super::base_digits(&drift) + FREEZE_ALLOWANCE_DIGITS
            {
                self.promote();
            }
            match drift_sign {
                Ordering::Less => self.parked.sub_magnitude(&drift),
                _ => self.parked.add_magnitude(&drift),
            }
            self.live.reset();
            self.seg = Accumulator::new();
        }

        fn settle_segment(&mut self) {
            let (_, seg_mag, seg_shift) = self.seg.sign_magnitude_shl();
            if seg_mag == suanpan::UBig::ZERO {
                return;
            }
            let seg = Base::from(seg_mag);
            self.pos_local.add_magnitude_shl(&seg, seg_shift);
            if self.parked.is_literally_zero() {
                return;
            }
            let (p_sign, p_mag) = self.parked.sign_magnitude();
            if p_mag == suanpan::UBig::ZERO {
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

        fn settle(&mut self) {
            if self.parked.is_literally_zero() {
                return;
            }
            let (p_sign, p_mag) = self.parked.sign_magnitude();
            if p_mag == suanpan::UBig::ZERO {
                return;
            }
            let (_, seg_mag, seg_shift) = self.seg.sign_magnitude_shl();
            mul_into(
                &mut self.total,
                &Base::from(p_mag),
                &Base::from(seg_mag),
                seg_shift,
                p_sign == Ordering::Less,
            );
        }

        fn promote(&mut self) {
            let (p_sign, p_mag) = self.parked.sign_magnitude();
            if p_mag != suanpan::UBig::ZERO {
                let (_, w_mag, w_shift) = self.pos_local.sign_magnitude_shl();
                self.promotions.push(Arming {
                    neg: p_sign == Ordering::Less,
                    parked: Base::from(p_mag),
                    window: w_mag,
                    shift: w_shift,
                });
                self.pos_local = Accumulator::new();
            }
            self.parked.reset();
        }

        /// The refuted settle: one running suffix mass, assembled
        /// newest-first, each arming charged at its parked width times
        /// the suffix's whole balanced density.
        fn settle_armings(&mut self) {
            if self.promotions.is_empty() {
                return;
            }
            let (_, t_mag, t_shift) = self.pos_local.sign_magnitude_shl();
            let mut suffix = WindowMass::new();
            if t_mag != suanpan::UBig::ZERO {
                suffix.merge(&t_mag, t_shift);
            }
            let armings = core::mem::take(&mut self.promotions);
            for (i, arming) in armings.iter().enumerate().rev() {
                suffix.charge(&mut self.total, arming.neg, &arming.parked);
                if i > 0 {
                    suffix.merge(&arming.window, arming.shift);
                }
            }
        }

        fn finish(mut self, closing_shift: u64) -> Rank {
            self.settle();
            if !self.promotions.is_empty() {
                let (_, seg_mag, seg_shift) = self.seg.sign_magnitude_shl();
                if seg_mag != suanpan::UBig::ZERO {
                    self.pos_local
                        .add_magnitude_shl(&Base::from(seg_mag), seg_shift);
                }
                self.settle_armings();
            }
            if !self.base.is_literally_zero() {
                self.total.add_accum_shl(&self.base, closing_shift);
            }
            let (sign, num) = self.total.sign_magnitude();
            debug_assert_ne!(sign, Ordering::Less, "the integrands are nonnegative");
            let scale = u32::try_from(closing_shift).expect("the tripwire streams stay shallow");
            Rank::from_raw(Base::from(num), scale)
        }
    }

    /// The rank fold on the suffix-walk integrator: the shipped
    /// [`rank`](super::super::rank) loop verbatim, integrator swapped.
    fn suffix_walk_rank(bits: &BitsSlice) -> Rank {
        let max_depth = max_depth(bits);
        let (mut cursor, first) = LeafCursor::open(bits);
        let mut integral = SuffixWalkIntegrator::new();
        integral.open(&first);
        loop {
            let weight_shift = (max_depth - cursor.depth()) as u64;
            integral.interval(weight_shift);
            if cursor.done() {
                break;
            }
            let step = cursor.step(&mut integral.live, Side::A);
            integral.boundary(super::super::base_digits(&step.magnitude));
        }
        integral.finish(max_depth as u64)
    }

    /// The distance co-sweep on the suffix-walk integrator: the
    /// shipped pair loop verbatim (distance orientation), integrator
    /// swapped.
    fn suffix_walk_distance(a_bits: &BitsSlice, b_bits: &BitsSlice) -> Rank {
        let orientation = |sign: Ordering| -> i8 {
            match sign {
                Ordering::Greater => 1,
                Ordering::Less => -1,
                Ordering::Equal => 0,
            }
        };
        let overlay_depth = max_depth(a_bits).max(max_depth(b_bits));
        let (mut ca, a_first) = LeafCursor::open(a_bits);
        let (mut cb, b_first) = LeafCursor::open(b_bits);
        let mut diff = Accumulator::new();
        diff.add_magnitude(&a_first);
        diff.sub_magnitude(&b_first);
        let mut orient = orientation(diff.sign());
        let mut integral = SuffixWalkIntegrator::new();
        if orient != 0 {
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
            let new_orient = orientation(diff.sign());
            if orient != 0 {
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
            let funded = da
                .iter()
                .chain(db.iter())
                .map(|step| super::super::base_digits(&step.magnitude))
                .max()
                .unwrap_or(1);
            integral.boundary(funded);
        }
        integral.finish(overlay_depth as u64)
    }

    /// One rank tripwire run over `DS(p, p)`: packed bytes and the
    /// touch count over the known-bad fold, value-pinned against the
    /// shipped kernel.
    fn suffix_walk_rank_run(p: usize) -> (u64, u64) {
        let v = dense_suffix(p, p).version();
        let enc = encode(&v);
        let expected = v.rank();
        touch_meter::reset();
        let r = suffix_walk_rank(&enc);
        let touches = touch_meter::touches();
        assert_eq!(
            r, expected,
            "the known-bad fold must stay value-exact: a wrong demonstrator \
             proves nothing about the family's coverage"
        );
        (enc.len().div_ceil(8) as u64, touches)
    }

    /// One pair tripwire run over `(DS(p, p), DSM(p, p))`: the pair's
    /// packed bytes and the touch count over the known-bad co-sweep,
    /// value-pinned against the shipped kernel.
    fn suffix_walk_pair_run(p: usize) -> (u64, u64) {
        let a = dense_suffix(p, p).version();
        let b = dense_suffix_mate(p, p).version();
        let ea = encode(&a);
        let eb = encode(&b);
        let expected = a.distance(&b);
        touch_meter::reset();
        let d = suffix_walk_distance(&ea, &eb);
        let touches = touch_meter::touches();
        assert_eq!(
            d, expected,
            "the known-bad co-sweep must stay value-exact: a wrong \
             demonstrator proves nothing about the family's coverage"
        );
        ((ea.len() + eb.len()).div_ceil(8) as u64, touches)
    }

    /// `DS(p, p)` catches the per-arming suffix walk red on the
    /// single-stream integral: its per-byte touch cost grows across
    /// the doubling.
    ///
    /// A linear fold reads ~x1.00 here; the floor 1.48 sits midway
    /// between linear and the measured x1.96, while the shipped
    /// kernel's dense-suffix flatness band holds the same family at
    /// x1.25.
    ///
    /// [measured 2026-07-28, dev profile, exact counters: touches
    /// 3,417,450 -> 13,357,237 across DS(500, 500) -> DS(1,000,
    /// 1,000), packed 119,593B -> 239,030B: per-byte growth x1.96 —
    /// the parent fold's own public readings, digit for digit.]
    #[test]
    fn suffix_walk_settle_reads_superlinear_on_dense_suffix() {
        let (small_bytes, small_touches) = suffix_walk_rank_run(500);
        let (large_bytes, large_touches) = suffix_walk_rank_run(1_000);
        eprintln!(
            "MEASURED adequacy_suffix_walk_rank: small={small_touches}/{small_bytes}B \
             large={large_touches}/{large_bytes}B"
        );
        assert!(
            u128::from(large_touches) * u128::from(small_bytes) * 100
                >= u128::from(small_touches) * u128::from(large_bytes) * 148,
            "the per-arming suffix walk reads flat on the dense-suffix family \
             ({small_touches}/{small_bytes}B -> {large_touches}/{large_bytes}B): \
             the family no longer catches the mechanism it was built for, so the \
             flatness band it backs is decoration until a new witness lands"
        );
    }

    /// `(DS(p, p), DSM(p, p))` catches the per-arming suffix walk red
    /// on the pair integral: its per-byte touch cost grows across the
    /// doubling.
    ///
    /// The committed proof that the pair family drives the ledger
    /// settle through the co-sweep, not just freezes.
    ///
    /// [measured 2026-07-28, dev profile, exact counters: touches
    /// 6,426,091 -> 25,224,970 across p = 500 -> 1,000, packed pair
    /// 127,033B -> 253,909B: per-byte growth x1.96; the floor 1.48
    /// sits midway between linear and the measured growth, as the rank
    /// tripwire's.]
    #[test]
    fn suffix_walk_settle_reads_superlinear_on_dense_suffix_pair() {
        let (small_bytes, small_touches) = suffix_walk_pair_run(500);
        let (large_bytes, large_touches) = suffix_walk_pair_run(1_000);
        eprintln!(
            "MEASURED adequacy_suffix_walk_pair: small={small_touches}/{small_bytes}B \
             large={large_touches}/{large_bytes}B"
        );
        assert!(
            u128::from(large_touches) * u128::from(small_bytes) * 100
                >= u128::from(small_touches) * u128::from(large_bytes) * 148,
            "the per-arming suffix walk reads flat on the dense-suffix pair \
             ({small_touches}/{small_bytes}B -> {large_touches}/{large_bytes}B): \
             the pair family no longer drives the ledger settle through the \
             co-sweep, so the pair flatness band it backs is decoration until \
             a new witness lands"
        );
    }
}
