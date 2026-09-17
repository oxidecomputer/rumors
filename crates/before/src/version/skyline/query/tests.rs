//! Differential tests for the streaming queries.
//!
//! Generated versions, operation histories, and exhaustive small trees are
//! checked against the recursive tree oracle. Rank is also checked against an
//! independent Riemann-sum implementation. Distance and lag are checked both
//! against the tree oracle and against their join/meet identities.
//!
//! Every equality here is exact — `Rank` equality is structural on the
//! normalized form, projection agreement is byte identity of the emitted
//! canonical streams — so a fold that drifts by any amount anywhere has no
//! rounding to hide behind.

use num_bigint::{BigInt, BigUint, Sign};
use proptest::prelude::*;
use suanpan::Accumulator;

use crate::codec::accumulator;
use crate::meter::registry::Shape;
use crate::meter::Encoding;
use crate::testing::bridge::{from_oracle_version, to_oracle_party, to_oracle_version};
use crate::testing::exhaustive::{all_normal_events, all_normal_ids, EV_SMALL_DEPTH};
use crate::testing::generators::arb_oracle_version;
use crate::testing::{optrace, semantic_oracle};
use crate::version::skyline::{emit, encode};
use crate::{Clock, Party, Rank, Version};

use super::{distance, lag, min_ticks, project, rank, rank_cmp};

/// Big-integer adapters for the reference folds in this test module.
trait AccumulatorOracleExt {
    /// Add an unshifted oracle magnitude.
    fn add_big(&mut self, value: &BigUint);
    /// Subtract an unshifted oracle magnitude.
    fn sub_big(&mut self, value: &BigUint);
}

impl AccumulatorOracleExt for Accumulator {
    fn add_big(&mut self, value: &BigUint) {
        accumulator::fold(self, value, 0, false);
    }

    fn sub_big(&mut self, value: &BigUint) {
        accumulator::fold(self, value, 0, true);
    }
}

/// Decode a meter-generated encoded shape as a [`Version`].
fn version_of(p: &Encoding) -> Version {
    p.version()
}

/// Assert the single-operand folds against the recursive tree oracle's own
/// folds.
fn assert_single(v: &Version) {
    let enc = encode(v);
    let tree = to_oracle_version(v);
    assert_eq!(
        rank(crate::codec::built_view(&enc)),
        tree.rank(),
        "rank kernel disagrees with the tree-fold oracle: {v:?}"
    );
    assert_eq!(
        crate::Ticks(min_ticks(crate::codec::built_view(&enc))),
        tree.min_ticks(),
        "min_ticks kernel disagrees with the tree-fold oracle: {v:?}"
    );
}

/// Assert the projection kernel against the recursive oracle's mask for one
/// `(version, party)` operand pair: byte identity of the canonical streams.
fn assert_projection(v: &Version, p: &Party) {
    let enc = encode(v);
    let masked = from_oracle_version(&to_oracle_version(v).project(&to_oracle_party(p)));
    assert_eq!(
        project(crate::codec::built_view(&enc), p),
        encode(&masked),
        "projection must match the oracle mask: {v:?} / {p:?}"
    );
}

/// Assert the pair folds against the recursive oracle *and* the composed forms.
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
///
/// The signed co-sweep rides every pairing too: `rank_cmp` is pinned against
/// the oracle's rank order in both operand orders — the one fold whose total
/// carries a sign, and whose `Equal` answer demands the signed settle cancel
/// exactly through whatever parked/promoted state the pair arms (the
/// nonnegative measures never need that answer: their totals are monotone
/// differences, debug-asserted nonnegative at the fold).
fn assert_pair(a: &Version, b: &Version) {
    let (ea, eb) = (encode(a), encode(b));
    let (ta, tb) = (to_oracle_version(a), to_oracle_version(b));
    let order = ta.rank().cmp(&tb.rank());
    assert_eq!(
        rank_cmp(crate::codec::built_view(&ea), crate::codec::built_view(&eb)),
        order,
        "rank_cmp: {a:?} vs {b:?}"
    );
    assert_eq!(
        rank_cmp(crate::codec::built_view(&eb), crate::codec::built_view(&ea)),
        order.reverse(),
        "rank_cmp reversed: {b:?} vs {a:?}"
    );
    let join_rank = (ta.clone() | tb.clone()).rank();
    let meet_rank = (ta.clone() & tb.clone()).rank();
    let dist = join_rank
        .checked_sub(&meet_rank)
        .expect("rank is monotone: the meet's rank never exceeds the join's");
    assert_eq!(
        distance(crate::codec::built_view(&ea), crate::codec::built_view(&eb)),
        dist,
        "distance: {a:?} vs {b:?}"
    );
    assert_eq!(
        distance(crate::codec::built_view(&eb), crate::codec::built_view(&ea)),
        dist,
        "distance: {b:?} vs {a:?}"
    );
    let lag_a = join_rank
        .checked_sub(&ta.rank())
        .expect("rank is monotone: an operand's rank never exceeds the join's");
    let lag_b = join_rank
        .checked_sub(&tb.rank())
        .expect("rank is monotone: an operand's rank never exceeds the join's");
    assert_eq!(
        lag(crate::codec::built_view(&ea), crate::codec::built_view(&eb)),
        lag_a,
        "lag: {a:?} vs {b:?}"
    );
    assert_eq!(
        lag(crate::codec::built_view(&eb), crate::codec::built_view(&ea)),
        lag_b,
        "lag: {b:?} vs {a:?}"
    );
    // The composed forms, on this crate's own kernels.
    let kernel_join = rank(crate::codec::built_view(&emit::join(
        crate::codec::built_view(&ea),
        crate::codec::built_view(&eb),
    )));
    let kernel_meet = rank(crate::codec::built_view(&emit::meet(
        crate::codec::built_view(&ea),
        crate::codec::built_view(&eb),
    )));
    let composed_dist = kernel_join
        .checked_sub(&kernel_meet)
        .expect("rank is monotone: the meet's rank never exceeds the join's");
    assert_eq!(
        distance(crate::codec::built_view(&ea), crate::codec::built_view(&eb)),
        composed_dist,
        "distance vs the composed rank-of-meet arithmetic: {a:?} vs {b:?}"
    );
    let composed_lag_a = kernel_join
        .checked_sub(&rank(crate::codec::built_view(&ea)))
        .expect("rank is monotone: an operand's rank never exceeds the join's");
    assert_eq!(
        lag(crate::codec::built_view(&ea), crate::codec::built_view(&eb)),
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
        // must ride the live component without freezing.
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
/// The families are exactly the shapes whose costs the meter rows pin — carry
/// cliffs, wide teeth, cancelling chains, harmonic spines — so a
/// height-tracking or freeze bookkeeping error surfaces here before any
/// envelope moves.
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

/// The promoting family pool: every shape whose sweep parks, promotes, or
/// settles wide drift, at hand-checkable sizes.
///
/// The other pools stay under the freeze allowance almost everywhere: a
/// unit-funded fold freezes only past 9 digits (288 bits) of live drift, and
/// `arb_magnitude` tops out near 2^128, under half of that — so the promotion
/// ledger and its product-tree settle would run differentially unwitnessed
/// without this pool: these shapes are the only ones that arm it, and the
/// arming trains are the only ones that arm it more than once per sweep or with
/// mixed signs.
fn promoting_pool() -> Vec<Version> {
    vec![
        version_of(&Shape::PromotionRearm.build1(1)),
        version_of(&Shape::PromotionRearm.build1(3)),
        version_of(&Shape::PromotionRearmMate.build1(3)),
        version_of(&Shape::DenseSuffix.build2(1, 2)),
        version_of(&Shape::DenseSuffix.build2(3, 1)),
        version_of(&Shape::DenseSuffixMate.build2(3, 1)),
        version_of(&Shape::WideArming.build2(10, 2)),
        version_of(&Shape::WideArming.build2(13, 3)),
        version_of(&Shape::FreezePosition.build1(3)),
        version_of(&Shape::PlateauPuncture.build2(10, 3)),
        version_of(&Shape::PlateauPuncture.build2(12, 1)),
        // The first-freeze-gate straddles: the sweep's one freeze fired
        // arbitrarily late (a long never-freezing plateau prefix) and fired
        // early ahead of a long never-freezing tail — the settle's smallest
        // nonempty configuration, one parked drift against one final segment,
        // from both sides of the gate.
        version_of(&Shape::LoneFreeze.build2(2, 2)),
        version_of(&Shape::LoneFreeze.build2(6, 2)),
        version_of(&Shape::LoneFreeze.build2(2, 6)),
        // The multi-arming trains: same-sign and alternating, so the settle's
        // parked sums are exercised both accumulating and cancelling across
        // aggregate seams.
        version_of(&Shape::ArmingTrain.build_train(1, 19, 1, false)),
        version_of(&Shape::ArmingTrain.build_train(3, 19, 1, false)),
        version_of(&Shape::ArmingTrain.build_train(4, 19, 2, true)),
        version_of(&Shape::ArmingTrain.build_train(5, 20, 1, true)),
    ]
}

/// Every promoting family shape agrees with the tree-fold oracle on rank and
/// min_ticks, and every ordered pair agrees on distance and lag against both
/// the oracle and the composed forms.
///
/// The settle's value witness at the shapes the flatness bands and red pins
/// price: single and repeated armings, mixed-sign armings whose parked sums
/// cancel digit-wise inside the product tree's aggregates, dense windows
/// between armings, and the arming-free close-time settle (the plateau-puncture
/// family). The pair sweep crosses wide operands with wide operands — both
/// sides promoting, orientation flips inside wide plateaus — which the meter
/// bands' unit-twin mates never reach.
#[test]
fn promoting_families_agree_with_the_oracle() {
    let pool = promoting_pool();
    for v in &pool {
        assert_single(v);
    }
    for a in &pool {
        for b in &pool {
            assert_pair(a, b);
        }
    }
}

/// A right-spine version whose leaf heights, in stream order, are exactly
/// `heights`.
///
/// The freeze-schedule vocabulary: each adjacent difference is one folded
/// delta, so a height list is a delta script for the sweep's live component.
fn spine_of(heights: &[BigUint]) -> Version {
    use crate::oracle::Version as V;
    let mut tree = V::leaf(heights[heights.len() - 1].clone());
    for h in heights[..heights.len() - 1].iter().rev() {
        tree = V::node(0u64, V::leaf(h.clone()), tree);
    }
    from_oracle_version(&tree)
}

/// The exact-cancellation freeze schedule.
///
/// Heights whose freezes park `+2^(32p)`, `−(2^32 − 1)·2^(32(p−1))`, and
/// `−2^(32(p−1))`. Those terms cancel although the accumulator retains high
/// positive and negative digits. A fourth freeze follows a climb to `2^(32q)`
/// and return to the small drift `s`; it settles a segment against the
/// zero-valued parked component and promotes it into the skip-arming reset.
fn parked_cancellation_heights(p: u32, q: u32, s: u64) -> Vec<BigUint> {
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
        z,
        BigUint::from(s),
        BigUint::from(s + 1),
    ]
}

/// A freeze schedule whose buffered drift cancels to zero.
///
/// Deltas `+（2^(32p) + d)`, `−(2^32 − 1)·2^(32(p−1))`, `−2^(32(p−1))`, then
/// `−d`: the last, narrow delta trips the width trigger with the live
/// component's value exactly zero while its buffers still reach digit `p`.
/// The freeze therefore has no drift to park and must keep the epoch.
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
/// Four freezes fire, the fourth settling and promoting against a parked
/// component whose buffered positive and negative terms cancel. The cheap
/// `is_literally_zero` check cannot detect that cancellation, and every fold
/// stays exact against the tree oracle.
#[test]
fn parked_cancellation_settles_and_promotes_exactly() {
    let v = spine_of(&parked_cancellation_heights(11, 9, 5));
    let hits_before = super::integral::FREEZE_HITS.with(|hits| hits.get());
    assert_single(&v);
    assert!(
        super::integral::FREEZE_HITS.with(|hits| hits.get()) >= hits_before + 4,
        "the cancellation schedule no longer parks four drifts: the zero-valued \
         settle and promote arms it exists to drive are undriven"
    );
}

proptest! {
    /// The exact-cancellation family holds every fold to the tree oracle over
    /// the cancellation scale, the trailing climb's scale, and the final
    /// narrow drift.
    ///
    /// A parked component whose terms cancel must charge nothing at later
    /// settles. A promotion triggered after a large climb returns to a narrow
    /// drift, skips arming on that zero component, and still resets. Any
    /// misaccounting in either zero case changes the exact totals.
    #[test]
    fn parked_cancellation_family_agrees(p in 11u32..=14, q in 9u32..=12, s in 1u64..=6) {
        let v = spine_of(&parked_cancellation_heights(p, q, s));
        let hits_before = super::integral::FREEZE_HITS.with(|hits| hits.get());
        assert_single(&v);
        prop_assert!(
            super::integral::FREEZE_HITS.with(|hits| hits.get()) >= hits_before + 4,
            "a cancellation schedule stopped parking"
        );
    }

    /// The zero-drift family: a width trigger tripped by buffered terms that
    /// cancel exactly freezes nothing.
    ///
    /// The rank integral parks no drift and min_ticks keeps its epoch, and
    /// both folds stay exact against the tree oracle through the empty
    /// freeze.
    #[test]
    fn zero_drift_freezes_keep_the_totals_exact(p in 9u32..=13, d in 1u64..=6) {
        assert_single(&spine_of(&zero_drift_heights(p, d)));
    }
}

/// `rank_cmp` agrees with the oracle rank order across the promoting pool and
/// reads `Equal` on a mirrored equal-rank promoting pair, with the freeze tap
/// proving both legs actually park drift.
///
/// The signed co-sweep's value witness in the freeze/promotion regime, with
/// its liveness floors: the promoting-pool cross pins the sign against the
/// oracle's rank order in both operand orders where the sweeps park, promote,
/// and settle wide drift, and the mirrored pair — one promoting shape hung on
/// each side of a fresh root fork, two distinct streams of exactly equal
/// rank — pins the `Equal` answer, which demands that the signed settle cancel
/// to zero through the whole parked/promoted/settled pipeline. The
/// [`FREEZE_HITS`](super::integral::FREEZE_HITS) floors make the regime claim
/// non-vacuous: a pool or pair that never froze would pass any value pin
/// while exercising none of the ledger.
#[test]
fn rank_cmp_agrees_with_the_oracle_in_the_freeze_regime() {
    let assert_cmp = |a: &Version, b: &Version| {
        let (ea, eb) = (encode(a), encode(b));
        let want = to_oracle_version(a)
            .rank()
            .cmp(&to_oracle_version(b).rank());
        assert_eq!(
            rank_cmp(crate::codec::built_view(&ea), crate::codec::built_view(&eb)),
            want,
            "rank_cmp: {a:?} vs {b:?}"
        );
        assert_eq!(
            rank_cmp(crate::codec::built_view(&eb), crate::codec::built_view(&ea)),
            want.reverse(),
            "rank_cmp reversed: {a:?} vs {b:?}"
        );
    };
    let hits_before = super::integral::FREEZE_HITS.with(|hits| hits.get());
    let pool = promoting_pool();
    for a in &pool {
        for b in &pool {
            assert_cmp(a, b);
        }
    }
    let pool_hits = super::integral::FREEZE_HITS.with(|hits| hits.get());
    assert!(
        pool_hits > hits_before,
        "liveness: the promoting-pool cross must run freezes under rank_cmp"
    );
    let t = to_oracle_version(&version_of(
        &Shape::ArmingTrain.build_train(3, 19, 1, false),
    ));
    let zero = crate::oracle::Version::leaf(0u64);
    let left = from_oracle_version(&crate::oracle::Version::node(0u64, t.clone(), zero.clone()));
    let right = from_oracle_version(&crate::oracle::Version::node(0u64, zero, t));
    assert_ne!(
        encode(&left),
        encode(&right),
        "the mirrored pair must be distinct streams"
    );
    assert_eq!(
        to_oracle_version(&left).rank(),
        to_oracle_version(&right).rank(),
        "the mirrored pair must tie exactly in rank"
    );
    let mirror_hits_before = super::integral::FREEZE_HITS.with(|hits| hits.get());
    assert_cmp(&left, &right);
    assert!(
        super::integral::FREEZE_HITS.with(|hits| hits.get()) > mirror_hits_before,
        "liveness: the mirrored equal-rank pair must run freezes under rank_cmp"
    );
}

/// The two version-pair families agree with the oracle and the composed forms
/// on distance and lag, at their own constructed pairings.
///
/// The jump-pair dimensions straddle the freeze allowance (`k = 3` up to `k =
/// 512`, past the 256-bit digit bound), so the pair measures are pinned on
/// shapes where the difference's live component rides wide drift across the
/// other operand's cheap boundaries — the interleaving the family exists to
/// reach; the concurrent pair pins the side-switch density population, where
/// the difference's sign flips at every one of the `n − 1` overlay boundaries.
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
/// rather than sampling. The oracle leg over the same identities rides the
/// family and proptest sweeps; here the composed kernels are the witness so the
/// quadratic pair product stays fast.
#[test]
fn exhaustive_small_scope_pairs_agree() {
    let events: Vec<crate::codec::BitsBuf> = all_normal_events(EV_SMALL_DEPTH)
        .iter()
        .map(|t| encode(&from_oracle_version(t)))
        .collect();
    for ea in &events {
        let rank_a = rank(crate::codec::built_view(ea));
        for eb in &events {
            let join = rank(crate::codec::built_view(&emit::join(
                crate::codec::built_view(ea),
                crate::codec::built_view(eb),
            )));
            let meet = rank(crate::codec::built_view(&emit::meet(
                crate::codec::built_view(ea),
                crate::codec::built_view(eb),
            )));
            let composed_dist = join
                .checked_sub(&meet)
                .expect("rank is monotone: the meet's rank never exceeds the join's");
            assert_eq!(
                distance(crate::codec::built_view(ea), crate::codec::built_view(eb)),
                composed_dist,
                "distance at a small-scope pair"
            );
            let composed_lag = join
                .checked_sub(&rank_a)
                .expect("rank is monotone: an operand's rank never exceeds the join's");
            assert_eq!(
                lag(crate::codec::built_view(ea), crate::codec::built_view(eb)),
                composed_lag,
                "lag at a small-scope pair"
            );
        }
    }
}

proptest! {
    /// Arbitrary normal-form trees agree with the recursive oracle on every
    /// query fold.
    ///
    /// Rank is additionally realized by the semantic oracle's Riemann sum over
    /// the resolving grid — the geometric ground truth that shares no
    /// recursion, no delta, and no accumulator with either implementation.
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
            rank(crate::codec::built_view(&encode(&a))),
            "the Riemann sum disagrees with the rank kernel: {:?}", a
        );
        prop_assert_eq!(
            semantic_oracle::min_ticks(&ev, g),
            min_ticks(crate::codec::built_view(&encode(&a))),
            "the semantic tick floor disagrees with the min_ticks kernel: {:?}", a
        );
    }

    /// One organic fork/tick/send/sync/join history agrees with the recursive
    /// oracle on every query fold.
    ///
    /// Projection runs onto the history's own parties — the operand pairing
    /// production code actually builds.
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

    /// Arming trains at arbitrary dimensions agree with the oracle on every
    /// query fold, singly and as a promoting × promoting pair.
    ///
    /// The dimensions cover arming counts across several product-tree shapes (a
    /// lone entry, a full level, an odd drain), both sign schedules, and window
    /// densities from trivial to multi-digit, beyond `arb_magnitude`'s
    /// 128-bit ceiling keeps the arbitrary-tree sweep from ever arming. The
    /// pair leg crosses the train against its opposite-schedule twin, so the
    /// co-sweep promotes on both operands with the difference's orientation
    /// flipping inside wide plateaus.
    #[test]
    fn arbitrary_arming_trains_agree(
        n in 1usize..6,
        w in 19usize..23,
        g in 1usize..4,
        alternate: bool,
    ) {
        let a = version_of(&Shape::ArmingTrain.build_train(n, w, g, alternate));
        let b = version_of(&Shape::ArmingTrain.build_train(n, w, g, !alternate));
        assert_single(&a);
        assert_pair(&a, &b);
    }

    /// An arming train mirrored across a fresh root fork yields two distinct
    /// streams of exactly equal rank, and `rank_cmp` reads `Equal` on them in
    /// both operand orders, freezing on both legs.
    ///
    /// The `Equal` generator arm of the signed co-sweep's freeze-regime
    /// coverage: over the train dimensions, the signed settle must cancel to
    /// zero through the parked/promoted/settled pipeline — the one
    /// answer the nonnegative pair measures can never exercise (their totals
    /// are monotone differences), and one no organically drawn pair reaches
    /// at freezing scale. Every train in the sampled box parks drift under the
    /// co-sweep, so the property exercises the intended path.
    #[test]
    fn arbitrary_mirrored_arming_trains_cancel_to_equal(
        n in 1usize..6,
        w in 19usize..23,
        g in 1usize..4,
        alternate: bool,
    ) {
        let t = to_oracle_version(&version_of(&Shape::ArmingTrain.build_train(n, w, g, alternate)));
        let zero = crate::oracle::Version::leaf(0u64);
        let left = from_oracle_version(&crate::oracle::Version::node(0u64, t.clone(), zero.clone()));
        let right = from_oracle_version(&crate::oracle::Version::node(0u64, zero, t));
        let (el, er) = (encode(&left), encode(&right));
        prop_assert_ne!(&el, &er, "the mirrored pair must be distinct streams");
        prop_assert_eq!(
            to_oracle_version(&left).rank(),
            to_oracle_version(&right).rank(),
            "the mirrored pair must tie exactly in rank"
        );
        let hits_before = super::integral::FREEZE_HITS.with(|hits| hits.get());
        prop_assert_eq!(
            rank_cmp(crate::codec::built_view(&el), crate::codec::built_view(&er)),
            core::cmp::Ordering::Equal,
            "rank_cmp on the mirrored pair: {:?} vs {:?}", left, right
        );
        prop_assert_eq!(
            rank_cmp(crate::codec::built_view(&er), crate::codec::built_view(&el)),
            core::cmp::Ordering::Equal,
            "rank_cmp on the mirrored pair reversed: {:?} vs {:?}", right, left
        );
        prop_assert!(
            super::integral::FREEZE_HITS.with(|hits| hits.get()) > hits_before,
            "liveness: the mirrored train must run freezes under rank_cmp"
        );
    }

    /// The projection kernel agrees with the oracle's semantic mask over
    /// arbitrary tree × arbitrary id operands, where the id's absent children
    /// exercise the synthetic-empty arm at every depth.
    #[test]
    fn arbitrary_projections_agree(
        ov in arb_oracle_version(),
        oi in proptest::sample::select(all_normal_ids(3)),
    ) {
        let v = from_oracle_version(&ov);
        let p = crate::testing::bridge::from_oracle_party(&oi);
        assert_projection(&v, &p);
    }

    /// The exact rank embeds the product of two arbitrary integers: `rank(V(x,
    /// y)) = (2·x·y + 1) / 2^bits(2y)` for every positive `x` and `y`, through
    /// the public fold.
    ///
    /// The `Ω(M(·))` floor's evidence of record — a reduction from arbitrary
    /// integer multiplication, not a bet on one committed shape: `V(x, y)`
    /// stores `Θ(bits(x) + bits(y))` bits, and its exact rank's numerator
    /// carries the full product (one subtraction and one shift recover `x·y`),
    /// so any fold that answers this family exactly multiplies two arbitrary
    /// input-funded factors at linear overhead. The independent witness is the
    /// backend's own multiplication, computed here outside any fold. The pair
    /// measures inherit the floor through their valuation identities — against
    /// the empty version, distance and lag both collapse to the rank — asserted
    /// here on the public entry points.
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
        // The reduction's size premise, pinned where the reduction is
        // constructed: the STORED stream is Θ(bits(x) + bits(y)) — the deltas
        // collapse to one climb (≤ 2·bits(x) + 1 code bits) and one plunge (≤
        // 2·bits(x) + 3), and each of the `bits(2y)` levels costs O(1) topology
        // and payload bits — even though the encoded construction spells the
        // plateau per turn. Without this bound the floor argument would rest on
        // a stored size nothing checks: a fold could be charged M(|v|) against
        // an operand secretly as large as the product itself.
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

/// Clusters split exactly at gaps wider than the limit: runs whose
/// interior gaps stay within it stay whole, and a single over-wide gap is the
/// only cut.
///
/// The deterministic geometry leg under the value proptest below: the split
/// points are what the settle's cost bound reasons from (gaps wider than the
/// factor never densify), so they are pinned by position, not just by
/// round-tripped value.
#[test]
fn clusters_split_exactly_at_the_gap_limit() {
    let digits: &[(u64, i64)] = &[(0, 1), (3, -2), (4, 5), (8, 1), (20, -7)];
    // gap(0→3) = 2, gap(4→8) = 3, gap(8→20) = 11.
    let split = |limit: u64| -> Vec<Vec<u64>> {
        super::integral::WindowMass::clusters(digits, limit)
            .map(|c| c.iter().map(|&(i, _)| i).collect())
            .collect()
    };
    assert_eq!(
        split(1),
        vec![vec![0], vec![3, 4], vec![8], vec![20]],
        "gaps of 2, 3, and 11 all exceed a limit of 1"
    );
    assert_eq!(
        split(3),
        vec![vec![0, 3, 4, 8], vec![20]],
        "gaps of 2 and 3 bridge at a limit of 3; the 11 splits"
    );
    assert_eq!(
        split(11),
        vec![vec![0, 3, 4, 8, 20]],
        "every gap bridges once the limit reaches the widest"
    );
}

proptest! {
    /// The clustered settle charge agrees exactly with two whole-span backend
    /// products over the same signed mass.
    ///
    /// Cluster splitting, the densified positive/negative images, the
    /// single-digit fast path, and the scaled adds re-spell the same integer
    /// for every gap schedule — including gaps straddling the factor-width
    /// split threshold and cancellation across cluster edges.
    #[test]
    fn clustered_charge_agrees_with_whole_span_products(
        factor_bytes in proptest::collection::vec(any::<u8>(), 1..200),
        entries in proptest::collection::vec(
            (0u64..80, (-(1i64 << 31)..(1i64 << 31)).prop_filter("nonzero", |d| *d != 0)),
            1..60,
        ),
        neg in any::<bool>(),
    ) {
        // Ascending balanced digits from the gap schedule.
        let mut digits: Vec<(u64, i64)> = Vec::with_capacity(entries.len());
        let mut index = 0u64;
        for (gap, digit) in entries {
            index += gap;
            digits.push((index, digit));
            index += 1;
        }
        let factor = BigUint::from_bytes_le(&factor_bytes);
        let mut clustered = Accumulator::new();
        let sign = if neg { Sign::Minus } else { Sign::Plus };
        // The oracle: one whole-span product per sign side, no
        // clustering anywhere on the path.
        let mut positive = BigUint::ZERO;
        let mut negative = BigUint::ZERO;
        for &(i, d) in &digits {
            let term = BigUint::from(d.unsigned_abs()) << usize::try_from(32 * i).expect("test spans fit");
            if d < 0 {
                negative += term;
            } else {
                positive += term;
            }
        }
        let signed_factor = BigInt::from_biguint(sign, factor.clone());
        super::integral::WindowMass { digits }.charge(&mut clustered, &signed_factor);
        let mut expected = Accumulator::new();
        let (add_side, sub_side) = if sign == Sign::Minus {
            (&negative, &positive)
        } else {
            (&positive, &negative)
        };
        expected.add_big(&(add_side * &factor));
        expected.sub_big(&(sub_side * &factor));
        expected.sub_accum(&clustered);
        prop_assert_eq!(
            expected.sign(),
            core::cmp::Ordering::Equal,
            "the clustered charge and the whole-span products must spell one value"
        );
    }
}

/// Prefix sums of a mass vector, each leaf's mass floored at one — the measure
/// [`Integrator::mass_split`](super::integral::Integrator::mass_split) consumes,
/// built exactly as the shipped settle builds it.
fn mass_prefix(masses: &[u64]) -> Vec<u64> {
    let mut prefix: Vec<u64> = Vec::with_capacity(masses.len() + 1);
    prefix.push(0);
    for &m in masses {
        prefix.push(prefix.last().expect("seeded nonempty") + m.max(1));
    }
    prefix
}

/// Depth of the deepest leaf under the shipped split rule
/// ([`Integrator::mass_split`](super::integral::Integrator::mass_split)), by the same
/// explicit-stack expansion the settle runs.
fn split_depth(masses: &[u64]) -> usize {
    let prefix = mass_prefix(masses);
    let mut deepest = 0;
    let mut stack = vec![(0usize, masses.len(), 0usize)];
    while let Some((lo, hi, depth)) = stack.pop() {
        if hi - lo == 1 {
            deepest = deepest.max(depth);
            continue;
        }
        let mid = super::integral::Integrator::mass_split(&prefix, lo, hi);
        stack.push((mid, hi, depth + 1));
        stack.push((lo, mid, depth + 1));
    }
    deepest
}

/// The mass-balanced split isolates one leaf per level on exponentially spread
/// masses: the product tree's depth is `n − 1` there, linear in the entry
/// count, while uniform masses keep it at `⌈log₂ n⌉`.
///
/// The deterministic points behind the `integral` module doc's depth
/// denomination, driven through the shipped split rule itself: a leaf's depth
/// is governed by the *total mass*, never by any function of the entry count
/// alone — with `n` entries of masses `2^1..2^n` the deepest entry is re-read
/// `n − 1` times, not `O(log n)`. The doubled family (each mass repeated
/// twice) pins the bound's constant: it also chains one isolating split per
/// level, but on half the mass budget per level — the straddling-leaf regime
/// where mass halves only every *second* level, which is why the pinned bound
/// is `2·log₂(total) + 2` and not `log₂(total) + 1`. Every aggregate cost
/// bound absorbs this (heavy leaves sit shallow, so mass-weighted traffic
/// stays entropy-bounded at the total mass times the entry-count logarithm),
/// which is why the `integral` module doc denominates the tree's depth in
/// settle mass — `O(log |v|)`, the mass being input-funded — and why its
/// `O((n + D) log n)` claim is conditioned on `O(1)`-wide parked masses.
#[test]
fn mass_midpoint_split_runs_linear_depth_on_exponential_masses() {
    let n = 16usize;
    let exponential: Vec<u64> = (1..=n as u32).map(|i| 1u64 << i).collect();
    assert_eq!(
        split_depth(&exponential),
        n - 1,
        "exponentially spread masses must chain: one isolating split per level"
    );
    // Three unit leaves, then the powers 2^1..2^k twice each: total mass
    // 2^(k+2) − 1 against 2k + 3 entries, and every second split only sheds
    // a straddling leaf instead of halving.
    let mut doubled: Vec<u64> = vec![1, 1, 1];
    for i in 1..=6u32 {
        doubled.push(1 << i);
        doubled.push(1 << i);
    }
    assert_eq!(
        split_depth(&doubled),
        doubled.len() - 1,
        "doubled masses must chain one isolating split per level on half the \
         mass budget: the two-levels-per-halving regime is real"
    );
    let uniform: Vec<u64> = vec![8; n];
    assert_eq!(
        split_depth(&uniform),
        4,
        "uniform masses must balance to ⌈log₂ n⌉ levels"
    );
}

proptest! {
    /// The shipped split rule keeps both halves nonempty at every node, and
    /// the deepest leaf sits within `2·log₂(total mass) + 2` levels, on
    /// arbitrary mass vectors.
    ///
    /// The size-generic contract of
    /// [`Integrator::mass_split`](super::integral::Integrator::mass_split),
    /// checked by a naive recursive reference expanding the same rule: the
    /// right half never exceeds half the node's mass, and the left half exceeds
    /// it only by its straddling last leaf — which the next split isolates — so
    /// mass at least halves every second level along any root-to-leaf path. The
    /// masses are drawn log-uniformly across 48 bits of magnitude at arbitrary
    /// lengths, so both regimes (balanced splits and straddling chains) fall
    /// in-support.
    #[test]
    fn arbitrary_mass_vectors_split_nonempty_and_entropy_bounded(
        masses in proptest::collection::vec(
            (0u32..48).prop_flat_map(|s| (1u64 << s)..=((1u64 << (s + 1)) - 1)),
            1..64,
        ),
    ) {
        // Deepest-leaf depth by naive recursion on the shipped rule,
        // asserting both halves nonempty at every node; recursion depth is
        // bounded by the leaf count, which the strategy caps.
        fn depth_by_recursion(prefix: &[u64], lo: usize, hi: usize) -> usize {
            if hi - lo == 1 {
                return 0;
            }
            let mid = super::integral::Integrator::mass_split(prefix, lo, hi);
            assert!(
                lo < mid && mid < hi,
                "both halves must be nonempty: lo {lo:?}, mid {mid:?}, hi {hi:?}"
            );
            1 + depth_by_recursion(prefix, lo, mid).max(depth_by_recursion(prefix, mid, hi))
        }

        let prefix = mass_prefix(&masses);
        let total = *prefix.last().expect("seeded nonempty");
        let depth = depth_by_recursion(&prefix, 0, masses.len());
        prop_assert_eq!(
            depth,
            split_depth(&masses),
            "the recursive reference and the settle's explicit-stack expansion \
             must walk the same tree"
        );
        prop_assert!(
            depth as u32 <= 2 * total.ilog2() + 2,
            "the deepest leaf must sit within the entropy bound: depth {} \
             against total mass {}",
            depth,
            total
        );
    }
}
