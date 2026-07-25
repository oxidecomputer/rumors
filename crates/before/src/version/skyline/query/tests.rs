//! Differential pins for the query folds against the recursive oracle.
//!
//! The recursive tree oracle (through the bridge) is the behavioral
//! witness over the adversarial families, arbitrary trees, organic
//! histories, and the exhaustive small scope: rank and min_ticks by its
//! own folds, projection by its mask, distance and lag re-derived from
//! its join, meet, and rank through the valuation identities the rustdoc
//! states. Rank is additionally pinned against the semantic Riemann-sum
//! oracle, which shares no structure with either implementation.
//!
//! Every equality here is exact — `Rank` equality is structural on the
//! normalized form, projection agreement is byte identity of the emitted
//! canonical streams — so a fold that drifts by any amount anywhere has
//! no rounding to hide behind.

use proptest::prelude::*;

use crate::meter::{
    alt_spine, bigroot, cancelling_chain, cliff_comb, cliff_fan, dense, harmonic, hugeleaf,
    jump_comb, wide_tooth_comb, Packed,
};
use crate::testing::bridge::{from_oracle_version, to_oracle_party, to_oracle_version};
use crate::testing::exhaustive::{all_normal_events, all_normal_ids, EV_SMALL_DEPTH};
use crate::testing::generators::arb_oracle_version;
use crate::testing::{optrace, semantic_oracle};
use crate::version::skyline::encode;
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
        min_ticks(&enc),
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

/// Assert the pair folds against the recursive oracle.
///
/// Each measure is re-derived from the oracle's join, meet, and rank
/// through the valuation identities the rustdoc states:
/// `distance = rank(a ∨ b) − rank(a ∧ b)` and
/// `lag(a, b) = rank(a ∨ b) − rank(a)`.
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
            semantic_oracle::min_ticks(&ev, g).to_u64_saturating(),
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
