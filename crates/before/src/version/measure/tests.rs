//! Differential tests for the streaming numeric measures.
//!
//! Generated versions, operation histories, and exhaustive small trees are
//! checked against the recursive tree oracle. Rank is also checked against an
//! independent Riemann-sum implementation. Distance and lag are checked both
//! against the tree oracle and against their join/meet identities.
//!
//! Every equality here is exact: `Rank` equality is structural on the
//! normalized form, and projection agreement is byte identity of the emitted
//! canonical streams. A fold that is off by any amount anywhere therefore
//! fails.
//!
//! `rank_cmp`, `distance`, and `lag` each traverse both operands together in
//! one *co-sweep*, which integrates a function of their pointwise difference.

use num_bigint::BigUint;
use proptest::prelude::*;

use crate::testing::bridge::{from_oracle_version, to_oracle_party, to_oracle_version};
use crate::testing::exhaustive::{all_normal_events, all_normal_ids, EV_SMALL_DEPTH};
use crate::testing::generators::arb_oracle_version;
use crate::testing::generators::census::seeded::{
    assert_default_case_count, regime_floor, sample, CENSUS_CASES,
};
use crate::testing::generators::leaf_list::{
    arb_deep_or_correlated_pair, arb_multiscale_pair, TWO_VERSION_DIVISOR,
};
use crate::testing::meter::registry::Shape;
use crate::testing::meter::{Encoding, FREEZE_DRIFT_BITS, MIN_DEFERRED_ARMING_DIGITS};
use crate::testing::optrace;
use crate::testing::oracles::flat::{self, check_rank};
use crate::testing::oracles::function;
use crate::testing::property::divided_proptest;
use crate::{Clock, Party, Rank, Version};

/// Decode a meter-generated encoded shape as a [`Version`].
fn version_of(p: &Encoding) -> Version {
    p.version()
}

/// Assert the single-operand folds against the recursive tree oracle's own
/// folds.
fn assert_single(v: &Version) {
    let tree = to_oracle_version(v);
    assert_eq!(
        v.rank(),
        tree.rank(),
        "rank kernel disagrees with the tree-fold oracle: {v:?}"
    );
    assert_eq!(
        v.min_ticks(),
        tree.min_ticks(),
        "min_ticks kernel disagrees with the tree-fold oracle: {v:?}"
    );
}

/// Assert the projection kernel against the recursive oracle's mask for one
/// `(version, party)` operand pair: byte identity of the canonical streams.
fn assert_projection(v: &Version, p: &Party) {
    let masked = from_oracle_version(&to_oracle_version(v).project(&to_oracle_party(p)));
    assert_eq!(
        v.project(p).to_version(),
        masked,
        "projection must match the oracle mask: {v:?} / {p:?}"
    );
}

/// Assert the pair folds against the recursive oracle *and* the composed forms.
///
/// Each measure is checked against two independent oracles, both exact:
///
/// - **The paper oracle**: each measure re-derived from the recursive
///   oracle's join, meet, and rank through the valuation identities the
///   rustdoc states, `distance = rank(a ∨ b) − rank(a ∧ b)` and
///   `lag(a, b) = rank(a ∨ b) − rank(a)`.
/// - **The composed forms**: the same identities assembled from this
///   crate's own kernels (the emission sweep's join/meet streams, the
///   rank fold over them, and [`Rank::checked_sub`]), so the pair measures
///   are checked digit for digit against rank arithmetic computed along a
///   code path they do not share.
///
/// Every pairing also checks the signed co-sweep: `rank_cmp` must match the
/// oracle's rank order in both operand orders. It is the one fold whose total
/// carries a sign, and its `Equal` answer requires the signed settlement to
/// cancel exactly through whatever parked or deferred state the pair creates.
/// The nonnegative measures never need that answer: their totals are
/// monotone differences, debug-asserted nonnegative at the fold.
fn assert_pair(a: &Version, b: &Version) {
    let (ta, tb) = (to_oracle_version(a), to_oracle_version(b));
    let order = ta.rank().cmp(&tb.rank());
    assert_eq!(a.rank_cmp(b), order, "rank_cmp: {a:?} vs {b:?}");
    assert_eq!(
        b.rank_cmp(a),
        order.reverse(),
        "rank_cmp reversed: {b:?} vs {a:?}"
    );
    let join_rank = (ta.clone() | tb.clone()).rank();
    let meet_rank = (ta.clone() & tb.clone()).rank();
    let dist = join_rank
        .checked_sub(&meet_rank)
        .expect("rank is monotone: the meet's rank never exceeds the join's");
    assert_eq!(a.distance(b), dist, "distance: {a:?} vs {b:?}");
    assert_eq!(b.distance(a), dist, "distance: {b:?} vs {a:?}");
    let lag_a = join_rank
        .checked_sub(&ta.rank())
        .expect("rank is monotone: an operand's rank never exceeds the join's");
    let lag_b = join_rank
        .checked_sub(&tb.rank())
        .expect("rank is monotone: an operand's rank never exceeds the join's");
    assert_eq!(a.lag(b), lag_a, "lag: {a:?} vs {b:?}");
    assert_eq!(b.lag(a), lag_b, "lag: {b:?} vs {a:?}");
    // The composed forms, on this crate's own kernels.
    let kernel_join = a.join(b).rank();
    let kernel_meet = a.meet(b).rank();
    let composed_dist = kernel_join
        .checked_sub(&kernel_meet)
        .expect("rank is monotone: the meet's rank never exceeds the join's");
    assert_eq!(
        a.distance(b),
        composed_dist,
        "distance vs the composed rank-of-meet arithmetic: {a:?} vs {b:?}"
    );
    let composed_lag_a = kernel_join
        .checked_sub(&a.rank())
        .expect("rank is monotone: an operand's rank never exceeds the join's");
    assert_eq!(
        a.lag(b),
        composed_lag_a,
        "lag vs the composed rank-of-join arithmetic: {a:?} vs {b:?}"
    );
}

/// Representative shapes used by the deterministic differential tests.
fn family_pool() -> Vec<Version> {
    vec![
        Version::new(),
        version_of(&Shape::Dense.build1(1)),
        version_of(&Shape::Dense.build1(2)),
        version_of(&Shape::Dense.build1(64)),
        version_of(&Shape::Bigroot.build2(7, 3)),
        version_of(&Shape::Bigroot.build2(64, 16)),
        version_of(&Shape::Hugeleaf.build1(1)),
        version_of(&Shape::Hugeleaf.build1(64)),
        version_of(&Shape::CliffComb.build2(3, 2)),
        version_of(&Shape::CliffComb.build2(16, 16)),
        version_of(&Shape::WideToothComb.build3(16, 8, 8)),
        // Wide teeth over the freeze allowance: bounded oscillation that
        // must stay in the live component without freezing.
        version_of(&Shape::WideToothComb.build3(320, 300, 6)),
        // The stale-drift shape: the mid-stream jump is wide enough that
        // the first cheap delta behind it fires a freeze.
        version_of(&Shape::JumpComb.build2(16, 8)),
        version_of(&Shape::JumpComb.build2(320, 4)),
        version_of(&Shape::CliffFan.build2(16, 8)),
        version_of(&Shape::CancellingChain.build2(16, 8)),
        version_of(&Shape::AltSpine.build1(3)),
        version_of(&Shape::AltSpine.build1(64)),
        version_of(&Shape::Harmonic.build1(16)),
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
        Party::decode(&Shape::ScatteredId.build1(1).bytes[..])
            .expect("scattered id is strict normal form"),
        Party::decode(&Shape::ScatteredId.build1(9).bytes[..])
            .expect("scattered id is strict normal form"),
    ]
}

/// Every representative shape agrees with the tree-fold oracle's rank and
/// min_ticks; crosses and pairs agree on projection, distance, and lag.
///
/// The id-pool cross checks the oracle's projection mask; the ordered pairs
/// check distance and lag as re-derived from the oracle's lattice folds.
///
/// The families are the shapes the amplification board measures (carry
/// cliffs, wide teeth, cancelling chains, harmonic spines), so a
/// height-tracking or freeze bookkeeping error fails here before any cost
/// bound does.
#[test]
fn families_agree_with_the_encodings() {
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

/// Shapes whose sweeps park, defer, or settle wide drift, at hand-checkable
/// sizes.
///
/// The other pools freeze but almost never defer. A unit-funded fold freezes
/// when its live drift is wider than
/// [`HEIGHT_FREEZE_ALLOWANCE_BITS`](super::integral::HEIGHT_FREEZE_ALLOWANCE_BITS)
/// plus one bit, and `arb_magnitude` reaches 514 bits, so arbitrary trees do
/// freeze. A deferral needs more: at a later freeze, a parked height wider
/// than that freeze's drift by more than the allowance, so a parked height
/// wider than twice the allowance plus one bit beside a drift wider than the
/// allowance plus one bit. Arbitrary trees reach that conjunction too rarely
/// for their sweeps to cover the deferred-height reduction. These shapes
/// reach it, and the arming trains create multiple entries with both uniform
/// and alternating signs.
fn deferral_pool() -> Vec<Version> {
    vec![
        version_of(&Shape::PromotionRearm.build1(1)),
        version_of(&Shape::PromotionRearm.build1(3)),
        version_of(&Shape::PromotionRearmMate.build1(3)),
        version_of(&Shape::DenseSuffix.build2(1, 2)),
        version_of(&Shape::DenseSuffix.build2(3, 1)),
        version_of(&Shape::DenseSuffixMate.build2(3, 1)),
        version_of(&Shape::WideArming.build2(MIN_DEFERRED_ARMING_DIGITS, 2)),
        version_of(&Shape::WideArming.build2(MIN_DEFERRED_ARMING_DIGITS + 3, 3)),
        version_of(&Shape::FreezePosition.build1(3)),
        // The narrow freezes: a freeze at every second boundary, each drift
        // nearly cancelling the one parked before it, so no freeze defers.
        version_of(&Shape::NarrowFreezes.build2(FREEZE_DRIFT_BITS, 1)),
        version_of(&Shape::PlateauPuncture.build2(10, 3)),
        version_of(&Shape::PlateauPuncture.build2(12, 1)),
        // The sweep's one freeze, fired late (after a long plateau prefix
        // that never freezes) and fired early (ahead of a long tail that
        // never freezes): the settlement's smallest nonempty configuration,
        // one parked drift against one final segment, from both sides.
        version_of(&Shape::LoneFreeze.build2(2, 2)),
        version_of(&Shape::LoneFreeze.build2(6, 2)),
        version_of(&Shape::LoneFreeze.build2(2, 6)),
        // The multi-arming trains, same-sign and alternating, so the
        // settlement's parked sums both accumulate and cancel across
        // aggregate merge boundaries.
        version_of(&Shape::ArmingTrain.build_train(1, 19, 1, false)),
        version_of(&Shape::ArmingTrain.build_train(3, 19, 1, false)),
        version_of(&Shape::ArmingTrain.build_train(4, 19, 2, true)),
        version_of(&Shape::ArmingTrain.build_train(5, 20, 1, true)),
    ]
}

/// Every deferred-height family shape agrees with the tree-fold oracle on rank and
/// min_ticks, and every ordered pair agrees on distance and lag against both
/// the oracle and the composed forms.
///
/// This checks the settlement's values at the shapes the cost tests measure:
/// single and repeated armings, mixed-sign armings whose parked sums cancel
/// digit by digit inside the balanced reduction's aggregates, dense windows
/// between armings, and the arming-free settlement at the close (the
/// plateau-puncture family). The pair sweep crosses wide operands with wide
/// operands, with both sides deferring and orientation flips inside wide
/// plateaus, which the families' unit-base mates never reach.
///
/// The [`DEFERRAL_HITS`](super::integral::DEFERRAL_HITS) floors keep both
/// claims live: the single folds defer, and so do the pair measures'
/// co-sweeps, each counted around its own production calls alone.
#[test]
fn deferral_families_agree_with_the_oracle() {
    let pool = deferral_pool();
    let (_, single_deferrals) = freezes_and_deferrals_during(|| {
        for v in &pool {
            v.rank();
        }
    });
    let (_, cosweep_deferrals) = freezes_and_deferrals_during(|| {
        for a in &pool {
            for b in &pool {
                run_cosweeps(a, b);
            }
        }
    });
    assert!(
        single_deferrals > 0 && cosweep_deferrals > 0,
        "liveness: the deferral pool must defer in single folds and in co-sweeps \
         ({single_deferrals} and {cosweep_deferrals} deferrals)"
    );
    for v in &pool {
        assert_single(v);
    }
    for a in &pool {
        for b in &pool {
            assert_pair(a, b);
        }
    }
}

/// Runs the pair measures that sweep both operands together, `rank_cmp`,
/// `distance`, and `lag`, in both operand orders, for their counter readings.
fn run_cosweeps(a: &Version, b: &Version) {
    for (x, y) in [(a, b), (b, a)] {
        x.rank_cmp(y);
        x.distance(y);
        x.lag(y);
    }
}

/// A right-spine version whose leaf heights, in stream order, are exactly
/// `heights`.
///
/// The freeze-schedule vocabulary: each adjacent difference is one folded
/// delta, so a height list is a delta script for the sweep's live component.
fn spine_of(heights: &[BigUint]) -> Version {
    use crate::testing::oracles::tree::Version as V;
    let mut tree = V::leaf(heights[heights.len() - 1].clone());
    for h in heights[..heights.len() - 1].iter().rev() {
        tree = V::node(0u64, V::leaf(h.clone()), tree);
    }
    from_oracle_version(&tree)
}

/// The exact-cancellation freeze schedule.
///
/// Heights whose freezes park `+2^(32p)`, `−(2^32 − 1)·2^(32(p−1))`, and
/// `−2^(32(p−1))`. Those terms cancel although the parked accumulator retains
/// a high positive digit over negative ones. A fourth freeze follows a climb to
/// `2^(32q)`; it settles a segment against the zero-valued parked component.
/// The final height `s` leaves the newly parked climb a closing segment.
///
/// Each freeze fires at a unit delta, after a drift whose working width must
/// exceed that delta's one bit plus the freeze allowance `a` bits. `suanpan`
/// keeps every value below `2^(stored_bits + 2)`, so a power `2^(32m)` holds at
/// least `32m − 1` working bits in every stored form, and the smallest drifts,
/// `2^(32(p−1))` and `2^(32q)`, cross the trigger in every form when `32(p − 1)`
/// and `32q` exceed `a + 2`. No compaction can then keep a freeze from firing.
///
/// # Panics
///
/// Panics if `32(p − 1)` or `32q` is at most `a + 2`.
fn parked_cancellation_heights(p: u32, q: u32, s: u64) -> Vec<BigUint> {
    let crosses = |m: u32| 32 * u64::from(m) > super::integral::HEIGHT_FREEZE_ALLOWANCE_BITS + 2;
    assert!(
        p >= 1 && crosses(p - 1) && crosses(q),
        "every drift must cross the freeze trigger in its shortest form"
    );
    let x = BigUint::from(1u8) << (32 * p);
    let t = BigUint::from(1u8) << (32 * (p - 1));
    let z = BigUint::from(1u8) << (32 * q);
    vec![
        BigUint::ZERO,
        x.clone() - &BigUint::from(1u8),
        x,
        t.clone() + &BigUint::from(1u8),
        t,
        BigUint::from(1u8),
        BigUint::ZERO,
        z.clone() - &BigUint::from(1u8),
        z,
        BigUint::from(s),
    ]
}

/// A schedule whose drift cancels to zero at a narrow delta.
///
/// Deltas `+(2^(32p) + d)`, `−(2^32 − 1)·2^(32(p−1))`, `−2^(32(p−1))`, then
/// `−d`: the last, narrow delta leaves the drift's value exactly zero after
/// wide deltas wrote digits up to `p`.
fn zero_drift_heights(p: u32, d: u64) -> Vec<BigUint> {
    let x = BigUint::from(1u8) << (32 * p);
    let t = BigUint::from(1u8) << (32 * (p - 1));
    vec![
        BigUint::ZERO,
        x + &BigUint::from(d),
        t.clone() + &BigUint::from(d),
        BigUint::from(d),
        BigUint::ZERO,
    ]
}

/// The worked point of the cancellation family, with the freeze tap proving
/// the schedule really parks.
///
/// Four freezes fire, the fourth settling a segment against a parked
/// component whose buffered positive and negative terms cancel. The cheap
/// `is_known_zero` check cannot detect that cancellation, and every fold
/// stays exact against the tree oracle.
#[test]
fn parked_cancellation_settles_exactly() {
    let v = spine_of(&parked_cancellation_heights(11, 10, 5));
    let hits_before = super::integral::FREEZE_HITS.with(|hits| hits.get());
    assert_single(&v);
    assert!(
        super::integral::FREEZE_HITS.with(|hits| hits.get()) >= hits_before + 4,
        "the cancellation schedule no longer parks four drifts: the zero-valued \
         settlement it exists to drive is undriven"
    );
}

proptest! {
    /// The exact-cancellation family holds every fold to the tree oracle over
    /// the cancellation scale, the trailing climb's scale, and the final
    /// height.
    ///
    /// A parked component whose terms cancel must charge nothing at the
    /// settlement that follows; any misaccounting changes the exact totals.
    #[test]
    fn parked_cancellation_family_agrees(p in 11u32..=14, q in 10u32..=13, s in 1u64..=6) {
        let v = spine_of(&parked_cancellation_heights(p, q, s));
        let hits_before = super::integral::FREEZE_HITS.with(|hits| hits.get());
        assert_single(&v);
        prop_assert!(
            super::integral::FREEZE_HITS.with(|hits| hits.get()) >= hits_before + 4,
            "a cancellation schedule stopped parking"
        );
    }

    /// The zero-drift family: a drift that cancels exactly never trips the
    /// freeze trigger, and both folds stay exact against the tree oracle.
    ///
    /// The rank integral compacts its drift before each boundary, and
    /// min_ticks normalizes its live offset at each leaf, so the cancelled
    /// wide digits are gone before the narrow delta's width check. Neither
    /// freeze therefore needs a case for a zero drift.
    #[test]
    fn zero_drift_never_freezes(p in 9u32..=13, d in 1u64..=6) {
        let freezes = || {
            (
                super::integral::FREEZE_HITS.with(|hits| hits.get()),
                super::min_ticks::FREEZE_HITS.with(|hits| hits.get()),
            )
        };
        let before = freezes();
        assert_single(&spine_of(&zero_drift_heights(p, d)));
        prop_assert_eq!(
            freezes(),
            before,
            "a drift that cancels exactly tripped a freeze trigger (rank, min_ticks)"
        );
    }
}

/// `rank_cmp` agrees with the oracle rank order across the deferral pool and
/// reads `Equal` on a mirrored equal-rank pair, with the freeze tap
/// proving both legs actually park drift.
///
/// This checks the signed co-sweep's values where freezes and deferrals
/// fire. The pool cross checks the sign against the oracle's rank order in
/// both operand orders where the sweeps park, defer, and settle wide drift.
/// The mirrored pair, one deferring shape on each side of a fresh root fork,
/// is two distinct streams of exactly equal rank, so it checks the `Equal`
/// answer, which requires the signed settlement to cancel to zero through
/// parking, deferral, and settlement. The
/// [`FREEZE_HITS`](super::integral::FREEZE_HITS) and
/// [`DEFERRAL_HITS`](super::integral::DEFERRAL_HITS) floors show that the
/// regime is reached: a pool or pair that never froze, or froze without
/// deferring, would pass every value check while running none of the
/// deferred reduction.
#[test]
fn rank_cmp_agrees_with_the_oracle_in_the_freeze_regime() {
    let assert_cmp = |a: &Version, b: &Version| {
        let want = to_oracle_version(a)
            .rank()
            .cmp(&to_oracle_version(b).rank());
        assert_eq!(a.rank_cmp(b), want, "rank_cmp: {a:?} vs {b:?}");
        assert_eq!(
            b.rank_cmp(a),
            want.reverse(),
            "rank_cmp reversed: {a:?} vs {b:?}"
        );
    };
    let pool = deferral_pool();
    let (pool_freezes, pool_deferrals) = freezes_and_deferrals_during(|| {
        for a in &pool {
            for b in &pool {
                assert_cmp(a, b);
            }
        }
    });
    assert!(
        pool_freezes > 0 && pool_deferrals > 0,
        "liveness: the deferral-pool cross must freeze and defer under rank_cmp \
         ({pool_freezes} freezes, {pool_deferrals} deferrals)"
    );
    let t = to_oracle_version(&version_of(
        &Shape::ArmingTrain.build_train(3, 19, 1, false),
    ));
    let zero = crate::testing::oracles::tree::Version::leaf(0u64);
    let left = from_oracle_version(&crate::testing::oracles::tree::Version::node(
        0u64,
        t.clone(),
        zero.clone(),
    ));
    let right = from_oracle_version(&crate::testing::oracles::tree::Version::node(0u64, zero, t));
    assert_ne!(left, right, "the mirrored pair must be distinct streams");
    assert_eq!(
        to_oracle_version(&left).rank(),
        to_oracle_version(&right).rank(),
        "the mirrored pair must tie exactly in rank"
    );
    let (mirror_freezes, mirror_deferrals) =
        freezes_and_deferrals_during(|| assert_cmp(&left, &right));
    assert!(
        mirror_freezes > 0 && mirror_deferrals > 0,
        "liveness: the mirrored equal-rank pair must freeze and defer under rank_cmp \
         ({mirror_freezes} freezes, {mirror_deferrals} deferrals)"
    );
}

/// The multi-scale census seed: names the sampled corpus (see
/// [`crate::testing::rng::strategy_rng`]).
///
/// Fixed because the census measures its generators
/// deterministically: a regime it counts has no mass derived from the
/// generators' weights, so its floors hold for this corpus alone.
const MULTISCALE_CENSUS_SEED: u64 = 17;

/// Multi-scale pairs keep generator mass on deferral, in single folds and in
/// co-sweeps, and on exact rank ties.
///
/// Each default run of every property drawing them must expect
/// [`REGIME_CASES_PER_RUN`](crate::testing::generators::census::seeded::REGIME_CASES_PER_RUN)
/// pairs in each regime.
///
/// A pair defers in single folds when
/// [`DEFERRAL_HITS`](super::integral::DEFERRAL_HITS) rises across either
/// operand's rank, and in co-sweeps when it rises across `rank_cmp`,
/// `distance`, and `lag`, counted apart. A pair ties when the recursive
/// oracle finds its versions distinct and of equal rank. The law drivers
/// draw the same pairs at the case count divided by
/// [`TWO_VERSION_DIVISOR`], the fewest cases of any consumer, so the floors
/// follow from that premise and their case count. A generator change that
/// stops reaching a regime fails here whatever the census last read.
#[test]
fn multiscale_pairs_keep_mass_on_deferral_and_ties() {
    assert_default_case_count();
    let (mut single_deferring, mut cosweep_deferring, mut tied) = (0usize, 0usize, 0usize);
    for (a, b) in sample(&arb_multiscale_pair(), MULTISCALE_CENSUS_SEED) {
        let (ta, tb) = (a.tree(), b.tree());
        let (a, b) = (from_oracle_version(&ta), from_oracle_version(&tb));
        let (_, single_deferrals) = freezes_and_deferrals_during(|| {
            a.rank();
            b.rank();
        });
        let (_, cosweep_deferrals) = freezes_and_deferrals_during(|| run_cosweeps(&a, &b));
        single_deferring += usize::from(single_deferrals > 0);
        cosweep_deferring += usize::from(cosweep_deferrals > 0);
        tied += usize::from(ta != tb && ta.rank() == tb.rank());
    }
    let floor = regime_floor(CENSUS_CASES, TWO_VERSION_DIVISOR);
    eprintln!(
        "census over {CENSUS_CASES} multi-scale pairs: deferring in single folds \
         {single_deferring}, in co-sweeps {cosweep_deferring}, tied {tied} (floor {floor} each)"
    );
    for (regime, count) in [
        ("deferral in single folds", single_deferring),
        ("deferral in co-sweeps", cosweep_deferring),
        ("an exact rank tie", tied),
    ] {
        assert!(
            count >= floor,
            "multi-scale pairs reach {regime} in {count} of {CENSUS_CASES} sampled \
             pairs (floor {floor}): a relation or a height scale has gone dead"
        );
    }
}

/// Count the freezes and the deferrals the integral fires while `measure` runs.
fn freezes_and_deferrals_during(measure: impl FnOnce()) -> (u64, u64) {
    let read = || {
        (
            super::integral::FREEZE_HITS.with(|hits| hits.get()),
            super::integral::DEFERRAL_HITS.with(|hits| hits.get()),
        )
    };
    let (freezes, deferrals) = read();
    measure();
    let (freezes_after, deferrals_after) = read();
    (freezes_after - freezes, deferrals_after - deferrals)
}

/// Every registry family built to fire the integral's freeze fires exactly
/// the freezes and deferrals its layout implies.
///
/// A family that stops freezing or deferring still passes its value checks and
/// its cost ceilings, while measuring a cheaper regime than it claims, so
/// each count is asserted here by name. The wide drifts are
/// [`FREEZE_DRIFT_BITS`](crate::testing::meter::FREEZE_DRIFT_BITS) or wider, so
/// they trip the trigger in every stored form, and no compaction of `live` can
/// lower a count. The armings that must defer are
/// [`MIN_DEFERRED_ARMING_DIGITS`] or wider, so the freeze that follows them
/// defers them in every stored form. A deferral needs a parked component wider
/// than the incoming drift by more than the freeze allowance, so families
/// without such an arming defer nothing. Per family, freezes and then
/// deferrals:
///
/// - `LF(pre, post)`: one freeze, at the unit after the plateau drop. No
///   deferral: the only freeze has no parked height before it.
/// - `FP(k)`: `k − 1` freezes; the first block's drop lies in the opening
///   height. No deferral: every parked drop is as wide as the next drift.
/// - `FZ(k)`: `k − 1` freezes, one per cross-pair code, and no deferral.
/// - `NF(e, c)`: `4c` freezes, one at each one-tick step after a wide step.
///   No deferral: the parked height alternates `2^e + 1` and `1`, never
///   wider than the next drift by the allowance.
/// - `PR(p)`: `2p` freezes, each block's arming and settling freezes, and `p`
///   deferrals, each block's settling freeze deferring its arming.
/// - `DS(p, d)`: `2p + 1` freezes, the blocks' freezes and one where the
///   trailing run follows the descent, and `p` deferrals, as for `PR(p)`.
/// - `WA(w, d)` and `HW(w, d, t)`: three freezes, the arming, the settle, and
///   the descent, and one deferral, the settle's freeze deferring the arming.
/// - `AT(n, ..)`: `2n + 1` freezes, each block's swing and kicker, and the
///   closing plunge, and `n` deferrals, each kicker's freeze deferring its
///   swing.
/// - `PP(w, d)`: one freeze, at the plunge, and no deferral.
/// - `JP(k, m, d)` under `distance`: `2m` freezes, two crests per comb level,
///   and no deferral: no parked crest outspans the drift that follows it by
///   the allowance.
#[test]
fn freeze_families_fire_their_freezes() {
    let rank_counts = |encoded: Encoding| {
        let version = version_of(&encoded);
        freezes_and_deferrals_during(|| {
            version.rank();
        })
    };
    let w = MIN_DEFERRED_ARMING_DIGITS;
    let p = FREEZE_DRIFT_BITS;
    let cases: [(&str, Encoding, (u64, u64)); 20] = [
        ("LF(2, 2)", Shape::LoneFreeze.build2(2, 2), (1, 0)),
        ("LF(6, 2)", Shape::LoneFreeze.build2(6, 2), (1, 0)),
        ("LF(2, 64)", Shape::LoneFreeze.build2(2, 64), (1, 0)),
        ("FP(1)", Shape::FreezePosition.build1(1), (0, 0)),
        ("FP(3)", Shape::FreezePosition.build1(3), (2, 0)),
        ("FP(8)", Shape::FreezePosition.build1(8), (7, 0)),
        ("FZ(1)", Shape::FreezeParade.build1(1), (0, 0)),
        ("FZ(4)", Shape::FreezeParade.build1(4), (3, 0)),
        (
            "NF(FREEZE_DRIFT_BITS, 1)",
            Shape::NarrowFreezes.build2(p, 1),
            (4, 0),
        ),
        (
            "NF(FREEZE_DRIFT_BITS, 5)",
            Shape::NarrowFreezes.build2(p, 5),
            (20, 0),
        ),
        ("PR(1)", Shape::PromotionRearm.build1(1), (2, 1)),
        ("PR(3)", Shape::PromotionRearm.build1(3), (6, 3)),
        ("DS(1, 2)", Shape::DenseSuffix.build2(1, 2), (3, 1)),
        ("DS(3, 1)", Shape::DenseSuffix.build2(3, 1), (7, 3)),
        (
            "WA(MIN_DEFERRED_ARMING_DIGITS, 2)",
            Shape::WideArming.build2(w, 2),
            (3, 1),
        ),
        (
            "HW(MIN_DEFERRED_ARMING_DIGITS, 1, 32(MIN_DEFERRED_ARMING_DIGITS + 2))",
            Shape::HoistedWindow.build3(w, 1, 32 * (w + 2)),
            (3, 1),
        ),
        ("PP(10, 3)", Shape::PlateauPuncture.build2(10, 3), (1, 0)),
        (
            "AT(1, 19, 1)",
            Shape::ArmingTrain.build_train(1, 19, 1, false),
            (3, 1),
        ),
        (
            "AT(3, 19, 1)",
            Shape::ArmingTrain.build_train(3, 19, 1, false),
            (7, 3),
        ),
        (
            "AT(4, 19, 2, alternating)",
            Shape::ArmingTrain.build_train(4, 19, 2, true),
            (9, 4),
        ),
    ];
    for (name, encoded, expected) in cases {
        assert_eq!(
            rank_counts(encoded),
            expected,
            "(freeze, deferral) counts of rank over {name}"
        );
    }
    for (k, m, d) in [(320, 1, 1), (320, 6, 3)] {
        let (a, b) = Shape::JumpPair.build_pair3(k, m, d);
        let (a, b) = (version_of(&a), version_of(&b));
        let expected = (2 * m as u64, 0);
        assert_eq!(
            freezes_and_deferrals_during(|| {
                a.distance(&b);
            }),
            expected,
            "(freeze, deferral) counts of distance over JP({k}, {m}, {d})"
        );
    }
}

/// The two version-pair families agree with the oracle and the composed forms
/// on distance and lag, at their own constructed pairings.
///
/// The jump-pair dimensions straddle the freeze allowance (`k = 3` up to `k =
/// 512`, past the 256-bit digit bound), so the pair measures are checked on
/// shapes where the difference's live component carries wide drift across
/// the other operand's cheap boundaries, the interleaving the family exists
/// to reach. The concurrent pair checks the side-switch density population,
/// where the difference's sign flips at every one of the `n − 1` overlay
/// boundaries.
#[test]
fn pair_families_agree() {
    for (k, m, d) in [(3, 1, 1), (16, 4, 2), (320, 6, 3), (512, 8, 2)] {
        let (pa, pb) = Shape::JumpPair.build_pair3(k, m, d);
        assert_pair(&version_of(&pa), &version_of(&pb));
    }
    for n in [2, 4, 16, 64] {
        let (v, w) = Shape::ConcurrentPair.version_pair(n);
        assert_pair(&v, &w);
    }
}

/// Every normal-form event tree to the small depth agrees on rank and
/// `min_ticks`, and every such tree and normal-form id agree on projection.
///
/// This covers every boundary reachable at the chosen depth without sampling.
#[test]
fn exhaustive_small_scope_agrees() {
    let events: Vec<Version> = all_normal_events(EV_SMALL_DEPTH)
        .iter()
        .map(from_oracle_version)
        .collect();
    let ids: Vec<Party> = all_normal_ids(2)
        .iter()
        .filter(|party| !party.is_empty())
        .map(crate::testing::bridge::from_oracle_party)
        .collect();
    for v in &events {
        assert_single(v);
        for p in &ids {
            assert_projection(v, p);
        }
    }
}

/// Every *ordered pair* of normal-form event trees to the small depth agrees
/// between the pair co-sweep and the composed forms: distance equals
/// `rank(join) − rank(meet)` and lag equals `rank(join) − rank(a)`,
/// digit-exact.
///
/// The total check over the pair space covers every boundary the comparison
/// sweep's exhaustive suite reaches (aligned ties, flush-right ties at unequal
/// depths, plateau consumption, zero deltas across subtree boundaries) crossed
/// with every orientation schedule reachable at this scope, by brute force
/// rather than sampling. The family and proptest sweeps check the same
/// identities against the oracle; here the composed kernels are the oracle,
/// so the quadratic pair product stays fast.
#[test]
fn exhaustive_small_scope_pairs_agree() {
    let events: Vec<Version> = all_normal_events(EV_SMALL_DEPTH)
        .iter()
        .map(from_oracle_version)
        .collect();
    for a in &events {
        let rank_a = a.rank();
        for b in &events {
            let join = a.join(b).rank();
            let meet = a.meet(b).rank();
            let composed_dist = join
                .checked_sub(&meet)
                .expect("rank is monotone: the meet's rank never exceeds the join's");
            assert_eq!(
                a.distance(b),
                composed_dist,
                "distance at a small-scope pair"
            );
            let composed_lag = join
                .checked_sub(&rank_a)
                .expect("rank is monotone: an operand's rank never exceeds the join's");
            assert_eq!(a.lag(b), composed_lag, "lag at a small-scope pair");
        }
    }
}

proptest! {
    /// Arbitrary normal-form trees agree with the recursive oracle on every
    /// measure fold.
    ///
    /// Rank is additionally realized by the semantic oracle's Riemann sum over
    /// the resolving grid, which shares no recursion, no delta, and no
    /// accumulator with either implementation.
    #[test]
    fn arbitrary_trees_agree(oa in arb_oracle_version(), ob in arb_oracle_version()) {
        let a = from_oracle_version(&oa);
        let b = from_oracle_version(&ob);
        assert_single(&a);
        assert_pair(&a, &b);
        let ev = function::lift_ev(oa);
        let g = function::ev_res(&ev);
        prop_assert_eq!(
            function::rank(&ev, g),
            a.rank(),
            "the Riemann sum disagrees with the rank kernel: {:?}", a
        );
        prop_assert_eq!(
            function::min_ticks(&ev, g),
            a.min_ticks().0,
            "the semantic tick floor disagrees with the min_ticks kernel: {:?}", a
        );
    }

    /// One organic op-trace history agrees with the recursive
    /// oracle on every measure fold.
    ///
    /// Projection runs onto the history's own parties, the operand pairing
    /// production code builds.
    #[test]
    fn organic_histories_agree(ops in optrace::production_strategy_up_to(40)) {
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

    /// Jump-comb pairs at arbitrary dimensions agree with the oracle and the
    /// composed forms on distance and lag.
    ///
    /// The tooth width `k` is drawn across the freeze allowance's 256-bit digit
    /// bound, so the sampled family covers both the bounded-oscillation regime
    /// (wide folds cancel adjacently, nothing freezes) and the wide-drift
    /// regime (the difference's live component crosses cheap boundaries wide),
    /// at varying comb depth `m` and spine density `d`.
    #[test]
    fn arbitrary_jump_pairs_agree(k in 3usize..400, m in 1usize..8, d in 1usize..4) {
        let (pa, pb) = Shape::JumpPair.build_pair3(k, m, d);
        assert_pair(&version_of(&pa), &version_of(&pb));
    }

    /// Concurrent pairs at arbitrary power-of-two fork widths agree with the
    /// oracle and the composed forms on distance and lag: the side-switch
    /// density family, where every overlay boundary flips which operand
    /// dominates.
    #[test]
    fn arbitrary_concurrent_pairs_agree(log_n in 1u32..7) {
        let (v, w) = Shape::ConcurrentPair.version_pair(1 << log_n);
        assert_pair(&v, &w);
    }

    /// Multi-scale pairs agree with the oracle and the composed forms on
    /// every measure fold.
    ///
    /// Each pair is a version of bushy regions, a quarter of the time beside
    /// a deep same-side spine, with heights moved by steps from a few bits to
    /// 2,200, and a second version derived from it ([`arb_multiscale_pair`]).
    /// These are arbitrary inputs, not named shapes, that drive the integral
    /// through freezes and deferrals, in single folds and in co-sweeps, and
    /// that tie exactly in rank while differing as versions;
    /// `multiscale_pairs_keep_mass_on_deferral_and_ties` floors each regime.
    #[test]
    fn multiscale_pairs_agree((a, b) in arb_multiscale_pair()) {
        let (a, b) = (from_oracle_version(&a.tree()), from_oracle_version(&b.tree()));
        assert_single(&a);
        assert_single(&b);
        assert_pair(&a, &b);
    }

    /// The projection kernel agrees with the oracle's semantic mask over
    /// arbitrary Version × arbitrary Party operands, where absent Party
    /// children exercise the unowned arm at every depth.
    #[test]
    fn arbitrary_projections_agree(
        ov in arb_oracle_version(),
        oi in proptest::sample::select(
            all_normal_ids(3).into_iter().filter(|party| !party.is_empty()).collect::<Vec<_>>()
        ),
    ) {
        let v = from_oracle_version(&ov);
        let p = crate::testing::bridge::from_oracle_party(&oi);
        assert_projection(&v, &p);
    }

    /// The exact rank embeds the product of two arbitrary integers: `rank(V(x,
    /// y)) = (2·x·y + 1) / 2^bits(2y)` for every positive `x` and `y`, through
    /// the public fold.
    ///
    /// This is the evidence for the `Ω(M(·))` lower bound: a reduction from
    /// arbitrary integer multiplication, not one committed shape. `V(x, y)`
    /// stores `Θ(bits(x) + bits(y))` bits, and its exact rank's numerator
    /// carries the full product (one subtraction and one shift recover `x·y`),
    /// so any fold that answers this family exactly multiplies two arbitrary
    /// input-funded factors at linear overhead. The independent oracle is the
    /// backend's own multiplication, computed here outside any fold. The pair
    /// measures inherit the bound through their valuation identities: against
    /// the empty version, distance and lag both reduce to the rank, which the
    /// test asserts on the public entry points.
    #[test]
    fn arbitrary_factors_embed_their_product_in_exact_rank(
        x_bytes in proptest::collection::vec(any::<u8>(), 1..64),
        y_bytes in proptest::collection::vec(any::<u8>(), 1..64),
    ) {
        let nonzero = |bytes: &[u8]| {
            let v = BigUint::from_bytes_le(bytes);
            if v == BigUint::ZERO { BigUint::ONE } else { v }
        };
        let (x, y) = (nonzero(&x_bytes), nonzero(&y_bytes));
        let v = Shape::PunctureProduct.build_product(&x, &y).version();
        let wire = v.encode();
        prop_assert_eq!(
            &Version::decode(&wire[..]).expect("a stored version's wire bytes decode"),
            &v,
            "the constructor's output must be canonical"
        );
        // The reduction's size premise, checked where the reduction is
        // constructed: the *stored* stream is Θ(bits(x) + bits(y)), even
        // though the encoded construction spells the plateau per turn. The
        // deltas collapse to one climb (≤ 2·bits(x) + 1 code bits) and one
        // plunge (≤ 2·bits(x) + 3), and each of the `bits(2y)` levels costs
        // O(1) topology and payload bits. Without this check the lower-bound
        // argument would rest on a stored size nothing verifies: a fold could
        // be charged M(|v|) against an operand as large as the product itself.
        prop_assert!(
            v.encoded_bits() <= 4 * x.bits() + 4 * (y.bits() + 1) + 64,
            "the stored stream must stay linear in the factors' widths: \
             {} stored bits against bits(x) = {}, bits(y) = {}",
            v.encoded_bits(),
            x.bits(),
            y.bits(),
        );
        let numerator = ((&x * &y) << 1usize) + 1u8;
        prop_assert_eq!(
            v.rank(),
            Rank::from_raw(
                numerator,
                y.bits() + 1,
            ),
            "the exact rank must embed the arbitrary product"
        );
        let empty = Version::new();
        prop_assert_eq!(
            v.distance(&empty),
            v.rank(),
            "distance to the empty version is the rank"
        );
        prop_assert_eq!(
            empty.lag(&v),
            v.rank(),
            "the empty version lags by the rank"
        );
    }
}

/// Returns every arming train the exhaustive train tests cover, as `(n, w, g,
/// alternate)`.
///
/// The box spans entry counts across several reduction shapes (a lone entry,
/// a full level, an odd drain), arming widths of 19 to 22 digits (608 to 704
/// bits), wider than any height an arbitrary tree reaches, window densities
/// from trivial to multi-digit, and both sign schedules. An arbitrary leaf's
/// height sums at most five `arb_magnitude` draws along its path, each below
/// `2^514`, so it stays below `2^517`.
fn arming_train_box() -> impl Iterator<Item = (usize, usize, usize, bool)> {
    (1usize..6).flat_map(|n| {
        (19usize..23).flat_map(move |w| {
            (1usize..4).flat_map(move |g| [false, true].map(move |alternate| (n, w, g, alternate)))
        })
    })
}

/// Every arming train in [`arming_train_box`] agrees with the oracle on every
/// measure fold, singly and paired with its opposite-schedule twin.
///
/// A train's rank defers once per block, and the
/// [`DEFERRAL_HITS`](super::integral::DEFERRAL_HITS) floor keeps that regime
/// live for every train. The pair leg crosses the train against its
/// opposite-schedule twin, so the difference's orientation flips inside wide
/// plateaus. That co-sweep freezes for trains of more than one block and
/// defers for none: no nonzero parked height in the difference is wider than
/// the drift after it by the freeze allowance. The mirrored trains and the
/// multi-scale pairs drive deferral through the co-sweeps.
#[test]
fn arming_trains_agree_across_their_box() {
    for (n, w, g, alternate) in arming_train_box() {
        let a = version_of(&Shape::ArmingTrain.build_train(n, w, g, alternate));
        let b = version_of(&Shape::ArmingTrain.build_train(n, w, g, !alternate));
        let (_, deferrals) = freezes_and_deferrals_during(|| assert_single(&a));
        assert!(
            deferrals > 0,
            "liveness: the rank of AT({n}, {w}, {g}, alternate {alternate}) must defer"
        );
        assert_pair(&a, &b);
    }
}

/// Every arming train in [`arming_train_box`], mirrored across a fresh root
/// fork, yields two distinct streams of exactly equal rank, and `rank_cmp`
/// reads `Equal` on them in both operand orders, freezing and deferring.
///
/// This covers the signed co-sweep's `Equal` answer where freezes fire: over
/// the train dimensions, the signed settlement must cancel to zero through
/// parking, deferral, and settlement. The nonnegative pair measures never
/// produce that answer (their totals are monotone differences), and no
/// organically drawn pair reaches it at freezing scale. Every mirrored train
/// parks and defers drift under the co-sweep, which the freeze and deferral
/// floors check, so the test runs the intended path.
#[test]
fn mirrored_arming_trains_cancel_to_equal_across_their_box() {
    use crate::testing::oracles::tree::Version as V;
    for (n, w, g, alternate) in arming_train_box() {
        let train = format!("AT({n}, {w}, {g}, alternate {alternate})");
        let t = to_oracle_version(&version_of(
            &Shape::ArmingTrain.build_train(n, w, g, alternate),
        ));
        let left = from_oracle_version(&V::node(0u64, t.clone(), V::leaf(0u64)));
        let right = from_oracle_version(&V::node(0u64, V::leaf(0u64), t));
        assert_ne!(left, right, "the mirrored {train} must be distinct streams");
        assert_eq!(
            to_oracle_version(&left).rank(),
            to_oracle_version(&right).rank(),
            "the mirrored {train} must tie exactly in rank"
        );
        let mut orders = None;
        let (freezes, deferrals) = freezes_and_deferrals_during(|| {
            orders = Some((left.rank_cmp(&right), right.rank_cmp(&left)));
        });
        let orders = orders.expect("freezes_and_deferrals_during runs its closure once");
        assert_eq!(
            orders,
            (core::cmp::Ordering::Equal, core::cmp::Ordering::Equal),
            "rank_cmp on the mirrored {train}, in both orders"
        );
        assert!(
            freezes > 0 && deferrals > 0,
            "liveness: the mirrored {train} must freeze and defer under rank_cmp \
             ({freezes} freezes, {deferrals} deferrals)"
        );
    }
}

/// Checks every measure of a pair, in both operand orders, against the flat
/// model's exact values.
///
/// The values are each rank's normalized parts and bytes, the distance, both
/// lags, and the rank order through `rank_cmp` and through [`Rank`]'s own
/// order.
fn check_measures_exact(a: &flat::Version, b: &flat::Version) -> Result<(), TestCaseError> {
    let (va, vb) = (a.production(), b.production());
    let (rank_a, rank_b) = (va.rank(), vb.rank());
    check_rank(&rank_a, &a.rank(), "rank(a)")?;
    check_rank(&rank_b, &b.rank(), "rank(b)")?;
    let distance = a.distance(b);
    check_rank(&va.distance(&vb), &distance, "a.distance(b)")?;
    check_rank(&vb.distance(&va), &distance, "b.distance(a)")?;
    check_rank(&va.lag(&vb), &a.lag(b), "a.lag(b)")?;
    check_rank(&vb.lag(&va), &b.lag(a), "b.lag(a)")?;
    let order = a.rank_cmp(b);
    prop_assert_eq!(va.rank_cmp(&vb), order, "a.rank_cmp(b)");
    prop_assert_eq!(vb.rank_cmp(&va), order.reverse(), "b.rank_cmp(a)");
    prop_assert_eq!(rank_a.cmp(&rank_b), order, "rank(a).cmp(rank(b))");
    Ok(())
}

divided_proptest! {
    #![divisor(TWO_VERSION_DIVISOR)]

    /// Every rank, distance, lag, and rank order of a deep pair equals the
    /// flat model's exact value, in both operand orders.
    ///
    /// The model integrates each measure directly over the pair's common
    /// refinement, sharing nothing with the streaming integral or the tree
    /// fold. The deep pairs are related by construction, so equal pairs,
    /// distinct pairs of equal rank, and pairs differing by a narrow amount
    /// over wide heights are all common.
    #[test]
    fn measures_match_the_flat_model_on_deep_pairs((a, b) in arb_deep_or_correlated_pair()) {
        check_measures_exact(&flat::Version::from_rows(a.leaves()), &flat::Version::from_rows(b.leaves()))?;
    }

    /// Every rank, distance, lag, and rank order of a multi-scale pair equals
    /// the flat model's exact value, in both operand orders.
    ///
    /// These pairs drive the integral through freezes and deferrals and tie
    /// exactly in rank while differing as versions
    /// (`multiscale_pairs_keep_mass_on_deferral_and_ties` floors each
    /// regime); here their values are judged against the model rather than
    /// the tree fold.
    #[test]
    fn measures_match_the_flat_model_on_multiscale_pairs((a, b) in arb_multiscale_pair()) {
        check_measures_exact(&flat::Version::from_rows(a.leaves()), &flat::Version::from_rows(b.leaves()))?;
    }
}
