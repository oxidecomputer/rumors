//! Differential tests for the streaming numeric measures.
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

use num_bigint::BigUint;
use proptest::prelude::*;

use crate::testing::bridge::{from_oracle_version, to_oracle_party, to_oracle_version};
use crate::testing::exhaustive::{all_normal_events, all_normal_ids, EV_SMALL_DEPTH};
use crate::testing::generators::arb_oracle_version;
use crate::testing::meter::registry::Shape;
use crate::testing::meter::Encoding;
use crate::testing::optrace;
use crate::testing::oracles::function;
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
/// The freeze therefore has no height change to park and must keep the prefix.
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
/// `is_known_zero` check cannot detect that cancellation, and every fold
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
    /// The rank integral parks no drift and min_ticks keeps its prefix, and
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
/// rather than sampling. The oracle leg over the same identities rides the
/// family and proptest sweeps; here the composed kernels are the witness so the
/// quadratic pair product stays fast.
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
    /// the resolving grid — the geometric ground truth that shares no
    /// recursion, no delta, and no accumulator with either implementation.
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

    /// One organic fork/tick/send/sync/join history agrees with the recursive
    /// oracle on every measure fold.
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
    /// measure fold, singly and as a promoting × promoting pair.
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
        let zero = crate::testing::oracles::tree::Version::leaf(0u64);
        let left = from_oracle_version(&crate::testing::oracles::tree::Version::node(0u64, t.clone(), zero.clone()));
        let right = from_oracle_version(&crate::testing::oracles::tree::Version::node(0u64, zero, t));
        prop_assert_ne!(&left, &right, "the mirrored pair must be distinct streams");
        prop_assert_eq!(
            to_oracle_version(&left).rank(),
            to_oracle_version(&right).rank(),
            "the mirrored pair must tie exactly in rank"
        );
        let hits_before = super::integral::FREEZE_HITS.with(|hits| hits.get());
        prop_assert_eq!(
            left.rank_cmp(&right),
            core::cmp::Ordering::Equal,
            "rank_cmp on the mirrored pair: {:?} vs {:?}", left, right
        );
        prop_assert_eq!(
            right.rank_cmp(&left),
            core::cmp::Ordering::Equal,
            "rank_cmp on the mirrored pair reversed: {:?} vs {:?}", right, left
        );
        prop_assert!(
            super::integral::FREEZE_HITS.with(|hits| hits.get()) > hits_before,
            "liveness: the mirrored train must run freezes under rank_cmp"
        );
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
