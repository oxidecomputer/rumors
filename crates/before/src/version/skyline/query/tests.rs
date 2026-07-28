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
}
