//! Differential pins for the query folds against the recursive oracle and the
//! composed forms.
//!
//! The recursive tree oracle (through the bridge) is the behavioral witness
//! over the adversarial families, arbitrary trees, organic histories, and the
//! exhaustive small scope: rank and min_ticks by its own folds, projection by
//! its mask, distance and lag re-derived from its join, meet, and rank through
//! the valuation identities the rustdoc states. Distance and lag are
//! additionally pinned against the composed forms — the same identities
//! assembled from this crate's own emission and rank kernels — and the pair
//! sweeps include the two version-pair families (the two-operand jump comb, the
//! concurrent pair), both as deterministic dimension sweeps and as proptests
//! over their generator dimensions. Rank is additionally pinned against the
//! semantic Riemann-sum oracle, which shares no structure with either
//! implementation.
//!
//! Every equality here is exact — `Rank` equality is structural on the
//! normalized form, projection agreement is byte identity of the emitted
//! canonical streams — so a fold that drifts by any amount anywhere has no
//! rounding to hide behind.

use proptest::prelude::*;

use crate::meter::registry::Shape;
use crate::meter::{dense_factor, factor_digit, Packed};
use crate::testing::bridge::{from_oracle_version, to_oracle_party, to_oracle_version};
use crate::testing::exhaustive::{all_normal_events, all_normal_ids, EV_SMALL_DEPTH};
use crate::testing::generators::arb_oracle_version;
use crate::testing::{optrace, semantic_oracle};
use crate::version::skyline::{emit, encode};
use crate::{Clock, Party, Version};

use super::{distance, lag, min_ticks, project, rank, rank_cmp};

/// Decode a meter-generated packed shape as a [`Version`].
fn version_of(p: &Packed) -> Version {
    p.version()
}

/// Assert the single-operand folds against the recursive tree oracle's own
/// folds.
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

/// Assert the projection kernel against the recursive oracle's mask for one
/// `(version, party)` operand pair: byte identity of the canonical streams.
fn assert_projection(v: &Version, p: &Party) {
    let enc = encode(v);
    let masked = from_oracle_version(&to_oracle_version(v).project(&to_oracle_party(p)));
    assert_eq!(
        project(&enc, p),
        encode(&masked),
        "projection must match the oracle mask: {v} / {p}"
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
    assert_eq!(rank_cmp(&ea, &eb), order, "rank_cmp: {a} vs {b}");
    assert_eq!(
        rank_cmp(&eb, &ea),
        order.reverse(),
        "rank_cmp reversed: {b} vs {a}"
    );
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
        version_of(&Shape::Dense.packed1(1)),
        version_of(&Shape::Dense.packed1(2)),
        version_of(&Shape::Dense.packed1(64)),
        version_of(&Shape::Bigroot.packed2(7, 3)),
        version_of(&Shape::Bigroot.packed2(64, 16)),
        version_of(&Shape::Hugeleaf.packed1(1)),
        version_of(&Shape::Hugeleaf.packed1(64)),
        version_of(&Shape::CliffComb.packed2(3, 2)),
        version_of(&Shape::CliffComb.packed2(16, 16)),
        version_of(&Shape::WideToothComb.packed3(16, 8, 8)),
        // Wide teeth over the freeze allowance: bounded oscillation that
        // must ride the live component without freezing.
        version_of(&Shape::WideToothComb.packed3(320, 300, 6)),
        // The stale-drift shape: the mid-stream jump is wide enough that
        // the first cheap delta behind it fires a freeze.
        version_of(&Shape::JumpComb.packed2(16, 8)),
        version_of(&Shape::JumpComb.packed2(320, 4)),
        version_of(&Shape::CliffFan.packed2(16, 8)),
        version_of(&Shape::CancellingChain.packed2(16, 8)),
        version_of(&Shape::AltSpine.packed1(3)),
        version_of(&Shape::AltSpine.packed1(64)),
        version_of(&Shape::Harmonic.packed1(16)),
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
        Party::decode(&Shape::ScatteredId.packed1(1).bytes[..])
            .expect("scattered id is strict normal form"),
        Party::decode(&Shape::ScatteredId.packed1(9).bytes[..])
            .expect("scattered id is strict normal form"),
    ]
}

/// Every adversarial family shape agrees with the tree-fold oracle's rank and
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

/// The promoting family pool: every shape whose sweep parks, promotes, or
/// settles wide drift, at hand-checkable sizes.
///
/// The other pools stay under the freeze allowance almost everywhere: a
/// unit-funded fold freezes only past 9 digits (288 bits) of live drift, and
/// `arb_base` tops out near 2^128, under half of that — so the promotion ledger
/// and its product-tree settle would run differentially unwitnessed without
/// this pool: these shapes are the only ones that arm it, and the arming trains
/// are the only ones that arm it more than once per sweep or with mixed signs.
fn promoting_pool() -> Vec<Version> {
    vec![
        version_of(&Shape::PromotionRearm.packed1(1)),
        version_of(&Shape::PromotionRearm.packed1(3)),
        version_of(&Shape::PromotionRearmMate.packed1(3)),
        version_of(&Shape::DenseSuffix.packed2(1, 2)),
        version_of(&Shape::DenseSuffix.packed2(3, 1)),
        version_of(&Shape::DenseSuffixMate.packed2(3, 1)),
        version_of(&Shape::WideArming.packed2(10, 2)),
        version_of(&Shape::WideArming.packed2(13, 3)),
        version_of(&Shape::FreezePosition.packed1(3)),
        version_of(&Shape::PlateauPuncture.packed2(10, 3)),
        version_of(&Shape::PlateauPuncture.packed2(12, 1)),
        // The first-freeze-gate straddles: the sweep's one freeze fired
        // arbitrarily late (a long never-freezing plateau prefix) and fired
        // early ahead of a long never-freezing tail — the settle's smallest
        // nonempty configuration, one parked drift against one final segment,
        // from both sides of the gate.
        version_of(&Shape::LoneFreeze.packed2(2, 2)),
        version_of(&Shape::LoneFreeze.packed2(6, 2)),
        version_of(&Shape::LoneFreeze.packed2(2, 6)),
        // The multi-arming trains: same-sign and alternating, so the settle's
        // parked sums are exercised both accumulating and cancelling across
        // aggregate seams.
        version_of(&Shape::ArmingTrain.packed_train(1, 19, 1, false)),
        version_of(&Shape::ArmingTrain.packed_train(3, 19, 1, false)),
        version_of(&Shape::ArmingTrain.packed_train(4, 19, 2, true)),
        version_of(&Shape::ArmingTrain.packed_train(5, 20, 1, true)),
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

/// `rank_cmp` agrees with the oracle rank order across the promoting pool and
/// reads `Equal` on a mirrored equal-rank promoting pair, with the freeze tap
/// proving both legs actually park drift.
///
/// The signed co-sweep's value witness in the freeze/promotion regime, with
/// its liveness floors: the promoting-pool cross pins the sign against the
/// oracle's rank order in both operand orders where the sweeps park, promote,
/// and settle wide drift, and the mirrored pair — one promoting shape hung on
/// each side of a fresh root fork, two distinct streams of exactly equal
/// rank — pins the `Equal` answer, which demands the signed settle cancel to
/// a spelled zero through the whole parked/promoted/settled pipeline. The
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
        assert_eq!(rank_cmp(&ea, &eb), want, "rank_cmp: {a} vs {b}");
        assert_eq!(
            rank_cmp(&eb, &ea),
            want.reverse(),
            "rank_cmp reversed: {a} vs {b}"
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
        &Shape::ArmingTrain.packed_train(3, 19, 1, false),
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
        let (pa, pb) = Shape::JumpPair.packed_pair3(k, m, d);
        assert_pair(&version_of(&pa), &version_of(&pb));
    }
    for n in [2, 4, 16, 64] {
        let (v, w) = Shape::ConcurrentPair.version_pair(n);
        assert_pair(&v, &w);
    }
}

/// The exhaustive small scope: every normal-form event tree to the small depth
/// agrees on rank and min_ticks, and every tree × normal-form id agrees on
/// projection — every boundary genre by brute force rather than sampling.
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
/// The total check over the pair space: every boundary genre the comparison
/// sweep's exhaustive suite reaches (aligned ties, flush-right ties at unequal
/// depths, plateau consumption, zero deltas across subtree boundaries) crossed
/// with every orientation schedule reachable at this scope, by brute force
/// rather than sampling. The oracle leg over the same identities rides the
/// family and proptest sweeps; here the composed kernels are the witness so the
/// quadratic pair product stays fast.
#[test]
fn exhaustive_small_scope_pairs_agree() {
    let events: Vec<crate::codec::BitsMut> = all_normal_events(EV_SMALL_DEPTH)
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
            rank(&encode(&a)),
            "the Riemann sum disagrees with the rank kernel: {}", a
        );
        prop_assert_eq!(
            semantic_oracle::min_ticks(&ev, g),
            min_ticks(&encode(&a)),
            "the semantic tick floor disagrees with the min_ticks kernel: {}", a
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
        let (pa, pb) = Shape::JumpPair.packed_pair3(k, m, d);
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
    /// densities from trivial to multi-digit — the ledger genres `arb_base`'s
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
        let a = version_of(&Shape::ArmingTrain.packed_train(n, w, g, alternate));
        let b = version_of(&Shape::ArmingTrain.packed_train(n, w, g, !alternate));
        assert_single(&a);
        assert_pair(&a, &b);
    }

    /// An arming train mirrored across a fresh root fork yields two distinct
    /// streams of exactly equal rank, and `rank_cmp` reads `Equal` on them in
    /// both operand orders, freezing on both legs.
    ///
    /// The `Equal` generator arm of the signed co-sweep's freeze-regime
    /// coverage: over the train dimensions, the signed settle must cancel to
    /// a spelled zero through the parked/promoted/settled pipeline — the one
    /// answer the nonnegative pair measures can never exercise (their totals
    /// are monotone differences), and one no organically drawn pair reaches
    /// at freezing scale. The freeze floor keeps the arm honest: every train
    /// in the sampled box parks drift under the co-sweep.
    #[test]
    fn arbitrary_mirrored_arming_trains_cancel_to_equal(
        n in 1usize..6,
        w in 19usize..23,
        g in 1usize..4,
        alternate: bool,
    ) {
        let t = to_oracle_version(&version_of(&Shape::ArmingTrain.packed_train(n, w, g, alternate)));
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
            rank_cmp(&el, &er),
            core::cmp::Ordering::Equal,
            "rank_cmp on the mirrored pair: {} vs {}", left, right
        );
        prop_assert_eq!(
            rank_cmp(&er, &el),
            core::cmp::Ordering::Equal,
            "rank_cmp on the mirrored pair reversed: {} vs {}", right, left
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
        use dashu_int::ops::BitTest;
        use suanpan::UBig;
        let nonzero = |bytes: &[u8]| {
            let v = UBig::from_le_bytes(bytes);
            if v == UBig::ZERO { UBig::ONE } else { v }
        };
        let (x, y) = (nonzero(&x_bytes), nonzero(&y_bytes));
        let v = Shape::PunctureProduct.packed_product(&x, &y).version();
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
        // and payload bits — even though the packed construction spells the
        // plateau per turn. Without this bound the floor argument would rest on
        // a stored size nothing checks: a fold could be charged M(|v|) against
        // an operand secretly as large as the product itself.
        prop_assert!(
            v.encoded_bits() <= 4 * x.bit_len() + 4 * (y.bit_len() + 1) + 64,
            "the stored stream must stay linear in the factors' widths: \
             {} stored bits against bits(x) = {}, bits(y) = {}",
            v.encoded_bits(),
            x.bit_len(),
            y.bit_len(),
        );
        let numerator = ((&x * &y) << 1usize) + 1u8;
        prop_assert_eq!(
            v.rank().to_string(),
            format!("{}/2^{}", numerator, y.bit_len() + 1),
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

/// The cluster seam splits exactly at gaps wider than the limit: runs whose
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
        super::integral::clusters(digits, limit)
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
        use suanpan::{Accumulator, UBig};

        use crate::codec::Base;

        // Ascending balanced digits from the gap schedule.
        let mut digits: Vec<(u64, i64)> = Vec::with_capacity(entries.len());
        let mut index = 0u64;
        for (gap, digit) in entries {
            index += gap;
            digits.push((index, digit));
            index += 1;
        }
        let factor = Base::from(UBig::from_le_bytes(&factor_bytes));
        let mut clustered = Accumulator::new();
        let sign = crate::version::skyline::signed::Sign::from_is_negative(neg);
        super::integral::charge_digits(&mut clustered, sign, &factor, &digits);
        // The oracle: one whole-span product per sign side, no
        // clustering anywhere on the path.
        let mut positive = UBig::ZERO;
        let mut negative = UBig::ZERO;
        for &(i, d) in &digits {
            let term = UBig::from(d.unsigned_abs()) << usize::try_from(32 * i).expect("test spans fit");
            if d < 0 {
                negative += term;
            } else {
                positive += term;
            }
        }
        let mut expected = Accumulator::new();
        let (add_side, sub_side) = if sign.is_negative() {
            (&negative, &positive)
        } else {
            (&positive, &negative)
        };
        expected.add_wide(&(add_side * &factor.0));
        expected.sub_wide(&(sub_side * &factor.0));
        expected.sub_accum(&clustered);
        prop_assert_eq!(
            expected.sign(),
            core::cmp::Ordering::Equal,
            "the clustered charge and the whole-span products must spell one value"
        );
    }
}

/// The clustered charge agrees with the whole-span products at the backend's
/// own multiplication-tier boundaries, with cluster-edge cancellation and the
/// balanced range's extreme digits in play.
///
/// The committed proptest above samples factors up to 200 bytes (50 base-2^32
/// digits ≈ 25 dashu words), so on a 64-bit target it never pushes a settle
/// product past the backend's simple→Karatsuba dispatch seam, let alone
/// Karatsuba→Toom-3 or Toom-3→NTT. This deterministic leg holds the value seam
/// at every dispatch boundary the shipped dashu 0.5 backend has (smaller side
/// 24 / 96 / 4,000 words, one width at and one past each), against the same
/// un-clustered whole-span oracle, over three mass geometries per width: a
/// dense run wider than the factor, digits spaced exactly at the gap limit (one
/// bridged cluster) and exactly past it (split clusters), and an
/// equal-magnitude ± pair straddling a forced split so the cancellation happens
/// in the total, never inside one densified image. Digits include both
/// balanced-range extremes (`−2^31` and `2^31 − 1`).
#[test]
fn clustered_charge_agrees_at_backend_tier_boundaries() {
    use suanpan::{Accumulator, UBig};

    use crate::codec::Base;
    use crate::version::skyline::signed::Sign;

    /// One whole-span differential: `charge_digits` versus two un-clustered
    /// products (positive and negative sides separately), exact to the digit.
    fn assert_matches(factor: &Base, digits: &[(u64, i64)], sign: Sign, label: &str) {
        let mut clustered = Accumulator::new();
        super::integral::charge_digits(&mut clustered, sign, factor, digits);
        let mut positive = UBig::ZERO;
        let mut negative = UBig::ZERO;
        for &(i, d) in digits {
            let term =
                UBig::from(d.unsigned_abs()) << usize::try_from(32 * i).expect("test spans fit");
            if d < 0 {
                negative += term;
            } else {
                positive += term;
            }
        }
        let mut expected = Accumulator::new();
        let (add_side, sub_side) = if sign.is_negative() {
            (&negative, &positive)
        } else {
            (&positive, &negative)
        };
        expected.add_wide(&(add_side * &factor.0));
        expected.sub_wide(&(sub_side * &factor.0));
        expected.sub_accum(&clustered);
        assert_eq!(
            expected.sign(),
            core::cmp::Ordering::Equal,
            "clustered charge disagrees with the whole-span products: {label}"
        );
    }

    // The balanced range's extremes, alternated so carries propagate through
    // the densified images in both sign parts.
    const EXTREMES: [i64; 4] = [-(1i64 << 31), (1i64 << 31) - 1, 1, -1];

    // dashu 0.5 dispatches on the smaller side in 64-bit words: simple ≤ 24,
    // Karatsuba ≤ 96, Toom-3 ≤ 4,000, NTT above. One width at each threshold
    // and one past it, in base-2^32 digits.
    for words in [24usize, 25, 96, 97, 4_000, 4_001] {
        let width = 2 * words;
        // A patterned full-width factor (no digit zero, top digit set).
        let factor_bytes: Vec<u8> = (0..width * 4).map(|i| (i % 251) as u8 + 1).collect();
        let factor = Base::from(UBig::from_le_bytes(&factor_bytes));
        let gap_limit = super::integral::base_digits(&factor) as u64;
        // A dense run wider than the factor: the product's smaller side is the
        // factor, so the backend engages this width's own tier.
        let dense: Vec<(u64, i64)> = (0..width as u64 + 64)
            .map(|i| (i, EXTREMES[i as usize % EXTREMES.len()]))
            .collect();
        assert_matches(&factor, &dense, Sign::Positive, "dense run");
        assert_matches(&factor, &dense, Sign::Negative, "dense run, credited");
        // Digits spaced exactly at the gap limit bridge into one cluster;
        // exactly one position further they split. Both must spell the same
        // value either way.
        for (spacing, label) in [
            (gap_limit + 1, "gaps exactly at the limit (bridged)"),
            (gap_limit + 2, "gaps exactly past the limit (split)"),
        ] {
            let spaced: Vec<(u64, i64)> = (0..6u64)
                .map(|k| (k * spacing, EXTREMES[k as usize % EXTREMES.len()]))
                .collect();
            assert_matches(&factor, &spaced, Sign::Positive, label);
        }
        // Equal magnitudes of opposite sign in adjacent clusters forced apart
        // by an over-limit gap: the value cancels only in the total, after two
        // independent densified products.
        let straddle: Vec<(u64, i64)> =
            vec![(0, (1i64 << 31) - 1), (gap_limit + 2, -((1i64 << 31) - 1))];
        assert_matches(
            &factor,
            &straddle,
            Sign::Positive,
            "cancellation across a split",
        );
    }
}

/// The settle-product tap ([`meter_product`]'s recording inside
/// [`charge_digits`]) is alive: on the wide-arming close's own operands, the
/// limb window records at least the mechanism floor, at two operand scales.
///
/// Deliberate internal-entry pin, decided here: the tap's recording is a few
/// percent of any public fold's limb column (the fold's own metered `Base`
/// arithmetic dominates), so no public-surface floor can sit above the
/// tap-dark residue without banning honest work — the seam window, where the
/// backend products are the only width-scale recording, is the one place the
/// tap's liveness is a testable number. The flatness bands this tap feeds
/// (`ledger_wide_arming`, `answer_embedded_product`, `tests/meter.rs`) bound
/// their limb columns only from above, so every one of them stays green with
/// the tap deleted; this floor is what fails instead.
///
/// The floor is derived per boundary from a universal premise, never from
/// readings. Premise: the settle delegates each cluster's product whole to
/// the backend (the `integral` module doc's settle bound), and the tap prices
/// every backend product by both operands' and the product's limb widths. On
/// the wide-arming family's close-time operands — the parked arming climb
/// `2^(32w)` as the factor, the trailing gap spine's dense `w`-digit window
/// as the mass — at least one backend product carries the full factor, so a
/// lit tap records at least
///
/// `limbs(factor) + limbs(product) + limbs(mass) ≥ 2·⌈(32w + 1)/64⌉ + 1`
///
/// (the product is at least as wide as the factor because the mass is
/// nonzero, and the nonzero mass side records at least one limb). Parked
/// width Θ(w) therefore implies Ω(w) recorded limbs from settle products
/// alone, and the floor is asserted at both scales so the linear growth is
/// pinned, not just one point. The constant reads the operand widths alone;
/// re-derive it only if the delegation shape itself changes.
#[cfg(feature = "limb-meter")]
#[test]
fn settle_product_tap_is_alive_on_the_wide_arming_close() {
    use suanpan::{Accumulator, UBig};

    use crate::codec::Base;
    use crate::meter::{limb_ops, reset_limb_ops};
    use crate::version::skyline::signed::Sign;

    for w in [500usize, 1_000] {
        // The wide-arming close's operands by construction: the parked
        // component is the family's one arming climb, the mass its dense
        // trailing window (unit digits at consecutive indices, one cluster).
        let factor = Base::from(UBig::ONE << (32 * w));
        let digits: Vec<(u64, i64)> = (0..w as u64).map(|i| (i, 1)).collect();
        // The per-boundary mechanism floor: 2·limbs(factor) + 1.
        let floor = 2 * (32 * w as u64 + 1).div_ceil(64) + 1;
        let mut total = Accumulator::new();
        reset_limb_ops();
        super::integral::charge_digits(&mut total, Sign::Positive, &factor, &digits);
        let recorded = limb_ops();
        eprintln!("MEASURED settle_product_tap w={w}: recorded={recorded} floor={floor}");
        assert!(
            recorded >= floor,
            "the settle window at parked width {w} digits records {recorded} \
             limbs, under the {floor}-limb mechanism floor: the settle-product \
             tap is not watching the backend products, and every limb ceiling \
             it feeds is passing vacuously"
        );
        // The value leg: a charge that recorded enough while computing the
        // wrong integer proves nothing, so hold the window to the exact
        // product `factor × Σᵢ 2^(32·i)`.
        let mass_bytes: Vec<u8> = (0..w * 4).map(|i| u8::from(i % 4 == 0)).collect();
        let mut expected = Accumulator::new();
        expected.add_wide(&(UBig::from_le_bytes(&mass_bytes) * &factor.0));
        expected.sub_accum(&total);
        assert_eq!(
            expected.sign(),
            core::cmp::Ordering::Equal,
            "the metered charge must spell exactly factor × mass"
        );
    }
}

/// [`charge_digits`]' densify tap records exactly the zero-filled capacity of
/// a cluster's two byte images, at shallow and deep absolute positions alike.
///
/// The rate is `2·span` digits per multi-digit cluster — `span` the
/// first-to-last live digit distance inclusive — and nothing for a
/// single-digit cluster, which takes the word-scale product without an
/// image.
///
/// Deliberate internal-entry pin, decided here (as the settle-product tap's
/// liveness pin above): the images are transient allocations whose zero fill
/// no other counter reads — a zeroed byte no digit lands on enters no operand
/// width, touches no accumulator digit, and raises no peak under the walk's
/// high-water mark — so the seam window is the one place the recorded
/// quantity can be held to the span, value-exact. The worst artifact this pin
/// excludes is a densification sized by the cluster's absolute digit
/// position: O(position) zero fill per cluster, green on every width and
/// touch counter, red here because the tap records the images' own lengths
/// and the deep cluster's equality breaks the moment the allocation outgrows
/// its span. The value leg holds the charge to the exact signed product, so a
/// densification that recorded the right capacity while landing digits at the
/// wrong offsets proves nothing.
#[cfg(feature = "limb-meter")]
#[test]
fn densify_tap_prices_the_cluster_span() {
    use suanpan::{Accumulator, UBig};

    use crate::codec::Base;
    use crate::meter::{densified_digits, reset_densified_digits};
    use crate::version::skyline::signed::Sign;

    // A 5-digit factor: the cluster gap limit under test is its width.
    let factor = Base::from(UBig::ONE << 128);
    for floor in [0u64, 100_000] {
        // One multi-digit cluster: live digits at `floor` and `floor + 2`
        // (the interior gap of 1 sits inside the factor's 5-digit gap
        // limit), span 3, both signs live so both images carry digits.
        let digits = [(floor, 1i64), (floor + 2, -3i64)];
        let mut total = Accumulator::new();
        reset_densified_digits();
        super::integral::charge_digits(&mut total, Sign::Positive, &factor, &digits);
        assert_eq!(
            densified_digits(),
            6,
            "two images at span 3 record 6 digits: the densify tap must read \
             the images' own capacity, independent of the cluster's absolute \
             position (floor {floor})"
        );
        // The value leg: exactly factor · (2^(32·floor) − 3 · 2^(32·(floor + 2))).
        let mut expected = Accumulator::new();
        expected.add_wide_shl(&factor.0, 32 * floor);
        expected.sub_wide_shl(&(&factor.0 * UBig::from(3u8)), 32 * (floor + 2));
        expected.sub_accum(&total);
        assert_eq!(
            expected.sign(),
            core::cmp::Ordering::Equal,
            "the metered charge must spell exactly factor × mass"
        );
    }
    // A single-digit cluster takes the word-scale product: no image, no fill,
    // nothing recorded.
    let digits = [(7u64, 5i64)];
    let mut total = Accumulator::new();
    reset_densified_digits();
    super::integral::charge_digits(&mut total, Sign::Positive, &factor, &digits);
    assert_eq!(
        densified_digits(),
        0,
        "a single-digit cluster densifies no image"
    );
}

/// Dense committed factors drive one settle product through the public rank at
/// each backend multiplication-tier boundary, exact against the recursive
/// oracle and the closed form.
///
/// The gap the seam-level differentials leave open: the tier-boundary charge
/// test above holds the charge kernel value-exact at every dispatch boundary,
/// but only a `charge_digits`-level operand ever reached the upper tiers — no
/// public fold drove an incompressible factor through a settle product there.
/// Here the puncture-product family does it end to end: the plateau `x` is
/// dense pseudorandom at exactly 24, 25, 96, and 97 dashu words (the
/// simple/Karatsuba and Karatsuba/Toom-3 boundaries of dashu-int 0.5.0's
/// THRESHOLD constants, dispatched on the product's smaller side), the mass `y`
/// spans 16 digits past the factor with every digit populated — fully dense at
/// 24/25, four pseudorandom bits per digit at 96/97 (the packed construction
/// pays one plateau code per mass bit, so popcount is the test's whole budget;
/// per-digit population is what the product's carry chains see) — so the
/// close-time settle's one product meets the boundary width with dense content
/// on both sides. Value legs per width: the recursive tree oracle and the
/// closed form `(2·x·y + 1) / 2^bits(2y)` through an independent backend
/// multiplication.
///
/// The Toom-3/NTT boundary (4,000/4,001 words) rides the same construction with
/// the mass thinned to one jittered turn every 400 digits: a
/// per-digit-populated NTT-scale mass would build a packed operand in the
/// hundreds of megabits, and the recursive oracle's fold over the ~256,000-leaf
/// tree is likewise out of test budget — the punctured trailing run still
/// densifies to one cluster image spanning the full smaller-side width the
/// backend dispatches on (gaps of ~399 digits sit far inside the factor-width
/// gap limit), the factor side stays fully dense, and the value leg is the
/// closed form alone.
#[test]
fn dense_factors_agree_through_the_public_fold_at_tier_boundaries() {
    // The recursive oracle and its bridge are test-only plain recursion on tree
    // depth, and the dense masses here run the spine thousands of levels deep —
    // the production folds are stack-safe (`crate::recurse::descend!`), so the
    // headroom is for the witnesses, not the code under test.
    let body = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(dense_factor_tier_legs)
        .expect("the fat-stack witness thread spawns");
    if let Err(panic) = body.join() {
        std::panic::resume_unwind(panic);
    }
}

/// The tier legs of
/// [`dense_factors_agree_through_the_public_fold_at_tier_boundaries`], on the
/// fat-stack thread the recursive oracle needs at these depths.
fn dense_factor_tier_legs() {
    use dashu_int::ops::BitTest;
    use suanpan::UBig;

    /// One puncture-product leg: the public rank against the closed form, and
    /// (where the tree fits the budget) the recursive oracle.
    fn assert_leg(x: &UBig, y: &UBig, oracle: bool, label: &str) {
        let v = Shape::PunctureProduct.packed_product(x, y).version();
        let numerator = ((x * y) << 1usize) + 1u8;
        assert_eq!(
            v.rank().to_string(),
            format!("{}/2^{}", numerator, y.bit_len() + 1),
            "the closed form must hold at the {label} boundary"
        );
        if oracle {
            assert_eq!(
                v.rank(),
                to_oracle_version(&v).rank(),
                "the tree-fold oracle must agree at the {label} boundary"
            );
        }
    }

    /// A mass with every base-2^32 digit populated by `bits`-many pseudorandom
    /// bit choices (collisions allowed, so one to `bits` live bits per digit).
    fn spread_mass(seed: u64, digits: usize, bits: u64) -> UBig {
        let mut y = UBig::ZERO;
        for digit in 0..digits {
            for b in 0..bits {
                let j = u64::from(factor_digit(seed, digit as u64 * bits + b)) % 32;
                y |= UBig::ONE << (32 * digit + j as usize);
            }
        }
        y
    }

    for (words, dense_mass) in [(24usize, true), (25, true), (96, false), (97, false)] {
        let x = dense_factor(0x5449_4552 ^ words as u64, 2 * words);
        let mass_seed = 0x4D41_5353 ^ words as u64;
        let y = if dense_mass {
            dense_factor(mass_seed, 2 * words + 16)
        } else {
            spread_mass(mass_seed, 2 * words + 16, 4)
        };
        assert_leg(&x, &y, true, &format!("{words}-word"));
    }
    for words in [4_000usize, 4_001] {
        let x = dense_factor(0x4E54_5400 ^ words as u64, 2 * words);
        let span = 2 * words + 16;
        let mut y = UBig::ZERO;
        let mut digit = 0usize;
        let mut turn = 0u64;
        while digit < span {
            let jitter = u64::from(factor_digit(0x4A49_5454, turn)) % 32;
            y |= UBig::ONE << (32 * digit + jitter as usize);
            digit += 400;
            turn += 1;
        }
        // The top turn sits at the span's last digit, so the settle product's
        // mass side strictly out-spans the factor and the backend dispatches on
        // the factor's word count exactly.
        y |= UBig::ONE << (32 * (span - 1) + 31);
        assert_leg(&x, &y, false, &format!("{words}-word"));
    }
}

/// Prefix sums of a mass vector, each leaf's mass floored at one — the split
/// currency [`integral::mass_split`](super::integral::mass_split) consumes,
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
/// ([`integral::mass_split`](super::integral::mass_split)), by the same
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
        let mid = super::integral::mass_split(&prefix, lo, hi);
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
    /// [`integral::mass_split`](super::integral::mass_split), checked by a
    /// naive recursive reference expanding the same rule: the right half
    /// never exceeds half the node's mass, and the left half exceeds it only
    /// by its straddling last leaf — which the next split isolates — so mass
    /// at least halves every second level along any root-to-leaf path. The
    /// masses are drawn log-uniformly across 48 bits of magnitude at
    /// arbitrary lengths, so both regimes (balanced splits and straddling
    /// chains) fall in-support.
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
            let mid = super::integral::mass_split(prefix, lo, hi);
            assert!(
                lo < mid && mid < hi,
                "both halves must be nonempty: lo {lo}, mid {mid}, hi {hi}"
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

/// The committed known-bad freeze accounting: the freeze-position family's
/// adequacy tripwire.
///
/// The anchored-segment integral exists because a freeze must not settle
/// evicted drift against its absolute position (the `integral` module doc's
/// discipline).
/// This module keeps the refuted accounting — the frozen/live split whose every
/// freeze correction multiplies the drift by the whole position accumulator,
/// read across its full written span — committed and *failing*: the tripwire
/// proves `FP(k)` still catches the mechanism red, so the family's green
/// flatness band (`skyline_rank_freeze_position_is_flat_per_unit`,
/// `tests/meter.rs`) is never decoration. The kernel is value-exact against the
/// shipped rank, so the demonstrator is a real implementation, not a strawman.
#[cfg(feature = "limb-meter")]
mod adequacy {
    use core::cmp::Ordering;

    use suanpan::{touch_meter, Accumulator};

    use crate::codec::{Base, BitsSlice, Int};
    use crate::meter::registry::Shape;
    use crate::version::skyline::encode;
    use crate::version::skyline::overlay::{fold, LeafCursor, PlateauCursor, Side};
    use crate::Rank;

    use crate::version::skyline::signed::{fold_signed, fold_signed_int, Sign};

    use super::super::integral::{int_digits, FREEZE_ALLOWANCE_DIGITS};
    use super::super::max_depth;
    use super::super::web::mul_into;

    /// The absolute-position rank fold: heights on a frozen/live split whose
    /// freeze correction is `drift × position` with the position accumulator
    /// read whole per freeze.
    ///
    /// Value-exact — the summation-by-parts identity `Σᵢ F(i)·massᵢ =
    /// F_final·2^S − Σ_freezes drift·position` is sound — and superlinear
    /// exactly where the tripwire asserts it: freeze `i`'s position read walks
    /// the accumulator's whole written span, which `FP(k)`'s descending spine
    /// grows with every block.
    fn absolute_position_rank(bits: &BitsSlice) -> Rank {
        let max_depth = max_depth(bits);
        let scale = max_depth as u64;
        let (mut cursor, first) = LeafCursor::open(bits);
        let mut total = Accumulator::new();
        let mut live_height = Accumulator::new();
        let mut frozen = Accumulator::new();
        fold_signed_int(&mut frozen, Sign::Positive, &first);
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
            let (_, step) = cursor.step();
            fold(&mut live_height, Side::A, step.sign, &step.magnitude);
            if live_height.digit_count() > int_digits(&step.magnitude) + FREEZE_ALLOWANCE_DIGITS {
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

    /// One tripwire run: packed bytes and the touch count over the known-bad
    /// fold, value-pinned against the shipped kernel.
    fn run(k: usize) -> (u64, u64) {
        let v = Shape::FreezePosition.packed1(k).version();
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

    /// `FP(k)` catches the absolute-position accounting red: its per-byte touch
    /// cost grows across the doubling.
    ///
    /// A linear fold reads ~x1.00 here; the floor 1.25 sits midway between
    /// linear and the measured x1.50, while the shipped kernel's flatness band
    /// holds the same family at x1.25.
    ///
    /// [measured in the dev profile, exact counters: touches 124,368 -> 372,859
    /// across FP(1,000) -> FP(2,000), packed 73,328B -> 146,579B: per-byte
    /// growth x1.50.]
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
    // whole-history position state (the `integral` module doc's
    // promotion-ledger section).
    // This kernel keeps the refuted accounting — the full anchored-segment
    // integrator whose promotion debits `P × position` by reading an absolute
    // position accumulator across its written span, re-anchoring the parked
    // component into the base — committed and failing on the promotion re-arm
    // family, through both the single-stream and the pair integrals, so the
    // green re-arm flatness bands (`skyline_flatness`, `tests/meter.rs`) are
    // never decoration. Value-exact against the shipped folds: the identity `P
    // · (2^S − position) = P · 2^S − P · position` is sound; only its cost
    // class is not.

    use crate::version::skyline::overlay::advance_diff;

    /// The anchored-segment integral with the span-reading promotion.
    ///
    /// Segments settle at the write watermark (linear on the freeze-position
    /// family), but a promotion multiplies the parked component by the absolute
    /// position accumulator, read across its full written span, and re-anchors
    /// it into the base.
    struct SpanIntegrator {
        total: Accumulator,
        live: Accumulator,
        parked: Accumulator,
        segment_mass: Accumulator,
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
                segment_mass: Accumulator::new(),
                base: Accumulator::new(),
                position: Accumulator::new(),
                one: Base::from(1u8),
            }
        }

        fn open(&mut self, opening: &Int) {
            fold_signed_int(&mut self.base, Sign::Positive, opening);
        }

        fn interval(&mut self, weight_shift: u64) {
            if !self.live.is_literally_zero() {
                self.total.add_accum_shl(&self.live, weight_shift);
            }
            self.segment_mass.add_magnitude_shl(&self.one, weight_shift);
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
                > super::super::integral::base_digits(&drift) + FREEZE_ALLOWANCE_DIGITS
            {
                self.promote();
            }
            match drift_sign {
                Ordering::Less => self.parked.sub_magnitude(&drift),
                _ => self.parked.add_magnitude(&drift),
            }
            self.live.reset();
            self.segment_mass = Accumulator::new();
        }

        fn settle_segment(&mut self) {
            let (_, segment_magnitude, segment_shift) = self.segment_mass.sign_magnitude_shl();
            if segment_magnitude == suanpan::UBig::ZERO {
                return;
            }
            let segment = Base::from(segment_magnitude);
            self.position.add_magnitude_shl(&segment, segment_shift);
            if self.parked.is_literally_zero() {
                return;
            }
            let (parked_sign, parked_magnitude) = self.parked.sign_magnitude();
            if parked_magnitude == suanpan::UBig::ZERO {
                return;
            }
            mul_into(
                &mut self.total,
                &Base::from(parked_magnitude),
                &segment,
                segment_shift,
                parked_sign == Ordering::Less,
            );
        }

        fn settle(&mut self) {
            if self.parked.is_literally_zero() {
                return;
            }
            let (parked_sign, parked_magnitude) = self.parked.sign_magnitude();
            if parked_magnitude == suanpan::UBig::ZERO {
                return;
            }
            let (_, segment_magnitude, segment_shift) = self.segment_mass.sign_magnitude_shl();
            mul_into(
                &mut self.total,
                &Base::from(parked_magnitude),
                &Base::from(segment_magnitude),
                segment_shift,
                parked_sign == Ordering::Less,
            );
        }

        /// The refuted move: `P × position` with the position read whole, then
        /// `P` re-anchored into the base.
        fn promote(&mut self) {
            let (parked_sign, parked_magnitude) = self.parked.sign_magnitude();
            if parked_magnitude != suanpan::UBig::ZERO {
                let (_, pos_mag, pos_shift) = self.position.sign_magnitude_shl();
                mul_into(
                    &mut self.total,
                    &Base::from(parked_magnitude),
                    &Base::from(pos_mag),
                    pos_shift,
                    parked_sign == Ordering::Greater,
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
            let scale = closing_shift;
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
            let (_, step) = cursor.step();
            fold(&mut integral.live, Side::A, step.sign, &step.magnitude);
            integral.boundary(super::super::integral::int_digits(&step.magnitude));
        }
        integral.finish(max_depth as u64)
    }

    /// The distance co-sweep on the span-reading integrator: the shipped pair
    /// loop verbatim (distance orientation), integrator swapped.
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
        fold_signed_int(&mut diff, Sign::Positive, &a_first);
        fold_signed_int(&mut diff, Sign::Negative, &b_first);
        let mut orient = orientation(diff.sign());
        let mut integral = SpanIntegrator::new();
        if orient != 0 {
            let (_, opening) = diff.sign_magnitude();
            integral.open(&Int::from_ubig(opening));
        }
        loop {
            let weight_shift = (overlay_depth - ca.depth().max(cb.depth())) as u64;
            integral.interval(weight_shift);
            if ca.done() && cb.done() {
                break;
            }
            let (da, db) = advance_diff(&mut ca, &mut cb, &mut diff);
            let new_orient = orientation(diff.sign());
            if orient != 0 {
                for (side, step) in [(Side::A, &da), (Side::B, &db)] {
                    if let Some(step) = step {
                        let toward = if orient > 0 { side } else { side.other() };
                        fold(&mut integral.live, toward, step.sign, &step.magnitude);
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
                .map(|step| super::super::integral::int_digits(&step.magnitude))
                .max()
                .unwrap_or(1);
            integral.boundary(funded);
        }
        integral.finish(overlay_depth as u64)
    }

    /// One rank tripwire run over `PR(p)`: packed bytes and the touch count
    /// over the known-bad fold, value-pinned against the shipped kernel.
    fn span_rank_run(p: usize) -> (u64, u64) {
        let v = Shape::PromotionRearm.packed1(p).version();
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

    /// One pair tripwire run over `(PR(p), PRM(p))`: the pair's packed bytes
    /// and the touch count over the known-bad co-sweep, value-pinned against
    /// the shipped kernel.
    fn span_pair_run(p: usize) -> (u64, u64) {
        let a = Shape::PromotionRearm.packed1(p).version();
        let b = Shape::PromotionRearmMate.packed1(p).version();
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

    /// `PR(p)` catches the span-reading promotion red on the single-stream
    /// integral: its per-byte touch cost grows across the doubling.
    ///
    /// A linear fold reads ~x1.00 here; the floor 1.36 sits midway between
    /// linear and the measured x1.74, while the shipped kernel's re-arm
    /// flatness band holds the same family at x1.25.
    ///
    /// [measured in the dev profile, exact counters: touches 1,440,756 ->
    /// 5,006,506 across PR(1,000) -> PR(2,000), packed 246,501B -> 493,001B:
    /// per-byte growth x1.74.]
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

    /// `(PR(p), PRM(p))` catches the span-reading promotion red on the pair
    /// integral: its per-byte touch cost grows across the doubling.
    ///
    /// The committed proof that the pair family drives promotions through the
    /// co-sweep, not just freezes.
    ///
    /// [measured in the dev profile, exact counters: touches 1,504,885 ->
    /// 5,134,635 across p = 1,000 -> 2,000, packed pair 269,001B -> 538,001B:
    /// per-byte growth x1.71; the floor 1.36 sits midway between linear and the
    /// measured growth, as the rank tripwire's.]
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
    // The mass-balanced product-tree settle exists because the ledger's debt
    // must not be charged by walking a shared suffix once per arming (the
    // `integral` module doc's settle bound). This kernel keeps the refuted
    // accounting —
    // the ledger assembled newest-first into one running suffix mass, each
    // arming's charge re-reading that suffix's whole density — committed and
    // failing on the dense-suffix family, through both the single-stream and
    // the pair integrals, so the green dense-suffix flatness bands
    // (`skyline_flatness`, `tests/meter.rs`) are never decoration. Value-exact
    // against the shipped folds: the suffix walk computes the same cross-term
    // sum, term by term; only its cost class is not the tree's.

    use crate::version::skyline::query::integral::{Arming, WindowMass};

    /// The anchored-segment integral with the per-arming suffix-walk settle.
    ///
    /// Promotions record funded-width ledger entries exactly as the shipped
    /// integrator does; the close then walks one running suffix mass per arming
    /// instead of reducing the entries through the balanced product tree.
    struct SuffixWalkIntegrator {
        total: Accumulator,
        live: Accumulator,
        parked: Accumulator,
        segment_mass: Accumulator,
        base: Accumulator,
        banked_window: Accumulator,
        promotions: Vec<Arming>,
        one: Base,
    }

    impl SuffixWalkIntegrator {
        fn new() -> SuffixWalkIntegrator {
            SuffixWalkIntegrator {
                total: Accumulator::new(),
                live: Accumulator::new(),
                parked: Accumulator::new(),
                segment_mass: Accumulator::new(),
                base: Accumulator::new(),
                banked_window: Accumulator::new(),
                promotions: Vec::new(),
                one: Base::from(1u8),
            }
        }

        fn open(&mut self, opening: &Int) {
            fold_signed_int(&mut self.base, Sign::Positive, opening);
        }

        fn interval(&mut self, weight_shift: u64) {
            if !self.live.is_literally_zero() {
                self.total.add_accum_shl(&self.live, weight_shift);
            }
            self.segment_mass.add_magnitude_shl(&self.one, weight_shift);
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
                > super::super::integral::base_digits(&drift) + FREEZE_ALLOWANCE_DIGITS
            {
                self.promote();
            }
            match drift_sign {
                Ordering::Less => self.parked.sub_magnitude(&drift),
                _ => self.parked.add_magnitude(&drift),
            }
            self.live.reset();
            self.segment_mass = Accumulator::new();
        }

        fn settle_segment(&mut self) {
            let (_, segment_magnitude, segment_shift) = self.segment_mass.sign_magnitude_shl();
            if segment_magnitude == suanpan::UBig::ZERO {
                return;
            }
            let segment = Base::from(segment_magnitude);
            self.banked_window
                .add_magnitude_shl(&segment, segment_shift);
            if self.parked.is_literally_zero() {
                return;
            }
            let (parked_sign, parked_magnitude) = self.parked.sign_magnitude();
            if parked_magnitude == suanpan::UBig::ZERO {
                return;
            }
            mul_into(
                &mut self.total,
                &Base::from(parked_magnitude),
                &segment,
                segment_shift,
                parked_sign == Ordering::Less,
            );
        }

        fn settle(&mut self) {
            if self.parked.is_literally_zero() {
                return;
            }
            let (parked_sign, parked_magnitude) = self.parked.sign_magnitude();
            if parked_magnitude == suanpan::UBig::ZERO {
                return;
            }
            let (_, segment_magnitude, segment_shift) = self.segment_mass.sign_magnitude_shl();
            mul_into(
                &mut self.total,
                &Base::from(parked_magnitude),
                &Base::from(segment_magnitude),
                segment_shift,
                parked_sign == Ordering::Less,
            );
        }

        fn promote(&mut self) {
            let (parked_sign, parked_magnitude) = self.parked.sign_magnitude();
            if parked_magnitude != suanpan::UBig::ZERO {
                let (_, window_magnitude, window_shift) = self.banked_window.sign_magnitude_shl();
                self.promotions.push(Arming {
                    sign: Sign::from_is_negative(parked_sign == Ordering::Less),
                    parked: Base::from(parked_magnitude),
                    window: window_magnitude,
                    shift: window_shift,
                });
                self.banked_window = Accumulator::new();
            }
            self.parked.reset();
        }

        /// The refuted settle: one running suffix mass, assembled newest-first,
        /// each arming's charge re-reading the suffix's whole balanced density.
        fn settle_armings(&mut self) {
            if self.promotions.is_empty() {
                return;
            }
            let (_, final_window_magnitude, final_window_shift) =
                self.banked_window.sign_magnitude_shl();
            let mut suffix = WindowMass::new();
            if final_window_magnitude != suanpan::UBig::ZERO {
                suffix.merge(&final_window_magnitude, final_window_shift);
            }
            let armings = core::mem::take(&mut self.promotions);
            for (i, arming) in armings.iter().enumerate().rev() {
                suffix.charge(&mut self.total, arming.sign, &arming.parked);
                if i > 0 {
                    suffix.merge(&arming.window, arming.shift);
                }
            }
        }

        fn finish(mut self, closing_shift: u64) -> Rank {
            self.settle();
            if !self.promotions.is_empty() {
                let (_, segment_magnitude, segment_shift) = self.segment_mass.sign_magnitude_shl();
                if segment_magnitude != suanpan::UBig::ZERO {
                    self.banked_window
                        .add_magnitude_shl(&Base::from(segment_magnitude), segment_shift);
                }
                self.settle_armings();
            }
            if !self.base.is_literally_zero() {
                self.total.add_accum_shl(&self.base, closing_shift);
            }
            let (sign, num) = self.total.sign_magnitude();
            debug_assert_ne!(sign, Ordering::Less, "the integrands are nonnegative");
            let scale = closing_shift;
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
            let (_, step) = cursor.step();
            fold(&mut integral.live, Side::A, step.sign, &step.magnitude);
            integral.boundary(super::super::integral::int_digits(&step.magnitude));
        }
        integral.finish(max_depth as u64)
    }

    /// The distance co-sweep on the suffix-walk integrator: the shipped pair
    /// loop verbatim (distance orientation), integrator swapped.
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
        fold_signed_int(&mut diff, Sign::Positive, &a_first);
        fold_signed_int(&mut diff, Sign::Negative, &b_first);
        let mut orient = orientation(diff.sign());
        let mut integral = SuffixWalkIntegrator::new();
        if orient != 0 {
            let (_, opening) = diff.sign_magnitude();
            integral.open(&Int::from_ubig(opening));
        }
        loop {
            let weight_shift = (overlay_depth - ca.depth().max(cb.depth())) as u64;
            integral.interval(weight_shift);
            if ca.done() && cb.done() {
                break;
            }
            let (da, db) = advance_diff(&mut ca, &mut cb, &mut diff);
            let new_orient = orientation(diff.sign());
            if orient != 0 {
                for (side, step) in [(Side::A, &da), (Side::B, &db)] {
                    if let Some(step) = step {
                        let toward = if orient > 0 { side } else { side.other() };
                        fold(&mut integral.live, toward, step.sign, &step.magnitude);
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
                .map(|step| super::super::integral::int_digits(&step.magnitude))
                .max()
                .unwrap_or(1);
            integral.boundary(funded);
        }
        integral.finish(overlay_depth as u64)
    }

    /// One rank tripwire run over `DS(p, p)`: packed bytes and the touch count
    /// over the known-bad fold, value-pinned against the shipped kernel.
    fn suffix_walk_rank_run(p: usize) -> (u64, u64) {
        let v = Shape::DenseSuffix.packed2(p, p).version();
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

    /// One pair tripwire run over `(DS(p, p), DSM(p, p))`: the pair's packed
    /// bytes and the touch count over the known-bad co-sweep, value-pinned
    /// against the shipped kernel.
    fn suffix_walk_pair_run(p: usize) -> (u64, u64) {
        let a = Shape::DenseSuffix.packed2(p, p).version();
        let b = Shape::DenseSuffixMate.packed2(p, p).version();
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

    /// `DS(p, p)` catches the per-arming suffix walk red on the single-stream
    /// integral: its per-byte touch cost grows across the doubling.
    ///
    /// A linear fold reads ~x1.00 here; the floor 1.48 sits between linear and
    /// the measured x1.75, while the shipped kernel's dense-suffix flatness
    /// band holds the same family at x1.25.
    ///
    /// [measured in the dev profile, exact counters: touches 698,584 ->
    /// 2,449,356 across DS(500, 500) -> DS(1,000, 1,000), packed 119,593B ->
    /// 239,030B: per-byte growth x1.75.]
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

    /// `(DS(p, p), DSM(p, p))` catches the per-arming suffix walk red on the
    /// pair integral: its per-byte touch cost grows across the doubling.
    ///
    /// The committed proof that the pair family drives the ledger settle
    /// through the co-sweep, not just freezes.
    ///
    /// [measured in the dev profile, exact counters: touches 810,227 ->
    /// 2,749,954 across p = 500 -> 1,000, packed pair 127,033B -> 253,909B:
    /// per-byte growth x1.70; the floor 1.48 sits between linear and the
    /// measured growth, as the rank tripwire's.]
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

    // ── the per-digit window absorb ─────────────────────────────────────
    //
    // The settle's window masses move digits as plain `i64` vector traffic,
    // invisible to the touch meter and to every `Base` shim; the per-digit tap
    // in `WindowMass::combine` is their only meter. This kernel keeps the
    // refuted merge — a product-tree absorb that folds the right half's window
    // digits into the left one digit at a time, each single-digit combine
    // re-walking the whole live vector, `O(density²)` per merge where the
    // shipped absorb is one pass over both operands — committed and failing on
    // the dense-suffix family *in the limb currency*: the committed-and-failing
    // form is available here exactly because the tap exists (without it this
    // kernel reads byte-identical to the shipped settle on every committed
    // counter — the hole the tap closes), so this tripwire is simultaneously
    // the tap's liveness proof and the dense-suffix flatness bands' adequacy
    // witness for the digit-traffic genre. Value-exact: the balanced
    // recentering is canonical per position, so digit-at-a-time recombination
    // converges to the same digit stream and every charge and the final rank
    // agree with the shipped fold exactly.

    use crate::meter::{limb_ops, reset_limb_ops};
    use crate::version::skyline::query::integral::{mass_split, Aggregate, Integrator};
    use suanpan::UBig;

    /// Fold `other` into `dst` one digit at a time: each single-digit
    /// combine re-walks `dst`'s whole live vector — the `O(density²)`
    /// absorb.
    fn per_digit_absorb(dst: &mut WindowMass, other: WindowMass) {
        for entry in other.digits {
            dst.combine(core::iter::once(entry));
        }
    }

    /// One product-tree node under the per-digit absorb: charge and
    /// parked sum exactly as [`Aggregate::merge`], the window merge
    /// swapped for [`per_digit_absorb`].
    fn merge_per_digit(left: &mut Aggregate, right: Aggregate, total: &mut Accumulator) {
        let (parked_sign, parked_magnitude) = left.parked.sign_magnitude();
        if parked_magnitude != UBig::ZERO {
            right.windows.charge(
                total,
                Sign::from_is_negative(parked_sign == Ordering::Less),
                &Base::from(parked_magnitude),
            );
        }
        left.parked.add_accum(&right.parked);
        per_digit_absorb(&mut left.windows, right.windows);
    }

    /// The shipped ledger settle with the per-digit absorb: the
    /// mass-balanced product-tree reduction verbatim, every window
    /// merge routed through [`merge_per_digit`].
    fn per_digit_settle_armings(integ: &mut Integrator) {
        if integ.promotions.is_empty() {
            return;
        }
        let armings = core::mem::take(&mut integ.promotions);
        let (_, final_window_magnitude, final_window_shift) =
            integ.banked_window.sign_magnitude_shl();
        let mut leaves: Vec<Aggregate> = Vec::with_capacity(armings.len() + 1);
        for arming in armings {
            let mut parked = Accumulator::new();
            fold_signed(&mut parked, arming.sign, &arming.parked);
            let mut windows = WindowMass::new();
            windows.merge(&arming.window, arming.shift);
            leaves.push(Aggregate { parked, windows });
        }
        let mut windows = WindowMass::new();
        if final_window_magnitude != UBig::ZERO {
            windows.merge(&final_window_magnitude, final_window_shift);
        }
        leaves.push(Aggregate {
            parked: Accumulator::new(),
            windows,
        });
        let mut prefix: Vec<u64> = Vec::with_capacity(leaves.len() + 1);
        let mut running = 0u64;
        prefix.push(0);
        for leaf in &leaves {
            running += (leaf.parked.digit_count() + leaf.windows.digits.len()).max(1) as u64;
            prefix.push(running);
        }
        enum Step {
            Open(usize, usize),
            Merge,
        }
        let leaf_count = leaves.len();
        let mut leaves = leaves.into_iter();
        let mut next_leaf = 0;
        let mut control = vec![Step::Open(0, leaf_count)];
        let mut reduced: Vec<Aggregate> = Vec::new();
        while let Some(step) = control.pop() {
            match step {
                Step::Open(lo, hi) => {
                    if hi - lo == 1 {
                        debug_assert_eq!(
                            next_leaf, lo,
                            "the left-first reduction reaches unit ranges in ascending order"
                        );
                        next_leaf += 1;
                        reduced.push(leaves.next().expect("one aggregate per unit range"));
                    } else {
                        let mid = mass_split(&prefix, lo, hi);
                        control.push(Step::Merge);
                        control.push(Step::Open(mid, hi));
                        control.push(Step::Open(lo, mid));
                    }
                }
                Step::Merge => {
                    let right = reduced.pop().expect("the right half reduced");
                    let mut left = reduced.pop().expect("the left half reduced");
                    merge_per_digit(&mut left, right, &mut integ.total);
                    reduced.push(left);
                }
            }
        }
    }

    /// The rank fold's close under the per-digit settle: the shipped
    /// `Integrator::finish` verbatim, the settle swapped.
    fn per_digit_finish(mut integ: Integrator, closing_shift: u64) -> Rank {
        integ.settle();
        if !integ.promotions.is_empty() {
            let (_, segment_magnitude, segment_shift) = integ.segment_mass.sign_magnitude_shl();
            if segment_magnitude != UBig::ZERO {
                integ
                    .banked_window
                    .add_magnitude_shl(&Base::from(segment_magnitude), segment_shift);
            }
            per_digit_settle_armings(&mut integ);
        }
        if !integ.base.is_literally_zero() {
            integ.total.add_accum_shl(&integ.base, closing_shift);
        }
        let (sign, num) = integ.total.sign_magnitude();
        debug_assert_ne!(sign, Ordering::Less, "heights are nonnegative");
        let scale = closing_shift;
        Rank::from_raw(Base::from(num), scale)
    }

    /// The rank fold on the shipped integrator with the per-digit
    /// close: the shipped [`rank`](super::super::rank) loop verbatim,
    /// only the close swapped.
    fn per_digit_rank(bits: &BitsSlice) -> Rank {
        let max_depth = max_depth(bits);
        let (mut cursor, first) = LeafCursor::open(bits);
        let mut integral = Integrator::new();
        integral.open(Sign::Positive, &first);
        loop {
            let weight_shift = (max_depth - cursor.depth()) as u64;
            integral.interval(weight_shift);
            if cursor.done() {
                break;
            }
            let (_, step) = cursor.step();
            fold(&mut integral.live, Side::A, step.sign, &step.magnitude);
            integral.boundary(super::super::integral::int_digits(&step.magnitude));
        }
        per_digit_finish(integral, max_depth as u64)
    }

    /// One tripwire run over `DS(p, p)`: packed bytes and the limb
    /// count over the known-bad fold, value-pinned against the shipped
    /// kernel.
    ///
    /// The limb currency is the point: the bad absorb's excess is pure
    /// window-digit traffic, which only the combine tap meters.
    fn per_digit_run(p: usize) -> (u64, u64) {
        let v = Shape::DenseSuffix.packed2(p, p).version();
        let enc = encode(&v);
        let expected = v.rank();
        reset_limb_ops();
        let r = per_digit_rank(&enc);
        let limbs = limb_ops();
        assert_eq!(
            r, expected,
            "the known-bad fold must stay value-exact: a wrong demonstrator \
             proves nothing about the family's coverage"
        );
        (enc.len().div_ceil(8) as u64, limbs)
    }

    /// `DS(p, p)` catches the per-digit window absorb red through the combine
    /// tap: its per-byte limb cost grows across the doubling.
    ///
    /// A linear settle reads ~x1.00 here; the floor 1.42 sits midway between
    /// linear (x1.00) and the measured growth, while the shipped kernel's
    /// dense-suffix flatness band holds the same family at x1.25 in the same
    /// currency.
    ///
    /// [measured in the dev profile, exact counters: limb ops 725,957 ->
    /// 2,702,714 across DS(500, 500) -> DS(1,000, 1,000), packed 119,593B ->
    /// 239,030B: per-byte growth x1.86 — against the shipped settle's 97,381 ->
    /// 195,491 (x1.00/byte) on the same operands.]
    #[test]
    fn per_digit_window_absorb_reads_superlinear_on_dense_suffix() {
        let (small_bytes, small_limbs) = per_digit_run(500);
        let (large_bytes, large_limbs) = per_digit_run(1_000);
        eprintln!(
            "MEASURED adequacy_per_digit_absorb: small={small_limbs}/{small_bytes}B \
             large={large_limbs}/{large_bytes}B"
        );
        assert!(
            u128::from(large_limbs) * u128::from(small_bytes) * 100
                >= u128::from(small_limbs) * u128::from(large_bytes) * 142,
            "the per-digit window absorb reads flat on the dense-suffix family \
             ({small_limbs}/{small_bytes}B -> {large_limbs}/{large_bytes}B limb \
             ops): either the combine tap went dark (the digit traffic is \
             unmetered again) or the family no longer drives dense windows \
             through the settle — in both cases the dense-suffix flatness \
             bands are decoration for this genre until a new witness lands"
        );
    }

    // ── the schoolbook settle products ──────────────────────────────────
    //
    // The settle's products are delegated cluster-wise to the backend's
    // sub-quadratic multiplication because a per-digit charge pays the factor's
    // width once per multiplicand digit — the schoolbook product (the
    // `integral` module doc's settle bound). This kernel keeps the retired
    // charge — every settle
    // product formed one factor-wide product per balanced digit — committed and
    // failing on both wide × dense families: the wide-arming family (the
    // ledger's one aggregate product) and the plateau-puncture family (the
    // arming-free close-time settle), so the `ledger_wide_arming` and
    // `answer_embedded_product` flatness bands (`tests/meter.rs`) are never
    // decoration. Value-exact against the shipped folds: the per-digit charge
    // computes the same products digit by digit; only its cost class is not the
    // backend's. Mid-sweep segment settles ride the shipped path — both
    // families' wide × dense work sits entirely at the close, which is what
    // this kernel swaps.

    /// The retired per-digit charge: one `parked`-wide product per
    /// balanced digit of the mass.
    fn schoolbook_charge(
        total: &mut Accumulator,
        sign: Sign,
        parked: &Base,
        digits: &[(u64, i64)],
    ) {
        for &(index, digit) in digits {
            let mut product = parked.clone();
            product *= u32::try_from(digit.unsigned_abs()).expect("balanced digits fit 32 bits");
            if sign.is_negative() == (digit < 0) {
                total.add_magnitude_shl(&product, 32 * index);
            } else {
                total.sub_magnitude_shl(&product, 32 * index);
            }
        }
    }

    /// One product-tree node under the schoolbook charge: parked sum and window
    /// absorb exactly as [`Aggregate::merge`], the product routed through
    /// [`schoolbook_charge`].
    fn merge_schoolbook(left: &mut Aggregate, right: Aggregate, total: &mut Accumulator) {
        let (parked_sign, parked_magnitude) = left.parked.sign_magnitude();
        if parked_magnitude != UBig::ZERO {
            schoolbook_charge(
                total,
                Sign::from_is_negative(parked_sign == Ordering::Less),
                &Base::from(parked_magnitude),
                &right.windows.digits,
            );
        }
        left.parked.add_accum(&right.parked);
        left.windows.absorb(right.windows);
    }

    /// The shipped ledger settle with the schoolbook charge: the mass-balanced
    /// product-tree reduction verbatim, every aggregate product routed through
    /// [`merge_schoolbook`].
    fn schoolbook_settle_armings(integ: &mut Integrator) {
        if integ.promotions.is_empty() {
            return;
        }
        let armings = core::mem::take(&mut integ.promotions);
        let (_, final_window_magnitude, final_window_shift) =
            integ.banked_window.sign_magnitude_shl();
        let mut leaves: Vec<Aggregate> = Vec::with_capacity(armings.len() + 1);
        for arming in armings {
            let mut parked = Accumulator::new();
            fold_signed(&mut parked, arming.sign, &arming.parked);
            let mut windows = WindowMass::new();
            windows.merge(&arming.window, arming.shift);
            leaves.push(Aggregate { parked, windows });
        }
        let mut windows = WindowMass::new();
        if final_window_magnitude != UBig::ZERO {
            windows.merge(&final_window_magnitude, final_window_shift);
        }
        leaves.push(Aggregate {
            parked: Accumulator::new(),
            windows,
        });
        let mut prefix: Vec<u64> = Vec::with_capacity(leaves.len() + 1);
        let mut running = 0u64;
        prefix.push(0);
        for leaf in &leaves {
            running += (leaf.parked.digit_count() + leaf.windows.digits.len()).max(1) as u64;
            prefix.push(running);
        }
        enum Step {
            Open(usize, usize),
            Merge,
        }
        let leaf_count = leaves.len();
        let mut leaves = leaves.into_iter();
        let mut next_leaf = 0;
        let mut control = vec![Step::Open(0, leaf_count)];
        let mut reduced: Vec<Aggregate> = Vec::new();
        while let Some(step) = control.pop() {
            match step {
                Step::Open(lo, hi) => {
                    if hi - lo == 1 {
                        debug_assert_eq!(
                            next_leaf, lo,
                            "the left-first reduction reaches unit ranges in ascending order"
                        );
                        next_leaf += 1;
                        reduced.push(leaves.next().expect("one aggregate per unit range"));
                    } else {
                        let mid = mass_split(&prefix, lo, hi);
                        control.push(Step::Merge);
                        control.push(Step::Open(mid, hi));
                        control.push(Step::Open(lo, mid));
                    }
                }
                Step::Merge => {
                    let right = reduced.pop().expect("the right half reduced");
                    let mut left = reduced.pop().expect("the left half reduced");
                    merge_schoolbook(&mut left, right, &mut integ.total);
                    reduced.push(left);
                }
            }
        }
    }

    /// The rank fold's close under the schoolbook settle: the shipped
    /// `Integrator::finish` verbatim, the close-time `P · segment` settle
    /// routed through [`mul_into`] and the ledger settle through
    /// [`schoolbook_settle_armings`].
    fn schoolbook_finish(mut integ: Integrator, closing_shift: u64) -> Rank {
        if !integ.parked.is_literally_zero() {
            let (parked_sign, parked_magnitude) = integ.parked.sign_magnitude();
            if parked_magnitude != UBig::ZERO {
                let (_, segment_magnitude, segment_shift) = integ.segment_mass.sign_magnitude_shl();
                mul_into(
                    &mut integ.total,
                    &Base::from(parked_magnitude),
                    &Base::from(segment_magnitude),
                    segment_shift,
                    parked_sign == Ordering::Less,
                );
            }
        }
        if !integ.promotions.is_empty() {
            let (_, segment_magnitude, segment_shift) = integ.segment_mass.sign_magnitude_shl();
            if segment_magnitude != UBig::ZERO {
                integ
                    .banked_window
                    .add_magnitude_shl(&Base::from(segment_magnitude), segment_shift);
            }
            schoolbook_settle_armings(&mut integ);
        }
        if !integ.base.is_literally_zero() {
            integ.total.add_accum_shl(&integ.base, closing_shift);
        }
        let (sign, num) = integ.total.sign_magnitude();
        debug_assert_ne!(sign, Ordering::Less, "heights are nonnegative");
        let scale = closing_shift;
        Rank::from_raw(Base::from(num), scale)
    }

    /// The rank fold on the shipped integrator with the schoolbook close: the
    /// shipped [`rank`](super::super::rank) loop verbatim, only the close
    /// swapped.
    fn schoolbook_rank(bits: &BitsSlice) -> Rank {
        let max_depth = max_depth(bits);
        let (mut cursor, first) = LeafCursor::open(bits);
        let mut integral = Integrator::new();
        integral.open(Sign::Positive, &first);
        loop {
            let weight_shift = (max_depth - cursor.depth()) as u64;
            integral.interval(weight_shift);
            if cursor.done() {
                break;
            }
            let (_, step) = cursor.step();
            fold(&mut integral.live, Side::A, step.sign, &step.magnitude);
            integral.boundary(super::super::integral::int_digits(&step.magnitude));
        }
        schoolbook_finish(integral, max_depth as u64)
    }

    /// One schoolbook tripwire run: packed bytes and both counters over
    /// the known-bad fold, value-pinned against the shipped kernel.
    fn schoolbook_run(packed: crate::meter::Packed) -> (u64, u64, u64) {
        let v = packed.version();
        let enc = encode(&v);
        let expected = v.rank();
        touch_meter::reset();
        reset_limb_ops();
        let r = schoolbook_rank(&enc);
        let touches = touch_meter::touches();
        let limbs = limb_ops();
        assert_eq!(
            r, expected,
            "the known-bad fold must stay value-exact: a wrong demonstrator \
             proves nothing about the family's coverage"
        );
        (enc.len().div_ceil(8) as u64, touches, limbs)
    }

    /// `WA(w, w)` catches the schoolbook charge red in both width currencies:
    /// its per-byte cost grows across the doubling.
    ///
    /// The ledger's one aggregate product pays the parked width once per window
    /// digit under this kernel; a linear fold reads ~x1.00 here, and the floor
    /// 1.44 sits midway between linear and the measured growth, while the
    /// shipped kernel's `ledger_wide_arming` band holds the same family at
    /// x1.25.
    ///
    /// [measured in the dev profile, exact counters: touches 285,747 ->
    /// 1,079,383 and limb ops 293,119 -> 1,094,191 across WA(500, 500) ->
    /// WA(1,000, 1,000), packed 14,263B -> 28,451B: per-byte growth x1.89 touch
    /// and x1.87 limb.]
    #[test]
    fn schoolbook_settle_reads_superlinear_on_wide_arming() {
        let (small_bytes, small_touches, small_limbs) =
            schoolbook_run(Shape::WideArming.packed2(500, 500));
        let (large_bytes, large_touches, large_limbs) =
            schoolbook_run(Shape::WideArming.packed2(1_000, 1_000));
        eprintln!(
            "MEASURED adequacy_schoolbook_wide_arming: small={small_touches}/{small_bytes}B \
             (limb {small_limbs}) large={large_touches}/{large_bytes}B (limb {large_limbs})"
        );
        for (name, small, large) in [
            ("touches", small_touches, large_touches),
            ("limb ops", small_limbs, large_limbs),
        ] {
            assert!(
                u128::from(large) * u128::from(small_bytes) * 100
                    >= u128::from(small) * u128::from(large_bytes) * 144,
                "the schoolbook charge reads flat ({name}) on the wide-arming \
                 family ({small}/{small_bytes}B -> {large}/{large_bytes}B): \
                 the family no longer catches the mechanism it was built for, \
                 so the wide-arming flatness band is decoration until a new \
                 witness lands"
            );
        }
    }

    /// `PP(s, s)` catches the schoolbook close-time settle red in both width
    /// currencies: its per-byte cost grows across the doubling.
    ///
    /// The arming-free site: no promotion ever fires, so the whole excess is
    /// the close-time `P · segment` product paid one digit at a time. The floor
    /// 1.32 sits midway between linear and the lower measured currency, while
    /// the shipped kernel's `answer_embedded_product` band holds the same
    /// family at x1.25.
    ///
    /// [measured in the dev profile, exact counters: touches 482,968 ->
    /// 1,843,181 and limb ops 198,320 -> 653,131 across PP(500, 500) ->
    /// PP(1,000, 1,000), packed 20,376B -> 40,751B: per-byte growth x1.91 touch
    /// and x1.65 limb.]
    #[test]
    fn schoolbook_settle_reads_superlinear_on_plateau_puncture() {
        let (small_bytes, small_touches, small_limbs) =
            schoolbook_run(Shape::PlateauPuncture.packed2(500, 500));
        let (large_bytes, large_touches, large_limbs) =
            schoolbook_run(Shape::PlateauPuncture.packed2(1_000, 1_000));
        eprintln!(
            "MEASURED adequacy_schoolbook_plateau_puncture: small={small_touches}/{small_bytes}B \
             (limb {small_limbs}) large={large_touches}/{large_bytes}B (limb {large_limbs})"
        );
        for (name, small, large) in [
            ("touches", small_touches, large_touches),
            ("limb ops", small_limbs, large_limbs),
        ] {
            assert!(
                u128::from(large) * u128::from(small_bytes) * 100
                    >= u128::from(small) * u128::from(large_bytes) * 132,
                "the schoolbook close-time settle reads flat ({name}) on the \
                 plateau-puncture family ({small}/{small_bytes}B -> \
                 {large}/{large_bytes}B): the family no longer catches the \
                 mechanism it was built for, so the answer-embedded-product \
                 flatness band is decoration until a new witness lands"
            );
        }
    }
}
